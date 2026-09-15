//! LinApple native SDL joystick configuration writer.
//!
//! Pinned source: `linappleii/linapple` commit
//! `fa31e11b579edec32dd431c8b400a04e60a21dab`.  LinApple's SDL frontend
//! consumes the `linapple.conf` registry keys `Joystick 0/1`, index, button,
//! and axis values.  This module patches those source-defined keys in a
//! copied configuration; SDL enumeration remains a runtime slot contract and
//! must be probed and rechecked by the launch caller.

use anyhow::{Context, Result, ensure};
use serde::{Deserialize, Serialize};
use std::collections::BTreeSet;

pub(crate) const SOURCE_COMMIT: &str = "fa31e11b579edec32dd431c8b400a04e60a21dab";
pub(crate) const PROFILE_ID: &str = "linapple:standalone-linapple-joystick";

/// Values from the source's `joyinfo` table in JoystickFrontend.cpp.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
#[repr(u32)]
pub(crate) enum JoystickMode {
    Disabled = 0,
    HostJoystick = 1,
    KeyboardStandard = 2,
    KeyboardCentering = 3,
    Mouse = 4,
}

impl JoystickMode {
    fn value(self) -> u32 {
        self as u32
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) struct JoystickSelection {
    pub mode: JoystickMode,
    /// SDL enumeration index observed immediately before launch. LinApple
    /// has no persisted GUID/name selector, so this is deliberately not a
    /// stable identity and is only valid with same-launch revalidation.
    pub runtime_sdl_index: Option<u32>,
    pub button_1: u32,
    pub button_2: u32,
    pub axis_x: u32,
    pub axis_y: u32,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) struct InputProfile {
    pub joysticks: [JoystickSelection; 2],
    pub exit_enabled: bool,
    pub exit_buttons: [u32; 2],
}

const FIELDS: [&str; 14] = [
    "Joystick 0",
    "Joystick 1",
    "Joystick 0 Index",
    "Joystick 1 Index",
    "Joystick 0 Button 1",
    "Joystick 0 Button 2",
    "Joystick 1 Button 1",
    "Joystick 0 Axis 0",
    "Joystick 0 Axis 1",
    "Joystick 1 Axis 0",
    "Joystick 1 Axis 1",
    "Joystick Exit Enable",
    "Joystick Exit Button 0",
    "Joystick Exit Button 1",
];

fn values(profile: InputProfile) -> Result<[String; FIELDS.len()]> {
    let mut indices = BTreeSet::new();
    for selection in profile.joysticks {
        if let Some(index) = selection.runtime_sdl_index {
            ensure!(
                indices.insert(index),
                "LinApple SDL joystick index is duplicated"
            );
        } else {
            ensure!(
                !matches!(selection.mode, JoystickMode::HostJoystick),
                "LinApple host joystick mode needs a probed SDL index"
            );
        }
        ensure!(
            selection.button_1 <= 255
                && selection.button_2 <= 255
                && selection.axis_x <= 255
                && selection.axis_y <= 255,
            "LinApple SDL button and axis indices must fit the source uint8-compatible range"
        );
    }
    ensure!(profile.exit_buttons.iter().all(|button| *button <= 255));
    Ok([
        profile.joysticks[0].mode.value().to_string(),
        profile.joysticks[1].mode.value().to_string(),
        profile.joysticks[0]
            .runtime_sdl_index
            .unwrap_or_default()
            .to_string(),
        profile.joysticks[1]
            .runtime_sdl_index
            .unwrap_or_default()
            .to_string(),
        profile.joysticks[0].button_1.to_string(),
        profile.joysticks[0].button_2.to_string(),
        profile.joysticks[1].button_1.to_string(),
        profile.joysticks[0].axis_x.to_string(),
        profile.joysticks[0].axis_y.to_string(),
        profile.joysticks[1].axis_x.to_string(),
        profile.joysticks[1].axis_y.to_string(),
        u32::from(profile.exit_enabled).to_string(),
        profile.exit_buttons[0].to_string(),
        profile.exit_buttons[1].to_string(),
    ])
}

/// Patch only LinApple's source-defined input keys in a complete copied
/// `linapple.conf`. Unknown settings, comments, ordering, and line endings
/// are retained. Missing or duplicate controller keys fail closed rather than
/// relying on the executable's defaults or section fallback behavior.
pub(crate) fn patch_config(baseline: &[u8], profile: InputProfile) -> Result<String> {
    ensure!(
        baseline.len() <= 4 * 1024 * 1024,
        "LinApple config is too large"
    );
    let original = std::str::from_utf8(baseline).context("LinApple config is not UTF-8")?;
    ensure!(
        !original.contains('\0'),
        "LinApple config contains a NUL byte"
    );
    let values = values(profile)?;
    let newline = if original.contains("\r\n") {
        "\r\n"
    } else {
        "\n"
    };
    let mut output = String::with_capacity(original.len() + 256);
    let mut seen = [false; FIELDS.len()];

    for line in original.split_inclusive('\n') {
        let (content, ending) = line.strip_suffix('\n').map_or((line, ""), |line| {
            line.strip_suffix('\r')
                .map_or((line, "\n"), |line| (line, "\r\n"))
        });
        let key = content
            .split_once('=')
            .map(|(key, _)| key.trim())
            .and_then(|key| {
                FIELDS
                    .iter()
                    .position(|known| known.eq_ignore_ascii_case(key))
            });
        if let Some(index) = key {
            ensure!(
                !seen[index],
                "LinApple config contains duplicate {} entry",
                FIELDS[index]
            );
            output.push_str(FIELDS[index]);
            output.push_str(" = ");
            output.push_str(&values[index]);
            output.push_str(if ending.is_empty() { newline } else { ending });
            seen[index] = true;
        } else {
            output.push_str(line);
        }
    }
    ensure!(
        seen.iter().all(|present| *present),
        "LinApple config is missing a source-defined controller field"
    );
    Ok(output)
}

pub(crate) fn source_boundary() -> &'static str {
    "LinApple reads controller modes, SDL enumeration indices, button indices and axis indices from linapple.conf; SDL device identity is runtime enumeration only. Probe and recheck the selected SDL slots immediately before launch, and pass a private copied configuration."
}

#[cfg(test)]
mod tests {
    use super::*;

    fn profile() -> InputProfile {
        InputProfile {
            joysticks: [
                JoystickSelection {
                    mode: JoystickMode::HostJoystick,
                    runtime_sdl_index: Some(2),
                    button_1: 0,
                    button_2: 1,
                    axis_x: 0,
                    axis_y: 1,
                },
                JoystickSelection {
                    mode: JoystickMode::Disabled,
                    runtime_sdl_index: None,
                    button_1: 0,
                    button_2: 0,
                    axis_x: 0,
                    axis_y: 1,
                },
            ],
            exit_enabled: true,
            exit_buttons: [8, 9],
        }
    }

    fn baseline(newline: &str) -> String {
        let mut text = format!("# keep{newline}[Configuration]{newline}Other = retained{newline}");
        for field in FIELDS {
            text.push_str(&format!("{field} = 0{newline}"));
        }
        text.push_str(&format!("[Preferences]{newline}untouched = yes{newline}"));
        text
    }

    #[test]
    fn patches_source_fields_and_preserves_unknown_config() {
        let output = patch_config(baseline("\r\n").as_bytes(), profile()).unwrap();
        assert!(output.contains("Joystick 0 = 1\r\n"));
        assert!(output.contains("Joystick 0 Index = 2\r\n"));
        assert!(output.contains("Joystick 0 Button 2 = 1\r\n"));
        assert!(output.contains("Joystick Exit Enable = 1\r\n"));
        assert!(output.contains("Other = retained\r\n"));
        assert!(output.contains("untouched = yes\r\n"));
    }

    #[test]
    fn refuses_unprobed_or_duplicate_slots_and_incomplete_baselines() {
        let mut invalid = profile();
        invalid.joysticks[0].runtime_sdl_index = None;
        assert!(patch_config(baseline("\n").as_bytes(), invalid).is_err());

        invalid = profile();
        invalid.joysticks[1].mode = JoystickMode::Mouse;
        invalid.joysticks[1].runtime_sdl_index = Some(2);
        assert!(patch_config(baseline("\n").as_bytes(), invalid).is_err());

        let incomplete = baseline("\n").replace("Joystick 1 Axis 1 = 0\n", "");
        assert!(patch_config(incomplete.as_bytes(), profile()).is_err());
    }
}

/// Layout target ids covered by the native profile.
pub(crate) const ROUTES: [(&str, &str); 6] = [
    ("up", "Up"),
    ("down", "Down"),
    ("left", "Left"),
    ("right", "Right"),
    ("fire", "Button 1"),
    ("fire2", "Button 2"),
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
                "LinApple setup needs an emulator identity"
            );
            for path in [&self.content, &self.probe_program, &self.sdl_library] {
                ensure!(
                    path.is_absolute()
                        && !path
                            .components()
                            .any(|part| matches!(part, std::path::Component::ParentDir)),
                    "LinApple setup paths must be absolute without parent traversal"
                );
            }
            ensure!(
                self.executable_sha256.len() == 64
                    && self
                        .executable_sha256
                        .bytes()
                        .all(|byte| byte.is_ascii_hexdigit()),
                "LinApple setup needs a trusted executable SHA-256"
            );
            let limit = catalog()
                .emulator_profiles
                .iter()
                .find(|profile| profile.id == PROFILE_ID)
                .and_then(|profile| profile.native_launch.as_ref())
                .context("Missing LinApple native profile")?
                .max_players;
            ensure!(
                !self.players.is_empty() && self.players.len() <= limit,
                "LinApple setup needs one or two players"
            );
            let mut controllers = BTreeSet::new();
            for (index, player) in self.players.iter().enumerate() {
                ensure!(
                    usize::from(player.player) == index + 1
                        && !player.controller_id.trim().is_empty()
                        && controllers.insert(&player.controller_id),
                    "LinApple players must be distinct, contiguous, and start at player one"
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
                .context("Missing LinApple native profile")?;
            let mut players = Vec::new();
            for player in &self.players {
                let calibration = calibrations
                    .get(&player.controller_id)
                    .context("LinApple controller has no saved calibration")?;
                ensure!(
                    ["linux", "macos", "windows"].contains(&calibration.os.as_str()),
                    "LinApple mapping requires Linux physical calibration"
                );
                let mapping = calibration.plan_profile(profile)?;
                ensure!(
                    mapping.rows.iter().all(|row| row.physical_id.is_some()
                        && row
                            .input
                            .as_ref()
                            .is_some_and(|input| input.native.is_some())),
                    "LinApple needs native calibration for every joystick control"
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
                "detail": "Native launch patches a private linapple.conf, then rechecks the exact SDL routes. Only analog-axis directions plus two buttons on ports one/two are supported; runtime behavior remains unverified."
            }))
        }
    }

    pub(crate) fn validate_setups(setups: &[SavedSetup]) -> Result<()> {
        ensure!(setups.len() <= 1024, "Too many LinApple saved setups");
        let mut identities = BTreeSet::new();
        for setup in setups {
            setup.validate()?;
            ensure!(
                identities.insert((&setup.emulator_id, &setup.content)),
                "Duplicate LinApple emulator/content setup"
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
            serde_json::from_slice(&output).context("Invalid LinApple SDL2 capture")?;
        ensure!(
            snapshot.version[0] == 2
                && snapshot.library.canonicalize()? == setup.sdl_library.canonicalize()?
                && snapshot.library_sha256 == file_hash(&setup.sdl_library)?,
            "LinApple helper inspected a different SDL2 runtime"
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

    struct StickMapping {
        axis_x: u8,
        axis_y: u8,
        button_1: u8,
        button_2: u8,
    }

    fn stick_mapping(calibration: &Calibration, device: &Device) -> Result<StickMapping> {
        let profile = crate::controller_catalog::catalog()
            .emulator_profiles
            .iter()
            .find(|profile| profile.id == PROFILE_ID)
            .context("Missing LinApple native profile")?;
        let physical = PhysicalMap::from_device(device)?;
        let state = device
            .sampled_state
            .as_ref()
            .context("LinApple SDL released state is missing")?;
        state.validate(
            device
                .controls
                .as_ref()
                .context("LinApple SDL control counts are missing")?,
        )?;
        let mut translated = BTreeMap::new();
        for row in calibration.plan_profile(profile)?.rows {
            ensure!(
                ROUTES.iter().any(|(target, _)| *target == row.target_id),
                "LinApple target {} outside contract",
                row.target_id
            );
            let input = row
                .input
                .as_ref()
                .context("LinApple joystick control is not calibrated")?;
            let native = input
                .native
                .as_ref()
                .context("LinApple requires measured native controls")?;
            let measured = input.axis.as_ref().map(|axis| AxisEndpoints {
                released: axis.released,
                pressed: axis.pressed,
            });
            let item = physical.digital_input(native.code, measured)?;
            let released = match item {
                DigitalInput::Button(index) => state.buttons.get(&index) == Some(&false),
                DigitalInput::Hat { .. } => {
                    anyhow::bail!("LinApple analog axes cannot be driven by a hat")
                }
                DigitalInput::Axis {
                    index, released, ..
                } => state.axes.get(&index) == Some(&released),
            };
            ensure!(
                released,
                "Release the LinApple controls before launch preparation"
            );
            ensure!(
                translated.insert(row.target_id, item).is_none(),
                "LinApple target control appears twice"
            );
        }
        let mut get = |target: &str| {
            translated
                .remove(target)
                .with_context(|| format!("LinApple control {target} is not calibrated"))
        };
        // Directions must share one axis per pair with opposite polarity,
        // mirroring the Atari800 source-supported pair discipline; the
        // writer's Axis 0/1 fields accept only analog axes.
        let axis_pair = |negative: DigitalInput, positive: DigitalInput| match (negative, positive)
        {
            (
                DigitalInput::Axis {
                    index: first,
                    released: first_rest,
                    pressed: first_press,
                },
                DigitalInput::Axis {
                    index: second,
                    released: second_rest,
                    pressed: second_press,
                },
            ) if first == second
                && first_rest.abs() < 10_000
                && second_rest.abs() < 10_000
                && first_press < -10_000
                && second_press > 10_000 =>
            {
                Some(first)
            }
            _ => None,
        };
        let horizontal = axis_pair(get("left")?, get("right")?)
            .context("LinApple left/right must share one analog axis with opposite polarity")?;
        let vertical = axis_pair(get("up")?, get("down")?)
            .context("LinApple up/down must share one analog axis with opposite polarity")?;
        ensure!(
            horizontal != vertical,
            "LinApple direction pairs must use distinct axes"
        );
        let mut button = |target: &str| match get(target)? {
            DigitalInput::Button(index) => {
                u8::try_from(index).context("LinApple button index is too large")
            }
            other => anyhow::bail!("LinApple {target} must be a raw button, not {other:?}"),
        };
        Ok(StickMapping {
            axis_x: u8::try_from(horizontal).context("LinApple axis index is too large")?,
            axis_y: u8::try_from(vertical).context("LinApple axis index is too large")?,
            button_1: button("fire")?,
            button_2: button("fire2")?,
        })
    }

    pub(crate) struct PreparedSession {
        directory: tempfile::TempDir,
        config_path: PathBuf,
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
                "LinApple content must be a direct regular file with canonical ancestry"
            );
            let mut selected = Vec::new();
            for player in &setup.players {
                let found = inventory
                    .iter()
                    .filter(|device| device.stable_id == player.controller_id)
                    .collect::<Vec<_>>();
                ensure!(
                    found.len() == 1 && !found[0].is_virtual,
                    "LinApple physical controller is missing or ambiguous"
                );
                selected.push(found[0].device_path.clone());
            }
            #[cfg(target_os = "linux")]
            let topology = InputTopology::capture(&selected)?;
            let initial = routing(observe(setup, None, cancel)?);
            let mut physical_paths = Vec::new();
            let mut selections = Vec::new();
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
                        "linapple physical controller is missing or ambiguous in SDL"
                    );
                    selected_string
                };
                ensure!(
                    !physical_paths.contains(&path),
                    "LinApple players share a controller"
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
                let mapping = stick_mapping(
                    calibrations
                        .get(&player.controller_id)
                        .context("LinApple calibration disappeared")?,
                    device,
                )?;
                let sdl_index = device.device_index;
                selections.push(JoystickSelection {
                    mode: JoystickMode::HostJoystick,
                    runtime_sdl_index: Some(sdl_index),
                    button_1: u32::from(mapping.button_1),
                    button_2: u32::from(mapping.button_2),
                    axis_x: u32::from(mapping.axis_x),
                    axis_y: u32::from(mapping.axis_y),
                });
                device_indices.push(device.device_index);
                physical_paths.push(path);
            }
            let disabled = JoystickSelection {
                mode: JoystickMode::Disabled,
                runtime_sdl_index: None,
                button_1: 0,
                button_2: 0,
                axis_x: 0,
                axis_y: 1,
            };
            let mut joysticks = [disabled, disabled];
            for (index, selection) in selections.iter().enumerate() {
                joysticks[index] = *selection;
            }
            let profile = InputProfile {
                joysticks,
                exit_enabled: false,
                exit_buttons: [0, 0],
            };
            let directory = tempfile::Builder::new()
                .prefix("lunchbox-linapple-")
                .tempdir()?;
            let config_path = directory.path().join("linapple.conf");
            // A minimal baseline carries every source-defined controller
            // field; the writer preserves unknown settings verbatim.
            let baseline = FIELDS
                .iter()
                .map(|field| format!("{field} = 0\n"))
                .collect::<String>();
            fs::write(&config_path, patch_config(baseline.as_bytes(), profile)?)?;
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

        pub(crate) fn config_path(&self) -> &std::path::Path {
            &self.config_path
        }

        pub(crate) fn verify(&self, cancel: &AtomicBool) -> Result<()> {
            cancelled(cancel)?;
            #[cfg(target_os = "linux")]
            self.topology.verify()?;
            for (path, hash) in &self.hashes {
                ensure!(file_hash(path)? == *hash, "LinApple launch input changed");
            }
            let fresh = routing(observe(&self.setup, None, cancel)?);
            self.initial.ensure_same_routing(&fresh)?;
            for (path, index) in self.physical_paths.iter().zip(&self.device_indices) {
                let captured = observe(&self.setup, Some(path), cancel)?;
                self.initial
                    .ensure_same_routing(&routing(captured.clone()))?;
                let device = captured.device_at_path(path)?;
                ensure!(
                    device.device_index == *index,
                    "LinApple SDL joystick moved before launch"
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
    use crate::controller_native_platform as platform;
    #[cfg(target_os = "linux")]
    use crate::controller_native_process::native_pid;
    use crate::{
        controller_catalog::Calibration,
        controller_native_process::cancelled,
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
                "LinApple executable differs from the saved trusted runtime"
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
                "LinApple launch plan changed after preparation"
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
                    "LinApple exited before controller handoff"
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
                    "LinApple did not open the selected SDL controllers before timeout"
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
            anyhow::bail!("LinApple calibrated launch requires a native build");
        };
        ensure!(
            option.emulator_name.eq_ignore_ascii_case("LinApple")
                && setup.emulator_id == option.emulator_id
                && original.environment.is_empty()
                && original.retroarch_content.is_none(),
            "LinApple identity differs or custom environment needs resolution"
        );
        let executable = executable.canonicalize()?;
        ensure!(
            executable == original.program.canonicalize()?,
            "LinApple launch executable differs from selection"
        );
        ensure!(
            original.arguments.as_slice() == [setup.content.as_os_str()],
            "LinApple calibrated launch requires exactly the saved content argument"
        );
        ensure!(
            file_hash(&executable)?.eq_ignore_ascii_case(&setup.executable_sha256),
            "LinApple executable differs from the saved trusted runtime"
        );
        let inputs = session::PreparedSession::prepare(setup, calibrations, inventory, cancel)?;
        // The config search order is --config, then XDG, then CWD: the
        // explicit flag always wins, so a private file fully isolates input.
        let mut plan = original.clone();
        plan.program = executable.clone();
        plan.arguments = vec![
            std::ffi::OsString::from("--config"),
            inputs.config_path().as_os_str().to_owned(),
            std::ffi::OsString::from("-1"),
            setup.content.as_os_str().to_owned(),
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
