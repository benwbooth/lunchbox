//! Nestopia UE standalone FLTK/SDL2 controller mappings.
//!
//! Functional contract pinned to 0ldsk00l/nestopia tag 1.53.2 (commit
//! 4470a2e99199d8010322eef4bf680fb3760f6eda):
//! - `source/fltkui/inputmanager.cpp` stores joystick bindings in a sibling
//!   `<device>j` INI section as `j<player>b<N>`, `j<player>h<N>`, or
//!   `j<player>a<N>`; axes encode each half as `axis * 2 + polarity`.
//! - `source/fltkui/jg/jg_nes.h` names the standard controller sections
//!   `nespad1` through `nespad4` and the controls Up, Down, Left, Right,
//!   Select, Start, A, B, TurboA and TurboB.
//! - `source/fltkui/jg.cpp` selects standard controllers with `port1` through
//!   `port4 = 1` in the `[nestopia]` section.
//!
//! The writer emits only these exact fragments.  SDL player indices are
//! assigned dynamically by hotplug/order (`SDL_JoystickSetPlayerIndex`), so
//! callers must verify the selected physical device immediately before
//! launch; this module does not claim persistent device identity.

use anyhow::{Result, ensure};
use std::collections::{BTreeMap, BTreeSet};

pub(crate) const SOURCE_COMMIT: &str = "4470a2e99199d8010322eef4bf680fb3760f6eda";
pub(crate) const PROFILE_ID: &str = "nestopia-ue:standalone-nes";

/// Standard NES controller fields in the `jg_nes.h` order.
pub(crate) const CONTROLS: [(&str, &str); 10] = [
    ("up", "Up"),
    ("down", "Down"),
    ("left", "Left"),
    ("right", "Right"),
    ("select", "Select"),
    ("start", "Start"),
    ("a", "A"),
    ("b", "B"),
    ("turbo_a", "TurboA"),
    ("turbo_b", "TurboB"),
];

/// Native SDL joystick input code used by `InputManager::remap_js`.
#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub(crate) enum Binding {
    Button(u8),
    Axis { index: u8, positive: bool },
    Hat { index: u8, direction: u8 },
}

impl Binding {
    fn code(self, player: u8) -> Result<String> {
        ensure!(
            player <= 9,
            "Nestopia SDL player index must fit j[0-9] grammar"
        );
        match self {
            Self::Button(index) => {
                ensure!(index < 64, "Nestopia SDL button index is out of range");
                Ok(format!("j{player}b{index}"))
            }
            Self::Axis { index, positive } => {
                ensure!(index < 16, "Nestopia SDL axis index is out of range");
                let half = u16::from(index) * 2 + u16::from(positive);
                Ok(format!("j{player}a{half}"))
            }
            Self::Hat { index, direction } => {
                ensure!(index < 4, "Nestopia SDL hat index is out of range");
                ensure!(direction < 4, "Nestopia hat direction must be cardinal");
                // Nestopia fltkui maps SDL_HAT_UP/DOWN/LEFT/RIGHT to 0/1/2/3.
                Ok(format!(
                    "j{player}h{}",
                    u16::from(index) * 4 + direction as u16
                ))
            }
        }
    }
}

/// Render the `[nespadN j]` section fragment for one emulated port.  The
/// eight standard controls are required; TurboA/TurboB are optional and are
/// left to the existing baseline when omitted.
pub(crate) fn input_fragment(
    emulated_port: u8,
    host_player: u8,
    mappings: &BTreeMap<String, Binding>,
) -> Result<String> {
    ensure!(
        (1..=4).contains(&emulated_port),
        "Nestopia supports four NES ports"
    );
    let required = ["up", "down", "left", "right", "select", "start", "a", "b"];
    ensure!(
        required
            .iter()
            .all(|control| mappings.contains_key(*control)),
        "Nestopia needs every standard NES control"
    );
    let mut used = BTreeSet::new();
    let mut result = format!("[nespad{emulated_port}j]\n");
    for (control, key) in CONTROLS {
        let Some(binding) = mappings.get(control) else {
            continue;
        };
        ensure!(used.insert(*binding), "Nestopia reuses one physical input");
        result.push_str(key);
        result.push_str(" = ");
        result.push_str(&binding.code(host_player)?);
        result.push('\n');
    }
    Ok(result)
}

/// Render a `[nestopia]` fragment selecting standard controller hardware for
/// the listed emulated ports.  Other ports and settings remain in the
/// caller's copied baseline.
pub(crate) fn controller_fragment(ports: &[u8]) -> Result<String> {
    ensure!(
        !ports.is_empty() && ports.len() <= 4,
        "Nestopia has four NES ports"
    );
    let mut used = BTreeSet::new();
    let mut result = String::from("[nestopia]\n");
    for &port in ports {
        ensure!(
            (1..=4).contains(&port),
            "Nestopia port must be one through four"
        );
        ensure!(used.insert(port), "Nestopia port is duplicated");
        result.push_str(&format!("port{port} = 1\n"));
    }
    Ok(result)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn mappings() -> BTreeMap<String, Binding> {
        [
            (
                "up",
                Binding::Axis {
                    index: 1,
                    positive: false,
                },
            ),
            (
                "down",
                Binding::Axis {
                    index: 1,
                    positive: true,
                },
            ),
            (
                "left",
                Binding::Axis {
                    index: 0,
                    positive: false,
                },
            ),
            (
                "right",
                Binding::Axis {
                    index: 0,
                    positive: true,
                },
            ),
            ("select", Binding::Button(2)),
            ("start", Binding::Button(3)),
            ("a", Binding::Button(0)),
            ("b", Binding::Button(1)),
        ]
        .into_iter()
        .map(|(name, binding)| (name.to_owned(), binding))
        .collect()
    }

    #[test]
    fn input_fragment_uses_pinned_j_codes() {
        let text = input_fragment(1, 0, &mappings()).unwrap();
        assert!(text.starts_with("[nespad1j]\nUp = j0a2\nDown = j0a3\n"));
        assert!(text.contains("A = j0b0\n"));
        assert!(text.ends_with("B = j0b1\n"));
    }

    #[test]
    fn controller_fragment_selects_standard_ports() {
        assert_eq!(
            controller_fragment(&[1, 3]).unwrap(),
            "[nestopia]\nport1 = 1\nport3 = 1\n"
        );
        assert!(controller_fragment(&[1, 1]).is_err());
        assert!(controller_fragment(&[5]).is_err());
    }

    #[test]
    fn standard_controls_cover_the_native_profile() {
        assert_eq!(STANDARD_CONTROLS.len(), 8);
        for target in STANDARD_CONTROLS {
            assert!(CONTROLS.iter().any(|(id, _)| *id == target));
        }
    }

    #[test]
    fn malformed_or_ambiguous_mappings_fail_closed() {
        assert!(input_fragment(1, 10, &mappings()).is_err());
        let mut duplicate = mappings();
        duplicate.insert("turbo_a".into(), Binding::Button(0));
        assert!(input_fragment(1, 0, &duplicate).is_err());
        assert!(
            Binding::Hat {
                index: 0,
                direction: 4
            }
            .code(0)
            .is_err()
        );
    }
}

/// Standard NES controls covered by the native profile. TurboA/TurboB are
/// writer-supported but outside the profile; the session maps only these.
pub(crate) const STANDARD_CONTROLS: [&str; 8] =
    ["up", "down", "left", "right", "select", "start", "a", "b"];

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
                "Nestopia setup needs an emulator identity"
            );
            for path in [&self.content, &self.probe_program, &self.sdl_library] {
                ensure!(
                    path.is_absolute()
                        && !path
                            .components()
                            .any(|part| matches!(part, std::path::Component::ParentDir)),
                    "Nestopia setup paths must be absolute without parent traversal"
                );
            }
            ensure!(
                self.executable_sha256.len() == 64
                    && self
                        .executable_sha256
                        .bytes()
                        .all(|byte| byte.is_ascii_hexdigit()),
                "Nestopia setup needs a trusted executable SHA-256"
            );
            let limit = catalog()
                .emulator_profiles
                .iter()
                .find(|profile| profile.id == PROFILE_ID)
                .and_then(|profile| profile.native_launch.as_ref())
                .context("Missing Nestopia native profile")?
                .max_players;
            ensure!(
                !self.players.is_empty() && self.players.len() <= limit,
                "Nestopia setup needs one or two players"
            );
            let mut controllers = BTreeSet::new();
            for (index, player) in self.players.iter().enumerate() {
                ensure!(
                    usize::from(player.player) == index + 1
                        && !player.controller_id.trim().is_empty()
                        && controllers.insert(&player.controller_id),
                    "Nestopia players must be distinct, contiguous, and start at player one"
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
                .context("Missing Nestopia native profile")?;
            let mut players = Vec::new();
            for player in &self.players {
                let calibration = calibrations
                    .get(&player.controller_id)
                    .context("Nestopia controller has no saved calibration")?;
                ensure!(
                    ["linux", "macos", "windows"].contains(&calibration.os.as_str()),
                    "Nestopia mapping requires Linux physical calibration"
                );
                let mapping = calibration.plan_profile(profile)?;
                ensure!(
                    mapping.rows.iter().all(|row| row.physical_id.is_some()
                        && row
                            .input
                            .as_ref()
                            .is_some_and(|input| input.native.is_some())),
                    "Nestopia needs native calibration for every NES control"
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
                "profile_id": PROFILE_ID,
                "players": players,
                "launch_ready": false,
                "launch_integration": "partial",
                "detail": "Native Linux launch writes a private nestopia.conf/input.conf pair, then rechecks the exact SDL2 routes. Only standard NES pads on ports one/two are supported; runtime behavior remains unverified."
            }))
        }
    }

    pub(crate) fn validate_setups(setups: &[SavedSetup]) -> Result<()> {
        ensure!(setups.len() <= 1024, "Too many Nestopia saved setups");
        let mut identities = BTreeSet::new();
        for setup in setups {
            setup.validate()?;
            ensure!(
                identities.insert((&setup.emulator_id, &setup.content)),
                "Duplicate Nestopia emulator/content setup"
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
            serde_json::from_slice(&output).context("Invalid Nestopia SDL2 capture")?;
        ensure!(
            snapshot.version[0] == 2
                && snapshot.library.canonicalize()? == setup.sdl_library.canonicalize()?
                && snapshot.library_sha256 == file_hash(&setup.sdl_library)?,
            "Nestopia helper inspected a different SDL2 runtime"
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

    /// SDL hat bitmask to the fltkui 0-3 hat code (`set_inputdef` order:
    /// UP/DOWN/LEFT/RIGHT).
    pub(crate) fn hat_code(direction: u8) -> Result<u8> {
        Ok(match direction {
            0x01 => 0,
            0x04 => 1,
            0x08 => 2,
            0x02 => 3,
            _ => anyhow::bail!("Nestopia hat direction is not cardinal"),
        })
    }

    fn player_bindings(
        calibration: &Calibration,
        device: &Device,
    ) -> Result<BTreeMap<String, Binding>> {
        let profile = crate::controller_catalog::catalog()
            .emulator_profiles
            .iter()
            .find(|profile| profile.id == PROFILE_ID)
            .context("Missing Nestopia native profile")?;
        let physical = PhysicalMap::from_device(device)?;
        let state = device
            .sampled_state
            .as_ref()
            .context("Nestopia SDL released state is missing")?;
        state.validate(
            device
                .controls
                .as_ref()
                .context("Nestopia SDL control counts are missing")?,
        )?;
        let mut result = BTreeMap::new();
        for row in calibration.plan_profile(profile)?.rows {
            ensure!(
                STANDARD_CONTROLS.contains(&row.target_id.as_str()),
                "Nestopia turbo controls are outside the native profile"
            );
            let input = row
                .input
                .as_ref()
                .context("Nestopia NES control is not calibrated")?;
            let native = input
                .native
                .as_ref()
                .context("Nestopia requires measured native controls")?;
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
                "Release the Nestopia controls before launch preparation"
            );
            let binding = match translated {
                DigitalInput::Button(index) => Binding::Button(
                    u8::try_from(index).context("Nestopia button index is too large")?,
                ),
                DigitalInput::Hat { index, direction } => Binding::Hat {
                    index: u8::try_from(index).context("Nestopia hat index is too large")?,
                    direction: hat_code(direction)?,
                },
                DigitalInput::Axis { index, .. } => Binding::Axis {
                    index: u8::try_from(index).context("Nestopia axis index is too large")?,
                    positive: native.direction > 0,
                },
            };
            ensure!(
                result.insert(row.target_id, binding).is_none(),
                "Nestopia target control appears twice"
            );
        }
        ensure!(
            result.len() == STANDARD_CONTROLS.len(),
            "Nestopia native mapping is incomplete"
        );
        Ok(result)
    }

    pub(crate) struct PreparedSession {
        directory: tempfile::TempDir,
        config_home: PathBuf,
        physical_paths: Vec<String>,
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
            cancel: &AtomicBool,
        ) -> Result<Self> {
            cancelled(cancel)?;
            setup.review(calibrations)?;
            ensure!(
                fs::symlink_metadata(&setup.content)?.file_type().is_file()
                    && setup.content.canonicalize()? == setup.content,
                "Nestopia content must be a direct regular file with canonical ancestry"
            );
            let mut selected = Vec::new();
            for player in &setup.players {
                let found = inventory
                    .iter()
                    .filter(|device| device.stable_id == player.controller_id)
                    .collect::<Vec<_>>();
                ensure!(
                    found.len() == 1 && !found[0].is_virtual,
                    "Nestopia physical controller is missing or ambiguous"
                );
                selected.push(found[0].device_path.clone());
            }
            #[cfg(target_os = "linux")]
            let topology = InputTopology::capture(&selected)?;
            let initial = routing(observe(setup, None, cancel)?);
            let mut physical_paths = Vec::new();
            let mut fragments = Vec::new();
            let mut ports = Vec::new();
            let mut device_indices = Vec::new();
            for (player, selected_path) in setup.players.iter().zip(&selected) {
                // Linux resolves through the sysfs topology; other hosts
                // match the SDL device-interface path and require uniqueness.
                #[cfg(target_os = "linux")]
                let path = topology.resolve_runtime_path(
                    selected_path,
                    initial
                        .devices
                        .iter()
                        .filter_map(|device| device.path.as_deref()),
                )?;
                #[cfg(not(target_os = "linux"))]
                let path = {
                    let selected_string = selected_path.to_string_lossy().into_owned();
                    let candidates = initial
                        .devices
                        .iter()
                        .filter(|device| device.path.as_deref() == Some(selected_string.as_str()))
                        .collect::<Vec<_>>();
                    ensure!(
                        candidates.len() == 1,
                        "Nestopia physical controller is missing or ambiguous in SDL"
                    );
                    selected_string
                };
                ensure!(
                    !physical_paths.contains(&path),
                    "Nestopia players share a controller"
                );
                let captured = observe(setup, Some(&path), cancel)?;
                initial.ensure_same_routing(&routing(captured.clone()))?;
                #[cfg(target_os = "linux")]
                topology.verify()?;
                let device = captured.device_at_path(&path)?;
                #[cfg(not(target_os = "linux"))]
                platform::require_unique_device_path(
                    &captured.devices,
                    &path,
                    device.device_index,
                )?;
                // The j-port prefix is the player index Nestopia assigns in
                // SDL open order (`SDL_JoystickSetPlayerIndex(joystick[i],
                // i)` on SDL_JOYDEVICEADDED); the probe's device_index is
                // that same enumeration order. Hotplug churn can reassign
                // it, so the routing — and the index below — are rechecked
                // immediately before launch.
                ensure!(
                    device.device_index <= 9,
                    "Nestopia SDL player index is outside the j-port grammar"
                );
                let host_player = device.device_index as u8;
                let bindings = player_bindings(
                    calibrations
                        .get(&player.controller_id)
                        .context("Nestopia calibration disappeared")?,
                    device,
                )?;
                fragments.push(input_fragment(player.player, host_player, &bindings)?);
                ports.push(player.player);
                device_indices.push(device.device_index);
                physical_paths.push(path);
            }
            let nestopia_conf = controller_fragment(&ports)?;
            let config_home = tempfile::Builder::new()
                .prefix("lunchbox-nestopia-config-")
                .tempdir()?;
            // SettingManager resolves $XDG_CONFIG_HOME/nestopia and keeps
            // defaults for absent keys, so partial files are safe.
            let config_dir = config_home.path().join("nestopia");
            fs::create_dir(&config_dir)?;
            fs::write(config_dir.join("nestopia.conf"), nestopia_conf)?;
            let mut input_conf = String::new();
            for fragment in &fragments {
                input_conf.push_str(fragment);
            }
            fs::write(config_dir.join("input.conf"), input_conf)?;
            let mut hashes = BTreeMap::new();
            for path in [
                &setup.content,
                &setup.probe_program,
                &setup.sdl_library,
                &config_dir.join("nestopia.conf"),
                &config_dir.join("input.conf"),
            ] {
                hashes.insert(path.clone(), file_hash(path)?);
            }
            let prepared = Self {
                directory: config_home,
                config_home: config_dir,
                physical_paths,
                device_indices,
                #[cfg(target_os = "linux")]
                topology,
                initial,
                setup: setup.clone(),
                hashes,
            };
            prepared.verify(cancel)?;
            Ok(prepared)
        }

        /// Private config root for `XDG_CONFIG_HOME`; SettingManager
        /// appends `/nestopia` itself.
        pub(crate) fn xdg_config_home(&self) -> &std::path::Path {
            self.directory.path()
        }

        pub(crate) fn verify(&self, cancel: &AtomicBool) -> Result<()> {
            cancelled(cancel)?;
            #[cfg(target_os = "linux")]
            self.topology.verify()?;
            for (path, hash) in &self.hashes {
                ensure!(file_hash(path)? == *hash, "Nestopia launch input changed");
            }
            let fresh = routing(observe(&self.setup, None, cancel)?);
            self.initial.ensure_same_routing(&fresh)?;
            for ((path, expected), index) in self
                .physical_paths
                .iter()
                .zip(self.setup.players.iter())
                .zip(&self.device_indices)
            {
                let captured = observe(&self.setup, Some(path), cancel)?;
                self.initial
                    .ensure_same_routing(&routing(captured.clone()))?;
                let position = self
                    .initial
                    .devices
                    .iter()
                    .position(|other| other.path.as_deref() == Some(path.as_str()))
                    .context("Nestopia device left SDL enumeration")?;
                ensure!(
                    captured
                        .devices
                        .iter()
                        .position(|other| other.path.as_deref() == Some(path.as_str()))
                        == Some(position),
                    "Nestopia SDL enumeration order moved for player {}",
                    expected.player
                );
                let device = captured.device_at_path(path)?;
                ensure!(
                    device.device_index == *index,
                    "Nestopia SDL player index moved before launch"
                );
                #[cfg(not(target_os = "linux"))]
                platform::require_unique_device_path(&captured.devices, path, *index)?;
                #[cfg(target_os = "linux")]
                self.topology.verify()?;
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
            for (path, index) in self.physical_paths.iter().zip(&self.device_indices) {
                platform::require_unique_device_path(&fresh.devices, path, *index)?;
            }
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
                "Nestopia executable differs from the saved trusted runtime"
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
                "Nestopia launch plan changed after preparation"
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
                    "Nestopia exited before controller handoff"
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
                    "Nestopia did not open the selected SDL controllers before timeout"
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
            anyhow::bail!("Nestopia calibrated launch requires a native build");
        };
        ensure!(
            option.emulator_name.eq_ignore_ascii_case("Nestopia UE")
                && setup.emulator_id == option.emulator_id
                && original.environment.is_empty()
                && original.retroarch_content.is_none(),
            "Nestopia identity differs or custom environment needs resolution"
        );
        let executable = executable.canonicalize()?;
        ensure!(
            executable == original.program.canonicalize()?,
            "Nestopia launch executable differs from selection"
        );
        ensure!(
            original.arguments.as_slice() == [setup.content.as_os_str()],
            "Nestopia calibrated launch requires exactly the saved content argument"
        );
        ensure!(
            file_hash(&executable)?.eq_ignore_ascii_case(&setup.executable_sha256),
            "Nestopia executable differs from the saved trusted runtime"
        );
        let inputs = session::PreparedSession::prepare(setup, calibrations, inventory, cancel)?;
        let mut plan = original.clone();
        plan.program = executable.clone();
        // SettingManager resolves $XDG_CONFIG_HOME/nestopia.
        plan.environment.push((
            std::ffi::OsString::from("XDG_CONFIG_HOME"),
            inputs.xdg_config_home().as_os_str().to_owned(),
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

#[cfg(all(test, target_os = "linux"))]
mod session_tests {
    use super::session::hat_code;

    #[test]
    fn hat_codes_follow_capture_order() {
        assert_eq!(hat_code(0x01).unwrap(), 0);
        assert_eq!(hat_code(0x04).unwrap(), 1);
        assert_eq!(hat_code(0x08).unwrap(), 2);
        assert_eq!(hat_code(0x02).unwrap(), 3);
        assert!(hat_code(0x03).is_err());
        assert!(hat_code(0x00).is_err());
    }
}
