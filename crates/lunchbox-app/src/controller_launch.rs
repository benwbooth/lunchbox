//! Launch-scoped controller adapters. Never write a user's emulator config.
use std::collections::BTreeMap;
use std::ffi::OsString;
use std::io::Read;
use std::path::Path;

use crate::controller_catalog::{Calibration, EmulatorProfile, NativeInput, catalog};
use crate::controllers::ControllerDevice;
use crate::emulator::{EmulatorExecutable, EmulatorRuntimeKind, LaunchPlan, RomEmulatorOption};
use crate::settings::AppSettings;
use anyhow::{Context, Result, bail, ensure};

mod mame_automatic;
mod mame_configuration;

#[cfg(all(test, target_os = "linux"))]
#[path = "controller_launch_oracle.rs"]
mod oracle;

enum PreparedBizhawkLaunch {
    Configuration(crate::controller_bizhawk::PreparedConfig),
    #[cfg(target_os = "linux")]
    Captured(crate::controller_bizhawk::digital_session::PreparedCartridgeHandoff),
}

impl PreparedBizhawkLaunch {
    fn verify(&self) -> Result<()> {
        match self {
            Self::Configuration(config) => config.verify_source(),
            #[cfg(target_os = "linux")]
            Self::Captured(handoff) => handoff.verify(),
        }
    }
}

#[derive(Default)]
pub struct CalibratedLaunch {
    #[cfg(target_os = "linux")]
    snes9x_native: Option<crate::controller_snes9x::native_command::NativeSession>,
    #[cfg(target_os = "linux")]
    fceux_native: Option<crate::controller_fceux::native_command::NativeSession>,
    #[cfg(target_os = "linux")]
    sameboy_native: Option<crate::controller_sameboy::native_command::NativeSession>,
    bsnes_native: Option<crate::controller_bsnes::native_command::NativeSession>,
    #[cfg(target_os = "linux")]
    mednafen_native: Option<crate::controller_mednafen::native_command::NativeSession>,
    #[cfg(target_os = "linux")]
    mame_native: Option<crate::controller_mame_native::native_command::NativeSession>,
    #[cfg(target_os = "linux")]
    flycast_native: Option<crate::controller_flycast_native::native_command::NativeSession>,
    #[cfg(target_os = "linux")]
    pcsx2_native: Option<crate::controller_pcsx2::native_command::NativeSession>,
    #[cfg(target_os = "linux")]
    rpcs3_native: Option<crate::controller_rpcs3::native_command::NativeSession>,
    #[cfg(target_os = "linux")]
    melonds_native: Option<crate::controller_melonds::native_command::NativeSession>,
    #[cfg(target_os = "linux")]
    dolphin_native: Option<crate::controller_dolphin::standalone::native_command::PreparedCommand>,
    #[cfg(target_os = "linux")]
    mgba: Option<crate::controller_mgba::native_command::NativeSession>,
    #[cfg(target_os = "linux")]
    ppsspp: Option<crate::controller_ppsspp::native_command::PreparedLaunch>,
    #[cfg(target_os = "linux")]
    duckstation: Option<crate::controller_duckstation::native_command::NativeSession>,
    // Keeps the private append config alive until the child exits, including errors.
    _directory: Option<tempfile::TempDir>,
    bizhawk: Option<PreparedBizhawkLaunch>,
    #[cfg(target_os = "linux")]
    bizhawk_topology: Option<crate::controller_bizhawk_guard::InputTopology>,
    transports: Vec<PlayerTransport>,
    crocods: Option<crate::controller_crocods::InputSnapshot>,
    ep128emu: Option<crate::controller_ep128emu::InputSnapshot>,
    hatari: Option<crate::controller_hatari::InputSnapshot>,
    simcp: Option<crate::controller_simcp::PreparedInput>,
    steemsse: Option<crate::controller_steemsse::InputSnapshot>,
    scummvm: Option<crate::controller_scummvm::InputSnapshot>,
    dolphin: Option<crate::controller_dolphin::InputSnapshot>,
    same_cdi: Option<PreparedSameCdiInput>,
    fbneo: Option<FbneoSession>,
    mame: Option<PreparedMameInput>,
    puae: Option<PreparedPuaeInput>,
    #[cfg(target_os = "linux")]
    stella: Option<PreparedStellaDetection>,
    pub description: String,
}

impl CalibratedLaunch {
    #[cfg(all(target_os = "linux", target_pointer_width = "64"))]
    fn spawn_fbneo_relative_frontend(
        &mut self,
        plan: &LaunchPlan,
        group: crate::controller_axis::relative::RelativeMouseGroup,
        topology: crate::controller_axis::relative_topology::RelativeTopologyGuard,
        cancel: &std::sync::atomic::AtomicBool,
    ) -> Result<std::process::Child> {
        check_preparation_cancel(cancel)?;
        self.check_launch_inputs()?;
        let input = self
            .fbneo
            .as_ref()
            .context("Missing retained FBNeo session")?;
        ensure!(
            input.relative_launch_plan.as_ref() == Some(plan) && input.relative_bridge.is_none(),
            "FBNeo relative launch differs from its prepared plan or already owns a bridge"
        );
        let mut child = crate::emulator::spawn_launch_plan_with_controller_pipes(plan)?;
        let attached = (|| -> Result<()> {
            check_preparation_cancel(cancel)?;
            ensure!(
                child.try_wait()?.is_none(),
                "FBNeo frontend exited before input handoff"
            );
            let input = self
                .fbneo
                .as_mut()
                .context("Missing retained FBNeo session")?;
            input.verify_inputs()?;
            let executable = std::fs::canonicalize(format!("/proc/{}/exe", child.id()))?;
            ensure!(
                executable == input.inspection.launch_identity.program.canonicalize()?
                    && lunchbox_controller_probe::file_hash(&executable)?
                        == input.inspection.runtime_sha256,
                "FBNeo command child differs from the inspected frontend"
            );
            let commands =
                crate::controller_axis::relative_command::RelativeCommandChannel::take_from_child(
                    &mut child,
                )?;
            let bridge =
                crate::controller_axis::relative::RelativeMouseBridge::start_with_command_channel(
                    group, topology, commands,
                )?;
            input.relative_bridge = Some(bridge);
            Ok(())
        })();
        if let Err(error) = attached {
            let _ = child.kill();
            let _ = child.wait();
            return Err(error.context("Attaching FBNeo relative transport to the owned frontend"));
        }
        Ok(child)
    }

    #[cfg(all(target_os = "linux", target_pointer_width = "64"))]
    fn prepare_fbneo_relative_sources(
        &mut self,
        plan: &mut LaunchPlan,
        cancel: &std::sync::atomic::AtomicBool,
    ) -> Result<()> {
        use std::io::Write;
        check_preparation_cancel(cancel)?;
        self.check_launch_inputs()?;
        let input = self
            .fbneo
            .as_ref()
            .context("Missing retained FBNeo session")?;
        ensure!(
            input.launch_plan == *plan
                && input.relative_launch_plan.is_none()
                && input.relative_pending.is_none(),
            "FBNeo relative startup plan changed or is already prepared"
        );
        ensure!(
            !input.inspection.relative_sources.is_empty(),
            "No fresh-bound FBNeo relative sources"
        );
        let mut group = crate::controller_axis::relative::RelativeMouseGroup::open_saved(
            &input.inspection.relative_sources,
        )?;
        let deadline = std::time::Instant::now() + std::time::Duration::from_secs(2);
        loop {
            check_preparation_cancel(cancel)?;
            if group.endpoints()?.is_some() {
                break;
            }
            ensure!(
                std::time::Instant::now() < deadline,
                "FBNeo relative endpoints did not become ready"
            );
            std::thread::sleep(std::time::Duration::from_millis(10));
        }
        let mut topology =
            crate::controller_axis::relative_topology::RelativeTopologyGuard::subscribe()?;
        topology.verify()?;
        let root = self
            ._directory
            .as_ref()
            .context("Missing private FBNeo launch directory")?
            .path();
        let path = root.join("relative-bootstrap.cfg");
        let mut staged = plan.clone();
        attach_config(
            &mut staged,
            &EmulatorExecutable::Native(plan.program.clone()),
            &path,
        )?;
        staged.arguments.push("--verbose".into());
        let mut file = std::fs::OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&path)?;
        file.write_all(
            crate::controller_axis::relative_frontend::relative_bootstrap_config().as_bytes(),
        )?;
        file.sync_all()?;
        check_preparation_cancel(cancel)?;
        let input = self
            .fbneo
            .as_mut()
            .context("Missing retained FBNeo session")?;
        input
            .files
            .push((path.clone(), lunchbox_controller_probe::file_hash(&path)?));
        input.relative_launch_plan = Some(staged.clone());
        input.relative_pending = Some((group, topology));
        *plan = staged;
        Ok(())
    }

    #[cfg(all(target_os = "linux", target_pointer_width = "64"))]
    fn prepare_mame_relative_sources(
        &mut self,
        sources: &[crate::controller_mame::RelativeSource],
        plan: &mut LaunchPlan,
        cancel: &std::sync::atomic::AtomicBool,
    ) -> Result<()> {
        check_preparation_cancel(cancel)?;
        let input = self
            .mame
            .as_ref()
            .context("Missing retained MAME session")?;
        ensure!(
            input.relative_pending.is_none() && input.relative_bridge.is_none(),
            "MAME relative sources are already owned"
        );
        ensure!(
            !input.inspection.relative_assignments.is_empty()
                || !input.inspection.relative_button_assignments.is_empty(),
            "No inspected relative assignments"
        );
        input.verify_inputs(&input.inspection.launch_identity)?;
        let sources = crate::controller_mame::relative_buttons::prepare_sources(
            &input.inspection.relative_assignments,
            &input.inspection.relative_button_assignments,
            sources,
        )?;
        let mut group = crate::controller_axis::relative::RelativeMouseGroup::open_saved(&sources)?;
        let deadline = std::time::Instant::now() + std::time::Duration::from_secs(2);
        loop {
            check_preparation_cancel(cancel)?;
            if group.endpoints()?.is_some() {
                break;
            }
            ensure!(
                std::time::Instant::now() < deadline,
                "Relative virtual endpoints did not become ready"
            );
            std::thread::sleep(std::time::Duration::from_millis(10));
        }
        let mut topology =
            crate::controller_axis::relative_topology::RelativeTopologyGuard::subscribe()?;
        topology.verify()?;
        self.prepare_mame_relative_bootstrap(plan)?;
        check_preparation_cancel(cancel)?;
        self.mame
            .as_mut()
            .context("Missing retained MAME session")?
            .relative_pending = Some((group, topology));
        Ok(())
    }

    /// Choose stdio ownership only when a prepared relative group requires it.
    pub(crate) fn spawn_frontend(
        &mut self,
        plan: &LaunchPlan,
        cancel: &std::sync::atomic::AtomicBool,
    ) -> Result<std::process::Child> {
        check_preparation_cancel(cancel)?;
        #[cfg(target_os = "linux")]
        if let Some(native) = &mut self.snes9x_native {
            return native.spawn(plan, cancel);
        }
        #[cfg(target_os = "linux")]
        if let Some(native) = &mut self.fceux_native {
            return native.spawn(plan, cancel);
        }
        #[cfg(target_os = "linux")]
        if let Some(native) = &mut self.sameboy_native {
            return native.spawn(plan, cancel);
        }
        #[cfg(target_os = "linux")]
        if let Some(native) = &mut self.bsnes_native {
            return native.spawn(plan, cancel);
        }
        #[cfg(target_os = "linux")]
        if let Some(native) = &mut self.mednafen_native {
            return native.spawn(plan, cancel);
        }
        #[cfg(target_os = "linux")]
        if let Some(native) = &mut self.mame_native {
            return native.spawn(plan, cancel);
        }
        #[cfg(target_os = "linux")]
        if let Some(native) = &mut self.flycast_native {
            return native.spawn(plan, cancel);
        }
        #[cfg(target_os = "linux")]
        if let Some(native) = &mut self.pcsx2_native {
            return native.spawn(plan, cancel);
        }
        #[cfg(target_os = "linux")]
        if let Some(native) = &mut self.rpcs3_native {
            return native.spawn(plan, cancel);
        }
        #[cfg(target_os = "linux")]
        if let Some(native) = &mut self.melonds_native {
            return native.spawn(plan, cancel);
        }
        #[cfg(target_os = "linux")]
        if let Some(native) = &mut self.dolphin_native {
            return native.spawn(plan, cancel);
        }
        #[cfg(target_os = "linux")]
        if let Some(ppsspp) = &mut self.ppsspp {
            return ppsspp.spawn(plan, cancel);
        }
        #[cfg(target_os = "linux")]
        if let Some(mgba) = &mut self.mgba {
            return mgba.spawn(plan, cancel);
        }
        #[cfg(target_os = "linux")]
        if let Some(duckstation) = &mut self.duckstation {
            return duckstation.spawn(plan, cancel);
        }
        #[cfg(all(target_os = "linux", target_pointer_width = "64"))]
        if let Some((group, topology)) = self
            .mame
            .as_mut()
            .and_then(|input| input.relative_pending.take())
        {
            return self.spawn_mame_relative_frontend(plan, group, topology, cancel);
        }
        #[cfg(all(target_os = "linux", target_pointer_width = "64"))]
        if let Some((group, topology)) = self
            .fbneo
            .as_mut()
            .and_then(|input| input.relative_pending.take())
        {
            return self.spawn_fbneo_relative_frontend(plan, group, topology, cancel);
        }
        #[cfg(all(target_os = "linux", target_pointer_width = "64"))]
        ensure!(
            self.fbneo
                .as_ref()
                .is_none_or(|input| input.relative_launch_plan.is_none()
                    && input.inspection.relative_sources.is_empty()),
            "FBNeo relative startup is missing or already consumed; prepare a new launch session"
        );
        #[cfg(all(target_os = "linux", target_pointer_width = "64"))]
        ensure!(
            self.mame
                .as_ref()
                .is_none_or(|input| input.relative_launch_plan.is_none()
                    && input.inspection.relative_assignments.is_empty()
                    && input.inspection.relative_button_assignments.is_empty()),
            "MAME relative startup is missing or already consumed; prepare a new launch session"
        );
        crate::emulator::spawn_launch_plan(plan)
    }

    /// Stage last, after ordinary native/frontend routing is complete. The
    /// retained exact plan prevents subsequent argument changes from silently
    /// invalidating the disabled-mouse startup contract.
    #[cfg(all(target_os = "linux", target_pointer_width = "64"))]
    pub(crate) fn prepare_mame_relative_bootstrap(&mut self, plan: &mut LaunchPlan) -> Result<()> {
        use std::io::Write;
        self.check_launch_inputs()?;
        let input = self
            .mame
            .as_mut()
            .context("Missing retained MAME session")?;
        ensure!(
            input.relative_launch_plan.is_none() && input.relative_bridge.is_none(),
            "MAME relative startup is already prepared"
        );
        ensure!(
            input.inspection.request.inspect_mouse,
            "MAME relative startup requires mouse inspection"
        );
        let content = plan
            .retroarch_content
            .as_ref()
            .context("Missing final MAME content")?;
        ensure!(
            plan.program.canonicalize()? == input.inspection.request.retroarch
                && content.core.canonicalize()? == input.inspection.request.core
                && content.content == input.command_path,
            "Relative bootstrap does not target the retained MAME session"
        );
        let path = input.root.join("relative-bootstrap.cfg");
        let config = crate::controller_axis::relative_frontend::relative_bootstrap_config();
        let mut staged = plan.clone();
        attach_config(
            &mut staged,
            &EmulatorExecutable::Native(input.inspection.request.retroarch.clone()),
            &path,
        )?;
        // The owned stderr startup table is required by the route handshake.
        if !staged.arguments.iter().any(|arg| arg == "--verbose") {
            staged.arguments.push("--verbose".into());
        }
        let mut file = std::fs::OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&path)?;
        file.write_all(config.as_bytes())?;
        file.sync_all()?;
        input
            .files
            .push((path.clone(), lunchbox_controller_probe::file_hash(&path)?));
        input.relative_launch_plan = Some(staged.clone());
        *plan = staged;
        Ok(())
    }

    /// Opt-in spawn plus pipe/bridge handoff. The normal launch worker still
    /// owns the returned child and must apply its existing health/wait loop.
    #[cfg(all(target_os = "linux", target_pointer_width = "64"))]
    pub(crate) fn spawn_mame_relative_frontend(
        &mut self,
        plan: &LaunchPlan,
        group: crate::controller_axis::relative::RelativeMouseGroup,
        topology: crate::controller_axis::relative_topology::RelativeTopologyGuard,
        cancel: &std::sync::atomic::AtomicBool,
    ) -> Result<std::process::Child> {
        check_preparation_cancel(cancel)?;
        self.check_launch_inputs()?;
        ensure!(
            self.mame
                .as_ref()
                .and_then(|input| input.relative_launch_plan.as_ref())
                == Some(plan),
            "MAME relative launch differs from its prepared bootstrap plan"
        );
        let mut child = crate::emulator::spawn_launch_plan_with_controller_pipes(plan)?;
        let attached = check_preparation_cancel(cancel)
            .and_then(|()| self.attach_mame_relative_transport(&mut child, group, topology));
        if let Err(error) = attached {
            let _ = child.kill();
            let _ = child.wait();
            return Err(error.context("Attaching relative transport to the owned frontend"));
        }
        Ok(child)
    }

    /// Post-spawn ownership handoff. Takes the startup log from this Child's
    /// stderr; the caller retains the Child in the normal
    /// health/cancellation loop. This does not select or persist relative inputs.
    #[cfg(all(target_os = "linux", target_pointer_width = "64"))]
    pub(crate) fn attach_mame_relative_transport(
        &mut self,
        child: &mut std::process::Child,
        group: crate::controller_axis::relative::RelativeMouseGroup,
        topology: crate::controller_axis::relative_topology::RelativeTopologyGuard,
    ) -> Result<()> {
        let input = self
            .mame
            .as_mut()
            .context("No retained MAME session for relative transport")?;
        ensure!(
            input.relative_bridge.is_none(),
            "MAME already owns a relative transport"
        );
        ensure!(
            input.inspection.request.inspect_mouse,
            "MAME session was not inspected with native mouse routing enabled"
        );
        input.inspection.fields.mouse_routes()?;
        crate::controller_mame::validate_inspection_options(
            &input.inspection.request.core_options,
            true,
        )?;
        input.verify_inputs(&input.inspection.launch_identity)?;
        ensure!(
            child.try_wait()?.is_none(),
            "MAME frontend has already exited"
        );
        let executable = std::fs::canonicalize(format!("/proc/{}/exe", child.id()))?;
        ensure!(
            executable == input.inspection.request.retroarch,
            "Relative command child differs from the inspected MAME frontend"
        );
        let commands =
            crate::controller_axis::relative_command::RelativeCommandChannel::take_from_child(
                child,
            )?;
        let bridge =
            crate::controller_axis::relative::RelativeMouseBridge::start_with_command_channel(
                group, topology, commands,
            )?;
        input.relative_bridge = Some(bridge);
        Ok(())
    }

    /// Route handshake status, not proof that the game or devices are playable.
    pub(crate) fn controller_startup_ready(&self) -> Result<bool> {
        self.check_health()?;
        #[cfg(all(target_os = "linux", target_pointer_width = "64"))]
        if let Some(bridge) = self
            .mame
            .as_ref()
            .and_then(|input| input.relative_bridge.as_ref())
        {
            return bridge.routing_confirmed();
        }
        #[cfg(all(target_os = "linux", target_pointer_width = "64"))]
        if let Some(input) = &self.fbneo {
            if let Some(bridge) = &input.relative_bridge {
                return bridge.routing_confirmed();
            }
            if input.relative_launch_plan.is_some() {
                return Ok(false);
            }
        }
        Ok(true)
    }

    /// Attach MAME's native layer after physical frontend calibration. Save and
    /// state directories must already be resolved for the original content, not
    /// recomputed from the temporary session.cmd basename.
    pub(crate) fn attach_mame_input(
        &mut self,
        mut input: PreparedMameInput,
        plan: &mut LaunchPlan,
        frontend_save: &Path,
        frontend_state: &Path,
    ) -> Result<()> {
        input.verify_inputs(&input.inspection.launch_identity)?;
        let original = input
            .inspection
            .launch_identity
            .retroarch_content
            .as_ref()
            .context("Missing original MAME content")?;
        ensure!(
            plan.retroarch_content.as_ref() == Some(original)
                && plan.program.canonicalize()? == input.inspection.request.retroarch,
            "MAME launch identity changed before native configuration attachment"
        );
        for directory in [frontend_save, frontend_state] {
            ensure!(
                directory.is_absolute()
                    && directory.is_dir()
                    && directory.canonicalize()? == directory
                    && !directory.starts_with(&input.root),
                "MAME frontend persistence must use resolved directories outside private staging"
            );
        }
        let private_config = input.root.join("frontend-config");
        std::fs::create_dir(&private_config)?;
        let mut overlay = String::from(
            "auto_overrides_enable = \"false\"\nauto_remaps_enable = \"false\"\nconfig_save_on_exit = \"false\"\nremap_save_on_exit = \"false\"\ngame_specific_options = \"false\"\nglobal_core_options = \"false\"\nsavefiles_in_content_dir = \"false\"\nsavestates_in_content_dir = \"false\"\nsort_savefiles_enable = \"false\"\nsort_savefiles_by_content_enable = \"false\"\nsort_savestates_enable = \"false\"\nsort_savestates_by_content_enable = \"false\"\nsavestate_auto_load = \"false\"\nsavestate_auto_save = \"false\"\n",
        );
        for (key, path) in [
            ("system_directory", input.root.join("system")),
            ("core_options_path", input.root.join("core-options.cfg")),
            ("rgui_config_directory", private_config),
            ("savefile_directory", frontend_save.to_path_buf()),
            ("savestate_directory", frontend_state.to_path_buf()),
        ] {
            let value = path.to_str().context("MAME frontend path is not UTF-8")?;
            ensure!(
                !value.chars().any(|c| c.is_control() || "\"\\".contains(c)),
                "MAME frontend path cannot be represented in configuration"
            );
            overlay.push_str(&format!("{key} = \"{value}\"\n"));
        }
        let path = input.root.join("frontend.cfg");
        std::fs::write(&path, overlay)?;
        input
            .files
            .push((path.clone(), lunchbox_controller_probe::file_hash(&path)?));
        input.frontend_persistent = vec![frontend_save.to_path_buf(), frontend_state.to_path_buf()];
        let mut staged = plan.clone();
        let positions: Vec<_> = staged
            .arguments
            .iter()
            .enumerate()
            .filter_map(|(index, arg)| (arg == original.content.as_os_str()).then_some(index))
            .collect();
        ensure!(
            positions.len() == 1,
            "Original MAME content argument is not unique"
        );
        staged.arguments[positions[0]] = input.command_path.clone().into_os_string();
        staged.retroarch_content.as_mut().unwrap().content = input.command_path.clone();
        attach_config(
            &mut staged,
            &EmulatorExecutable::Native(input.inspection.request.retroarch.clone()),
            &path,
        )?;
        self.retain_mame_input(input, &staged)?;
        *plan = staged;
        Ok(())
    }

    /// Transfer native MAME file ownership to the same object that keeps the
    /// calibrated physical transports alive. Call after the launch adapter has
    /// routed the final command and configuration, before spawning the child.
    pub(crate) fn retain_mame_input(
        &mut self,
        input: PreparedMameInput,
        staged_plan: &LaunchPlan,
    ) -> Result<()> {
        ensure!(
            self.mame.is_none()
                && self.fbneo.is_none()
                && self.bizhawk.is_none()
                && self.same_cdi.is_none(),
            "Conflicting native controller session owner"
        );
        input.verify_inputs(&input.inspection.launch_identity)?;
        let content = staged_plan
            .retroarch_content
            .as_ref()
            .context("MAME session requires final core/content identity")?;
        ensure!(
            staged_plan.program.canonicalize()? == input.inspection.request.retroarch
                && content.core.canonicalize()? == input.inspection.request.core
                && content.content == input.command_path,
            "Final MAME launch does not select the staged runtime and command"
        );
        ensure!(
            staged_plan
                .arguments
                .iter()
                .filter(|arg| *arg == input.command_path.as_os_str())
                .count()
                == 1,
            "MAME session command must occur exactly once in final arguments"
        );
        self.check_health()?;
        self.mame = Some(input);
        Ok(())
    }

    /// Native config lifetime integrates with the same retained launch session
    /// used by the GUI. Runtime device monitoring must be attached by the native
    /// launch adapter before exposing it as an available coverage mode.
    #[cfg(target_os = "linux")]
    pub(crate) fn from_bizhawk(
        config: crate::controller_bizhawk::PreparedConfig,
        topology: crate::controller_bizhawk_guard::InputTopology,
        description: String,
    ) -> Self {
        Self::from_bizhawk_owner(
            PreparedBizhawkLaunch::Configuration(config),
            topology,
            description,
        )
    }

    /// Retain an independently checked cartridge handoff through the GUI's
    /// normal launch lifetime. The caller must first validate the final plan
    /// against the handoff and establish the supported loaded-core/build policy.
    #[cfg(target_os = "linux")]
    pub(crate) fn from_bizhawk_capture(
        handoff: crate::controller_bizhawk::digital_session::PreparedCartridgeHandoff,
        topology: crate::controller_bizhawk_guard::InputTopology,
        description: String,
    ) -> Result<Self> {
        let session = Self::from_bizhawk_owner(
            PreparedBizhawkLaunch::Captured(handoff),
            topology,
            description,
        );
        session.check_launch_inputs()?;
        Ok(session)
    }

    #[cfg(target_os = "linux")]
    fn from_bizhawk_owner(
        owner: PreparedBizhawkLaunch,
        topology: crate::controller_bizhawk_guard::InputTopology,
        description: String,
    ) -> Self {
        Self {
            _directory: None,
            bizhawk: Some(owner),
            mgba: None,
            #[cfg(target_os = "linux")]
            dolphin_native: None,
            #[cfg(target_os = "linux")]
            snes9x_native: None,
            #[cfg(target_os = "linux")]
            fceux_native: None,
            #[cfg(target_os = "linux")]
            sameboy_native: None,
            bsnes_native: None,
            #[cfg(target_os = "linux")]
            mednafen_native: None,
            #[cfg(target_os = "linux")]
            mame_native: None,
            #[cfg(target_os = "linux")]
            flycast_native: None,
            #[cfg(target_os = "linux")]
            pcsx2_native: None,
            #[cfg(target_os = "linux")]
            rpcs3_native: None,
            #[cfg(target_os = "linux")]
            melonds_native: None,
            duckstation: None,
            ppsspp: None,
            bizhawk_topology: Some(topology),
            transports: Vec::new(),
            crocods: None,
            ep128emu: None,
            hatari: None,
            simcp: None,
            steemsse: None,
            scummvm: None,
            dolphin: None,
            same_cdi: None,
            fbneo: None,
            mame: None,
            puae: None,
            #[cfg(target_os = "linux")]
            stella: None,
            description,
        }
    }

    pub fn check_launch_inputs(&self) -> Result<()> {
        #[cfg(target_os = "linux")]
        if let Some(native) = &self.snes9x_native {
            native.verify(&std::sync::atomic::AtomicBool::new(false))?;
        }
        #[cfg(target_os = "linux")]
        if let Some(native) = &self.fceux_native {
            native.verify(&std::sync::atomic::AtomicBool::new(false))?;
        }
        #[cfg(target_os = "linux")]
        if let Some(native) = &self.sameboy_native {
            native.verify(&std::sync::atomic::AtomicBool::new(false))?;
        }
        #[cfg(target_os = "linux")]
        if let Some(native) = &self.bsnes_native {
            native.verify(&std::sync::atomic::AtomicBool::new(false))?;
        }
        #[cfg(target_os = "linux")]
        if let Some(native) = &self.mednafen_native {
            native.verify(&std::sync::atomic::AtomicBool::new(false))?;
        }
        #[cfg(target_os = "linux")]
        if let Some(native) = &self.mame_native {
            native.verify(&std::sync::atomic::AtomicBool::new(false))?;
        }
        #[cfg(target_os = "linux")]
        if let Some(native) = &self.flycast_native {
            native.verify(&std::sync::atomic::AtomicBool::new(false))?;
        }
        #[cfg(target_os = "linux")]
        if let Some(native) = &self.pcsx2_native {
            native.verify(&std::sync::atomic::AtomicBool::new(false))?;
        }
        #[cfg(target_os = "linux")]
        if let Some(native) = &self.rpcs3_native {
            native.verify(&std::sync::atomic::AtomicBool::new(false))?;
        }
        #[cfg(target_os = "linux")]
        if let Some(native) = &self.melonds_native {
            native.verify(&std::sync::atomic::AtomicBool::new(false))?;
        }
        #[cfg(target_os = "linux")]
        if let Some(native) = &self.dolphin_native {
            native.verify()?;
        }
        self.check_health()?;
        #[cfg(target_os = "linux")]
        if let Some(mgba) = &self.mgba {
            mgba.verify()?;
        }
        #[cfg(target_os = "linux")]
        if let Some(ppsspp) = &self.ppsspp {
            ppsspp.verify()?;
        }
        #[cfg(target_os = "linux")]
        if let Some(duckstation) = &self.duckstation {
            duckstation.verify()?;
        }
        if let Some(fbneo) = &self.fbneo {
            #[cfg(all(target_os = "linux", target_pointer_width = "64"))]
            if let Some(bridge) = &fbneo.relative_bridge {
                bridge.check_health()?;
            }
            fbneo.verify_inputs()?;
        }
        if let Some(mame) = &self.mame {
            mame.verify_inputs(&mame.inspection.launch_identity)?;
        }
        if let Some(bizhawk) = &self.bizhawk {
            bizhawk.verify()?;
        }
        if let Some(ep128emu) = &self.ep128emu {
            ep128emu.verify()?;
        }
        if let Some(hatari) = &self.hatari {
            hatari.verify()?;
        }
        if let Some(simcp) = &self.simcp {
            simcp.verify()?;
        }
        if let Some(steemsse) = &self.steemsse {
            steemsse.verify()?;
        }
        if let Some(scummvm) = &self.scummvm {
            scummvm.verify()?;
        }
        if let Some(dolphin) = &self.dolphin {
            dolphin.verify()?;
        }
        if let Some(same_cdi) = &self.same_cdi {
            same_cdi.verify()?;
        }
        if let Some(crocods) = &self.crocods {
            crocods.verify()?;
        }
        if let Some(puae) = &self.puae {
            puae.inputs.verify()?;
            ensure!(
                puae.save_directory.is_dir(),
                "PUAE save directory disappeared during launch preparation"
            );
        }
        #[cfg(target_os = "linux")]
        if let Some(stella) = &self.stella {
            stella.context.verify_inputs()?;
        }
        Ok(())
    }
    pub fn check_health(&self) -> Result<()> {
        #[cfg(target_os = "linux")]
        if let Some(native) = &self.snes9x_native {
            native.check_health()?;
        }
        #[cfg(target_os = "linux")]
        if let Some(native) = &self.fceux_native {
            native.check_health()?;
        }
        #[cfg(target_os = "linux")]
        if let Some(native) = &self.sameboy_native {
            native.check_health()?;
        }
        #[cfg(target_os = "linux")]
        if let Some(native) = &self.bsnes_native {
            native.check_health()?;
        }
        #[cfg(target_os = "linux")]
        if let Some(native) = &self.mednafen_native {
            native.check_health()?;
        }
        #[cfg(target_os = "linux")]
        if let Some(native) = &self.mame_native {
            native.check_health()?;
        }
        #[cfg(target_os = "linux")]
        if let Some(native) = &self.flycast_native {
            native.check_health()?;
        }
        #[cfg(target_os = "linux")]
        if let Some(native) = &self.pcsx2_native {
            native.check_health()?;
        }
        #[cfg(target_os = "linux")]
        if let Some(native) = &self.rpcs3_native {
            native.check_health()?;
        }
        #[cfg(target_os = "linux")]
        if let Some(native) = &self.melonds_native {
            native.check_health()?;
        }
        #[cfg(target_os = "linux")]
        if let Some(native) = &self.dolphin_native {
            native.session.check_health()?;
        }
        #[cfg(target_os = "linux")]
        if let Some(mgba) = &self.mgba {
            mgba.check_health()?;
        }
        #[cfg(target_os = "linux")]
        if let Some(ppsspp) = &self.ppsspp {
            ppsspp.check_health()?;
        }
        #[cfg(all(target_os = "linux", target_pointer_width = "64"))]
        if let Some(bridge) = self
            .mame
            .as_ref()
            .and_then(|input| input.relative_bridge.as_ref())
        {
            bridge.check_health()?;
        }
        if let Some(fbneo) = &self.fbneo {
            for player in fbneo.players.values() {
                player.check_health()?;
            }
        }
        #[cfg(target_os = "linux")]
        if let Some(topology) = &self.bizhawk_topology {
            topology.verify()?;
        }
        for transport in &self.transports {
            transport.check_health()?;
        }
        Ok(())
    }
}

struct PlayerTransport {
    numbering: JoydevMap,
    pressure_codes: std::collections::BTreeSet<u16>,
    #[cfg(all(target_os = "linux", target_pointer_width = "64"))]
    bridge: Option<crate::controller_axis::GamepadBridge>,
}

impl PlayerTransport {
    fn check_health(&self) -> Result<()> {
        #[cfg(all(target_os = "linux", target_pointer_width = "64"))]
        if let Some(bridge) = &self.bridge {
            bridge.check_health()?;
        }
        Ok(())
    }
}

const OUTPUTS: &[(&str, &str)] = &[
    ("South", "b"),
    ("East", "a"),
    ("West", "y"),
    ("North", "x"),
    ("Select", "select"),
    ("Start", "start"),
    ("DPadUp", "up"),
    ("DPadDown", "down"),
    ("DPadLeft", "left"),
    ("DPadRight", "right"),
    ("LeftBumper", "l"),
    ("RightBumper", "r"),
    ("LeftTrigger", "l2"),
    ("RightTrigger", "r2"),
    ("LeftStick", "l3"),
    ("RightStick", "r3"),
    ("LeftStickLeft", "l_x_minus"),
    ("LeftStickRight", "l_x_plus"),
    ("LeftStickUp", "l_y_minus"),
    ("LeftStickDown", "l_y_plus"),
    ("RightStickLeft", "r_x_minus"),
    ("RightStickRight", "r_x_plus"),
    ("RightStickUp", "r_y_minus"),
    ("RightStickDown", "r_y_plus"),
];

/// Select by the actual launched core and platform, never emulator display name.
pub fn contract(core: &str, platform: &str) -> Option<&'static EmulatorProfile> {
    catalog().launch_profile(core, platform)
}

pub fn supports_profile(profile: &EmulatorProfile) -> bool {
    (cfg!(target_os = "linux") && profile.retroarch_launch.is_some())
        || profile.transport == "ares-settings"
}

pub fn selection_key(core: &str, platform: &str) -> String {
    serde_json::to_string(&[core, &platform.trim().to_lowercase()])
        .expect("string tuple serializes")
}

pub(crate) struct FbneoLaunchInspection {
    launch_identity: LaunchPlan,
    runtime_sha256: String,
    base_config_path: std::path::PathBuf,
    base_config_text: String,
    pub(crate) request: lunchbox_controller_probe::content_inspection::Request,
    pub(crate) report: lunchbox_controller_probe::libretro_input::ContentControllerReport,
    pub(crate) targets: Vec<crate::controller_fbneo::MappingTarget>,
    relative_sources:
        BTreeMap<u8, crate::controller_axis::relative_settings::RelativeDeviceSettings>,
    /// Prepared frontend channels, separate from the immutable native report.
    keyboard_targets: Option<Vec<crate::controller_fbneo::MappingTarget>>,
    keyboard_remap: Option<String>,
    keyboard_passthrough_port: Option<u32>,
    absolute_sources:
        BTreeMap<u32, crate::controller_axis::absolute_settings::AbsoluteDeviceSettings>,
    /// The selected RetroArch option file, before restricting the request to
    /// FBNeo's namespace. Keep it for the eventual launch snapshot writer.
    pub(crate) baseline_options: String,
}

impl FbneoLaunchInspection {
    fn gamepad_targets(&self) -> Vec<crate::controller_fbneo::MappingTarget> {
        self.keyboard_targets
            .as_ref()
            .unwrap_or(&self.targets)
            .iter()
            .filter(|target| {
                !(target.address.device == 2
                    && self
                        .relative_sources
                        .contains_key(&((target.address.port + 1) as u8)))
                    && !(self.absolute_sources.contains_key(&target.address.port)
                        && crate::controller_fbneo::arcade_aim_address(&target.address))
            })
            .cloned()
            .collect()
    }

    /// Bind source settings only after the retained fresh report agrees with
    /// the saved contract. This opens no input device and assigns no mouse index.
    fn bind_saved_relative_sources(
        &mut self,
        saved: &crate::controller_fbneo::NativeLaunchSettings,
    ) -> Result<()> {
        saved.validate()?;
        saved.validate_review(&self.report)?;
        ensure!(
            self.relative_sources.is_empty(),
            "FBNeo relative sources already retained"
        );
        let mut prepared = BTreeMap::new();
        for source in &saved.relative_sources {
            let required: Vec<_> = self
                .targets
                .iter()
                .filter(|target| target.address.port == source.port && target.address.device == 2)
                .map(|target| target.address.clone())
                .collect();
            let device = crate::controller_fbneo::prepare_relative_port(&required, &source.device)?;
            ensure!(
                prepared
                    .insert(u8::try_from(source.port + 1)?, device)
                    .is_none(),
                "Duplicate fresh FBNeo relative source port"
            );
        }
        crate::controller_axis::relative_settings::validate_devices(
            &prepared.values().cloned().collect::<Vec<_>>(),
        )?;
        ensure!(
            prepared == saved.prepared_relative_sources()?,
            "Fresh FBNeo relative requirements differ from the reviewed preparation"
        );
        self.relative_sources = prepared;
        self.absolute_sources = saved.prepared_absolute_sources()?;
        self.keyboard_passthrough_port = saved.prepared_keyboard_passthrough()?;
        if self.keyboard_passthrough_port.is_some() {
            self.keyboard_targets = Some(saved.gamepad_targets()?);
        }
        if let Some((_, _, _, remap)) = saved.keyboard_plan()? {
            self.keyboard_targets = Some(saved.gamepad_targets()?);
            self.keyboard_remap = Some(remap);
        }
        Ok(())
    }
}

struct FbneoSession {
    // Drop the running bridge before the remaining session inputs.
    #[cfg(all(target_os = "linux", target_pointer_width = "64"))]
    relative_bridge: Option<crate::controller_axis::relative::RelativeMouseBridge>,
    #[cfg(all(target_os = "linux", target_pointer_width = "64"))]
    relative_pending: Option<(
        crate::controller_axis::relative::RelativeMouseGroup,
        crate::controller_axis::relative_topology::RelativeTopologyGuard,
    )>,
    #[cfg(all(target_os = "linux", target_pointer_width = "64"))]
    relative_launch_plan: Option<LaunchPlan>,
    launch_plan: LaunchPlan,
    inspection: FbneoLaunchInspection,
    players: BTreeMap<u32, PreparedFbneoPlayer>,
    files: Vec<(std::path::PathBuf, String)>,
    persistent_directories: Vec<(std::path::PathBuf, std::path::PathBuf)>,
}

impl FbneoSession {
    fn verify_inputs(&self) -> Result<()> {
        ensure!(
            lunchbox_controller_probe::file_hash(&self.inspection.launch_identity.program)?
                == self.inspection.runtime_sha256,
            "RetroArch executable changed after FBNeo inspection"
        );
        let request = &self.inspection.request;
        request.parse_report(&serde_json::to_vec(&self.inspection.report)?)?;
        ensure!(
            read_optional(&self.inspection.base_config_path)? == self.inspection.base_config_text,
            "RetroArch base configuration changed after FBNeo inspection"
        );
        for (path, expected) in &self.files {
            ensure!(
                lunchbox_controller_probe::file_hash(path)? == *expected,
                "Private FBNeo launch configuration changed"
            );
        }
        for (path, canonical) in &self.persistent_directories {
            ensure!(
                path.is_dir() && path.canonicalize()? == *canonical,
                "FBNeo persistent save/state directory changed during preparation"
            );
        }
        Ok(())
    }
}

/// Attach a complete fragment set atomically. Ordinary sessions prevent remap
/// loading; explicit keyboard plans stage a single retained private remap with
/// identity gamepad routes. External adapters must be resolved first.
pub(crate) fn attach_fbneo_session(
    option: &RomEmulatorOption,
    plan: &mut LaunchPlan,
    inspection: FbneoLaunchInspection,
    players: BTreeMap<u32, PreparedFbneoPlayer>,
    cancel: &std::sync::atomic::AtomicBool,
) -> Result<CalibratedLaunch> {
    check_preparation_cancel(cancel)?;
    ensure!(
        cfg!(all(target_os = "linux", target_pointer_width = "64")),
        "FBNeo normalized sessions currently require 64-bit Linux"
    );
    ensure!(
        *plan == inspection.launch_identity,
        "FBNeo launch changed after inspection"
    );
    let represented_ports: std::collections::BTreeSet<_> = players
        .keys()
        .copied()
        .chain(inspection.keyboard_passthrough_port)
        .chain(
            inspection
                .relative_sources
                .keys()
                .map(|player| u32::from(*player) - 1),
        )
        .collect();
    ensure!(
        !represented_ports.is_empty()
            && represented_ports
                .iter()
                .all(|port| (*port as usize) < crate::controller_fbneo::FRONTEND_PORTS)
            && represented_ports
                .iter()
                .eq(inspection.report.requested_devices.keys()),
        "FBNeo session needs prepared input for every inspected port"
    );
    ensure!(
        inspection
            .gamepad_targets()
            .iter()
            .all(|target| players.contains_key(&target.address.port)),
        "FBNeo relative-only port still has input requirements without a prepared gamepad"
    );
    ensure!(
        option.runtime_kind == EmulatorRuntimeKind::RetroArch
            && option.core_name == "fbneo"
            && matches!(&option.executable, EmulatorExecutable::Native(_)),
        "FBNeo session requires its inspected native RetroArch runtime"
    );
    for player in players.values() {
        ensure!(
            player.fragment.missing.is_empty() && player.fragment.external_targets.is_empty(),
            "FBNeo mapping still has missing assignments or unresolved external inputs"
        );
        player.check_health()?;
    }
    ensure!(
        !config_bool(&inspection.base_config_text, "savestate_auto_load", false)?,
        "FBNeo inspected controller setup requires a fresh start, not automatic state restoration"
    );
    let directory = tempfile::Builder::new()
        .prefix("lunchbox-fbneo-session-")
        .tempdir()?;
    let remaps = directory.path().join("remaps");
    std::fs::create_dir(&remaps)?;
    let system = directory.path().join("system");
    std::fs::create_dir(&system)?;
    let content_directory = directory.path().join("content");
    std::fs::create_dir(&content_directory)?;
    let staged_content = content_directory.join(
        inspection
            .request
            .content
            .file_name()
            .context("FBNeo content has no basename")?,
    );
    let (persistent_config, persistent_directories) = fbneo_persistent_directories(&inspection)?;
    let system_text = system
        .to_str()
        .context("FBNeo system directory is not UTF-8")?;
    ensure!(
        !system_text.contains(['"', '\\', '\n', '\r']),
        "FBNeo system directory cannot be encoded losslessly"
    );
    let mut system_files = Vec::new();
    if let Some(remap) = &inspection.keyboard_remap {
        let path = remaps.join(crate::controller_fbneo::keyboard::relative_remap_path(
            &inspection.report.core.core_name,
        )?);
        std::fs::create_dir(
            path.parent()
                .context("Private keyboard remap has no parent")?,
        )?;
        std::fs::write(&path, remap)?;
        let hash = lunchbox_controller_probe::file_hash(&path)?;
        system_files.push((path, hash));
    }
    for (group, source) in std::iter::once(("content", &inspection.request.content))
        .chain(
            inspection
                .request
                .content_dependencies
                .iter()
                .map(|path| ("content", path)),
        )
        .chain(
            inspection
                .request
                .system_files
                .iter()
                .map(|path| ("system", path)),
        )
    {
        check_preparation_cancel(cancel)?;
        let source = source.canonicalize()?;
        let name = source
            .file_name()
            .and_then(|name| name.to_str())
            .context("FBNeo staged input has no UTF-8 basename")?;
        let expected = inspection
            .report
            .staged_files
            .iter()
            .find(|file| file.filename == format!("{group}/{name}"))
            .context("FBNeo staged input was not present during inspection")?;
        let destination = directory.path().join(group).join(name);
        // Never overwrite a duplicate canonical basename. The copied bytes,
        // not only the current source, must match the inspected artifact.
        let mut input = std::fs::File::open(&source)?;
        ensure!(
            input.metadata()?.is_file(),
            "FBNeo staged input is not a regular file"
        );
        let mut output = std::fs::OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&destination)?;
        let mut bounded = input.by_ref().take(expected.bytes.saturating_add(1));
        let mut buffer = [0u8; 64 * 1024];
        let mut copied = 0u64;
        loop {
            check_preparation_cancel(cancel)?;
            let count = bounded.read(&mut buffer)?;
            if count == 0 {
                break;
            }
            std::io::Write::write_all(&mut output, &buffer[..count])?;
            copied += count as u64;
        }
        drop(output);
        check_preparation_cancel(cancel)?;
        let hash = lunchbox_controller_probe::file_hash(&destination)?;
        check_preparation_cancel(cancel)?;
        ensure!(
            copied == expected.bytes && hash.eq_ignore_ascii_case(&expected.sha256),
            "FBNeo input changed while preparing launch: {group}/{name}"
        );
        system_files.push((destination, hash));
    }
    let remaps_text = remaps
        .to_str()
        .context("FBNeo remap directory is not UTF-8")?;
    ensure!(
        !remaps_text.contains(['"', '\\', '\n', '\r']),
        "FBNeo remap directory cannot be encoded losslessly"
    );
    let mut options = String::new();
    for (key, value) in &inspection.report.effective_options {
        ensure!(
            !key.contains(['"', '\\', '\n', '\r', '=']) && !value.contains(['"', '\\', '\n', '\r']),
            "FBNeo option cannot be represented losslessly in RetroArch configuration"
        );
        options.push_str(&format!("{key} = \"{value}\"\n"));
    }
    let mut output = String::from(
        "# Lunchbox private FBNeo controller session.\ninput_joypad_driver = \"linuxraw\"\ninput_autodetect_enable = \"false\"\nauto_overrides_enable = \"false\"\nconfig_save_on_exit = \"false\"\nremap_save_on_exit = \"false\"\nsavestate_auto_load = \"false\"\n",
    );
    output.push_str(&fbneo_hotkey_isolation(
        inspection.keyboard_remap.is_some() || inspection.keyboard_passthrough_port.is_some(),
    ));
    if inspection.keyboard_passthrough_port.is_some() {
        // Pinned udev_input.c supplies RETRO_DEVICE_KEYBOARD from its frontend
        // keyboard state, without routing through player joypad channels. This
        // is deliberately not a claim of exclusive physical keyboard ownership.
        output.push_str("input_driver = \"udev\"\n");
    }
    if !inspection.relative_sources.is_empty() {
        output.push_str("menu_mouse_enable = \"false\"\nmenu_pointer_enable = \"false\"\n");
    }
    output.push_str(&persistent_config);
    output.push_str(&format!(
        "system_directory = \"{system_text}\"\nsystemfiles_in_content_dir = \"false\"\n"
    ));
    output.push_str(&write_options_file(&options, directory.path())?);
    output.push_str(&format!(
        "input_max_users = \"{}\"\ninput_remap_binds_enable = \"{}\"\nauto_remaps_enable = \"{}\"\n",
        crate::controller_fbneo::FRONTEND_PORTS,
        inspection.keyboard_remap.is_some(),
        inspection.keyboard_remap.is_some(),
    ));
    // configuration_set_defaults -> input_remapping_set_defaults establishes
    // keybind->id for buttons, j for axes and i for ports in a fresh process.
    // CLI validation excludes remap/config injection. Ordinary sessions keep
    // automatic remaps disabled; keyboard sessions load only our private file.
    output.push_str(&format!("input_remapping_directory = \"{remaps_text}\"\n"));
    if inspection.keyboard_remap.is_some() {
        // This directory contains only the retained core-level remap. No
        // inherited controller/content remap can precede it in the search.
        output.push_str("input_remap_sort_by_controller_enable = \"false\"\n");
    }
    output.push_str("input_analog_deadzone = \"0\"\ninput_analog_sensitivity = \"1\"\n");
    for player in players.values() {
        output.push_str(&player.fragment.config);
    }
    for port in 0..crate::controller_fbneo::FRONTEND_PORTS as u32 {
        if !players.contains_key(&port) {
            let device = inspection
                .report
                .requested_devices
                .get(&port)
                .copied()
                .unwrap_or(0);
            output.push_str(&empty_port_config(port as usize + 1, device));
            for (_, field) in OUTPUTS {
                output.push_str(&format!(
                    "input_player{}_{}_mbtn = \"nul\"\n",
                    port + 1,
                    field
                ));
            }
        }
    }
    let path = directory.path().join("controllers.cfg");
    crate::controller_launch_modes::validate_generated_modes(
        &output,
        emulator_arguments(plan, &option.executable)?,
        crate::controller_fbneo::FRONTEND_PORTS,
    )?;
    std::fs::write(&path, output)?;
    let mut files = [path.clone(), directory.path().join("core-options.cfg")]
        .into_iter()
        .map(|path| Ok((path.clone(), lunchbox_controller_probe::file_hash(&path)?)))
        .collect::<Result<Vec<_>>>()?;
    files.extend(system_files);
    let mut staged = plan.clone();
    let original = &plan
        .retroarch_content
        .as_ref()
        .context("FBNeo content identity disappeared")?
        .content;
    let positions = staged
        .arguments
        .iter()
        .enumerate()
        .filter_map(|(index, value)| (value == original.as_os_str()).then_some(index))
        .collect::<Vec<_>>();
    ensure!(positions.len() == 1, "FBNeo content argument is not unique");
    staged.arguments[positions[0]] = staged_content.clone().into_os_string();
    staged.retroarch_content.as_mut().unwrap().content = staged_content;
    attach_config(&mut staged, &option.executable, &path)?;
    let session = FbneoSession {
        #[cfg(all(target_os = "linux", target_pointer_width = "64"))]
        relative_bridge: None,
        #[cfg(all(target_os = "linux", target_pointer_width = "64"))]
        relative_pending: None,
        #[cfg(all(target_os = "linux", target_pointer_width = "64"))]
        relative_launch_plan: None,
        launch_plan: staged.clone(),
        inspection,
        players,
        files,
        persistent_directories,
    };
    session.verify_inputs()?;
    let result = CalibratedLaunch {
        #[cfg(target_os = "linux")]
        mgba: None,
        #[cfg(target_os = "linux")]
        dolphin_native: None,
        #[cfg(target_os = "linux")]
        snes9x_native: None,
        #[cfg(target_os = "linux")]
        fceux_native: None,
        #[cfg(target_os = "linux")]
        sameboy_native: None,
        bsnes_native: None,
        #[cfg(target_os = "linux")]
        mednafen_native: None,
        #[cfg(target_os = "linux")]
        mame_native: None,
        #[cfg(target_os = "linux")]
        flycast_native: None,
        #[cfg(target_os = "linux")]
        pcsx2_native: None,
        #[cfg(target_os = "linux")]
        rpcs3_native: None,
        #[cfg(target_os = "linux")]
        melonds_native: None,
        #[cfg(target_os = "linux")]
        ppsspp: None,
        #[cfg(target_os = "linux")]
        duckstation: None,
        _directory: Some(directory),
        bizhawk: None,
        #[cfg(target_os = "linux")]
        bizhawk_topology: None,
        transports: Vec::new(),
        crocods: None,
        ep128emu: None,
        hatari: None,
        simcp: None,
        steemsse: None,
        scummvm: None,
        dolphin: None,
        same_cdi: None,
        fbneo: Some(session),
        mame: None,
        puae: None,
        #[cfg(target_os = "linux")]
        stella: None,
        description: "Applied inspected per-game FBNeo controller bindings".into(),
    };
    result.check_launch_inputs()?;
    check_preparation_cancel(cancel)?;
    *plan = staged;
    Ok(result)
}

/// Keep gamepad gestures and mouse clicks out of frontend actions in this private
/// session. Existing keyboard shortcut assignments are left unchanged. Tokens
/// follow configuration.c input_config_bind_map at RetroArch 69a4f0ea.
fn fbneo_gamepad_hotkey_isolation() -> String {
    fbneo_hotkey_isolation(false)
}

fn fbneo_hotkey_isolation(isolate_keyboard: bool) -> String {
    let mut config = String::from(
        "input_menu_toggle_gamepad_combo = \"0\"\ninput_quit_gamepad_combo = \"0\"\ninput_turbo_enable = \"false\"\n",
    );
    for field in [
        "enable_hotkey",
        "menu_toggle",
        "exit_emulator",
        "close_content",
        "reset",
        "toggle_fast_forward",
        "hold_fast_forward",
        "toggle_slowmotion",
        "hold_slowmotion",
        "rewind",
        "pause_toggle",
        "frame_advance",
        "audio_mute",
        "volume_up",
        "volume_down",
        "load_state",
        "save_state",
        "state_slot_increase",
        "state_slot_decrease",
        "play_replay",
        "record_replay",
        "halt_replay",
        "save_replay_checkpoint",
        "prev_replay_checkpoint",
        "next_replay_checkpoint",
        "replay_slot_increase",
        "replay_slot_decrease",
        "disk_eject_toggle",
        "disk_next",
        "disk_prev",
        "shader_toggle",
        "shader_hold",
        "shader_next",
        "shader_prev",
        "cheat_toggle",
        "cheat_index_plus",
        "cheat_index_minus",
        "screenshot",
        "recording_toggle",
        "streaming_toggle",
        "turbo_fire_toggle",
        "grab_mouse_toggle",
        "game_focus_toggle",
        "toggle_fullscreen",
        "desktop_menu_toggle",
        "toggle_vrr_runloop",
        "runahead_toggle",
        "preempt_toggle",
        "fps_toggle",
        "toggle_statistics",
        "ai_service",
        "netplay_ping_toggle",
        "netplay_host_toggle",
        "netplay_game_watch",
        "netplay_player_chat",
        "netplay_fade_chat_toggle",
        "overlay_next",
        "osk_toggle",
    ] {
        if isolate_keyboard {
            config.push_str(&format!("input_{field} = \"nul\"\n"));
        }
        for suffix in ["btn", "axis", "mbtn"] {
            config.push_str(&format!("input_{field}_{suffix} = \"nul\"\n"));
        }
    }
    config
}

/// Resolve sorting against the original content before substituting its
/// private copy. Boolean defaults follow the pinned RetroArch contract;
/// directory defaults use the already-resolved desktop Linux config root,
/// never a temporary path.
fn fbneo_persistent_directories(
    inspection: &FbneoLaunchInspection,
) -> Result<(String, Vec<(std::path::PathBuf, std::path::PathBuf)>)> {
    let original = &inspection
        .launch_identity
        .retroarch_content
        .as_ref()
        .context("FBNeo original content identity is missing")?
        .content;
    let parent = original
        .parent()
        .context("FBNeo content has no parent directory")?;
    let base = &inspection.base_config_text;
    let library = &inspection.report.core.core_name;
    ensure!(
        !library.is_empty()
            && !matches!(library.as_str(), "." | "..")
            && !library.contains(['/', '\\'])
            && !library.chars().any(char::is_control),
        "FBNeo library name cannot be used for save sorting"
    );
    let mut config = String::new();
    let mut directories = Vec::new();
    for (directory_key, content_key, sort_content_key, sort_core_key) in [
        (
            "savefile_directory",
            "savefiles_in_content_dir",
            "sort_savefiles_by_content_enable",
            "sort_savefiles_enable",
        ),
        (
            "savestate_directory",
            "savestates_in_content_dir",
            "sort_savestates_by_content_enable",
            "sort_savestates_enable",
        ),
    ] {
        // config.def.h: per-core sorting defaults true; per-content sorting
        // and saves-in-content default false, without platform conditionals.
        let mut path = if config_bool(base, content_key, false)? {
            parent.to_path_buf()
        } else {
            match cfg_value(base, directory_key)?
                .filter(|value| !value.is_empty() && value != "default")
            {
                Some(value) => {
                    let path = configured_path(&value)?;
                    ensure!(
                        path.is_dir(),
                        "FBNeo configured {directory_key} is not a directory; RetroArch would ignore it, so resolve that setting first"
                    );
                    path
                }
                None => {
                    // Desktop platform_unix.c sets DEFAULT_DIR_SRAM and
                    // DEFAULT_DIR_SAVESTATE under the same XDG/HOME root
                    // already resolved by retroarch_base. Custom HOME/XDG
                    // overrides and non-native runtimes are rejected above.
                    inspection
                        .base_config_path
                        .parent()
                        .context("FBNeo configuration root is missing")?
                        .join(if directory_key == "savefile_directory" {
                            "saves"
                        } else {
                            "states"
                        })
                }
            }
        };
        // RetroArch runloop_path_set_redirect appends content then core.
        if config_bool(base, sort_content_key, false)? {
            path.push(
                parent
                    .file_name()
                    .context("FBNeo save sorting needs a named content directory")?,
            );
        }
        if config_bool(base, sort_core_key, true)? {
            path.push(library);
        }
        ensure!(
            path.is_absolute() && path.is_dir(),
            "FBNeo resolved {directory_key} must already exist to preserve save routing"
        );
        let text = path
            .to_str()
            .context("FBNeo save/state path is not UTF-8")?;
        ensure!(
            !text.contains(['"', '\\']) && !text.chars().any(char::is_control),
            "FBNeo save/state path cannot be encoded in RetroArch configuration"
        );
        config.push_str(&format!("{directory_key} = \"{text}\"\n{content_key} = \"false\"\n{sort_content_key} = \"false\"\n{sort_core_key} = \"false\"\n"));
        let canonical = path.canonicalize()?;
        directories.push((path, canonical));
    }
    Ok((config, directories))
}

pub(crate) struct FbneoBindingFragment {
    pub(crate) config: String,
    /// RetroArch's analog-button path requires identity remap IDs; disabling
    /// automatic remap loading alone is not evidence that this is satisfied.
    pub(crate) requires_identity_remap: bool,
    pub(crate) missing: Vec<(
        crate::controller_fbneo::InputAddress,
        crate::controller_fbneo::BindingPart,
    )>,
    pub(crate) external_targets: Vec<crate::controller_fbneo::InputAddress>,
    pub(crate) pressure_fallback_aliases: Vec<crate::controller_fbneo::InputAddress>,
}

pub(crate) struct PreparedFbneoPlayer {
    transport: PlayerTransport,
    pub(crate) fragment: FbneoBindingFragment,
}

impl PreparedFbneoPlayer {
    pub(crate) fn check_health(&self) -> Result<()> {
        self.transport.check_health()
    }
}

#[cfg(all(target_os = "linux", target_pointer_width = "64"))]
pub(crate) struct FbneoNormalizedAssignments {
    frame: crate::controller_axis::GamepadFrame,
    bindings: Vec<crate::controller_fbneo::PhysicalBinding>,
    bipolar_codes: std::collections::BTreeSet<u16>,
    pressure_codes: std::collections::BTreeSet<u16>,
    axis_codes: std::collections::BTreeSet<u16>,
}

/// Validate the output capacity of a mixed aim/gamepad setup before opening
/// devices. Port requirements are validated separately by the shared analyzer.
#[cfg(all(target_os = "linux", target_pointer_width = "64"))]
pub(crate) fn fbneo_absolute_output_axes(
    calibration: &Calibration,
    assignments: &[crate::controller_fbneo::SourceBinding],
) -> Result<[u16; 2]> {
    let normalized = normalize_fbneo_assignments(calibration, assignments)?;
    crate::controller_axis::absolute_gamepad::output_axes(Some(&normalized.frame))
}

/// Pure saved-measurement normalization shared by review and live preparation.
/// No input-node reads, virtual devices or native core execution occur here.
#[cfg(all(target_os = "linux", target_pointer_width = "64"))]
pub(crate) fn fbneo_axis_pair_choices(calibration: &Calibration) -> Result<Vec<serde_json::Value>> {
    calibration.validate()?;
    ensure!(
        calibration.os == "linux",
        "FBNeo axis pairs require Linux calibration"
    );
    let layout = catalog()
        .layout(&calibration.layout)
        .context("Unknown FBNeo source layout")?;
    let mut choices = Vec::new();
    for negative in layout.controls.iter().filter(|control| control.analog) {
        let Some(input) = calibration.bindings.get(&negative.id) else {
            continue;
        };
        let Some((native, measured)) = input.native.as_ref().zip(input.axis.as_ref()) else {
            continue;
        };
        if native.code >> 16 != 3 || native.direction != -1 || measured.direction() != -1 {
            continue;
        }
        for positive in layout.controls.iter().filter(|control| control.analog) {
            let Some(other) = calibration.bindings.get(&positive.id) else {
                continue;
            };
            let Some((other_native, other_measured)) =
                other.native.as_ref().zip(other.axis.as_ref())
            else {
                continue;
            };
            if other_native.code != native.code
                || other_native.direction != 1
                || other_measured.direction() != 1
            {
                continue;
            }
            // Candidate eligibility also uses the actual transport axis-code
            // restrictions (e.g. hats cannot masquerade as continuous sticks).
            let Ok(axis) =
                crate::controller_axis::BipolarAxis::from_measurements(measured, other_measured)
            else {
                continue;
            };
            if !calibration
                .bindings
                .values()
                .filter(|input| {
                    input
                        .native
                        .as_ref()
                        .is_some_and(|other| other.code == native.code)
                })
                .filter_map(|input| input.axis.as_ref())
                .all(|sample| {
                    sample
                        == if sample.direction() < 0 {
                            measured
                        } else {
                            other_measured
                        }
                })
            {
                continue;
            }
            if crate::controller_axis::GamepadFrame::new(
                [],
                [(
                    native.code as u16,
                    crate::controller_axis::GamepadAxis::Bipolar(axis),
                )],
            )
            .is_err()
            {
                continue;
            }
            for reversed in [false, true] {
                let (minus, plus) = if reversed {
                    (positive, negative)
                } else {
                    (negative, positive)
                };
                choices.push(serde_json::json!({"negative_source":minus.id,"positive_source":plus.id,
                    "label":format!("{} / {}{}", minus.label, plus.label, if reversed { " (reversed)" } else { "" })}));
            }
        }
    }
    Ok(choices)
}

#[cfg(all(target_os = "linux", target_pointer_width = "64"))]
pub(crate) fn normalize_fbneo_assignments(
    calibration: &Calibration,
    assignments: &[crate::controller_fbneo::SourceBinding],
) -> Result<FbneoNormalizedAssignments> {
    use crate::controller_axis::{BipolarAxis, GamepadAxis, GamepadFrame, PressureAxis};
    use crate::controller_fbneo::{BindingPart, PhysicalBinding};
    calibration.validate()?;
    ensure!(
        calibration.os == "linux" && !assignments.is_empty() && assignments.len() <= 8192,
        "FBNeo requires bounded assignments and Linux physical calibration"
    );
    let layout = catalog()
        .layout(&calibration.layout)
        .context("Unknown FBNeo source layout")?;
    let mut buttons = std::collections::BTreeSet::new();
    let mut roles = BTreeMap::<u16, BindingPart>::new();
    let mut measurements = BTreeMap::<u16, Vec<crate::controller_axis::AxisMeasurement>>::new();
    let mut bindings = Vec::new();
    for assignment in assignments {
        let control = layout
            .controls
            .iter()
            .find(|control| control.id == assignment.source)
            .context("FBNeo assignment source is absent from the physical layout")?;
        let input = calibration
            .bindings
            .get(&assignment.source)
            .context("FBNeo assignment source has no physical calibration")?;
        let native = input
            .native
            .as_ref()
            .context("FBNeo source needs physical-input capture")?;
        let code = (native.code & 0xffff) as u16;
        match native.code >> 16 {
            1 => {
                ensure!(
                    assignment.part == BindingPart::Digital,
                    "A physical button cannot supply continuous FBNeo input"
                );
                buttons.insert(code);
            }
            3 => {
                let measured = input
                    .axis
                    .as_ref()
                    .context("FBNeo axes require measured calibration")?;
                measured.validate()?;
                ensure!(
                    native.direction == measured.direction(),
                    "FBNeo axis direction differs from its measurement"
                );
                let role = if assignment.part == BindingPart::Pressure {
                    BindingPart::Pressure
                } else if matches!(
                    assignment.part,
                    BindingPart::Negative | BindingPart::Positive
                ) {
                    BindingPart::Positive
                } else {
                    BindingPart::Digital
                };
                ensure!(
                    role == BindingPart::Digital || control.analog,
                    "Continuous FBNeo input requires a physical analog control"
                );
                if let Some(previous) = roles.get(&code) {
                    ensure!(
                        *previous == role
                            || *previous == BindingPart::Digital
                            || role == BindingPart::Digital,
                        "One physical axis cannot mix FBNeo pressure and bipolar roles"
                    );
                }
                if role != BindingPart::Digital || !roles.contains_key(&code) {
                    roles.insert(code, role);
                }
                measurements.entry(code).or_default().push(measured.clone());
            }
            _ => bail!("FBNeo normalized transport requires gamepad buttons or axes"),
        }
        bindings.push(PhysicalBinding {
            target: assignment.target.clone(),
            part: assignment.part,
            input: native.clone(),
        });
    }
    let mut axes = BTreeMap::new();
    let mut bipolar_codes = std::collections::BTreeSet::new();
    let mut pressure_codes = std::collections::BTreeSet::new();
    for (&code, &role) in &roles {
        let selected = &measurements[&code];
        let first = &selected[0];
        ensure!(
            selected.iter().all(|axis| axis.minimum == first.minimum
                && axis.maximum == first.maximum
                && axis.released == first.released
                && axis.flat == first.flat
                && axis.fuzz == first.fuzz
                && axis.resolution == first.resolution),
            "FBNeo shared axis has inconsistent physical measurements"
        );
        let axis = match role {
            BindingPart::Pressure => {
                ensure!(
                    selected.iter().all(|axis| axis.pressed == first.pressed),
                    "FBNeo pressure and its digital fallback must use the same measured gesture"
                );
                for binding in bindings
                    .iter_mut()
                    .filter(|binding| binding.input.code == (3 << 16 | u32::from(code)))
                {
                    binding.input.direction = 1;
                }
                pressure_codes.insert(code);
                GamepadAxis::Pressure(PressureAxis::from_measurement(first)?)
            }
            BindingPart::Positive => {
                let all = calibration
                    .bindings
                    .values()
                    .filter(|input| {
                        input
                            .native
                            .as_ref()
                            .is_some_and(|native| native.code == (3 << 16 | u32::from(code)))
                    })
                    .filter_map(|input| input.axis.as_ref())
                    .collect::<Vec<_>>();
                let negative = all
                    .iter()
                    .copied()
                    .find(|axis| axis.direction() < 0)
                    .context("Calibrate both halves of the FBNeo source axis")?;
                let positive = all
                    .iter()
                    .copied()
                    .find(|axis| axis.direction() > 0)
                    .context("Calibrate both halves of the FBNeo source axis")?;
                ensure!(
                    all.iter().all(|axis| *axis
                        == if axis.direction() < 0 {
                            negative
                        } else {
                            positive
                        }),
                    "FBNeo source axis has inconsistent endpoints"
                );
                bipolar_codes.insert(code);
                GamepadAxis::Bipolar(BipolarAxis::from_measurements(negative, positive)?)
            }
            _ => GamepadAxis::Passthrough {
                minimum: first.minimum,
                maximum: first.maximum,
                neutral: first.released,
            },
        };
        axes.insert(code, axis);
    }
    Ok(FbneoNormalizedAssignments {
        frame: GamepadFrame::new(buttons, axes)?,
        bindings,
        bipolar_codes,
        pressure_codes,
        axis_codes: roles.keys().copied().collect(),
    })
}

#[cfg(all(target_os = "linux", target_pointer_width = "64"))]
pub(crate) fn prepare_fbneo_player(
    inspection: &FbneoLaunchInspection,
    port: u32,
    calibration: &Calibration,
    device: &ControllerDevice,
    assignments: &[crate::controller_fbneo::SourceBinding],
    cancel: &std::sync::atomic::AtomicBool,
) -> Result<PreparedFbneoPlayer> {
    check_preparation_cancel(cancel)?;
    let FbneoNormalizedAssignments {
        frame,
        bindings,
        bipolar_codes,
        pressure_codes,
        axis_codes,
    } = normalize_fbneo_assignments(calibration, assignments)?;
    check_preparation_cancel(cancel)?;
    let original_numbering = JoydevMap::read(&device.device_path)?;
    for assignment in assignments {
        let native = calibration
            .bindings
            .get(&assignment.source)
            .and_then(|input| input.native.as_ref())
            .context("FBNeo source has no native calibration")?;
        original_numbering.binding(native)?;
    }
    // Validate the complete mapping before opening a virtual input device.
    let checked_fragment = render_fbneo_bindings(
        inspection,
        port,
        &original_numbering,
        &bindings,
        &bipolar_codes,
        &pressure_codes,
    )?;
    ensure!(
        checked_fragment.missing.is_empty() && checked_fragment.external_targets.is_empty(),
        "FBNeo mapping is incomplete; no virtual input device was opened"
    );
    let (event, identity) = measured_event_path(
        &device.device_path,
        &device.event_paths,
        Path::new("/sys/class/input"),
    )?;
    crate::controller_axis::validate_recorded_axes(
        event,
        calibration
            .bindings
            .values()
            .filter_map(|input| input.native.as_ref().zip(input.axis.as_ref()))
            .filter(|(native, _)| axis_codes.contains(&((native.code & 0xffff) as u16)))
            .map(|(native, axis)| (native.code, axis)),
    )?;
    let (bridge, absolute_axes) = if let Some(absolute) = inspection.absolute_sources.get(&port) {
        let (bridge, axes) = crate::controller_axis::absolute_gamepad::start(
            absolute,
            Some((event, frame, &identity)),
            cancel,
        )?;
        (bridge, Some(axes))
    } else {
        (
            crate::controller_axis::GamepadBridge::start(event, frame, &identity, cancel)?,
            None,
        )
    };
    ensure!(
        JoydevMap::read(&device.device_path)? == original_numbering
            && measured_event_path(
                &device.device_path,
                &device.event_paths,
                Path::new("/sys/class/input")
            )?
            .1 == identity,
        "FBNeo physical controller changed during transport preparation"
    );
    let deadline = std::time::Instant::now() + std::time::Duration::from_secs(2);
    let path = loop {
        check_preparation_cancel(cancel)?;
        bridge.check_health()?;
        if let Some(path) = bridge.joydev_path()? {
            break path;
        }
        ensure!(
            std::time::Instant::now() < deadline,
            "FBNeo normalized controller has no joydev node"
        );
        std::thread::sleep(std::time::Duration::from_millis(10));
    };
    let numbering = JoydevMap::read(&path)?;
    let mut fragment = render_fbneo_bindings(
        inspection,
        port,
        &numbering,
        &bindings,
        &bipolar_codes,
        &pressure_codes,
    )?;
    if let Some(axes) = absolute_axes {
        add_fbneo_absolute_aim(&mut fragment, &numbering, port, axes)?;
    }
    Ok(PreparedFbneoPlayer {
        transport: PlayerTransport {
            numbering,
            pressure_codes,
            bridge: Some(bridge),
        },
        fragment,
    })
}

#[cfg(all(target_os = "linux", target_pointer_width = "64"))]
fn add_fbneo_absolute_aim(
    fragment: &mut FbneoBindingFragment,
    numbering: &JoydevMap,
    port: u32,
    axes: [u16; 2],
) -> Result<()> {
    for (axis, stem) in axes.into_iter().zip(["l_x", "l_y"]) {
        for (direction, suffix) in [(-1, "minus"), (1, "plus")] {
            let key = format!("input_player{}_{}_{}_axis", port + 1, stem, suffix);
            ensure!(
                cfg_value(&fragment.config, &key)?.as_deref() == Some("nul"),
                "Absolute aim collides with a gamepad axis assignment"
            );
            let (kind, value) = numbering.binding(&NativeInput {
                code: 0x30000 + u32::from(axis),
                direction,
            })?;
            ensure!(kind == "axis", "Absolute aim output is not a virtual axis");
            let prefix = format!("{key} =");
            fragment.config = fragment
                .config
                .lines()
                .filter(|line| !line.starts_with(&prefix))
                .map(|line| format!("{line}\n"))
                .collect();
            fragment.config.push_str(&format!("{key} = \"{value}\"\n"));
        }
    }
    Ok(())
}

#[cfg(all(target_os = "linux", target_pointer_width = "64"))]
fn prepare_fbneo_absolute_only(
    inspection: &FbneoLaunchInspection,
    port: u32,
    cancel: &std::sync::atomic::AtomicBool,
) -> Result<PreparedFbneoPlayer> {
    ensure!(
        !inspection
            .gamepad_targets()
            .iter()
            .any(|target| target.address.port == port),
        "Absolute-only FBNeo port still requires calibrated gamepad inputs"
    );
    let source = inspection
        .absolute_sources
        .get(&port)
        .context("No absolute source for this port")?;
    let (bridge, axes) = crate::controller_axis::absolute_gamepad::start(source, None, cancel)?;
    let deadline = std::time::Instant::now() + std::time::Duration::from_secs(2);
    let path = loop {
        check_preparation_cancel(cancel)?;
        bridge.check_health()?;
        if let Some(path) = bridge.joydev_path()? {
            break path;
        }
        ensure!(
            std::time::Instant::now() < deadline,
            "Absolute aim output has no joydev node"
        );
        std::thread::sleep(std::time::Duration::from_millis(10));
    };
    let numbering = JoydevMap::read(&path)?;
    let mut fragment = render_fbneo_bindings(
        inspection,
        port,
        &numbering,
        &[],
        &Default::default(),
        &Default::default(),
    )?;
    add_fbneo_absolute_aim(&mut fragment, &numbering, port, axes)?;
    Ok(PreparedFbneoPlayer {
        transport: PlayerTransport {
            numbering,
            pressure_codes: Default::default(),
            bridge: Some(bridge),
        },
        fragment,
    })
}

/// Render explicit per-game assignments using the current physical numbering.
/// The caller must retain and health-check the transport that owns normalized
/// axis codes. A fragment with missing/external targets is not launch-complete.
pub(crate) fn render_fbneo_bindings(
    inspection: &FbneoLaunchInspection,
    port: u32,
    numbering: &JoydevMap,
    bindings: &[crate::controller_fbneo::PhysicalBinding],
    normalized_axis_codes: &std::collections::BTreeSet<u16>,
    normalized_pressure_codes: &std::collections::BTreeSet<u16>,
) -> Result<FbneoBindingFragment> {
    let device = inspection
        .report
        .requested_devices
        .get(&port)
        .context("FBNeo binding port was not selected during inspection")?;
    let gamepad_targets = inspection.gamepad_targets();
    let analysis = analyze_fbneo_bindings(
        &gamepad_targets,
        port,
        bindings,
        normalized_axis_codes,
        normalized_pressure_codes,
    )?;
    let player = port + 1;
    let mut values = BTreeMap::new();
    for field in analysis.fields {
        for suffix in ["", "_btn", "_axis", "_mbtn"] {
            values.insert(
                format!("input_player{player}_{field}{suffix}"),
                "nul".to_owned(),
            );
        }
    }
    for binding in bindings {
        let field = crate::controller_fbneo::retroarch_field(&binding.target, binding.part)?;
        let (suffix, value) = numbering.binding(&binding.input)?;
        values.insert(format!("input_player{player}_{field}_{suffix}"), value);
    }
    values.insert(
        format!("input_player{player}_joypad_index"),
        numbering.index.to_string(),
    );
    values.insert(format!("input_player{player}_analog_dpad_mode"), "0".into());
    values.insert(
        format!("input_libretro_device_p{player}"),
        device.to_string(),
    );
    Ok(FbneoBindingFragment {
        requires_identity_remap: bindings
            .iter()
            .any(|binding| binding.part == crate::controller_fbneo::BindingPart::Pressure),
        config: values
            .into_iter()
            .map(|(key, value)| format!("{key} = \"{value}\"\n"))
            .collect(),
        missing: analysis.missing,
        external_targets: analysis.external_targets,
        pressure_fallback_aliases: analysis.pressure_fallback_aliases,
    })
}

struct FbneoBindingAnalysis {
    required_parts: usize,
    fields: std::collections::BTreeSet<&'static str>,
    missing: Vec<(
        crate::controller_fbneo::InputAddress,
        crate::controller_fbneo::BindingPart,
    )>,
    external_targets: Vec<crate::controller_fbneo::InputAddress>,
    pressure_fallback_aliases: Vec<crate::controller_fbneo::InputAddress>,
}

/// Semantic binding validation without invented runtime device numbering.
fn analyze_fbneo_bindings(
    targets: &[crate::controller_fbneo::MappingTarget],
    port: u32,
    bindings: &[crate::controller_fbneo::PhysicalBinding],
    normalized_axis_codes: &std::collections::BTreeSet<u16>,
    normalized_pressure_codes: &std::collections::BTreeSet<u16>,
) -> Result<FbneoBindingAnalysis> {
    use crate::controller_fbneo::{BindingPart, binding_parts, retroarch_field};
    ensure!(
        port < 16 && bindings.len() <= 8192,
        "FBNeo binding request exceeds its bounds"
    );
    let mut requested = std::collections::BTreeSet::new();
    let mut external_targets = Vec::new();
    let mut fields = OUTPUTS
        .iter()
        .map(|(_, field)| *field)
        .collect::<std::collections::BTreeSet<_>>();
    for target in targets.iter().filter(|target| target.address.port == port) {
        let parts = binding_parts(&target.address);
        if parts.is_empty() {
            external_targets.push(target.address.clone());
        }
        for &part in parts {
            requested.insert((target.address.clone(), part));
            fields.insert(retroarch_field(&target.address, part)?);
        }
    }
    // Clear every gun button too: an inherited unadvertised trigger/reload must
    // not leak into this explicitly assigned player's core input.
    fields.extend([
        "gun_trigger",
        "gun_offscreen_shot",
        "gun_aux_a",
        "gun_aux_b",
        "gun_aux_c",
        "gun_start",
        "gun_select",
        "gun_dpad_up",
        "gun_dpad_down",
        "gun_dpad_left",
        "gun_dpad_right",
    ]);
    let mut assigned = std::collections::BTreeSet::new();
    let mut channels = BTreeMap::new();
    let mut axis_pairs =
        BTreeMap::<crate::controller_fbneo::InputAddress, BTreeMap<BindingPart, NativeInput>>::new(
        );
    for binding in bindings {
        let target = (binding.target.clone(), binding.part);
        ensure!(
            requested.contains(&target) && assigned.insert(target),
            "FBNeo assignment has an absent, wrong-port or duplicate target"
        );
        let input = &binding.input;
        let code = (input.code & 0xffff) as u16;
        match binding.part {
            BindingPart::Pressure => ensure!(
                input.code >> 16 == 3
                    && input.direction == 1
                    && normalized_pressure_codes.contains(&code),
                "FBNeo pressure requires an active positive normalized pressure axis"
            ),
            BindingPart::Negative | BindingPart::Positive => ensure!(
                input.code >> 16 == 3
                    && normalized_axis_codes.contains(&code)
                    && matches!(input.direction, -1 | 1),
                "FBNeo analog direction requires an active normalized bipolar axis"
            ),
            BindingPart::Digital => ensure!(
                matches!((input.code >> 16, input.direction), (1, 0) | (3, -1 | 1)),
                "FBNeo digital binding requires a button or directional axis gesture"
            ),
        }
        let field = retroarch_field(&binding.target, binding.part)?;
        if let Some((previous_target, previous_input)) =
            // A button and an axis suffix still feed the same semantic
            // channel; different suffixes must not bypass alias validation.
            channels.insert(field, (binding.target.clone(), input.clone()))
        {
            ensure!(
                crate::controller_fbneo::retroarch_address_alias(&binding.target).as_ref()
                    == Some(&previous_target)
                    && previous_input == *input,
                "FBNeo assignments collide on the same RetroArch channel; native address aliases must share one physical gesture and pressure/digital fallback may not overwrite each other"
            );
        }
        if matches!(binding.part, BindingPart::Negative | BindingPart::Positive) {
            let mut axis_target = binding.target.clone();
            if axis_target.device == 1029 {
                // Validate the pair even when its halves were assigned on
                // different advertised/queried aliases of the same axis.
                axis_target.device = 5;
            }
            axis_pairs
                .entry(axis_target)
                .or_default()
                .insert(binding.part, input.clone());
        }
    }
    for pair in axis_pairs.values() {
        if let (Some(negative), Some(positive)) = (
            pair.get(&BindingPart::Negative),
            pair.get(&BindingPart::Positive),
        ) {
            ensure!(
                negative.code == positive.code && negative.direction == -positive.direction,
                "FBNeo bipolar halves must use opposite gestures of the same physical axis"
            );
        }
    }
    let mut pressure_fallback_aliases = Vec::new();
    for binding in bindings {
        if let Some(alias) = crate::controller_fbneo::retroarch_address_alias(&binding.target) {
            let slot = (alias, binding.part);
            if requested.contains(&slot) {
                assigned.insert(slot);
            }
        }
    }
    for binding in bindings
        .iter()
        .filter(|binding| binding.part == BindingPart::Pressure)
    {
        // Both libretro addresses read the same RetroArch bind field. Its
        // normalized axis supplies continuous pressure and the core's digital
        // fallback; a second physical bind is not mandatory for that alias.
        let alias = crate::controller_fbneo::InputAddress {
            port,
            device: 1,
            index: 0,
            id: binding.target.id,
        };
        let slot = (alias.clone(), BindingPart::Digital);
        if requested.contains(&slot) && assigned.insert(slot) {
            pressure_fallback_aliases.push(alias);
        }
    }
    Ok(FbneoBindingAnalysis {
        required_parts: requested.len(),
        fields,
        missing: requested.difference(&assigned).cloned().collect(),
        external_targets,
        pressure_fallback_aliases,
    })
}

#[cfg(all(target_os = "linux", target_pointer_width = "64"))]
pub(crate) fn review_fbneo_assignments(
    targets: &[crate::controller_fbneo::MappingTarget],
    port: u32,
    calibration: &Calibration,
    assignments: &[crate::controller_fbneo::SourceBinding],
) -> Result<serde_json::Value> {
    let analysis = analyze_saved_fbneo_assignments(targets, port, calibration, assignments)?;
    let mapped_parts = analysis
        .required_parts
        .checked_sub(analysis.missing.len())
        .context("FBNeo input-part coverage accounting is inconsistent")?;
    Ok(
        serde_json::json!({"port":port,"error":"","missing":analysis.missing,
        "required_parts":analysis.required_parts,"mapped_parts":mapped_parts,
        "mapped_percent":if analysis.required_parts == 0 { None } else { Some(100.0 * mapped_parts as f64 / analysis.required_parts as f64) },
        "external_targets":analysis.external_targets,"pressure_fallback_aliases":analysis.pressure_fallback_aliases,
        "launch_ready":false}),
    )
}

#[cfg(all(target_os = "linux", target_pointer_width = "64"))]
fn analyze_saved_fbneo_assignments(
    targets: &[crate::controller_fbneo::MappingTarget],
    port: u32,
    calibration: &Calibration,
    assignments: &[crate::controller_fbneo::SourceBinding],
) -> Result<FbneoBindingAnalysis> {
    calibration.validate()?;
    let analysis = if assignments.is_empty() {
        // An empty editable draft has no physical requirements yet; still
        // report its real missing/external target set without creating a frame.
        analyze_fbneo_bindings(targets, port, &[], &Default::default(), &Default::default())?
    } else {
        let normalized = normalize_fbneo_assignments(calibration, assignments)?;
        analyze_fbneo_bindings(
            targets,
            port,
            &normalized.bindings,
            &normalized.bipolar_codes,
            &normalized.pressure_codes,
        )?
    };
    Ok(analysis)
}

/// Staging acceptance for the implemented transport, never live readiness.
#[cfg(all(target_os = "linux", target_pointer_width = "64"))]
pub(crate) fn review_fbneo_keyboard_assignments(
    targets: &[crate::controller_fbneo::MappingTarget],
    port: u32,
    calibration: &Calibration,
    keys: &[crate::controller_fbneo::keyboard::KeyboardBinding],
    assignments: &[crate::controller_fbneo::SourceBinding],
    selected_devices: &BTreeMap<u32, u32>,
) -> Result<serde_json::Value> {
    let (physical_targets, physical_assignments, remap) =
        crate::controller_fbneo::keyboard::physical_plan(
            targets,
            port,
            keys,
            assignments,
            selected_devices,
        )?;
    let mut review =
        review_fbneo_assignments(&physical_targets, port, calibration, &physical_assignments)?;
    review["keyboard_target_count"] = serde_json::json!(keys.len());
    review["remap_preview"] = serde_json::json!(remap);
    review["launch_ready"] = serde_json::json!(false);
    review["notice"] = serde_json::json!(
        "Physical frontend-channel review only. Launch stages a private remap and isolates frontend hotkeys. Native keyboard identities remain separate; runtime behavior is unverified."
    );
    Ok(review)
}

/// Staging acceptance for the implemented transport, never live readiness.
#[cfg(all(target_os = "linux", target_pointer_width = "64"))]
pub(crate) fn validate_saved_fbneo_assignments(
    targets: &[crate::controller_fbneo::MappingTarget],
    port: u32,
    calibration: &Calibration,
    assignments: &[crate::controller_fbneo::SourceBinding],
) -> Result<()> {
    ensure!(
        !assignments.is_empty(),
        "FBNeo player has no assigned physical controls"
    );
    let analysis = analyze_saved_fbneo_assignments(targets, port, calibration, assignments)?;
    ensure!(
        analysis.external_targets.is_empty(),
        "FBNeo player {} needs external adapters for {} native targets",
        port + 1,
        analysis.external_targets.len()
    );
    ensure!(
        analysis.missing.is_empty(),
        "FBNeo player {} has {} unassigned native input parts",
        port + 1,
        analysis.missing.len()
    );
    Ok(())
}

/// Resolve inspection against the actual prepared launch, not an independently
/// entered game/core path. Dependencies and driver topology must already be
/// resolved by the caller; this does not guess parent ROMs from title strings.
pub(crate) fn prepare_fbneo_inspection(
    option: &RomEmulatorOption,
    plan: &LaunchPlan,
    helper: &Path,
    expected_library: &str,
    dependencies: &[std::path::PathBuf],
    system_files: &[std::path::PathBuf],
    devices: BTreeMap<u32, u32>,
    topology: &crate::controller_fbneo::ControllerTopology,
    cancel: &std::sync::atomic::AtomicBool,
) -> Result<FbneoLaunchInspection> {
    use lunchbox_controller_probe::content_inspection::{DeviceSelection, OptionOverride, Request};
    check_preparation_cancel(cancel)?;
    ensure!(
        option.runtime_kind == EmulatorRuntimeKind::RetroArch && option.core_name == "fbneo",
        "FBNeo inspection requires the actual FBNeo RetroArch launch"
    );
    let EmulatorExecutable::Native(executable) = &option.executable else {
        bail!(
            "FBNeo inspection needs a native runtime adapter; Flatpak and Wine require their own helper environment"
        );
    };
    ensure!(
        std::fs::canonicalize(executable)? == std::fs::canonicalize(&plan.program)?,
        "RetroArch executable changed before FBNeo inspection"
    );
    let mut magic = [0u8; 4];
    std::fs::File::open(&plan.program)?.read_exact(&mut magic)?;
    ensure!(
        magic == *b"\x7fELF",
        "FBNeo inspection requires a direct native RetroArch ELF executable; wrappers need explicit runtime resolution"
    );
    let runtime_sha256 = lunchbox_controller_probe::file_hash(&plan.program)?;
    ensure!(
        !expected_library.is_empty()
            && !expected_library.contains(['/', '\\'])
            && expected_library != "."
            && expected_library != "..",
        "FBNeo library identity must be a single config-directory component"
    );
    ensure!(
        !plan
            .environment
            .iter()
            .any(|(key, _)| key == "HOME" || key == "XDG_CONFIG_HOME"),
        "Custom configuration roots need resolution before FBNeo inspection"
    );
    let content = plan
        .retroarch_content
        .as_ref()
        .context("FBNeo inspection requires prepared core/content identity")?;
    crate::controller_launch_modes::validate_arguments(
        emulator_arguments(plan, &option.executable)?,
        content,
    )?;
    let (base, config) = retroarch_base(&option.executable)?;
    ensure!(
        !config
            .lines()
            .any(|line| line.trim_start().starts_with("#include")),
        "Included RetroArch configuration needs resolution before FBNeo inspection"
    );
    let config_directory = cfg_value(&config, "rgui_config_directory")?
        .filter(|value| !value.is_empty() && value != "default")
        .map(|value| configured_path(&value))
        .transpose()?
        .unwrap_or_else(|| base.join("config"));
    let baseline_options = effective_mode_core_options(
        &base,
        &config,
        &config_directory,
        expected_library,
        &content.content,
    )?;
    ensure!(
        baseline_options.len() <= 1024 * 1024,
        "FBNeo core options exceed the snapshot limit"
    );
    let mut selected_options = BTreeMap::new();
    for line in baseline_options.lines() {
        let line = line.trim();
        ensure!(
            !line.starts_with("#include"),
            "Included core options need resolution before FBNeo inspection"
        );
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        let (key, _) = line
            .split_once('=')
            .context("Malformed core option in FBNeo snapshot")?;
        let key = key.trim();
        // Global option files also contain other cores. FBNeo declares its
        // keys with this exact namespace, including hyphenated DIP/macro keys.
        if !key.starts_with("fbneo-") {
            continue;
        }
        let value = cfg_value(line, key)?.context("Missing FBNeo core option value")?;
        ensure!(
            selected_options.insert(key.to_owned(), value).is_none(),
            "Duplicate FBNeo core option"
        );
    }
    let request = Request {
        schema_version: 1,
        core: content.core.canonicalize()?,
        core_sha256: lunchbox_controller_probe::file_hash(&content.core)?,
        core_name: expected_library.to_owned(),
        content: content.content.canonicalize()?,
        content_dependencies: dependencies
            .iter()
            .map(|path| path.canonicalize())
            .collect::<std::io::Result<Vec<_>>>()?,
        system_files: system_files
            .iter()
            .map(|path| path.canonicalize())
            .collect::<std::io::Result<Vec<_>>>()?,
        options: selected_options
            .into_iter()
            .map(|(key, value)| OptionOverride { key, value })
            .collect(),
        devices: devices
            .into_iter()
            .map(|(port, device)| DeviceSelection { port, device })
            .collect(),
    };
    ensure!(
        content.content.file_name() == request.content.file_name(),
        "FBNeo inspection cannot change the content basename through a symlink"
    );
    // Launch stages this exact manifest too; dependencies need not live beside
    // the original content or in the user's global system directory.
    let report = crate::controller_fbneo::inspect_runtime(
        helper,
        &plan.environment,
        &request,
        topology,
        std::time::Duration::from_secs(30),
        cancel,
    )?;
    let targets = crate::controller_fbneo::mapping_targets(&report)?;
    check_preparation_cancel(cancel)?;
    Ok(FbneoLaunchInspection {
        launch_identity: plan.clone(),
        runtime_sha256,
        base_config_path: base.join("retroarch.cfg"),
        base_config_text: config,
        request,
        report,
        targets,
        relative_sources: BTreeMap::new(),
        keyboard_targets: None,
        keyboard_remap: None,
        keyboard_passthrough_port: None,
        absolute_sources: BTreeMap::new(),
        baseline_options,
    })
}

/// Launch-bound MAME discovery and mapping, retained for subsequent private
/// launch staging. This does not enable a catalog profile or launch a game.
pub(crate) struct MameLaunchInspection {
    selected_players: std::collections::BTreeSet<usize>,
    analog_assignments: Vec<crate::controller_mame::AnalogAssignment>,
    digital_assignments: Vec<crate::controller_mame::DigitalAssignment>,
    relative_assignments: Vec<crate::controller_mame::RelativeAssignment>,
    relative_button_assignments:
        Vec<crate::controller_mame::relative_buttons::RelativeButtonAssignment>,
    directory: tempfile::TempDir,
    pub launch_identity: LaunchPlan,
    pub request: crate::controller_mame::InspectionRequest,
    pub source_hashes: Vec<(std::path::PathBuf, [u8; 32])>,
    pub fields: crate::controller_mame::ActiveFieldSnapshot,
    pub mapping: crate::controller_mame::DigitalFieldPlan,
    original_controller_config: Option<String>,
    original_game_config: Option<String>,
    configuration: mame_configuration::Snapshot,
}

pub(crate) fn prepare_mame_inspection(
    option: &RomEmulatorOption,
    plan: &LaunchPlan,
    machine: &str,
    expected_library: &str,
    inputs: Vec<crate::controller_mame::InspectionInput>,
    players: &std::collections::BTreeSet<usize>,
    configuration: Option<mame_configuration::Snapshot>,
    dependency_roots: Vec<std::path::PathBuf>,
    analog_assignments: &[crate::controller_mame::AnalogAssignment],
    digital_assignments: &[crate::controller_mame::DigitalAssignment],
    automatic_player_limit: Option<usize>,
    relative_assignments: &[crate::controller_mame::RelativeAssignment],
    relative_button_assignments: &[crate::controller_mame::relative_buttons::RelativeButtonAssignment],
    inspect_mouse: bool,
    cancel: &std::sync::atomic::AtomicBool,
) -> Result<MameLaunchInspection> {
    check_preparation_cancel(cancel)?;
    ensure!(
        relative_assignments.len() <= 32768 && relative_button_assignments.len() <= 32768,
        "Too many native relative assignments"
    );
    ensure!(
        (relative_assignments.is_empty() && relative_button_assignments.is_empty())
            || inspect_mouse,
        "Relative native planning requires explicit mouse inspection"
    );
    ensure!(
        option.runtime_kind == EmulatorRuntimeKind::RetroArch && option.core_name == "mame",
        "MAME inspection requires the selected MAME RetroArch runtime"
    );
    let EmulatorExecutable::Native(executable) = &option.executable else {
        bail!(
            "MAME inspection requires a resolved native runtime; container and Wine adapters are not implemented"
        );
    };
    ensure!(
        executable.canonicalize()? == plan.program.canonicalize()?,
        "MAME frontend changed before inspection"
    );
    let mut magic = [0u8; 4];
    std::fs::File::open(&plan.program)?.read_exact(&mut magic)?;
    ensure!(
        magic == *b"\x7fELF",
        "MAME inspection requires a direct native ELF frontend"
    );
    ensure!(
        !expected_library.is_empty()
            && !expected_library.contains(['/', '\\'])
            && expected_library != "."
            && expected_library != "..",
        "Invalid MAME core configuration identity"
    );
    ensure!(
        !plan
            .environment
            .iter()
            .any(|(key, _)| key == "HOME" || key == "XDG_CONFIG_HOME"),
        "Resolve custom configuration roots before MAME inspection"
    );
    let content = plan
        .retroarch_content
        .as_ref()
        .context("MAME inspection requires prepared content identity")?;
    crate::controller_launch_modes::validate_arguments(
        emulator_arguments(plan, &option.executable)?,
        content,
    )?;
    let prepared_content = content.content.canonicalize()?;
    ensure!(
        inputs
            .iter()
            .any(|input| input.destination.starts_with("roms")
                && input.source.canonicalize().ok().as_ref() == Some(&prepared_content)),
        "MAME dependency manifest omits the prepared launch content"
    );
    let configuration = match configuration {
        Some(snapshot) => snapshot,
        None => mame_configuration::Snapshot::read(
            &option.executable,
            expected_library,
            &content.content,
            false,
        )?,
    };
    ensure!(
        configuration.library == expected_library && configuration.content == content.content,
        "MAME configuration snapshot belongs to different content or core"
    );
    configuration.verify()?;
    // The native sequence writer assumes the wrapper's fixed button order.
    // Enforce it before creating the fresh inspection core, not only at launch.
    // Retain unrelated options (including per-game DIP settings) verbatim.
    let core_options = core_options_overlay(
        &configuration.options,
        &crate::controller_mame::inspection_options(inspect_mouse),
    )?;
    let mut request = crate::controller_mame::InspectionRequest {
        retroarch: plan.program.canonicalize()?,
        core: content.core.canonicalize()?,
        machine: machine.to_owned(),
        inspect_mouse,
        inputs,
        dependency_roots,
        core_options,
        environment: plan.environment.clone(),
    };
    let inspected = crate::controller_mame::inspect_runtime(
        &request,
        std::time::Duration::from_secs(if request.dependency_roots.is_empty() {
            30
        } else {
            120
        }),
        cancel,
    )?;
    // Retain exactly what was staged for the eventual session tree and source
    // checks, including dependencies added by the selected core's discovery.
    if !relative_button_assignments.is_empty() {
        inspected.validate_mouse_button_configuration(cancel)?;
    }
    request.inputs = inspected.inputs;
    configuration.verify()?;
    // Inspection always retains the full native snapshot. Automatic allocation
    // only considers as many active ports as there are selected controllers;
    // saved per-game setups keep their explicit player set untouched.
    let automatic_players = if let Some(limit) = automatic_player_limit {
        ensure!(
            (1..=8).contains(&limit)
                && analog_assignments.is_empty()
                && digital_assignments.is_empty(),
            "Invalid automatic MAME player allocation"
        );
        ensure!(
            relative_assignments.is_empty() && relative_button_assignments.is_empty(),
            "Automatic gamepad allocation cannot select relative sources"
        );
        let mut selected = std::collections::BTreeSet::new();
        for port in players {
            check_preparation_cancel(cancel)?;
            if selected.len() == limit {
                break;
            }
            if crate::controller_mame::optional_digital_profile(
                &inspected.snapshot,
                *port,
                crate::controller_mame::DigitalLayout::FixedChannels,
            )?
            .is_some()
            {
                selected.insert(*port);
            }
        }
        ensure!(
            !selected.is_empty(),
            "MAME reported no standard digital arcade players"
        );
        Some(selected)
    } else {
        None
    };
    let players = automatic_players.as_ref().unwrap_or(players);
    if !relative_button_assignments.is_empty() {
        inspected.snapshot.require_game_mouse_mode()?;
    }
    let mapping = if !relative_assignments.is_empty() || !relative_button_assignments.is_empty() {
        crate::controller_mame::relative_buttons::plan_relative_button_fields(
            &inspected.snapshot,
            inspected.original_controller_config.as_deref(),
            inspected.original_game_config.as_deref(),
            players,
            analog_assignments,
            digital_assignments,
            relative_assignments,
            relative_button_assignments,
        )?
    } else if !digital_assignments.is_empty() {
        crate::controller_mame::plan_explicit_fields(
            &inspected.snapshot,
            inspected.original_controller_config.as_deref(),
            inspected.original_game_config.as_deref(),
            players,
            analog_assignments,
            digital_assignments,
        )?
    } else if analog_assignments.is_empty() {
        crate::controller_mame::plan_digital_fields(
            &inspected.snapshot,
            inspected.original_controller_config.as_deref(),
            inspected.original_game_config.as_deref(),
            players,
        )?
    } else {
        crate::controller_mame::plan_mixed_fields(
            &inspected.snapshot,
            inspected.original_controller_config.as_deref(),
            inspected.original_game_config.as_deref(),
            players,
            analog_assignments,
        )?
    };
    check_preparation_cancel(cancel)?;
    Ok(MameLaunchInspection {
        selected_players: players.clone(),
        analog_assignments: analog_assignments.to_vec(),
        digital_assignments: digital_assignments.to_vec(),
        relative_assignments: relative_assignments.to_vec(),
        relative_button_assignments: relative_button_assignments.to_vec(),
        directory: inspected.directory,
        launch_identity: plan.clone(),
        request,
        source_hashes: inspected.source_hashes,
        fields: inspected.snapshot,
        mapping,
        original_controller_config: inspected.original_controller_config,
        original_game_config: inspected.original_game_config,
        configuration,
    })
}

/// Build an inspection request before a reviewed snapshot exists. Extra setup
/// fields are not consumed here; full setup validation still happens at staging.
pub(crate) fn mame_request_from_draft(
    draft: &str,
    runtime: &Path,
    discovery_roots: &str,
    cancel: &std::sync::atomic::AtomicBool,
) -> Result<crate::controller_mame::InspectionRequest> {
    #[derive(serde::Deserialize)]
    struct Context {
        core: std::path::PathBuf,
        content: std::path::PathBuf,
        machine: String,
        library: String,
        inputs: Vec<crate::controller_mame::InspectionInput>,
        persistent: crate::controller_mame::PersistentPaths,
    }
    ensure!(
        draft.len() <= 16 * 1024 * 1024,
        "MAME setup draft exceeds limit"
    );
    check_preparation_cancel(cancel)?;
    let context: Context = serde_json::from_str(draft)?;
    ensure!(
        discovery_roots.len() <= 256 * 1024,
        "MAME dependency-root text exceeds limit"
    );
    let mut dependency_roots = Vec::new();
    for value in discovery_roots
        .lines()
        .map(str::trim)
        .filter(|value| !value.is_empty())
    {
        ensure!(
            dependency_roots.len() < 64,
            "MAME discovery supports at most 64 roots"
        );
        let path = Path::new(value);
        ensure!(path.is_absolute(), "MAME dependency roots must be absolute");
        let path = path
            .canonicalize()
            .with_context(|| format!("Resolving MAME dependency root {value}"))?;
        ensure!(
            path.is_dir() && !dependency_roots.contains(&path),
            "MAME dependency roots must be distinct directories"
        );
        dependency_roots.push(path);
    }
    ensure!(
        runtime.is_absolute() && context.core.is_absolute() && context.content.is_absolute(),
        "MAME runtime/core/content paths must be absolute"
    );
    let runtime = runtime.canonicalize()?;
    let mut magic = [0u8; 4];
    std::fs::File::open(&runtime)?.read_exact(&mut magic)?;
    ensure!(
        magic == *b"\x7fELF",
        "MAME inspection requires direct native RetroArch, not a wrapper"
    );
    ensure!(
        !context.library.is_empty()
            && context.library != "."
            && context.library != ".."
            && !context
                .library
                .chars()
                .any(|c| c.is_control() || "/\\".contains(c)),
        "Invalid MAME library configuration identity"
    );
    let executable = EmulatorExecutable::Native(runtime.clone());
    let configuration =
        mame_configuration::Snapshot::read(&executable, &context.library, &context.content, false)?;
    let core_options = core_options_overlay(
        &configuration.options,
        &crate::controller_mame::digital_options(),
    )?;
    let state = crate::controller_mame::persistent_inputs(&context.persistent, cancel)?;
    for input in context.inputs.iter().filter(|input| {
        input.destination.starts_with("nvram") || input.destination.starts_with("diff")
    }) {
        ensure!(
            state.contains(input),
            "Explicit MAME state entry conflicts with persistent paths"
        );
    }
    let content = context.content.canonicalize()?;
    let mut inputs: Vec<_> = context
        .inputs
        .into_iter()
        .filter(|input| {
            !input.destination.starts_with("nvram") && !input.destination.starts_with("diff")
        })
        .collect();
    ensure!(
        inputs
            .iter()
            .any(|input| input.destination.starts_with("roms")
                && input.source.canonicalize().ok().as_ref() == Some(&content)),
        "MAME manifest omits the selected content"
    );
    inputs.extend(state);
    check_preparation_cancel(cancel)?;
    configuration.verify()?;
    Ok(crate::controller_mame::InspectionRequest {
        retroarch: runtime,
        core: context.core.canonicalize()?,
        machine: context.machine,
        inspect_mouse: false,
        inputs,
        dependency_roots,
        core_options,
        environment: Vec::new(),
    })
}

/// Owns a clean game-session dependency tree for the entire child lifetime. The caller
/// must retain this object and route RetroArch's system/save/config paths to it.
/// This is staged native input, not yet a calibrated frontend launch session.
pub(crate) struct PreparedMameInput {
    // Fields drop in declaration order: stop/join relative forwarding before
    // releasing the private tree or inspection evidence it belongs to.
    #[cfg(all(target_os = "linux", target_pointer_width = "64"))]
    relative_bridge: Option<crate::controller_axis::relative::RelativeMouseBridge>,
    #[cfg(all(target_os = "linux", target_pointer_width = "64"))]
    relative_launch_plan: Option<LaunchPlan>,
    #[cfg(all(target_os = "linux", target_pointer_width = "64"))]
    relative_pending: Option<(
        crate::controller_axis::relative::RelativeMouseGroup,
        crate::controller_axis::relative_topology::RelativeTopologyGuard,
    )>,
    _directory: tempfile::TempDir,
    persistent: crate::controller_mame::PersistentPaths,
    frontend_persistent: Vec<std::path::PathBuf>,
    inspection: MameLaunchInspection,
    pub command_path: std::path::PathBuf,
    pub root: std::path::PathBuf,
    files: Vec<(std::path::PathBuf, String)>,
}

pub(crate) fn prepare_mame_calibrated_session(
    mut input: PreparedMameInput,
    players: &BTreeMap<usize, (&Calibration, &ControllerDevice)>,
    plan: &mut LaunchPlan,
    frontend_save: &Path,
    frontend_state: &Path,
    digital_layout: crate::controller_mame::DigitalLayout,
    player_digital_layouts: &BTreeMap<usize, crate::controller_mame::DigitalLayout>,
    analog_assignments: &[crate::controller_mame::AnalogAssignment],
    digital_assignments: &[crate::controller_mame::DigitalAssignment],
    cancel: &std::sync::atomic::AtomicBool,
) -> Result<CalibratedLaunch> {
    ensure!(
        cfg!(target_os = "linux"),
        "MAME physical calibration requires Linux"
    );
    check_preparation_cancel(cancel)?;
    input.verify_inputs(plan)?;
    let profile_snapshot = crate::controller_mame::relative_buttons::gamepad_snapshot(
        &input.inspection.fields,
        &input.inspection.relative_button_assignments,
    )?;
    ensure!(
        digital_assignments == input.inspection.digital_assignments.as_slice(),
        "MAME digital assignments changed after native mapping preparation"
    );
    ensure!(
        analog_assignments == input.inspection.analog_assignments.as_slice(),
        "MAME analog assignments changed after native mapping preparation"
    );
    ensure!(
        players
            .keys()
            .copied()
            .chain(
                input
                    .inspection
                    .relative_assignments
                    .iter()
                    .map(|entry| entry.source_player)
            )
            .chain(
                input
                    .inspection
                    .relative_button_assignments
                    .iter()
                    .map(|entry| entry.source_player)
            )
            .collect::<std::collections::BTreeSet<_>>()
            == input.inspection.selected_players,
        "Physical MAME player assignments differ from the inspected native mapping"
    );
    // Validate the fresh inspection here as well as in saved settings: callers
    // must not clear a relative-only port that still owns gamepad channels.
    for port in input
        .inspection
        .selected_players
        .iter()
        .filter(|port| !players.contains_key(port))
    {
        ensure!(
            crate::controller_mame::explicit_profile(
                &profile_snapshot,
                &input.inspection.selected_players,
                analog_assignments,
                digital_assignments,
                *port,
                crate::controller_mame::DigitalLayout::Automatic,
            )?
            .is_none(),
            "Relative-only MAME player {port} still requires gamepad channels; assign a gamepad or explicitly route those actions to another gamepad player"
        );
    }
    let mut devices = std::collections::BTreeSet::new();
    ensure!(
        player_digital_layouts
            .keys()
            .all(|port| players.contains_key(port)),
        "MAME layout overrides refer to an unselected physical player"
    );
    let mut transports = Vec::new();
    let mut config = String::from(
        "input_joypad_driver = \"linuxraw\"\ninput_autodetect_enable = \"false\"\ninput_max_users = \"8\"\n",
    );
    config.push_str(&fbneo_gamepad_hotkey_isolation());
    for (port, (calibration, device)) in players {
        check_preparation_cancel(cancel)?;
        ensure!(
            devices.insert(&device.stable_id),
            "One physical controller cannot occupy multiple MAME players"
        );
        let profile = crate::controller_mame::explicit_profile(
            &profile_snapshot,
            &input.inspection.selected_players,
            analog_assignments,
            digital_assignments,
            *port,
            player_digital_layouts
                .get(port)
                .copied()
                .unwrap_or(digital_layout),
        )?
        .context("Selected MAME player has no active mapped controls")?;
        let (transport, normalized) = if analog_assignments
            .iter()
            .any(|assignment| assignment.source_player == *port)
        {
            let (transport, normalized) = prepare_mame_analog_transport(
                calibration,
                &profile,
                device,
                analog_assignments,
                *port,
                cancel,
            )?;
            (transport, Some(normalized))
        } else {
            (
                prepare_player_transport(calibration, &profile, device, cancel)?,
                None,
            )
        };
        config.push_str(&player_config_transport(
            normalized.as_ref().unwrap_or(calibration),
            &profile,
            &transport.numbering,
            *port,
            1,
            &transport.pressure_codes,
        )?);
        transports.push(transport);
    }
    for port in 1..=8 {
        if !players.contains_key(&port) {
            config.push_str(&empty_port_config(
                port,
                if input.inspection.selected_players.contains(&port) {
                    1
                } else {
                    0
                },
            ));
        }
    }
    let directory = tempfile::Builder::new()
        .prefix("lunchbox-mame-calibration-")
        .tempdir()?;
    let path = directory.path().join("controllers.cfg");
    std::fs::write(&path, config)?;
    input
        .files
        .push((path.clone(), lunchbox_controller_probe::file_hash(&path)?));
    let mut staged = plan.clone();
    attach_config(
        &mut staged,
        &EmulatorExecutable::Native(input.inspection.request.retroarch.clone()),
        &path,
    )?;
    let mut session = CalibratedLaunch {
        #[cfg(target_os = "linux")]
        mgba: None,
        #[cfg(target_os = "linux")]
        dolphin_native: None,
        #[cfg(target_os = "linux")]
        snes9x_native: None,
        #[cfg(target_os = "linux")]
        fceux_native: None,
        #[cfg(target_os = "linux")]
        sameboy_native: None,
        bsnes_native: None,
        #[cfg(target_os = "linux")]
        mednafen_native: None,
        #[cfg(target_os = "linux")]
        mame_native: None,
        #[cfg(target_os = "linux")]
        flycast_native: None,
        #[cfg(target_os = "linux")]
        pcsx2_native: None,
        #[cfg(target_os = "linux")]
        rpcs3_native: None,
        #[cfg(target_os = "linux")]
        melonds_native: None,
        #[cfg(target_os = "linux")]
        ppsspp: None,
        #[cfg(target_os = "linux")]
        duckstation: None,
        _directory: Some(directory),
        bizhawk: None,
        #[cfg(target_os = "linux")]
        bizhawk_topology: None,
        transports,
        crocods: None,
        ep128emu: None,
        hatari: None,
        simcp: None,
        steemsse: None,
        scummvm: None,
        dolphin: None,
        same_cdi: None,
        fbneo: None,
        mame: None,
        puae: None,
        #[cfg(target_os = "linux")]
        stella: None,
        description: format!(
            "Applied runtime-inspected MAME mappings — {}",
            players
                .keys()
                .map(|port| format!(
                    "P{port}: {}",
                    player_digital_layouts
                        .get(port)
                        .copied()
                        .unwrap_or(digital_layout)
                        .label()
                ))
                .collect::<Vec<_>>()
                .join("; ")
        ),
    };
    session.attach_mame_input(input, &mut staged, frontend_save, frontend_state)?;
    session.check_launch_inputs()?;
    check_preparation_cancel(cancel)?;
    *plan = staged;
    Ok(session)
}

impl PreparedMameInput {
    pub(crate) fn verify_inputs(&self, plan: &LaunchPlan) -> Result<()> {
        use sha2::{Digest, Sha256};
        ensure!(
            plan == &self.inspection.launch_identity,
            "MAME launch changed after inspection"
        );
        self.inspection.configuration.verify()?;
        for (path, expected) in &self.inspection.source_hashes {
            let mut input = std::fs::File::open(path)?;
            let mut hash = Sha256::new();
            let mut buffer = [0u8; 65536];
            loop {
                let count = input.read(&mut buffer)?;
                if count == 0 {
                    break;
                }
                hash.update(&buffer[..count]);
            }
            let actual: [u8; 32] = hash.finalize().into();
            ensure!(
                actual == *expected,
                "MAME input changed after inspection: {}",
                path.display()
            );
        }
        for (path, expected) in &self.files {
            ensure!(
                lunchbox_controller_probe::file_hash(path)? == *expected,
                "Staged MAME configuration changed"
            );
        }
        if !self.inspection.relative_button_assignments.is_empty() {
            crate::controller_mame::validate_mouse_button_configuration_tree(
                &self.root,
                &self.inspection.fields,
                &std::sync::atomic::AtomicBool::new(false),
            )?;
        }
        for path in self.persistent.directories() {
            ensure!(
                path.is_dir() && path.canonicalize()? == path,
                "MAME persistent directory changed before launch"
            );
        }
        let actual_state = crate::controller_mame::persistent_inputs(
            &self.persistent,
            &std::sync::atomic::AtomicBool::new(false),
        )?;
        let mut inspected_state: Vec<_> = self
            .inspection
            .request
            .inputs
            .iter()
            .filter(|input| {
                input.destination.starts_with("nvram") || input.destination.starts_with("diff")
            })
            .cloned()
            .collect();
        inspected_state.sort_by(|a, b| a.destination.cmp(&b.destination));
        ensure!(
            actual_state == inspected_state,
            "MAME persistent state files changed after inspection; inspect again before launch"
        );
        for path in &self.frontend_persistent {
            ensure!(
                path.is_dir() && path.canonicalize()? == *path,
                "MAME frontend persistent directory changed before launch"
            );
        }
        Ok(())
    }
}

pub(crate) fn stage_mame_input(
    inspection: MameLaunchInspection,
    persistent: crate::controller_mame::PersistentPaths,
) -> Result<PreparedMameInput> {
    ensure!(
        inspection.mapping.unhandled_fields.is_empty(),
        "MAME has unmapped active fields; resolve their native input contracts before launch"
    );
    let directory = tempfile::Builder::new()
        .prefix("lunchbox-mame-session-")
        .tempdir()?;
    let root = directory.path().canonicalize()?;
    for name in [
        "roms", "cfg", "ctrlr", "system", "saves", "states", "snaps", "nvram", "diff", "input",
    ] {
        std::fs::create_dir(root.join(name))?;
    }
    let original_filename = inspection
        .launch_identity
        .retroarch_content
        .as_ref()
        .context("Missing inspected MAME content identity")?
        .content
        .file_name()
        .context("MAME content has no filename")?;
    // RetroArch derives per-content save/state names from this basename.
    let command_path = root.join(original_filename).with_extension("cmd");
    let command = crate::controller_mame::session_command_with_mouse(
        &root,
        &inspection.request.machine,
        &persistent,
        inspection.request.inspect_mouse,
    )?;
    let mut files = Vec::new();
    // Never promote the inspection tree: it may contain newly-created state,
    // cfg files or differencing images that were not in the baseline manifest.
    let mut destinations = std::collections::BTreeSet::new();
    for input in &inspection.request.inputs {
        let parts: Vec<_> = input.destination.components().collect();
        ensure!(
            parts.len() >= 2
                && parts
                    .iter()
                    .all(|part| matches!(part, std::path::Component::Normal(_))),
            "MAME staging destination must be a contained relative file"
        );
        ensure!(
            matches!(
                parts[0].as_os_str().to_str(),
                Some("roms" | "cfg" | "ctrlr" | "system" | "nvram" | "diff")
            ),
            "Invalid MAME staging category"
        );
        ensure!(
            destinations.insert(input.destination.clone()),
            "Duplicate MAME staging destination"
        );
        let expected = inspection
            .source_hashes
            .iter()
            .find(|(path, _)| path == &input.source)
            .context("Missing inspected MAME source identity")?
            .1;
        let expected = expected
            .iter()
            .map(|byte| format!("{byte:02x}"))
            .collect::<String>();
        let target = root.join(&input.destination);
        for (category, persistent_root) in
            [("nvram", &persistent.nvram), ("diff", &persistent.diff)]
        {
            if let Ok(relative) = input.destination.strip_prefix(category) {
                ensure!(
                    input.source.canonicalize()?
                        == persistent_root.join(relative).canonicalize()?,
                    "MAME persistent state does not match the inspected baseline"
                );
            }
        }
        std::fs::create_dir_all(target.parent().context("Missing MAME staging parent")?)?;
        std::fs::copy(&input.source, &target)?;
        ensure!(
            lunchbox_controller_probe::file_hash(&target)? == expected,
            "MAME input changed while restoring launch baseline"
        );
        if input.destination != Path::new("ctrlr/lunchbox-original.cfg")
            && input.destination
                != std::path::PathBuf::from(format!("cfg/{}.cfg", inspection.request.machine))
        {
            files.push((target, expected));
        }
    }
    let options_path = root.join("core-options.cfg");
    ensure!(
        std::fs::read_to_string(inspection.directory.path().join("core-options.cfg"))?
            == inspection.request.core_options,
        "MAME core options changed during inspection"
    );
    std::fs::write(&options_path, &inspection.request.core_options)?;
    files.push((
        options_path.clone(),
        lunchbox_controller_probe::file_hash(&options_path)?,
    ));
    for (path, contents) in [
        (
            root.join("ctrlr/lunchbox-original.cfg"),
            inspection.mapping.controller_xml.as_str(),
        ),
        (
            root.join("cfg")
                .join(format!("{}.cfg", inspection.request.machine)),
            inspection.mapping.game_config_xml.as_str(),
        ),
        (command_path.clone(), command.as_str()),
    ] {
        std::fs::write(&path, contents)?;
        files.push((path.clone(), lunchbox_controller_probe::file_hash(&path)?));
    }
    let prepared = PreparedMameInput {
        #[cfg(all(target_os = "linux", target_pointer_width = "64"))]
        relative_launch_plan: None,
        #[cfg(all(target_os = "linux", target_pointer_width = "64"))]
        relative_pending: None,
        #[cfg(all(target_os = "linux", target_pointer_width = "64"))]
        relative_bridge: None,
        _directory: directory,
        persistent,
        frontend_persistent: Vec::new(),
        inspection,
        root,
        command_path,
        files,
    };
    prepared.verify_inputs(&prepared.inspection.launch_identity)?;
    Ok(prepared)
}

fn cfg_value(text: &str, key: &str) -> Result<Option<String>> {
    let mut result = None;
    for line in text.lines() {
        let Some((name, value)) = line.trim().split_once('=') else {
            continue;
        };
        if name.trim() != key {
            continue;
        }
        ensure!(
            name.ends_with(|c: char| c.is_ascii_whitespace()),
            "RetroArch requires whitespace before '=' in {key}"
        );
        ensure!(result.is_none(), "Duplicate {key} setting");
        let value = value.trim();
        let (value, rest) = if let Some(quoted) = value.strip_prefix('"') {
            quoted
                .split_once('"')
                .with_context(|| format!("Unterminated {key} setting"))?
        } else {
            let end = value
                .find(|c: char| c.is_ascii_whitespace() || c == '#')
                .unwrap_or(value.len());
            ensure!(end > 0, "Missing {key} value");
            (&value[..end], &value[end..])
        };
        ensure!(
            rest.trim().is_empty() || rest.trim_start().starts_with('#'),
            "Unresolved {key} setting suffix"
        );
        result = Some(value.to_owned());
    }
    Ok(result)
}

fn core_options_overlay(baseline: &str, options: &BTreeMap<String, String>) -> Result<String> {
    ensure!(
        !baseline
            .lines()
            .any(|line| line.trim_start().starts_with("#include")),
        "Included core-options files require an adapter upgrade before calibrated launch"
    );
    let mut output = baseline
        .lines()
        .filter(|line| {
            !line
                .split_once('=')
                .is_some_and(|(key, _)| options.contains_key(key.trim()))
        })
        .map(|line| format!("{line}\n"))
        .collect::<String>();
    for (key, value) in options {
        ensure!(
            key.bytes()
                .all(|b| b.is_ascii_alphanumeric() || b == b'-' || b == b'_')
                && !value.contains(['"', '\n', '\r', '\\']),
            "Invalid core option"
        );
        output.push_str(&format!("{key} = \"{value}\"\n"));
    }
    Ok(output)
}

fn read_optional(path: &Path) -> Result<String> {
    match std::fs::read_to_string(path) {
        Ok(value) => Ok(value),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(String::new()),
        Err(error) => Err(error).with_context(|| format!("Reading {}", path.display())),
    }
}

fn retroarch_base(executable: &EmulatorExecutable) -> Result<(std::path::PathBuf, String)> {
    let dirs = directories::BaseDirs::new().context("Finding RetroArch config directory")?;
    let base = match executable {
        EmulatorExecutable::Flatpak { app_id, .. } => dirs
            .home_dir()
            .join(".var/app")
            .join(app_id)
            .join("config/retroarch"),
        _ => dirs.config_dir().join("retroarch"),
    };
    let path = base.join("retroarch.cfg");
    ensure!(
        path.exists()
            || matches!(executable, EmulatorExecutable::Flatpak { .. })
            || !dirs.home_dir().join(".retroarch.cfg").exists(),
        "Legacy ~/.retroarch.cfg needs explicit configuration resolution for calibrated launch"
    );
    Ok((base, read_optional(&path)?))
}

fn configured_path(value: &str) -> Result<std::path::PathBuf> {
    let path = if let Some(relative) = value.strip_prefix("~/") {
        directories::BaseDirs::new()
            .context("Finding RetroArch home directory")?
            .home_dir()
            .join(relative)
    } else {
        std::path::PathBuf::from(value)
    };
    ensure!(
        path.is_absolute(),
        "Custom relative RetroArch paths need effective configuration resolution"
    );
    Ok(path)
}

fn emulator_arguments<'a>(
    plan: &'a LaunchPlan,
    executable: &EmulatorExecutable,
) -> Result<&'a [OsString]> {
    match executable {
        EmulatorExecutable::Flatpak { app_id, .. } => {
            let boundary = plan
                .arguments
                .iter()
                .position(|arg| arg.to_str() == Some(app_id))
                .context("Missing Flatpak app boundary")?;
            Ok(&plan.arguments[boundary + 1..])
        }
        _ => Ok(&plan.arguments),
    }
}

fn write_core_options(
    profile: &EmulatorProfile,
    plan: &LaunchPlan,
    executable: &EmulatorExecutable,
    directory: &Path,
) -> Result<String> {
    if profile.core_options.is_empty() {
        return Ok(String::new());
    }
    ensure!(
        !plan.arguments.iter().any(|arg| arg == "--config"
            || arg == "-c"
            || arg.to_string_lossy().starts_with("--config=")
            || arg.to_string_lossy().starts_with("--appendconfig")),
        "Custom RetroArch configuration arguments need core-options resolution before calibrated launch with core options"
    );
    let (base, config) = retroarch_base(executable)?;
    let baseline = if let (Some(content), Some(library)) =
        (&plan.retroarch_content, &profile.retroarch_library)
    {
        let directory = cfg_value(&config, "rgui_config_directory")?
            .filter(|value| !value.is_empty())
            .map(|value| configured_path(&value))
            .transpose()?
            .unwrap_or_else(|| base.join("config"));
        effective_mode_core_options(&base, &config, &directory, library, &content.content)?
    } else {
        read_core_options(&base, &config)?
    };
    write_core_options_snapshot(profile, &baseline, directory)
}

struct PreparedSameCdiInput {
    configuration: crate::controller_same_cdi::PreparedConfiguration,
    system_directory: std::path::PathBuf,
    save_directory: std::path::PathBuf,
    state_directory: std::path::PathBuf,
    state_canonical: std::path::PathBuf,
}

impl PreparedSameCdiInput {
    fn verify(&self) -> Result<()> {
        self.configuration.verify()?;
        ensure!(
            self.state_directory.is_dir()
                && self.state_directory.canonicalize()? == self.state_canonical,
            "SAME CD-i state directory changed during preparation"
        );
        Ok(())
    }

    fn append_config(&self) -> Result<String> {
        let encode = |path: &Path| -> Result<String> {
            let value = path
                .to_str()
                .context("SAME CD-i frontend directory is not UTF-8")?;
            ensure!(
                !value.chars().any(|c| c.is_control() || "\"\\".contains(c)),
                "SAME CD-i directory cannot be represented in the private RetroArch config"
            );
            Ok(value.to_owned())
        };
        let system = encode(&self.system_directory)?;
        let save = encode(&self.save_directory)?;
        let state = encode(&self.state_directory)?;
        // The core receives an owned .cmd file, but save sorting must continue
        // to refer to the original disc's directory, not that temporary file.
        Ok(format!(
            "system_directory = \"{system}\"\nsavefile_directory = \"{save}\"\nsort_savefiles_enable = \"false\"\nsort_savefiles_by_content_enable = \"false\"\nsavefiles_in_content_dir = \"false\"\nsavestate_directory = \"{state}\"\nsort_savestates_enable = \"false\"\nsort_savestates_by_content_enable = \"false\"\nsavestates_in_content_dir = \"false\"\n"
        ))
    }
}

fn prepare_same_cdi_input(
    option: &RomEmulatorOption,
    plan: &LaunchPlan,
    profile: &EmulatorProfile,
) -> Result<PreparedSameCdiInput> {
    ensure!(
        profile.core == "same_cdi"
            && profile.explicit_selection
            && profile.retroarch_library.as_deref() == Some("SAME_CDI")
            && profile.requires_fresh_start
            && profile.frontend_ports == Some(6)
            && matches!(&option.executable, EmulatorExecutable::Native(_))
            && plan.environment.is_empty(),
        "SAME CD-i fixed pointer preparation requires an explicit native mode without custom environment"
    );
    crate::controller_same_cdi::validate_options(&profile.core_options)?;
    let mode = crate::controller_same_cdi::mode_for_layout(&profile.target_layout)?;
    let content = plan
        .retroarch_content
        .as_ref()
        .context("SAME CD-i requires original prepared disc identity")?;
    crate::controller_launch_modes::validate_arguments(
        emulator_arguments(plan, &option.executable)?,
        content,
    )?;
    let (base_path, base) = retroarch_base(&option.executable)?;
    ensure!(
        !base
            .lines()
            .any(|line| line.trim_start().starts_with("#include")),
        "SAME CD-i requires included RetroArch configuration to be resolved"
    );
    let options_directory = cfg_value(&base, "rgui_config_directory")?
        .filter(|value| !value.is_empty())
        .map(|value| configured_path(&value))
        .transpose()?
        .unwrap_or_else(|| base_path.join("config"));
    let baseline = effective_mode_core_options(
        &base_path,
        &base,
        &options_directory,
        "SAME_CDI",
        &content.content,
    )?;
    ensure!(
        cfg_value(&baseline, "same_cdi_read_config")?
            .as_deref()
            .unwrap_or("disabled")
            == "disabled",
        "SAME CD-i native INI reading is enabled; resolve those settings before selecting fixed pointer mode. Native files were not changed."
    );
    let system = cfg_value(&base, "system_directory")?
        .filter(|value| !value.is_empty() && value != "default")
        .context("SAME CD-i requires an explicit system_directory")?;
    let system_directory = configured_path(&system)?;
    for key in [
        "sort_savefiles_enable",
        "sort_savefiles_by_content_enable",
        "savefiles_in_content_dir",
    ] {
        ensure!(
            cfg_value(&base, key)?.is_some(),
            "SAME CD-i requires explicit {key} to preserve original-disc save routing"
        );
    }
    let parent = content
        .content
        .parent()
        .context("SAME CD-i disc has no parent")?;
    let mut save_directory = if config_bool(&base, "savefiles_in_content_dir", false)? {
        parent.to_path_buf()
    } else {
        let value = cfg_value(&base, "savefile_directory")?
            .filter(|value| !value.is_empty() && value != "default")
            .context("SAME CD-i requires an explicit savefile_directory")?;
        configured_path(&value)?
    };
    if config_bool(&base, "sort_savefiles_by_content_enable", false)? {
        save_directory.push(
            parent
                .file_name()
                .context("SAME CD-i save sorting requires a named disc directory")?,
        );
    }
    if config_bool(&base, "sort_savefiles_enable", false)? {
        save_directory.push("SAME_CDI");
    }
    for key in [
        "sort_savestates_enable",
        "sort_savestates_by_content_enable",
        "savestates_in_content_dir",
    ] {
        ensure!(
            cfg_value(&base, key)?.is_some(),
            "SAME CD-i requires explicit {key} to preserve original-disc state routing"
        );
    }
    let mut state_directory = if config_bool(&base, "savestates_in_content_dir", false)? {
        parent.to_path_buf()
    } else {
        let value = cfg_value(&base, "savestate_directory")?
            .filter(|value| !value.is_empty() && value != "default")
            .context("SAME CD-i requires an explicit savestate_directory")?;
        configured_path(&value)?
    };
    if config_bool(&base, "sort_savestates_by_content_enable", false)? {
        state_directory.push(
            parent
                .file_name()
                .context("SAME CD-i state sorting requires a named disc directory")?,
        );
    }
    if config_bool(&base, "sort_savestates_enable", false)? {
        state_directory.push("SAME_CDI");
    }
    ensure!(
        state_directory.is_absolute() && state_directory.is_dir(),
        "SAME CD-i requires the resolved state directory to exist before command-file substitution"
    );
    let prepared = PreparedSameCdiInput {
        configuration: crate::controller_same_cdi::prepare_configuration(
            &system_directory,
            &save_directory,
            &content.content,
            mode,
        )?,
        system_directory,
        save_directory,
        state_canonical: state_directory.canonicalize()?,
        state_directory,
    };
    prepared.append_config()?;
    prepared.verify()?;
    Ok(prepared)
}

struct PreparedPuaeInput {
    inputs: crate::controller_puae::InputSnapshot,
    save_directory: std::path::PathBuf,
}

impl PreparedPuaeInput {
    fn append_config(&self) -> Result<String> {
        let path = self
            .save_directory
            .to_str()
            .context("PUAE save directory is not UTF-8")?;
        ensure!(
            !path.chars().any(|c| c.is_control() || "\"\\".contains(c)),
            "PUAE save directory cannot be represented in the private RetroArch config"
        );
        // Preserve the resolved save location while preventing a second sorting
        // pass. PUAE applies its own one-letter-directory normalization later.
        Ok(format!(
            "savefile_directory = \"{path}\"\nsort_savefiles_enable = \"false\"\nsort_savefiles_by_content_enable = \"false\"\nsavefiles_in_content_dir = \"false\"\n"
        ))
    }
}

fn prepare_puae_input(
    option: &RomEmulatorOption,
    plan: &LaunchPlan,
    profile: &EmulatorProfile,
) -> Result<PreparedPuaeInput> {
    ensure!(
        matches!(&option.executable, EmulatorExecutable::Native(_)) && plan.environment.is_empty(),
        "PUAE custom input resolution currently requires a native launch without custom environment; Flatpak needs a namespace adapter"
    );
    ensure!(
        profile.core == "puae" && profile.explicit_selection && profile.requires_fresh_start,
        "PUAE input resolution requires an explicit fresh-start PUAE profile"
    );
    let content = plan
        .retroarch_content
        .as_ref()
        .context("PUAE requires exact prepared content")?;
    crate::controller_launch_modes::validate_arguments(
        emulator_arguments(plan, &option.executable)?,
        content,
    )?;
    let (_, config) = retroarch_base(&option.executable)?;
    ensure!(
        !config
            .lines()
            .any(|line| line.trim_start().starts_with("#include")),
        "PUAE save-directory resolution requires included RetroArch configs to be resolved first"
    );
    // Explicit values avoid depending on platform/build-specific defaults.
    for key in [
        "sort_savefiles_enable",
        "sort_savefiles_by_content_enable",
        "savefiles_in_content_dir",
    ] {
        ensure!(
            cfg_value(&config, key)?.is_some(),
            "PUAE needs an explicit {key} setting to resolve its custom configuration directory"
        );
    }
    let parent = content
        .content
        .parent()
        .context("PUAE content has no parent directory")?;
    let in_content = config_bool(&config, "savefiles_in_content_dir", false)?;
    let mut save_directory = if in_content {
        parent.to_path_buf()
    } else {
        let value = cfg_value(&config, "savefile_directory")?
            .filter(|value| !value.is_empty() && value != "default")
            .context("PUAE needs an explicit savefile_directory or savefiles_in_content_dir; platform default save paths remain unresolved")?;
        configured_path(&value)?
    };
    if config_bool(&config, "sort_savefiles_by_content_enable", false)? {
        save_directory.push(
            parent
                .file_name()
                .context("PUAE content-directory sorting requires a named parent directory")?,
        );
    }
    if config_bool(&config, "sort_savefiles_enable", false)? {
        save_directory.push("PUAE");
    }
    ensure!(
        save_directory.is_dir(),
        "PUAE resolved save directory must already exist so RetroArch cannot fall back after a failed directory creation: {}",
        save_directory.display()
    );
    let model = profile
        .core_options
        .get("puae_model")
        .context("PUAE profile needs a fixed machine model")?;
    let inputs = crate::controller_puae::prepare(&save_directory, &content.content, model)?;
    let prepared = PreparedPuaeInput {
        inputs,
        save_directory,
    };
    prepared.append_config()?;
    Ok(prepared)
}

fn prepare_hatari_input(
    option: &RomEmulatorOption,
    plan: &LaunchPlan,
) -> Result<crate::controller_hatari::InputSnapshot> {
    ensure!(
        matches!(&option.executable, EmulatorExecutable::Native(_)) && plan.environment.is_empty(),
        "Hatari configuration resolution requires native RetroArch without custom environment"
    );
    let content = plan
        .retroarch_content
        .as_ref()
        .context("Hatari requires prepared content identity")?;
    crate::controller_launch_modes::validate_arguments(
        emulator_arguments(plan, &option.executable)?,
        content,
    )?;
    let (_, config) = retroarch_base(&option.executable)?;
    ensure!(
        !config
            .lines()
            .any(|line| line.trim_start().starts_with("#include")),
        "Hatari requires included RetroArch configuration to be resolved first"
    );
    let system = cfg_value(&config, "system_directory")?
        .filter(|value| !value.is_empty() && value != "default")
        .context("Hatari requires an explicit system_directory containing tos.img")?;
    let system = configured_path(&system)?;
    for key in [
        "sort_savefiles_enable",
        "sort_savefiles_by_content_enable",
        "savefiles_in_content_dir",
    ] {
        ensure!(
            cfg_value(&config, key)?.is_some(),
            "Hatari needs an explicit {key} setting to locate its native configuration"
        );
    }
    let parent = content
        .content
        .parent()
        .context("Hatari content has no parent")?;
    let mut save = if config_bool(&config, "savefiles_in_content_dir", false)? {
        parent.to_path_buf()
    } else {
        let value = cfg_value(&config, "savefile_directory")?
            .filter(|value| !value.is_empty() && value != "default")
            .context("Hatari requires an explicit savefile_directory")?;
        configured_path(&value)?
    };
    if config_bool(&config, "sort_savefiles_by_content_enable", false)? {
        save.push(
            parent
                .file_name()
                .context("Hatari save sorting requires a named content directory")?,
        );
    }
    if config_bool(&config, "sort_savefiles_enable", false)? {
        // Exact library_name from the pinned Hatari libretro contract.
        save.push("hatari");
    }
    crate::controller_hatari::prepare(&system, &save)
}

fn read_core_options(base: &Path, config: &str) -> Result<String> {
    ensure!(
        !config
            .lines()
            .any(|line| line.trim_start().starts_with("#include")),
        "Included RetroArch configs require core-options resolution before calibrated launch with core options"
    );
    let path = match cfg_value(config, "core_options_path")?.filter(|v| !v.is_empty()) {
        Some(path) => configured_path(&path)?,
        None => base.join("retroarch-core-options.cfg"),
    };
    read_optional(&path)
}

fn config_bool(config: &str, key: &str, default: bool) -> Result<bool> {
    let mut result = None;
    for line in config.lines().map(str::trim) {
        let Some((name, value)) = line.split_once('=') else {
            continue;
        };
        if name.trim() != key {
            continue;
        }
        ensure!(
            name.ends_with(|c: char| c.is_ascii_whitespace()),
            "RetroArch requires whitespace before '=' in {key}"
        );
        ensure!(result.is_none(), "Duplicate {key} setting");
        let value = value.trim();
        let value = if let Some(quoted) = value.strip_prefix('"') {
            let (value, rest) = quoted
                .split_once('"')
                .context("Unterminated boolean setting")?;
            ensure!(
                rest.trim().is_empty() || rest.trim_start().starts_with('#'),
                "Invalid boolean setting suffix"
            );
            value
        } else {
            value.split('#').next().unwrap().trim()
        };
        result = Some(match value {
            "true" => true,
            "false" => false,
            _ => bail!("Unresolved {key} boolean setting"),
        });
    }
    Ok(result.unwrap_or(default))
}

fn effective_mode_core_options(
    base: &Path,
    config: &str,
    config_directory: &Path,
    library: &str,
    content: &Path,
) -> Result<String> {
    // RetroArch 1.22.2 runloop_init_core_options_path selects ONE file, not a
    // merge: game -> folder -> per-core -> global. Its defaults enable game
    // options and per-core storage (global_core_options = false).
    let directory = config_directory.join(library);
    let mut candidates = Vec::new();
    if config_bool(config, "game_specific_options", true)? {
        let game = content
            .file_stem()
            .context("Prepared content has no game basename")?;
        let folder = content
            .parent()
            .and_then(Path::file_name)
            .context("Prepared content has no folder basename")?;
        for name in [game, folder] {
            let mut filename = name.to_os_string();
            filename.push(".opt");
            candidates.push(directory.join(filename));
        }
    }
    if !config_bool(config, "global_core_options", false)? {
        candidates.push(directory.join(format!("{library}.opt")));
    }
    for path in candidates {
        if path.exists() {
            return read_optional(&path);
        }
    }
    read_core_options(base, config)
}

fn write_core_options_snapshot(
    profile: &EmulatorProfile,
    baseline: &str,
    directory: &Path,
) -> Result<String> {
    if profile.core_options.is_empty() {
        return Ok(String::new());
    }
    let options = core_options_overlay(baseline, &profile.core_options)?;
    write_options_file(&options, directory)
}

fn write_options_file(options: &str, directory: &Path) -> Result<String> {
    let output = directory.join("core-options.cfg");
    let output_text = output.to_str().context("Core-options path must be UTF-8")?;
    ensure!(
        !output_text.contains(['"', '\n', '\r', '\\']),
        "Core-options path cannot be encoded"
    );
    std::fs::write(&output, options)?;
    // RetroArch may save a per-core .opt under this directory even with
    // game_specific_options disabled. Keep that exit-time write private too.
    let private_config = directory.join("config");
    std::fs::create_dir_all(&private_config)?;
    let private_config = private_config
        .to_str()
        .context("Private config path must be UTF-8")?;
    Ok(format!(
        "game_specific_options = \"false\"\nglobal_core_options = \"false\"\ncore_options_path = \"{output_text}\"\nrgui_config_directory = \"{private_config}\"\n"
    ))
}

/// Resolve the same effective options file that the selected native Stella
/// launch would read. Preserve this map for the later private launch snapshot;
/// detecting with defaults and launching with per-game options is not valid.
#[cfg(target_os = "linux")]
pub(crate) struct PreparedStellaDetection {
    pub context: crate::controller_stella::NativeDetection,
    pub options: BTreeMap<String, String>,
}

#[cfg(target_os = "linux")]
pub(crate) fn prepare_stella_detection(
    option: &RomEmulatorOption,
    plan: &LaunchPlan,
) -> Result<PreparedStellaDetection> {
    ensure!(
        option.runtime_kind == EmulatorRuntimeKind::RetroArch && option.core_name == "stella",
        "Stella detection requires the selected Stella libretro launch"
    );
    let EmulatorExecutable::Native(executable) = &option.executable else {
        bail!("Stella detection for this runtime still needs its native path/config adapter");
    };
    ensure!(
        &plan.program == executable && plan.environment.is_empty(),
        "Custom Stella launch executable/environment requires matching detection-context resolution"
    );
    let prepared = plan
        .retroarch_content
        .as_ref()
        .context("Stella detection requires exact prepared core/content identity")?;
    crate::controller_launch_modes::validate_arguments(
        emulator_arguments(plan, &option.executable)?,
        prepared,
    )?;
    let (base, config) = retroarch_base(&option.executable)?;
    ensure!(
        !config
            .lines()
            .any(|line| line.trim_start().starts_with("#include")),
        "Included RetroArch configuration needs resolution before Stella detection"
    );
    let directory = cfg_value(&config, "rgui_config_directory")?
        .filter(|value| !value.is_empty())
        .map(|value| configured_path(&value))
        .transpose()?
        .unwrap_or_else(|| base.join("config"));
    let baseline =
        effective_mode_core_options(&base, &config, &directory, "Stella 2023", &prepared.content)?;
    let options = parse_detection_options(&baseline)?;
    let context = crate::controller_stella::NativeDetection::prepare(
        executable,
        &prepared.core,
        &prepared.content,
        &options,
    )?;
    Ok(PreparedStellaDetection { context, options })
}

fn parse_detection_options(text: &str) -> Result<BTreeMap<String, String>> {
    ensure!(
        text.len() <= 1024 * 1024,
        "Core options exceed the detection snapshot limit"
    );
    let mut options = BTreeMap::new();
    for line in text.lines() {
        let line = line.trim();
        ensure!(
            !line.starts_with("#include"),
            "Included core options need resolution before controller detection"
        );
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        let (key, _) = line
            .split_once('=')
            .context("Malformed core option in detection snapshot")?;
        let key = key.trim();
        ensure!(
            !key.is_empty()
                && key
                    .bytes()
                    .all(|byte| byte.is_ascii_alphanumeric() || byte == b'_'),
            "Invalid core option name in detection snapshot"
        );
        let value =
            cfg_value(line, key)?.context("Missing core option value in detection snapshot")?;
        ensure!(
            options.insert(key.to_owned(), value).is_none(),
            "Duplicate core option in detection snapshot"
        );
    }
    Ok(options)
}

/// Read-only runtime numbering, independent of the labels printed on a pad.
#[derive(Debug, PartialEq, Eq)]
pub struct JoydevMap {
    pub index: usize,
    pub buttons: Vec<u16>,
    pub axes: Vec<u8>,
}

#[cfg(target_os = "linux")]
fn measured_event_path<'a>(
    joystick: &Path,
    events: &'a [std::path::PathBuf],
    sys_input: &Path,
) -> Result<(&'a Path, std::path::PathBuf)> {
    let identity = |path: &Path, prefix: &str| -> Result<std::path::PathBuf> {
        ensure!(
            path.parent() == Some(Path::new("/dev/input")),
            "Expected a physical input node"
        );
        let name = path
            .file_name()
            .and_then(|name| name.to_str())
            .context("Input node name is not UTF-8")?;
        ensure!(
            name.strip_prefix(prefix)
                .is_some_and(|n| !n.is_empty() && n.bytes().all(|b| b.is_ascii_digit())),
            "Unexpected input node name"
        );
        Ok(sys_input.join(name).join("device").canonicalize()?)
    };
    let expected = identity(joystick, "js")?;
    let mut selected = None;
    for event in events {
        if identity(event, "event")? == expected {
            ensure!(
                selected.is_none(),
                "Ambiguous physical event node for calibrated controller"
            );
            selected = Some(event.as_path());
        }
    }
    Ok((
        selected
            .context("No exact evdev node for the selected joystick; reconnect the controller")?,
        expected,
    ))
}

/// The same physical input mode that produced the gesture must still be present
/// when its per-launch mapping is written. Older button-number calibrations stay
/// readable; missing axis measurements are never fabricated for them.
fn validated_numbering(
    calibration: &Calibration,
    profile: &EmulatorProfile,
    device: &ControllerDevice,
) -> Result<JoydevMap> {
    let plan = calibration.plan_profile(profile)?;
    let numbering = JoydevMap::read(&device.device_path)?;
    let measurements = plan
        .rows
        .iter()
        .filter_map(|row| {
            let input = row.input.as_ref()?;
            Some((input.native.as_ref()?.code, input.axis.as_ref()?))
        })
        .collect::<Vec<_>>();
    if measurements.is_empty() {
        return Ok(numbering);
    }
    #[cfg(target_os = "linux")]
    {
        let sys_input = Path::new("/sys/class/input");
        let (event, identity) =
            measured_event_path(&device.device_path, &device.event_paths, sys_input)?;
        crate::controller_axis::validate_recorded_axes(event, measurements)?;
        ensure!(
            JoydevMap::read(&device.device_path)? == numbering
                && measured_event_path(&device.device_path, &device.event_paths, sys_input)?.1
                    == identity,
            "Controller changed while preparing its mapping; try launching again"
        );
        Ok(numbering)
    }
    #[cfg(not(target_os = "linux"))]
    bail!("Physical axis validation requires the recorded Linux input backend")
}

#[cfg(all(target_os = "linux", target_pointer_width = "64"))]
fn prepare_native_normalized_transport(
    calibration: &Calibration,
    device: &ControllerDevice,
    digital_layout: Option<&str>,
    dualshock: bool,
    dualanalog: bool,
    analog_joystick: bool,
    rhythm: Option<crate::controller_bizhawk::DigitalPeripheral>,
    negcon: bool,
    pointer: Option<crate::controller_bizhawk::PointerPeripheral>,
    desktop_cursor: bool,
    analog_toggle: Option<&str>,
    cancel: &std::sync::atomic::AtomicBool,
) -> Result<(PlayerTransport, Calibration, std::path::PathBuf)> {
    check_preparation_cancel(cancel)?;
    calibration.validate()?;
    ensure!(
        [
            dualshock,
            dualanalog,
            analog_joystick,
            rhythm.is_some(),
            negcon,
            pointer.is_some()
        ]
        .into_iter()
        .filter(|mode| *mode)
        .count()
            <= 1
            && dualshock == analog_toggle.is_some(),
        "DualShock requires its explicit analog toggle"
    );
    let source = catalog()
        .layout(&calibration.layout)
        .context("Unknown physical controller layout")?;
    let target = catalog()
        .layout(if let Some(layout) = digital_layout {
            layout
        } else if let Some(kind) = pointer {
            kind.layout_id()
        } else if negcon {
            "playstation-negcon"
        } else if let Some(kind) = rhythm {
            kind.layout_id()
        } else if analog_joystick {
            "playstation-analog-joystick"
        } else if dualshock || dualanalog {
            "dualshock"
        } else {
            "playstation-digital"
        })
        .context("Missing native controller target layout")?;
    let available = calibration
        .bindings
        .keys()
        .map(String::as_str)
        .filter(|id| Some(*id) != analog_toggle)
        .collect();
    let requested = target
        .controls
        .iter()
        .filter(|control| !desktop_cursor || !control.analog)
        .map(|control| control.id.as_str())
        .collect();
    let resolution = crate::controller_bizhawk::guided::resolve(
        calibration,
        source,
        target,
        &available,
        &requested,
    )?;
    let mut required = std::collections::BTreeSet::new();
    for control in &target.controls {
        if desktop_cursor && control.analog {
            continue;
        }
        required.insert(
            resolution
                .assignments
                .get(&control.id)
                .with_context(|| {
                    format!("No physical assignment for normalized {}", control.label)
                })?
                .clone(),
        );
    }
    if let Some(toggle) = analog_toggle {
        ensure!(
            calibration.bindings.contains_key(toggle),
            "Analog toggle has no physical calibration"
        );
        required.insert(toggle.to_owned());
    }
    if rhythm.is_some() {
        ensure!(
            required.len() == target.controls.len(),
            "Rhythm controls require distinct physical assignments"
        );
        let mut channels = std::collections::BTreeSet::new();
        for id in &required {
            let native = calibration
                .bindings
                .get(id)
                .and_then(|input| input.native.as_ref())
                .context("Rhythm normalization requires physical input identities")?;
            ensure!(
                channels.insert(native.code),
                "Independent rhythm controls share a physical button or axis; resolve calibration before normalization"
            );
        }
    }
    let selected_axes: std::collections::BTreeSet<_> = required
        .iter()
        .filter_map(|id| {
            calibration
                .bindings
                .get(id)
                .and_then(|input| input.native.as_ref())
        })
        .filter(|native| native.code >> 16 == 3)
        .map(|native| native.code)
        .collect();
    let mut physical = calibration.clone();
    // Opposite-axis gestures may not be gameplay assignments, but they still
    // define the full physical travel used by a selected axis's normalizer.
    physical.bindings.retain(|id, input| {
        required.contains(id)
            || input
                .native
                .as_ref()
                .is_some_and(|native| selected_axes.contains(&native.code))
    });
    // This is a launch-local subset. Keep active manual assignments, but do
    // not carry unrelated target overrides referring to excluded controls.
    physical.target_mappings.retain(|_, choices| {
        choices
            .values()
            .all(|id| physical.bindings.contains_key(id))
    });
    let (frame, mut translated) =
        crate::controller_axis::normalized_gamepad_calibration(&physical)?;
    translated.bindings.retain(|id, _| required.contains(id));
    translated.target_mappings.retain(|_, choices| {
        choices
            .values()
            .all(|id| translated.bindings.contains_key(id))
    });
    translated.validate()?;
    let original_numbering = JoydevMap::read(&device.device_path)?;
    let (event, identity) = measured_event_path(
        &device.device_path,
        &device.event_paths,
        Path::new("/sys/class/input"),
    )?;
    crate::controller_axis::validate_recorded_axes(
        event,
        physical.bindings.values().filter_map(|input| {
            input
                .native
                .as_ref()
                .zip(input.axis.as_ref())
                .map(|(native, axis)| (native.code, axis))
        }),
    )?;
    let bridge = crate::controller_axis::GamepadBridge::start(event, frame, &identity, cancel)?;
    ensure!(
        JoydevMap::read(&device.device_path)? == original_numbering
            && measured_event_path(
                &device.device_path,
                &device.event_paths,
                Path::new("/sys/class/input")
            )?
            .1 == identity,
        "Controller changed while starting native normalized transport"
    );
    let deadline = std::time::Instant::now() + std::time::Duration::from_secs(2);
    let path = loop {
        check_preparation_cancel(cancel)?;
        bridge.check_health()?;
        if let Some(path) = bridge.joydev_path()? {
            break path;
        }
        ensure!(
            std::time::Instant::now() < deadline,
            "Normalized controller has no joydev node; check input permissions and joydev support"
        );
        std::thread::sleep(std::time::Duration::from_millis(10));
    };
    let numbering = JoydevMap::read(&path)?;
    Ok((
        PlayerTransport {
            numbering,
            pressure_codes: Default::default(),
            bridge: Some(bridge),
        },
        translated,
        path,
    ))
}

fn prepare_mame_analog_transport(
    calibration: &Calibration,
    profile: &EmulatorProfile,
    device: &ControllerDevice,
    assignments: &[crate::controller_mame::AnalogAssignment],
    player: usize,
    cancel: &std::sync::atomic::AtomicBool,
) -> Result<(PlayerTransport, Calibration)> {
    check_preparation_cancel(cancel)?;
    #[cfg(all(target_os = "linux", target_pointer_width = "64"))]
    {
        let original = validated_numbering(calibration, profile, device)?;
        let (frame, normalized, pressure_codes) = crate::controller_mame::normalized_calibration(
            assignments,
            player,
            calibration,
            profile,
        )?;
        let sys_input = Path::new("/sys/class/input");
        let (event, identity) =
            measured_event_path(&device.device_path, &device.event_paths, sys_input)?;
        let bridge = crate::controller_axis::GamepadBridge::start(event, frame, &identity, cancel)?;
        ensure!(
            JoydevMap::read(&device.device_path)? == original
                && measured_event_path(&device.device_path, &device.event_paths, sys_input)?.1
                    == identity,
            "MAME physical controller changed while starting analog transport"
        );
        let deadline = std::time::Instant::now() + std::time::Duration::from_secs(2);
        let path = loop {
            check_preparation_cancel(cancel)?;
            bridge.check_health()?;
            if let Some(path) = bridge.joydev_path()? {
                break path;
            }
            ensure!(
                std::time::Instant::now() < deadline,
                "MAME normalized controller has no joydev node; check joydev and input permissions"
            );
            std::thread::sleep(std::time::Duration::from_millis(10));
        };
        let numbering = JoydevMap::read(&path)?;
        bridge.check_health()?;
        Ok((
            PlayerTransport {
                numbering,
                pressure_codes,
                bridge: Some(bridge),
            },
            normalized,
        ))
    }
    #[cfg(not(all(target_os = "linux", target_pointer_width = "64")))]
    bail!("MAME measured analog transport requires 64-bit Linux")
}

fn prepare_player_transport(
    calibration: &Calibration,
    profile: &EmulatorProfile,
    device: &ControllerDevice,
    cancel: &std::sync::atomic::AtomicBool,
) -> Result<PlayerTransport> {
    check_preparation_cancel(cancel)?;
    let numbering = validated_numbering(calibration, profile, device)?;
    let plan = calibration.plan_profile(profile)?;
    let target = catalog()
        .layout(&profile.target_layout)
        .context("Unknown controller target")?;
    // A target layout describes every available control, not this profile's
    // requested subset. Stick-only MAME analog profiles must not require an
    // unrequested trigger merely because their target layout has triggers.
    // Keep requested but unmapped pressure rows in this decision so their
    // existing physical-measurement checks cannot be bypassed.
    let needs_pressure = plan.rows.iter().any(|row| {
        target
            .controls
            .iter()
            .any(|control| control.id == row.target_id && control.is_pressure())
    });
    if !needs_pressure {
        return Ok(PlayerTransport {
            numbering,
            pressure_codes: Default::default(),
            #[cfg(all(target_os = "linux", target_pointer_width = "64"))]
            bridge: None,
        });
    }
    #[cfg(all(target_os = "linux", target_pointer_width = "64"))]
    {
        use crate::controller_axis::{GamepadAxis, GamepadBridge, GamepadFrame, PressureAxis};
        let mut buttons = std::collections::BTreeSet::new();
        let mut axes = BTreeMap::new();
        let mut measured_axes = BTreeMap::new();
        let mut pressure_codes = std::collections::BTreeSet::new();
        for row in &plan.rows {
            let control = target
                .controls
                .iter()
                .find(|control| control.id == row.target_id)
                .context("Unknown controller target control")?;
            let Some(input) = &row.input else {
                ensure!(
                    control.optional,
                    "Required controller control is not mapped"
                );
                continue;
            };
            let native = input
                .native
                .as_ref()
                .context("Recalibrate this controller to capture physical inputs")?;
            // Validate each physical binding before replacing its numbering.
            numbering.binding(native)?;
            let code = (native.code & 0xffff) as u16;
            let pressure = control.is_pressure();
            match native.code >> 16 {
                1 => {
                    ensure!(
                        !pressure,
                        "Proportional trigger output requires a measured physical axis"
                    );
                    buttons.insert(code);
                }
                3 => {
                    let measured = input.axis.as_ref().context(
                        "Recalibrate all mapped axes before using normalized trigger transport",
                    )?;
                    measured.validate()?;
                    if let Some((previous, previous_pressure)) = measured_axes.get(&code) {
                        let previous: &crate::controller_axis::AxisMeasurement = previous;
                        ensure!(
                            *previous_pressure == pressure,
                            "One physical axis cannot mix pressure and stick/hat output"
                        );
                        ensure!(
                            previous.minimum == measured.minimum
                                && previous.maximum == measured.maximum
                                && previous.flat == measured.flat
                                && previous.fuzz == measured.fuzz
                                && previous.resolution == measured.resolution
                                && previous.released == measured.released,
                            "Mapped directions disagree about physical axis bounds or neutral; recalibrate"
                        );
                        ensure!(
                            !pressure,
                            "A physical pressure axis cannot supply independent trigger controls"
                        );
                        continue;
                    }
                    let axis = if pressure {
                        pressure_codes.insert(code);
                        GamepadAxis::Pressure(PressureAxis::from_measurement(measured)?)
                    } else {
                        GamepadAxis::Passthrough {
                            minimum: measured.minimum,
                            maximum: measured.maximum,
                            neutral: measured.released,
                        }
                    };
                    measured_axes.insert(code, (measured.clone(), pressure));
                    axes.insert(code, axis);
                }
                _ => bail!("Unsupported physical controller transport"),
            }
        }
        ensure!(
            !pressure_codes.is_empty(),
            "No measured pressure controls were mapped"
        );
        let sys_input = Path::new("/sys/class/input");
        let (event, identity) =
            measured_event_path(&device.device_path, &device.event_paths, sys_input)?;
        let bridge =
            GamepadBridge::start(event, GamepadFrame::new(buttons, axes)?, &identity, cancel)?;
        ensure!(
            JoydevMap::read(&device.device_path)? == numbering
                && measured_event_path(&device.device_path, &device.event_paths, sys_input)?.1
                    == identity,
            "Physical controller changed while starting normalized transport"
        );
        let deadline = std::time::Instant::now() + std::time::Duration::from_secs(2);
        let virtual_path = loop {
            check_preparation_cancel(cancel)?;
            if let Some(path) = bridge.joydev_path()? {
                break path;
            }
            ensure!(
                std::time::Instant::now() < deadline,
                "Normalized gamepad has no readable joydev node; check joydev and input permissions"
            );
            std::thread::sleep(std::time::Duration::from_millis(10));
        };
        let virtual_numbering = JoydevMap::read(&virtual_path)?;
        bridge.check_health()?;
        Ok(PlayerTransport {
            numbering: virtual_numbering,
            pressure_codes,
            bridge: Some(bridge),
        })
    }
    #[cfg(not(all(target_os = "linux", target_pointer_width = "64")))]
    bail!("Normalized controller pressure transport requires 64-bit Linux")
}

#[cfg(target_os = "linux")]
pub fn physical_button_present(path: &Path, code: u16) -> Result<bool> {
    use std::os::fd::AsRawFd;
    ensure!(code < 768, "Unsupported evdev key code");
    let file = std::fs::File::open(path)?;
    let mut keys = [0u8; 96];
    // EVIOCGBIT(EV_KEY, KEY_CNT / 8): read-only key capability query.
    let result = unsafe {
        libc::ioctl(
            file.as_raw_fd(),
            0x80604521 as libc::c_ulong,
            keys.as_mut_ptr(),
        )
    };
    ensure!(result >= 0, "Cannot verify physical D-pad capabilities");
    Ok(keys[usize::from(code) / 8] & (1 << (code % 8)) != 0)
}

pub fn numbering_probe() -> Result<()> {
    let mut warnings = Vec::new();
    for device in crate::controllers::list_local_controllers(&mut warnings) {
        let numbering = JoydevMap::read(&device.device_path)?;
        #[cfg(target_os = "linux")]
        let physical_axes = {
            let (event, _) = measured_event_path(
                &device.device_path,
                &device.event_paths,
                Path::new("/sys/class/input"),
            )?;
            numbering
                .axes
                .iter()
                .map(|code| crate::controller_axis::probe(event, *code))
                .collect::<Result<Vec<_>>>()?
        };
        #[cfg(not(target_os = "linux"))]
        let physical_axes: Option<Vec<serde_json::Value>> = None;
        println!(
            "{}",
            serde_json::json!({ "device": device.device_path, "name": device.name,
            "index": numbering.index, "buttons": numbering.buttons, "axes": numbering.axes,
            "physical_axes": physical_axes })
        );
    }
    Ok(())
}

impl JoydevMap {
    pub fn binding(&self, input: &NativeInput) -> Result<(&'static str, String)> {
        let code = (input.code & 0xffff) as u16;
        match input.code >> 16 {
            1 if input.direction == 0 => {
                let index = self
                    .buttons
                    .iter()
                    .position(|value| *value == code)
                    .context(
                        "Recorded button is not present in this device's current mode; recalibrate",
                    )?;
                ensure!(
                    index < 32,
                    "RetroArch linuxraw supports only the first 32 buttons"
                );
                Ok(("btn", index.to_string()))
            }
            3 if matches!(input.direction, -1 | 1) => {
                let index = self
                    .axes
                    .iter()
                    .position(|value| u16::from(*value) == code)
                    .context(
                        "Recorded axis is not present in this device's current mode; recalibrate",
                    )?;
                ensure!(
                    index < 32,
                    "RetroArch linuxraw supports only the first 32 axes"
                );
                Ok((
                    "axis",
                    format!("{}{index}", if input.direction < 0 { "-" } else { "+" }),
                ))
            }
            _ => bail!("Unsupported physical input encoding"),
        }
    }

    #[cfg(target_os = "linux")]
    pub fn read(path: &Path) -> Result<Self> {
        use std::os::fd::AsRawFd;
        ensure!(
            path.parent() == Some(Path::new("/dev/input")),
            "Expected a native joystick device"
        );
        let index = path
            .file_name()
            .and_then(|s| s.to_str())
            .and_then(|s| s.strip_prefix("js"))
            .context("Expected a joystick device")?
            .parse::<usize>()?;
        ensure!(
            index < 16,
            "Joystick index is outside RetroArch's supported player range"
        );
        let file = std::fs::File::open(path).context("Opening calibrated controller")?;
        let mut buttons = [0u16; 512];
        let mut axes = [0u8; 64];
        let mut button_count = 0u8;
        let mut axis_count = 0u8;
        // Linux joystick.h read-only ABI; the request sizes exactly match the
        // writable stack arrays. The FD remains owned and live for every call.
        for (request, destination) in [
            (
                0x80016a12u64,
                (&mut button_count as *mut u8).cast::<libc::c_void>(),
            ),
            (0x80016a11, (&mut axis_count as *mut u8).cast()),
            (0x84006a34, buttons.as_mut_ptr().cast()),
            (0x80406a32, axes.as_mut_ptr().cast()),
        ] {
            let result =
                unsafe { libc::ioctl(file.as_raw_fd(), request as libc::c_ulong, destination) };
            ensure!(
                result >= 0,
                "Reading controller numbering: {}",
                std::io::Error::last_os_error()
            );
        }
        ensure!(
            usize::from(axis_count) <= axes.len(),
            "Invalid joystick axis count"
        );
        Ok(Self {
            index,
            buttons: buttons[..usize::from(button_count)].to_vec(),
            axes: axes[..usize::from(axis_count)].to_vec(),
        })
    }

    #[cfg(not(target_os = "linux"))]
    pub fn read(_path: &Path) -> Result<Self> {
        bail!("Native controller numbering adapter is not available on this OS yet")
    }
}

fn player_config(
    calibration: &Calibration,
    profile: &EmulatorProfile,
    device: &JoydevMap,
    player: usize,
) -> Result<String> {
    let requested = profile
        .launch_device_for_port(player)
        .context("Preview-only controller contract or invalid frontend port")?;
    player_config_requested(calibration, profile, device, player, requested)
}

fn player_config_requested(
    calibration: &Calibration,
    profile: &EmulatorProfile,
    device: &JoydevMap,
    player: usize,
    requested_mode: u32,
) -> Result<String> {
    // Diagnostic/test funnel without a live probe: the calibration's own
    // measured pressure gestures are the active normalized transports. The
    // launch path still confirms them against the running device.
    let mut pressure_codes = std::collections::BTreeSet::new();
    for input in calibration.bindings.values() {
        if let (Some(native), Some(axis)) = (&input.native, &input.axis)
            && native.code >> 16 == 3
            && crate::controller_axis::PressureAxis::from_measurement(axis).is_ok()
        {
            pressure_codes.insert((native.code & 0xffff) as u16);
        }
    }
    player_config_transport(
        calibration,
        profile,
        device,
        player,
        requested_mode,
        &pressure_codes,
    )
}

fn player_config_transport(
    calibration: &Calibration,
    profile: &EmulatorProfile,
    device: &JoydevMap,
    player: usize,
    requested_mode: u32,
    pressure_codes: &std::collections::BTreeSet<u16>,
) -> Result<String> {
    calibration.validate()?;
    ensure!(
        calibration.os == std::env::consts::OS && calibration.os == "linux",
        "Recalibrate on this host OS"
    );
    let launch = profile
        .retroarch_launch
        .as_ref()
        .context("Preview-only controller contract")?;
    ensure!(
        Some(requested_mode) == profile.launch_device_for_port(player)
            || (matches!(profile.core.as_str(), "mednafen_psx" | "mednafen_psx_hw")
                && matches!(requested_mode, 1 | 517)),
        "Requested controller mode disagrees with the binding contract"
    );
    ensure!(
        (1..=launch.max_players).contains(&player),
        "Player number exceeds this input mode's port count"
    );
    let profile = profile.for_port(player);
    let plan = calibration.plan_profile(&profile)?;
    let target = catalog().layout(&profile.target_layout).unwrap();
    let mut values = BTreeMap::new();
    // Clear BOTH sides: an inherited axis must not survive a new button bind.
    // Keyboard bindings remain available. Unused auto-config inputs are disabled.
    for (_, output) in OUTPUTS {
        for suffix in ["btn", "axis"] {
            values.insert(
                format!("input_player{player}_{output}_{suffix}"),
                "nul".to_string(),
            );
        }
    }
    let mut assigned_channels = std::collections::BTreeSet::new();
    for row in plan.rows {
        let Some(input) = row.input else {
            let optional = target
                .controls
                .iter()
                .find(|c| c.id == row.target_id)
                .is_some_and(|c| c.optional);
            ensure!(
                optional,
                "{} is not mapped for {}. Complete calibration or select a compatible controller.",
                row.target,
                profile.name
            );
            continue;
        };
        let target_control = target
            .controls
            .iter()
            .find(|control| control.id == row.target_id)
            .context("Unknown mapped target control")?;
        let mut native = input.native.context("This calibration predates physical-input capture. Calibrate this controller again once")?;
        if target_control.is_pressure() {
            ensure!(
                native.code >> 16 == 3 && pressure_codes.contains(&((native.code & 0xffff) as u16)),
                "{} requires an active normalized pressure transport",
                target_control.label
            );
            // Physical polarity belongs to the measured normalizer. The
            // session-local output always uses positive pressure travel.
            native.direction = 1;
        }
        let (suffix, value) = device.binding(&native)?;
        let (_, output) = OUTPUTS
            .iter()
            .find(|(logical, _)| *logical == row.output)
            .context("Unknown RetroPad output")?;
        if target_control.is_pressure() {
            // A pressure-aware core may fall back to a full digital press at
            // zero magnitude. Inherited keyboard/mouse buttons must not inject
            // that fallback into a measured-only pressure contract.
            values.insert(format!("input_player{player}_{output}"), "nul".into());
            values.insert(format!("input_player{player}_{output}_mbtn"), "nul".into());
        }
        let channel = format!("input_player{player}_{output}_{suffix}");
        ensure!(
            assigned_channels.insert(channel.clone()),
            "Multiple controls resolve to the same RetroPad {output} {suffix} channel; use a physical button for the digital trigger fallback"
        );
        values.insert(channel, value);
    }
    values.insert(
        format!("input_player{player}_joypad_index"),
        device.index.to_string(),
    );
    values.insert(format!("input_player{player}_analog_dpad_mode"), "0".into());
    values.insert(
        format!("input_libretro_device_p{player}"),
        requested_mode.to_string(),
    );
    Ok(values
        .into_iter()
        .map(|(key, value)| format!("{key} = \"{value}\"\n"))
        .collect())
}

/// A device attachment (for example a multitap on console port two) can remain
/// present without a pad assigned to its first frontend slot. Clear every input
/// source for that empty slot so inherited keyboard or joypad bindings cannot
/// leak into it. Other slots behind the attachment keep their own bindings.
fn empty_port_config(port: usize, device: u32) -> String {
    let mut output = String::new();
    for (_, control) in OUTPUTS {
        output.push_str(&format!("input_player{port}_{control} = \"nul\"\n"));
        for suffix in ["btn", "axis"] {
            output.push_str(&format!(
                "input_player{port}_{control}_{suffix} = \"nul\"\n"
            ));
        }
    }
    output.push_str(&format!("input_libretro_device_p{port} = \"{device}\"\n"));
    output
}

fn append_argument(arguments: &mut Vec<OsString>, path: &Path) -> Result<()> {
    let path = path
        .to_str()
        .context("RetroArch config path is not UTF-8")?;
    ensure!(
        !path.contains('|'),
        "RetroArch config path contains its list separator"
    );
    // A custom append list must be preserved; RetroArch accepts a pipe-separated list.
    if let Some(index) = crate::controller_launch_modes::append_config_index(arguments)? {
        let value_index = if arguments[index] == "--appendconfig" {
            index + 1
        } else {
            index
        };
        let previous = arguments[value_index]
            .to_str()
            .context("Invalid --appendconfig")?;
        arguments[value_index] = format!("{previous}|{path}").into();
    } else {
        // Inserting at the front also works with a user's final `--` separator.
        arguments.splice(
            0..0,
            [OsString::from("--appendconfig"), OsString::from(path)],
        );
    }
    Ok(())
}

fn selected_devices<'a>(
    settings: &AppSettings,
    devices: &'a [ControllerDevice],
    platform: &str,
) -> Vec<&'a ControllerDevice> {
    let mapping = &settings.controller_mapping;
    if mapping.explicit_player_selection {
        // Preserve the chosen order exactly. Missing players are rejected by the
        // launch guard below, rather than silently promoting P2 to P1.
        return mapping
            .player_mappings
            .iter()
            .filter_map(|player| {
                devices
                    .iter()
                    .find(|device| player.controller_id.as_deref() == Some(&device.stable_id))
            })
            .collect();
    }
    let system = crate::controllers::system_layout(platform);
    let preferred = mapping
        .preferred_devices
        .get(crate::controllers::system_layout(platform));
    let mut devices = devices
        .iter()
        .filter(|device| {
            mapping.calibrations.contains_key(&device.stable_id)
                && !mapping.hidden_controller_ids.contains(&device.stable_id)
        })
        .collect::<Vec<_>>();
    devices.sort_by_key(|device| {
        let explicit = mapping
            .player_mappings
            .iter()
            .position(|p| p.controller_id.as_deref() == Some(&device.stable_id));
        let family = catalog()
            .layout(&mapping.calibrations[&device.stable_id].layout)
            .map(|layout| layout.family.as_str())
            .unwrap_or("");
        let fit = match (system, family) {
            ("two-button", "two-button" | "horizontal-four")
            | ("n64", "n64")
            | ("six-button", "six-button" | "three-button")
            | ("modern", "diamond") => 0,
            ("six-button", "n64") => 1,
            _ => 2,
        };
        (
            explicit.is_none(),
            explicit.unwrap_or(usize::MAX),
            preferred != Some(&device.stable_id),
            fit,
        )
    });
    devices
}

fn compatible(calibration: &Calibration, profile: &EmulatorProfile) -> bool {
    calibration
        .plan_profile(profile)
        .is_ok_and(|plan| plan.automatic_launch_ready)
}

fn restrict_players(
    devices: &mut Vec<&ControllerDevice>,
    profile: &EmulatorProfile,
) -> Result<usize> {
    let eligible_count = devices.len();
    let launch = profile
        .retroarch_launch
        .as_ref()
        .context("Preview-only controller contract")?;
    // Preference ordering chooses active players without creating nonexistent ports.
    devices.truncate(launch.max_players);
    Ok(eligible_count)
}

/// Returns None only when no saved calibrated launch was requested. Failures are
/// surfaced at the launch boundary instead of reporting a miswired game as ready.
pub fn prepare(
    settings: &AppSettings,
    platform: &str,
    option: &RomEmulatorOption,
    plan: &mut LaunchPlan,
) -> Result<Option<CalibratedLaunch>> {
    prepare_with_cancellation(
        settings,
        platform,
        option,
        plan,
        &std::sync::atomic::AtomicBool::new(false),
    )
}

fn check_preparation_cancel(cancel: &std::sync::atomic::AtomicBool) -> Result<()> {
    ensure!(
        !cancel.load(std::sync::atomic::Ordering::Relaxed),
        "{}",
        crate::rom_launch_preparation::LAUNCH_CANCELLED_ERROR
    );
    Ok(())
}

pub fn prepare_with_cancellation(
    settings: &AppSettings,
    platform: &str,
    option: &RomEmulatorOption,
    plan: &mut LaunchPlan,
    cancel: &std::sync::atomic::AtomicBool,
) -> Result<Option<CalibratedLaunch>> {
    check_preparation_cancel(cancel)?;
    let mapping = &settings.controller_mapping;
    if !mapping.calibrated_launch {
        return Ok(None);
    }
    let mut warnings = Vec::new();
    let inventory = crate::controllers::list_local_controllers(&mut warnings);
    let guided_settings = crate::controller_guided_native::settings_for_launch(
        settings, option, platform, plan, &inventory, cancel,
    )?;
    let settings = guided_settings.as_ref();
    let mapping = &settings.controller_mapping;
    let guided_target = crate::controller_target::selected(mapping, option, platform)?;
    if guided_target.is_none()
        && option.runtime_kind == EmulatorRuntimeKind::RetroArch
        && option.core_name == "mame"
    {
        let content = plan
            .retroarch_content
            .as_ref()
            .context("MAME requires prepared content identity")?;
        let core_path = content.core.canonicalize()?;
        let content_path = content.content.canonicalize()?;
        let mut setups = mapping.mame_launches.iter().filter(|saved| {
            saved.emulator_id == option.emulator_id
                && saved.core == core_path
                && saved.content == content_path
        });
        if let Some(saved) = setups.next() {
            ensure!(
                setups.next().is_none(),
                "Duplicate saved MAME controller setups"
            );
            saved.validate()?;
            ensure!(
                (saved.relative_assignments.is_empty()
                    && saved.relative_button_assignments.is_empty())
                    || cfg!(all(target_os = "linux", target_pointer_width = "64")),
                "MAME physical relative sources require 64-bit Linux"
            );
            ensure!(
                lunchbox_controller_probe::file_hash(&core_path)?
                    .eq_ignore_ascii_case(&saved.core_sha256)
                    && lunchbox_controller_probe::file_hash(&content_path)?
                        .eq_ignore_ascii_case(&saved.content_sha256),
                "MAME core or content changed; review the controller setup again"
            );
            let state_inputs =
                crate::controller_mame::persistent_inputs(&saved.persistent, cancel)?;
            // The resolved state roots are authoritative. Reject conflicting
            // explicit entries, then add the complete current state manifest.
            for explicit in saved.inputs.iter().filter(|input| {
                input.destination.starts_with("nvram") || input.destination.starts_with("diff")
            }) {
                ensure!(
                    state_inputs
                        .iter()
                        .any(|state| state.destination == explicit.destination
                            && state.source == explicit.source),
                    "Explicit MAME state input conflicts with its persistent directory"
                );
            }
            let mut inputs: Vec<_> = saved
                .inputs
                .iter()
                .filter(|input| {
                    !input.destination.starts_with("nvram")
                        && !input.destination.starts_with("diff")
                })
                .cloned()
                .collect();
            inputs.extend(state_inputs);
            let inspection = prepare_mame_inspection(
                option,
                plan,
                &saved.machine,
                &saved.library,
                inputs,
                &saved.selected_players(),
                None,
                Vec::new(),
                &saved.analog_assignments,
                &saved.digital_assignments,
                None,
                &saved.relative_assignments,
                &saved.relative_button_assignments,
                saved
                    .reviewed_snapshot
                    .mouse_state
                    .as_ref()
                    .is_some_and(|state| state.enabled),
                cancel,
            )?;
            saved.validate_review(&inspection.fields)?;
            let mut players = BTreeMap::new();
            for (port, controller) in &saved.players {
                check_preparation_cancel(cancel)?;
                let mut matches = inventory
                    .iter()
                    .filter(|device| &device.stable_id == controller);
                let device = matches
                    .next()
                    .context("Configured MAME controller is disconnected")?;
                ensure!(
                    matches.next().is_none(),
                    "Configured MAME controller identity is ambiguous"
                );
                let calibration = mapping
                    .calibrations
                    .get(controller)
                    .context("MAME controller lacks saved physical calibration")?;
                players.insert(*port, (calibration, device));
            }
            let input = stage_mame_input(inspection, saved.persistent.clone())?;
            let mut session = prepare_mame_calibrated_session(
                input,
                &players,
                plan,
                &saved.frontend_save,
                &saved.frontend_state,
                saved.digital_layout,
                &saved.player_digital_layouts,
                &saved.analog_assignments,
                &saved.digital_assignments,
                cancel,
            )?;
            #[cfg(all(target_os = "linux", target_pointer_width = "64"))]
            if !saved.relative_assignments.is_empty()
                || !saved.relative_button_assignments.is_empty()
            {
                session.prepare_mame_relative_sources(&saved.relative_sources, plan, cancel)?;
            }
            return Ok(Some(session));
        }
        if mapping.mame_arcade_layout.is_none()
            && !mapping
                .launch_mode_selections
                .contains_key(&selection_key("mame", platform))
        {
            return Ok(None);
        }
        if let Some(
            layout @ (crate::controller_mame::DigitalLayout::NeoGeo
            | crate::controller_mame::DigitalLayout::FixedChannels),
        ) = mapping.mame_arcade_layout
        {
            return mame_automatic::prepare(
                settings, platform, option, plan, &inventory, layout, cancel,
            )
            .map(Some);
        }
    }
    if guided_target.is_none()
        && option.runtime_kind == EmulatorRuntimeKind::RetroArch
        && option.core_name == "fbneo"
    {
        let content = plan
            .retroarch_content
            .as_ref()
            .context("FBNeo requires prepared content identity")?;
        let core_path = content.core.canonicalize()?;
        let content_path = content.content.canonicalize()?;
        let mut saved_setups = mapping.fbneo_launches.iter().filter(|saved| {
            saved.emulator_id == option.emulator_id
                && saved.core == core_path
                && saved.content == content_path
        });
        if let Some(saved) = saved_setups.next() {
            ensure!(
                saved_setups.next().is_none(),
                "Duplicate FBNeo per-game controller setups"
            );
            #[cfg(all(target_os = "linux", target_pointer_width = "64"))]
            {
                let topology = saved.validate()?;
                ensure!(
                    lunchbox_controller_probe::file_hash(&core_path)?
                        .eq_ignore_ascii_case(&saved.core_sha256)
                        && lunchbox_controller_probe::file_hash(&content_path)?
                            .eq_ignore_ascii_case(&saved.content_sha256),
                    "FBNeo core or game changed since controller setup; review the mapping again"
                );
                let mut inspection = prepare_fbneo_inspection(
                    option,
                    plan,
                    &saved.helper,
                    &saved.library,
                    &saved.dependencies,
                    &saved.system_files,
                    saved
                        .players
                        .iter()
                        .map(|player| (player.port, player.device))
                        .collect(),
                    &topology,
                    cancel,
                )?;
                inspection.bind_saved_relative_sources(saved)?;
                let gamepad_free_ports = saved.gamepad_free_ports()?;
                let mut players = BTreeMap::new();
                for player in &saved.players {
                    check_preparation_cancel(cancel)?;
                    if player.controller_id.is_empty() && gamepad_free_ports.contains(&player.port)
                    {
                        if inspection.absolute_sources.contains_key(&player.port) {
                            players.insert(
                                player.port,
                                prepare_fbneo_absolute_only(&inspection, player.port, cancel)?,
                            );
                        }
                        continue;
                    }
                    let mut devices = inventory
                        .iter()
                        .filter(|device| device.stable_id == player.controller_id);
                    let device = devices
                        .next()
                        .context("Configured FBNeo controller is disconnected")?;
                    ensure!(
                        devices.next().is_none(),
                        "Configured FBNeo controller identity is ambiguous"
                    );
                    let calibration = mapping
                        .calibrations
                        .get(&player.controller_id)
                        .context("Configured FBNeo controller has no saved physical calibration")?;
                    players.insert(
                        player.port,
                        prepare_fbneo_player(
                            &inspection,
                            player.port,
                            calibration,
                            device,
                            &saved.physical_assignments(player)?,
                            cancel,
                        )?,
                    );
                }
                check_preparation_cancel(cancel)?;
                let mut session = attach_fbneo_session(option, plan, inspection, players, cancel)?;
                if !saved.relative_sources.is_empty() {
                    session.prepare_fbneo_relative_sources(plan, cancel)?;
                }
                return Ok(Some(session));
            }
            #[cfg(not(all(target_os = "linux", target_pointer_width = "64")))]
            bail!("Saved FBNeo normalized controller sessions require 64-bit Linux");
        }
    }
    // Saved per-game setups above may be entirely relative-input backed. Only
    // the remaining gamepad-calibration paths require a nonempty pad registry.
    if mapping.calibrations.is_empty() {
        return Ok(None);
    }
    #[cfg(target_os = "linux")]
    if option.runtime_kind == EmulatorRuntimeKind::Standalone
        && option.emulator_name.eq_ignore_ascii_case("melonDS")
    {
        let matches: Vec<_> = mapping
            .melonds_launches
            .iter()
            .filter(|setup| {
                setup.emulator_id == option.emulator_id && plan.arguments.len() == 1 && {
                    let argument = std::path::PathBuf::from(&plan.arguments[0]);
                    let resolved = if argument.is_absolute() {
                        argument
                    } else {
                        plan.current_directory.join(argument)
                    };
                    setup
                        .content
                        .canonicalize()
                        .ok()
                        .zip(resolved.canonicalize().ok())
                        .is_some_and(|(a, b)| a == b)
                }
            })
            .collect();
        ensure!(matches.len() <= 1, "Ambiguous melonDS saved setup");
        if let Some(setup) = matches.first() {
            let native = crate::controller_melonds::native_command::prepare(
                setup,
                &mapping.calibrations,
                &inventory,
                option,
                plan,
                cancel,
            )?;
            *plan = native.plan().clone();
            return Ok(Some(CalibratedLaunch {
                mgba: None,
                #[cfg(target_os = "linux")]
                dolphin_native: None,
                snes9x_native: None,
                fceux_native: None,
                sameboy_native: None,
            bsnes_native: None,
                mednafen_native: None,
                #[cfg(target_os = "linux")]
                mame_native: None,
                #[cfg(target_os = "linux")]
                flycast_native: None,
                #[cfg(target_os = "linux")]
                pcsx2_native: None,
                #[cfg(target_os = "linux")]
                rpcs3_native: None,
                #[cfg(target_os = "linux")]
                melonds_native: Some(native),
                ppsspp: None,
                duckstation: None,
                _directory: None,
                bizhawk: None,
                bizhawk_topology: None,
                transports: Vec::new(),
                crocods: None,
                ep128emu: None,
                hatari: None,
                simcp: None,
                steemsse: None,
                scummvm: None,
                dolphin: None,
                same_cdi: None,
                fbneo: None,
                mame: None,
                puae: None,
                stella: None,
                description: "melonDS: standard DS buttons; partial Linux SDL2 support; runtime and internal mapping verification incomplete"
                    .into(),
            }));
        }
    }

    #[cfg(target_os = "linux")]
    if option.runtime_kind == EmulatorRuntimeKind::Standalone
        && option.emulator_name.eq_ignore_ascii_case("RPCS3")
    {
        let matches: Vec<_> = mapping
            .rpcs3_launches
            .iter()
            .filter(|setup| {
                setup.emulator_id == option.emulator_id && plan.arguments.len() == 1 && {
                    let argument = std::path::PathBuf::from(&plan.arguments[0]);
                    let resolved = if argument.is_absolute() {
                        argument
                    } else {
                        plan.current_directory.join(argument)
                    };
                    setup
                        .content
                        .canonicalize()
                        .ok()
                        .zip(resolved.canonicalize().ok())
                        .is_some_and(|(a, b)| a == b)
                }
            })
            .collect();
        ensure!(matches.len() <= 1, "Ambiguous RPCS3 saved setup");
        if let Some(setup) = matches.first() {
            let native = crate::controller_rpcs3::native_command::prepare(
                setup,
                &mapping.calibrations,
                &inventory,
                option,
                plan,
                cancel,
            )?;
            *plan = native.plan().clone();
            return Ok(Some(CalibratedLaunch {
                mgba: None,
                #[cfg(target_os = "linux")]
                dolphin_native: None,
                snes9x_native: None,
                fceux_native: None,
                sameboy_native: None,
            bsnes_native: None,
                mednafen_native: None,
                #[cfg(target_os = "linux")]
                mame_native: None,
                #[cfg(target_os = "linux")]
                flycast_native: None,
                #[cfg(target_os = "linux")]
                pcsx2_native: None,
                #[cfg(target_os = "linux")]
                rpcs3_native: Some(native),
                #[cfg(target_os = "linux")]
                melonds_native: None,
                ppsspp: None,
                duckstation: None,
                _directory: None,
                bizhawk: None,
                bizhawk_topology: None,
                transports: Vec::new(),
                crocods: None,
                ep128emu: None,
                hatari: None,
                simcp: None,
                steemsse: None,
                scummvm: None,
                dolphin: None,
                same_cdi: None,
                fbneo: None,
                mame: None,
                puae: None,
                stella: None,
                description: "RPCS3: standard pads, file boot targets; partial Linux SDL3 support; runtime and internal mapping verification incomplete"
                    .into(),
            }));
        }
    }

    #[cfg(target_os = "linux")]
    if option.runtime_kind == EmulatorRuntimeKind::Standalone
        && option.emulator_name.eq_ignore_ascii_case("PCSX2")
    {
        let matches: Vec<_> = mapping
            .pcsx2_launches
            .iter()
            .filter(|setup| {
                setup.emulator_id == option.emulator_id && plan.arguments.len() == 1 && {
                    let argument = std::path::PathBuf::from(&plan.arguments[0]);
                    let resolved = if argument.is_absolute() {
                        argument
                    } else {
                        plan.current_directory.join(argument)
                    };
                    setup
                        .content
                        .canonicalize()
                        .ok()
                        .zip(resolved.canonicalize().ok())
                        .is_some_and(|(a, b)| a == b)
                }
            })
            .collect();
        ensure!(matches.len() <= 1, "Ambiguous PCSX2 saved setup");
        if let Some(setup) = matches.first() {
            let native = crate::controller_pcsx2::native_command::prepare(
                setup,
                &mapping.calibrations,
                &inventory,
                option,
                plan,
                cancel,
            )?;
            *plan = native.plan().clone();
            return Ok(Some(CalibratedLaunch {
                mgba: None,
                #[cfg(target_os = "linux")]
                dolphin_native: None,
                snes9x_native: None,
                fceux_native: None,
                sameboy_native: None,
            bsnes_native: None,
                mednafen_native: None,
                #[cfg(target_os = "linux")]
                mame_native: None,
                #[cfg(target_os = "linux")]
                flycast_native: None,
                #[cfg(target_os = "linux")]
                pcsx2_native: Some(native),
                #[cfg(target_os = "linux")]
                rpcs3_native: None,
                #[cfg(target_os = "linux")]
                melonds_native: None,
                ppsspp: None,
                duckstation: None,
                _directory: None,
                bizhawk: None,
                bizhawk_topology: None,
                transports: Vec::new(),
                crocods: None,
                ep128emu: None,
                hatari: None,
                simcp: None,
                steemsse: None,
                scummvm: None,
                dolphin: None,
                same_cdi: None,
                fbneo: None,
                mame: None,
                puae: None,
                stella: None,
                description: "PCSX2: native DualShock 2; partial Linux SDL3 support; runtime and internal mapping verification incomplete"
                    .into(),
            }));
        }
    }

    #[cfg(target_os = "linux")]
    if option.runtime_kind == EmulatorRuntimeKind::Standalone
        && option.emulator_name.eq_ignore_ascii_case("Flycast")
    {
        let matches: Vec<_> = mapping
            .flycast_native_launches
            .iter()
            .filter(|setup| {
                setup.emulator_id == option.emulator_id && plan.arguments.len() == 1 && {
                    let argument = std::path::PathBuf::from(&plan.arguments[0]);
                    let resolved = if argument.is_absolute() {
                        argument
                    } else {
                        plan.current_directory.join(argument)
                    };
                    setup
                        .content
                        .canonicalize()
                        .ok()
                        .zip(resolved.canonicalize().ok())
                        .is_some_and(|(a, b)| a == b)
                }
            })
            .collect();
        ensure!(matches.len() <= 1, "Ambiguous Flycast saved setup");
        if let Some(setup) = matches.first() {
            let native = crate::controller_flycast_native::native_command::prepare(
                setup,
                &mapping.calibrations,
                &inventory,
                option,
                plan,
                cancel,
            )?;
            *plan = native.plan().clone();
            return Ok(Some(CalibratedLaunch {
                mgba: None,
                #[cfg(target_os = "linux")]
                dolphin_native: None,
                snes9x_native: None,
                fceux_native: None,
                sameboy_native: None,
            bsnes_native: None,
                mednafen_native: None,
                #[cfg(target_os = "linux")]
                mame_native: None,
                #[cfg(target_os = "linux")]
                flycast_native: Some(native),
                #[cfg(target_os = "linux")]
                pcsx2_native: None,
                #[cfg(target_os = "linux")]
                rpcs3_native: None,
                #[cfg(target_os = "linux")]
                melonds_native: None,
                ppsspp: None,
                duckstation: None,
                _directory: None,
                bizhawk: None,
                bizhawk_topology: None,
                transports: Vec::new(),
                crocods: None,
                ep128emu: None,
                hatari: None,
                simcp: None,
                steemsse: None,
                scummvm: None,
                dolphin: None,
                same_cdi: None,
                fbneo: None,
                mame: None,
                puae: None,
                stella: None,
                description: "Flycast: native standard six/eight-button panels; partial Linux raw SDL support; runtime and internal mapping verification incomplete"
                    .into(),
            }));
        }
    }

    #[cfg(target_os = "linux")]
    if option.runtime_kind == EmulatorRuntimeKind::Standalone
        && option.emulator_name.eq_ignore_ascii_case("MAME")
    {
        let matches: Vec<_> = mapping
            .mame_native_launches
            .iter()
            .filter(|setup| setup.emulator_id == option.emulator_id)
            .collect();
        ensure!(matches.len() <= 1, "Ambiguous MAME saved setup");
        if let Some(setup) = matches.first() {
            let native = crate::controller_mame_native::native_command::prepare(
                setup,
                &mapping.calibrations,
                &inventory,
                option,
                plan,
                cancel,
            )?;
            *plan = native.plan().clone();
            return Ok(Some(CalibratedLaunch {
                mgba: None,
                #[cfg(target_os = "linux")]
                dolphin_native: None,
                snes9x_native: None,
                fceux_native: None,
                sameboy_native: None,
            bsnes_native: None,
                mednafen_native: None,
                #[cfg(target_os = "linux")]
                mame_native: Some(native),
                #[cfg(target_os = "linux")]
                flycast_native: None,
                #[cfg(target_os = "linux")]
                pcsx2_native: None,
                #[cfg(target_os = "linux")]
                rpcs3_native: None,
                #[cfg(target_os = "linux")]
                melonds_native: None,
                ppsspp: None,
                duckstation: None,
                _directory: None,
                bizhawk: None,
                bizhawk_topology: None,
                transports: Vec::new(),
                crocods: None,
                ep128emu: None,
                hatari: None,
                simcp: None,
                steemsse: None,
                scummvm: None,
                dolphin: None,
                same_cdi: None,
                fbneo: None,
                mame: None,
                puae: None,
                stella: None,
                description: "MAME: native standard six/eight-button panels; partial Linux raw SDL support; runtime and internal mapping verification incomplete"
                    .into(),
            }));
        }
    }

    #[cfg(target_os = "linux")]
    if option.runtime_kind == EmulatorRuntimeKind::Standalone
        && option.emulator_name.eq_ignore_ascii_case("Mednafen")
    {
        let matches: Vec<_> = mapping
            .mednafen_launches
            .iter()
            .filter(|setup| {
                setup.emulator_id == option.emulator_id
                    && plan
                        .arguments
                        .iter()
                        .any(|arg| arg == setup.content.as_os_str())
            })
            .collect();
        ensure!(matches.len() <= 1, "Ambiguous Mednafen saved setup");
        if let Some(setup) = matches.first() {
            let native = crate::controller_mednafen::native_command::prepare(
                setup,
                &mapping.calibrations,
                &inventory,
                option,
                plan,
                cancel,
            )?;
            *plan = native.plan.clone();
            return Ok(Some(CalibratedLaunch {
                mgba: None,
                #[cfg(target_os = "linux")]
                dolphin_native: None,
                snes9x_native: None,
                fceux_native: None,
                sameboy_native: None,
            bsnes_native: None,
                mednafen_native: Some(native),
                #[cfg(target_os = "linux")]
                mame_native: None,
                #[cfg(target_os = "linux")]
                flycast_native: None,
                #[cfg(target_os = "linux")]
                pcsx2_native: None,
                #[cfg(target_os = "linux")]
                rpcs3_native: None,
                #[cfg(target_os = "linux")]
                melonds_native: None,
                ppsspp: None,
                duckstation: None,
                _directory: None,
                bizhawk: None,
                bizhawk_topology: None,
                transports: Vec::new(),
                crocods: None,
                ep128emu: None,
                hatari: None,
                simcp: None,
                steemsse: None,
                scummvm: None,
                dolphin: None,
                same_cdi: None,
                fbneo: None,
                mame: None,
                puae: None,
                stella: None,
                description: "Mednafen: native GB/GBA/Lynx/Neo Geo Pocket/WonderSwan/Virtual Boy/Game Gear/Master System/PC Engine controls; partial Linux joydev support; child IDs and other native drivers unverified"
                    .into(),
            }));
        }
    }

    #[cfg(target_os = "linux")]
    if option.runtime_kind == EmulatorRuntimeKind::Standalone
        && option.emulator_name.eq_ignore_ascii_case("bsnes")
    {
        let matches: Vec<_> = mapping
            .bsnes_launches
            .iter()
            .filter(|setup| {
                setup.emulator_id == option.emulator_id
                    && plan
                        .arguments
                        .iter()
                        .any(|arg| arg == setup.content.as_os_str())
            })
            .collect();
        ensure!(matches.len() <= 1, "Ambiguous bsnes saved setup");
        if let Some(setup) = matches.first() {
            let native = crate::controller_bsnes::native_command::prepare(
                setup,
                &mapping.calibrations,
                &inventory,
                option,
                plan,
                cancel,
            )?;
            *plan = native.plan.clone();
            return Ok(Some(CalibratedLaunch {
                mgba: None,
                #[cfg(target_os = "linux")]
                dolphin_native: None,
                snes9x_native: None,
                fceux_native: None,
                sameboy_native: None,
                bsnes_native: Some(native),
                #[cfg(target_os = "linux")]
                mednafen_native: None,
                #[cfg(target_os = "linux")]
                mame_native: None,
                #[cfg(target_os = "linux")]
                flycast_native: None,
                #[cfg(target_os = "linux")]
                pcsx2_native: None,
                #[cfg(target_os = "linux")]
                rpcs3_native: None,
                #[cfg(target_os = "linux")]
                melonds_native: None,
                ppsspp: None,
                duckstation: None,
                _directory: None,
                bizhawk: None,
                bizhawk_topology: None,
                transports: Vec::new(),
                crocods: None,
                ep128emu: None,
                hatari: None,
                simcp: None,
                steemsse: None,
                scummvm: None,
                dolphin: None,
                same_cdi: None,
                fbneo: None,
                mame: None,
                puae: None,
                stella: None,
                description: "bsnes settings.bml: calibrated SNES gamepad controls through the SDL joypad driver; private per-launch settings file; partial Linux support; Mouse/Super Multitap/Super Scope/Justifier targets not covered"
                    .into(),
            }));
        }
    }

    #[cfg(target_os = "linux")]
    if option.runtime_kind == EmulatorRuntimeKind::Standalone
        && option.emulator_name.eq_ignore_ascii_case("FCEUX")
    {
        let matches: Vec<_> = mapping
            .fceux_launches
            .iter()
            .filter(|setup| {
                setup.emulator_id == option.emulator_id
                    && plan
                        .arguments
                        .iter()
                        .any(|arg| arg == setup.content.as_os_str())
            })
            .collect();
        ensure!(matches.len() <= 1, "Ambiguous FCEUX saved setup");
        if let Some(setup) = matches.first() {
            let native = crate::controller_fceux::native_command::prepare(
                setup,
                &mapping.calibrations,
                &inventory,
                option,
                plan,
                cancel,
            )?;
            *plan = native.plan.clone();
            return Ok(Some(CalibratedLaunch {
                mgba: None,
                #[cfg(target_os = "linux")]
                dolphin_native: None,
                snes9x_native: None,
                fceux_native: Some(native),
                #[cfg(target_os = "linux")]
                sameboy_native: None,
            bsnes_native: None,
                #[cfg(target_os = "linux")]
                mednafen_native: None,
                #[cfg(target_os = "linux")]
                mame_native: None,
                #[cfg(target_os = "linux")]
                flycast_native: None,
                #[cfg(target_os = "linux")]
                pcsx2_native: None,
                #[cfg(target_os = "linux")]
                rpcs3_native: None,
                #[cfg(target_os = "linux")]
                melonds_native: None,
                ppsspp: None,
                duckstation: None,
                _directory: None,
                bizhawk: None,
                bizhawk_topology: None,
                transports: Vec::new(),
                crocods: None,
                ep128emu: None,
                hatari: None,
                simcp: None,
                steemsse: None,
                scummvm: None,
                dolphin: None,
                same_cdi: None,
                fbneo: None,
                mame: None,
                puae: None,
                stella: None,
                description: "FCEUX Qt: calibrated native NES controls; partial Linux support; ROM device overrides not resolved"
                    .into(),
            }));
        }
    }

    #[cfg(target_os = "linux")]
    if option.runtime_kind == EmulatorRuntimeKind::Standalone
        && option.emulator_name.eq_ignore_ascii_case("Snes9x")
    {
        let matches: Vec<_> = mapping
            .snes9x_launches
            .iter()
            .filter(|setup| {
                setup.emulator_id == option.emulator_id
                    && plan
                        .arguments
                        .iter()
                        .any(|arg| arg == setup.content.as_os_str())
            })
            .collect();
        ensure!(matches.len() <= 1, "Ambiguous Snes9x saved setup");
        if let Some(setup) = matches.first() {
            let native = crate::controller_snes9x::native_command::prepare(
                setup,
                &mapping.calibrations,
                &inventory,
                option,
                plan,
                cancel,
            )?;
            *plan = native.plan.clone();
            return Ok(Some(CalibratedLaunch {
                mgba: None,
                #[cfg(target_os = "linux")]
                dolphin_native: None,
                snes9x_native: Some(native),
                #[cfg(target_os = "linux")]
                fceux_native: None,
                #[cfg(target_os = "linux")]
                sameboy_native: None,
                bsnes_native: None,
                #[cfg(target_os = "linux")]
                mednafen_native: None,
                #[cfg(target_os = "linux")]
                mame_native: None,
                #[cfg(target_os = "linux")]
                flycast_native: None,
                #[cfg(target_os = "linux")]
                pcsx2_native: None,
                #[cfg(target_os = "linux")]
                rpcs3_native: None,
                #[cfg(target_os = "linux")]
                melonds_native: None,
                ppsspp: None,
                duckstation: None,
                _directory: None,
                bizhawk: None,
                bizhawk_topology: None,
                transports: Vec::new(),
                crocods: None,
                ep128emu: None,
                hatari: None,
                simcp: None,
                steemsse: None,
                scummvm: None,
                dolphin: None,
                same_cdi: None,
                fbneo: None,
                mame: None,
                puae: None,
                stella: None,
                description: "Snes9x GTK: calibrated native SNES controls; partial Linux support"
                    .into(),
            }));
        }
    }

    #[cfg(target_os = "linux")]
    if option.runtime_kind == EmulatorRuntimeKind::Standalone
        && option.emulator_name.eq_ignore_ascii_case("Dolphin")
    {
        let matches: Vec<_> = mapping
            .dolphin_launches
            .iter()
            .filter(|setup| {
                setup.emulator_id == option.emulator_id
                    && plan
                        .arguments
                        .iter()
                        .any(|arg| arg == setup.content.as_os_str())
            })
            .collect();
        ensure!(matches.len() <= 1, "Ambiguous Dolphin saved setup");
        if let Some(setup) = matches.first() {
            let native = crate::controller_dolphin::standalone::native_command::prepare(
                setup,
                &mapping.calibrations,
                &inventory,
                option,
                plan,
                cancel,
            )?;
            *plan = native.plan.clone();
            return Ok(Some(CalibratedLaunch {
                mgba: None,
                #[cfg(target_os = "linux")]
                dolphin_native: Some(native),
                #[cfg(target_os = "linux")]
                snes9x_native: None,
                #[cfg(target_os = "linux")]
                fceux_native: None,
                #[cfg(target_os = "linux")]
                sameboy_native: None,
            bsnes_native: None,
                #[cfg(target_os = "linux")]
                mednafen_native: None,
                #[cfg(target_os = "linux")]
                mame_native: None,
                #[cfg(target_os = "linux")]
                flycast_native: None,
                #[cfg(target_os = "linux")]
                pcsx2_native: None,
                #[cfg(target_os = "linux")]
                rpcs3_native: None,
                #[cfg(target_os = "linux")]
                melonds_native: None,
                ppsspp: None,
                duckstation: None,
                _directory: None,
                bizhawk: None,
                bizhawk_topology: None,
                transports: Vec::new(),
                crocods: None,
                ep128emu: None,
                hatari: None,
                simcp: None,
                steemsse: None,
                scummvm: None,
                dolphin: None,
                same_cdi: None,
                fbneo: None,
                mame: None,
                puae: None,
                stella: None,
                description:
                    "Dolphin 2606: calibrated native GameCube controls; partial Linux ISO/GCM support".into(),
            }));
        }
    }
    #[cfg(target_os = "linux")]
    if option.runtime_kind == EmulatorRuntimeKind::Standalone
        && option.emulator_name.eq_ignore_ascii_case("mGBA")
    {
        let matches: Vec<_> = mapping
            .mgba_launches
            .iter()
            .filter(|setup| {
                setup.emulator_id == option.emulator_id
                    && plan
                        .arguments
                        .iter()
                        .any(|arg| arg == setup.content.as_os_str())
            })
            .collect();
        ensure!(matches.len() <= 1, "Ambiguous mGBA saved setup");
        if let Some(setup) = matches.first() {
            let native = crate::controller_mgba::native_command::prepare(
                setup,
                &mapping.calibrations,
                &inventory,
                option,
                plan,
                cancel,
            )?;
            *plan = native.launch_plan().clone();
            return Ok(Some(CalibratedLaunch {
                mgba: Some(native),
                #[cfg(target_os = "linux")]
                dolphin_native: None,
                #[cfg(target_os = "linux")]
                snes9x_native: None,
                #[cfg(target_os = "linux")]
                fceux_native: None,
                #[cfg(target_os = "linux")]
                sameboy_native: None,
                bsnes_native: None,
                #[cfg(target_os = "linux")]
                mednafen_native: None,
                #[cfg(target_os = "linux")]
                mame_native: None,
                #[cfg(target_os = "linux")]
                flycast_native: None,
                #[cfg(target_os = "linux")]
                pcsx2_native: None,
                #[cfg(target_os = "linux")]
                rpcs3_native: None,
                #[cfg(target_os = "linux")]
                melonds_native: None,
                ppsspp: None,
                duckstation: None,
                _directory: None,
                bizhawk: None,
                bizhawk_topology: None,
                transports: Vec::new(),
                crocods: None,
                ep128emu: None,
                hatari: None,
                simcp: None,
                steemsse: None,
                scummvm: None,
                dolphin: None,
                same_cdi: None,
                fbneo: None,
                mame: None,
                puae: None,
                stella: None,
                description:
                    "mGBA SDL: calibrated native handheld controls; runtime handoff required".into(),
            }));
        }
    }
    #[cfg(target_os = "linux")]
    if option.runtime_kind == EmulatorRuntimeKind::Standalone
        && option.emulator_name.eq_ignore_ascii_case("PPSSPP")
    {
        let matches: Vec<_> = mapping
            .ppsspp_launches
            .iter()
            .filter(|setup| {
                setup.emulator_id == option.emulator_id
                    && plan
                        .arguments
                        .iter()
                        .any(|argument| argument == setup.content.as_os_str())
            })
            .collect();
        ensure!(matches.len() <= 1, "Ambiguous PPSSPP saved setup");
        if let Some(setup) = matches.first() {
            let native = crate::controller_ppsspp::native_command::prepare(
                setup,
                &mapping.calibrations,
                &inventory,
                option,
                plan,
                cancel,
            )?;
            *plan = native.launch_plan().clone();
            return Ok(Some(CalibratedLaunch {
                ppsspp: Some(native),
                mgba: None,
                #[cfg(target_os = "linux")]
                dolphin_native: None,
                #[cfg(target_os = "linux")]
                snes9x_native: None,
                #[cfg(target_os = "linux")]
                fceux_native: None,
                #[cfg(target_os = "linux")]
                sameboy_native: None,
                bsnes_native: None,
                #[cfg(target_os = "linux")]
                mednafen_native: None,
                #[cfg(target_os = "linux")]
                mame_native: None,
                #[cfg(target_os = "linux")]
                flycast_native: None,
                #[cfg(target_os = "linux")]
                pcsx2_native: None,
                #[cfg(target_os = "linux")]
                rpcs3_native: None,
                #[cfg(target_os = "linux")]
                melonds_native: None,
                duckstation: None,
                _directory: None,
                bizhawk: None,
                bizhawk_topology: None,
                transports: Vec::new(),
                crocods: None,
                ep128emu: None,
                hatari: None,
                simcp: None,
                steemsse: None,
                scummvm: None,
                dolphin: None,
                same_cdi: None,
                fbneo: None,
                mame: None,
                puae: None,
                stella: None,
                description:
                    "PPSSPP: calibrated native PSP controls; startup confirmation required".into(),
            }));
        }
    }
    #[cfg(target_os = "linux")]
    if option.runtime_kind == EmulatorRuntimeKind::Standalone
        && option.emulator_name.eq_ignore_ascii_case("DuckStation")
    {
        let matches: Vec<_> = mapping
            .duckstation_launches
            .iter()
            .filter(|setup| {
                setup.emulator_id == option.emulator_id
                    && plan
                        .arguments
                        .iter()
                        .any(|argument| argument == setup.content.as_os_str())
            })
            .collect();
        ensure!(matches.len() <= 1, "Ambiguous DuckStation saved setup");
        if let Some(setup) = matches.first() {
            let native = crate::controller_duckstation::native_command::prepare(
                setup,
                &mapping.calibrations,
                &inventory,
                option,
                plan,
                cancel,
            )?;
            return Ok(Some(CalibratedLaunch {
                duckstation: Some(native),
                mgba: None,
                #[cfg(target_os = "linux")]
                dolphin_native: None,
                #[cfg(target_os = "linux")]
                snes9x_native: None,
                #[cfg(target_os = "linux")]
                fceux_native: None,
                #[cfg(target_os = "linux")]
                sameboy_native: None,
                bsnes_native: None,
                #[cfg(target_os = "linux")]
                mednafen_native: None,
                #[cfg(target_os = "linux")]
                mame_native: None,
                #[cfg(target_os = "linux")]
                flycast_native: None,
                #[cfg(target_os = "linux")]
                pcsx2_native: None,
                #[cfg(target_os = "linux")]
                rpcs3_native: None,
                #[cfg(target_os = "linux")]
                melonds_native: None,
                ppsspp: None,
                _directory: None,
                bizhawk: None,
                bizhawk_topology: None,
                transports: Vec::new(),
                crocods: None,
                ep128emu: None,
                hatari: None,
                simcp: None,
                steemsse: None,
                scummvm: None,
                dolphin: None,
                same_cdi: None,
                fbneo: None,
                mame: None,
                puae: None,
                stella: None,
                description: format!(
                    "DuckStation: {} calibrated native players; startup confirmation required",
                    setup.players.len()
                ),
            }));
        }
    }
    #[cfg(target_os = "linux")]
    if option.runtime_kind == EmulatorRuntimeKind::Standalone
        && option.emulator_name.eq_ignore_ascii_case("BizHawk")
        && let Some(native) = mapping.native_launch_for(&option.emulator_id, platform)?
    {
        native.validate()?;
        ensure!(
            native.emulator_id == option.emulator_id,
            "Saved native controller setup belongs to a different BizHawk installation"
        );
        let EmulatorExecutable::Native(executable) = &option.executable else {
            bail!(
                "Native BizHawk calibration requires a Linux runtime; Wine and Flatpak need separate runtime adapters"
            );
        };
        let executable = std::fs::canonicalize(executable)?;
        ensure!(
            std::fs::canonicalize(&plan.program)? == executable,
            "BizHawk launch program changed during controller preparation"
        );
        let directory = std::fs::canonicalize(&native.exe_directory)?;
        ensure!(
            (native.mono_program.is_some() || executable.parent() == Some(directory.as_path()))
                && directory.join("EmuHawk.exe").is_file(),
            "Resolve the actual EmuHawk executable directory before using native calibration"
        );
        // Work on a copy: failure must not leave a half-rewritten invocation.
        let mut native_plan = plan.clone();
        if let Some(mono) = &native.mono_program {
            use std::os::unix::fs::PermissionsExt;
            let mono = std::fs::canonicalize(mono).context("Resolving direct Mono executable")?;
            let mut file = std::fs::File::open(&mono)?;
            let mut magic = [0u8; 4];
            file.read_exact(&mut magic)?;
            ensure!(
                magic == *b"\x7fELF" && file.metadata()?.permissions().mode() & 0o111 != 0,
                "Direct Mono mode requires an executable ELF binary, not another shell wrapper"
            );
            native_plan.program = mono;
            native_plan.current_directory = std::fs::canonicalize(native.working_directory())
                .context("Resolve the existing native runtime data directory")?;
            ensure!(
                native_plan.current_directory.is_dir(),
                "Native runtime data path is not a directory"
            );
            if let Some(config) = &native.base_config {
                ensure!(
                    !native_plan
                        .arguments
                        .iter()
                        .take_while(|argument| *argument != "--")
                        .filter_map(|argument| argument.to_str())
                        .any(|argument| argument == "--config"
                            || argument.starts_with("--config=")
                            || argument.starts_with("--config:")),
                    "Explicit base config conflicts with an existing launch --config argument"
                );
                native_plan.arguments.splice(
                    0..0,
                    [OsString::from("--config"), config.as_os_str().to_owned()],
                );
            }
            for (key, value) in native.controlled_environment()? {
                native_plan
                    .environment
                    .retain(|(previous, _)| previous != &key);
                native_plan.environment.push((key, value));
            }
        }
        let mut paths = Vec::with_capacity(native.players.len());
        let mut native_calibrations = Vec::with_capacity(native.players.len());
        let mut native_transports = Vec::new();
        for player in &native.players {
            let mut matches = inventory
                .iter()
                .filter(|device| device.stable_id == player.controller_id);
            let device = matches
                .next()
                .context("A configured BizHawk controller is disconnected")?;
            ensure!(
                matches.next().is_none(),
                "Configured BizHawk controller identity is ambiguous"
            );
            let calibration = mapping
                .calibrations
                .get(&player.controller_id)
                .context("A configured BizHawk player has no saved calibration")?;
            if player.normalized_input {
                #[cfg(target_pointer_width = "64")]
                {
                    let (transport, calibration, path) = prepare_native_normalized_transport(
                        calibration,
                        device,
                        native
                            .digital_deck()
                            .map(|deck| deck.layout_id(player.virtual_port + 1))
                            .transpose()?,
                        player.dualshock,
                        player.dualanalog,
                        player.analog_joystick,
                        player.rhythm,
                        player.negcon,
                        player.pointer,
                        player.desktop_cursor,
                        player.analog_toggle_id.as_deref(),
                        cancel,
                    )?;
                    native_transports.push(transport);
                    native_calibrations.push(calibration);
                    paths.push(
                        path.to_str()
                            .context("Virtual controller path is not UTF-8")?
                            .to_owned(),
                    );
                }
                #[cfg(not(target_pointer_width = "64"))]
                bail!("Native normalized input currently requires 64-bit Linux");
            } else {
                native_calibrations.push(calibration.clone());
                paths.push(
                    device
                        .device_path
                        .to_str()
                        .context("Controller path is not UTF-8")?
                        .to_owned(),
                );
            }
        }
        let mut requests = Vec::with_capacity(native.players.len());
        for ((player, path), calibration) in
            native.players.iter().zip(&paths).zip(&native_calibrations)
        {
            requests.push(crate::controller_bizhawk::NativePadRequest {
                logical: if player.normalized_input {
                    None
                } else {
                    mapping.native_logical_calibration(native, &player.controller_id)
                },
                normalized_input: player.normalized_input,
                calibration,
                runtime_path: path,
                virtual_port: player.virtual_port,
                dualshock: player.dualshock,
                dualanalog: player.dualanalog,
                analog_joystick: player.analog_joystick,
                rhythm: player.rhythm,
                negcon: player.negcon,
                pointer: player.pointer,
                desktop_cursor: player.desktop_cursor,
                mouse_speed: f32::from(player.mouse_speed_basis_points) / 10000.0,
                analog_toggle_id: player.analog_toggle_id.as_deref(),
                deadzone: f32::from(player.deadzone_basis_points) / 10000.0,
                rumble: player.rumble,
            });
        }
        let mut session = if let Some(deck) = native.digital_deck() {
            let digital_requests: Vec<_> = requests
                .iter()
                .map(
                    |request| crate::controller_bizhawk::digital_session::PlayerRequest {
                        player: request.virtual_port + 1,
                        calibration: request.calibration,
                        logical: request.logical,
                        normalized_input: request.normalized_input,
                        runtime_path: request.runtime_path,
                    },
                )
                .collect();
            crate::controller_bizhawk::digital_session::prepare_session(
                &mut native_plan,
                &directory,
                &native.probe_program,
                &native.sdl_library,
                deck,
                &digital_requests,
                cancel,
            )?
        } else {
            crate::controller_bizhawk::prepare_native_session(
                &mut native_plan,
                &directory,
                &native.probe_program,
                &native.sdl_library,
                &requests,
                native.multitaps,
                cancel,
            )?
        };
        session.transports = native_transports;
        session.check_launch_inputs()?;
        if native.mono_program.is_some() {
            native_plan
                .arguments
                .insert(0, directory.join("EmuHawk.exe").into_os_string());
        }
        if !warnings.is_empty() {
            session.description.push_str(" · ");
            session.description.push_str(&warnings.join(" · "));
        }
        *plan = native_plan;
        return Ok(Some(session));
    }
    let mut devices = selected_devices(settings, &inventory, platform);
    if settings.controller_mapping.explicit_player_selection {
        ensure!(
            devices.len() == settings.controller_mapping.player_mappings.len(),
            "A selected player's controller is disconnected. Reconnect it or change the players in Controller setup."
        );
        for (index, device) in devices.iter().enumerate() {
            ensure!(
                settings
                    .controller_mapping
                    .calibrations
                    .contains_key(&device.stable_id)
                    && !settings
                        .controller_mapping
                        .hidden_controller_ids
                        .contains(&device.stable_id),
                "Finish setting up Player {} in Controller setup before launching.",
                index + 1
            );
        }
    }
    if devices.is_empty() {
        return Ok(None);
    }
    if option.runtime_kind == EmulatorRuntimeKind::Standalone
        && option.emulator_name.eq_ignore_ascii_case("ares")
    {
        let directory =
            crate::controller_ares::prepare(settings, platform, option, plan, &devices, cancel)?;
        return Ok(Some(CalibratedLaunch {
            _directory: Some(directory),
            description: format!(
                "ares: applied {} player mapping(s) to a private settings file",
                devices.len()
            ),
            ..Default::default()
        }));
    }
    ensure!(
        option.runtime_kind == EmulatorRuntimeKind::RetroArch,
        "Calibrated launch adapter for {} is not implemented yet. Disable Apply saved calibrations to keep its native setup.",
        option.emulator_name
    );
    ensure!(
        matches!(
            &option.executable,
            EmulatorExecutable::Native(_) | EmulatorExecutable::Flatpak { .. }
        ),
        "Calibrated launch for Wine RetroArch is not implemented yet"
    );
    let selected = mapping
        .launch_mode_selections
        .get(&selection_key(&option.core_name, platform));
    let arcade_default = (option.core_name == "mame" && platform == "Arcade")
        .then_some(mapping.mame_arcade_layout)
        .flatten()
        .filter(|layout| *layout == crate::controller_mame::DigitalLayout::EightButton)
        .and_then(|_| {
            catalog()
                .platform_profiles("mame", platform)
                .into_iter()
                .find(|profile| profile.id == "retroarch:mame:arcade-8")
        });
    let profile = if let Some(profile) = guided_target {
        profile
    } else if let Some(id) = selected {
        catalog().platform_profiles(&option.core_name, platform).into_iter().find(|profile| profile.id == *id && profile.explicit_selection)
            .context("Selected controller mode is no longer available for this core/platform; choose a mode in Controller coverage")?
    } else {
        arcade_default.or_else(|| contract(&option.core_name, platform)).context("No automatic controller contract for this core/platform yet; choose an available explicit mode in Controller coverage, or disable Apply saved calibrations to use native setup")?
    };
    if profile.requires_fresh_start {
        let content = plan
            .retroarch_content
            .as_ref()
            .context("Fresh-start controller mode requires prepared content identity")?;
        // In particular, do not accept custom entry-state/config flags that can
        // bypass the base configuration's auto-load setting.
        crate::controller_launch_modes::validate_arguments(
            emulator_arguments(plan, &option.executable)?,
            content,
        )?;
        ensure!(
            !plan
                .environment
                .iter()
                .any(|(key, _)| key == "HOME" || key == "XDG_CONFIG_HOME"),
            "Custom emulator environment needs save-state configuration resolution"
        );
        let (_, config) = retroarch_base(&option.executable)?;
        ensure!(
            !config
                .lines()
                .any(|line| line.trim_start().starts_with("#include")),
            "Included RetroArch configuration needs save-state mode resolution"
        );
        ensure!(
            !config_bool(&config, "savestate_auto_load", false)?,
            "{} requires a fresh launch: automatic save-state loading can restore different controller devices. Disable auto-load or use native setup; the saved state has not been changed.",
            profile.name
        );
    }
    let same_cdi = if profile.core == "same_cdi" {
        Some(prepare_same_cdi_input(option, plan, profile)?)
    } else {
        None
    };
    let dolphin = if profile.core == "dolphin" {
        ensure!(
            profile.content_guard
                == Some(crate::controller_catalog::ContentGuard::DolphinGamecubeRaw)
                && matches!(&option.executable, EmulatorExecutable::Native(_))
                && plan.environment.is_empty(),
            "Dolphin requires its fixed GameCube guard and native RetroArch without custom environment"
        );
        let (_, base) = retroarch_base(&option.executable)?;
        ensure!(
            !base
                .lines()
                .any(|line| line.trim_start().starts_with("#include")),
            "Dolphin requires included RetroArch configuration to be resolved"
        );
        let system = cfg_value(&base, "system_directory")?
            .filter(|value| !value.is_empty() && value != "default")
            .context("Dolphin requires an explicit system_directory")?;
        let content = plan
            .retroarch_content
            .as_ref()
            .context("Dolphin requires prepared content identity")?;
        for key in [
            "sort_savefiles_enable",
            "sort_savefiles_by_content_enable",
            "savefiles_in_content_dir",
        ] {
            ensure!(
                cfg_value(&base, key)?.is_some(),
                "Dolphin needs explicit {key} to resolve its native User directory"
            );
        }
        let parent = content
            .content
            .parent()
            .context("Dolphin content has no parent directory")?;
        let mut save = if config_bool(&base, "savefiles_in_content_dir", false)? {
            parent.to_path_buf()
        } else {
            let value = cfg_value(&base, "savefile_directory")?
                .filter(|value| !value.is_empty() && value != "default")
                .context("Dolphin requires an explicit savefile_directory")?;
            configured_path(&value)?
        };
        if config_bool(&base, "sort_savefiles_by_content_enable", false)? {
            save.push(
                parent
                    .file_name()
                    .context("Dolphin save sorting requires a named content directory")?,
            );
        }
        if config_bool(&base, "sort_savefiles_enable", false)? {
            save.push("dolphin-emu");
        }
        crate::controller_launch_modes::validate_arguments(
            emulator_arguments(plan, &option.executable)?,
            content,
        )?;
        Some(crate::controller_dolphin::prepare(
            &configured_path(&system)?,
            &save,
            &content.content,
        )?)
    } else {
        None
    };
    let scummvm = if profile.core == "scummvm" {
        ensure!(
            profile.content_guard
                == Some(crate::controller_catalog::ContentGuard::ScummvmRetropadCursor)
                && matches!(&option.executable, EmulatorExecutable::Native(_))
                && plan.environment.is_empty(),
            "ScummVM requires its fixed RetroPad contract and native RetroArch without custom environment"
        );
        let (_, base) = retroarch_base(&option.executable)?;
        ensure!(
            !base
                .lines()
                .any(|line| line.trim_start().starts_with("#include")),
            "ScummVM needs included RetroArch configuration resolved before launch"
        );
        let value = cfg_value(&base, "system_directory")?
            .filter(|value| !value.is_empty() && value != "default")
            .context("ScummVM requires an explicit system_directory")?;
        let content = plan
            .retroarch_content
            .as_ref()
            .context("ScummVM requires prepared content identity")?;
        crate::controller_launch_modes::validate_arguments(
            emulator_arguments(plan, &option.executable)?,
            content,
        )?;
        Some(crate::controller_scummvm::prepare(
            &configured_path(&value)?,
            &content.content,
        )?)
    } else {
        None
    };
    let steemsse = if profile.core == "steemsse" {
        ensure!(
            cfg!(target_os = "windows"),
            "Steem SSE r1501 supplies a Windows libretro core; native Linux/macOS or Windows-runtime controller transport must be implemented before this mode can launch"
        );
        ensure!(
            profile.content_guard
                == Some(crate::controller_catalog::ContentGuard::SteemsseEmbeddedSte)
                && matches!(&option.executable, EmulatorExecutable::Native(_))
                && plan.environment.is_empty(),
            "Steem SSE requires its explicit embedded-ROM contract and native RetroArch without custom environment"
        );
        let (_, base) = retroarch_base(&option.executable)?;
        ensure!(
            !base
                .lines()
                .any(|line| line.trim_start().starts_with("#include")),
            "Steem SSE needs included RetroArch configuration resolved before launch"
        );
        let value = cfg_value(&base, "system_directory")?
            .filter(|value| !value.is_empty() && value != "default")
            .context("Steem SSE requires an explicit system_directory")?;
        Some(crate::controller_steemsse::prepare(&configured_path(
            &value,
        )?)?)
    } else {
        None
    };
    let simcp = if profile.core == "simcp" {
        ensure!(
            cfg!(target_os = "linux")
                && matches!(&option.executable, EmulatorExecutable::Native(_))
                && plan.environment.is_empty()
                && profile.explicit_selection
                && profile.requires_fresh_start,
            "Patched SimCoupe input requires an explicit fresh native Linux RetroArch launch without custom environment"
        );
        let content = plan
            .retroarch_content
            .as_ref()
            .context("SimCoupe requires prepared core/content identity")?;
        crate::controller_launch_modes::validate_arguments(
            emulator_arguments(plan, &option.executable)?,
            content,
        )?;
        let interface = profile
            .content_guard
            .and_then(crate::controller_catalog::ContentGuard::simcp_interface)
            .context("SimCoupe needs an explicit emulated joystick interface")?;
        let home =
            std::env::var_os("HOME").context("SimCoupe native configuration requires HOME")?;
        Some(crate::controller_simcp::prepare_input(
            &content.core,
            Path::new(&home),
            interface,
        )?)
    } else {
        None
    };
    let hatari = if profile.core == "hatari" {
        ensure!(
            profile.content_guard
                == Some(crate::controller_catalog::ContentGuard::HatariStJoystick),
            "Hatari calibrated launch requires an explicit supported input contract"
        );
        Some(prepare_hatari_input(option, plan)?)
    } else {
        None
    };
    let ep128emu = if matches!(
        profile.content_guard,
        Some(
            crate::controller_catalog::ContentGuard::Ep128emuCpcDefaults
                | crate::controller_catalog::ContentGuard::Ep128emuEnterpriseDefaults
                | crate::controller_catalog::ContentGuard::Ep128emuZxDefaults
                | crate::controller_catalog::ContentGuard::Ep128emuTvcDefaults
        )
    ) {
        ensure!(
            matches!(&option.executable, EmulatorExecutable::Native(_))
                && plan.environment.is_empty(),
            "ep128emu configuration resolution currently needs native RetroArch without custom environment"
        );
        let (_, base) = retroarch_base(&option.executable)?;
        ensure!(
            !base
                .lines()
                .any(|line| line.trim_start().starts_with("#include")),
            "ep128emu needs included configuration resolved before calibrated launch"
        );
        let value = cfg_value(&base, "system_directory")?
            .filter(|value| !value.is_empty() && value != "default")
            .context("ep128emu needs an explicit system_directory for configuration resolution")?;
        let system = configured_path(&value)?;
        let content = plan
            .retroarch_content
            .as_ref()
            .context("ep128emu needs prepared content identity")?;
        Some(
            if profile.content_guard
                == Some(crate::controller_catalog::ContentGuard::Ep128emuEnterpriseDefaults)
            {
                crate::controller_ep128emu::prepare_enterprise(&system, &content.content)?
            } else if profile.content_guard
                == Some(crate::controller_catalog::ContentGuard::Ep128emuZxDefaults)
            {
                crate::controller_ep128emu::prepare_zx(&system, &content.content)?
            } else if profile.content_guard
                == Some(crate::controller_catalog::ContentGuard::Ep128emuTvcDefaults)
            {
                crate::controller_ep128emu::prepare_tvc(&system, &content.content)?
            } else {
                crate::controller_ep128emu::prepare_cpc(&system, &content.content)?
            },
        )
    } else {
        None
    };
    let puae =
        if profile.content_guard == Some(crate::controller_catalog::ContentGuard::PuaeFixedInput) {
            Some(prepare_puae_input(option, plan, profile)?)
        } else {
            None
        };
    let crocods = if profile.content_guard
        == Some(crate::controller_catalog::ContentGuard::CrocodsJoystick)
    {
        ensure!(
            matches!(&option.executable, EmulatorExecutable::Native(_)),
            "CrocoDS calibrated input currently resolves native Linux HOME only; Flatpak's persisted-home mapping still needs an adapter"
        );
        let home = std::env::var_os("HOME").context("CrocoDS needs its effective HOME")?;
        let content = plan
            .retroarch_content
            .as_ref()
            .context("CrocoDS needs prepared content identity")?;
        Some(crate::controller_crocods::prepare(
            Path::new(&home),
            &content.content,
        )?)
    } else {
        None
    };
    if !profile.content_extensions.is_empty() || profile.content_guard.is_some() {
        let content = plan
            .retroarch_content
            .as_ref()
            .context("This core input profile requires prepared content identity")?;
        crate::controller_launch_modes::validate_arguments(
            emulator_arguments(plan, &option.executable)?,
            content,
        )?;
        let extension = content
            .content
            .extension()
            .and_then(|value| value.to_str())
            .unwrap_or("")
            .to_ascii_lowercase();
        ensure!(
            profile.content_extensions.contains(&extension),
            "{} does not cover this content type; choose a profile for its disk/music/peripheral controls",
            profile.name
        );
        if ep128emu.is_none()
            && dolphin.is_none()
            && scummvm.is_none()
            && steemsse.is_none()
            && simcp.is_none()
            && hatari.is_none()
            && crocods.is_none()
            && puae.is_none()
            && profile
                .content_guard
                .and_then(|guard| guard.stella_contract())
                .is_none()
        {
            validate_content_guard(profile, &content.content)?;
        }
    }
    #[cfg(target_os = "linux")]
    let stella = if let Some(expected) = profile
        .content_guard
        .and_then(|guard| guard.stella_contract())
    {
        check_preparation_cancel(cancel)?;
        let prepared = prepare_stella_detection(option, plan)?;
        let routes = prepared.context.detect_digital_contract(expected, cancel)?;
        ensure!(
            routes.len() == profile.frontend_port_count(),
            "Stella profile disagrees with detected frontend port count"
        );
        for route in routes {
            let expected: BTreeMap<String, String> = route
                .digital_outputs()?
                .into_iter()
                .map(|(control, output)| (control.to_owned(), output.to_owned()))
                .collect();
            ensure!(
                profile.for_port(route.frontend_port).bindings == expected,
                "Stella profile disagrees with detected per-port controller/console outputs"
            );
        }
        Some(prepared)
    } else {
        None
    };
    #[cfg(not(target_os = "linux"))]
    ensure!(
        profile
            .content_guard
            .and_then(|guard| guard.stella_contract())
            .is_none(),
        "Stella controller detection requires Linux"
    );
    if selected.is_none()
        && (catalog().launch_modes(&option.core_name, platform).len() > 1
            || profile
                .retroarch_launch
                .as_ref()
                .is_some_and(|launch| launch.player_topology.is_some()))
    {
        return prepare_mode_aware(settings, platform, option, plan, &devices, cancel);
    }
    // A connected N30 must not prevent a calibrated Brawler64 from playing N64.
    // Only controllers with every required target capability enter the player list.
    let launch = profile.retroarch_launch.as_ref().unwrap();
    devices.retain(|device| {
        (1..=launch.max_players).any(|port| {
            compatible(
                &mapping.calibrations[&device.stable_id],
                &profile.for_port(port),
            )
        })
    });
    ensure!(
        !devices.is_empty(),
        "No connected calibration has all required controls for {}. Complete calibration (older calibrations need physical-input capture), or disable Apply saved calibrations.",
        profile.name
    );
    let eligible_count = devices.len();
    if profile.port_controls.is_empty()
        && profile.port_layouts.is_empty()
        && profile.port_bindings.is_empty()
    {
        restrict_players(&mut devices, profile)?;
    }
    let targets: Vec<_> = (1..=launch.max_players)
        .map(|port| (port, profile))
        .collect();
    let players = assign_players(settings, &targets, &devices)?;
    let cache = directories::BaseDirs::new()
        .context("Finding controller launch cache")?
        .cache_dir()
        .join("lunchbox/controller-launch");
    std::fs::create_dir_all(&cache)?;
    let directory = tempfile::Builder::new()
        .prefix("session-")
        .tempdir_in(cache)?;
    let mut config = String::from(
        "# Lunchbox per-launch physical calibration. User config is never rewritten.\ninput_joypad_driver = \"linuxraw\"\ninput_autodetect_enable = \"false\"\nauto_remaps_enable = \"false\"\nauto_overrides_enable = \"false\"\nconfig_save_on_exit = \"false\"\nremap_save_on_exit = \"false\"\n",
    );
    let mut transports = Vec::new();
    for (port, port_profile, device) in &players {
        let calibration = &mapping.calibrations[&device.stable_id];
        let port_profile = port_profile.for_port(*port);
        let transport = prepare_player_transport(calibration, &port_profile, device, cancel)?;
        config.push_str(&player_config_transport(
            calibration,
            &port_profile,
            &transport.numbering,
            *port,
            port_profile
                .launch_device_for_port(*port)
                .context("Missing launch controller mode")?,
            &transport.pressure_codes,
        )?);
        transports.push(transport);
    }
    // Do not leave later ports pointing at one of these same devices through an
    // inherited automatic configuration. Players are the calibrated selection.
    let highest_port = players
        .iter()
        .map(|(port, _, _)| *port)
        .chain(profile.port_devices.keys().copied())
        .max()
        .unwrap();
    config.push_str(&format!("input_max_users = \"{highest_port}\"\n"));
    for port in 1..=profile.frontend_port_count() {
        if !players.iter().any(|(assigned, _, _)| *assigned == port) {
            config.push_str(&empty_port_config(
                port,
                profile.port_devices.get(&port).copied().unwrap_or(0),
            ));
        }
    }
    let path = directory.path().join("controllers.cfg");
    #[cfg(target_os = "linux")]
    if let Some(stella) = &stella {
        // Always freeze the accepted detection options, including an empty map.
        // Otherwise normal per-game/core defaults could diverge after detection.
        let options = core_options_overlay("", &stella.options)?;
        config.push_str(&write_options_file(&options, directory.path())?);
    } else {
        config.push_str(&write_core_options(
            profile,
            plan,
            &option.executable,
            directory.path(),
        )?);
    }
    #[cfg(not(target_os = "linux"))]
    config.push_str(&write_core_options(
        profile,
        plan,
        &option.executable,
        directory.path(),
    )?);
    crate::controller_launch_modes::validate_generated_modes(
        &config,
        emulator_arguments(plan, &option.executable)?,
        profile.frontend_port_count(),
    )?;
    if let Some(puae) = &puae {
        config.push_str(&puae.append_config()?);
    }
    if let Some(ep128emu) = &ep128emu {
        config.push_str(&ep128emu.append_config()?);
    }
    if let Some(hatari) = &hatari {
        config.push_str(&hatari.append_config()?);
    }
    if let Some(steemsse) = &steemsse {
        config.push_str(&steemsse.append_config()?);
    }
    if let Some(scummvm) = &scummvm {
        config.push_str(&scummvm.append_config()?);
    }
    if let Some(dolphin) = &dolphin {
        config.push_str(dolphin.append_config());
    }
    if let Some(same_cdi) = &same_cdi {
        config.push_str(&same_cdi.append_config()?);
    }
    std::fs::write(&path, config)?;
    check_preparation_cancel(cancel)?;
    if let Some(same_cdi) = &same_cdi {
        // Core-option lookup and generated-mode validation above used the
        // original disc. Only now substitute the session-owned command file.
        let original = &plan
            .retroarch_content
            .as_ref()
            .context("SAME CD-i original disc identity disappeared")?
            .content;
        let positions = plan
            .arguments
            .iter()
            .enumerate()
            .filter_map(|(index, value)| (value == original.as_os_str()).then_some(index))
            .collect::<Vec<_>>();
        ensure!(
            positions.len() == 1,
            "SAME CD-i disc argument is not unique"
        );
        let mut staged = plan.clone();
        staged.arguments[positions[0]] =
            same_cdi.configuration.command_file.clone().into_os_string();
        staged.retroarch_content.as_mut().unwrap().content =
            same_cdi.configuration.command_file.clone();
        attach_config(&mut staged, &option.executable, &path)?;
        same_cdi.verify()?;
        *plan = staged;
    } else {
        attach_config(plan, &option.executable, &path)?;
    }
    Ok(Some(CalibratedLaunch {
        _directory: Some(directory),
        bizhawk: None,
        #[cfg(target_os = "linux")]
        bizhawk_topology: None,
        transports,
        crocods,
        #[cfg(target_os = "linux")]
        mgba: None,
        #[cfg(target_os = "linux")]
        dolphin_native: None,
        #[cfg(target_os = "linux")]
        snes9x_native: None,
        #[cfg(target_os = "linux")]
        fceux_native: None,
        #[cfg(target_os = "linux")]
        sameboy_native: None,
        bsnes_native: None,
        #[cfg(target_os = "linux")]
        mednafen_native: None,
        #[cfg(target_os = "linux")]
        mame_native: None,
        #[cfg(target_os = "linux")]
        flycast_native: None,
        #[cfg(target_os = "linux")]
        pcsx2_native: None,
        #[cfg(target_os = "linux")]
        rpcs3_native: None,
        #[cfg(target_os = "linux")]
        melonds_native: None,
        #[cfg(target_os = "linux")]
        ppsspp: None,
        #[cfg(target_os = "linux")]
        duckstation: None,
        ep128emu,
        hatari,
        simcp,
        steemsse,
        scummvm,
        dolphin,
        same_cdi,
        fbneo: None,
        mame: None,
        puae,
        #[cfg(target_os = "linux")]
        stella,
        description: format!(
            "Applied {} of {} compatible calibrated controller(s) for {} · RetroArch automatic overrides/remaps suspended for this launch",
            players.len(),
            eligible_count,
            profile.name
        ),
    }))
}

fn validate_content_guard(profile: &EmulatorProfile, path: &Path) -> Result<()> {
    use crate::controller_catalog::ContentGuard;
    match profile.content_guard {
        Some(guard @ (ContentGuard::AtariComputerMedia | ContentGuard::Atari5200Cartridge)) => {
            let extension = path
                .extension()
                .and_then(|extension| extension.to_str())
                .context("Atari800 media routing needs a UTF-8 extension")?;
            // HandleExtension compares exactly three characters and the caller
            // tries only lower-case and upper-case spellings, not mixed case.
            ensure!(
                extension == extension.to_ascii_lowercase()
                    || extension == extension.to_ascii_uppercase(),
                "Atari800 does not consistently route mixed-case file extensions; use an all-lowercase or all-uppercase extension"
            );
            if guard == ContentGuard::Atari5200Cartridge {
                ensure!(
                    extension.eq_ignore_ascii_case("a52"),
                    "5200 mode requires an explicit A52 cartridge path"
                );
                return Ok(());
            }
            let mut file = std::fs::File::open(path)
                .context("Opening Atari computer media for type identification")?;
            let mut header = [0u8; 4];
            file.read_exact(&mut header)
                .context("Atari media header is incomplete")?;
            let size = file.metadata()?.len();
            let identified = match extension.to_ascii_lowercase().as_str() {
                "atr" => header[..2] == [0x96, 0x02],
                "atx" => &header == b"AT8X",
                "xex" | "com" => header[..2] == [0xff, 0xff] && header[2..] != [0xff, 0xff],
                "dcm" => matches!(header[0], 0xf9 | 0xfa),
                // AFILE_DetectFileType checks raw ROM sizes before tape magic.
                "cas" => {
                    &header == b"FUJI"
                        && !(size >= 4096 && (size.is_power_of_two() || size == 40 * 1024))
                }
                _ => false,
            };
            ensure!(
                identified,
                "Atari computer mode needs recognized uncompressed disk/tape/executable media; this input could select a cartridge or restore machine state"
            );
            Ok(())
        }
        Some(ContentGuard::BkJoystick) => {
            // load_game_real searches the entire supplied path. BASIC wins
            // over FOCAL, and either marker overrides the selected model.
            let path = path
                .to_str()
                .context("BK model routing needs a UTF-8 content path")?
                .to_ascii_uppercase();
            let forced = if path.contains("BASIC") {
                Some("BK-0010.01")
            } else if path.contains("FOCAL") {
                Some("BK-0010")
            } else {
                None
            };
            let expected = profile.core_options.get("bk_model").map(String::as_str);
            ensure!(
                forced.is_none() || forced == expected,
                "BK content path overrides the selected machine model to {}; choose that model's joystick profile",
                forced.unwrap_or("the configured model")
            );
            Ok(())
        }
        Some(
            ContentGuard::Ep128emuCpcDefaults
            | ContentGuard::Ep128emuEnterpriseDefaults
            | ContentGuard::Ep128emuZxDefaults
            | ContentGuard::Ep128emuTvcDefaults,
        ) => bail!("ep128emu requires effective system directory and configuration resolution"),
        Some(
            ContentGuard::SimcpSamOne | ContentGuard::SimcpSamTwo | ContentGuard::SimcpKempston,
        ) => {
            bail!("SimCoupe requires patched core identity and native configuration resolution")
        }
        Some(ContentGuard::SteemsseEmbeddedSte) => {
            bail!("Steem SSE requires embedded-ROM and system-directory resolution")
        }
        Some(ContentGuard::ScummvmRetropadCursor) => {
            bail!("ScummVM requires startup and native keymapper configuration resolution")
        }
        Some(ContentGuard::DolphinGamecubeRaw) => {
            bail!("Dolphin requires retained content and native configuration preparation")
        }
        Some(ContentGuard::HatariStJoystick) => {
            bail!("Hatari requires effective system/save directory and configuration resolution")
        }
        Some(ContentGuard::CrocodsJoystick) => {
            bail!("CrocoDS requires effective HOME and configuration resolution")
        }
        Some(ContentGuard::PuaeFixedInput) => {
            bail!("PUAE requires effective save-directory and custom configuration resolution")
        }
        Some(ContentGuard::Quasi88Disk) => {
            let text = path
                .to_str()
                .context("QUASI88 content routing requires a UTF-8 path")?;
            ensure!(
                !text.contains(".m3u"),
                "QUASI88 treats any path containing lowercase .m3u as a playlist; this disk profile cannot use that path"
            );
            Ok(())
        }
        Some(guard @ (ContentGuard::ViceJoystickPort1 | ContentGuard::ViceJoystickPort2)) => {
            // VICE check_joystick_control scans the whole supplied path with
            // case-insensitive substring matching, with j1 taking precedence.
            // These markers override the core option and lock it until restart.
            let text = path
                .to_str()
                .context("VICE controller routing needs a UTF-8 content path")?
                .to_ascii_lowercase();
            let forced = if text.contains("_j1.") || text.contains("(j1).") {
                Some(1)
            } else if text.contains("_j2.") || text.contains("(j2).") {
                Some(2)
            } else {
                None
            };
            let expected = if guard == ContentGuard::ViceJoystickPort1 {
                1
            } else {
                2
            };
            ensure!(
                forced.is_none_or(|port| port == expected),
                "VICE content path forces joystick port {}; choose that port's controller profile",
                forced.unwrap_or(expected)
            );
            Ok(())
        }
        Some(
            ContentGuard::StellaJoysticks
            | ContentGuard::StellaGenesisPads
            | ContentGuard::StellaBoosterGripOrJoy2BPlus,
        ) => {
            bail!("Stella content requires its runtime controller-report guard")
        }
        None => Ok(()),
        Some(ContentGuard::GeolithStandardCartridge) => {
            // Geolith geo_neo_load reads NEO v1 magic and the little-endian
            // NGH at byte 40. Match its identity source, not the file's name.
            let mut file = std::fs::File::open(path)
                .context("Opening NEO content for controller-mode identification")?;
            let mut header = [0u8; 4096];
            file.read_exact(&mut header).context(
                "NEO controller-mode identification requires the complete 4096-byte header",
            )?;
            ensure!(
                &header[..4] == b"NEO\x01",
                "Unrecognized NEO header; cannot identify this cartridge's controller mode"
            );
            let ngh = u32::from_le_bytes([header[40], header[41], header[42], header[43]]);
            ensure!(
                !matches!(ngh, 0x004 | 0x027 | 0x036 | 0x048 | 0x236 | 0x3e7 | 0x999),
                "NEO cartridge NGH {ngh:03X} has a mahjong, trackball or V-Liner control interface; choose a dedicated peripheral contract or native setup"
            );
            // This is input-mode identification, not full ROM validation.
            Ok(())
        }
    }
}

fn ensure_no_mode_overrides(directory: &Path, library: &str) -> Result<()> {
    if !directory.exists() {
        return Ok(());
    }
    // RetroArch opens symlinked configs normally (common with declarative
    // configuration). Follow them too; an unreadable/cyclic tree is unresolved,
    // never evidence that no device-changing override exists.
    for entry in walkdir::WalkDir::new(directory).follow_links(true) {
        let entry = entry?;
        ensure!(
            !entry.file_type().is_file()
                || !entry
                    .path()
                    .extension()
                    .is_some_and(|ext| ext == "rmp" || ext == "cfg"),
            "Saved {library} core/game overrides need effective controller-mode resolution before calibrated launch"
        );
    }
    Ok(())
}

/// Independent target modes per console port, selected from the emulator's
/// configuration, never inferred by downgrading an incompatible physical pad.
fn prepare_mode_aware(
    settings: &AppSettings,
    platform: &str,
    option: &RomEmulatorOption,
    plan: &mut LaunchPlan,
    devices: &[&ControllerDevice],
    cancel: &std::sync::atomic::AtomicBool,
) -> Result<Option<CalibratedLaunch>> {
    check_preparation_cancel(cancel)?;
    let (base, config) = retroarch_base(&option.executable)?;
    ensure!(
        !plan
            .environment
            .iter()
            .any(|(key, _)| key == "XDG_CONFIG_HOME" || key == "HOME"),
        "Custom emulator environment needs effective controller-mode resolution"
    );
    let profiles = catalog().launch_modes(&option.core_name, platform);
    let first = profiles.first().context("No controller mode contract")?;
    let ports = first.frontend_port_count();
    ensure!(
        profiles
            .iter()
            .all(|profile| profile.frontend_port_count() == ports
                && profile.retroarch_launch.as_ref().unwrap().max_players
                    == first.retroarch_launch.as_ref().unwrap().max_players
                && profile.core_options == first.core_options
                && profile.retroarch_library == first.retroarch_library
                && profile.retroarch_launch.as_ref().unwrap().player_topology
                    == first.retroarch_launch.as_ref().unwrap().player_topology),
        "Controller modes disagree on port topology, library name or core options"
    );
    let library = first
        .retroarch_library
        .as_ref()
        .context("Mode-aware core lacks its exact library name")?;
    let arguments = emulator_arguments(plan, &option.executable)?;
    ensure!(
        !arguments.iter().any(|arg| arg == "--config"
            || arg.to_string_lossy().starts_with("-c")
            || arg.to_string_lossy().starts_with("--config=")
            || arg.to_string_lossy().starts_with("--appendconfig")),
        "Custom RetroArch config arguments need effective controller-mode resolution"
    );
    // A saved core/game remap can select a device mode, not just button wiring.
    // Do not silently fall back to the base mode while those layers are unresolved.
    let remaps = cfg_value(&config, "input_remapping_directory")?
        .filter(|v| !v.is_empty())
        .map(|value| configured_path(&value))
        .transpose()?
        .unwrap_or_else(|| base.join("config/remaps"));
    let overrides = cfg_value(&config, "rgui_config_directory")?
        .filter(|v| !v.is_empty())
        .map(|value| configured_path(&value))
        .transpose()?
        .unwrap_or_else(|| base.join("config"));
    for directory in [remaps.join(library), overrides.join(library)] {
        ensure_no_mode_overrides(&directory, library)?;
    }
    let beetle_content = beetle_launch_content(
        &option.core_name,
        arguments,
        plan.retroarch_content.as_ref(),
    )?;
    let option_content = if first
        .retroarch_launch
        .as_ref()
        .unwrap()
        .player_topology
        .is_some()
    {
        let content = plan
            .retroarch_content
            .as_ref()
            .context("Option-dependent controller topology requires prepared content identity")?;
        crate::controller_launch_modes::validate_arguments(arguments, content)?;
        Some(content)
    } else {
        beetle_content
    };
    // Read once: the exact effective options determine both topology and the
    // snapshot delivered to the core; per-game settings are not flattened away.
    let baseline_options = if let Some(content) = option_content {
        effective_mode_core_options(&base, &config, &overrides, library, &content.content)?
    } else {
        read_core_options(&base, &config)?
    };
    let (active_ports, snapshot_profile) = topology_snapshot(first, &baseline_options)?;
    let mut modes = crate::controller_launch_modes::configured_modes(&config, arguments, ports)?;
    modes.truncate(active_ports);
    let binding_modes = if let Some(content) = beetle_content {
        crate::controller_psx::launch_binding_modes(
            content,
            &option.core_name,
            &modes,
            &baseline_options,
        )?
    } else {
        modes.clone()
    };
    let players = mode_players(
        settings,
        &option.core_name,
        platform,
        &binding_modes,
        devices,
    )?;
    let cache = directories::BaseDirs::new()
        .context("Finding controller launch cache")?
        .cache_dir()
        .join("lunchbox/controller-launch");
    std::fs::create_dir_all(&cache)?;
    let directory = tempfile::Builder::new()
        .prefix("session-")
        .tempdir_in(cache)?;
    let mut output = String::from(
        "# Lunchbox per-launch physical calibration; original configuration is unchanged.\ninput_joypad_driver = \"linuxraw\"\ninput_autodetect_enable = \"false\"\nauto_remaps_enable = \"false\"\nauto_overrides_enable = \"false\"\nconfig_save_on_exit = \"false\"\nremap_save_on_exit = \"false\"\n",
    );
    let highest_port = players.iter().map(|(port, _, _)| *port).max().unwrap();
    // Disabled gaps must not inherit a joystick already assigned to another port.
    for port in 1..=ports {
        if !players.iter().any(|(assigned, _, _)| *assigned == port) {
            for (_, control) in OUTPUTS {
                for suffix in ["btn", "axis"] {
                    output.push_str(&format!(
                        "input_player{port}_{control}_{suffix} = \"nul\"\n"
                    ));
                }
            }
            output.push_str(&format!("input_libretro_device_p{port} = \"0\"\n"));
        }
    }
    let mut transports = Vec::new();
    for (port, profile, device) in &players {
        let calibration = &settings.controller_mapping.calibrations[&device.stable_id];
        let profile = profile.for_port(*port);
        let transport = prepare_player_transport(calibration, &profile, device, cancel)?;
        output.push_str(&player_config_transport(
            calibration,
            &profile,
            &transport.numbering,
            *port,
            modes[*port - 1],
            &transport.pressure_codes,
        )?);
        transports.push(transport);
    }
    output.push_str(&format!("input_max_users = \"{highest_port}\"\n"));
    output.push_str(&write_core_options_snapshot(
        &snapshot_profile,
        &baseline_options,
        directory.path(),
    )?);
    crate::controller_launch_modes::validate_generated_modes(&output, arguments, ports)?;
    let path = directory.path().join("controllers.cfg");
    std::fs::write(&path, output)?;
    check_preparation_cancel(cancel)?;
    attach_config(plan, &option.executable, &path)?;
    Ok(Some(CalibratedLaunch {
        _directory: Some(directory),
        bizhawk: None,
        #[cfg(target_os = "linux")]
        bizhawk_topology: None,
        transports,
        crocods: None,
        #[cfg(target_os = "linux")]
        ppsspp: None,
        #[cfg(target_os = "linux")]
        mgba: None,
        #[cfg(target_os = "linux")]
        dolphin_native: None,
        #[cfg(target_os = "linux")]
        snes9x_native: None,
        #[cfg(target_os = "linux")]
        fceux_native: None,
        #[cfg(target_os = "linux")]
        sameboy_native: None,
        bsnes_native: None,
        #[cfg(target_os = "linux")]
        mednafen_native: None,
        #[cfg(target_os = "linux")]
        mame_native: None,
        #[cfg(target_os = "linux")]
        flycast_native: None,
        #[cfg(target_os = "linux")]
        pcsx2_native: None,
        #[cfg(target_os = "linux")]
        rpcs3_native: None,
        #[cfg(target_os = "linux")]
        melonds_native: None,
        #[cfg(target_os = "linux")]
        duckstation: None,
        ep128emu: None,
        hatari: None,
        simcp: None,
        steemsse: None,
        scummvm: None,
        dolphin: None,
        same_cdi: None,
        fbneo: None,
        mame: None,
        puae: None,
        #[cfg(target_os = "linux")]
        stella: None,
        description: format!(
            "Applied {} calibrated controller(s) using the selected per-port modes · RetroArch automatic overrides/remaps suspended for this launch",
            players.len()
        ),
    }))
}

/// Resolve a declared option-dependent port count without changing the selected
/// model. Freeze that same value into the private options snapshot, including
/// the declared default when the effective file omits it.
fn topology_snapshot(
    profile: &EmulatorProfile,
    baseline: &str,
) -> Result<(usize, EmulatorProfile)> {
    let launch = profile
        .retroarch_launch
        .as_ref()
        .context("Missing launch topology")?;
    let mut snapshot = profile.clone();
    let Some(topology) = &launch.player_topology else {
        return Ok((launch.max_players, snapshot));
    };
    ensure!(
        !baseline
            .lines()
            .any(|line| line.trim_start().starts_with("#include")),
        "Included core-options files require effective player-topology resolution"
    );
    let value = cfg_value(baseline, &topology.option)?.unwrap_or_else(|| topology.default.clone());
    let ports = *topology.values.get(&value).with_context(|| {
        format!(
            "Unverified controller topology for {} = {value}",
            topology.option
        )
    })?;
    snapshot.core_options.insert(topology.option.clone(), value);
    Ok((ports, snapshot))
}

/// Only Beetle needs disc identity to resolve compatibility-forced devices.
/// Other mode-aware cores retain their own argument and option contracts.
fn beetle_launch_content<'a>(
    core: &str,
    arguments: &[OsString],
    prepared: Option<&'a crate::emulator::PreparedRetroarchContent>,
) -> Result<Option<&'a crate::emulator::PreparedRetroarchContent>> {
    if !matches!(core, "mednafen_psx" | "mednafen_psx_hw") {
        return Ok(None);
    }
    let content = prepared.context("Missing prepared PlayStation content identity")?;
    crate::controller_psx::validate_arguments(arguments, content)?;
    Ok(Some(content))
}

fn mode_players<'a>(
    settings: &AppSettings,
    core: &str,
    platform: &str,
    modes: &[u32],
    devices: &[&'a ControllerDevice],
) -> Result<Vec<(usize, &'static EmulatorProfile, &'a ControllerDevice)>> {
    let mut targets = Vec::new();
    for (index, mode) in modes.iter().copied().enumerate() {
        if mode == 0 {
            continue;
        }
        let profile = catalog()
            .launch_mode(core, platform, mode)
            .with_context(|| {
                format!(
                    "No calibrated contract for {core} controller mode {mode} on port {}",
                    index + 1
                )
            })?;
        ensure!(
            index < profile.retroarch_launch.as_ref().unwrap().max_players,
            "Controller port exceeds the verified mode"
        );
        targets.push((index + 1, profile));
    }
    assign_players(settings, &targets, devices)
}

fn assign_players<'a>(
    settings: &AppSettings,
    targets: &[(usize, &'static EmulatorProfile)],
    devices: &[&'a ControllerDevice],
) -> Result<Vec<(usize, &'static EmulatorProfile, &'a ControllerDevice)>> {
    let candidates = targets
        .iter()
        .map(|(port, profile)| {
            let profile = profile.for_port(*port);
            devices
                .iter()
                .enumerate()
                .filter_map(|(index, device)| {
                    settings
                        .controller_mapping
                        .calibrations
                        .get(&device.stable_id)
                        .is_some_and(|calibration| compatible(calibration, &profile))
                        .then_some(index)
                })
                .collect::<Vec<_>>()
        })
        .collect::<Vec<_>>();
    // Augmenting paths maximize filled ports without consuming a uniquely
    // capable pad on a port that another connected controller could serve.
    fn assign(
        port: usize,
        candidates: &[Vec<usize>],
        owners: &mut [Option<usize>],
        seen: &mut [bool],
    ) -> bool {
        // Preserve earlier ports' preferences whenever an unused compatible
        // controller is available. Reassign only to fill an otherwise empty
        // port (for example digital first, then DualShock-only second).
        for &device in &candidates[port] {
            if !seen[device] && owners[device].is_none() {
                owners[device] = Some(port);
                return true;
            }
        }
        for &device in &candidates[port] {
            if seen[device] {
                continue;
            }
            seen[device] = true;
            if owners[device].is_none_or(|other| assign(other, candidates, owners, seen)) {
                owners[device] = Some(port);
                return true;
            }
        }
        false
    }
    let mut owners = vec![None; devices.len()];
    for (index, (port, profile)) in targets.iter().enumerate() {
        if !assign(
            index,
            &candidates,
            &mut owners,
            &mut vec![false; devices.len()],
        ) && *port == 1
        {
            bail!(
                "No connected calibration supplies all controls for {}. Complete calibration or choose a compatible controller; the emulated mode was not changed",
                profile.name
            );
        }
    }
    let mut players = owners
        .iter()
        .enumerate()
        .filter_map(|(device, owner)| {
            owner.map(|target| (targets[target].0, targets[target].1, devices[device]))
        })
        .collect::<Vec<_>>();
    players.sort_by_key(|(port, _, _)| *port);
    ensure!(
        !players.is_empty(),
        "No enabled console port has a compatible calibrated controller"
    );
    Ok(players)
}

fn attach_config(
    plan: &mut LaunchPlan,
    executable: &EmulatorExecutable,
    path: &Path,
) -> Result<()> {
    match executable {
        EmulatorExecutable::Flatpak { app_id, .. } => {
            let boundary = plan
                .arguments
                .iter()
                .position(|arg| arg.to_str() == Some(app_id))
                .context("Missing Flatpak app boundary")?;
            let mut app_arguments = plan.arguments[boundary + 1..].to_vec();
            append_argument(&mut app_arguments, path)?;
            let parent = path.parent().context("Missing config directory")?;
            let mut access = OsString::from("--filesystem=");
            access.push(parent);
            // Permission applies only to this process and this private directory.
            plan.arguments.truncate(boundary + 1);
            plan.arguments.insert(boundary, access);
            plan.arguments.extend(app_arguments);
            Ok(())
        }
        EmulatorExecutable::Native(_) => append_argument(&mut plan.arguments, path),
        _ => bail!("Unsupported controller adapter transport"),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::controller_catalog::InputBinding;

    pub(super) fn calibrated_layout(layout: &str) -> (Calibration, JoydevMap) {
        let mut buttons = Vec::new();
        let mut axes = vec![3u8, 2, 1, 0];
        let mut bindings = BTreeMap::new();
        for control in &catalog().layout(layout).unwrap().controls {
            let native = if control.analog {
                let (axis, direction) = match control.id.as_str() {
                    "stick_up" | "left_stick_up" => (1u32, -1),
                    "stick_down" | "left_stick_down" => (1, 1),
                    "stick_left" | "left_stick_left" => (0, -1),
                    "stick_right" | "left_stick_right" => (0, 1),
                    "right_stick_up" => (3, -1),
                    "right_stick_down" => (3, 1),
                    "right_stick_left" => (2, -1),
                    "right_stick_right" => (2, 1),
                    // Analog triggers and pressure-sensitive buttons each
                    // measure a dedicated axis with a positive press gesture.
                    _ => {
                        let axis = axes.len() as u32;
                        axes.push(axis as u8);
                        (axis, 1)
                    }
                };
                NativeInput {
                    code: 0x30000 + axis,
                    direction,
                }
            } else {
                let code = 288 + buttons.len() as u16;
                buttons.push(code);
                NativeInput {
                    code: 0x10000 + u32::from(code),
                    direction: 0,
                }
            };
            bindings.insert(
                control.id.clone(),
                InputBinding {
                    code: native.code,
                    direction: native.direction,
                    kind: if control.analog { "axis" } else { "button" }.into(),
                    logical: "Driver label is deliberately irrelevant".into(),
                    axis: control
                        .is_pressure()
                        .then(|| crate::controller_axis::AxisMeasurement {
                            minimum: 0,
                            maximum: 255,
                            flat: 0,
                            fuzz: 0,
                            resolution: 0,
                            released: 0,
                            pressed: 255,
                        }),
                    native: Some(native),
                },
            );
        }
        buttons.reverse();
        (
            Calibration {
                target_mappings: Default::default(),
                layout: layout.into(),
                os: "linux".into(),
                backend: "gilrs-0.11".into(),
                bindings,
            },
            JoydevMap {
                index: 3,
                buttons,
                axes,
            },
        )
    }

    #[test]
    #[cfg(target_os = "linux")]
    fn every_launch_contract_checks_cli_against_generated_device_and_empty_ports() {
        for profile in &catalog().emulator_profiles {
            let Some(launch) = &profile.retroarch_launch else {
                continue;
            };
            let (calibration, numbering) = calibrated_layout(&profile.target_layout);
            let mut config = player_config(&calibration, profile, &numbering, 1).unwrap();
            for port in 2..=launch.max_players {
                config.push_str(&format!("input_libretro_device_p{port} = \"0\"\n"));
            }
            let validate = |args: &[OsString]| {
                crate::controller_launch_modes::validate_generated_modes(
                    &config,
                    args,
                    launch.max_players,
                )
            };
            validate(&[]).unwrap();
            validate(&[format!("--device=1:{}", launch.device).into()]).unwrap();
            assert!(
                validate(&["--nodevice=1".into()]).is_err(),
                "{}",
                profile.id
            );
            assert!(validate(&["-vd1:5".into()]).is_err(), "{}", profile.id);
            if launch.max_players > 1 {
                validate(&["--nodevice=2".into()]).unwrap();
                assert!(
                    validate(&[format!("--device=2:{}", launch.device).into()]).is_err(),
                    "{}",
                    profile.id
                );
            }
        }
    }

    #[test]
    #[cfg(target_os = "linux")]
    fn sameboy_preserves_effective_model_and_its_frontend_port_count() {
        let profile = contract("sameboy", "Nintendo Game Boy").unwrap();
        for (model, count) in [
            ("Auto", 1),
            ("Auto (SGB)", 1),
            ("Game Boy", 1),
            ("Game Boy Pocket", 1),
            ("Game Boy Color 0", 1),
            ("Game Boy Color A", 1),
            ("Game Boy Color B", 1),
            ("Game Boy Color D", 1),
            ("Game Boy Player", 1),
            ("Game Boy Color C", 1),
            ("Game Boy Color", 1),
            ("Game Boy Advance", 1),
            ("Super Game Boy", 4),
            ("Super Game Boy PAL", 4),
            ("Super Game Boy 2", 4),
        ] {
            let baseline =
                format!("sameboy_model = \"{model}\"\nsameboy_mono_palette = \"olive\"\n");
            let (ports, snapshot) = topology_snapshot(profile, &baseline).unwrap();
            assert_eq!(ports, count, "{model}");
            assert_eq!(snapshot.core_options["sameboy_model"], model);
            let dir = tempfile::tempdir().unwrap();
            let config = write_core_options_snapshot(&snapshot, &baseline, dir.path()).unwrap();
            let output = std::fs::read_to_string(dir.path().join("core-options.cfg")).unwrap();
            assert_eq!(
                cfg_value(&output, "sameboy_model").unwrap().as_deref(),
                Some(model)
            );
            assert!(output.contains("sameboy_mono_palette = \"olive\""));
            assert!(config.contains("game_specific_options = \"false\""));
            assert!(config.contains("rgui_config_directory"));
            let modes = crate::controller_launch_modes::configured_modes("", &[], ports).unwrap();
            assert_eq!(modes, vec![1; count]);
        }
        let (ports, snapshot) = topology_snapshot(profile, "").unwrap();
        assert_eq!(ports, 1);
        assert_eq!(snapshot.core_options["sameboy_model"], "Auto");
        for bad in [
            "sameboy_model = \"Future model\"",
            "sameboy_model = \"\"",
            "sameboy_model = \"Auto\"\nsameboy_model = \"Super Game Boy\"",
            "sameboy_model=Auto",
        ] {
            assert!(topology_snapshot(profile, bad).is_err(), "{bad}");
        }
        assert!(topology_snapshot(profile, "#include \"hidden.opt\"").is_err());
        assert!(contract("sameboy", "Nintendo Game Boy Advance").is_none());
        assert!(contract("sameboy", "Super Nintendo Entertainment System").is_none());
    }

    #[test]
    #[cfg(target_os = "linux")]
    fn sameboy_topology_uses_one_effective_options_file_not_merged_defaults() {
        let root = tempfile::tempdir().unwrap();
        let base = root.path();
        let configs = base.join("config");
        let core_dir = configs.join("SameBoy");
        std::fs::create_dir_all(&core_dir).unwrap();
        let profile = contract("sameboy", "Game Boy").unwrap();
        let content = base.join("roms/Game.gb");
        let global = base.join("retroarch-core-options.cfg");
        std::fs::write(&global, "sameboy_model = \"Super Game Boy 2\"\n").unwrap();
        let resolve = || {
            let options =
                effective_mode_core_options(base, "", &configs, "SameBoy", &content).unwrap();
            topology_snapshot(profile, &options).unwrap()
        };
        assert_eq!(resolve().0, 4);
        std::fs::write(
            core_dir.join("SameBoy.opt"),
            "sameboy_model = \"Game Boy\"\n",
        )
        .unwrap();
        assert_eq!(resolve().0, 1);
        std::fs::write(
            core_dir.join("roms.opt"),
            "sameboy_model = \"Super Game Boy PAL\"\n",
        )
        .unwrap();
        assert_eq!(resolve().0, 4);
        // A game file replaces the folder/core/global file. Its absent model
        // means the core's Auto default, not the lower-priority four-port value.
        std::fs::write(
            core_dir.join("Game.opt"),
            "sameboy_mono_palette = \"olive\"\n",
        )
        .unwrap();
        let (ports, snapshot) = resolve();
        assert_eq!(ports, 1);
        assert_eq!(snapshot.core_options["sameboy_model"], "Auto");
        assert_eq!(
            std::fs::read_to_string(&global).unwrap(),
            "sameboy_model = \"Super Game Boy 2\"\n"
        );
        let identity = crate::emulator::PreparedRetroarchContent {
            core: "/cores/sameboy_libretro.so".into(),
            content,
        };
        let args = vec![
            "-L".into(),
            identity.core.clone().into_os_string(),
            identity.content.clone().into_os_string(),
        ];
        crate::controller_launch_modes::validate_arguments(&args, &identity).unwrap();
        for prefix in [
            "--subsystem=gb_link_2p",
            "--appendconfig=hidden.cfg",
            "--config=hidden.cfg",
        ] {
            let mut altered = vec![prefix.into()];
            altered.extend(args.clone());
            assert!(
                crate::controller_launch_modes::validate_arguments(&altered, &identity).is_err()
            );
        }
        let mut changed = args;
        *changed.last_mut().unwrap() = "/other/Game.gb".into();
        assert!(crate::controller_launch_modes::validate_arguments(&changed, &identity).is_err());
    }

    #[test]
    #[cfg(target_os = "linux")]
    fn sameboy_composes_brawler_inputs_and_assigns_only_model_ports() {
        let (calibration, numbering) = calibrated_layout("brawler64");
        for mode in [1, 257] {
            let profile = catalog()
                .launch_mode("sameboy", "Game Boy Color", mode)
                .unwrap();
            let config = player_config(&calibration, profile, &numbering, 1).unwrap();
            for (target, output) in [
                ("up", "up"),
                ("down", "down"),
                ("left", "left"),
                ("right", "right"),
                ("a", "a"),
                ("b", "b"),
                ("start", "start"),
                ("select", "select"),
            ] {
                let (_, button) = numbering
                    .binding(calibration.bindings[target].native.as_ref().unwrap())
                    .unwrap();
                assert_eq!(
                    cfg_value(&config, &format!("input_player1_{output}_btn")).unwrap(),
                    Some(button)
                );
            }
            assert_eq!(
                cfg_value(&config, "input_libretro_device_p1").unwrap(),
                Some(mode.to_string())
            );
            for unused in ["x", "y", "l", "r", "l2", "r2", "l3", "r3"] {
                assert_eq!(
                    cfg_value(&config, &format!("input_player1_{unused}_btn"))
                        .unwrap()
                        .as_deref(),
                    Some("nul")
                );
            }
        }
        let devices = (0..5)
            .map(|index| ControllerDevice {
                stable_id: format!("pad{index}"),
                name: "Identical name".into(),
                device_path: format!("/dev/input/js{index}").into(),
                event_paths: vec![],
                vendor_id: None,
                product_id: None,
                version: None,
                bus_type: None,
                physical_path: None,
                unique_id: None,
                is_virtual: false,
            })
            .collect::<Vec<_>>();
        let mut settings = AppSettings::default();
        for device in &devices {
            settings
                .controller_mapping
                .calibrations
                .insert(device.stable_id.clone(), calibration.clone());
        }
        let order = devices.iter().rev().collect::<Vec<_>>();
        for (model, count) in [("Auto", 1), ("Super Game Boy 2", 4)] {
            let profile = contract("sameboy", "Game Boy").unwrap();
            let (ports, _) =
                topology_snapshot(profile, &format!("sameboy_model = \"{model}\"")).unwrap();
            let modes = crate::controller_launch_modes::configured_modes(
                "input_libretro_device_p1 = 257",
                &[],
                ports,
            )
            .unwrap();
            let players = mode_players(&settings, "sameboy", "Game Boy", &modes, &order).unwrap();
            assert_eq!(players.len(), count);
            for (index, (port, _, device)) in players.iter().enumerate() {
                assert_eq!(*port, index + 1);
                assert_eq!(device.stable_id, order[index].stable_id);
            }
        }
        let players =
            mode_players(&settings, "sameboy", "Game Boy", &[257, 0, 1, 257], &order).unwrap();
        assert_eq!(players.iter().map(|p| p.0).collect::<Vec<_>>(), [1, 3, 4]);
        assert!(mode_players(&settings, "sameboy", "Game Boy", &[5], &order).is_err());
        assert!(
            crate::controller_launch_modes::configured_modes("", &["--device=2:257".into()], 1)
                .is_err()
        );
    }

    #[test]
    #[cfg(target_os = "linux")]
    fn swanstation_uses_mode_specific_physical_bindings_not_printed_letters() {
        for (mode, layout) in [
            (1, "brawler64"),
            (1, "dualshock"),
            (261, "dualshock"),
            (261, "xbox"),
        ] {
            let profile = catalog()
                .launch_mode("swanstation", "Sony Playstation", mode)
                .unwrap();
            let (calibration, numbering) = calibrated_layout(layout);
            let config = player_config(&calibration, profile, &numbering, 1).unwrap();
            assert!(compatible(&calibration, profile));
            assert!(config.contains(&format!("input_libretro_device_p1 = \"{mode}\"")));
            let physical = if layout == "brawler64" { "a" } else { "b" };
            let (_, cross) = numbering
                .binding(calibration.bindings[physical].native.as_ref().unwrap())
                .unwrap();
            assert!(config.contains(&format!("input_player1_b_btn = \"{cross}\"")));
            if mode == 261 {
                for (control, output) in [
                    ("stick_up", "l_y_minus"),
                    ("stick_right", "l_x_plus"),
                    ("right_stick_down", "r_y_plus"),
                    ("right_stick_left", "r_x_minus"),
                ] {
                    let (_, axis) = numbering
                        .binding(calibration.bindings[control].native.as_ref().unwrap())
                        .unwrap();
                    assert!(config.contains(&format!("input_player1_{output}_axis = \"{axis}\"")));
                    assert!(config.contains(&format!("input_player1_{output}_btn = \"nul\"")));
                }
            }
        }
        let analog = catalog().launch_mode("swanstation", "PSX", 261).unwrap();
        assert!(!compatible(&calibrated_layout("brawler64").0, analog));
        assert!(!compatible(&calibrated_layout("horizontal-four").0, analog));
        let mut missing_click = calibrated_layout("dualshock").0;
        missing_click.bindings.remove("l3");
        assert!(!compatible(&missing_click, analog));
        assert!(catalog().launch_mode("swanstation", "PSX", 517).is_none());
    }

    #[test]
    #[cfg(target_os = "linux")]
    fn beetle_keeps_requested_devices_separate_from_effective_bindings() {
        for core in ["mednafen_psx", "mednafen_psx_hw"] {
            let digital = catalog().launch_mode(core, "PSX", 1).unwrap();
            let analog = catalog().launch_mode(core, "PSX", 517).unwrap();
            let (brawler, numbering) = calibrated_layout("brawler64");
            let config = player_config_requested(&brawler, digital, &numbering, 1, 517).unwrap();
            assert_eq!(
                cfg_value(&config, "input_libretro_device_p1")
                    .unwrap()
                    .as_deref(),
                Some("517")
            );
            assert_eq!(
                cfg_value(&config, "input_player1_l_x_plus_axis")
                    .unwrap()
                    .as_deref(),
                Some("nul")
            );
            let (_, cross) = numbering
                .binding(brawler.bindings["a"].native.as_ref().unwrap())
                .unwrap();
            assert_eq!(
                cfg_value(&config, "input_player1_b_btn").unwrap(),
                Some(cross)
            );
            assert!(!compatible(&brawler, analog));
            let (dualshock, numbering) = calibrated_layout("dualshock");
            let config = player_config_requested(&dualshock, analog, &numbering, 1, 1).unwrap();
            assert_eq!(
                cfg_value(&config, "input_libretro_device_p1")
                    .unwrap()
                    .as_deref(),
                Some("1")
            );
            assert_ne!(
                cfg_value(&config, "input_player1_r_x_plus_axis")
                    .unwrap()
                    .as_deref(),
                Some("nul")
            );
            assert!(player_config_requested(&dualshock, analog, &numbering, 1, 261).is_err());
            let dir = tempfile::tempdir().unwrap();
            let prefix = if core.ends_with("_hw") {
                "beetle_psx_hw"
            } else {
                "beetle_psx"
            };
            let baseline = format!(
                "{prefix}_compatibility_settings = \"enabled\"\n{prefix}_analog_toggle = \"enabled\"\n"
            );
            write_core_options_snapshot(digital, &baseline, dir.path()).unwrap();
            let copy = std::fs::read_to_string(dir.path().join("core-options.cfg")).unwrap();
            assert!(copy.starts_with(&baseline));
            for port in [1, 2] {
                assert_eq!(
                    cfg_value(&copy, &format!("{prefix}_enable_multitap_port{port}"))
                        .unwrap()
                        .as_deref(),
                    Some("disabled")
                );
            }
        }
    }

    #[test]
    fn beetle_content_validation_does_not_restrict_swanstation_arguments() {
        let arguments = ["--sram-mode", "noload-nosave"].map(OsString::from);
        assert!(
            beetle_launch_content("swanstation", &arguments, None)
                .unwrap()
                .is_none()
        );
        let prepared = crate::emulator::PreparedRetroarchContent {
            core: "/cores/mednafen_psx_libretro.so".into(),
            content: "/games/title.cue".into(),
        };
        assert!(
            beetle_launch_content("swanstation", &arguments, Some(&prepared))
                .unwrap()
                .is_none()
        );
        for core in ["mednafen_psx", "mednafen_psx_hw"] {
            assert!(beetle_launch_content(core, &arguments, None).is_err());
            assert!(beetle_launch_content(core, &arguments, Some(&prepared)).is_err());
            let valid = [
                OsString::from("-L"),
                prepared.core.clone().into_os_string(),
                prepared.content.clone().into_os_string(),
            ];
            assert!(
                beetle_launch_content(core, &valid, Some(&prepared))
                    .unwrap()
                    .is_some()
            );
        }
    }

    #[test]
    fn compact_option_syntax_cannot_change_effective_file_selection() {
        for key in ["game_specific_options", "global_core_options"] {
            for value in ["true", "false", "\"true\"", "\"false\""] {
                assert!(config_bool(&format!("{key}={value}"), key, true).is_err());
                assert!(config_bool(&format!("{key}={value}"), key, false).is_err());
            }
        }
        for key in [
            "core_options_path",
            "rgui_config_directory",
            "input_remapping_directory",
        ] {
            assert!(cfg_value(&format!("{key}=/custom/path"), key).is_err());
        }
    }

    #[test]
    fn path_settings_accept_quoted_and_unquoted_values_without_silent_fallback() {
        for key in [
            "core_options_path",
            "rgui_config_directory",
            "input_remapping_directory",
        ] {
            for suffix in [
                "/custom/path",
                "\"/custom/path\"",
                "/custom/path # note",
                "\"/custom/path\" # note",
            ] {
                assert_eq!(
                    cfg_value(&format!("{key} = {suffix}"), key)
                        .unwrap()
                        .as_deref(),
                    Some("/custom/path")
                );
            }
            assert_eq!(
                cfg_value(&format!("{key} = \"/path with # hash\""), key)
                    .unwrap()
                    .as_deref(),
                Some("/path with # hash")
            );
            for value in [
                "",
                "# no value",
                "\"unterminated",
                "\"/custom/path\" extra",
                "/custom/path extra",
            ] {
                assert!(cfg_value(&format!("{key} = {value}"), key).is_err());
            }
            assert!(cfg_value(&format!("{key} = /first\n{key} = /second"), key).is_err());
            assert_eq!(cfg_value("# no setting", key).unwrap(), None);
        }
    }

    #[test]
    fn unquoted_custom_core_options_preserve_compatibility_choice() {
        let temp = tempfile::tempdir().unwrap();
        let custom = temp.path().join("custom.opt");
        let contents = "beetle_psx_compatibility_settings = \"disabled\"\n";
        std::fs::write(&custom, contents).unwrap();
        std::fs::write(
            temp.path().join("retroarch-core-options.cfg"),
            "beetle_psx_compatibility_settings = \"enabled\"\n",
        )
        .unwrap();
        let config = format!(
            "core_options_path = {}\ngame_specific_options = false\nglobal_core_options = true\n",
            custom.display()
        );
        let baseline = effective_mode_core_options(
            temp.path(),
            &config,
            &temp.path().join("config"),
            "Beetle PSX",
            &temp.path().join("games/title.cue"),
        )
        .unwrap();
        assert_eq!(baseline, contents);
        let private = tempfile::tempdir().unwrap();
        let profile = catalog().launch_mode("mednafen_psx", "PSX", 1).unwrap();
        write_core_options_snapshot(profile, &baseline, private.path()).unwrap();
        let written = std::fs::read_to_string(private.path().join("core-options.cfg")).unwrap();
        assert_eq!(
            cfg_value(&written, "beetle_psx_compatibility_settings")
                .unwrap()
                .as_deref(),
            Some("disabled")
        );
        assert_eq!(std::fs::read_to_string(custom).unwrap(), contents);
    }

    #[test]
    fn mode_options_follow_retroarch_file_precedence_without_merging() {
        let temp = tempfile::tempdir().unwrap();
        let base = temp.path();
        let configs = base.join("config");
        let directory = configs.join("Beetle PSX");
        std::fs::create_dir_all(&directory).unwrap();
        let content = base.join("games/title.cue");
        let global = base.join("retroarch-core-options.cfg");
        std::fs::write(&global, "global_only=1\n").unwrap();
        let resolve = |config: &str| {
            effective_mode_core_options(base, config, &configs, "Beetle PSX", &content).unwrap()
        };
        assert_eq!(resolve(""), "global_only=1\n");
        let core = directory.join("Beetle PSX.opt");
        std::fs::write(&core, "beetle_psx_compatibility_settings = disabled\n").unwrap();
        assert_eq!(
            resolve(""),
            "beetle_psx_compatibility_settings = disabled\n"
        );
        assert_eq!(resolve("global_core_options = true"), "global_only=1\n");
        let folder = directory.join("games.opt");
        std::fs::write(&folder, "folder_only=1\n").unwrap();
        assert_eq!(resolve("global_core_options = true"), "folder_only=1\n");
        let game = directory.join("title.opt");
        std::fs::write(&game, "game_only=1\n").unwrap();
        assert_eq!(resolve(""), "game_only=1\n");
        assert_eq!(
            resolve("game_specific_options = false"),
            "beetle_psx_compatibility_settings = disabled\n"
        );
        assert_eq!(
            resolve("game_specific_options = false\nglobal_core_options = true"),
            "global_only=1\n"
        );
        assert_eq!(std::fs::read_to_string(&game).unwrap(), "game_only=1\n");
        assert_eq!(std::fs::read_to_string(&global).unwrap(), "global_only=1\n");
        assert!(
            effective_mode_core_options(
                base,
                "game_specific_options = maybe",
                &configs,
                "Beetle PSX",
                &content
            )
            .is_err()
        );
    }

    #[test]
    #[cfg(unix)]
    fn mode_override_guard_follows_symlinked_files_and_directories() {
        use std::os::unix::fs::symlink;
        let temp = tempfile::tempdir().unwrap();
        let directory = temp.path().join("Beetle PSX");
        std::fs::create_dir(&directory).unwrap();
        let external = temp.path().join("managed");
        std::fs::create_dir(&external).unwrap();
        let options = external.join("core.opt");
        std::fs::write(&options, "beetle_psx_compatibility_settings = disabled").unwrap();
        symlink(&options, directory.join("Beetle PSX.opt")).unwrap();
        ensure_no_mode_overrides(&directory, "Beetle PSX").unwrap();
        let remap = external.join("game.rmp");
        std::fs::write(&remap, "input_libretro_device_p1=517").unwrap();
        let link = directory.join("game.rmp");
        symlink(&remap, &link).unwrap();
        assert!(ensure_no_mode_overrides(&directory, "Beetle PSX").is_err());
        std::fs::remove_file(&link).unwrap();
        symlink(&external, directory.join("nested")).unwrap();
        assert!(ensure_no_mode_overrides(&directory, "Beetle PSX").is_err());
        std::fs::remove_file(&remap).unwrap();
        ensure_no_mode_overrides(&directory, "Beetle PSX").unwrap();
        symlink(&directory, external.join("loop")).unwrap();
        assert!(ensure_no_mode_overrides(&directory, "Beetle PSX").is_err());
    }

    #[test]
    #[cfg(target_os = "linux")]
    fn gamegear_uses_two_buttons_and_start_without_requiring_select() {
        let profile = contract("genesis_plus_gx", "Sega Game Gear").unwrap();
        assert_eq!(profile.bindings.len(), 7);
        assert_eq!(profile.target_layout, "gamegear");
        assert_eq!(
            profile.core_options["genesis_plus_gx_system_hw"],
            "game gear"
        );
        for (layout, one, two) in [
            ("gamegear", "b", "a"),
            ("nes", "b", "a"),
            ("horizontal-four", "b", "a"),
            ("n30-turbo", "b", "a"),
            ("brawler64", "b", "a"),
            ("xbox", "y", "b"),
        ] {
            let (mut calibration, numbering) = calibrated_layout(layout);
            calibration.bindings.remove("select");
            assert!(compatible(&calibration, profile), "{layout}");
            let config = player_config(&calibration, profile, &numbering, 1).unwrap();
            assert!(config.contains("input_libretro_device_p1 = \"769\""));
            for (output, physical) in [("b", one), ("a", two), ("start", "start")] {
                let (_, button) = numbering
                    .binding(calibration.bindings[physical].native.as_ref().unwrap())
                    .unwrap();
                assert!(
                    config.contains(&format!("input_player1_{output}_btn = \"{button}\"")),
                    "{layout}/{output}"
                );
            }
            for output in ["select", "x", "y", "l", "r", "l2", "r2"] {
                for suffix in ["btn", "axis"] {
                    assert!(config.contains(&format!("input_player1_{output}_{suffix} = \"nul\"")));
                }
            }
            assert!(player_config(&calibration, profile, &numbering, 2).is_err());
            calibration.bindings.remove("start");
            // Richer layouts can supply Start with a spare gameplay button.
            // Two-button pads have no such spare; reserved D-pad/menu inputs
            // and hardware turbo repeats cannot stand in for independent keys.
            let has_spare = matches!(layout, "horizontal-four" | "brawler64" | "xbox");
            assert_eq!(compatible(&calibration, profile), has_spare, "{layout}");
            if has_spare {
                let config = player_config(&calibration, profile, &numbering, 1).unwrap();
                assert!(!config.contains("input_player1_start_btn = \"nul\""));
            }
        }
    }

    #[test]
    #[cfg(target_os = "linux")]
    fn mgba_distinguishes_gba_shoulders_from_gameboy_and_preserves_thumb_pairs() {
        let gba = catalog()
            .launch_profile("mgba", "Nintendo Game Boy Advance")
            .unwrap();
        let gb = catalog()
            .launch_profile("mgba", "Nintendo Game Boy Color")
            .unwrap();
        assert_eq!(gba.target_layout, "gba");
        assert_eq!(gb.target_layout, "gameboy");
        assert_eq!(gba.bindings.len(), 10);
        assert_eq!(gb.bindings.len(), 8);
        assert_eq!(gba.retroarch_launch.as_ref().unwrap().max_players, 1);
        assert!(catalog().launch_profile("mgba", "Nintendo DS").is_none());
        let (n30, numbering) = calibrated_layout("horizontal-four");
        assert!(compatible(&n30, gb));
        assert!(compatible(&n30, gba));
        let config = player_config(&n30, gba, &numbering, 1).unwrap();
        for (target, physical) in [("l", "y"), ("r", "x")] {
            let (_, button) = numbering
                .binding(n30.bindings[physical].native.as_ref().unwrap())
                .unwrap();
            assert!(config.contains(&format!("input_player1_{target}_btn = \"{button}\"")));
        }
        let (turbo, _) = calibrated_layout("n30-turbo");
        assert!(
            !compatible(&turbo, gba),
            "hardware repeats are not extra independent buttons"
        );
        for (layout, physical_a, physical_b) in [
            ("gba", "a", "b"),
            ("brawler64", "a", "b"),
            ("xbox", "b", "y"),
            ("dualshock", "b", "y"),
        ] {
            let (calibration, numbering) = calibrated_layout(layout);
            assert!(compatible(&calibration, gba));
            let config = player_config(&calibration, gba, &numbering, 1).unwrap();
            for (output, physical) in [("a", physical_a), ("b", physical_b), ("l", "l"), ("r", "r")]
            {
                let (suffix, button) = numbering
                    .binding(calibration.bindings[physical].native.as_ref().unwrap())
                    .unwrap();
                assert_eq!(suffix, "btn");
                assert!(
                    config.contains(&format!("input_player1_{output}_btn = \"{button}\"")),
                    "{layout}: {output}"
                );
            }
            // Do not accidentally inherit turbo or solar-sensor controls.
            for output in ["x", "y", "l2", "r2", "l3", "r3"] {
                assert!(config.contains(&format!("input_player1_{output}_btn = \"nul\"")));
                assert!(config.contains(&format!("input_player1_{output}_axis = \"nul\"")));
            }
        }
    }

    #[test]
    #[cfg(target_os = "linux")]
    fn mixed_console_port_modes_allocate_independent_compatible_controllers() {
        let make_device = |id: &str| ControllerDevice {
            stable_id: id.into(),
            name: "Same USB name".into(),
            device_path: format!("/dev/input/{id}").into(),
            event_paths: vec![],
            vendor_id: None,
            product_id: None,
            version: None,
            bus_type: None,
            physical_path: None,
            unique_id: None,
            is_virtual: false,
        };
        let devices = [make_device("js4"), make_device("js5")];
        let mut settings = AppSettings::default();
        settings
            .controller_mapping
            .calibrations
            .insert("js4".into(), calibrated_layout("brawler64").0);
        settings
            .controller_mapping
            .calibrations
            .insert("js5".into(), calibrated_layout("dualshock").0);
        for order in [[&devices[0], &devices[1]], [&devices[1], &devices[0]]] {
            let players = mode_players(&settings, "swanstation", "PSX", &[1, 1], &order).unwrap();
            assert_eq!(players.len(), 2);
            assert_eq!(players[0].2.stable_id, order[0].stable_id);
            assert_eq!(players[1].2.stable_id, order[1].stable_id);
        }
        let digital_first = mode_players(
            &settings,
            "swanstation",
            "PSX",
            &[1, 261],
            &[&devices[1], &devices[0]],
        )
        .unwrap();
        assert_eq!(digital_first.len(), 2);
        assert_eq!(digital_first[0].2.stable_id, "js4");
        assert_eq!(digital_first[1].2.stable_id, "js5");
        let selected = mode_players(
            &settings,
            "swanstation",
            "PSX",
            &[261, 1],
            &[&devices[0], &devices[1]],
        )
        .unwrap();
        assert_eq!(
            (selected[0].0, selected[0].2.stable_id.as_str()),
            (1, "js5")
        );
        assert_eq!(
            (selected[1].0, selected[1].2.stable_id.as_str()),
            (2, "js4")
        );
        assert!(mode_players(&settings, "swanstation", "PSX", &[261, 1], &[&devices[0]]).is_err());
        let selected =
            mode_players(&settings, "swanstation", "PSX", &[0, 1], &[&devices[0]]).unwrap();
        assert_eq!(selected[0].0, 2);
        assert!(mode_players(&settings, "swanstation", "PSX", &[517, 1], &[&devices[1]]).is_err());
    }

    #[test]
    #[cfg(target_os = "linux")]
    #[ignore = "Real RetroArch Flatpak BIOS-only smoke check; requires explicit trusted core and BIOS directory"]
    fn swanstation_flatpak_mode_startup_oracle() {
        use std::os::unix::process::CommandExt;
        use std::{
            fs,
            process::{Command, Stdio},
            time::{Duration, Instant},
        };
        let core = std::path::PathBuf::from(
            std::env::var_os("LUNCHBOX_TEST_SWANSTATION_CORE").expect("Set trusted core path"),
        )
        .canonicalize()
        .unwrap();
        let bios = std::path::PathBuf::from(
            std::env::var_os("LUNCHBOX_TEST_PSX_BIOS_DIRECTORY").expect("Set BIOS directory"),
        )
        .canonicalize()
        .unwrap();
        assert!(core.is_file() && bios.is_dir());
        let directory = tempfile::Builder::new()
            .prefix("lunchbox-swanstation-mode-oracle-")
            .tempdir_in("/tmp")
            .unwrap()
            .keep();
        for mode in [1, 261] {
            let root = directory.join(mode.to_string());
            fs::create_dir(&root).unwrap();
            for folder in ["saves", "states", "cache", "logs"] {
                fs::create_dir(root.join(folder)).unwrap();
            }
            let profile = catalog().launch_mode("swanstation", "PSX", mode).unwrap();
            let (calibration, numbering) = calibrated_layout("dualshock");
            let mut config = player_config(&calibration, profile, &numbering, 1).unwrap();
            config.push_str("menu_show_start_screen = \"false\"\nmenu_pause_libretro = \"false\"\naudio_enable = \"false\"\nhistory_list_enable = \"false\"\ngame_specific_options = \"false\"\n");
            config.push_str("config_save_on_exit = \"false\"\ninput_autodetect_enable = \"false\"\nauto_remaps_enable = \"false\"\nauto_overrides_enable = \"false\"\ninput_max_users = \"1\"\ninput_joypad_driver = \"linuxraw\"\ninput_driver = \"udev\"\nvideo_driver = \"null\"\naudio_driver = \"null\"\nvideo_vsync = \"false\"\nmenu_enable_widgets = \"false\"\npause_nonactive = \"false\"\npause_on_disconnect = \"false\"\n");
            // A real video driver is necessary for a meaningful rendered-frame
            // limit; a null display can run without advancing that counter.
            config = config.replace("video_driver = \"null\"", "video_driver = \"glcore\"");
            for (key, path) in [
                ("system_directory", bios.clone()),
                ("savefile_directory", root.join("saves")),
                ("savestate_directory", root.join("states")),
                ("cache_directory", root.join("cache")),
                ("log_dir", root.join("logs")),
                ("core_options_path", root.join("options.cfg")),
                ("rgui_config_directory", root.join("config")),
                ("content_history_path", root.join("history.lpl")),
                ("content_favorites_path", root.join("favorites.lpl")),
                ("content_image_history_path", root.join("images.lpl")),
                ("content_music_history_path", root.join("music.lpl")),
                ("content_video_history_path", root.join("videos.lpl")),
            ] {
                let path = path.to_str().unwrap();
                assert!(!path.contains(['"', '\n', '\r']));
                config.push_str(&format!("{key} = \"{path}\"\n"));
            }
            fs::write(root.join("options.cfg"), "swanstation_GPU_Renderer = \"Software\"\nswanstation_ControllerPorts_MultitapMode = \"Disabled\"\n").unwrap();
            fs::write(root.join("retroarch.cfg"), config).unwrap();
            let log = fs::File::create(root.join("startup.log")).unwrap();
            let mut child = Command::new("flatpak")
                .process_group(0)
                .arg("run")
                .arg(format!(
                    "--filesystem={}:ro",
                    core.parent().unwrap().display()
                ))
                .arg(format!("--filesystem={}:ro", bios.display()))
                .arg(format!("--filesystem={}", root.display()))
                .args(["org.libretro.RetroArch", "--verbose", "--config"])
                .arg(root.join("retroarch.cfg"))
                .arg("-L")
                .arg(&core)
                .args(["--max-frames", "8", "--sram-mode", "noload-nosave"])
                .stdout(Stdio::from(log.try_clone().unwrap()))
                .stderr(Stdio::from(log))
                .spawn()
                .unwrap();
            let started = Instant::now();
            let status = loop {
                if let Some(status) = child.try_wait().unwrap() {
                    break status;
                }
                if started.elapsed() > Duration::from_secs(30) {
                    // A Flatpak launcher can exit before its sandbox child.
                    // This test alone owns the fresh process group; never kill
                    // every RetroArch process or a shared terminal job group.
                    unsafe {
                        libc::kill(-(child.id() as i32), libc::SIGTERM);
                    }
                    std::thread::sleep(Duration::from_millis(200));
                    unsafe {
                        libc::kill(-(child.id() as i32), libc::SIGKILL);
                    }
                    child.wait().unwrap();
                    panic!("Core startup timed out; logs at {}", root.display());
                }
                std::thread::sleep(Duration::from_millis(20));
            };
            let text = fs::read_to_string(root.join("startup.log")).unwrap();
            assert!(
                status.success(),
                "Core startup failed; logs at {}",
                root.display()
            );
            assert!(
                text.contains("SwanStation"),
                "Wrong core; logs at {}",
                root.display()
            );
            println!(
                "SWANSTATION_MODE_ORACLE mode={mode} log={}",
                root.join("startup.log").display()
            );
        }
    }
    #[test]
    #[cfg(target_os = "linux")]
    fn measured_event_identity_never_falls_back_to_first_event_or_same_model() {
        use std::os::unix::fs::symlink;
        let directory = tempfile::tempdir().unwrap();
        let sys = directory.path();
        for name in ["pad-a", "pad-b", "js4", "event8", "event9", "event10"] {
            std::fs::create_dir(sys.join(name)).unwrap();
        }
        for (node, physical) in [
            ("js4", "pad-a"),
            ("event8", "pad-b"),
            ("event9", "pad-a"),
            ("event10", "pad-a"),
        ] {
            symlink(sys.join(physical), sys.join(node).join("device")).unwrap();
        }
        let joystick = Path::new("/dev/input/js4");
        let events = vec!["/dev/input/event8".into(), "/dev/input/event9".into()];
        assert_eq!(
            measured_event_path(joystick, &events, sys).unwrap().0,
            Path::new("/dev/input/event9")
        );
        assert!(measured_event_path(joystick, &events[..1], sys).is_err());
        let mut ambiguous = events;
        ambiguous.push("/dev/input/event10".into());
        assert!(measured_event_path(joystick, &ambiguous, sys).is_err());
        assert!(measured_event_path(Path::new("/tmp/js4"), &ambiguous, sys).is_err());
    }

    #[test]
    #[cfg(target_os = "linux")]
    fn snes_cores_compose_physical_inputs_through_retropad() {
        for core in ["snes9x", "bsnes"] {
            let profile = contract(core, "Nintendo - Super Nintendo Entertainment System").unwrap();
            for layout in ["snes", "xbox", "dualshock", "brawler64"] {
                let mut cal = nes();
                cal.layout = layout.into();
                cal.bindings = catalog()
                    .layout(layout)
                    .unwrap()
                    .controls
                    .iter()
                    .filter(|control| !control.analog)
                    .enumerate()
                    .map(|(index, control)| {
                        (
                            control.id.clone(),
                            InputBinding {
                                code: index as u32,
                                kind: "button".into(),
                                direction: 0,
                                logical: control.label.clone(),
                                axis: None,
                                native: Some(NativeInput {
                                    code: 0x10120 + index as u32,
                                    direction: 0,
                                }),
                            },
                        )
                    })
                    .collect();
                let map = JoydevMap {
                    index: 3,
                    buttons: (288..288 + cal.bindings.len() as u16).rev().collect(),
                    axes: vec![],
                };
                let config = player_config(&cal, profile, &map, 1).unwrap();
                assert!(compatible(&cal, profile), "{core}/{layout}");
                let face = if layout == "brawler64" {
                    [("a", "c_down"), ("b", "a"), ("x", "c_left"), ("y", "b")]
                } else {
                    [("a", "a"), ("b", "b"), ("x", "x"), ("y", "y")]
                };
                for (output, physical) in face.into_iter().chain([
                    ("l", "l"),
                    ("r", "r"),
                    ("start", "start"),
                    ("select", "select"),
                ]) {
                    let (_, number) = map
                        .binding(cal.bindings[physical].native.as_ref().unwrap())
                        .unwrap();
                    assert!(
                        config.contains(&format!("input_player1_{output}_btn = \"{number}\"")),
                        "{core}/{layout}/{output}"
                    );
                }
                assert!(config.contains("input_libretro_device_p1 = \"1\""));
                assert!(player_config(&cal, profile, &map, 3).is_err());
            }
            // Four face buttons alone do not supply the required SNES shoulders.
            let mut n30 = nes();
            n30.layout = "horizontal-four".into();
            assert!(!compatible(&n30, profile));
        }
    }

    #[test]
    fn launch_opt_in_and_port_count_come_from_the_catalog() {
        for profile in &catalog().emulator_profiles {
            assert_eq!(
                supports_profile(profile),
                (cfg!(target_os = "linux") && profile.retroarch_launch.is_some())
                    || profile.transport == "ares-settings"
            );
            if let Some(launch) = &profile.retroarch_launch {
                // Explicit-selection profiles are deliberately unreachable as a
                // default launch mode; they are chosen in the guided target UI.
                for alias in &launch.platforms {
                    if !profile.explicit_selection {
                        assert_eq!(
                            catalog()
                                .launch_mode(&profile.core, alias, launch.device)
                                .unwrap()
                                .id,
                            profile.id
                        );
                    }
                }
            }
        }
        let preview = catalog()
            .emulator_profiles
            .iter()
            .find(|p| p.id == "retroarch:genesis_plus_gx:md3")
            .unwrap();
        assert!(!supports_profile(preview));
        assert!(restrict_players(&mut vec![], preview).is_err());
    }
    #[test]
    #[cfg(target_os = "linux")]
    fn horizontal_and_turbo_pads_compose_pce_gameplay_without_upper_button_aliases() {
        let profile = contract("mednafen_pce_fast", "NEC TurboGrafx-16").unwrap();
        for layout in ["horizontal-four", "n30-turbo", "pce-2"] {
            let mut cal = nes();
            cal.layout = layout.into();
            let map = JoydevMap {
                index: 2,
                buttons: (288..296).rev().collect(),
                axes: vec![],
            };
            let config = player_config(&cal, profile, &map, 1).unwrap();
            for (physical, output) in [("a", "a"), ("b", "b")] {
                let (_, number) = map
                    .binding(cal.bindings[physical].native.as_ref().unwrap())
                    .unwrap();
                assert!(
                    config.contains(&format!("input_player1_{output}_btn = \"{number}\"")),
                    "{layout}"
                );
            }
            for output in ["x", "y", "l", "r", "l2"] {
                assert!(
                    config.contains(&format!("input_player1_{output}_btn = \"nul\"")),
                    "{layout}"
                );
            }
        }
        let options = core_options_overlay(
            "pce_fast_default_joypad_type_p1 = \"6 Buttons\"\npce_fast_cdspeed = \"4\"\n",
            &profile.core_options,
        )
        .unwrap();
        assert!(!options.contains("6 Buttons"));
        assert!(options.contains("pce_fast_cdspeed = \"4\""));
        for player in 1..=5 {
            assert!(options.contains(&format!(
                "pce_fast_default_joypad_type_p{player} = \"2 Buttons\""
            )));
        }
    }
    #[test]
    fn n64_independent_buttons_and_core_options_are_composed_together() {
        let profile = contract("mupen64plus_next", "Nintendo 64").unwrap();
        assert_eq!(profile.bindings["c_left"], "LeftBumper");
        assert_eq!(profile.bindings["c_right"], "RightBumper");
        assert_eq!(profile.bindings["l"], "Select");
        assert_eq!(profile.bindings["r"], "RightTrigger");
        let baseline = "mupen64plus-alt-map = \"False\"\nmupen64plus-pak1 = \"rumble\"\n";
        let options = core_options_overlay(baseline, &profile.core_options).unwrap();
        assert_eq!(options.matches("mupen64plus-alt-map").count(), 1);
        assert!(options.contains("mupen64plus-alt-map = \"True\""));
        assert!(options.contains("mupen64plus-pak1 = \"rumble\""));
        assert!(core_options_overlay("#include \"other.cfg\"", &profile.core_options).is_err());
    }
    #[test]
    fn flatpak_config_is_scoped_and_inserted_inside_app_arguments() {
        let executable = EmulatorExecutable::Flatpak {
            command: "flatpak".into(),
            app_id: "org.libretro.RetroArch".into(),
        };
        let mut plan = LaunchPlan {
            emulator_name: "RetroArch".into(),
            program: "flatpak".into(),
            arguments: vec![
                "run".into(),
                "--filesystem=/roms:ro".into(),
                "org.libretro.RetroArch".into(),
                "-L".into(),
                "/cores/fceumm_libretro.so".into(),
                "/roms/game.nes".into(),
            ],
            current_directory: "/roms".into(),
            environment: vec![],
            cleanup_paths: vec![],
            retroarch_content: None,
        };
        attach_config(
            &mut plan,
            &executable,
            Path::new("/cache/private session/controllers.cfg"),
        )
        .unwrap();
        assert_eq!(plan.arguments[2], "--filesystem=/cache/private session");
        assert_eq!(plan.arguments[3], "org.libretro.RetroArch");
        assert_eq!(plan.arguments[4], "--appendconfig");
        assert_eq!(plan.arguments[5], "/cache/private session/controllers.cfg");
        assert_eq!(plan.arguments.last().unwrap(), "/roms/game.nes");
    }
    #[test]
    fn selected_controller_order_honors_explicit_order_preference_and_hidden_devices() {
        let device = |id: &str| ControllerDevice {
            stable_id: id.into(),
            name: id.into(),
            device_path: format!("/dev/input/{id}").into(),
            event_paths: vec![],
            vendor_id: None,
            product_id: None,
            version: None,
            bus_type: None,
            physical_path: None,
            unique_id: None,
            is_virtual: false,
        };
        let devices = vec![device("pad-a"), device("pad-b")];
        let mut ordered = vec![&devices[1], &devices[0]];
        assert_eq!(
            restrict_players(
                &mut ordered,
                contract("gambatte", "Nintendo Game Boy").unwrap()
            )
            .unwrap(),
            2
        );
        assert_eq!(ordered.len(), 1);
        assert_eq!(ordered[0].stable_id, "pad-b");
        let mut settings = AppSettings::default();
        for device in &devices {
            settings
                .controller_mapping
                .calibrations
                .insert(device.stable_id.clone(), nes());
        }
        let brawler = settings
            .controller_mapping
            .calibrations
            .get_mut("pad-b")
            .unwrap();
        brawler.layout = "brawler64".into();
        brawler.bindings.retain(|id, _| id == "a" || id == "b");
        brawler.validate().unwrap();
        assert_eq!(
            selected_devices(&settings, &devices, "Nintendo 64")[0].stable_id,
            "pad-b"
        );
        assert_eq!(
            selected_devices(&settings, &devices, "Nintendo Entertainment System")[0].stable_id,
            "pad-a"
        );
        settings
            .controller_mapping
            .preferred_devices
            .insert("n64".into(), "pad-b".into());
        assert_eq!(
            selected_devices(&settings, &devices, "Nintendo 64")[0].stable_id,
            "pad-b"
        );
        settings.controller_mapping.player_mappings.push(
            crate::settings::ControllerPlayerMapping {
                controller_id: Some("pad-a".into()),
                ..Default::default()
            },
        );
        assert_eq!(
            selected_devices(&settings, &devices, "Nintendo 64")[0].stable_id,
            "pad-a"
        );
        settings
            .controller_mapping
            .hidden_controller_ids
            .push("pad-a".into());
        assert_eq!(
            selected_devices(&settings, &devices, "Nintendo 64").len(),
            1
        );
        settings.controller_mapping.hidden_controller_ids.clear();
        settings.controller_mapping.explicit_player_selection = true;
        assert_eq!(
            selected_devices(&settings, &devices, "Nintendo 64").len(),
            1
        );
        assert_eq!(
            selected_devices(&settings, &devices, "Nintendo 64")[0].stable_id,
            "pad-a"
        );
    }
    fn nes() -> Calibration {
        let bindings = catalog()
            .layout("nes")
            .unwrap()
            .controls
            .iter()
            .enumerate()
            .map(|(index, c)| {
                (
                    c.id.clone(),
                    InputBinding {
                        code: 0x10000 + index as u32,
                        kind: "button".into(),
                        direction: 0,
                        logical: c.label.clone(),
                        axis: None,
                        native: Some(NativeInput {
                            code: 0x10120 + index as u32,
                            direction: 0,
                        }),
                    },
                )
            })
            .collect();
        Calibration {
            target_mappings: Default::default(),
            layout: "nes".into(),
            os: "linux".into(),
            backend: "gilrs-0.11".into(),
            bindings,
        }
    }
    #[test]
    fn device_indices_are_not_xbox_button_numbers() {
        let map = JoydevMap {
            index: 5,
            buttons: vec![305, 304],
            axes: vec![0, 1, 16, 17],
        };
        assert_eq!(
            map.binding(&NativeInput {
                code: 0x10130,
                direction: 0
            })
            .unwrap(),
            ("btn", "1".into())
        );
        assert_eq!(
            map.binding(&NativeInput {
                code: 0x30011,
                direction: -1
            })
            .unwrap(),
            ("axis", "-3".into())
        );
        assert!(
            map.binding(&NativeInput {
                code: 0x101ff,
                direction: 0
            })
            .is_err()
        );
    }
    #[test]
    #[cfg(target_os = "linux")]
    fn exact_physical_bindings_clear_inherited_axes_and_reject_legacy_calibration() {
        let mut cal = nes();
        let map = JoydevMap {
            index: 5,
            buttons: (288..296).rev().collect(),
            axes: vec![],
        };
        let profile = contract("fceumm", "Nintendo Entertainment System").unwrap();
        let config = player_config(&cal, profile, &map, 1).unwrap();
        assert!(config.contains("input_player1_b_btn = \"7\""));
        assert!(config.contains("input_player1_b_axis = \"nul\""));
        assert!(config.contains("input_player1_joypad_index = \"5\""));
        cal.bindings.get_mut("b").unwrap().native = None;
        assert!(player_config(&cal, profile, &map, 1).is_err());
        cal.bindings.remove("b");
        assert!(player_config(&cal, profile, &map, 1).is_err());
    }
    #[test]
    fn append_config_preserves_custom_lists_and_argument_boundaries() {
        let mut arguments = vec![
            "--appendconfig".into(),
            "/tmp/user config.cfg".into(),
            "game.nes".into(),
        ];
        append_argument(&mut arguments, Path::new("/tmp/lunchbox config.cfg")).unwrap();
        assert_eq!(
            arguments[1],
            "/tmp/user config.cfg|/tmp/lunchbox config.cfg"
        );
        assert_eq!(arguments[2], "game.nes");
        let mut arguments = vec!["--".into(), "game.nes".into()];
        append_argument(&mut arguments, Path::new("/tmp/controller.cfg")).unwrap();
        assert_eq!(arguments[2], "--");
        for original in [
            vec!["--", "--appendconfig=content.nes"],
            vec!["-c", "--appendconfig=base.cfg", "game.nes"],
            vec!["-L", "--appendconfig", "game.nes"],
        ] {
            let mut arguments = original.iter().map(OsString::from).collect::<Vec<_>>();
            append_argument(&mut arguments, Path::new("/tmp/controller.cfg")).unwrap();
            assert_eq!(
                &arguments[2..],
                &original.iter().map(OsString::from).collect::<Vec<_>>()
            );
        }
        let mut arguments = vec!["--appendconfig=a.cfg|b.cfg".into(), "game.nes".into()];
        append_argument(&mut arguments, Path::new("/tmp/controller.cfg")).unwrap();
        assert_eq!(
            arguments[0],
            "--appendconfig=a.cfg|b.cfg|/tmp/controller.cfg"
        );
    }

    #[test]
    fn duplicate_append_configs_are_rejected_without_mutation() {
        for original in [
            vec!["--appendconfig", "a.cfg", "--appendconfig", "b.cfg"],
            vec!["--appendconfig=a.cfg", "--appendconfig=b.cfg"],
            vec!["--appendconfig=a.cfg", "--appendconfig", "b.cfg"],
            vec!["--appendconfig", "a.cfg", "--appendconfig=b.cfg"],
        ] {
            let mut arguments = original.iter().map(OsString::from).collect::<Vec<_>>();
            let unchanged = arguments.clone();
            assert!(append_argument(&mut arguments, Path::new("/tmp/controller.cfg")).is_err());
            assert_eq!(arguments, unchanged);
            assert!(
                crate::controller_launch_modes::validate_generated_modes("", &arguments, 1)
                    .is_err()
            );
        }
    }
    #[test]
    fn contracts_never_infer_system_from_core_name_alone() {
        assert!(contract("mednafen_pce_fast", "NEC SuperGrafx").is_none());
        assert!(contract("mednafen_pce_fast", "NEC PC-FX").is_none());
        assert!(contract("mednafen_pce", "NEC PC Engine").is_none());
        assert!(contract("mednafen_pce_fast", "NEC TurboGrafx-CD").is_some());
        assert_eq!(
            contract("mednafen_pce_fast", "NEC - PC Engine CD - TurboGrafx-CD")
                .unwrap()
                .target_layout,
            "pce-2"
        );
        assert!(contract("genesis_plus_gx", "Sega Master System").is_none());
        assert!(contract("fceumm", "Super Nintendo Entertainment System").is_none());
        assert!(contract("unknown", "Nintendo Entertainment System").is_none());
        assert_eq!(
            contract("genesis_plus_gx", "Sega Genesis")
                .unwrap()
                .target_layout,
            "genesis-6"
        );
    }
}
