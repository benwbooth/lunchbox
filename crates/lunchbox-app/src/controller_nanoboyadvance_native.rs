//! NanoBoyAdvance standalone Qt/SDL3 controller mapping and guarded launch.
//!
//! Functional contract pinned to nba-emu/NanoBoyAdvance
//! 55b5cf0ae3d929582ac5bfd486558173502b8354. The Qt frontend stores one SDL
//! joystick GUID and raw button/axis/hat indices in `config.toml`. Because the
//! upstream GUID lookup takes the first match, a launch is safe only while the
//! selected GUID is unique in the exact SDL3 inventory.

use anyhow::{Context, Result, ensure};
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};
use toml_edit::{Array, DocumentMut, Item, Table, value};

pub(crate) const SOURCE_COMMIT: &str = "55b5cf0ae3d929582ac5bfd486558173502b8354";
pub(crate) const PROFILE_ID: &str = "nanoboyadvance:standalone-gba-controller";

/// GBA controls and their `QtConfig::Input::gba` TOML keys.
pub(crate) const CONTROLS: [(&str, &str); 10] = [
    ("a", "input.gba.a"),
    ("b", "input.gba.b"),
    ("select", "input.gba.select"),
    ("start", "input.gba.start"),
    ("right", "input.gba.right"),
    ("left", "input.gba.left"),
    ("up", "input.gba.up"),
    ("down", "input.gba.down"),
    ("r", "input.gba.r"),
    ("l", "input.gba.l"),
];

const DEFAULT_KEYBOARD: [i32; 10] = [
    65, 83, 16_777_219, 16_777_220, 16_777_236, 16_777_234, 16_777_235, 16_777_237, 70, 68,
];
const DIGITAL_THRESHOLD: i32 = i16::MAX as i32 / 2;

/// Raw SDL3 joystick fields in NanoBoyAdvance's five-int map.
#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub(crate) enum Binding {
    Button(u32),
    Axis { index: u32, negative: bool },
    Hat { index: u32, direction: u8 },
}

impl Binding {
    fn fields(self) -> Result<(i32, i32, i32, i32)> {
        match self {
            Self::Button(index) => {
                let index = i32::try_from(index).context("NanoBoyAdvance button index overflow")?;
                ensure!(
                    index <= u8::MAX.into(),
                    "NanoBoyAdvance SDL button index is out of range"
                );
                Ok((index, -1, -1, 0))
            }
            Self::Axis { index, negative } => {
                let index = i32::try_from(index).context("NanoBoyAdvance axis index overflow")?;
                ensure!(
                    index <= u8::MAX.into(),
                    "NanoBoyAdvance SDL axis index is out of range"
                );
                Ok((-1, index | if negative { 0x100 } else { 0 }, -1, 0))
            }
            Self::Hat { index, direction } => {
                let index = i32::try_from(index).context("NanoBoyAdvance hat index overflow")?;
                ensure!(
                    index <= u8::MAX.into(),
                    "NanoBoyAdvance SDL hat index is out of range"
                );
                ensure!(
                    matches!(direction, 1 | 2 | 4 | 8),
                    "NanoBoyAdvance hat direction must be cardinal"
                );
                Ok((-1, -1, index, i32::from(direction)))
            }
        }
    }
}

fn guid_valid(guid: &str) -> Result<()> {
    ensure!(
        guid.len() == 32
            && guid
                .bytes()
                .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte)),
        "NanoBoyAdvance SDL GUID must be 32 lowercase hexadecimal characters"
    );
    Ok(())
}

fn keyboard_values(document: &DocumentMut) -> Result<[i32; 10]> {
    let Some(input) = document.get("input") else {
        return Ok(DEFAULT_KEYBOARD);
    };
    let input = input
        .as_table()
        .context("NanoBoyAdvance input configuration must be a table")?;
    let Some(gba) = input.get("gba") else {
        return Ok(DEFAULT_KEYBOARD);
    };
    let gba = gba
        .as_table()
        .context("NanoBoyAdvance input.gba configuration must be a table")?;
    let mut keyboards = [0; 10];
    for (index, (control, _)) in CONTROLS.iter().enumerate() {
        let Some(item) = gba.get(control) else {
            keyboards[index] = 0;
            continue;
        };
        let array = item
            .as_array()
            .with_context(|| format!("NanoBoyAdvance input.gba.{control} must be an array"))?;
        ensure!(
            array.len() == 5,
            "NanoBoyAdvance input.gba.{control} must have five integers"
        );
        ensure!(
            array.iter().all(|item| item.as_integer().is_some()),
            "NanoBoyAdvance input.gba.{control} must contain integers"
        );
        keyboards[index] = i32::try_from(array.get(0).and_then(|item| item.as_integer()).unwrap())
            .context("NanoBoyAdvance keyboard value is out of range")?;
    }
    Ok(keyboards)
}

/// Patch only the controller GUID and ten controller halves in a parsed copy.
/// Every unrelated value, including keyboard bindings, BIOS, cartridge, save,
/// state, video and hotkey options, remains in the private baseline.
pub(crate) fn patch_config(
    baseline: &[u8],
    guid: &str,
    bindings: &BTreeMap<String, Binding>,
) -> Result<String> {
    ensure!(
        baseline.len() <= 4 * 1024 * 1024,
        "NanoBoyAdvance config is too large"
    );
    guid_valid(guid)?;
    ensure!(
        bindings.len() == CONTROLS.len()
            && CONTROLS
                .iter()
                .all(|(control, _)| bindings.contains_key(*control)),
        "NanoBoyAdvance needs all ten GBA controls"
    );
    ensure!(
        bindings.values().collect::<BTreeSet<_>>().len() == bindings.len(),
        "NanoBoyAdvance controls share one physical input"
    );
    let text = std::str::from_utf8(baseline).context("NanoBoyAdvance config is not UTF-8")?;
    ensure!(
        !text.contains('\0'),
        "NanoBoyAdvance config contains a NUL byte"
    );
    let mut document = text
        .parse::<DocumentMut>()
        .context("Invalid NanoBoyAdvance TOML configuration")?;
    let keyboards = keyboard_values(&document)?;
    let input = document
        .entry("input")
        .or_insert(Item::Table(Table::new()))
        .as_table_mut()
        .context("NanoBoyAdvance input configuration must be a table")?;
    input.insert("controller_guid", value(guid));
    let gba = input
        .entry("gba")
        .or_insert(Item::Table(Table::new()))
        .as_table_mut()
        .context("NanoBoyAdvance input.gba configuration must be a table")?;
    for (index, (control, _)) in CONTROLS.iter().enumerate() {
        let (button, axis, hat, direction) = bindings[*control].fields()?;
        let mut array = Array::new();
        for number in [keyboards[index], button, axis, hat, direction] {
            array.push(i64::from(number));
        }
        gba.insert(control, value(array));
    }
    Ok(document.to_string())
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
        /// Existing selected config, copied and overlaid at this exact path.
        pub config_path: PathBuf,
        pub probe_program: PathBuf,
        /// Exact SDL3 library linked by the selected emulator build.
        pub sdl_library: PathBuf,
        pub bubblewrap_program: PathBuf,
        pub executable_sha256: String,
        pub players: Vec<Player>,
    }

    impl SavedSetup {
        pub(crate) fn validate(&self) -> Result<()> {
            ensure!(
                !self.emulator_id.trim().is_empty(),
                "NanoBoyAdvance setup needs an emulator identity"
            );
            for path in [
                &self.content,
                &self.config_path,
                &self.probe_program,
                &self.sdl_library,
                &self.bubblewrap_program,
            ] {
                ensure!(
                    path.is_absolute()
                        && !path
                            .components()
                            .any(|part| matches!(part, std::path::Component::ParentDir)),
                    "NanoBoyAdvance setup paths must be absolute without parent traversal"
                );
            }
            ensure!(
                self.config_path.file_name().and_then(|name| name.to_str()) == Some("config.toml"),
                "NanoBoyAdvance config_path must name config.toml"
            );
            ensure!(
                self.executable_sha256.len() == 64
                    && self
                        .executable_sha256
                        .bytes()
                        .all(|byte| byte.is_ascii_hexdigit()),
                "NanoBoyAdvance setup needs a trusted executable SHA-256"
            );
            let limit = catalog()
                .emulator_profiles
                .iter()
                .find(|profile| profile.id == PROFILE_ID)
                .and_then(|profile| profile.native_launch.as_ref())
                .context("Missing NanoBoyAdvance native profile")?
                .max_players;
            ensure!(
                self.players.len() == 1 && limit == 1,
                "NanoBoyAdvance setup needs exactly one player"
            );
            ensure!(
                self.players[0].player == 1 && !self.players[0].controller_id.trim().is_empty(),
                "NanoBoyAdvance player must be nonempty player one"
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
                .context("Missing NanoBoyAdvance native profile")?;
            let player = &self.players[0];
            let calibration = calibrations
                .get(&player.controller_id)
                .context("NanoBoyAdvance controller has no saved calibration")?;
            ensure!(
                calibration.os == "linux" && calibration.backend != crate::controller_sdl3::BACKEND,
                "NanoBoyAdvance mapping requires Linux physical calibration"
            );
            let mapping = calibration.plan_profile(profile)?;
            ensure!(
                mapping.rows.iter().all(|row| row.physical_id.is_some()
                    && row
                        .input
                        .as_ref()
                        .is_some_and(|input| input.native.is_some())),
                "NanoBoyAdvance needs native calibration for every GBA control"
            );
            Ok(serde_json::json!({
                "players": [{
                    "player": 1,
                    "controller_id": player.controller_id,
                    "source_layout": calibration.layout,
                    "target_layout": profile.target_layout,
                    "mapping": mapping,
                }],
                "launch_ready": false,
                "launch_integration": "partial",
                "detail": "Native Linux launch overlays a private config at the selected config path, preserves keyboard, BIOS, cartridge, save and state settings, verifies the configured 16 KiB BIOS and save directory, and requires the selected GUID to remain unique in the exact SDL3 inventory. Runtime behavior is unverified."
            }))
        }
    }

    pub(crate) fn validate_setups(setups: &[SavedSetup]) -> Result<()> {
        ensure!(setups.len() <= 1024, "Too many NanoBoyAdvance saved setups");
        let mut identities = BTreeSet::new();
        for setup in setups {
            setup.validate()?;
            ensure!(
                identities.insert((&setup.emulator_id, &setup.content)),
                "Duplicate NanoBoyAdvance emulator/content setup"
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
        duckstation::DigitalInput,
        file_hash,
        linux_classic::{AxisEndpoints, ClassicMap},
    };
    use std::{
        fs,
        os::unix::fs::MetadataExt,
        path::{Path, PathBuf},
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
            command.arg("--match-path").arg(path);
        }
        let (output, _) = capture(&mut command, cancel)?;
        let snapshot: Snapshot =
            serde_json::from_slice(&output).context("Invalid NanoBoyAdvance SDL3 capture")?;
        ensure!(
            snapshot.library.canonicalize()? == setup.sdl_library.canonicalize()?
                && snapshot.library_sha256 == file_hash(&setup.sdl_library)?
                && snapshot
                    .effective_hints
                    .get("SDL_JOYSTICK_LINUX_CLASSIC")
                    .and_then(Option::as_deref)
                    == Some("1"),
            "NanoBoyAdvance helper inspected a different SDL3 runtime or backend"
        );
        Ok(snapshot)
    }

    fn comparable(snapshot: &Snapshot) -> Result<serde_json::Value> {
        let mut value = serde_json::to_value(snapshot)?;
        value
            .as_object_mut()
            .context("Invalid NanoBoyAdvance snapshot shape")?
            .remove("warnings");
        Ok(value)
    }

    fn bindings(
        calibration: &Calibration,
        physical: &ClassicMap,
    ) -> Result<BTreeMap<String, Binding>> {
        let profile = crate::controller_catalog::catalog()
            .emulator_profiles
            .iter()
            .find(|profile| profile.id == PROFILE_ID)
            .context("Missing NanoBoyAdvance native profile")?;
        let mut result = BTreeMap::new();
        for row in calibration.plan_profile(profile)?.rows {
            let input = row
                .input
                .as_ref()
                .context("NanoBoyAdvance GBA control is not calibrated")?;
            let native = input
                .native
                .as_ref()
                .context("NanoBoyAdvance requires measured native controls")?;
            let endpoints = input.axis.as_ref().map(|axis| AxisEndpoints {
                released: axis.released,
                pressed: axis.pressed,
            });
            let binding = match physical.digital_input(native.code, endpoints)? {
                DigitalInput::Button(index) => Binding::Button(index),
                DigitalInput::Hat { index, direction } => Binding::Hat { index, direction },
                DigitalInput::Axis {
                    index,
                    released,
                    pressed,
                } => {
                    let released = i32::from(released);
                    let pressed = i32::from(pressed);
                    let negative = pressed < 0;
                    ensure!(
                        if negative {
                            released >= -DIGITAL_THRESHOLD && pressed < -DIGITAL_THRESHOLD
                        } else {
                            released <= DIGITAL_THRESHOLD && pressed > DIGITAL_THRESHOLD
                        },
                        "NanoBoyAdvance axis calibration does not cross its source threshold from a released half"
                    );
                    Binding::Axis { index, negative }
                }
            };
            ensure!(
                result.insert(row.target_id, binding).is_none(),
                "NanoBoyAdvance target appears twice"
            );
        }
        ensure!(
            result.len() == CONTROLS.len()
                && CONTROLS
                    .iter()
                    .all(|(control, _)| result.contains_key(*control)),
            "NanoBoyAdvance mapping is incomplete"
        );
        ensure!(
            result.values().collect::<BTreeSet<_>>().len() == result.len(),
            "NanoBoyAdvance controls share one native input"
        );
        Ok(result)
    }

    #[derive(Debug)]
    struct DirectoryIdentity {
        path: PathBuf,
        canonical: PathBuf,
        device: u64,
        inode: u64,
    }

    impl DirectoryIdentity {
        fn capture(path: PathBuf) -> Result<Self> {
            ensure!(path.is_dir(), "NanoBoyAdvance save directory is missing");
            let metadata = fs::metadata(&path)?;
            Ok(Self {
                canonical: path.canonicalize()?,
                path,
                device: metadata.dev(),
                inode: metadata.ino(),
            })
        }
        fn verify(&self) -> Result<()> {
            let metadata = fs::metadata(&self.path)?;
            ensure!(
                self.path.canonicalize()? == self.canonical
                    && metadata.dev() == self.device
                    && metadata.ino() == self.inode,
                "NanoBoyAdvance save directory changed"
            );
            Ok(())
        }
    }

    fn configured_string(document: &DocumentMut, key: &str, default: &str) -> Result<String> {
        let Some(general) = document.get("general") else {
            return Ok(default.to_owned());
        };
        let general = general
            .as_table()
            .context("NanoBoyAdvance general configuration must be a table")?;
        let Some(item) = general.get(key) else {
            return Ok(default.to_owned());
        };
        Ok(item
            .as_str()
            .with_context(|| format!("NanoBoyAdvance general.{key} must be a string"))?
            .to_owned())
    }

    fn resolve_from(base: &Path, configured: &str) -> PathBuf {
        let path = PathBuf::from(configured);
        if path.is_absolute() {
            path
        } else {
            base.join(path)
        }
    }

    fn persistence_and_bios(
        baseline: &[u8],
        executable_directory: &Path,
        content: &Path,
    ) -> Result<(DirectoryIdentity, PathBuf)> {
        let document = std::str::from_utf8(baseline)
            .context("NanoBoyAdvance config is not UTF-8")?
            .parse::<DocumentMut>()
            .context("Invalid NanoBoyAdvance TOML configuration")?;
        let save_folder = configured_string(&document, "save_folder", "")?;
        let save_root = if save_folder.is_empty() {
            content
                .parent()
                .context("NanoBoyAdvance content has no save directory")?
                .to_path_buf()
        } else {
            resolve_from(executable_directory, &save_folder)
        };
        let persistence = DirectoryIdentity::capture(save_root)?;
        let bios = resolve_from(
            executable_directory,
            &configured_string(&document, "bios_path", "bios.bin")?,
        );
        ensure!(
            bios.is_file() && fs::metadata(&bios)?.len() == 0x4000,
            "NanoBoyAdvance configured BIOS must be a readable 16 KiB file"
        );
        Ok((persistence, bios.canonicalize()?))
    }

    pub(crate) struct PreparedSession {
        directory: tempfile::TempDir,
        pub(crate) private_config: PathBuf,
        runtime_path: String,
        guid: String,
        topology: InputTopology,
        snapshot: Snapshot,
        classic_map: ClassicMap,
        setup: settings::SavedSetup,
        hashes: BTreeMap<PathBuf, String>,
        persistence: DirectoryIdentity,
    }

    impl PreparedSession {
        pub(crate) fn prepare(
            setup: &settings::SavedSetup,
            calibrations: &std::collections::HashMap<String, Calibration>,
            inventory: &[ControllerDevice],
            executable_directory: &Path,
            cancel: &AtomicBool,
        ) -> Result<Self> {
            cancelled(cancel)?;
            setup.review(calibrations)?;
            let player = &setup.players[0];
            let found = inventory
                .iter()
                .filter(|device| device.stable_id == player.controller_id)
                .collect::<Vec<_>>();
            ensure!(
                found.len() == 1 && !found[0].is_virtual,
                "NanoBoyAdvance physical controller is missing or ambiguous"
            );
            let selected = found[0].device_path.clone();
            let topology = InputTopology::capture(std::slice::from_ref(&selected))?;
            let initial = observe(setup, &[], cancel)?;
            let runtime_path = topology.resolve_runtime_path(
                &selected,
                initial
                    .devices
                    .iter()
                    .filter_map(|device| device.path.as_deref()),
            )?;
            ensure!(
                runtime_path.starts_with("/dev/input/js"),
                "NanoBoyAdvance needs SDL's classic /dev/input/js* backend"
            );
            let snapshot = observe(setup, std::slice::from_ref(&runtime_path), cancel)?;
            ensure!(
                comparable(&initial)? == comparable(&snapshot)?,
                "NanoBoyAdvance SDL routing changed during capture"
            );
            topology.verify()?;
            let device = snapshot.device_at_path(&runtime_path)?;
            guid_valid(&device.guid)?;
            ensure!(
                snapshot
                    .devices
                    .iter()
                    .filter(|candidate| candidate.guid == device.guid)
                    .count()
                    == 1,
                "NanoBoyAdvance cannot distinguish controllers that share the selected SDL GUID"
            );
            let guid = device.guid.clone();
            let classic_map =
                lunchbox_controller_probe::linux_classic::read(Path::new(&runtime_path))?;
            let mapped = bindings(
                calibrations
                    .get(&player.controller_id)
                    .context("NanoBoyAdvance calibration disappeared")?,
                &classic_map,
            )?;
            let baseline =
                fs::read(&setup.config_path).context("Reading declared NanoBoyAdvance config")?;
            let (persistence, bios) =
                persistence_and_bios(&baseline, executable_directory, &setup.content)?;
            let directory = tempfile::Builder::new()
                .prefix("lunchbox-nanoboyadvance-")
                .tempdir()?;
            let private_config = directory.path().join("config.toml");
            fs::write(&private_config, patch_config(&baseline, &guid, &mapped)?)?;
            let mut hashes = BTreeMap::new();
            for path in [
                &setup.content,
                &setup.config_path,
                &setup.probe_program,
                &setup.sdl_library,
                &setup.bubblewrap_program,
                &private_config,
                &bios,
            ] {
                hashes.insert(path.clone(), file_hash(path)?);
            }
            let session = Self {
                directory,
                private_config,
                runtime_path,
                guid,
                topology,
                snapshot,
                classic_map,
                setup: setup.clone(),
                hashes,
                persistence,
            };
            session.verify(cancel)?;
            Ok(session)
        }

        pub(crate) fn verify(&self, cancel: &AtomicBool) -> Result<()> {
            cancelled(cancel)?;
            ensure!(
                self.directory.path().is_dir() && self.private_config.is_file(),
                "NanoBoyAdvance private configuration disappeared"
            );
            self.topology.verify()?;
            self.persistence.verify()?;
            for (path, expected) in &self.hashes {
                ensure!(
                    file_hash(path)? == *expected,
                    "NanoBoyAdvance launch input changed: {}",
                    path.display()
                );
            }
            let fresh = observe(
                &self.setup,
                std::slice::from_ref(&self.runtime_path),
                cancel,
            )?;
            ensure!(
                comparable(&fresh)? == comparable(&self.snapshot)?,
                "NanoBoyAdvance SDL routing changed before launch"
            );
            let device = fresh.device_at_path(&self.runtime_path)?;
            ensure!(
                device.guid == self.guid
                    && fresh
                        .devices
                        .iter()
                        .filter(|candidate| candidate.guid == self.guid)
                        .count()
                        == 1,
                "NanoBoyAdvance selected SDL GUID changed or became ambiguous"
            );
            ensure!(
                lunchbox_controller_probe::linux_classic::read(Path::new(&self.runtime_path))?
                    == self.classic_map,
                "NanoBoyAdvance SDL classic control numbering changed"
            );
            self.topology.verify()
        }

        pub(crate) fn check_health(&self) -> Result<()> {
            self.persistence.verify()?;
            self.topology.verify()
        }
    }
}

#[cfg(target_os = "linux")]
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
                "NanoBoyAdvance executable differs from the saved trusted runtime"
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
                "NanoBoyAdvance launch plan changed after preparation"
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
            anyhow::bail!("NanoBoyAdvance calibrated launch requires native Linux")
        };
        ensure!(
            option.emulator_name.eq_ignore_ascii_case("NanoBoyAdvance")
                && setup.emulator_id == option.emulator_id
                && original.environment.is_empty(),
            "NanoBoyAdvance identity differs or custom environment needs resolution"
        );
        let executable = executable.canonicalize()?;
        ensure!(
            executable == original.program.canonicalize()?,
            "NanoBoyAdvance executable differs from selection"
        );
        ensure!(
            original.arguments.len() == 1 && original.arguments[0] == setup.content.as_os_str(),
            "NanoBoyAdvance calibrated launch requires exactly the saved ROM argument"
        );
        ensure!(
            file_hash(&executable)?.eq_ignore_ascii_case(&setup.executable_sha256),
            "NanoBoyAdvance executable differs from the saved trusted runtime"
        );
        let executable_directory = executable
            .parent()
            .context("NanoBoyAdvance executable has no parent")?;
        let inputs = session::PreparedSession::prepare(
            setup,
            calibrations,
            inventory,
            executable_directory,
            cancel,
        )?;
        let cwd = original.current_directory.canonicalize()?;
        let mut arguments = vec![
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
        ];
        arguments.extend(original.arguments.iter().cloned());
        let mut plan = original.clone();
        plan.program = setup.bubblewrap_program.clone();
        plan.arguments = arguments;
        plan.environment
            .push(("SDL_JOYSTICK_LINUX_CLASSIC".into(), "1".into()));
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

#[cfg(test)]
mod tests {
    use super::*;

    fn bindings() -> BTreeMap<String, Binding> {
        CONTROLS
            .iter()
            .enumerate()
            .map(|(index, (control, _))| ((*control).to_owned(), Binding::Button(index as u32)))
            .collect()
    }

    #[test]
    fn patches_controller_fields_and_preserves_persistence_and_keyboard() {
        let baseline = br#"[general]
bios_path = "/firmware/gba.bin"
save_folder = "/saves/gba"
[cartridge]
save_type = "flash128"
[input]
hold_fast_forward = false
[input.gba]
a = [999, -1, -1, -1, 0]
b = [83, -1, -1, -1, 0]
select = [1, -1, -1, -1, 0]
start = [2, -1, -1, -1, 0]
right = [3, -1, -1, -1, 0]
left = [4, -1, -1, -1, 0]
up = [5, -1, -1, -1, 0]
down = [6, -1, -1, -1, 0]
r = [7, -1, -1, -1, 0]
l = [8, -1, -1, -1, 0]
"#;
        let text = patch_config(baseline, "0123456789abcdef0123456789abcdef", &bindings()).unwrap();
        assert!(text.contains("bios_path = \"/firmware/gba.bin\""));
        assert!(text.contains("save_folder = \"/saves/gba\""));
        assert!(text.contains("save_type = \"flash128\""));
        assert!(text.contains("hold_fast_forward = false"));
        assert!(text.contains("controller_guid = \"0123456789abcdef0123456789abcdef\""));
        assert!(text.contains("a = [999, 0, -1, -1, 0]"));
        assert!(text.contains("l = [8, 9, -1, -1, 0]"));
    }

    #[test]
    fn axis_hat_and_source_defaults_are_exact() {
        let mut map = bindings();
        map.insert(
            "a".into(),
            Binding::Axis {
                index: 2,
                negative: true,
            },
        );
        map.insert(
            "b".into(),
            Binding::Hat {
                index: 0,
                direction: 1,
            },
        );
        let text = patch_config(
            b"[window]\nscale = 3\n",
            "0123456789abcdef0123456789abcdef",
            &map,
        )
        .unwrap();
        assert!(text.contains("a = [65, -1, 258, -1, 0]"));
        assert!(text.contains("b = [83, -1, -1, 0, 1]"));
        assert!(text.contains("l = [68, 9, -1, -1, 0]"));
        assert!(text.contains("scale = 3"));
    }

    #[test]
    fn malformed_guid_duplicate_or_malformed_baseline_fails_closed() {
        assert!(patch_config(b"", "not-a-guid", &bindings()).is_err());
        let mut duplicate = bindings();
        duplicate.insert("b".into(), Binding::Button(0));
        assert!(patch_config(b"", "0123456789abcdef0123456789abcdef", &duplicate).is_err());
        assert!(
            patch_config(
                b"[input.gba]\na=[1]\n",
                "0123456789abcdef0123456789abcdef",
                &bindings()
            )
            .is_err()
        );
    }
}
