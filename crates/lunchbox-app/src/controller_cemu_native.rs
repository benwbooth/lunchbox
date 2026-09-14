//! Source-accurate Cemu standalone controller-profile writer.
//!
//! Pinned source: cemu-project/Cemu commit
//! `3310f3b8b184d64a62b89fd59088c799432badf5`.
//!
//! Cemu's native profile format is XML, and its SDL controller UUID is the
//! occurrence number followed by SDL's lower-case GUID (`0_<guid>` for the
//! first device).  This module deliberately writes only the VPAD mappings;
//! MLC/save data and keys remain in the caller's normal Cemu data root.

use anyhow::{Context, Result, ensure};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

pub(crate) const PROFILE_ID: &str = "cemu:standalone-cemu-vpad";

/// Cemu VPAD mapping IDs from `VPADController::ButtonId`.
pub(crate) const CONTROLS: [(&str, u64); 18] = [
    ("a", 1),
    ("b", 2),
    ("x", 3),
    ("y", 4),
    ("l", 5),
    ("r", 6),
    ("zl", 7),
    ("zr", 8),
    ("plus", 9),
    ("minus", 10),
    ("up", 11),
    ("down", 12),
    ("left", 13),
    ("right", 14),
    ("stick_up", 17),
    ("stick_down", 18),
    ("stick_left", 19),
    ("stick_right", 20),
];

/// SDLController button/axis values from Cemu's `Controller.h`.
#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub(crate) enum Binding {
    Button(u64),
    Axis { index: u64, positive: bool },
}

impl Binding {
    fn cemu_value(self) -> Result<u64> {
        match self {
            Self::Button(index) => {
                ensure!(index < 32, "Cemu SDL gamepad button is out of range");
                Ok(index)
            }
            Self::Axis { index, positive } => match (index, positive) {
                (0, true) => Ok(38),
                (1, true) => Ok(39),
                (2, true) => Ok(40),
                (3, true) => Ok(41),
                (4, true) => Ok(42),
                (5, true) => Ok(43),
                (0, false) => Ok(44),
                (1, false) => Ok(45),
                (2, false) => Ok(46),
                (3, false) => Ok(47),
                (4, false) => Ok(48),
                (5, false) => Ok(49),
                _ => anyhow::bail!("Cemu SDL gamepad axis is out of range"),
            },
        }
    }
}

fn xml_escape(value: &str) -> String {
    value
        .replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
        .replace('\'', "&apos;")
}

/// Render the XML written by Cemu's input manager for one SDLController VPAD.
pub(crate) fn profile_xml(
    guid_index: u32,
    guid: &str,
    display_name: &str,
    mappings: &BTreeMap<String, Binding>,
) -> Result<Vec<u8>> {
    ensure!(
        guid.len() == 32 && guid.bytes().all(|b| b.is_ascii_hexdigit()),
        "Cemu SDL GUID must be 32 hex characters"
    );
    ensure!(
        !display_name.is_empty() && !display_name.chars().any(char::is_control),
        "Cemu display name is invalid"
    );
    ensure!(
        mappings.len() == CONTROLS.len(),
        "Cemu needs every VPAD gameplay mapping"
    );
    let mut out = String::from(
        "<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n<emulated_controller>\n  <type>VPAD</type>\n  <controller>\n    <api>SDLController</api>\n    <uuid>",
    );
    out.push_str(&guid_index.to_string());
    out.push('_');
    out.push_str(&guid.to_ascii_lowercase());
    out.push_str("</uuid>\n    <display_name>");
    out.push_str(&xml_escape(display_name));
    // ControllerBase's default settings in the pinned source use zero rumble
    // and a 0.25 deadzone for each axis family.  Keep the generated profile
    // source-shaped without silently enabling haptics.
    out.push_str("</display_name>\n    <rumble>0</rumble>\n    <axis><deadzone>0.25</deadzone><range>1</range></axis>\n    <rotation><deadzone>0.25</deadzone><range>1</range></rotation>\n    <trigger><deadzone>0.25</deadzone><range>1</range></trigger>\n    <mappings>\n");
    let mut used = std::collections::BTreeSet::new();
    for (name, target) in CONTROLS {
        let binding = mappings
            .get(name)
            .ok_or_else(|| anyhow::anyhow!("Cemu mapping {name} is absent"))?;
        let value = binding.cemu_value()?;
        ensure!(used.insert(value), "Cemu mapping is reused");
        out.push_str(&format!(
            "      <entry><mapping>{target}</mapping><button>{value}</button></entry>\n"
        ));
    }
    out.push_str(
        "    </mappings>\n    <toggle_display>0</toggle_display>\n  </controller>\n</emulated_controller>\n",
    );
    Ok(out.into_bytes())
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn emits_uuid_and_vpad_entries() {
        let mut map = BTreeMap::new();
        for (name, _) in CONTROLS {
            map.insert(name.to_owned(), Binding::Button(map.len() as u64));
        }
        let xml = String::from_utf8(
            profile_xml(2, "0123456789abcdef0123456789abcdef", "Pad & one", &map).unwrap(),
        )
        .unwrap();
        assert!(xml.contains("<uuid>2_0123456789abcdef0123456789abcdef</uuid>"));
        assert!(xml.contains("Pad &amp; one"));
        assert!(xml.contains("<rumble>0</rumble>"));
        assert_eq!(xml.matches("<entry>").count(), 18);
    }
}

/// Layout target ids covered by the native profile.
pub(crate) const ROUTES: [(&str, &str); 18] = [
    ("a", "A"),
    ("b", "B"),
    ("x", "X"),
    ("y", "Y"),
    ("l", "L"),
    ("r", "R"),
    ("zl", "ZL"),
    ("zr", "ZR"),
    ("plus", "Plus"),
    ("minus", "Minus"),
    ("up", "Up"),
    ("down", "Down"),
    ("left", "Left"),
    ("right", "Right"),
    ("stick_up", "Left stick up"),
    ("stick_down", "Left stick down"),
    ("stick_left", "Left stick left"),
    ("stick_right", "Left stick right"),
];

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
                "Cemu setup needs an emulator identity"
            );
            for path in [&self.content, &self.probe_program, &self.sdl_library] {
                ensure!(
                    path.is_absolute()
                        && !path
                            .components()
                            .any(|part| matches!(part, std::path::Component::ParentDir)),
                    "Cemu setup paths must be absolute without parent traversal"
                );
            }
            ensure!(
                self.executable_sha256.len() == 64
                    && self
                        .executable_sha256
                        .bytes()
                        .all(|byte| byte.is_ascii_hexdigit()),
                "Cemu setup needs a trusted executable SHA-256"
            );
            let limit = catalog()
                .emulator_profiles
                .iter()
                .find(|profile| profile.id == PROFILE_ID)
                .and_then(|profile| profile.native_launch.as_ref())
                .context("Missing Cemu native profile")?
                .max_players;
            ensure!(
                self.players.len() == 1 && self.players[0].player == 1,
                "Cemu supports exactly player one"
            );
            ensure!(
                self.players.len() <= limit && !self.players[0].controller_id.trim().is_empty(),
                "Cemu player needs a saved controller identity"
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
                .context("Missing Cemu native profile")?;
            let player = &self.players[0];
            let calibration = calibrations
                .get(&player.controller_id)
                .context("Cemu controller has no saved calibration")?;
            ensure!(
                calibration.os == "linux",
                "Cemu mapping requires Linux physical calibration"
            );
            let mapping = calibration.plan_profile(profile)?;
            ensure!(
                mapping.rows.iter().all(|row| row.physical_id.is_some()
                    && row
                        .input
                        .as_ref()
                        .is_some_and(|input| input.native.is_some())),
                "Cemu needs native calibration for every VPAD control"
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
                "detail": "Native Linux launch writes a private controller0.xml under XDG_CONFIG_HOME, then rechecks the exact SDL3 routes. Only the single VPAD on a unique gamepad GUID is supported; runtime behavior remains unverified."
            }))
        }
    }

    pub(crate) fn validate_setups(setups: &[SavedSetup]) -> Result<()> {
        ensure!(setups.len() <= 1024, "Too many Cemu saved setups");
        let mut identities = std::collections::BTreeSet::new();
        for setup in setups {
            setup.validate()?;
            ensure!(
                identities.insert((&setup.emulator_id, &setup.content)),
                "Duplicate Cemu emulator/content setup"
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
    use lunchbox_controller_probe::{
        Snapshot,
        bindings::{Input as ResolvedInput, Output as ResolvedOutput},
        duckstation::DigitalInput,
        file_hash,
        linux_classic::AxisEndpoints,
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
        paths: &[String],
        cancel: &AtomicBool,
    ) -> Result<Snapshot> {
        let mut command = Command::new(&setup.probe_program);
        command
            .arg("--sdl-library")
            .arg(&setup.sdl_library)
            .arg("--hint")
            .arg("SDL_JOYSTICK_LINUX_CLASSIC=1");
        for path in paths {
            command.arg("--bindings-for-path").arg(path);
        }
        let (output, _) = capture(&mut command, cancel)?;
        let snapshot: Snapshot =
            serde_json::from_slice(&output).context("Invalid Cemu SDL3 capture")?;
        ensure!(
            snapshot.library.canonicalize()? == setup.sdl_library.canonicalize()?
                && snapshot.library_sha256 == file_hash(&setup.sdl_library)?
                && snapshot
                    .effective_hints
                    .get("SDL_JOYSTICK_LINUX_CLASSIC")
                    .and_then(Option::as_deref)
                    == Some("1"),
            "Cemu helper inspected a different SDL runtime or backend"
        );
        Ok(snapshot)
    }

    fn comparable(snapshot: &Snapshot) -> Result<serde_json::Value> {
        let mut value = serde_json::to_value(snapshot)?;
        let object = value
            .as_object_mut()
            .context("Invalid Cemu snapshot shape")?;
        object.remove("warnings");
        for device in object
            .get_mut("devices")
            .and_then(serde_json::Value::as_array_mut)
            .into_iter()
            .flatten()
        {
            let object = device
                .as_object_mut()
                .context("Invalid Cemu device shape")?;
            object.remove("resolved");
            object.remove("linux_classic");
        }
        Ok(value)
    }

    /// Gamepad GUID from the effective mapping string: the same GUID family
    /// Cemu matches with `SDL_GetGamepadGUIDForID` for its `uuid`.
    fn gamepad_guid(mapping: &str) -> Result<String> {
        let guid = mapping
            .split(',')
            .next()
            .context("Cemu SDL mapping is empty")?
            .trim()
            .to_ascii_lowercase();
        ensure!(
            guid.len() == 32 && guid.bytes().all(|b| b.is_ascii_hexdigit()),
            "Cemu gamepad GUID is invalid"
        );
        Ok(guid)
    }

    /// Invert the resolved mapping: the profile wants SDL gamepad indices,
    /// the calibration yields raw joystick numbers.
    fn gamepad_button(
        resolved: &lunchbox_controller_probe::bindings::ResolvedGamepad,
        raw_button: u32,
    ) -> Result<u64> {
        for binding in &resolved.bindings {
            if let (ResolvedInput::Button { index }, ResolvedOutput::Button { index: gamepad }) =
                (&binding.input, &binding.output)
            {
                if *index == raw_button {
                    ensure!(*gamepad <= 31, "Cemu SDL gamepad button is out of range");
                    return Ok(u64::from(*gamepad));
                }
            }
        }
        anyhow::bail!("Cemu raw button {raw_button} has no gamepad mapping")
    }

    fn gamepad_axis(
        resolved: &lunchbox_controller_probe::bindings::ResolvedGamepad,
        raw_axis: u32,
    ) -> Result<u64> {
        for binding in &resolved.bindings {
            if let (
                ResolvedInput::Axis { index, .. },
                ResolvedOutput::Axis { index: gamepad, .. },
            ) = (&binding.input, &binding.output)
            {
                if *index == raw_axis {
                    ensure!(*gamepad <= 5, "Cemu SDL gamepad axis is out of range");
                    return Ok(u64::from(*gamepad));
                }
            }
        }
        anyhow::bail!("Cemu raw axis {raw_axis} has no gamepad mapping")
    }

    pub(crate) struct PreparedSession {
        directory: tempfile::TempDir,
        physical_path: String,
        topology: InputTopology,
        initial: serde_json::Value,
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
                "Cemu content must be a direct regular file with canonical ancestry"
            );
            let player = &setup.players[0];
            let found = inventory
                .iter()
                .filter(|device| device.stable_id == player.controller_id)
                .collect::<Vec<_>>();
            ensure!(
                found.len() == 1 && !found[0].is_virtual,
                "Cemu physical controller is missing or ambiguous"
            );
            let selected = found[0].device_path.clone();
            let topology = InputTopology::capture(std::slice::from_ref(&selected))?;
            let initial = comparable(&observe(setup, &[], cancel)?)?;
            let physical_path = topology.resolve_runtime_path(
                &selected,
                observe(setup, &[], cancel)?
                    .devices
                    .iter()
                    .filter_map(|device| device.path.as_deref()),
            )?;
            let captured = observe(setup, &[physical_path.clone()], cancel)?;
            ensure!(
                comparable(&captured)? == initial,
                "Cemu SDL inventory moved during preparation"
            );
            topology.verify()?;
            let device = captured.device_at_path(&physical_path)?;
            ensure!(device.is_gamepad, "Cemu needs an SDL-recognized gamepad");
            let resolved = device
                .resolved
                .as_ref()
                .context("Cemu SDL resolved bindings are absent")?;
            let classic = device
                .linux_classic
                .as_ref()
                .context("Cemu classic Linux control map is absent")?;
            classic.validate_counts(resolved)?;
            let mapping_text = device
                .mapping
                .as_deref()
                .context("Cemu SDL mapping is absent")?;
            let guid = gamepad_guid(mapping_text)?;
            // guid_index counts same-GUID gamepads ahead of ours in SDL
            // gamepad order; duplicates would make the profile ambiguous, so
            // the session requires a unique GUID.
            let full = observe(setup, &[], cancel)?;
            let mut duplicates = 0;
            for other in &full.devices {
                if other.is_gamepad
                    && let Some(other_mapping) = other.mapping.as_deref()
                    && gamepad_guid(other_mapping).is_ok_and(|other_guid| other_guid == guid)
                {
                    duplicates += 1;
                }
            }
            ensure!(
                duplicates == 1,
                "Cemu gamepad GUID is duplicated; unplug the twin pad"
            );
            let name = device
                .gamepad_name
                .as_deref()
                .or(device.name.as_deref())
                .context("Cemu SDL device has no name")?;
            let calibration = calibrations
                .get(&player.controller_id)
                .context("Cemu calibration disappeared")?;
            let profile = crate::controller_catalog::catalog()
                .emulator_profiles
                .iter()
                .find(|profile| profile.id == PROFILE_ID)
                .context("Missing Cemu native profile")?;
            let mut mappings: BTreeMap<String, Binding> = BTreeMap::new();
            let mut sticks: BTreeMap<String, (u32, bool)> = BTreeMap::new();
            for row in calibration.plan_profile(profile)?.rows {
                ensure!(
                    ROUTES.iter().any(|(target, _)| *target == row.target_id),
                    "Cemu target {} outside contract",
                    row.target_id
                );
                let input = row
                    .input
                    .as_ref()
                    .context("Cemu VPAD control is not calibrated")?;
                let native = input
                    .native
                    .as_ref()
                    .context("Cemu requires measured native controls")?;
                let measured = input.axis.as_ref().map(|axis| AxisEndpoints {
                    released: axis.released,
                    pressed: axis.pressed,
                });
                let translated = classic.digital_input(native.code, measured)?;
                // Release posture is proven by the saved calibration
                // endpoints; the SDL3 snapshot carries no live sample.
                match row.target_id.as_str() {
                    "stick_up" | "stick_down" | "stick_left" | "stick_right" => {
                        let DigitalInput::Axis { index, .. } = translated else {
                            anyhow::bail!("Cemu stick directions need proportional axes")
                        };
                        ensure!(
                            sticks
                                .insert(row.target_id.clone(), (index, native.direction > 0))
                                .is_none(),
                            "Cemu stick direction appears twice"
                        );
                    }
                    _ => {
                        let DigitalInput::Button(index) = translated else {
                            anyhow::bail!("Cemu VPAD buttons need raw buttons")
                        };
                        ensure!(
                            mappings
                                .insert(
                                    row.target_id,
                                    Binding::Button(gamepad_button(resolved, index)?)
                                )
                                .is_none(),
                            "Cemu VPAD button appears twice"
                        );
                    }
                }
            }
            // The single stick pair shares one axis with opposite polarity.
            let axis_of = |negative: &str, positive: &str| {
                let (neg_index, neg_dir) = sticks.get(negative).with_context(|| {
                    format!("Cemu stick direction {negative} is not calibrated")
                })?;
                let (pos_index, pos_dir) = sticks.get(positive).with_context(|| {
                    format!("Cemu stick direction {positive} is not calibrated")
                })?;
                ensure!(
                    neg_index == pos_index && !neg_dir && *pos_dir,
                    "Cemu stick halves must share one axis with opposite polarity"
                );
                gamepad_axis(resolved, *neg_index)
            };
            let stick_x = axis_of("stick_left", "stick_right")?;
            let stick_y = axis_of("stick_up", "stick_down")?;
            // Writer axis codes: 38+index positive, 44+index negative.
            mappings.insert(
                "stick_left".to_owned(),
                Binding::Axis {
                    index: stick_x,
                    positive: false,
                },
            );
            mappings.insert(
                "stick_right".to_owned(),
                Binding::Axis {
                    index: stick_x,
                    positive: true,
                },
            );
            mappings.insert(
                "stick_up".to_owned(),
                Binding::Axis {
                    index: stick_y,
                    positive: false,
                },
            );
            mappings.insert(
                "stick_down".to_owned(),
                Binding::Axis {
                    index: stick_y,
                    positive: true,
                },
            );
            let button_bindings = mappings;
            let directory = tempfile::Builder::new()
                .prefix("lunchbox-cemu-")
                .tempdir()?;
            // controllerProfiles/ resolves under the XDG config root; the
            // launch layer points it here so only controller0.xml is visible.
            let profiles_dir = directory.path().join("Cemu").join("controllerProfiles");
            fs::create_dir_all(&profiles_dir)?;
            let profile_path = profiles_dir.join("controller0.xml");
            fs::write(
                &profile_path,
                profile_xml(0, &guid, name, &button_bindings)?,
            )?;
            let mut hashes = BTreeMap::new();
            for path in [
                &setup.content,
                &setup.probe_program,
                &setup.sdl_library,
                &profile_path,
            ] {
                hashes.insert(path.clone(), file_hash(path)?);
            }
            let prepared = Self {
                directory,
                physical_path,
                topology,
                initial,
                setup: setup.clone(),
                hashes,
            };
            prepared.verify(cancel)?;
            Ok(prepared)
        }

        /// Private root for `XDG_CONFIG_HOME`; Cemu appends `/Cemu` itself.
        pub(crate) fn config_home(&self) -> &std::path::Path {
            self.directory.path()
        }

        pub(crate) fn verify(&self, cancel: &AtomicBool) -> Result<()> {
            cancelled(cancel)?;
            self.topology.verify()?;
            for (path, hash) in &self.hashes {
                ensure!(file_hash(path)? == *hash, "Cemu launch input changed");
            }
            let fresh = comparable(&observe(&self.setup, &[], cancel)?)?;
            ensure!(
                fresh == self.initial,
                "Cemu SDL inventory moved before launch"
            );
            let captured = observe(&self.setup, &[self.physical_path.clone()], cancel)?;
            ensure!(
                comparable(&captured)? == self.initial,
                "Cemu SDL inventory moved before launch"
            );
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
                "Cemu executable differs from the saved trusted runtime"
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
                "Cemu launch plan changed after preparation"
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
                    "Cemu exited before controller handoff"
                );
                if let Some(pid) = native_pid(child.id(), &self.executable)?
                    && self.ready(pid)?
                {
                    self.inputs.check_health()?;
                    return Ok(());
                }
                ensure!(
                    Instant::now() < deadline,
                    "Cemu did not open the selected SDL controller before timeout"
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
            anyhow::bail!("Cemu calibrated launch requires native Linux");
        };
        ensure!(
            option.emulator_name.eq_ignore_ascii_case("Cemu")
                && setup.emulator_id == option.emulator_id
                && original.environment.is_empty()
                && original.retroarch_content.is_none(),
            "Cemu identity differs or custom environment needs resolution"
        );
        let executable = executable.canonicalize()?;
        ensure!(
            executable == original.program.canonicalize()?,
            "Cemu launch executable differs from selection"
        );
        ensure!(
            original.arguments.as_slice() == [setup.content.as_os_str()],
            "Cemu calibrated launch requires exactly the saved content argument"
        );
        ensure!(
            file_hash(&executable)?.eq_ignore_ascii_case(&setup.executable_sha256),
            "Cemu executable differs from the saved trusted runtime"
        );
        let inputs = session::PreparedSession::prepare(setup, calibrations, inventory, cancel)?;
        let mut plan = original.clone();
        plan.program = executable.clone();
        // controllerProfiles/ resolves under the XDG config root; the MLC
        // stays in the real user data. -g launches the game directly.
        plan.environment.push((
            std::ffi::OsString::from("XDG_CONFIG_HOME"),
            inputs.config_home().as_os_str().to_owned(),
        ));
        plan.arguments = vec![
            std::ffi::OsString::from("-g"),
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
