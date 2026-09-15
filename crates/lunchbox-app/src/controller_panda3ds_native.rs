//! Panda3DS native-input boundary.
//!
//! Pinned source: wheremyfoodat/Panda3DS commit
//! `5aaa1d26565c834a6f1999026260e559f54aacf1`.
//!
//! The native SDL frontend in the pinned Panda3DS source opens controller 0
//! directly with `SDL_GameControllerOpen(0)` and hard-wires standard buttons
//! and the left stick. `config.toml` contains keyboard mappings, but there is
//! no native gamepad mapping table to author. This module emits the exact
//! keyboard file and refuses only the native gamepad mapping claim.

use anyhow::{Context, Result, bail, ensure};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

pub(crate) const PROFILE_ID: &str = "panda3ds:standalone-panda3ds-3ds";
pub(crate) const SOURCE_COMMIT: &str = "5aaa1d26565c834a6f1999026260e559f54aacf1";

/// Panda's Qt/SDL frontends do have an authorable keyboard map. This is the
/// exact `InputMappings::serialize` shape used by `controls_qt.toml`; it does
/// not alter the hard-wired SDL GameController 0 path.
#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct KeyboardBinding {
    /// One of HID::Keys::keyToName outputs, for example `A` or `D-Pad Up`.
    pub control: String,
    /// Qt QKeySequence text accepted by the source deserializer, e.g. `L`.
    pub key: String,
}

pub(crate) fn keyboard_mapping_toml(bindings: &[KeyboardBinding]) -> Result<String> {
    ensure!(
        !bindings.is_empty(),
        "Panda3DS needs at least one keyboard binding"
    );
    let known = [
        "A",
        "B",
        "Select",
        "Start",
        "D-Pad Right",
        "D-Pad Left",
        "D-Pad Up",
        "D-Pad Down",
        "R",
        "L",
        "X",
        "Y",
        "ZL",
        "ZR",
        "CirclePad Right",
        "CirclePad Left",
        "CirclePad Up",
        "CirclePad Down",
    ];
    let mut grouped = BTreeMap::<String, Vec<String>>::new();
    for binding in bindings {
        ensure!(
            known.contains(&binding.control.as_str()),
            "Panda3DS keyboard control is unknown"
        );
        ensure!(
            !binding.key.is_empty()
                && binding.key.len() <= 64
                && !binding.key.contains(['"', '\r', '\n']),
            "Panda3DS keyboard key is invalid"
        );
        grouped
            .entry(binding.control.clone())
            .or_default()
            .push(binding.key.clone());
    }
    let mut out = String::from(
        "[Metadata]\nName = \"Lunchbox\"\nDevice = \"Lunchbox\"\nFrontend = \"Qt\"\n\n[Mappings]\n",
    );
    for (control, keys) in grouped {
        if control
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || byte == b'_' || byte == b'-')
        {
            out.push_str(&control);
        } else {
            out.push('"');
            out.push_str(&control);
            out.push('"');
        }
        out.push_str(" = [");
        for (index, key) in keys.iter().enumerate() {
            if index != 0 {
                out.push_str(", ");
            }
            out.push('"');
            out.push_str(&key.replace('\\', "\\\\").replace('"', "\\\""));
            out.push('"');
        }
        out.push_str("]\n");
    }
    Ok(out)
}

pub(crate) fn native_mapping_unavailable() -> Result<()> {
    bail!(
        "Panda3DS native SDL gamepad input is not configurable: upstream opens only SDL gamepad 0 and hard-wires its standard controls; keyboard mappings can be authored in controls_qt.toml"
    )
}

pub(crate) fn launch_contract() -> &'static str {
    "Native Panda3DS may be launched only with a private working directory containing config.toml with General.UsePortableBuild=false; this isolates config while SDL_GetPrefPath keeps saves/app data persistent. No native gamepad mapping is staged."
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn native_path_is_explicitly_unsupported() {
        assert!(native_mapping_unavailable().is_err());
        assert!(launch_contract().contains("No native gamepad mapping"));
        let text = keyboard_mapping_toml(&[KeyboardBinding {
            control: "A".into(),
            key: "L".into(),
        }])
        .unwrap();
        assert!(text.contains("A = [\"L\"]"));
        let text = keyboard_mapping_toml(&[KeyboardBinding {
            control: "D-Pad Up".into(),
            key: "Up".into(),
        }])
        .unwrap();
        assert!(text.contains("\"D-Pad Up\" = [\"Up\"]"));
    }
}

/// Registered catalog profile id (`{core}:standalone-{layout}` convention).

/// Layout target ids covered by the native profile: the 14 hard-wired
/// buttons, both triggers, and both analog pairs.
pub(crate) const ROUTES: [(&str, &str); 22] = [
    ("a", "A"),
    ("b", "B"),
    ("x", "X"),
    ("y", "Y"),
    ("up", "Up"),
    ("down", "Down"),
    ("left", "Left"),
    ("right", "Right"),
    ("l", "L"),
    ("r", "R"),
    ("start", "Start"),
    ("select", "Select"),
    ("zl", "ZL"),
    ("zr", "ZR"),
    ("stick_up", "Circle pad up"),
    ("stick_down", "Circle pad down"),
    ("stick_left", "Circle pad left"),
    ("stick_right", "Circle pad right"),
    ("right_stick_up", "C-stick up"),
    ("right_stick_down", "C-stick down"),
    ("right_stick_left", "C-stick left"),
    ("right_stick_right", "C-stick right"),
];

/// Hard-wired SDL game-controller button each 3DS action reads. Note the
/// source's swapped face mapping: SDL A drives 3DS B and vice versa, and
/// SDL X drives 3DS Y and vice versa.
pub(crate) const ACTION_BUTTONS: [(&str, &str); 14] = [
    ("a", "b"),
    ("b", "a"),
    ("x", "y"),
    ("y", "x"),
    ("l", "leftshoulder"),
    ("r", "rightshoulder"),
    ("start", "start"),
    ("select", "back"),
    ("up", "dpup"),
    ("down", "dpdown"),
    ("left", "dpleft"),
    ("right", "dpright"),
    ("zl", "lefttrigger"),
    ("zr", "righttrigger"),
];

/// SDL game-controller axis outputs for the two sticks.
pub(crate) const STICK_AXES: [(&str, &str); 4] = [
    ("stick", "leftx"),
    ("stick_y", "lefty"),
    ("right_stick", "rightx"),
    ("right_stick_y", "righty"),
];

/// Minimal private `config.toml`: only the `[General]` portable flag.
/// `getConfigPath` prefers `./config.toml`, so this fully isolates input
/// while saves stay in the SDL preference root.
pub(crate) fn session_config() -> &'static str {
    "[General]\nUsePortableBuild = false\n"
}

pub(crate) mod settings {
    use super::*;
    use crate::controller_catalog::{Calibration, catalog};
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
                "Panda3DS setup needs an emulator identity"
            );
            for path in [&self.content, &self.probe_program, &self.sdl_library] {
                ensure!(
                    path.is_absolute()
                        && !path
                            .components()
                            .any(|part| matches!(part, std::path::Component::ParentDir)),
                    "Panda3DS setup paths must be absolute without parent traversal"
                );
            }
            ensure!(
                self.executable_sha256.len() == 64
                    && self
                        .executable_sha256
                        .bytes()
                        .all(|byte| byte.is_ascii_hexdigit()),
                "Panda3DS setup needs a trusted executable SHA-256"
            );
            let limit = catalog()
                .emulator_profiles
                .iter()
                .find(|profile| profile.id == PROFILE_ID)
                .and_then(|profile| profile.native_launch.as_ref())
                .context("Missing Panda3DS native profile")?
                .max_players;
            ensure!(
                self.players.len() == 1 && self.players[0].player == 1,
                "Panda3DS supports exactly player one"
            );
            ensure!(
                self.players.len() <= limit && !self.players[0].controller_id.trim().is_empty(),
                "Panda3DS player needs a saved controller identity"
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
                .context("Missing Panda3DS native profile")?;
            let player = &self.players[0];
            let calibration = calibrations
                .get(&player.controller_id)
                .context("Panda3DS controller has no saved calibration")?;
            ensure!(
                ["linux", "macos", "windows"].contains(&calibration.os.as_str()),
                "Panda3DS mapping requires Linux physical calibration"
            );
            let mapping = calibration.plan_profile(profile)?;
            ensure!(
                mapping.rows.iter().all(|row| row.physical_id.is_some()
                    && row
                        .input
                        .as_ref()
                        .is_some_and(|input| input.native.is_some())),
                "Panda3DS needs native calibration for every 3DS control"
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
                "detail": "Native launch runs in a session directory with a private config.toml, then rechecks the exact SDL routes on Linux, Windows, and macOS. Ownership is strongest on Linux; other hosts pin the executable plus a fresh device re-probe. Only the single 3DS pad at SDL index 0 with the hard-wired standard mapping is supported; runtime behavior remains unverified."
            }))
        }
    }

    pub(crate) fn validate_setups(setups: &[SavedSetup]) -> Result<()> {
        ensure!(setups.len() <= 1024, "Too many Panda3DS saved setups");
        let mut identities = std::collections::BTreeSet::new();
        for setup in setups {
            setup.validate()?;
            ensure!(
                identities.insert((&setup.emulator_id, &setup.content)),
                "Duplicate Panda3DS emulator/content setup"
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
            serde_json::from_slice(&output).context("Invalid Panda3DS SDL2 capture")?;
        ensure!(
            snapshot.version[0] == 2
                && snapshot.library.canonicalize()? == setup.sdl_library.canonicalize()?
                && snapshot.library_sha256 == file_hash(&setup.sdl_library)?,
            "Panda3DS helper inspected a different SDL2 runtime"
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

    fn mapping_fields(mapping: &str) -> BTreeMap<String, String> {
        let mut fields = BTreeMap::new();
        for entry in mapping.split(',').skip(2) {
            if let Some((key, value)) = entry.split_once(':') {
                fields.insert(key.trim().to_owned(), value.trim().to_owned());
            }
        }
        fields
    }

    /// Raw SDL2 control to game-controller output name through the pad's
    /// effective mapping. Buttons and hats resolve through button entries;
    /// axis halves resolve through full-axis stick or trigger entries.
    fn gamepad_output(
        fields: &BTreeMap<String, String>,
        translated: DigitalInput,
    ) -> Result<String> {
        const BUTTON_OUTPUTS: [&str; 15] = [
            "a",
            "b",
            "x",
            "y",
            "back",
            "guide",
            "start",
            "leftstick",
            "rightstick",
            "leftshoulder",
            "rightshoulder",
            "dpup",
            "dpdown",
            "dpleft",
            "dpright",
        ];
        const AXIS_OUTPUTS: [&str; 6] = [
            "leftx",
            "lefty",
            "rightx",
            "righty",
            "lefttrigger",
            "righttrigger",
        ];
        Ok(match translated {
            DigitalInput::Button(index) => {
                let raw = u32::try_from(index).context("Panda3DS button is too large")?;
                let mut found = None;
                for output in BUTTON_OUTPUTS {
                    if let Some(input) = fields.get(output) {
                        let candidate: u32 = input
                            .trim_end_matches('~')
                            .strip_prefix('b')
                            .context("Panda3DS mapping entry is not a button")?
                            .parse()
                            .context("Panda3DS mapping button is invalid")?;
                        if candidate == raw {
                            found = Some(output);
                            break;
                        }
                    }
                }
                found.context("Panda3DS raw button is unmapped")?.to_owned()
            }
            DigitalInput::Hat { direction, .. } => {
                let want = match direction {
                    0x01 => "dpup",
                    0x04 => "dpdown",
                    0x08 => "dpleft",
                    0x02 => "dpright",
                    _ => anyhow::bail!("Panda3DS hat direction is not cardinal"),
                };
                let input = fields
                    .get(want)
                    .with_context(|| format!("Panda3DS dpad entry {want} is unmapped"))?;
                ensure!(
                    input.trim_end_matches('~').starts_with('h'),
                    "Panda3DS dpad entry is not a hat"
                );
                want.to_owned()
            }
            DigitalInput::Axis { index, .. } => {
                let raw = u32::try_from(index).context("Panda3DS axis is too large")?;
                let mut found = None;
                for output in AXIS_OUTPUTS {
                    if let Some(input) = fields.get(output) {
                        let candidate: u32 = input
                            .trim_start_matches(['+', '-'])
                            .trim_end_matches('~')
                            .strip_prefix('a')
                            .context("Panda3DS stick entry is not an axis")?
                            .parse()
                            .context("Panda3DS stick axis is invalid")?;
                        if candidate == raw {
                            found = Some(output);
                            break;
                        }
                    }
                }
                found.context("Panda3DS raw axis is unmapped")?.to_owned()
            }
        })
    }

    pub(crate) struct PreparedSession {
        directory: tempfile::TempDir,
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
                "Panda3DS content must be a direct regular file with canonical ancestry"
            );
            let player = &setup.players[0];
            let found = inventory
                .iter()
                .filter(|device| device.stable_id == player.controller_id)
                .collect::<Vec<_>>();
            ensure!(
                found.len() == 1 && !found[0].is_virtual,
                "Panda3DS physical controller is missing or ambiguous"
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
                    "panda3ds physical controller is missing or ambiguous in SDL"
                );
                selected_string
            };
            let captured = observe(setup, Some(&physical_path), cancel)?;
            initial.ensure_same_routing(&routing(captured.clone()))?;
            #[cfg(target_os = "linux")]
            topology.verify()?;
            let device = captured.device_at_path(&physical_path)?;
            // SDL_GameControllerOpen(0): the pad must be SDL index 0.
            ensure!(
                device.device_index == 0,
                "Panda3DS opens SDL controller 0; selected pad is index {}",
                device.device_index
            );
            let fields = mapping_fields(
                device
                    .mapping
                    .as_deref()
                    .context("Panda3DS SDL mapping is absent")?,
            );
            let calibration = calibrations
                .get(&player.controller_id)
                .context("Panda3DS calibration disappeared")?;
            let profile = crate::controller_catalog::catalog()
                .emulator_profiles
                .iter()
                .find(|profile| profile.id == PROFILE_ID)
                .context("Missing Panda3DS native profile")?;
            let physical = PhysicalMap::from_device(device)?;
            let state = device
                .sampled_state
                .as_ref()
                .context("Panda3DS SDL released state is missing")?;
            state.validate(
                device
                    .controls
                    .as_ref()
                    .context("Panda3DS SDL control counts are missing")?,
            )?;
            // Every calibrated control must resolve to the exact
            // game-controller element the source hard-wires for its action;
            // anything else would silently drive the wrong 3DS input.
            let mut axes: BTreeMap<String, (&str, bool)> = BTreeMap::new();
            for row in calibration.plan_profile(profile)?.rows {
                ensure!(
                    ROUTES.iter().any(|(target, _)| *target == row.target_id),
                    "Panda3DS target {} outside contract",
                    row.target_id
                );
                let input = row
                    .input
                    .as_ref()
                    .context("Panda3DS 3DS control is not calibrated")?;
                let native = input
                    .native
                    .as_ref()
                    .context("Panda3DS requires measured native controls")?;
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
                    "Release the Panda3DS controls before launch preparation"
                );
                let output = gamepad_output(&fields, translated)?;
                if let Some((_, expected)) = ACTION_BUTTONS
                    .iter()
                    .find(|(target, _)| *target == row.target_id)
                {
                    ensure!(
                        output == *expected,
                        "Panda3DS {target} drives gamepad {output}, not the hard-wired {expected}; remap it",
                        target = row.target_id
                    );
                } else {
                    // Stick directions must ride the hard-wired axis; the
                    // halves pair into shared axes below.
                    let axis_name = STICK_AXES
                        .iter()
                        .find(|(name, _)| row.target_id.starts_with(*name))
                        .map(|(_, axis)| *axis)
                        .context("Panda3DS stick direction is outside the stick set")?;
                    ensure!(
                        output == axis_name,
                        "Panda3DS {target} drives {output}, not the hard-wired {axis_name} axis",
                        target = row.target_id
                    );
                    ensure!(
                        axes.insert(row.target_id.clone(), (axis_name, native.direction > 0))
                            .is_none(),
                        "Panda3DS stick direction appears twice"
                    );
                }
            }
            // Each stick needs all four directions with opposite polarity
            // per axis, proven by the saved calibration endpoints.
            for (prefix, x_axis, y_axis) in [
                ("stick", "leftx", "lefty"),
                ("right_stick", "rightx", "righty"),
            ] {
                for (negative, positive, axis) in [
                    (format!("{prefix}_left"), format!("{prefix}_right"), x_axis),
                    (format!("{prefix}_up"), format!("{prefix}_down"), y_axis),
                ] {
                    let (_, neg_dir) = axes.get(negative.as_str()).with_context(|| {
                        format!("Panda3DS stick direction {negative} is not calibrated")
                    })?;
                    let (_, pos_dir) = axes.get(positive.as_str()).with_context(|| {
                        format!("Panda3DS stick direction {positive} is not calibrated")
                    })?;
                    ensure!(
                        !neg_dir && *pos_dir,
                        "Panda3DS stick halves must face opposite polarity"
                    );
                    let _ = axis;
                }
            }
            let directory = tempfile::Builder::new()
                .prefix("lunchbox-panda3ds-")
                .tempdir()?;
            // ./config.toml wins the config search, so running here fully
            // isolates input while saves stay in the SDL preference root.
            let config_path = directory.path().join("config.toml");
            fs::write(&config_path, session_config())?;
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

        pub(crate) fn directory(&self) -> &std::path::Path {
            self.directory.path()
        }

        pub(crate) fn verify(&self, cancel: &AtomicBool) -> Result<()> {
            cancelled(cancel)?;
            #[cfg(target_os = "linux")]
            self.topology.verify()?;
            for (path, hash) in &self.hashes {
                ensure!(file_hash(path)? == *hash, "Panda3DS launch input changed");
            }
            let fresh = routing(observe(&self.setup, None, cancel)?);
            self.initial.ensure_same_routing(&fresh)?;
            let captured = observe(&self.setup, Some(&self.physical_path), cancel)?;
            self.initial
                .ensure_same_routing(&routing(captured.clone()))?;
            let device = captured.device_at_path(&self.physical_path)?;
            ensure!(
                device.device_index == 0,
                "Panda3DS SDL index 0 moved before launch"
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
                "Panda3DS executable differs from the saved trusted runtime"
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
                "Panda3DS launch plan changed after preparation"
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
                    "Panda3DS exited before controller handoff"
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
                    "Panda3DS did not open the selected SDL controller before timeout"
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
            anyhow::bail!("Panda3DS calibrated launch requires a native build");
        };
        ensure!(
            option.emulator_name.eq_ignore_ascii_case("Panda3DS")
                && setup.emulator_id == option.emulator_id
                && original.environment.is_empty()
                && original.retroarch_content.is_none(),
            "Panda3DS identity differs or custom environment needs resolution"
        );
        let executable = executable.canonicalize()?;
        ensure!(
            executable == original.program.canonicalize()?,
            "Panda3DS launch executable differs from selection"
        );
        ensure!(
            original.arguments.as_slice() == [setup.content.as_os_str()],
            "Panda3DS calibrated launch requires exactly the saved content argument"
        );
        ensure!(
            file_hash(&executable)?.eq_ignore_ascii_case(&setup.executable_sha256),
            "Panda3DS executable differs from the saved trusted runtime"
        );
        let inputs = session::PreparedSession::prepare(setup, calibrations, inventory, cancel)?;
        let mut plan = original.clone();
        plan.program = executable.clone();
        // ./config.toml wins the config search; the ROM resolves against
        // the working directory, keeping its default positional slot.
        plan.current_directory = inputs.directory().to_path_buf();
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
