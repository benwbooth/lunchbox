//! vector06sdl standalone-native gamecontrollerdb writer.
//!
//! Pinned source: svofski/vector06sdl commit
//! `5cbd54023df430446e283cb874cac36d71359d73`. The SDL frontend opens
//! controllers 0/1 by index (`SDL_GameControllerOpen(i)`) and layers
//! `./gamecontrollerdb.txt` over SDL's bundled database
//! (`refresh_joysticks`). Only six game-controller buttons drive the guest
//! stick: dpad directions plus A/B (`update_joysticks`); sticks, triggers,
//! and shoulders have no mapping. Content is the ROM/FDD image passed with
//! `--rom`/`--fdd`; the session runs in its own directory so only the
//! private database is visible.

use anyhow::{Context, Result, ensure};
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};

pub(crate) const SOURCE_COMMIT: &str = "5cbd54023df430446e283cb874cac36d71359d73";
pub(crate) const PROFILE_ID: &str = "vector06sdl:standalone-vector-stick";
pub(crate) const DB_FILE: &str = "gamecontrollerdb.txt";

/// Layout target ids covered by the native profile: four directions and
/// two fire buttons on stick 0.
pub(crate) const ROUTES: [(&str, &str); 6] = [
    ("up", "Up"),
    ("down", "Down"),
    ("left", "Left"),
    ("right", "Right"),
    ("a", "Fire A"),
    ("b", "Fire B"),
];

/// Game-controller outputs the source reads, with the layout target each
/// one feeds.
pub(crate) const OUTPUTS: [(&str, &str); 6] = [
    ("dpup", "up"),
    ("dpdown", "down"),
    ("dpleft", "left"),
    ("dpright", "right"),
    ("a", "a"),
    ("b", "b"),
];

/// Render one SDL game-controller database line mapping the pad's raw
/// controls to the six outputs the source reads. Unmapped outputs are
/// refused rather than left to SDL's bundled database.
pub(crate) fn gamecontrollerdb_line(
    guid: &str,
    name: &str,
    bindings: &BTreeMap<String, String>,
) -> Result<String> {
    ensure!(
        guid.len() == 32 && guid.bytes().all(|b| b.is_ascii_hexdigit()),
        "vector06sdl GUID must be 32 hex characters"
    );
    ensure!(
        !name.is_empty() && !name.chars().any(char::is_control) && !name.contains(','),
        "vector06sdl controller name is invalid"
    );
    ensure!(
        bindings.len() == OUTPUTS.len(),
        "vector06sdl needs every stick output mapping"
    );
    let mut out = format!("{},*,", guid.to_ascii_lowercase());
    out.push_str(&name.replace(',', " ").trim());
    out.push_str(",platform:Linux,");
    let mut used = BTreeSet::new();
    for (output, _) in OUTPUTS {
        let input = bindings
            .get(output)
            .with_context(|| format!("vector06sdl output {output} is absent"))?;
        ensure!(
            !input.is_empty()
                && input.len() <= 16
                && input
                    .bytes()
                    .all(|b| b.is_ascii_alphanumeric() || b == b'.'),
            "vector06sdl mapping input is invalid"
        );
        ensure!(
            used.insert(input.clone()),
            "vector06sdl mapping input is reused"
        );
        out.push_str(&format!("{output}:{input},"));
    }
    out.push('\n');
    Ok(out)
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
        pub probe_program: PathBuf,
        pub sdl_library: PathBuf,
        pub executable_sha256: String,
        pub players: Vec<Player>,
    }

    impl SavedSetup {
        pub(crate) fn validate(&self) -> Result<()> {
            ensure!(
                !self.emulator_id.trim().is_empty(),
                "vector06sdl setup needs an emulator identity"
            );
            for path in [&self.content, &self.probe_program, &self.sdl_library] {
                ensure!(
                    path.is_absolute()
                        && !path
                            .components()
                            .any(|part| matches!(part, std::path::Component::ParentDir)),
                    "vector06sdl setup paths must be absolute without parent traversal"
                );
            }
            ensure!(
                self.executable_sha256.len() == 64
                    && self
                        .executable_sha256
                        .bytes()
                        .all(|byte| byte.is_ascii_hexdigit()),
                "vector06sdl setup needs a trusted executable SHA-256"
            );
            let limit = catalog()
                .emulator_profiles
                .iter()
                .find(|profile| profile.id == PROFILE_ID)
                .and_then(|profile| profile.native_launch.as_ref())
                .context("Missing vector06sdl native profile")?
                .max_players;
            ensure!(
                self.players.len() == 1 && self.players[0].player == 1,
                "vector06sdl supports exactly player one"
            );
            ensure!(
                self.players.len() <= limit && !self.players[0].controller_id.trim().is_empty(),
                "vector06sdl player needs a saved controller identity"
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
                .context("Missing vector06sdl native profile")?;
            let player = &self.players[0];
            let calibration = calibrations
                .get(&player.controller_id)
                .context("vector06sdl controller has no saved calibration")?;
            ensure!(
                ["linux", "macos", "windows"].contains(&calibration.os.as_str()),
                "vector06sdl mapping requires a desktop physical calibration"
            );
            let mapping = calibration.plan_profile(profile)?;
            ensure!(
                mapping.rows.iter().all(|row| row.physical_id.is_some()
                    && row
                        .input
                        .as_ref()
                        .is_some_and(|input| input.native.is_some())),
                "vector06sdl needs native calibration for every stick control"
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
                "detail": "Native launch stages a session gamecontrollerdb.txt with the six stick outputs, then rechecks the exact SDL routes on Linux, Windows, and macOS. Ownership is strongest on Linux (/proc maps); other hosts pin the executable plus a fresh device re-probe. Only stick 0 is supported; runtime behavior remains unverified."
            }))
        }
    }

    pub(crate) fn validate_setups(setups: &[SavedSetup]) -> Result<()> {
        ensure!(setups.len() <= 1024, "Too many vector06sdl saved setups");
        let mut identities = BTreeSet::new();
        for setup in setups {
            setup.validate()?;
            ensure!(
                identities.insert((&setup.emulator_id, &setup.content)),
                "Duplicate vector06sdl emulator/content setup"
            );
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn bindings() -> BTreeMap<String, String> {
        OUTPUTS
            .iter()
            .enumerate()
            .map(|(i, (output, _))| ((*output).to_owned(), format!("b{i}")))
            .collect()
    }

    #[test]
    fn renders_database_line_with_all_stick_outputs() {
        let line =
            gamecontrollerdb_line("0123456789ABCDEF0123456789ABCDEF", "Pad", &bindings()).unwrap();
        assert!(line.starts_with("0123456789abcdef0123456789abcdef,*,Pad,platform:Linux,"));
        assert!(line.contains("dpup:b0,"));
        assert!(line.contains("b:b5,"));
        assert!(line.ends_with('\n'));
    }

    #[test]
    fn rejects_bad_guids_names_and_reused_inputs() {
        assert!(gamecontrollerdb_line("short", "Pad", &bindings()).is_err());
        assert!(gamecontrollerdb_line(&"0".repeat(32), "a,b", &bindings()).is_err());
        let mut bad = bindings();
        bad.insert("b".into(), "b0".into());
        assert!(gamecontrollerdb_line(&"0".repeat(32), "Pad", &bad).is_err());
    }
}

mod session {
    use super::*;
    #[cfg(target_os = "linux")]
    use crate::controller_bizhawk_guard::InputTopology;
    use crate::{
        controller_catalog::Calibration,
        controller_native_platform as platform,
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
            serde_json::from_slice(&output).context("Invalid vector06sdl SDL2 capture")?;
        ensure!(
            snapshot.version[0] == 2
                && snapshot.library.canonicalize()? == setup.sdl_library.canonicalize()?
                && snapshot.library_sha256 == file_hash(&setup.sdl_library)?,
            "vector06sdl helper inspected a different SDL2 runtime"
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

    /// Raw SDL2 control to gamecontrollerdb input token through the pad's
    /// effective mapping. Buttons and hats resolve through button entries;
    /// axis halves resolve through full-axis stick entries. Anything else
    /// is refused rather than guessed.
    fn input_token(fields: &BTreeMap<String, String>, translated: DigitalInput) -> Result<String> {
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
                let raw = u32::try_from(index).context("vector06sdl button is too large")?;
                let mut found = None;
                for output in BUTTON_OUTPUTS {
                    if let Some(input) = fields.get(output) {
                        let candidate: u32 = input
                            .trim_end_matches('~')
                            .strip_prefix('b')
                            .context("vector06sdl mapping entry is not a button")?
                            .parse()
                            .context("vector06sdl mapping button is invalid")?;
                        if candidate == raw {
                            found = Some(output);
                            break;
                        }
                    }
                }
                let output = found.context("vector06sdl raw button is unmapped")?;
                format!(
                    "b{}",
                    BUTTON_OUTPUTS
                        .iter()
                        .position(|name| *name == output)
                        .context("vector06sdl SDL button is unknown")?
                )
            }
            DigitalInput::Hat { index, direction } => {
                let raw = u32::try_from(index).context("vector06sdl hat is too large")?;
                let mut found = None;
                for output in BUTTON_OUTPUTS {
                    if let Some(input) = fields.get(output) {
                        let rest = input.trim_end_matches('~');
                        if let Some(hat) = rest.strip_prefix('h') {
                            let (number, mask) = hat
                                .split_once('.')
                                .context("vector06sdl mapping hat is invalid")?;
                            // The mapped hat mask must equal the pressed
                            // direction; anything else drives another output.
                            if number.parse::<u32>().is_ok_and(|n| n == raw)
                                && mask.parse::<u8>().is_ok_and(|m| m == direction)
                            {
                                found = Some((output, mask.to_owned()));
                                break;
                            }
                        }
                    }
                }
                let (output, mask) = found.context("vector06sdl raw hat is unmapped")?;
                // Emit the source hat token verbatim: the mapping already
                // proved this (hat, mask) pair drives the output.
                let _ = output;
                format!("h{raw}.{mask}")
            }
            DigitalInput::Axis { index, .. } => {
                let raw = u32::try_from(index).context("vector06sdl axis is too large")?;
                let mut found: Option<&str> = None;
                for output in AXIS_OUTPUTS {
                    if let Some(input) = fields.get(output) {
                        let candidate: u32 = input
                            .trim_start_matches(['+', '-'])
                            .trim_end_matches('~')
                            .strip_prefix('a')
                            .context("vector06sdl stick entry is not an axis")?
                            .parse()
                            .context("vector06sdl stick axis is invalid")?;
                        if candidate == raw {
                            found = Some(output);
                            break;
                        }
                    }
                }
                match AXIS_OUTPUTS.iter().position(|name| Some(*name) == found) {
                    Some(position) => format!("a{position}"),
                    None => anyhow::bail!("vector06sdl raw axis {raw} is unmapped"),
                }
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
                "vector06sdl content must be a direct regular image with canonical ancestry"
            );
            let player = &setup.players[0];
            let found = inventory
                .iter()
                .filter(|device| device.stable_id == player.controller_id)
                .collect::<Vec<_>>();
            ensure!(
                found.len() == 1 && !found[0].is_virtual,
                "vector06sdl physical controller is missing or ambiguous"
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
                    "vector06sdl physical controller is missing or ambiguous in SDL"
                );
                selected_string
            };
            let captured = observe(setup, Some(&physical_path), cancel)?;
            initial.ensure_same_routing(&routing(captured.clone()))?;
            #[cfg(target_os = "linux")]
            topology.verify()?;
            let device = captured.device_at_path(&physical_path)?;
            // SDL_GameControllerOpen(i) opens SDL slots; stick 0 reads
            // controller 0, so the pad must be index 0.
            ensure!(
                device.device_index == 0,
                "vector06sdl stick 0 needs SDL index 0; selected pad is index {}",
                device.device_index
            );
            // Non-Linux hosts additionally pin path plus index through the
            // shared helper; Linux holds the sysfs topology instead.
            #[cfg(not(target_os = "linux"))]
            platform::require_unique_device_path(
                &captured.devices,
                &physical_path,
                device.device_index,
            )?;
            let guid = device.guid.to_ascii_lowercase();
            ensure!(
                guid.len() == 32 && guid.bytes().all(|b| b.is_ascii_hexdigit()),
                "vector06sdl SDL GUID is invalid"
            );
            let name = device
                .name
                .as_deref()
                .context("vector06sdl SDL device has no name")?;
            let fields = mapping_fields(
                device
                    .mapping
                    .as_deref()
                    .context("vector06sdl SDL mapping is absent")?,
            );
            let calibration = calibrations
                .get(&player.controller_id)
                .context("vector06sdl calibration disappeared")?;
            let profile = crate::controller_catalog::catalog()
                .emulator_profiles
                .iter()
                .find(|profile| profile.id == PROFILE_ID)
                .context("Missing vector06sdl native profile")?;
            let physical = PhysicalMap::from_device(device)?;
            let state = device
                .sampled_state
                .as_ref()
                .context("vector06sdl SDL released state is missing")?;
            state.validate(
                device
                    .controls
                    .as_ref()
                    .context("vector06sdl SDL control counts are missing")?,
            )?;
            let mut bindings = BTreeMap::new();
            for row in calibration.plan_profile(profile)?.rows {
                let (_, output) = OUTPUTS
                    .iter()
                    .find(|(_, target)| *target == row.target_id)
                    .context("vector06sdl target is outside the stick profile")?;
                let input = row
                    .input
                    .as_ref()
                    .context("vector06sdl stick control is not calibrated")?;
                let native = input
                    .native
                    .as_ref()
                    .context("vector06sdl requires measured native controls")?;
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
                    "Release the vector06sdl controls before launch preparation"
                );
                ensure!(
                    bindings
                        .insert((*output).to_owned(), input_token(&fields, translated)?)
                        .is_none(),
                    "vector06sdl output appears twice"
                );
            }
            let directory = tempfile::Builder::new()
                .prefix("lunchbox-vector06sdl-")
                .tempdir()?;
            // gamecontrollerdb.txt resolves in the working directory; the
            // launch layer runs there so only the private file is visible.
            let db_path = directory.path().join(DB_FILE);
            fs::write(&db_path, gamecontrollerdb_line(&guid, name, &bindings)?)?;
            let mut hashes = BTreeMap::new();
            for path in [
                &setup.content,
                &setup.probe_program,
                &setup.sdl_library,
                &db_path,
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
                ensure!(
                    file_hash(path)? == *hash,
                    "vector06sdl launch input changed"
                );
            }
            let fresh = routing(observe(&self.setup, None, cancel)?);
            self.initial.ensure_same_routing(&fresh)?;
            let captured = observe(&self.setup, Some(&self.physical_path), cancel)?;
            self.initial
                .ensure_same_routing(&routing(captured.clone()))?;
            let device = captured.device_at_path(&self.physical_path)?;
            ensure!(
                device.device_index == self.device_index,
                "vector06sdl SDL index moved before launch"
            );
            #[cfg(not(target_os = "linux"))]
            platform::require_unique_device_path(
                &captured.devices,
                &self.physical_path,
                self.device_index,
            )?;
            #[cfg(target_os = "linux")]
            self.topology.verify()?;
            Ok(())
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
                "vector06sdl executable differs from the saved trusted runtime"
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
                "vector06sdl launch plan changed after preparation"
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
                    "vector06sdl exited before controller handoff"
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
                    "vector06sdl did not open the selected SDL controller before timeout"
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
            anyhow::bail!("vector06sdl calibrated launch requires a native build");
        };
        ensure!(
            option.emulator_name.eq_ignore_ascii_case("vector06sdl")
                && setup.emulator_id == option.emulator_id
                && original.environment.is_empty()
                && original.retroarch_content.is_none(),
            "vector06sdl identity differs or custom environment needs resolution"
        );
        let executable = executable.canonicalize()?;
        ensure!(
            executable == original.program.canonicalize()?,
            "vector06sdl launch executable differs from selection"
        );
        ensure!(
            original.arguments.as_slice() == [setup.content.as_os_str()],
            "vector06sdl calibrated launch requires exactly the saved content argument"
        );
        ensure!(
            file_hash(&executable)?.eq_ignore_ascii_case(&setup.executable_sha256),
            "vector06sdl executable differs from the saved trusted runtime"
        );
        let inputs = session::PreparedSession::prepare(setup, calibrations, inventory, cancel)?;
        let mut plan = original.clone();
        plan.program = executable.clone();
        // gamecontrollerdb.txt resolves in the working directory; the image
        // keeps its default --rom slot.
        plan.current_directory = inputs.directory().to_path_buf();
        plan.arguments = vec![
            std::ffi::OsString::from("--rom"),
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
