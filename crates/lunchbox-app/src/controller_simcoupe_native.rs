//! SimCoupe standalone-native joystick writer.
//!
//! Pinned source: simonowen/simcoupe commit
//! `1f966036543991c05022ce4f95cfbfbb0014b187`. The SDL frontend opens
//! joysticks by exact SDL name (`joydev1`, empty selects the first
//! available) and reads axes 0/1 as directions, hats as directions, and any
//! button as fire (`SDL/Input.cpp`). The desktops map to SAM input through
//! `joytype1` (1 = joystick 1, 2 = joystick 2, 3 = Kempston). Options live in
//! `key=value` lines in `$HOME/.simcoupe/SimCoupe.cfg` (`Base/Options.cpp`)
//! and the disk image is the positional launch argument. Content is the
//! disk image path.

use anyhow::{Context, Result, ensure};
use serde::{Deserialize, Serialize};
use std::collections::BTreeSet;

pub(crate) const SOURCE_COMMIT: &str = "1f966036543991c05022ce4f95cfbfbb0014b187";
pub(crate) const PROFILE_ID: &str = "simcoupe:standalone-sam-joystick";
pub(crate) const CONFIG_FILE: &str = "SimCoupe.cfg";

/// Layout target ids covered by the native profile: four directions and
/// fire on SAM joystick 1. The source reads any button as fire, so the
/// session pins raw button 0.
pub(crate) const ROUTES: [(&str, &str); 5] = [
    ("up", "Up"),
    ("down", "Down"),
    ("left", "Left"),
    ("right", "Right"),
    ("fire", "Fire"),
];

/// Patch only the joystick selection keys in a copied `SimCoupe.cfg`:
/// `joydev1` names the exact SDL joystick, `joytype1 = 1` maps it to SAM
/// joystick 1. All other keys (video, audio, disks, paths) survive.
/// Unknown keys, comments, ordering, and line endings are retained; a
/// complete baseline is required so defaults cannot silently replace the
/// staged selection.
pub(crate) fn patch_config(baseline: &[u8], device_name: &str) -> Result<String> {
    ensure!(
        !device_name.is_empty()
            && device_name.len() <= 256
            && !device_name
                .chars()
                .any(|c| c.is_control() || c == '\n' || c == '\r'),
        "SimCoupe joystick device name is invalid"
    );
    let text = std::str::from_utf8(baseline).context("SimCoupe config is not UTF-8")?;
    ensure!(!text.contains('\0'), "SimCoupe config contains a NUL byte");
    let newline = if text.contains("\r\n") { "\r\n" } else { "\n" };
    let mut out = String::with_capacity(text.len() + 128);
    let mut seen_dev = false;
    let mut seen_type = false;
    for raw in text.split_inclusive('\n') {
        let line = raw.strip_suffix('\n').unwrap_or(raw);
        let stripped = line.strip_suffix('\r').unwrap_or(line);
        if let Some((key, _)) = stripped.split_once('=') {
            match key.trim() {
                "joydev1" => {
                    ensure!(!seen_dev, "SimCoupe config contains duplicate joydev1");
                    seen_dev = true;
                    out.push_str(&format!("joydev1={device_name}{newline}"));
                    continue;
                }
                "joytype1" => {
                    ensure!(!seen_type, "SimCoupe config contains duplicate joytype1");
                    seen_type = true;
                    out.push_str(&format!("joytype1=1{newline}"));
                    continue;
                }
                _ => {}
            }
        }
        out.push_str(raw);
    }
    if !seen_dev {
        if !out.is_empty() && !out.ends_with('\n') {
            out.push_str(newline);
        }
        out.push_str(&format!("joydev1={device_name}{newline}"));
    }
    if !seen_type {
        if !out.is_empty() && !out.ends_with('\n') {
            out.push_str(newline);
        }
        out.push_str(&format!("joytype1=1{newline}"));
    }
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
        /// The user's real `SimCoupe.cfg`; only `joydev1`/`joytype1` are
        /// patched, so video, audio, disks, and paths survive.
        pub config_source: PathBuf,
        pub probe_program: PathBuf,
        pub sdl_library: PathBuf,
        pub executable_sha256: String,
        pub players: Vec<Player>,
    }

    impl SavedSetup {
        pub(crate) fn validate(&self) -> Result<()> {
            ensure!(
                !self.emulator_id.trim().is_empty(),
                "SimCoupe setup needs an emulator identity"
            );
            for path in [&self.content, &self.probe_program, &self.sdl_library] {
                ensure!(
                    path.is_absolute()
                        && !path
                            .components()
                            .any(|part| matches!(part, std::path::Component::ParentDir)),
                    "SimCoupe setup paths must be absolute without parent traversal"
                );
            }
            ensure!(
                self.executable_sha256.len() == 64
                    && self
                        .executable_sha256
                        .bytes()
                        .all(|byte| byte.is_ascii_hexdigit()),
                "SimCoupe setup needs a trusted executable SHA-256"
            );
            let limit = catalog()
                .emulator_profiles
                .iter()
                .find(|profile| profile.id == PROFILE_ID)
                .and_then(|profile| profile.native_launch.as_ref())
                .context("Missing SimCoupe native profile")?
                .max_players;
            ensure!(
                self.players.len() == 1 && self.players[0].player == 1,
                "SimCoupe supports exactly player one"
            );
            ensure!(
                self.players.len() <= limit && !self.players[0].controller_id.trim().is_empty(),
                "SimCoupe player needs a saved controller identity"
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
                .context("Missing SimCoupe native profile")?;
            let player = &self.players[0];
            let calibration = calibrations
                .get(&player.controller_id)
                .context("SimCoupe controller has no saved calibration")?;
            ensure!(
                ["linux", "macos", "windows"].contains(&calibration.os.as_str()),
                "SimCoupe mapping requires Linux physical calibration"
            );
            let mapping = calibration.plan_profile(profile)?;
            ensure!(
                mapping.rows.iter().all(|row| row.physical_id.is_some()
                    && row
                        .input
                        .as_ref()
                        .is_some_and(|input| input.native.is_some())),
                "SimCoupe needs native calibration for every SAM control"
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
                "detail": "Native Linux launch stages a session SimCoupe.cfg selecting the pad by exact SDL name, then rechecks the exact SDL routes. Only axes, hats, and fire on SAM joystick 1 are supported; runtime behavior remains unverified."
            }))
        }
    }

    pub(crate) fn validate_setups(setups: &[SavedSetup]) -> Result<()> {
        ensure!(setups.len() <= 1024, "Too many SimCoupe saved setups");
        let mut identities = BTreeSet::new();
        for setup in setups {
            setup.validate()?;
            ensure!(
                identities.insert((&setup.emulator_id, &setup.content)),
                "Duplicate SimCoupe emulator/content setup"
            );
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn patches_device_selection_and_preserves_settings() {
        let text = patch_config(b"fullscreen=1\njoydev1=Old Pad\n", "New Pad").unwrap();
        assert!(text.contains("fullscreen=1\n"));
        assert!(text.contains("joydev1=New Pad\n"));
        assert!(text.contains("joytype1=1\n"));
        assert!(!text.contains("Old Pad"));
    }

    #[test]
    fn rejects_bad_names_and_duplicates() {
        assert!(patch_config(b"", "").is_err());
        assert!(patch_config(b"joydev1=A\njoydev1=B\n", "C").is_err());
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
            serde_json::from_slice(&output).context("Invalid SimCoupe SDL2 capture")?;
        ensure!(
            snapshot.version[0] == 2
                && snapshot.library.canonicalize()? == setup.sdl_library.canonicalize()?
                && snapshot.library_sha256 == file_hash(&setup.sdl_library)?,
            "SimCoupe helper inspected a different SDL2 runtime"
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
        config_home: PathBuf,
        #[cfg(not(target_os = "linux"))]
        executable: PathBuf,
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
            executable: &std::path::Path,
            cancel: &AtomicBool,
        ) -> Result<Self> {
            cancelled(cancel)?;
            setup.review(calibrations)?;
            ensure!(
                fs::symlink_metadata(&setup.content)?.file_type().is_file()
                    && setup.content.canonicalize()? == setup.content,
                "SimCoupe content must be a direct regular disk image with canonical ancestry"
            );
            ensure!(
                fs::symlink_metadata(&setup.config_source)?
                    .file_type()
                    .is_file(),
                "SimCoupe config source must be a regular file"
            );
            let player = &setup.players[0];
            let found = inventory
                .iter()
                .filter(|device| device.stable_id == player.controller_id)
                .collect::<Vec<_>>();
            ensure!(
                found.len() == 1 && !found[0].is_virtual,
                "SimCoupe physical controller is missing or ambiguous"
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
                    "simcoupe physical controller is missing or ambiguous in SDL"
                );
                selected_string
            };
            let captured = observe(setup, Some(&physical_path), cancel)?;
            initial.ensure_same_routing(&routing(captured.clone()))?;
            #[cfg(target_os = "linux")]
            topology.verify()?;
            let device = captured.device_at_path(&physical_path)?;
            let sdl_name = device
                .name
                .as_deref()
                .context("SimCoupe SDL device has no name")?;
            // joydev1 matches by exact SDL name; duplicates would make the
            // selection ambiguous, so the session requires a unique name.
            ensure!(
                initial
                    .devices
                    .iter()
                    .filter_map(|other| other.name.as_deref())
                    .filter(|other| *other == sdl_name)
                    .count()
                    == 1,
                "SimCoupe SDL joystick name is duplicated"
            );
            let calibration = calibrations
                .get(&player.controller_id)
                .context("SimCoupe calibration disappeared")?;
            let profile = crate::controller_catalog::catalog()
                .emulator_profiles
                .iter()
                .find(|profile| profile.id == PROFILE_ID)
                .context("Missing SimCoupe native profile")?;
            let physical = PhysicalMap::from_device(device)?;
            let state = device
                .sampled_state
                .as_ref()
                .context("SimCoupe SDL released state is missing")?;
            state.validate(
                device
                    .controls
                    .as_ref()
                    .context("SimCoupe SDL control counts are missing")?,
            )?;
            // Axes 0/1 drive directions, hats drive directions, and any
            // button drives fire. The session only proves the shapes; the
            // pad's own indices are consumed live by the source.
            for row in calibration.plan_profile(profile)?.rows {
                ensure!(
                    ROUTES.iter().any(|(target, _)| *target == row.target_id),
                    "SimCoupe target {} outside contract",
                    row.target_id
                );
                let input = row
                    .input
                    .as_ref()
                    .context("SimCoupe SAM control is not calibrated")?;
                let native = input
                    .native
                    .as_ref()
                    .context("SimCoupe requires measured native controls")?;
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
                    "Release the SimCoupe controls before launch preparation"
                );
                match row.target_id.as_str() {
                    "fire" => ensure!(
                        matches!(translated, DigitalInput::Button(_)),
                        "SimCoupe fire needs a raw button"
                    ),
                    _ => ensure!(
                        matches!(
                            translated,
                            DigitalInput::Axis { .. } | DigitalInput::Hat { .. }
                        ),
                        "SimCoupe directions need an axis or hat"
                    ),
                }
            }
            let directory = tempfile::Builder::new()
                .prefix("lunchbox-simcoupe-")
                .tempdir()?;
            // $HOME/.simcoupe/SimCoupe.cfg resolves under HOME; the launch
            // layer points it here so only the private config is visible.
            let dot = directory.path().join(".simcoupe");
            fs::create_dir(&dot)?;
            let config_path = dot.join(CONFIG_FILE);
            fs::write(
                &config_path,
                patch_config(&fs::read(&setup.config_source)?, sdl_name)?,
            )?;
            let mut hashes = BTreeMap::new();
            for path in [
                &setup.content,
                &setup.config_source,
                &setup.probe_program,
                &setup.sdl_library,
                &config_path,
            ] {
                hashes.insert(path.clone(), file_hash(path)?);
            }
            let config_home = directory.path().to_path_buf();
            // Windows resolves settings beside the executable, so the
            // session links the trusted executable here and mirrors the
            // staged config beside the link. Other hosts use the config
            // home through the environment override below.
            #[cfg(not(target_os = "linux"))]
            let executable_link = {
                let link = directory.path().join("simcoupe");
                #[cfg(unix)]
                std::os::unix::fs::symlink(executable, &link)?;
                #[cfg(target_os = "windows")]
                if std::os::windows::fs::symlink_file(executable, &link).is_err() {
                    fs::copy(executable, &link)?;
                }
                ensure!(
                    link.canonicalize()? == executable.canonicalize()?,
                    "SimCoupe executable link escapes the trusted runtime"
                );
                let staged = fs::read(&config_path)?;
                fs::write(directory.path().join(CONFIG_FILE), staged)?;
                hashes.insert(
                    directory.path().join(CONFIG_FILE),
                    file_hash(&directory.path().join(CONFIG_FILE))?,
                );
                link
            };
            let prepared = Self {
                directory,
                config_home,
                #[cfg(not(target_os = "linux"))]
                executable: executable_link,
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

        /// Private root for `HOME`; SimCoupe appends `/.simcoupe`.
        pub(crate) fn config_home(&self) -> &std::path::Path {
            &self.config_home
        }

        /// Session executable link for hosts that resolve settings beside
        /// the executable (Windows). Linux launches the real executable.
        #[cfg(not(target_os = "linux"))]
        pub(crate) fn executable(&self) -> &std::path::Path {
            &self.executable
        }

        pub(crate) fn verify(&self, cancel: &AtomicBool) -> Result<()> {
            cancelled(cancel)?;
            #[cfg(target_os = "linux")]
            self.topology.verify()?;
            for (path, hash) in &self.hashes {
                ensure!(file_hash(path)? == *hash, "SimCoupe launch input changed");
            }
            let fresh = routing(observe(&self.setup, None, cancel)?);
            self.initial.ensure_same_routing(&fresh)?;
            let captured = observe(&self.setup, Some(&self.physical_path), cancel)?;
            self.initial
                .ensure_same_routing(&routing(captured.clone()))?;
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
                "SimCoupe executable differs from the saved trusted runtime"
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
                "SimCoupe launch plan changed after preparation"
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
                    "SimCoupe exited before controller handoff"
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
                    "SimCoupe did not open the selected SDL controller before timeout"
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
            anyhow::bail!("SimCoupe calibrated launch requires a native build");
        };
        ensure!(
            option.emulator_name.eq_ignore_ascii_case("SimCoupe")
                && setup.emulator_id == option.emulator_id
                && original.environment.is_empty()
                && original.retroarch_content.is_none(),
            "SimCoupe identity differs or custom environment needs resolution"
        );
        let executable = executable.canonicalize()?;
        ensure!(
            executable == original.program.canonicalize()?,
            "SimCoupe launch executable differs from selection"
        );
        ensure!(
            original.arguments.as_slice() == [setup.content.as_os_str()],
            "SimCoupe calibrated launch requires exactly the saved content argument"
        );
        ensure!(
            file_hash(&executable)?.eq_ignore_ascii_case(&setup.executable_sha256),
            "SimCoupe executable differs from the saved trusted runtime"
        );
        let inputs =
            session::PreparedSession::prepare(setup, calibrations, inventory, &executable, cancel)?;
        let mut plan = original.clone();
        plan.program = executable.clone();
        // $HOME/.simcoupe/SimCoupe.cfg resolves under HOME; the disk image
        // keeps its default positional slot. Windows resolves beside the
        // executable link and carries the session user roots instead.
        #[cfg(target_os = "linux")]
        plan.environment.push((
            std::ffi::OsString::from("HOME"),
            inputs.config_home().as_os_str().to_owned(),
        ));
        // Windows resolves beside the executable link and carries the
        // session user roots instead.
        #[cfg(not(target_os = "linux"))]
        {
            platform::prepare_user_dirs(inputs.config_home())?;
            for (key, value) in platform::session_user_env(inputs.config_home()) {
                plan.environment.push((key, value));
            }
            plan.program = inputs.executable().to_path_buf();
        }
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
