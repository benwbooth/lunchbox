//! Caprice32 standalone SDL joystick-enablement writer.
//!
//! Pinned source: ColinPitrat/caprice32 commit
//! 6c12c4c92360065cdc229ac9ada7551f941436b8. `cap32.cpp` persists the
//! `[system]` joystick switches, opens the first eight SDL devices, and
//! `keyboard.cpp` routes event instance 0/1 to CPC joystick 0/1 with fixed
//! axes 0/1 (also 2/3) and buttons 0/1. There is no device selector to write,
//! so callers must prove that the selected pads occupy those exact instances
//! for the exact child process.

use anyhow::{Context, Result, ensure};
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};

pub(crate) const SOURCE_COMMIT: &str = "6c12c4c92360065cdc229ac9ada7551f941436b8";
pub(crate) const PROFILE_ID: &str = "caprice32:standalone-caprice32-cpc";

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) struct FixedSdlSelection {
    /// SDL event instance IDs observed for CPC joystick 0 and optionally 1.
    /// The pinned mapper recognizes only instance IDs 0 and 1.
    pub instance_ids: [Option<u8>; 2],
    /// One-based values persisted by Caprice32; internally it subtracts one.
    pub menu_button: u8,
    pub virtual_keyboard_button: u8,
}

fn fields(selection: FixedSdlSelection) -> Result<BTreeMap<&'static str, String>> {
    ensure!(
        selection.instance_ids[0] == Some(0),
        "Caprice32 CPC joystick 0 requires measured SDL instance 0"
    );
    ensure!(
        selection.instance_ids[1].is_none() || selection.instance_ids[1] == Some(1),
        "Caprice32 CPC joystick 1 requires measured SDL instance 1"
    );
    ensure!(
        (1..=64).contains(&selection.menu_button)
            && (1..=64).contains(&selection.virtual_keyboard_button),
        "Caprice32 global buttons must be one-based values from 1 through 64"
    );
    ensure!(
        selection.menu_button != selection.virtual_keyboard_button,
        "Caprice32 menu and virtual-keyboard buttons must differ"
    );
    Ok(BTreeMap::from([
        ("joysticks", "1".into()),
        ("joystick_emulation", "0".into()),
        ("joystick_menu_button", selection.menu_button.to_string()),
        (
            "joystick_vkeyboard_button",
            selection.virtual_keyboard_button.to_string(),
        ),
    ]))
}

/// Patch only Caprice32's source-defined `[system]` joystick keys in a copied
/// `cap32.cfg`. Device ordering is not encoded by this file: the launch layer
/// must re-probe the exact SDL backend and fail if the intended controller(s)
/// do not have event instance IDs 0 and optionally 1 immediately before exec.
pub(crate) fn patch_config(baseline: &[u8], selection: FixedSdlSelection) -> Result<String> {
    ensure!(
        baseline.len() <= 1024 * 1024,
        "Caprice32 config is too large"
    );
    let original = std::str::from_utf8(baseline).context("Caprice32 config is not UTF-8")?;
    ensure!(
        !original.contains('\0'),
        "Caprice32 config contains a NUL byte"
    );
    let fields = fields(selection)?;
    patch_section(original, "system", &fields)
}

fn patch_section(original: &str, section: &str, fields: &BTreeMap<&str, String>) -> Result<String> {
    let newline = if original.contains("\r\n") {
        "\r\n"
    } else {
        "\n"
    };
    let mut output = String::new();
    let mut active = false;
    let mut found = false;
    let mut inserted = false;
    let mut seen = BTreeSet::new();
    for line in original.split_inclusive('\n') {
        if let Some(header) = line
            .trim_start()
            .strip_prefix('[')
            .and_then(|line| line.split_once(']').map(|(name, _)| name.trim()))
        {
            if active && !inserted {
                for (key, value) in fields {
                    if !seen.contains(*key) {
                        output.push_str(&format!("{key}={value}{newline}"));
                    }
                }
                inserted = true;
            }
            let matching = header.eq_ignore_ascii_case(section);
            if matching {
                ensure!(!found, "Caprice32 config section is duplicated");
                found = true;
            }
            active = matching;
            output.push_str(line);
        } else if active {
            let owned = line.split_once('=').and_then(|(key, _)| {
                fields
                    .keys()
                    .find(|known| key.trim().eq_ignore_ascii_case(known))
                    .copied()
            });
            if let Some(key) = owned {
                ensure!(seen.insert(key), "Caprice32 config key is duplicated");
                output.push_str(&format!("{key}={}{newline}", fields[key]));
            } else {
                output.push_str(line);
            }
        } else {
            output.push_str(line);
        }
    }
    if !inserted {
        if !output.is_empty() && !output.ends_with('\n') {
            output.push_str(newline);
        }
        if !active {
            output.push_str(&format!("[{section}]{newline}"));
        }
        for (key, value) in fields {
            if !seen.contains(*key) {
                output.push_str(&format!("{key}={value}{newline}"));
            }
        }
    }
    Ok(output)
}

pub(crate) fn source_boundary() -> &'static str {
    "The pinned executable has a fixed first-two-SDL-device contract, not a persistent device identity grammar. Re-probe event instance IDs for the exact child immediately before launch. Preserve all other cap32.cfg settings, writable .dsk media and .sna snapshots; Caprice32 libretro and native Flatpak remain separate contracts."
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn enables_fixed_sdl_mapping_and_preserves_other_sections() {
        let output = patch_config(
            b"[system]\njoysticks=0\njoystick_emulation=1\nspeed=4\n[video]\nscr_scale=3\n",
            FixedSdlSelection {
                instance_ids: [Some(0), Some(1)],
                menu_button: 9,
                virtual_keyboard_button: 10,
            },
        )
        .unwrap();
        assert!(output.contains("joysticks=1\n"));
        assert!(output.contains("joystick_emulation=0\n"));
        assert!(output.contains("joystick_menu_button=9\n"));
        assert!(output.contains("speed=4\n"));
        assert!(output.contains("[video]\nscr_scale=3\n"));
    }

    #[test]
    fn rejects_unmatched_instances_and_button_collisions() {
        let mut selection = FixedSdlSelection {
            instance_ids: [Some(2), None],
            menu_button: 9,
            virtual_keyboard_button: 10,
        };
        assert!(patch_config(b"", selection).is_err());
        selection.instance_ids = [Some(0), None];
        selection.virtual_keyboard_button = 9;
        assert!(patch_config(b"", selection).is_err());
        selection.virtual_keyboard_button = 10;
        assert!(patch_config(b"[system]\n[system]\n", selection).is_err());
    }
}

/// Layout target ids covered by the native profile.
pub(crate) const ROUTES: [(&str, &str); 8] = [
    ("up", "Up"),
    ("down", "Down"),
    ("left", "Left"),
    ("right", "Right"),
    ("a", "Fire 1"),
    ("b", "Fire 2"),
    ("start", "Menu"),
    ("select", "Virtual keyboard"),
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
                "Caprice32 setup needs an emulator identity"
            );
            for path in [&self.content, &self.probe_program, &self.sdl_library] {
                ensure!(
                    path.is_absolute()
                        && !path
                            .components()
                            .any(|part| matches!(part, std::path::Component::ParentDir)),
                    "Caprice32 setup paths must be absolute without parent traversal"
                );
            }
            ensure!(
                self.executable_sha256.len() == 64
                    && self
                        .executable_sha256
                        .bytes()
                        .all(|byte| byte.is_ascii_hexdigit()),
                "Caprice32 setup needs a trusted executable SHA-256"
            );
            let limit = catalog()
                .emulator_profiles
                .iter()
                .find(|profile| profile.id == PROFILE_ID)
                .and_then(|profile| profile.native_launch.as_ref())
                .context("Missing Caprice32 native profile")?
                .max_players;
            ensure!(
                !self.players.is_empty() && self.players.len() <= limit,
                "Caprice32 setup needs one or two players"
            );
            let mut controllers = BTreeSet::new();
            for (index, player) in self.players.iter().enumerate() {
                ensure!(
                    usize::from(player.player) == index + 1
                        && !player.controller_id.trim().is_empty()
                        && controllers.insert(&player.controller_id),
                    "Caprice32 players must be distinct, contiguous, and start at player one"
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
                .context("Missing Caprice32 native profile")?;
            let mut players = Vec::new();
            for player in &self.players {
                let calibration = calibrations
                    .get(&player.controller_id)
                    .context("Caprice32 controller has no saved calibration")?;
                ensure!(
                    ["linux", "macos", "windows"].contains(&calibration.os.as_str()),
                    "Caprice32 mapping requires Linux physical calibration"
                );
                let mapping = calibration.plan_profile(profile)?;
                ensure!(
                    mapping.rows.iter().all(|row| row.physical_id.is_some()
                        && row
                            .input
                            .as_ref()
                            .is_some_and(|input| input.native.is_some())),
                    "Caprice32 needs native calibration for every CPC control"
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
                "detail": "Native Linux launch patches a private cap32.cfg [system] block, then rechecks the exact SDL instance order. Only fixed CPC joystick ports with two menu buttons are supported; runtime behavior remains unverified."
            }))
        }
    }

    pub(crate) fn validate_setups(setups: &[SavedSetup]) -> Result<()> {
        ensure!(setups.len() <= 1024, "Too many Caprice32 saved setups");
        let mut identities = BTreeSet::new();
        for setup in setups {
            setup.validate()?;
            ensure!(
                identities.insert((&setup.emulator_id, &setup.content)),
                "Duplicate Caprice32 emulator/content setup"
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
            serde_json::from_slice(&output).context("Invalid Caprice32 SDL2 capture")?;
        ensure!(
            snapshot.version[0] == 2
                && snapshot.library.canonicalize()? == setup.sdl_library.canonicalize()?
                && snapshot.library_sha256 == file_hash(&setup.sdl_library)?,
            "Caprice32 helper inspected a different SDL2 runtime"
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
                "Caprice32 content must be a direct regular file with canonical ancestry"
            );
            let mut selected = Vec::new();
            for player in &setup.players {
                let found = inventory
                    .iter()
                    .filter(|device| device.stable_id == player.controller_id)
                    .collect::<Vec<_>>();
                ensure!(
                    found.len() == 1 && !found[0].is_virtual,
                    "Caprice32 physical controller is missing or ambiguous"
                );
                selected.push(found[0].device_path.clone());
            }
            #[cfg(target_os = "linux")]
            let topology = InputTopology::capture(&selected)?;
            let initial = routing(observe(setup, None, cancel)?);
            // The pinned mapper recognizes only event instance IDs 0 and 1
            // for CPC joystick ports 0 and 1; the probe reports the same
            // SDL instance IDs, so the selected pads must hold those slots.
            let mut physical_paths = Vec::new();
            let mut menu_buttons: Option<(u8, u8)> = None;
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
                        "Caprice32 physical controller is missing or ambiguous in SDL"
                    );
                    selected_string
                };
                ensure!(
                    !physical_paths.contains(&path),
                    "Caprice32 players share a controller"
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
                let expected_instance = i32::from(player.player) - 1;
                ensure!(
                    device.instance_id == expected_instance,
                    "Caprice32 player {} needs SDL instance {expected_instance}, found {}",
                    player.player,
                    device.instance_id
                );
                let calibration = calibrations
                    .get(&player.controller_id)
                    .context("Caprice32 calibration disappeared")?;
                let profile = crate::controller_catalog::catalog()
                    .emulator_profiles
                    .iter()
                    .find(|profile| profile.id == PROFILE_ID)
                    .context("Missing Caprice32 native profile")?;
                // Directions and fire buttons stay fixed; the session only
                // proves the pad holds its required instance slot and that
                // every profile control has native calibration. The two
                // global menu buttons come from player one's start/select.
                let rows = calibration.plan_profile(profile)?.rows;
                ensure!(
                    ROUTES
                        .iter()
                        .all(|(target, _)| rows.iter().any(|row| &row.target_id == target
                            && row
                                .input
                                .as_ref()
                                .is_some_and(|input| input.native.is_some()))),
                    "Caprice32 needs native calibration for every CPC control"
                );
                if menu_buttons.is_none() {
                    let mut pair = Vec::new();
                    for target in ["start", "select"] {
                        let row = rows
                            .iter()
                            .find(|row| row.target_id == target)
                            .with_context(|| {
                                format!("Caprice32 control {target} is not calibrated")
                            })?;
                        let native = row
                            .input
                            .as_ref()
                            .context("Caprice32 menu control is not calibrated")?
                            .native
                            .as_ref()
                            .context("Caprice32 requires measured native controls")?;
                        ensure!(
                            native.code >> 16 == 1,
                            "Caprice32 menu buttons must be raw buttons"
                        );
                        let raw = (native.code & 0xffff) as u64;
                        ensure!(raw <= 63, "Caprice32 menu button is out of range");
                        // Persisted values are one-based; the source subtracts one.
                        pair.push((raw + 1) as u8);
                    }
                    ensure!(
                        pair[0] != pair[1],
                        "Caprice32 menu and virtual-keyboard buttons must differ"
                    );
                    menu_buttons = Some((pair[0], pair[1]));
                }
                let _ = device as &Device;
                device_indices.push(device.device_index);
                physical_paths.push(path);
            }
            let (menu_button, vkeyboard_button) =
                menu_buttons.context("Caprice32 menu buttons were never resolved")?;
            let directory = tempfile::Builder::new()
                .prefix("lunchbox-caprice32-")
                .tempdir()?;
            let config_path = directory.path().join("cap32.cfg");
            // A minimal baseline carries the source-defined [system] block;
            // the patcher preserves unknown settings verbatim.
            let baseline = "[system]\njoysticks=0\n";
            fs::write(
                &config_path,
                patch_config(
                    baseline.as_bytes(),
                    FixedSdlSelection {
                        instance_ids: [
                            Some(0),
                            if setup.players.len() > 1 {
                                Some(1)
                            } else {
                                None
                            },
                        ],
                        menu_button,
                        virtual_keyboard_button: vkeyboard_button,
                    },
                )?,
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
                ensure!(file_hash(path)? == *hash, "Caprice32 launch input changed");
            }
            let fresh = routing(observe(&self.setup, None, cancel)?);
            self.initial.ensure_same_routing(&fresh)?;
            for (index, path) in self.physical_paths.iter().enumerate() {
                let captured = observe(&self.setup, Some(path), cancel)?;
                self.initial
                    .ensure_same_routing(&routing(captured.clone()))?;
                let device = captured.device_at_path(path)?;
                ensure!(
                    device.instance_id == index as i32,
                    "Caprice32 SDL instance order moved"
                );
                #[cfg(not(target_os = "linux"))]
                platform::require_unique_device_path(&captured.devices, path, device.device_index)?;
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
            for path in &self.physical_paths {
                let count = fresh
                    .devices
                    .iter()
                    .filter(|device| device.path.as_deref() == Some(path.as_str()))
                    .count();
                ensure!(
                    count == 1,
                    "Caprice32 SDL device path is missing or ambiguous"
                );
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
                "Caprice32 executable differs from the saved trusted runtime"
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
                "Caprice32 launch plan changed after preparation"
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
                    "Caprice32 exited before controller handoff"
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
                    "Caprice32 did not open the selected SDL controllers before timeout"
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
            anyhow::bail!("Caprice32 calibrated launch requires a native build");
        };
        ensure!(
            option.emulator_name.eq_ignore_ascii_case("Caprice32")
                && setup.emulator_id == option.emulator_id
                && original.environment.is_empty()
                && original.retroarch_content.is_none(),
            "Caprice32 identity differs or custom environment needs resolution"
        );
        let executable = executable.canonicalize()?;
        ensure!(
            executable == original.program.canonicalize()?,
            "Caprice32 launch executable differs from selection"
        );
        ensure!(
            original.arguments.as_slice() == [setup.content.as_os_str()],
            "Caprice32 calibrated launch requires exactly the saved content argument"
        );
        ensure!(
            file_hash(&executable)?.eq_ignore_ascii_case(&setup.executable_sha256),
            "Caprice32 executable differs from the saved trusted runtime"
        );
        let inputs = session::PreparedSession::prepare(setup, calibrations, inventory, cancel)?;
        // The search order starts with the explicit -c path, so a private
        // file fully isolates joystick settings. ROM positional args follow.
        let mut plan = original.clone();
        plan.program = executable.clone();
        plan.arguments = vec![
            std::ffi::OsString::from("-c"),
            inputs.config_path().as_os_str().to_owned(),
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
