//! Oricutron standalone SDL joystick-selection writer.
//!
//! Pinned source: pete-gordon/oricutron commit
//! 002279fce9fa756d1d63cdc40ae97939eb7de7ed. `main.c` parses
//! `joystick_a`, `joystick_b`, `telejoy_a`, and `telejoy_b` as `sdljoy0`
//! through `sdljoy9`; `joystick.c` opens the corresponding runtime SDL index.
//! The writer therefore accepts only a same-launch measured index and leaves
//! physical identity verification to the launch guard.

use anyhow::{Context, Result, ensure};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

pub(crate) const SOURCE_COMMIT: &str = "002279fce9fa756d1d63cdc40ae97939eb7de7ed";
pub(crate) const PROFILE_ID: &str = "oricutron:standalone-spectrum-joystick";
pub(crate) const CONTROLS: [(&str, &str); 5] = [
    ("up", "fixed SDL joystick Up"),
    ("down", "fixed SDL joystick Down"),
    ("left", "fixed SDL joystick Left"),
    ("right", "fixed SDL joystick Right"),
    ("fire", "fixed SDL joystick Fire"),
];

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum AtmosInterface {
    None,
    AltaiPase,
    Ijk,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub(crate) enum MachinePorts {
    AtmosIjk,
    AtmosAltaiPase,
    Telestrat,
}

impl AtmosInterface {
    fn value(self) -> &'static str {
        match self {
            Self::None => "none",
            Self::AltaiPase => "altai",
            Self::Ijk => "ijk",
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) struct ControllerSelection {
    pub atmos_interface: AtmosInterface,
    pub atmos_a: Option<u8>,
    pub atmos_b: Option<u8>,
    pub telestrat_a: Option<u8>,
    pub telestrat_b: Option<u8>,
}

fn selector(index: Option<u8>) -> Result<String> {
    match index {
        Some(index) => {
            ensure!(
                index <= 9,
                "Oricutron SDL joystick index must be 0 through 9"
            );
            Ok(format!("sdljoy{index}"))
        }
        None => Ok("none".into()),
    }
}

/// Patch all source-defined physical joystick selectors in a copied
/// `oricutron.cfg`. Keyboard joystick mappings, ROMs, media, autosave and
/// machine settings are preserved. Because Oricutron compares SDL event
/// instance numbers with the stored enumeration number, the launch layer must
/// re-probe and recheck each selected physical device immediately before
/// starting the exact executable.
pub(crate) fn patch_config(baseline: &[u8], selection: ControllerSelection) -> Result<String> {
    ensure!(
        baseline.len() <= 1024 * 1024,
        "Oricutron config is too large"
    );
    ensure!(
        selection.atmos_a.is_none()
            || selection.atmos_b.is_none()
            || selection.atmos_a != selection.atmos_b,
        "Oricutron Atmos ports cannot share one SDL runtime index"
    );
    ensure!(
        selection.telestrat_a.is_none()
            || selection.telestrat_b.is_none()
            || selection.telestrat_a != selection.telestrat_b,
        "Oricutron Telestrat ports cannot share one SDL runtime index"
    );
    ensure!(
        selection.atmos_interface != AtmosInterface::None
            || (selection.atmos_a.is_none() && selection.atmos_b.is_none()),
        "Oricutron Atmos controller requires an enabled joystick interface"
    );
    let fields = BTreeMap::from([
        ("joyinterface", selection.atmos_interface.value().to_owned()),
        ("joystick_a", selector(selection.atmos_a)?),
        ("joystick_b", selector(selection.atmos_b)?),
        ("telejoy_a", selector(selection.telestrat_a)?),
        ("telejoy_b", selector(selection.telestrat_b)?),
    ]);
    let original = std::str::from_utf8(baseline).context("Oricutron config is not UTF-8")?;
    ensure!(
        !original.contains('\0'),
        "Oricutron config contains a NUL byte"
    );
    let newline = if original.contains("\r\n") {
        "\r\n"
    } else {
        "\n"
    };
    let mut output = String::new();
    let mut found = BTreeMap::new();
    for line in original.split_inclusive('\n') {
        let owned = line.split_once('=').and_then(|(key, _)| {
            let key = key.trim();
            fields.get_key_value(key)
        });
        if let Some((key, value)) = owned {
            ensure!(
                !found.contains_key(key),
                "Oricutron config key is duplicated"
            );
            found.insert(key.clone(), ());
            output.push_str(&format!("{key} = {value}{newline}"));
        } else {
            output.push_str(line);
        }
    }
    for (key, value) in &fields {
        if !found.contains_key(key) {
            if !output.is_empty() && !output.ends_with('\n') {
                output.push_str(newline);
            }
            output.push_str(&format!("{key} = {value}{newline}"));
        }
    }
    Ok(output)
}

pub(crate) mod settings {
    use super::*;
    use crate::controller_catalog::{Calibration, catalog};
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
        /// The executable's sibling oricutron.cfg; overlaid read-only at launch.
        pub config_path: PathBuf,
        pub probe_program: PathBuf,
        pub sdl_library: PathBuf,
        pub bubblewrap_program: PathBuf,
        pub executable_sha256: String,
        pub machine_ports: MachinePorts,
        pub players: Vec<Player>,
    }

    impl SavedSetup {
        pub(crate) fn validate(&self) -> Result<()> {
            ensure!(
                !self.emulator_id.trim().is_empty(),
                "Oricutron setup needs an emulator identity"
            );
            let mut path_vec = vec![
                &self.content,
                &self.config_path,
                &self.probe_program,
                &self.sdl_library,
            ];
            // bubblewrap is required only when a launch can actually
            // sandbox; elsewhere the field is accepted and ignored.
            #[cfg(target_os = "linux")]
            path_vec.push(&self.bubblewrap_program);
            for path in path_vec {
                ensure!(
                    path.is_absolute()
                        && !path
                            .components()
                            .any(|part| matches!(part, std::path::Component::ParentDir)),
                    "Oricutron setup paths must be absolute without parent traversal"
                );
            }
            ensure!(
                self.config_path.file_name().and_then(|name| name.to_str())
                    == Some("oricutron.cfg"),
                "Oricutron config_path must name oricutron.cfg"
            );
            ensure!(
                self.executable_sha256.len() == 64
                    && self
                        .executable_sha256
                        .bytes()
                        .all(|byte| byte.is_ascii_hexdigit()),
                "Oricutron setup needs a trusted executable SHA-256"
            );
            ensure!(
                matches!(self.players.len(), 1 | 2),
                "Oricutron setup requires one or two joystick players"
            );
            let mut controllers = std::collections::BTreeSet::new();
            for (index, player) in self.players.iter().enumerate() {
                ensure!(
                    usize::from(player.player) == index + 1
                        && !player.controller_id.trim().is_empty()
                        && controllers.insert(&player.controller_id),
                    "Oricutron players must be distinct, contiguous, and start at one"
                );
            }
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
                .context("Missing Oricutron native profile")?;
            let mut players = Vec::new();
            for player in &self.players {
                let calibration = calibrations
                    .get(&player.controller_id)
                    .context("Oricutron controller has no saved calibration")?;
                ensure!(
                    ["linux", "macos", "windows"].contains(&calibration.os.as_str()),
                    "Oricutron mapping requires Linux calibration"
                );
                let mapping = calibration.plan_profile(profile)?;
                ensure!(
                    mapping.rows.iter().all(|row| {
                        row.physical_id.is_some()
                            && row
                                .input
                                .as_ref()
                                .is_some_and(|input| input.native.is_some())
                    }),
                    "Oricutron needs native calibration for every joystick control"
                );
                players.push(serde_json::json!({
                    "player": player.player,
                    "controller_id": player.controller_id,
                    "source_layout": calibration.layout,
                    "target_layout": profile.target_layout,
                    "mapping": mapping,
                }));
            }
            Ok(serde_json::json!({
                "players": players,
                "launch_ready": false,
                "launch_integration": "partial",
                "detail": "Native Linux launch overlays only oricutron.cfg inside bubblewrap and rechecks exact SDL2 joystick slots. The pinned source's fixed axis/hat/button interpretation and instance-ID comparison are enforced at launch; incompatible mappings are rejected. Firmware, media and snapshots retain their original locations. Runtime behavior is unverified."
            }))
        }
    }

    pub(crate) fn validate_setups(setups: &[SavedSetup]) -> Result<()> {
        ensure!(setups.len() <= 1024, "Too many Oricutron saved setups");
        let mut identities = std::collections::BTreeSet::new();
        for setup in setups {
            setup.validate()?;
            ensure!(
                identities.insert((&setup.emulator_id, &setup.content)),
                "Duplicate Oricutron emulator/content setup"
            );
        }
        Ok(())
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
        duckstation::DigitalInput, file_hash, linux_classic::AxisEndpoints, sdl2::Snapshot,
        sdl2_physical::PhysicalMap,
    };
    use std::{
        collections::{BTreeMap, BTreeSet, HashMap},
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
            serde_json::from_slice(&output).context("Invalid Oricutron SDL2 capture")?;
        ensure!(
            snapshot.version[0] == 2
                && snapshot.library.canonicalize()? == setup.sdl_library.canonicalize()?
                && snapshot.library_sha256 == file_hash(&setup.sdl_library)?,
            "Oricutron helper inspected a different SDL2 runtime"
        );
        Ok(snapshot)
    }

    fn routing(mut snapshot: Snapshot) -> Snapshot {
        for device in &mut snapshot.devices {
            device.controls = None;
            device.linux_classic = None;
            device.linux_evdev = None;
            device.sampled_state = None;
        }
        snapshot
    }

    fn inputs(
        calibration: &Calibration,
        snapshot: &Snapshot,
        runtime_path: &str,
    ) -> Result<BTreeMap<String, DigitalInput>> {
        let device = snapshot.device_at_path(runtime_path)?;
        let physical = PhysicalMap::from_device(device)?;
        let profile = crate::controller_catalog::catalog()
            .emulator_profiles
            .iter()
            .find(|profile| profile.id == PROFILE_ID)
            .context("Missing Oricutron native profile")?;
        let mut result = BTreeMap::new();
        for row in calibration.plan_profile(profile)?.rows {
            let input = row
                .input
                .as_ref()
                .context("Oricutron joystick control is not calibrated")?;
            let native = input
                .native
                .as_ref()
                .context("Oricutron requires measured native controls")?;
            let endpoints = input.axis.as_ref().map(|axis| AxisEndpoints {
                released: axis.released,
                pressed: axis.pressed,
            });
            let binding = physical.digital_input(native.code, endpoints)?;
            ensure!(
                result.insert(row.target_id, binding).is_none(),
                "Oricutron target control appears twice"
            );
        }
        Ok(result)
    }

    fn validate_fixed_mapping(
        device: &lunchbox_controller_probe::sdl2::Device,
        mapped: &BTreeMap<String, DigitalInput>,
    ) -> Result<()> {
        let input = |name: &str| {
            mapped
                .get(name)
                .cloned()
                .with_context(|| format!("Oricutron control {name} is absent"))
        };
        let [up, down, left, right] = [
            input("up")?,
            input("down")?,
            input("left")?,
            input("right")?,
        ];
        let axis_pair = |negative: &DigitalInput, positive: &DigitalInput, valid: &[u32]| {
            matches!((negative, positive),
                (DigitalInput::Axis { index: first, released: first_rest, pressed: first_press },
                 DigitalInput::Axis { index: second, released: second_rest, pressed: second_press })
                if first == second && valid.contains(first)
                    && first_rest.abs() < 10_000 && second_rest.abs() < 10_000
                    && *first_press < -10_000 && *second_press > 10_000)
        };
        let hat_pair = |negative: &DigitalInput, positive: &DigitalInput, first: u8, second: u8| {
            matches!((negative, positive),
                (DigitalInput::Hat { index: first_hat, direction: first_direction },
                 DigitalInput::Hat { index: second_hat, direction: second_direction })
                if first_hat == second_hat && *first_direction == first && *second_direction == second)
        };
        let dualsense_buttons = device.name.as_deref() == Some("DualSense Wireless Controller")
            && [up.clone(), down.clone(), left.clone(), right.clone()]
                == [
                    DigitalInput::Button(11),
                    DigitalInput::Button(12),
                    DigitalInput::Button(13),
                    DigitalInput::Button(14),
                ];
        ensure!(
            (axis_pair(&left, &right, &[0, 2]) && axis_pair(&up, &down, &[1, 3]))
                || (hat_pair(&left, &right, 8, 2) && hat_pair(&up, &down, 1, 4))
                || dualsense_buttons,
            "Oricutron's fixed source mapping cannot represent the selected directions"
        );
        match input("fire")? {
            DigitalInput::Button(_) => {}
            DigitalInput::Axis {
                index,
                released,
                pressed,
            } if matches!(index, 4 | 5) && released.abs() < 10_000 && pressed > 10_000 => {}
            _ => anyhow::bail!("Oricutron's fixed source mapping cannot represent Fire"),
        }
        Ok(())
    }

    pub(crate) struct PreparedSession {
        directory: tempfile::TempDir,
        pub(crate) private_config: PathBuf,
        runtime_paths: Vec<String>,
        device_indices: Vec<u32>,
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
            sandboxed: bool,
            cancel: &AtomicBool,
        ) -> Result<Self> {
            cancelled(cancel)?;
            setup.review(calibrations)?;
            let mut selected = Vec::new();
            for player in &setup.players {
                let matches = inventory
                    .iter()
                    .filter(|device| device.stable_id == player.controller_id)
                    .collect::<Vec<_>>();
                ensure!(
                    matches.len() == 1 && !matches[0].is_virtual,
                    "Oricutron physical controller is missing or ambiguous"
                );
                selected.push(matches[0].device_path.clone());
            }
            #[cfg(target_os = "linux")]
            let topology = InputTopology::capture(&selected)?;
            let initial = routing(observe(setup, None, cancel)?);
            let mut runtime_paths = Vec::new();
            let mut device_indices = Vec::new();
            let mut slots = Vec::new();
            for (player, selected) in setup.players.iter().zip(&selected) {
                // Linux resolves through the sysfs topology; other hosts
                // match the SDL device-interface path and require uniqueness.
                #[cfg(target_os = "linux")]
                let runtime_path = topology.resolve_runtime_path(
                    selected,
                    initial
                        .devices
                        .iter()
                        .filter_map(|device| device.path.as_deref()),
                )?;
                #[cfg(not(target_os = "linux"))]
                let runtime_path = {
                    let selected_string = selected.to_string_lossy().into_owned();
                    let candidates = initial
                        .devices
                        .iter()
                        .filter(|device| device.path.as_deref() == Some(selected_string.as_str()))
                        .collect::<Vec<_>>();
                    ensure!(
                        candidates.len() == 1,
                        "Oricutron physical controller is missing or ambiguous in SDL"
                    );
                    selected_string
                };
                ensure!(
                    !runtime_paths.contains(&runtime_path),
                    "Oricutron players resolved to the same controller"
                );
                let captured = observe(setup, Some(&runtime_path), cancel)?;
                initial.ensure_same_routing(&routing(captured.clone()))?;
                #[cfg(target_os = "linux")]
                topology.verify()?;
                let device = captured.device_at_path(&runtime_path)?;
                #[cfg(not(target_os = "linux"))]
                platform::require_unique_device_path(
                    &captured.devices,
                    &runtime_path,
                    device.device_index,
                )?;
                ensure!(
                    device.device_index <= 9
                        && device.instance_id == i32::try_from(device.device_index)?,
                    "Oricutron requires an SDL slot 0-9 whose event instance ID equals its slot"
                );
                validate_fixed_mapping(
                    device,
                    &inputs(
                        calibrations
                            .get(&player.controller_id)
                            .context("Oricutron calibration disappeared")?,
                        &captured,
                        &runtime_path,
                    )?,
                )?;
                slots.push(u8::try_from(device.device_index)?);
                device_indices.push(device.device_index);
                runtime_paths.push(runtime_path);
            }
            ensure!(
                slots.iter().collect::<BTreeSet<_>>().len() == slots.len(),
                "Oricutron SDL slots are duplicated"
            );
            let selection = match setup.machine_ports {
                MachinePorts::AtmosIjk | MachinePorts::AtmosAltaiPase => ControllerSelection {
                    atmos_interface: if setup.machine_ports == MachinePorts::AtmosIjk {
                        AtmosInterface::Ijk
                    } else {
                        AtmosInterface::AltaiPase
                    },
                    atmos_a: slots.first().copied(),
                    atmos_b: slots.get(1).copied(),
                    telestrat_a: None,
                    telestrat_b: None,
                },
                MachinePorts::Telestrat => ControllerSelection {
                    atmos_interface: AtmosInterface::None,
                    atmos_a: None,
                    atmos_b: None,
                    telestrat_a: slots.first().copied(),
                    telestrat_b: slots.get(1).copied(),
                },
            };
            let baseline = std::fs::read(&setup.config_path)
                .context("Reading the declared Oricutron configuration")?;
            let directory = tempfile::Builder::new()
                .prefix("lunchbox-oricutron-native-")
                .tempdir()?;
            let private_config = directory.path().join("oricutron.cfg");
            std::fs::write(&private_config, patch_config(&baseline, selection)?)?;
            let mut hashes = BTreeMap::new();
            let mut hash_paths = vec![
                &setup.content,
                &setup.config_path,
                &setup.probe_program,
                &setup.sdl_library,
                &private_config,
            ];
            if sandboxed {
                hash_paths.push(&setup.bubblewrap_program);
            }
            for path in hash_paths {
                hashes.insert(path.clone(), file_hash(path)?);
            }
            let session = Self {
                directory,
                private_config,
                runtime_paths,
                device_indices,
                #[cfg(target_os = "linux")]
                topology,
                initial,
                setup: setup.clone(),
                hashes,
            };
            session.verify(cancel)?;
            Ok(session)
        }

        pub(crate) fn verify(&self, cancel: &AtomicBool) -> Result<()> {
            cancelled(cancel)?;
            ensure!(
                self.directory.path().is_dir() && self.private_config.is_file(),
                "Oricutron private configuration disappeared"
            );
            #[cfg(target_os = "linux")]
            self.topology.verify()?;
            for (path, expected) in &self.hashes {
                ensure!(
                    file_hash(path)? == *expected,
                    "Oricutron launch input changed"
                );
            }
            let fresh = routing(observe(&self.setup, None, cancel)?);
            self.initial.ensure_same_routing(&fresh)?;
            for (path, index) in self.runtime_paths.iter().zip(&self.device_indices) {
                let device = fresh.device_at_path(path)?;
                ensure!(
                    device.device_index == *index
                        && device.device_index <= 9
                        && device.instance_id == i32::try_from(device.device_index)?,
                    "Oricutron SDL slot/instance identity changed"
                );
                #[cfg(not(target_os = "linux"))]
                platform::require_unique_device_path(&fresh.devices, path, *index)?;
            }
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
            let fresh = routing(observe(&self.setup, None, &AtomicBool::new(false))?);
            self.initial.ensure_same_routing(&fresh)?;
            for (path, index) in self.runtime_paths.iter().zip(&self.device_indices) {
                platform::require_unique_device_path(&fresh.devices, path, *index)?;
            }
            Ok(())
        }
    }
}

pub(crate) mod native_command {
    use super::*;
    use crate::{
        controller_catalog::Calibration,
        controller_native_process::cancelled,
        controllers::ControllerDevice,
        emulator::{EmulatorExecutable, LaunchPlan, RomEmulatorOption},
    };
    use lunchbox_controller_probe::file_hash;
    use std::{collections::HashMap, path::PathBuf, sync::atomic::AtomicBool};

    pub(crate) struct NativeSession {
        inputs: session::PreparedSession,
        executable: PathBuf,
        setup: settings::SavedSetup,
        files: BTreeMap<PathBuf, String>,
        pub(crate) plan: LaunchPlan,
    }

    impl NativeSession {
        pub(crate) fn check_health(&self) -> Result<()> {
            self.inputs.check_health()
        }

        pub(crate) fn verify(&self, cancel: &AtomicBool) -> Result<()> {
            cancelled(cancel)?;
            for (path, expected) in &self.files {
                ensure!(
                    file_hash(path)? == *expected,
                    "Oricutron launch executable changed"
                );
            }
            ensure!(
                self.files[&self.executable].eq_ignore_ascii_case(&self.setup.executable_sha256),
                "Oricutron executable differs from the saved trusted runtime"
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
                "Oricutron launch plan changed after preparation"
            );
            self.verify(cancel)?;
            crate::emulator::spawn_launch_plan(plan)
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
            anyhow::bail!("Oricutron calibrated launch requires a native build");
        };
        ensure!(
            option.emulator_name.eq_ignore_ascii_case("oricutron")
                && setup.emulator_id == option.emulator_id
                && original.environment.is_empty(),
            "Oricutron identity differs or custom environment needs resolution"
        );
        let executable = executable.canonicalize()?;
        ensure!(
            executable == original.program.canonicalize()?,
            "Oricutron executable differs from selection"
        );
        ensure!(
            original.arguments.as_slice() == [setup.content.as_os_str()],
            "Oricutron calibrated launch requires exactly the saved content argument"
        );
        ensure!(
            setup.config_path.canonicalize()?
                == executable
                    .parent()
                    .context("Oricutron executable has no parent")?
                    .join("oricutron.cfg")
                    .canonicalize()?,
            "Oricutron config is not the selected executable's sibling"
        );
        let mut files = BTreeMap::new();
        files.insert(executable.clone(), file_hash(&executable)?);
        // bubblewrap is hashed only when the launch actually sandboxes.
        let sandboxed = crate::controller_native_platform::use_bubblewrap_sandbox(&executable);
        if sandboxed {
            files.insert(
                setup.bubblewrap_program.clone(),
                file_hash(&setup.bubblewrap_program)?,
            );
        }
        ensure!(
            files[&executable].eq_ignore_ascii_case(&setup.executable_sha256),
            "Oricutron executable differs from the saved trusted runtime"
        );
        let inputs =
            session::PreparedSession::prepare(setup, calibrations, inventory, sandboxed, cancel)?;
        let mut plan = original.clone();
        // Sandbox or direct is a packaging decision, not an OS one:
        // bubblewrap nests under plain native/Nix Linux launches, while
        // Flatpak/AppImage-contained launches and other hosts run the
        // trusted executable directly against the private config.
        if sandboxed {
            let cwd = original.current_directory.canonicalize()?;
            plan.program = setup.bubblewrap_program.clone();
            plan.arguments = vec![
                "--die-with-parent".into(),
                "--bind".into(),
                "/".into(),
                "/".into(),
                "--ro-bind".into(),
                inputs.private_config.as_os_str().to_owned(),
                setup.config_path.as_os_str().to_owned(),
                "--chdir".into(),
                cwd.into_os_string(),
                "--".into(),
                executable.as_os_str().to_owned(),
                setup.content.as_os_str().to_owned(),
            ];
        } else {
            plan.program = executable.clone();
            plan.arguments = vec![setup.content.as_os_str().to_owned()];
        }
        let session = NativeSession {
            inputs,
            executable,
            setup: setup.clone(),
            files,
            plan,
        };
        session.verify(cancel)?;
        Ok(session)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn writes_measured_slots_and_preserves_unrelated_config() {
        let output = patch_config(
            b"; controller\njoystick_a = kbjoy1\njoystick_b = none\ndiskautosave = yes\n",
            ControllerSelection {
                atmos_interface: AtmosInterface::Ijk,
                atmos_a: Some(2),
                atmos_b: Some(3),
                telestrat_a: None,
                telestrat_b: None,
            },
        )
        .unwrap();
        assert!(output.contains("joystick_a = sdljoy2\n"));
        assert!(output.contains("joystick_b = sdljoy3\n"));
        assert!(output.contains("joyinterface = ijk\n"));
        assert!(output.contains("diskautosave = yes\n"));
    }

    #[test]
    fn rejects_out_of_range_or_ambiguous_slots() {
        let mut selection = ControllerSelection {
            atmos_interface: AtmosInterface::AltaiPase,
            atmos_a: Some(10),
            atmos_b: None,
            telestrat_a: None,
            telestrat_b: None,
        };
        assert!(patch_config(b"", selection).is_err());
        selection.atmos_a = Some(1);
        selection.atmos_b = Some(1);
        assert!(patch_config(b"", selection).is_err());
    }
}
