//! Fuse standalone physical-joystick logical mapping writer.
//!
//! Pinned source: fuse-emulator/fuse commit
//! 5ba7804a44483466d7403a6e646a228da562ed5d. `settings.dat` and
//! `settings.pl` define both key/value and libxml2 XML spellings such as
//! `joystick1output` and `joystick1fire1`; the SDL2 frontend opens runtime
//! slots 0 and 1 and maps their axes/hats plus 15 buttons. The physical slots
//! cannot be reordered in config, so they must be measured for the exact
//! process immediately before launch.

use anyhow::{Context, Result, ensure};
use quick_xml::{
    Reader, Writer,
    events::{BytesEnd, BytesStart, BytesText, Event},
};
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};

pub(crate) const SOURCE_COMMIT: &str = "5ba7804a44483466d7403a6e646a228da562ed5d";
pub(crate) const PROFILE_ID: &str = "fuse:standalone-native-fixed-sdl-v1";

#[derive(Clone, Copy, Debug, Eq, PartialEq, serde::Serialize, serde::Deserialize)]
#[repr(u8)]
pub(crate) enum JoystickType {
    Cursor = 1,
    Kempston = 2,
    Sinclair1 = 3,
    Sinclair2 = 4,
    Timex1 = 5,
    Timex2 = 6,
    Fuller = 7,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum FireTarget {
    None,
    Space,
    Digit(u8),
    Letter(char),
    Enter,
    CapsShift,
    SymbolShift,
    JoystickFire,
}

impl FireTarget {
    fn value(self) -> Result<u16> {
        match self {
            Self::None => Ok(0),
            Self::Space => Ok(0x20),
            Self::Digit(digit) => {
                ensure!(digit <= 9, "Fuse Spectrum digit must be 0 through 9");
                Ok(u16::from(b'0' + digit))
            }
            Self::Letter(letter) => {
                ensure!(
                    letter.is_ascii_alphabetic(),
                    "Fuse Spectrum letter is invalid"
                );
                Ok(letter.to_ascii_lowercase() as u16)
            }
            Self::Enter => Ok(0x100),
            Self::CapsShift => Ok(0x101),
            Self::SymbolShift => Ok(0x102),
            Self::JoystickFire => Ok(0x1000),
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct PortProfile {
    /// The pinned SDL frontend recognizes only physical slots 0 and 1.
    pub physical_slot: u8,
    pub output: JoystickType,
    /// Physical buttons 1 through 15 in source order.
    pub fire_targets: [FireTarget; 15],
}

fn fields(profiles: &[PortProfile]) -> Result<BTreeMap<String, String>> {
    ensure!(
        !profiles.is_empty() && profiles.len() <= 2,
        "Fuse needs one or two physical joystick profiles"
    );
    let mut slots = BTreeSet::new();
    let mut fields = BTreeMap::new();
    for profile in profiles {
        ensure!(profile.physical_slot <= 1, "Fuse SDL slot must be 0 or 1");
        ensure!(
            slots.insert(profile.physical_slot),
            "Fuse SDL slot is duplicated"
        );
        let number = profile.physical_slot + 1;
        fields.insert(
            format!("joystick{number}output"),
            (profile.output as u8).to_string(),
        );
        for (index, target) in profile.fire_targets.iter().enumerate() {
            fields.insert(
                format!("joystick{number}fire{}", index + 1),
                target.value()?.to_string(),
            );
        }
    }
    Ok(fields)
}

/// Patch a copied Fuse `fuserc`/`fuse.cfg`, supporting both source-defined
/// key/value and libxml2 `<settings>` encodings. ROM, media, UI, machine and
/// snapshot settings remain untouched.
pub(crate) fn patch_config(baseline: &[u8], profiles: &[PortProfile]) -> Result<String> {
    ensure!(
        baseline.len() <= 4 * 1024 * 1024,
        "Fuse config is too large"
    );
    let original = std::str::from_utf8(baseline).context("Fuse config is not UTF-8")?;
    ensure!(!original.contains('\0'), "Fuse config contains a NUL byte");
    let fields = fields(profiles)?;
    if original.trim_start().starts_with('<') {
        patch_xml(original, &fields)
    } else {
        patch_key_value(original, &fields)
    }
}

fn patch_key_value(original: &str, fields: &BTreeMap<String, String>) -> Result<String> {
    let newline = if original.contains("\r\n") {
        "\r\n"
    } else {
        "\n"
    };
    let mut output = String::new();
    let mut seen = BTreeSet::new();
    for line in original.split_inclusive('\n') {
        let owned = line.split_once('=').and_then(|(key, _)| {
            fields
                .keys()
                .find(|known| key.trim() == known.as_str())
                .cloned()
        });
        if let Some(key) = owned {
            ensure!(seen.insert(key.clone()), "Fuse config key is duplicated");
            output.push_str(&format!("{key}={}{newline}", fields[&key]));
        } else {
            output.push_str(line);
        }
    }
    if !output.is_empty() && !output.ends_with('\n') {
        output.push_str(newline);
    }
    for (key, value) in fields {
        if !seen.contains(key) {
            output.push_str(&format!("{key}={value}{newline}"));
        }
    }
    Ok(output)
}

fn patch_xml(original: &str, fields: &BTreeMap<String, String>) -> Result<String> {
    let mut reader = Reader::from_str(original);
    let mut writer = Writer::new(Vec::with_capacity(original.len() + fields.len() * 40));
    let mut stack: Vec<Vec<u8>> = Vec::new();
    let mut roots = 0;
    let mut seen = BTreeSet::new();
    loop {
        let event = reader.read_event()?;
        match event {
            Event::Start(ref element) => {
                let name = element.name().as_ref().to_vec();
                if stack.is_empty() {
                    roots += 1;
                    ensure!(
                        roots == 1 && name == b"settings",
                        "Fuse XML root must be settings"
                    );
                }
                stack.push(name);
                ensure!(stack.len() <= 8, "Fuse XML nesting is too deep");
                writer.write_event(event.into_owned())?;
            }
            Event::Empty(ref element) if stack.len() == 1 && stack[0] == b"settings" => {
                let key = std::str::from_utf8(element.name().as_ref())?.to_owned();
                if let Some(value) = fields.get(&key) {
                    ensure!(seen.insert(key.clone()), "Fuse XML key is duplicated");
                    writer.write_event(Event::Start(BytesStart::new(&key)))?;
                    writer.write_event(Event::Text(BytesText::new(value)))?;
                    writer.write_event(Event::End(BytesEnd::new(&key)))?;
                } else {
                    writer.write_event(event.into_owned())?;
                }
            }
            Event::Text(_) if stack.len() == 2 && stack[0] == b"settings" => {
                let key = std::str::from_utf8(&stack[1])?;
                if let Some(value) = fields.get(key) {
                    ensure!(seen.insert(key.to_owned()), "Fuse XML key is duplicated");
                    writer.write_event(Event::Text(BytesText::new(value)))?;
                } else {
                    writer.write_event(event.into_owned())?;
                }
            }
            Event::End(ref element) => {
                ensure!(
                    stack
                        .last()
                        .is_some_and(|name| name == element.name().as_ref()),
                    "Fuse XML is unbalanced"
                );
                if stack.len() == 1 && stack[0] == b"settings" {
                    for (key, value) in fields {
                        if !seen.contains(key) {
                            writer.write_event(Event::Start(BytesStart::new(key)))?;
                            writer.write_event(Event::Text(BytesText::new(value)))?;
                            writer.write_event(Event::End(BytesEnd::new(key)))?;
                        }
                    }
                }
                writer.write_event(event.into_owned())?;
                stack.pop();
            }
            Event::DocType(_) => anyhow::bail!("Fuse XML DTD is unsupported"),
            Event::Eof => {
                ensure!(roots == 1 && stack.is_empty(), "Fuse XML is incomplete");
                break;
            }
            _ => writer.write_event(event.into_owned())?,
        }
    }
    String::from_utf8(writer.into_inner()).context("Fuse XML output is not UTF-8")
}

pub(crate) fn source_boundary() -> &'static str {
    "Fuse config controls guest joystick type and all 15 physical fire-button targets, but the pinned SDL2 frontend fixes physical devices to runtime slots 0 and 1. Verify those slots against the intended pads for the exact child immediately before launch. Preserve ROM directories, writable media, snapshots and recordings; native Flatpak remains unverified."
}

#[cfg(test)]
mod tests {
    use super::*;

    fn profile() -> PortProfile {
        let mut targets = [FireTarget::None; 15];
        targets[0] = FireTarget::JoystickFire;
        targets[1] = FireTarget::Enter;
        targets[2] = FireTarget::Letter('m');
        PortProfile {
            physical_slot: 0,
            output: JoystickType::Kempston,
            fire_targets: targets,
        }
    }

    #[test]
    fn patches_key_value_and_xml_grammars() {
        let plain = patch_config(
            b"machine=48\njoystick1output=0\nvolumeay=80\n",
            &[profile()],
        )
        .unwrap();
        assert!(plain.contains("joystick1output=2\n"));
        assert!(plain.contains("joystick1fire1=4096\n"));
        assert!(plain.contains("joystick1fire2=256\n"));
        assert!(plain.contains("volumeay=80\n"));

        let xml = patch_config(
            b"<?xml version=\"1.0\"?><settings><machine>48</machine><joystick1output>0</joystick1output></settings>",
            &[profile()],
        )
        .unwrap();
        assert!(xml.contains("<machine>48</machine>"));
        assert!(xml.contains("<joystick1output>2</joystick1output>"));
        assert!(xml.contains("<joystick1fire1>4096</joystick1fire1>"));
    }

    #[test]
    fn rejects_unrecognized_physical_slots_and_targets() {
        let mut bad = profile();
        bad.physical_slot = 2;
        assert!(patch_config(b"", &[bad]).is_err());
        let mut bad = profile();
        bad.fire_targets[0] = FireTarget::Digit(10);
        assert!(patch_config(b"", &[bad]).is_err());
    }
}

/// Layout target ids covered by the native profile.
pub(crate) const ROUTES: [(&str, &str); 5] = [
    ("up", "Up"),
    ("down", "Down"),
    ("left", "Left"),
    ("right", "Right"),
    ("fire", "Fire"),
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
        pub joystick_type: JoystickType,
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
                "Fuse setup needs an emulator identity"
            );
            for path in [&self.content, &self.probe_program, &self.sdl_library] {
                ensure!(
                    path.is_absolute()
                        && !path
                            .components()
                            .any(|part| matches!(part, std::path::Component::ParentDir)),
                    "Fuse setup paths must be absolute without parent traversal"
                );
            }
            ensure!(
                self.executable_sha256.len() == 64
                    && self
                        .executable_sha256
                        .bytes()
                        .all(|byte| byte.is_ascii_hexdigit()),
                "Fuse setup needs a trusted executable SHA-256"
            );
            let limit = catalog()
                .emulator_profiles
                .iter()
                .find(|profile| profile.id == PROFILE_ID)
                .and_then(|profile| profile.native_launch.as_ref())
                .context("Missing Fuse native profile")?
                .max_players;
            ensure!(
                !self.players.is_empty() && self.players.len() <= limit,
                "Fuse setup needs one or two players"
            );
            let mut controllers = BTreeSet::new();
            for (index, player) in self.players.iter().enumerate() {
                ensure!(
                    usize::from(player.player) == index + 1
                        && !player.controller_id.trim().is_empty()
                        && controllers.insert(&player.controller_id),
                    "Fuse players must be distinct, contiguous, and start at player one"
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
                .context("Missing Fuse native profile")?;
            let mut players = Vec::new();
            for player in &self.players {
                let calibration = calibrations
                    .get(&player.controller_id)
                    .context("Fuse controller has no saved calibration")?;
                ensure!(
                    calibration.os == "linux",
                    "Fuse mapping requires Linux physical calibration"
                );
                let mapping = calibration.plan_profile(profile)?;
                ensure!(
                    mapping.rows.iter().all(|row| row.physical_id.is_some()
                        && row
                            .input
                            .as_ref()
                            .is_some_and(|input| input.native.is_some())),
                    "Fuse needs native calibration for every joystick control"
                );
                players.push(serde_json::json!({
                    "player": player.player,
                    "controller_id": player.controller_id,
                    "joystick_type": player.joystick_type as u8,
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
                "detail": "Native Linux launch patches a private fuserc with fixed-slot joystick types and fire targets, then rechecks the exact SDL routes. Only Kempston-style fire on slots 0/1 is supported; runtime behavior remains unverified."
            }))
        }
    }

    pub(crate) fn validate_setups(setups: &[SavedSetup]) -> Result<()> {
        ensure!(setups.len() <= 1024, "Too many Fuse saved setups");
        let mut identities = BTreeSet::new();
        for setup in setups {
            setup.validate()?;
            ensure!(
                identities.insert((&setup.emulator_id, &setup.content)),
                "Duplicate Fuse emulator/content setup"
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
            serde_json::from_slice(&output).context("Invalid Fuse SDL2 capture")?;
        ensure!(
            snapshot.version[0] == 2
                && snapshot.library.canonicalize()? == setup.sdl_library.canonicalize()?
                && snapshot.library_sha256 == file_hash(&setup.sdl_library)?,
            "Fuse helper inspected a different SDL2 runtime"
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

    fn fire_profile(
        calibration: &Calibration,
        device: &Device,
        slot: u8,
        output: JoystickType,
    ) -> Result<PortProfile> {
        let profile = crate::controller_catalog::catalog()
            .emulator_profiles
            .iter()
            .find(|profile| profile.id == PROFILE_ID)
            .context("Missing Fuse native profile")?;
        let physical = PhysicalMap::from_device(device)?;
        let state = device
            .sampled_state
            .as_ref()
            .context("Fuse SDL released state is missing")?;
        state.validate(
            device
                .controls
                .as_ref()
                .context("Fuse SDL control counts are missing")?,
        )?;
        // Directions are source-fixed to the slot's axes; only the fire
        // button is translated, landing on its own raw index so the game
        // sees the pad's own fire button. All other fire slots stay None.
        let mut fire_index = None;
        for row in calibration.plan_profile(profile)?.rows {
            ensure!(
                ROUTES.iter().any(|(target, _)| *target == row.target_id),
                "Fuse target {} outside contract",
                row.target_id
            );
            let input = row
                .input
                .as_ref()
                .context("Fuse joystick control is not calibrated")?;
            let native = input
                .native
                .as_ref()
                .context("Fuse requires measured native controls")?;
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
                "Release the Fuse controls before launch preparation"
            );
            if row.target_id == "fire" {
                let DigitalInput::Button(index) = translated else {
                    anyhow::bail!("Fuse fire must be a raw button")
                };
                ensure!(
                    index < 15,
                    "Fuse fire button is outside the fifteen-button table"
                );
                ensure!(
                    fire_index.replace(index as u8).is_none(),
                    "Fuse fire appears twice"
                );
            }
        }
        let fire = fire_index.context("Fuse fire control is not calibrated")?;
        let mut fire_targets = [FireTarget::None; 15];
        fire_targets[usize::from(fire)] = FireTarget::JoystickFire;
        Ok(PortProfile {
            physical_slot: slot,
            output,
            fire_targets,
        })
    }

    pub(crate) struct PreparedSession {
        directory: tempfile::TempDir,
        config_home: PathBuf,
        physical_paths: Vec<String>,
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
                "Fuse content must be a direct regular file with canonical ancestry"
            );
            let mut selected = Vec::new();
            for player in &setup.players {
                let found = inventory
                    .iter()
                    .filter(|device| device.stable_id == player.controller_id)
                    .collect::<Vec<_>>();
                ensure!(
                    found.len() == 1 && !found[0].is_virtual,
                    "Fuse physical controller is missing or ambiguous"
                );
                selected.push(found[0].device_path.clone());
            }
            let topology = InputTopology::capture(&selected)?;
            let initial = routing(observe(setup, None, cancel)?);
            // The pinned SDL frontend fixes physical devices to runtime
            // slots 0 and 1; the probe reports the same SDL enumeration
            // order, so player N must hold slot N-1.
            let mut physical_paths = Vec::new();
            let mut profiles = Vec::new();
            for (player, selected_path) in setup.players.iter().zip(&selected) {
                let path = topology.resolve_runtime_path(
                    selected_path,
                    initial
                        .devices
                        .iter()
                        .filter_map(|device| device.path.as_deref()),
                )?;
                ensure!(
                    !physical_paths.contains(&path),
                    "Fuse players share a controller"
                );
                let captured = observe(setup, Some(&path), cancel)?;
                initial.ensure_same_routing(&routing(captured.clone()))?;
                topology.verify()?;
                let device = captured.device_at_path(&path)?;
                ensure!(
                    device.device_index == u32::from(player.player) - 1,
                    "Fuse player {} needs SDL slot {}, found {}",
                    player.player,
                    player.player - 1,
                    device.device_index
                );
                profiles.push(fire_profile(
                    calibrations
                        .get(&player.controller_id)
                        .context("Fuse calibration disappeared")?,
                    device,
                    u8::try_from(device.device_index).context("Fuse SDL slot is too large")?,
                    player.joystick_type,
                )?);
                physical_paths.push(path);
            }
            let directory = tempfile::Builder::new()
                .prefix("lunchbox-fuse-")
                .tempdir()?;
            // compat_get_config_path resolves
            // $XDG_CONFIG_HOME/fuse-emulator; the launch layer points it
            // here so only the private fuserc is visible.
            let config_dir = directory.path().join("fuse-emulator");
            fs::create_dir(&config_dir)?;
            let config_path = config_dir.join("fuserc");
            let baseline = "joystick1output=0\n";
            fs::write(&config_path, patch_config(baseline.as_bytes(), &profiles)?)?;
            let mut hashes = BTreeMap::new();
            for path in [
                &setup.content,
                &setup.probe_program,
                &setup.sdl_library,
                &config_path,
            ] {
                hashes.insert(path.clone(), file_hash(path)?);
            }
            let config_home = directory.path().to_path_buf();
            let prepared = Self {
                directory,
                config_home,
                physical_paths,
                topology,
                initial,
                setup: setup.clone(),
                hashes,
            };
            prepared.verify(cancel)?;
            Ok(prepared)
        }

        /// Private root for `XDG_CONFIG_HOME`; the source appends
        /// `/fuse-emulator` itself.
        pub(crate) fn config_home(&self) -> &std::path::Path {
            &self.config_home
        }

        pub(crate) fn verify(&self, cancel: &AtomicBool) -> Result<()> {
            cancelled(cancel)?;
            self.topology.verify()?;
            for (path, hash) in &self.hashes {
                ensure!(file_hash(path)? == *hash, "Fuse launch input changed");
            }
            let fresh = routing(observe(&self.setup, None, cancel)?);
            self.initial.ensure_same_routing(&fresh)?;
            for (index, path) in self.physical_paths.iter().enumerate() {
                let captured = observe(&self.setup, Some(path), cancel)?;
                self.initial
                    .ensure_same_routing(&routing(captured.clone()))?;
                let device = captured.device_at_path(path)?;
                ensure!(
                    device.device_index == index as u32,
                    "Fuse SDL slot order moved"
                );
                self.topology.verify()?;
            }
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
                "Fuse executable differs from the saved trusted runtime"
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
                "Fuse launch plan changed after preparation"
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
                    "Fuse exited before controller handoff"
                );
                if let Some(pid) = native_pid(child.id(), &self.executable)?
                    && self.ready(pid)?
                {
                    self.inputs.check_health()?;
                    return Ok(());
                }
                ensure!(
                    Instant::now() < deadline,
                    "Fuse did not open the selected SDL controllers before timeout"
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
            anyhow::bail!("Fuse calibrated launch requires native Linux");
        };
        ensure!(
            option.emulator_name.eq_ignore_ascii_case("Fuse")
                && setup.emulator_id == option.emulator_id
                && original.environment.is_empty()
                && original.retroarch_content.is_none(),
            "Fuse identity differs or custom environment needs resolution"
        );
        let executable = executable.canonicalize()?;
        ensure!(
            executable == original.program.canonicalize()?,
            "Fuse launch executable differs from selection"
        );
        ensure!(
            original.arguments.as_slice() == [setup.content.as_os_str()],
            "Fuse calibrated launch requires exactly the saved content argument"
        );
        ensure!(
            file_hash(&executable)?.eq_ignore_ascii_case(&setup.executable_sha256),
            "Fuse executable differs from the saved trusted runtime"
        );
        let inputs = session::PreparedSession::prepare(setup, calibrations, inventory, cancel)?;
        let mut plan = original.clone();
        plan.program = executable.clone();
        // compat_get_config_path resolves $XDG_CONFIG_HOME/fuse-emulator;
        // the tape/snapshot positional arg keeps its default slot.
        plan.environment.push((
            std::ffi::OsString::from("XDG_CONFIG_HOME"),
            inputs.config_home().as_os_str().to_owned(),
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
