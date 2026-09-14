//! SkyEmu standalone-native SDL controller settings.
//!
//! Pinned to skylersaleh/SkyEmu commit 01516d6798e3652b583e6a366085bb51c43b528d.
//! The native SDL frontend stores each selected controller's key and analog
//! maps as a raw host-endian `int32_t[64 * 2]` file (little-endian on the
//! supported desktop hosts) named
//! `<SDL_GetPrefPath("Sky", "SkyEmu")><controller-name>-bindings.bin`.

use anyhow::{Result, ensure};
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};

pub(crate) const SOURCE_COMMIT: &str = "01516d6798e3652b583e6a366085bb51c43b528d";
pub(crate) const PROFILE_ID: &str = "skyemu:standalone-native-sdl-controller";
pub(crate) const UPSTREAM_URL: &str = "https://github.com/skylersaleh/SkyEmu";
pub(crate) const SDL_PREF_ORGANIZATION: &str = "Sky";
pub(crate) const SDL_PREF_APPLICATION: &str = "SkyEmu";
pub(crate) const BINDING_SLOTS: usize = 64;

/// The controller key-map indices from `sb_types.h`.
pub(crate) const KEY_CONTROLS: [(&str, usize); 36] = [
    ("a", 0),
    ("b", 1),
    ("x", 2),
    ("y", 3),
    ("up", 4),
    ("down", 5),
    ("left", 6),
    ("right", 7),
    ("l", 8),
    ("r", 9),
    ("start", 10),
    ("select", 11),
    ("fold_screen", 12),
    ("pen_down", 13),
    ("pause", 14),
    ("rewind", 15),
    ("fast_forward_2x", 16),
    ("fast_forward_max", 17),
    ("capture_state_0", 18),
    ("restore_state_0", 19),
    ("capture_state_1", 20),
    ("restore_state_1", 21),
    ("capture_state_2", 22),
    ("restore_state_2", 23),
    ("capture_state_3", 24),
    ("restore_state_3", 25),
    ("reset_game", 26),
    ("turbo_a", 27),
    ("turbo_b", 28),
    ("turbo_x", 29),
    ("turbo_y", 30),
    ("turbo_l", 31),
    ("turbo_r", 32),
    ("solar_sensor_plus", 33),
    ("solar_sensor_minus", 34),
    ("toggle_fullscreen", 35),
];

/// The analog-map indices from `main.c`.
pub(crate) const ANALOG_CONTROLS: [(&str, usize); 4] =
    [("up_down", 0), ("left_right", 1), ("l", 2), ("r", 3)];

/// A raw SDL controller binding as encoded by `se_get_sdl_key_bind`.
#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub(crate) enum Binding {
    Button(u8),
    Axis { index: u8, negative: bool },
    Hat { index: u8, direction: HatDirection },
}

#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub(crate) enum HatDirection {
    Up = 1,
    Down = 4,
    Left = 8,
    Right = 2,
}

impl Binding {
    fn encoded(self) -> i32 {
        match self {
            Self::Button(index) => i32::from(index),
            Self::Axis { index, negative } => {
                i32::from(index) | if negative { 1 << 18 } else { 1 << 17 }
            }
            Self::Hat { index, direction } => {
                (1 << 16) | (i32::from(index) << 8) | direction as i32
            }
        }
    }
}

fn valid_controller_name(name: &str) -> Result<()> {
    ensure!(
        !name.is_empty() && name.len() <= 127,
        "SkyEmu controller name is invalid"
    );
    ensure!(
        !name.chars().any(|ch| {
            ch == '/' || ch == '\\' || ch == '\0' || ch == '\n' || ch == '\r' || ch.is_control()
        }),
        "SkyEmu controller name contains a path separator or control character"
    );
    Ok(())
}

/// Return the exact per-controller filename SkyEmu derives from SDL's
/// preference directory. `pref_path` must be the SDL-returned path, including
/// its trailing separator.
pub(crate) fn binding_path(pref_path: &str, controller_name: &str) -> Result<String> {
    valid_controller_name(controller_name)?;
    ensure!(
        !pref_path.contains('\0'),
        "SkyEmu preference path contains NUL"
    );
    ensure!(
        pref_path.ends_with('/') || pref_path.ends_with('\\'),
        "SkyEmu SDL preference path must include its trailing separator"
    );
    Ok(format!("{pref_path}{controller_name}-bindings.bin"))
}

/// Render the exact 512-byte key/analog binding file. Supported desktop hosts
/// are little-endian, matching the source's raw `int32_t` write. Unspecified
/// slots retain SkyEmu's `-1` sentinel; unrelated user settings and save-state
/// files are not touched by this mapping-only writer.
pub(crate) fn controller_bindings(
    controller_name: &str,
    key_bindings: &BTreeMap<String, Binding>,
    analog_bindings: &BTreeMap<String, u8>,
) -> Result<Vec<u8>> {
    valid_controller_name(controller_name)?;
    ensure!(
        key_bindings
            .keys()
            .all(|key| KEY_CONTROLS.iter().any(|(name, _)| name == key)),
        "SkyEmu key binding name is outside the source contract"
    );
    ensure!(
        analog_bindings
            .keys()
            .all(|key| ANALOG_CONTROLS.iter().any(|(name, _)| name == key)),
        "SkyEmu analog binding name is outside the source contract"
    );
    ensure!(
        !key_bindings.is_empty() || !analog_bindings.is_empty(),
        "SkyEmu controller binding map is empty"
    );

    let mut slots = [-1_i32; BINDING_SLOTS * 2];
    for (name, binding) in key_bindings {
        let index = KEY_CONTROLS
            .iter()
            .find(|(known, _)| *known == name)
            .map(|(_, index)| *index)
            .expect("validated SkyEmu key binding name");
        slots[index] = binding.encoded();
    }
    for (name, axis) in analog_bindings {
        let index = ANALOG_CONTROLS
            .iter()
            .find(|(known, _)| *known == name)
            .map(|(_, index)| *index)
            .expect("validated SkyEmu analog binding name");
        slots[BINDING_SLOTS + index] = i32::from(*axis);
    }

    let mut output = Vec::with_capacity(slots.len() * 4);
    for value in slots {
        output.extend_from_slice(&value.to_le_bytes());
    }
    Ok(output)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn emits_source_binary_layout_and_sentinels() {
        let keys = BTreeMap::from([
            ("a".into(), Binding::Button(0)),
            (
                "up".into(),
                Binding::Hat {
                    index: 0,
                    direction: HatDirection::Up,
                },
            ),
        ]);
        let analog = BTreeMap::from([("left_right".into(), 1_u8)]);
        let bytes = controller_bindings("Pad One", &keys, &analog).unwrap();
        assert_eq!(bytes.len(), 512);
        assert_eq!(&bytes[0..4], &[0, 0, 0, 0]);
        assert_eq!(&bytes[16..20], &[1, 0, 1, 0]);
        assert_eq!(&bytes[64 * 4 + 4..64 * 4 + 8], &[1, 0, 0, 0]);
        assert_eq!(&bytes[12..16], &[255, 255, 255, 255]);
        assert_eq!(
            binding_path("/tmp/SkyEmu/", "Pad One").unwrap(),
            "/tmp/SkyEmu/Pad One-bindings.bin"
        );
        assert!(binding_path("/tmp/SkyEmu", "Pad One").is_err());
        assert_eq!(SOURCE_COMMIT.len(), 40);
        assert_eq!(UPSTREAM_URL, "https://github.com/skylersaleh/SkyEmu");
    }

    #[test]
    fn encodes_axis_and_hat_masks() {
        let keys = BTreeMap::from([
            (
                "a".into(),
                Binding::Axis {
                    index: 2,
                    negative: true,
                },
            ),
            (
                "b".into(),
                Binding::Hat {
                    index: 1,
                    direction: HatDirection::Right,
                },
            ),
        ]);
        let bytes = controller_bindings("Pad", &keys, &BTreeMap::new()).unwrap();
        assert_eq!(i32::from_le_bytes(bytes[0..4].try_into().unwrap()), 0x40002);
        assert_eq!(i32::from_le_bytes(bytes[4..8].try_into().unwrap()), 0x10102);
    }

    #[test]
    fn invalid_names_and_unknown_bindings_fail_closed() {
        let keys = BTreeMap::from([("a".into(), Binding::Button(0))]);
        assert!(binding_path("/tmp/", "../Pad").is_err());
        assert!(
            controller_bindings(
                "Pad",
                &BTreeMap::from([("unknown".into(), Binding::Button(0))]),
                &BTreeMap::new()
            )
            .is_err()
        );
        assert!(controller_bindings("Pad", &keys, &BTreeMap::new()).is_ok());
    }
}

/// Gameplay controls covered by the native profile, all present in the
/// writer's `KEY_CONTROLS` table.
pub(crate) const PROFILE_CONTROLS: [&str; 12] = [
    "a", "b", "x", "y", "up", "down", "left", "right", "l", "r", "start", "select",
];

pub(crate) mod settings {
    use super::*;
    use crate::controller_catalog::{Calibration, catalog};
    use anyhow::Context;
    use serde::{Deserialize, Serialize};
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
                "SkyEmu setup needs an emulator identity"
            );
            for path in [&self.content, &self.probe_program, &self.sdl_library] {
                ensure!(
                    path.is_absolute()
                        && !path
                            .components()
                            .any(|part| matches!(part, std::path::Component::ParentDir)),
                    "SkyEmu setup paths must be absolute without parent traversal"
                );
            }
            ensure!(
                self.executable_sha256.len() == 64
                    && self
                        .executable_sha256
                        .bytes()
                        .all(|byte| byte.is_ascii_hexdigit()),
                "SkyEmu setup needs a trusted executable SHA-256"
            );
            let limit = catalog()
                .emulator_profiles
                .iter()
                .find(|profile| profile.id == PROFILE_ID)
                .and_then(|profile| profile.native_launch.as_ref())
                .context("Missing SkyEmu native profile")?
                .max_players;
            ensure!(
                self.players.len() == 1 && self.players[0].player == 1,
                "SkyEmu supports exactly player one"
            );
            ensure!(
                self.players.len() <= limit && !self.players[0].controller_id.trim().is_empty(),
                "SkyEmu player needs a saved controller identity"
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
                .context("Missing SkyEmu native profile")?;
            let player = &self.players[0];
            let calibration = calibrations
                .get(&player.controller_id)
                .context("SkyEmu controller has no saved calibration")?;
            ensure!(
                calibration.os == "linux",
                "SkyEmu mapping requires Linux physical calibration"
            );
            let mapping = calibration.plan_profile(profile)?;
            ensure!(
                mapping.rows.iter().all(|row| row.physical_id.is_some()
                    && row
                        .input
                        .as_ref()
                        .is_some_and(|input| input.native.is_some())),
                "SkyEmu needs native calibration for every DS control"
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
                "detail": "Native Linux launch writes a private <name>-bindings.bin, then rechecks the exact SDL2 routes. Only the single DS pad is supported; runtime behavior remains unverified."
            }))
        }
    }

    pub(crate) fn validate_setups(setups: &[SavedSetup]) -> Result<()> {
        ensure!(setups.len() <= 1024, "Too many SkyEmu saved setups");
        let mut identities = BTreeSet::new();
        for setup in setups {
            setup.validate()?;
            ensure!(
                identities.insert((&setup.emulator_id, &setup.content)),
                "Duplicate SkyEmu emulator/content setup"
            );
        }
        Ok(())
    }
}

#[cfg(target_os = "linux")]
mod session {
    use super::*;
    use crate::{
        controller_bizhawk_guard::InputTopology,
        controller_catalog::Calibration,
        controller_native_process::{cancelled, capture},
        controllers::ControllerDevice,
    };
    use anyhow::Context;
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
            serde_json::from_slice(&output).context("Invalid SkyEmu SDL2 capture")?;
        ensure!(
            snapshot.version[0] == 2
                && snapshot.library.canonicalize()? == setup.sdl_library.canonicalize()?
                && snapshot.library_sha256 == file_hash(&setup.sdl_library)?,
            "SkyEmu helper inspected a different SDL2 runtime"
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

    /// SDL hat bitmask to the source's raw hat direction value (SDL_HAT_*).
    fn hat_direction(direction: u8) -> Result<HatDirection> {
        Ok(match direction {
            0x01 => HatDirection::Up,
            0x04 => HatDirection::Down,
            0x08 => HatDirection::Left,
            0x02 => HatDirection::Right,
            _ => anyhow::bail!("SkyEmu hat direction is not cardinal"),
        })
    }

    fn key_bindings(
        calibration: &Calibration,
        device: &Device,
    ) -> Result<BTreeMap<String, Binding>> {
        let profile = crate::controller_catalog::catalog()
            .emulator_profiles
            .iter()
            .find(|profile| profile.id == PROFILE_ID)
            .context("Missing SkyEmu native profile")?;
        let physical = PhysicalMap::from_device(device)?;
        let state = device
            .sampled_state
            .as_ref()
            .context("SkyEmu SDL released state is missing")?;
        state.validate(
            device
                .controls
                .as_ref()
                .context("SkyEmu SDL control counts are missing")?,
        )?;
        let mut result = BTreeMap::new();
        for row in calibration.plan_profile(profile)?.rows {
            ensure!(
                PROFILE_CONTROLS.contains(&row.target_id.as_str()),
                "SkyEmu target {} outside native profile",
                row.target_id
            );
            let input = row
                .input
                .as_ref()
                .context("SkyEmu DS control is not calibrated")?;
            let native = input
                .native
                .as_ref()
                .context("SkyEmu requires measured native controls")?;
            let measured = input.axis.as_ref().map(|axis| AxisEndpoints {
                released: axis.released,
                pressed: axis.pressed,
            });
            let translated = physical.digital_input(native.code, measured)?;
            let released = match translated {
                DigitalInput::Button(index) => state.buttons.get(&index) == Some(&false),
                DigitalInput::Hat { index, direction } => state
                    .hats
                    .get(&index)
                    .is_some_and(|mask| mask & direction == 0),
                DigitalInput::Axis {
                    index, released, ..
                } => state.axes.get(&index) == Some(&released),
            };
            ensure!(
                released,
                "Release the SkyEmu controls before launch preparation"
            );
            // The source encodes only raw SDL joystick numbers: buttons and
            // axis indices directly, hats as hat index plus SDL_HAT_*
            // direction. Stick axes arrive here as separate digital halves;
            // the encoding carries no polarity, so both halves of one axis
            // would collide in the same slot and are refused.
            let binding = match translated {
                DigitalInput::Button(index) => Binding::Button(
                    u8::try_from(index).context("SkyEmu button index is too large")?,
                ),
                DigitalInput::Hat { index, direction } => Binding::Hat {
                    index: u8::try_from(index).context("SkyEmu hat index is too large")?,
                    direction: hat_direction(direction)?,
                },
                DigitalInput::Axis { .. } => {
                    anyhow::bail!(
                        "SkyEmu analog halves have no polarity encoding; map this control to a button or hat"
                    )
                }
            };
            ensure!(
                result.insert(row.target_id, binding).is_none(),
                "SkyEmu target control appears twice"
            );
        }
        ensure!(
            result.len() == PROFILE_CONTROLS.len(),
            "SkyEmu native mapping is incomplete"
        );
        Ok(result)
    }

    pub(crate) struct PreparedSession {
        directory: tempfile::TempDir,
        bindings_path: PathBuf,
        controller_name: String,
        physical_path: String,
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
                "SkyEmu content must be a direct regular file with canonical ancestry"
            );
            let player = &setup.players[0];
            let found = inventory
                .iter()
                .filter(|device| device.stable_id == player.controller_id)
                .collect::<Vec<_>>();
            ensure!(
                found.len() == 1 && !found[0].is_virtual,
                "SkyEmu physical controller is missing or ambiguous"
            );
            let selected = found[0].device_path.clone();
            let topology = InputTopology::capture(std::slice::from_ref(&selected))?;
            let initial = routing(observe(setup, None, cancel)?);
            let physical_path = topology.resolve_runtime_path(
                &selected,
                initial
                    .devices
                    .iter()
                    .filter_map(|device| device.path.as_deref()),
            )?;
            let captured = observe(setup, Some(&physical_path), cancel)?;
            initial.ensure_same_routing(&routing(captured.clone()))?;
            topology.verify()?;
            let device = captured.device_at_path(&physical_path)?;
            // The filename is the file identity; the SDL GUID is
            // runtime-only. The name must be unique in this probe or the
            // source itself could not distinguish the pads.
            let controller_name = device
                .name
                .as_deref()
                .context("SkyEmu SDL device has no name")?;
            ensure!(
                initial
                    .devices
                    .iter()
                    .filter_map(|other| other.name.as_deref())
                    .filter(|other| *other == controller_name)
                    .count()
                    == 1,
                "SkyEmu SDL controller name is duplicated"
            );
            let bindings = key_bindings(
                calibrations
                    .get(&player.controller_id)
                    .context("SkyEmu calibration disappeared")?,
                device,
            )?;
            let bytes = controller_bindings(controller_name, &bindings, &BTreeMap::new())?;
            let directory = tempfile::Builder::new()
                .prefix("lunchbox-skyemu-")
                .tempdir()?;
            // SDL_GetPrefPath("Sky", "SkyEmu") resolves under XDG_DATA_HOME
            // on Linux; the launch layer points it here so only the private
            // bindings file is visible.
            let pref_dir = directory.path().join("Sky").join("SkyEmu");
            fs::create_dir_all(&pref_dir)?;
            let bindings_path = pref_dir.join(format!("{controller_name}-bindings.bin"));
            fs::write(&bindings_path, bytes)?;
            let mut hashes = BTreeMap::new();
            for path in [
                &setup.content,
                &setup.probe_program,
                &setup.sdl_library,
                &bindings_path,
            ] {
                hashes.insert(path.clone(), file_hash(path)?);
            }
            let prepared = Self {
                directory,
                bindings_path,
                controller_name: controller_name.to_owned(),
                physical_path,
                topology,
                initial,
                setup: setup.clone(),
                hashes,
            };
            prepared.verify(cancel)?;
            Ok(prepared)
        }

        /// Private data root for `XDG_DATA_HOME`; SDL appends
        /// `/Sky/SkyEmu` itself.
        pub(crate) fn data_home(&self) -> &std::path::Path {
            self.directory.path()
        }

        pub(crate) fn controller_name(&self) -> &str {
            &self.controller_name
        }

        pub(crate) fn verify(&self, cancel: &AtomicBool) -> Result<()> {
            cancelled(cancel)?;
            self.topology.verify()?;
            for (path, hash) in &self.hashes {
                ensure!(file_hash(path)? == *hash, "SkyEmu launch input changed");
            }
            let fresh = routing(observe(&self.setup, None, cancel)?);
            self.initial.ensure_same_routing(&fresh)?;
            let captured = observe(&self.setup, Some(&self.physical_path), cancel)?;
            self.initial.ensure_same_routing(&routing(captured))?;
            self.topology.verify()
        }

        pub(crate) fn check_health(&self) -> Result<()> {
            self.topology.verify()
        }
    }
}

#[cfg(target_os = "linux")]
pub(crate) mod native_command {
    use super::*;
    use crate::{
        controller_catalog::Calibration,
        controller_native_process::{cancelled, native_pid},
        controllers::ControllerDevice,
        emulator::{EmulatorExecutable, LaunchPlan, RomEmulatorOption},
    };
    use lunchbox_controller_probe::file_hash;
    use std::{
        collections::HashMap,
        path::{Path, PathBuf},
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
                "SkyEmu executable differs from the saved trusted runtime"
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
                "SkyEmu launch plan changed after preparation"
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
                    "SkyEmu exited before controller handoff"
                );
                if let Some(pid) = native_pid(child.id(), &self.executable)?
                    && self.ready(pid)?
                {
                    self.inputs.check_health()?;
                    return Ok(());
                }
                ensure!(
                    Instant::now() < deadline,
                    "SkyEmu did not open the selected SDL controller before timeout"
                );
                std::thread::sleep(Duration::from_millis(25));
            }
        }

        fn ready(&self, pid: u32) -> Result<bool> {
            let expected_sdl = self.setup.sdl_library.canonicalize()?;
            let maps = std::fs::read_to_string(format!("/proc/{pid}/maps"))?;
            if !maps.lines().any(|line| {
                let path = line
                    .split_whitespace()
                    .skip(5)
                    .collect::<Vec<_>>()
                    .join(" ")
                    .replace("\\040", " ");
                Path::new(&path) == expected_sdl
            }) {
                return Ok(false);
            }
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
            anyhow::bail!("SkyEmu calibrated launch requires native Linux");
        };
        ensure!(
            option.emulator_name.eq_ignore_ascii_case("SkyEmu")
                && setup.emulator_id == option.emulator_id
                && original.environment.is_empty()
                && original.retroarch_content.is_none(),
            "SkyEmu identity differs or custom environment needs resolution"
        );
        let executable = executable.canonicalize()?;
        ensure!(
            executable == original.program.canonicalize()?,
            "SkyEmu launch executable differs from selection"
        );
        ensure!(
            original.arguments.as_slice() == [setup.content.as_os_str()],
            "SkyEmu calibrated launch requires exactly the saved content argument"
        );
        ensure!(
            file_hash(&executable)?.eq_ignore_ascii_case(&setup.executable_sha256),
            "SkyEmu executable differs from the saved trusted runtime"
        );
        let inputs = session::PreparedSession::prepare(setup, calibrations, inventory, cancel)?;
        let mut plan = original.clone();
        plan.program = executable.clone();
        plan.environment.push((
            std::ffi::OsString::from("XDG_DATA_HOME"),
            inputs.data_home().as_os_str().to_owned(),
        ));
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
