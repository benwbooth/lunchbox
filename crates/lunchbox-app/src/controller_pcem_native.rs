//! PCem standalone-native gameport joystick writer.
//!
//! Pinned source: sarah-walker-pcem/pcem commit
//! `a5a54ab5981902fc227b0afd1756ebe4660178d0`. The per-machine config file
//! (selected with `--config`, defaulting beside the executable) carries a
//! `[Joysticks]` section: `joystick_0_nr` selects the SDL joystick plus one
//! (`plat_joystick_nr`, 0 unbinds), and `joystick_0_axis_N`,
//! `joystick_0_button_N`, `joystick_0_pov_N_x/y` carry raw SDL indices. The
//! standard 2-button joystick (`joystick_type` 0) needs axes 0/1 and buttons
//! 0/1. `SDL_JoystickOpen(c)` opens SDL slots, so the session proves the pad
//! holds slot 0. Content is the machine config path passed as `--config`.

use anyhow::{Context, Result, ensure};
use serde::{Deserialize, Serialize};
use std::collections::BTreeSet;

pub(crate) const SOURCE_COMMIT: &str = "a5a54ab5981902fc227b0afd1756ebe4660178d0";
pub(crate) const PROFILE_ID: &str = "pcem:standalone-gameport";
pub(crate) const SECTION: &str = "Joysticks";

/// Layout target ids covered by the native profile: the two gameport axes
/// and two buttons of the standard 2-button PC joystick.
pub(crate) const ROUTES: [(&str, &str); 6] = [
    ("left", "X axis left"),
    ("right", "X axis right"),
    ("up", "Y axis up"),
    ("down", "Y axis down"),
    ("a", "Button 1"),
    ("b", "Button 2"),
];

/// Patch only the player-1 gameport joystick keys in a copied PCem machine
/// config: `joystick_0_nr = 1` (SDL slot 0 plus one), axes, and buttons.
/// POV hats have no gameport slot and are refused by the session. All other
/// sections (machine, drives, video, sound) and keys survive verbatim.
pub(crate) fn patch_config(baseline: &[u8], axes: [u32; 2], buttons: [u32; 2]) -> Result<String> {
    ensure!(
        baseline.len() <= 4 * 1024 * 1024,
        "PCem config is too large"
    );
    let text = std::str::from_utf8(baseline).context("PCem config is not UTF-8")?;
    ensure!(!text.contains('\0'), "PCem config contains a NUL byte");
    for axis in axes {
        ensure!(axis <= 7, "PCem axis index is out of range");
    }
    for button in buttons {
        ensure!(button <= 31, "PCem button index is out of range");
    }
    let newline = if text.contains("\r\n") { "\r\n" } else { "\n" };
    let fields = [
        ("joystick_0_nr", 1.to_string()),
        ("joystick_0_axis_0", axes[0].to_string()),
        ("joystick_0_axis_1", axes[1].to_string()),
        ("joystick_0_button_0", buttons[0].to_string()),
        ("joystick_0_button_1", buttons[1].to_string()),
    ];
    let mut seen = [false; 5];
    let mut output = String::with_capacity(text.len() + 256);
    let mut section: Option<String> = None;
    for raw in text.split_inclusive('\n') {
        let line = raw.strip_suffix('\n').unwrap_or(raw);
        let trimmed = line.trim();
        if trimmed.starts_with('[') && trimmed.ends_with(']') {
            section = Some(trimmed[1..trimmed.len() - 1].trim().to_owned());
            output.push_str(raw);
            continue;
        }
        if section.as_deref() == Some(SECTION) {
            if let Some((key, _)) = line.split_once('=') {
                let key = key.trim();
                if let Some(index) = fields.iter().position(|(known, _)| *known == key) {
                    ensure!(!seen[index], "PCem config contains duplicate joystick key");
                    seen[index] = true;
                    output.push_str(&format!(
                        "{} = {}{newline}",
                        fields[index].0, fields[index].1
                    ));
                    continue;
                }
            }
        }
        output.push_str(raw);
    }
    for (index, (key, value)) in fields.iter().enumerate() {
        if !seen[index] {
            if !output.is_empty() && !output.ends_with('\n') {
                output.push_str(newline);
            }
            output.push_str(&format!("[{SECTION}]{newline}{key} = {value}{newline}"));
        }
    }
    Ok(output)
}

pub(crate) mod settings {
    use super::*;
    use crate::controller_catalog::{Calibration, catalog};
    use anyhow::Context;
    use std::{collections::HashMap, path::PathBuf};

    #[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
    #[serde(deny_unknown_fields)]
    pub(crate) struct Player {
        pub player: u8,
        pub controller_id: String,
    }

    #[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
    #[serde(deny_unknown_fields)]
    pub(crate) struct SavedSetup {
        pub emulator_id: String,
        pub content: PathBuf,
        pub probe_program: PathBuf,
        pub sdl_library: PathBuf,
        pub executable_sha256: String,
        pub players: Vec<Player>,
    }

    impl SavedSetup {
        pub(crate) fn validate(&self) -> Result<()> {
            ensure!(
                !self.emulator_id.trim().is_empty(),
                "PCem setup needs an emulator identity"
            );
            for path in [&self.content, &self.probe_program, &self.sdl_library] {
                ensure!(
                    path.is_absolute()
                        && !path
                            .components()
                            .any(|part| matches!(part, std::path::Component::ParentDir)),
                    "PCem setup paths must be absolute without parent traversal"
                );
            }
            ensure!(
                self.executable_sha256.len() == 64
                    && self
                        .executable_sha256
                        .bytes()
                        .all(|byte| byte.is_ascii_hexdigit()),
                "PCem setup needs a trusted executable SHA-256"
            );
            let limit = catalog()
                .emulator_profiles
                .iter()
                .find(|profile| profile.id == PROFILE_ID)
                .and_then(|profile| profile.native_launch.as_ref())
                .context("Missing PCem native profile")?
                .max_players;
            ensure!(
                self.players.len() == 1 && self.players[0].player == 1,
                "PCem supports exactly player one"
            );
            ensure!(
                self.players.len() <= limit && !self.players[0].controller_id.trim().is_empty(),
                "PCem player needs a saved controller identity"
            );
            Ok(())
        }

        pub(crate) fn review(
            &self,
            calibrations: &HashMap<String, Calibration>,
        ) -> Result<serde_json::Value> {
            self.validate()?;
            let profile = catalog()
                .emulator_profiles
                .iter()
                .find(|profile| profile.id == PROFILE_ID)
                .context("Missing PCem native profile")?;
            let player = &self.players[0];
            let calibration = calibrations
                .get(&player.controller_id)
                .context("PCem controller has no saved calibration")?;
            ensure!(
                ["linux", "macos", "windows"].contains(&calibration.os.as_str()),
                "PCem mapping requires Linux physical calibration"
            );
            let mapping = calibration.plan_profile(profile)?;
            ensure!(
                mapping.rows.iter().all(|row| row.physical_id.is_some()
                    && row
                        .input
                        .as_ref()
                        .is_some_and(|input| input.native.is_some())),
                "PCem needs native calibration for every gameport control"
            );
            Ok(serde_json::json!({
                "profile_id": PROFILE_ID,
                "player": player.player,
                "controller_id": player.controller_id,
                "source_layout": calibration.layout,
                "target_layout": profile.target_layout,
                "mapping": mapping,
                "launch_ready": false,
                "launch_integration": "partial",
                "detail": "Native launch passes a private machine config with a slot-0 gameport joystick, then rechecks the exact SDL2 routes. Only axes plus two raw buttons are supported; runtime behavior remains unverified."
            }))
        }
    }

    pub(crate) fn validate_setups(setups: &[SavedSetup]) -> Result<()> {
        ensure!(setups.len() <= 1024, "Too many PCem saved setups");
        let mut identities = BTreeSet::new();
        for setup in setups {
            setup.validate()?;
            ensure!(
                identities.insert((&setup.emulator_id, &setup.content)),
                "Duplicate PCem emulator/content setup"
            );
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn patches_joystick_keys_and_preserves_machine_sections() {
        let text = patch_config(
            b"[Machine]\nmodel = 1\n[Joysticks]\njoystick_0_nr = 0\n",
            [0, 1],
            [0, 1],
        )
        .unwrap();
        assert!(text.contains("[Machine]\nmodel = 1\n"));
        assert!(text.contains("joystick_0_nr = 1\n"));
        assert!(text.contains("joystick_0_axis_0 = 0\n"));
        assert!(text.contains("joystick_0_button_1 = 1\n"));
    }

    #[test]
    fn appends_missing_section_and_rejects_bad_indices() {
        let text = patch_config(b"[Machine]\n", [0, 1], [0, 1]).unwrap();
        assert!(text.contains("[Joysticks]"));
        assert!(patch_config(b"", [8, 1], [0, 1]).is_err());
        assert!(patch_config(b"", [0, 1], [0, 32]).is_err());
    }
}

mod session {
    use super::*;
    #[cfg(target_os = "linux")]
    use crate::controller_bizhawk_guard::InputTopology;
    #[cfg(not(target_os = "linux"))]
    use crate::controller_native_platform as platform;
    use crate::{
        controller_catalog::Calibration,
        controller_native_process::{cancelled, capture},
        controllers::ControllerDevice,
    };
    use lunchbox_controller_probe::{
        duckstation::DigitalInput,
        file_hash,
        linux_classic::AxisEndpoints,
        sdl2::{Device, Snapshot},
        sdl2_physical::PhysicalMap,
    };
    use std::{
        collections::{BTreeMap, HashMap},
        fs,
        path::PathBuf,
        process::Command,
        sync::atomic::AtomicBool,
    };

    fn observe(
        setup: &settings::SavedSetup,
        path: Option<&str>,
        cancel: &AtomicBool,
    ) -> Result<Snapshot> {
        let mut command = Command::new(&setup.probe_program);
        command
            .arg("--sdl2-inventory")
            .arg("--sdl-library")
            .arg(&setup.sdl_library);
        if let Some(path) = path {
            command.arg("--sdl2-controls-for-path").arg(path);
        }
        let (output, _) = capture(&mut command, cancel)?;
        let snapshot: Snapshot =
            serde_json::from_slice(&output).context("Invalid PCem SDL2 capture")?;
        ensure!(
            snapshot.version[0] == 2
                && snapshot.library.canonicalize()? == setup.sdl_library.canonicalize()?
                && snapshot.library_sha256 == file_hash(&setup.sdl_library)?,
            "PCem helper inspected a different SDL2 runtime"
        );
        Ok(snapshot)
    }

    fn routing(mut snapshot: Snapshot) -> Snapshot {
        for device in &mut snapshot.devices {
            device.controls = None;
            device.linux_classic = None;
            device.linux_evdev = None;
            device.sampled_state = None;
            device.mapping = None;
        }
        snapshot
    }

    pub(crate) struct PreparedSession {
        directory: tempfile::TempDir,
        config_path: PathBuf,
        physical_path: String,
        device_index: u32,
        #[cfg(target_os = "linux")]
        topology: InputTopology,
        initial: Snapshot,
        setup: settings::SavedSetup,
        hashes: BTreeMap<PathBuf, String>,
    }

    impl PreparedSession {
        pub(crate) fn prepare(
            setup: &settings::SavedSetup,
            calibrations: &HashMap<String, Calibration>,
            inventory: &[ControllerDevice],
            cancel: &AtomicBool,
        ) -> Result<Self> {
            cancelled(cancel)?;
            setup.review(calibrations)?;
            ensure!(
                fs::symlink_metadata(&setup.content)?.file_type().is_file()
                    && setup.content.canonicalize()? == setup.content,
                "PCem content must be a direct regular machine config with canonical ancestry"
            );
            let player = &setup.players[0];
            let found = inventory
                .iter()
                .filter(|device| device.stable_id == player.controller_id)
                .collect::<Vec<_>>();
            ensure!(
                found.len() == 1 && !found[0].is_virtual,
                "PCem physical controller is missing or ambiguous"
            );
            let selected = found[0].device_path.clone();
            let initial = routing(observe(setup, None, cancel)?);
            // Linux pins kernel input identity through the sysfs topology.
            // Other hosts pin the SDL device-interface path plus index and
            // re-probe it; names and GUIDs are never identity.
            #[cfg(target_os = "linux")]
            let (physical_path, topology) = {
                let topology = InputTopology::capture(std::slice::from_ref(&selected))?;
                let physical_path = topology.resolve_runtime_path(
                    &selected,
                    initial
                        .devices
                        .iter()
                        .filter_map(|device| device.path.as_deref()),
                )?;
                topology.verify()?;
                (physical_path, topology)
            };
            #[cfg(not(target_os = "linux"))]
            let physical_path = {
                let selected_string = selected.to_string_lossy().into_owned();
                let candidates = initial
                    .devices
                    .iter()
                    .filter(|device| device.path.as_deref() == Some(selected_string.as_str()))
                    .collect::<Vec<_>>();
                ensure!(
                    candidates.len() == 1,
                    "pcem physical controller is missing or ambiguous in SDL"
                );
                selected_string
            };
            let captured = observe(setup, Some(&physical_path), cancel)?;
            initial.ensure_same_routing(&routing(captured.clone()))?;
            #[cfg(target_os = "linux")]
            topology.verify()?;
            let device = captured.device_at_path(&physical_path)?;
            // SDL_JoystickOpen(c) opens SDL slots; plat_joystick_nr is
            // 1-based, so joystick_0_nr = 1 selects slot 0.
            ensure!(
                device.device_index == 0,
                "PCem joystick 0 needs SDL slot 0; selected pad is index {}",
                device.device_index
            );
            let calibration = calibrations
                .get(&player.controller_id)
                .context("PCem calibration disappeared")?;
            let profile = crate::controller_catalog::catalog()
                .emulator_profiles
                .iter()
                .find(|profile| profile.id == PROFILE_ID)
                .context("Missing PCem native profile")?;
            let physical = PhysicalMap::from_device(device)?;
            let state = device
                .sampled_state
                .as_ref()
                .context("PCem SDL released state is missing")?;
            state.validate(
                device
                    .controls
                    .as_ref()
                    .context("PCem SDL control counts are missing")?,
            )?;
            // Axes pair into X/Y halves; buttons land on their own raw
            // indices. Hats have no gameport slot and are refused.
            let mut axes: BTreeMap<String, (u32, bool)> = BTreeMap::new();
            let mut buttons: BTreeMap<String, u32> = BTreeMap::new();
            for row in calibration.plan_profile(profile)?.rows {
                ensure!(
                    ROUTES.iter().any(|(target, _)| *target == row.target_id),
                    "PCem target {} outside contract",
                    row.target_id
                );
                let input = row
                    .input
                    .as_ref()
                    .context("PCem gameport control is not calibrated")?;
                let native = input
                    .native
                    .as_ref()
                    .context("PCem requires measured native controls")?;
                let measured = input.axis.as_ref().map(|axis| AxisEndpoints {
                    released: axis.released,
                    pressed: axis.pressed,
                });
                let translated = physical.digital_input(native.code, measured)?;
                let released = match translated {
                    DigitalInput::Button(index) => state.buttons.get(&index) == Some(&false),
                    DigitalInput::Hat { .. } => {
                        anyhow::bail!("PCem gameport has no POV slot; map directions to axes")
                    }
                    DigitalInput::Axis {
                        index, released, ..
                    } => state.axes.get(&index) == Some(&released),
                };
                ensure!(
                    released,
                    "Release the PCem controls before launch preparation"
                );
                match translated {
                    DigitalInput::Button(index) => {
                        ensure!(
                            buttons
                                .insert(
                                    row.target_id,
                                    u32::try_from(index)
                                        .context("PCem button index is too large")?
                                )
                                .is_none(),
                            "PCem control appears twice"
                        );
                    }
                    DigitalInput::Axis { index, .. } => {
                        ensure!(
                            axes.insert(
                                row.target_id.clone(),
                                (
                                    u32::try_from(index).context("PCem axis index is too large")?,
                                    native.direction > 0
                                )
                            )
                            .is_none(),
                            "PCem stick direction appears twice"
                        );
                    }
                    DigitalInput::Hat { .. } => {
                        anyhow::bail!("PCem gameport has no POV slot; map directions to axes")
                    }
                }
            }
            let axis_of = |negative: &str, positive: &str| {
                let (neg_index, neg_dir) = axes
                    .get(negative)
                    .with_context(|| format!("PCem direction {negative} is not calibrated"))?;
                let (pos_index, pos_dir) = axes
                    .get(positive)
                    .with_context(|| format!("PCem direction {positive} is not calibrated"))?;
                ensure!(
                    neg_index == pos_index && !neg_dir && *pos_dir,
                    "PCem axis halves must share one axis with opposite polarity"
                );
                Ok::<u32, anyhow::Error>(*neg_index)
            };
            let axis_x = axis_of("left", "right")?;
            let axis_y = axis_of("up", "down")?;
            // The standard deck has exactly two buttons; anything the
            // user calibrated outside the four directions lands there in
            // sorted-target order for determinism.
            let mut names: Vec<String> = buttons.keys().cloned().collect();
            names.sort();
            ensure!(
                names.len() == 2,
                "PCem gameport needs exactly two buttons besides the axes"
            );
            let ordered = [buttons[&names[0]], buttons[&names[1]]];
            let directory = tempfile::Builder::new()
                .prefix("lunchbox-pcem-")
                .tempdir()?;
            let config_path = directory.path().join("pcem.cfg");
            fs::write(
                &config_path,
                patch_config(&fs::read(&setup.content)?, [axis_x, axis_y], ordered)?,
            )?;
            let mut hashes = BTreeMap::new();
            for path in [
                &setup.content,
                &setup.probe_program,
                &setup.sdl_library,
                &config_path,
            ] {
                hashes.insert(path.clone(), file_hash(path)?);
            }
            let prepared = Self {
                directory,
                config_path,
                physical_path,
                device_index: device.device_index,
                #[cfg(target_os = "linux")]
                topology,
                initial,
                setup: setup.clone(),
                hashes,
            };
            prepared.verify(cancel)?;
            Ok(prepared)
        }

        pub(crate) fn config_path(&self) -> &std::path::Path {
            &self.config_path
        }

        pub(crate) fn verify(&self, cancel: &AtomicBool) -> Result<()> {
            cancelled(cancel)?;
            #[cfg(target_os = "linux")]
            self.topology.verify()?;
            for (path, hash) in &self.hashes {
                ensure!(file_hash(path)? == *hash, "PCem launch input changed");
            }
            let fresh = routing(observe(&self.setup, None, cancel)?);
            self.initial.ensure_same_routing(&fresh)?;
            let captured = observe(&self.setup, Some(&self.physical_path), cancel)?;
            self.initial
                .ensure_same_routing(&routing(captured.clone()))?;
            let device = captured.device_at_path(&self.physical_path)?;
            ensure!(
                device.device_index == 0,
                "PCem SDL slot 0 moved before launch"
            );
            #[cfg(not(target_os = "linux"))]
            platform::require_unique_device_path(
                &captured.devices,
                &self.physical_path,
                self.device_index,
            )?;
            #[cfg(target_os = "linux")]
            {
                return self.topology.verify();
            }
            #[cfg(not(target_os = "linux"))]
            {
                return Ok(());
            }
        }

        pub(crate) fn check_health(&self) -> Result<()> {
            #[cfg(target_os = "linux")]
            return self.topology.verify();
            #[cfg(not(target_os = "linux"))]
            return self.verify_health_probe();
        }

        #[cfg(not(target_os = "linux"))]
        fn verify_health_probe(&self) -> Result<()> {
            // No sysfs exists here; health is a fresh same-routing probe
            // that still sees the pinned path at the pinned index.
            let fresh = routing(observe(&self.setup, None, &AtomicBool::new(false))?);
            self.initial.ensure_same_routing(&fresh)?;
            platform::require_unique_device_path(
                &fresh.devices,
                &self.physical_path,
                self.device_index,
            )?;
            Ok(())
        }
    }
}

pub(crate) mod native_command {
    use super::*;
    use crate::controller_native_process::cancelled;
    #[cfg(target_os = "linux")]
    use crate::controller_native_process::native_pid;
    use crate::{
        controller_catalog::Calibration,
        controller_native_platform as platform,
        controllers::ControllerDevice,
        emulator::{EmulatorExecutable, LaunchPlan, RomEmulatorOption},
    };
    use lunchbox_controller_probe::file_hash;
    use std::{
        collections::HashMap,
        path::PathBuf,
        sync::atomic::AtomicBool,
        time::{Duration, Instant},
    };

    pub(crate) struct NativeSession {
        inputs: session::PreparedSession,
        executable: PathBuf,
        setup: settings::SavedSetup,
        pub(crate) plan: LaunchPlan,
    }

    impl NativeSession {
        pub(crate) fn check_health(&self) -> Result<()> {
            self.inputs.check_health()
        }

        pub(crate) fn verify(&self, cancel: &AtomicBool) -> Result<()> {
            cancelled(cancel)?;
            ensure!(
                file_hash(&self.executable)?.eq_ignore_ascii_case(&self.setup.executable_sha256),
                "PCem executable differs from the saved trusted runtime"
            );
            self.inputs.verify(cancel)
        }

        pub(crate) fn spawn(
            &mut self,
            plan: &LaunchPlan,
            cancel: &AtomicBool,
        ) -> Result<std::process::Child> {
            ensure!(
                plan == &self.plan,
                "PCem launch plan changed after preparation"
            );
            self.verify(cancel)?;
            let mut child = crate::emulator::spawn_launch_plan(plan)?;
            if let Err(error) = self.confirm(&mut child, cancel) {
                let _ = child.kill();
                let _ = child.wait();
                return Err(error);
            }
            Ok(child)
        }

        fn confirm(&self, child: &mut std::process::Child, cancel: &AtomicBool) -> Result<()> {
            let deadline = Instant::now() + Duration::from_secs(20);
            loop {
                cancelled(cancel)?;
                ensure!(
                    child.try_wait()?.is_none(),
                    "PCem exited before controller handoff"
                );
                // Linux walks the launch tree (bubblewrap monitors); other
                // hosts check the direct child, which they spawn directly.
                #[cfg(target_os = "linux")]
                let owned = native_pid(child.id(), &self.executable)?
                    .is_some_and(|pid| self.ready(pid).unwrap_or(false));
                #[cfg(not(target_os = "linux"))]
                let owned = platform::child_exe_matches(child.id(), &self.executable)?
                    && self.ready(child.id())?;
                if owned {
                    self.inputs.check_health()?;
                    return Ok(());
                }
                ensure!(
                    Instant::now() < deadline,
                    "PCem did not open the selected SDL controller before timeout"
                );
                std::thread::sleep(Duration::from_millis(25));
            }
        }

        fn ready(&self, pid: u32) -> Result<bool> {
            // Linux proves the child mapped the exact SDL library. Other
            // hosts pin the executable plus a fresh device re-probe; the
            // weaker guarantee is explicit here and in the launch text.
            if cfg!(target_os = "linux") {
                return platform::child_maps_library(pid, &self.setup.sdl_library);
            }
            if !platform::child_exe_matches(pid, &self.executable)? {
                return Ok(false);
            }
            self.inputs.check_health()?;
            Ok(true)
        }
    }

    pub(crate) fn prepare(
        setup: &settings::SavedSetup,
        calibrations: &HashMap<String, Calibration>,
        inventory: &[ControllerDevice],
        option: &RomEmulatorOption,
        original: &LaunchPlan,
        cancel: &AtomicBool,
    ) -> Result<NativeSession> {
        cancelled(cancel)?;
        setup.validate()?;
        let EmulatorExecutable::Native(executable) = &option.executable else {
            anyhow::bail!("PCem calibrated launch requires a native build");
        };
        ensure!(
            option.emulator_name.eq_ignore_ascii_case("PCem")
                && setup.emulator_id == option.emulator_id
                && original.environment.is_empty()
                && original.retroarch_content.is_none(),
            "PCem identity differs or custom environment needs resolution"
        );
        let executable = executable.canonicalize()?;
        ensure!(
            executable == original.program.canonicalize()?,
            "PCem launch executable differs from selection"
        );
        ensure!(
            original.arguments.as_slice() == [setup.content.as_os_str()],
            "PCem calibrated launch requires exactly the saved content argument"
        );
        ensure!(
            file_hash(&executable)?.eq_ignore_ascii_case(&setup.executable_sha256),
            "PCem executable differs from the saved trusted runtime"
        );
        let inputs = session::PreparedSession::prepare(setup, calibrations, inventory, cancel)?;
        let mut plan = original.clone();
        plan.program = executable.clone();
        // --config selects the private machine config; the content path is
        // that same file.
        plan.arguments = vec![
            std::ffi::OsString::from("--config"),
            inputs.config_path().as_os_str().to_owned(),
        ];
        let session = NativeSession {
            inputs,
            executable,
            setup: setup.clone(),
            plan,
        };
        session.verify(cancel)?;
        Ok(session)
    }
}
