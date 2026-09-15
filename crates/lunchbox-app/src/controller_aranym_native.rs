//! ARAnyM standalone SDL joystick-selection writer.
//!
//! Pinned source: aranym/aranym commit
//! 5f4ebed6b039ddf42eef1122a315ad9608d20f6e. `src/parameters.cpp` defines
//! `[JOYSTICKS]` selectors and two 17-entry Jaguar joypad button permutations;
//! `src/input.cpp` passes nonnegative selectors to `SDL_JoystickOpen`.
//! Selectors are runtime enumeration indices, so callers must measure and
//! recheck them for the exact child SDL backend immediately before launch.

use anyhow::{Context, Result, ensure};
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};

pub(crate) const SOURCE_COMMIT: &str = "5f4ebed6b039ddf42eef1122a315ad9608d20f6e";
pub(crate) const PROFILE_ID: &str = "aranym:standalone-aranym-ikbd-joystick";
pub(crate) const CONTROLS: [(&str, &str); 5] = [
    ("up", "fixed SDL joystick Up"),
    ("down", "fixed SDL joystick Down"),
    ("left", "fixed SDL joystick Left"),
    ("right", "fixed SDL joystick Right"),
    ("fire", "fixed SDL joystick Fire"),
];

#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub(crate) enum GuestPort {
    Ikbd0,
    Ikbd1,
    JoypadA,
    JoypadB,
}

impl GuestPort {
    fn key(self) -> &'static str {
        match self {
            Self::Ikbd0 => "Ikbd0",
            Self::Ikbd1 => "Ikbd1",
            Self::JoypadA => "JoypadA",
            Self::JoypadB => "JoypadB",
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct PortBinding {
    pub port: GuestPort,
    /// SDL enumeration index measured for the exact launch. `None` writes -1
    /// and disables the port.
    pub runtime_index: Option<u8>,
    /// For JoypadA/JoypadB only: a permutation of 0..16 mapping the first 17
    /// SDL buttons to Jaguar fire, pause, option, keypad and undefined actions.
    pub button_permutation: Option<[u8; 17]>,
}

fn fields(bindings: &[PortBinding]) -> Result<BTreeMap<String, String>> {
    ensure!(
        !bindings.is_empty() && bindings.len() <= 4,
        "ARAnyM joystick binding count is invalid"
    );
    let mut ports = BTreeSet::new();
    let mut indices = BTreeSet::new();
    let mut result = BTreeMap::new();
    for binding in bindings {
        ensure!(
            ports.insert(binding.port),
            "ARAnyM guest port is duplicated"
        );
        let index = match binding.runtime_index {
            Some(index) => {
                ensure!(index <= 31, "ARAnyM SDL runtime index is out of range");
                ensure!(
                    indices.insert(index),
                    "ARAnyM SDL runtime index is duplicated"
                );
                index as i16
            }
            None => -1,
        };
        result.insert(binding.port.key().into(), index.to_string());

        let joypad = matches!(binding.port, GuestPort::JoypadA | GuestPort::JoypadB);
        ensure!(
            joypad == binding.button_permutation.is_some(),
            "ARAnyM button permutation is required only for JoypadA/JoypadB"
        );
        if let Some(permutation) = binding.button_permutation {
            let unique = permutation.iter().copied().collect::<BTreeSet<_>>();
            ensure!(
                unique.len() == 17 && unique.first() == Some(&0) && unique.last() == Some(&16),
                "ARAnyM joypad buttons must be a permutation of 0 through 16"
            );
            let value = permutation
                .iter()
                .map(u8::to_string)
                .collect::<Vec<_>>()
                .join(" ");
            result.insert(
                match binding.port {
                    GuestPort::JoypadA => "JoypadAButtons",
                    GuestPort::JoypadB => "JoypadBButtons",
                    _ => unreachable!(),
                }
                .into(),
                value,
            );
        }
    }
    Ok(result)
}

/// Patch only selected `[JOYSTICKS]` keys in a copied ARAnyM config. The
/// launch layer remains responsible for pinning the executable/SDL backend,
/// validating the measured indices against physical controller identities,
/// and preserving TOS, disks, GEMDOS folders, snapshots and guest saves.
pub(crate) fn patch_joysticks(baseline: &[u8], bindings: &[PortBinding]) -> Result<String> {
    ensure!(baseline.len() <= 1024 * 1024, "ARAnyM config is too large");
    let original = std::str::from_utf8(baseline).context("ARAnyM config is not UTF-8")?;
    let fields = fields(bindings)?;
    patch_section(original, "JOYSTICKS", &fields)
}

fn patch_section(
    original: &str,
    section: &str,
    fields: &BTreeMap<String, String>,
) -> Result<String> {
    ensure!(
        !original.contains('\0'),
        "ARAnyM config contains a NUL byte"
    );
    let newline = if original.contains("\r\n") {
        "\r\n"
    } else {
        "\n"
    };
    let mut output = String::new();
    let mut active = false;
    let mut inserted = false;
    for line in original.split_inclusive('\n') {
        if let Some(header) = line
            .trim_start()
            .strip_prefix('[')
            .and_then(|line| line.split_once(']').map(|(name, _)| name.trim()))
        {
            active = header == section;
            output.push_str(line);
            if active && !inserted {
                if !output.ends_with('\n') {
                    output.push_str(newline);
                }
                for (key, value) in fields {
                    output.push_str(&format!("{key} = {value}{newline}"));
                }
                inserted = true;
            }
        } else {
            let owned = active
                && line
                    .split_once('=')
                    .is_some_and(|(key, _)| fields.keys().any(|known| key.trim() == known));
            if !owned {
                output.push_str(line);
            }
        }
    }
    if !inserted {
        if !output.is_empty() && !output.ends_with('\n') {
            output.push_str(newline);
        }
        output.push_str(&format!("[{section}]{newline}"));
        for (key, value) in fields {
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
        /// The user's existing ARAnyM config. The private copy is mounted back
        /// at this exact path so config-relative TOS, disk and GEMDOS paths do
        /// not change meaning.
        pub config_path: PathBuf,
        pub probe_program: PathBuf,
        pub sdl_library: PathBuf,
        pub bubblewrap_program: PathBuf,
        pub executable_sha256: String,
        pub players: Vec<Player>,
    }

    impl SavedSetup {
        pub(crate) fn validate(&self) -> Result<()> {
            ensure!(
                !self.emulator_id.trim().is_empty(),
                "ARAnyM setup needs an emulator identity"
            );
            let mut path_vec = vec![
                &self.content,
                &self.config_path,
                &self.probe_program,
                &self.sdl_library,
            ];
            // bubblewrap is required only when the launch actually
            // sandboxes (plain native/Nix Linux); elsewhere the field is
            // accepted and ignored by the direct launch below.
            #[cfg(target_os = "linux")]
            path_vec.push(&self.bubblewrap_program);
            for path in path_vec {
                ensure!(
                    path.is_absolute()
                        && !path
                            .components()
                            .any(|part| matches!(part, std::path::Component::ParentDir)),
                    "ARAnyM setup paths must be absolute without parent traversal"
                );
            }
            ensure!(
                self.executable_sha256.len() == 64
                    && self
                        .executable_sha256
                        .bytes()
                        .all(|byte| byte.is_ascii_hexdigit()),
                "ARAnyM setup needs a trusted executable SHA-256"
            );
            ensure!(
                matches!(self.players.len(), 1 | 2),
                "ARAnyM IKBD setup requires one or two joystick players"
            );
            let mut controllers = BTreeSet::new();
            for (index, player) in self.players.iter().enumerate() {
                ensure!(
                    usize::from(player.player) == index + 1
                        && !player.controller_id.trim().is_empty()
                        && controllers.insert(&player.controller_id),
                    "ARAnyM players must be distinct, contiguous, and start at one"
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
                .context("Missing ARAnyM native profile")?;
            let mut players = Vec::new();
            for player in &self.players {
                let calibration = calibrations
                    .get(&player.controller_id)
                    .context("ARAnyM controller has no saved calibration")?;
                ensure!(
                    ["linux", "macos", "windows"].contains(&calibration.os.as_str()),
                    "ARAnyM mapping requires a desktop calibration"
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
                    "ARAnyM needs native calibration for every joystick control"
                );
                players.push(serde_json::json!({
                    "player": player.player,
                    "guest_port": if player.player == 1 { "Ikbd1" } else { "Ikbd0" },
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
                "detail": "Native Linux launch mounts a private controller-only copy at the original ARAnyM config path, rechecks the exact SDL2 runtime slots, and enforces the pinned source's axis-0/axis-1-or-hat and any-button IKBD behavior. TOS, disks, GEMDOS folders, NVRAM, guest saves and snapshots retain their original paths. Runtime behavior is unverified."
            }))
        }
    }

    pub(crate) fn validate_setups(setups: &[SavedSetup]) -> Result<()> {
        ensure!(setups.len() <= 1024, "Too many ARAnyM saved setups");
        let mut identities = BTreeSet::new();
        for setup in setups {
            setup.validate()?;
            ensure!(
                identities.insert((&setup.emulator_id, &setup.content)),
                "Duplicate ARAnyM emulator/content setup"
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
        collections::{BTreeMap, HashMap},
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
            serde_json::from_slice(&output).context("Invalid ARAnyM SDL2 capture")?;
        ensure!(
            snapshot.version[0] == 2
                && snapshot.library.canonicalize()? == setup.sdl_library.canonicalize()?
                && snapshot.library_sha256 == file_hash(&setup.sdl_library)?,
            "ARAnyM helper inspected a different SDL2 runtime"
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
            .context("Missing ARAnyM native profile")?;
        let mut result = BTreeMap::new();
        for row in calibration.plan_profile(profile)?.rows {
            let input = row
                .input
                .as_ref()
                .context("ARAnyM joystick control is not calibrated")?;
            let native = input
                .native
                .as_ref()
                .context("ARAnyM requires measured native controls")?;
            let endpoints = input.axis.as_ref().map(|axis| AxisEndpoints {
                released: axis.released,
                pressed: axis.pressed,
            });
            let binding = physical.digital_input(native.code, endpoints)?;
            ensure!(
                result.insert(row.target_id, binding).is_none(),
                "ARAnyM target control appears twice"
            );
        }
        Ok(result)
    }

    fn validate_fixed_mapping(mapped: &BTreeMap<String, DigitalInput>) -> Result<()> {
        let input = |name: &str| {
            mapped
                .get(name)
                .cloned()
                .with_context(|| format!("ARAnyM control {name} is absent"))
        };
        let [up, down, left, right] = [
            input("up")?,
            input("down")?,
            input("left")?,
            input("right")?,
        ];
        let axis_pair = |negative: &DigitalInput, positive: &DigitalInput, index: u32| {
            matches!((negative, positive),
                (DigitalInput::Axis { index: first, released: first_rest, pressed: first_press },
                 DigitalInput::Axis { index: second, released: second_rest, pressed: second_press })
                if first == second && *first == index
                    && first_rest.abs() < 10_000 && second_rest.abs() < 10_000
                    && *first_press < -16_384 && *second_press > 16_384)
        };
        let hat_pair = |negative: &DigitalInput, positive: &DigitalInput, first: u8, second: u8| {
            matches!((negative, positive),
                (DigitalInput::Hat { index: first_hat, direction: first_direction },
                 DigitalInput::Hat { index: second_hat, direction: second_direction })
                if first_hat == second_hat && *first_direction == first && *second_direction == second)
        };
        ensure!(
            (axis_pair(&left, &right, 0) && axis_pair(&up, &down, 1))
                || (hat_pair(&left, &right, 8, 2) && hat_pair(&up, &down, 1, 4)),
            "ARAnyM's fixed IKBD mapping requires SDL axes 0/1 or one cardinal hat"
        );
        ensure!(
            matches!(input("fire")?, DigitalInput::Button(_)),
            "ARAnyM IKBD fire requires a physical button"
        );
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
                    "ARAnyM physical controller is missing or ambiguous"
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
                        "ARAnyM physical controller is missing or ambiguous in SDL"
                    );
                    selected_string
                };
                ensure!(
                    !runtime_paths.contains(&runtime_path),
                    "ARAnyM players resolved to the same controller"
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
                    device.device_index <= 31
                        && device.instance_id == i32::try_from(device.device_index)?,
                    "ARAnyM requires an SDL slot 0-31 whose event instance ID equals its slot"
                );
                validate_fixed_mapping(&inputs(
                    calibrations
                        .get(&player.controller_id)
                        .context("ARAnyM calibration disappeared")?,
                    &captured,
                    &runtime_path,
                )?)?;
                slots.push(u8::try_from(device.device_index)?);
                device_indices.push(device.device_index);
                runtime_paths.push(runtime_path);
            }
            ensure!(
                slots.iter().collect::<BTreeSet<_>>().len() == slots.len(),
                "ARAnyM SDL slots are duplicated"
            );
            let identity = std::array::from_fn(|index| index as u8);
            let bindings = [
                PortBinding {
                    port: GuestPort::Ikbd0,
                    runtime_index: slots.get(1).copied(),
                    button_permutation: None,
                },
                PortBinding {
                    port: GuestPort::Ikbd1,
                    runtime_index: slots.first().copied(),
                    button_permutation: None,
                },
                PortBinding {
                    port: GuestPort::JoypadA,
                    runtime_index: None,
                    button_permutation: Some(identity),
                },
                PortBinding {
                    port: GuestPort::JoypadB,
                    runtime_index: None,
                    button_permutation: Some(identity),
                },
            ];
            let baseline = std::fs::read(&setup.config_path)
                .context("Reading the declared ARAnyM configuration")?;
            let directory = tempfile::Builder::new()
                .prefix("lunchbox-aranym-native-")
                .tempdir()?;
            let private_config = directory.path().join("config");
            std::fs::write(&private_config, patch_joysticks(&baseline, &bindings)?)?;
            let mut hashes = BTreeMap::new();
            let mut hash_paths = vec![
                &setup.content,
                &setup.config_path,
                &setup.probe_program,
                &setup.sdl_library,
                &private_config,
            ];
            // bubblewrap is required only when the launch actually
            // sandboxes (plain native/Nix Linux); elsewhere the field is
            // accepted and ignored by the direct launch below.
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
                "ARAnyM private configuration disappeared"
            );
            #[cfg(target_os = "linux")]
            self.topology.verify()?;
            for (path, expected) in &self.hashes {
                ensure!(file_hash(path)? == *expected, "ARAnyM launch input changed");
            }
            let fresh = routing(observe(&self.setup, None, cancel)?);
            self.initial.ensure_same_routing(&fresh)?;
            for (path, index) in self.runtime_paths.iter().zip(&self.device_indices) {
                let device = fresh.device_at_path(path)?;
                ensure!(
                    device.device_index == *index
                        && device.device_index <= 31
                        && device.instance_id == i32::try_from(device.device_index)?,
                    "ARAnyM SDL slot/instance identity changed"
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
                ensure!(file_hash(path)? == *expected, "ARAnyM executable changed");
            }
            ensure!(
                self.files[&self.executable].eq_ignore_ascii_case(&self.setup.executable_sha256),
                "ARAnyM executable differs from the saved trusted runtime"
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
                "ARAnyM launch plan changed after preparation"
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
            anyhow::bail!("ARAnyM calibrated launch requires a native build");
        };
        ensure!(
            option.emulator_name.eq_ignore_ascii_case("ARAnyM")
                && setup.emulator_id == option.emulator_id
                && original.environment.is_empty(),
            "ARAnyM identity differs or custom environment needs resolution"
        );
        let executable = executable.canonicalize()?;
        ensure!(
            executable == original.program.canonicalize()?,
            "ARAnyM executable differs from selection"
        );
        ensure!(
            original.arguments.as_slice() == [setup.content.as_os_str()],
            "ARAnyM calibrated launch currently requires the default single-content plan"
        );
        let mut files = BTreeMap::new();
        files.insert(executable.clone(), file_hash(&executable)?);
        // bubblewrap exists only on Linux; elsewhere the field is accepted
        // and ignored by the unsandboxed launch below.
        #[cfg(target_os = "linux")]
        files.insert(
            setup.bubblewrap_program.clone(),
            file_hash(&setup.bubblewrap_program)?,
        );
        ensure!(
            files[&executable].eq_ignore_ascii_case(&setup.executable_sha256),
            "ARAnyM executable differs from the saved trusted runtime"
        );
        let sandboxed = crate::controller_native_platform::use_bubblewrap_sandbox(&executable);
        let inputs =
            session::PreparedSession::prepare(setup, calibrations, inventory, sandboxed, cancel)?;
        #[cfg(target_os = "linux")]
        let cwd = original.current_directory.canonicalize()?;
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
                "--bind".into(),
                inputs.private_config.as_os_str().to_owned(),
                setup.config_path.as_os_str().to_owned(),
                "--chdir".into(),
                cwd.into_os_string(),
                "--".into(),
                executable.as_os_str().to_owned(),
                "--config".into(),
                setup.config_path.as_os_str().to_owned(),
                "--floppy".into(),
                setup.content.as_os_str().to_owned(),
            ];
        } else {
            plan.program = executable.clone();
            plan.arguments = vec![
                "--config".into(),
                inputs.private_config.as_os_str().to_owned(),
                "--floppy".into(),
                setup.content.as_os_str().to_owned(),
            ];
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
    fn writes_measured_index_and_exact_permutation() {
        let permutation = std::array::from_fn(|index| (16 - index) as u8);
        let output = patch_joysticks(
            b"[GLOBAL]\nFastRAM = 64\n[JOYSTICKS]\nJoypadA = -1\nJoypadAButtons = 0 1 2\nIkbd0 = -1\n",
            &[
                PortBinding {
                    port: GuestPort::Ikbd0,
                    runtime_index: Some(0),
                    button_permutation: None,
                },
                PortBinding {
                    port: GuestPort::JoypadA,
                    runtime_index: Some(1),
                    button_permutation: Some(permutation),
                },
            ],
        )
        .unwrap();
        assert!(output.contains("Ikbd0 = 0\nJoypadA = 1\nJoypadAButtons = 16 15 14 13"));
        assert!(output.contains("[GLOBAL]\nFastRAM = 64\n"));
        assert!(!output.contains("JoypadA = -1"));
    }

    #[test]
    fn rejects_stale_or_ambiguous_selection_data() {
        let duplicate = [
            PortBinding {
                port: GuestPort::Ikbd0,
                runtime_index: Some(0),
                button_permutation: None,
            },
            PortBinding {
                port: GuestPort::Ikbd1,
                runtime_index: Some(0),
                button_permutation: None,
            },
        ];
        assert!(patch_joysticks(b"", &duplicate).is_err());
        let mut invalid = std::array::from_fn(|index| index as u8);
        invalid[16] = 15;
        assert!(
            patch_joysticks(
                b"",
                &[PortBinding {
                    port: GuestPort::JoypadB,
                    runtime_index: Some(2),
                    button_permutation: Some(invalid),
                }]
            )
            .is_err()
        );
    }
}
