//! Shared launch adapter for the SDL3 desktop builds of Gearsystem and
//! Gearcoleco. Both frontends use SDL_GetPrefPath for `config.ini`, consume
//! logical SDL gamepad controls, and assign the first two SDL gamepads to the
//! first two emulated ports.

use anyhow::{Context, Result, ensure};
use serde::{Deserialize, Serialize};
use std::{
    collections::{BTreeMap, BTreeSet, HashMap},
    path::{Component, Path, PathBuf},
};

pub(crate) const GEARSYSTEM_GAME_GEAR_PROFILE: &str = "gearsystem:standalone-gamegear";
pub(crate) const GEARSYSTEM_MASTER_SYSTEM_PROFILE: &str = "gearsystem:standalone-master-system";
pub(crate) const GEARCOLECO_PROFILE: &str = "gearcoleco:standalone-colecovision";

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub(crate) enum Adapter {
    Gearsystem,
    Gearcoleco,
}

impl Adapter {
    pub(crate) fn core(self) -> &'static str {
        match self {
            Self::Gearsystem => "gearsystem",
            Self::Gearcoleco => "gearcoleco",
        }
    }

    fn title(self) -> &'static str {
        match self {
            Self::Gearsystem => "Gearsystem",
            Self::Gearcoleco => "Gearcoleco",
        }
    }

    fn accepts_profile(self, id: &str) -> bool {
        match self {
            Self::Gearsystem => matches!(
                id,
                GEARSYSTEM_GAME_GEAR_PROFILE | GEARSYSTEM_MASTER_SYSTEM_PROFILE
            ),
            Self::Gearcoleco => id == GEARCOLECO_PROFILE,
        }
    }
}

pub(crate) mod settings {
    use super::*;
    use crate::controller_catalog::{Calibration, catalog};

    #[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
    #[serde(deny_unknown_fields)]
    pub(crate) struct Player {
        pub player: u8,
        pub controller_id: String,
    }

    #[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
    #[serde(deny_unknown_fields)]
    pub(crate) struct SavedSetup {
        pub adapter: Adapter,
        pub profile_id: String,
        pub emulator_id: String,
        pub content: PathBuf,
        /// Existing desktop `config.ini`; it is copied, never edited in place.
        pub config_path: PathBuf,
        pub probe_program: PathBuf,
        /// Exact SDL3 library used by this emulator build.
        pub sdl_library: PathBuf,
        /// Exact `gamecontrollerdb.txt` beside the canonical executable.
        pub mapping_database: PathBuf,
        pub executable_sha256: String,
        pub players: Vec<Player>,
    }

    impl SavedSetup {
        pub(crate) fn validate(&self) -> Result<()> {
            ensure!(
                self.adapter.accepts_profile(&self.profile_id),
                "Gear adapter and native profile disagree"
            );
            ensure!(
                !self.emulator_id.trim().is_empty(),
                "Gear native setup needs an emulator identity"
            );
            for path in [
                &self.content,
                &self.config_path,
                &self.probe_program,
                &self.sdl_library,
                &self.mapping_database,
            ] {
                ensure!(
                    path.is_absolute()
                        && !path
                            .components()
                            .any(|part| matches!(part, Component::ParentDir)),
                    "Gear native setup paths must be absolute without parent traversal"
                );
            }
            ensure!(
                self.config_path.file_name().and_then(|name| name.to_str()) == Some("config.ini"),
                "Gear native config_path must name config.ini"
            );
            ensure!(
                self.mapping_database
                    .file_name()
                    .and_then(|name| name.to_str())
                    == Some("gamecontrollerdb.txt"),
                "Gear native mapping database must name gamecontrollerdb.txt"
            );
            ensure!(
                self.executable_sha256.len() == 64
                    && self
                        .executable_sha256
                        .bytes()
                        .all(|byte| byte.is_ascii_hexdigit()),
                "Gear native setup needs a trusted executable SHA-256"
            );
            let profile = catalog()
                .emulator_profiles
                .iter()
                .find(|profile| profile.id == self.profile_id)
                .context("Missing Gear native profile")?;
            let limit = profile
                .native_launch
                .as_ref()
                .context("Gear native profile has no player limit")?
                .max_players;
            ensure!(
                !self.players.is_empty() && self.players.len() <= limit,
                "Gear native setup has an invalid player count"
            );
            let mut controllers = BTreeSet::new();
            for (index, player) in self.players.iter().enumerate() {
                ensure!(
                    usize::from(player.player) == index + 1
                        && !player.controller_id.trim().is_empty()
                        && controllers.insert(&player.controller_id),
                    "Gear players must be distinct, contiguous, and start at player one"
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
                .find(|profile| profile.id == self.profile_id)
                .context("Missing Gear native profile")?;
            let mut players = Vec::new();
            for player in &self.players {
                let calibration = calibrations
                    .get(&player.controller_id)
                    .context("Gear native controller has no saved calibration")?;
                ensure!(
                    calibration.os == "linux"
                        && calibration.backend != crate::controller_sdl3::BACKEND,
                    "Gear native mapping needs Linux physical calibration"
                );
                let mapping = calibration.plan_profile(profile)?;
                ensure!(
                    mapping.rows.iter().all(|row| {
                        row.physical_id.is_some()
                            && row
                                .input
                                .as_ref()
                                .is_some_and(|input| input.native.is_some())
                    }),
                    "Gear native mapping needs every gameplay control calibrated"
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
                "adapter": self.adapter,
                "profile_id": self.profile_id,
                "players": players,
                "launch_ready": false,
                "launch_integration": "partial",
                "detail": "Native Linux launch copies config.ini under a private SDL preference root, preserves the original save/state destinations, and requires selected controllers to occupy the exact first SDL3 gamepad slots. Runtime behavior is unverified."
            }))
        }
    }

    pub(crate) fn validate_setups(setups: &[SavedSetup]) -> Result<()> {
        ensure!(setups.len() <= 1024, "Too many Gear native saved setups");
        let mut identities = BTreeSet::new();
        for setup in setups {
            setup.validate()?;
            ensure!(
                identities.insert((&setup.emulator_id, &setup.content)),
                "Duplicate Gear native emulator/content setup"
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
        controller_catalog::{Calibration, EmulatorProfile},
        controller_native_process::{cancelled, capture},
        controller_pcsx2::sdl::{AxisRange, Input as SdlInput},
        controllers::ControllerDevice,
    };
    use lunchbox_controller_probe::{Snapshot, file_hash, linux_classic::AxisEndpoints};
    use std::{fs, os::unix::fs::MetadataExt, process::Command, sync::atomic::AtomicBool};

    #[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
    enum LogicalInput {
        Button(u8),
        Axis { index: u8, range: AxisRange },
    }

    impl LogicalInput {
        fn from_sdl(input: SdlInput) -> Result<Self> {
            match input {
                SdlInput::GamepadButton { index } => Ok(Self::Button(index)),
                SdlInput::GamepadAxis { index, range } => Ok(Self::Axis { index, range }),
                _ => anyhow::bail!("Gear desktop builds require a resolved SDL gamepad control"),
            }
        }

        fn action(self) -> Result<(bool, u8)> {
            match self {
                Self::Button(index) if index <= 26 => Ok((false, index)),
                Self::Axis {
                    index,
                    range: AxisRange::Positive,
                } if index <= 5 => Ok((true, index)),
                _ => anyhow::bail!(
                    "Gear button actions require an SDL button or positive gamepad-axis half"
                ),
            }
        }
    }

    #[derive(Clone, Copy, Debug, Eq, PartialEq)]
    enum Directional {
        Dpad,
        Analog {
            x_axis: u8,
            y_axis: u8,
            invert_x: bool,
            invert_y: bool,
        },
    }

    fn directional(mapped: &BTreeMap<String, LogicalInput>) -> Result<Directional> {
        let input = |name: &str| {
            mapped
                .get(name)
                .copied()
                .with_context(|| format!("Gear directional control {name} is absent"))
        };
        let [up, down, left, right] = [
            input("up")?,
            input("down")?,
            input("left")?,
            input("right")?,
        ];
        if [up, down, left, right]
            == [
                LogicalInput::Button(11),
                LogicalInput::Button(12),
                LogicalInput::Button(13),
                LogicalInput::Button(14),
            ]
        {
            return Ok(Directional::Dpad);
        }
        let half = |value| match value {
            LogicalInput::Axis { index, range } if range != AxisRange::Full => {
                Ok((index, range == AxisRange::Positive))
            }
            _ => anyhow::bail!(
                "Gear directions must resolve to the standard D-pad or two gamepad axes"
            ),
        };
        let (up_axis, up_positive) = half(up)?;
        let (down_axis, down_positive) = half(down)?;
        let (left_axis, left_positive) = half(left)?;
        let (right_axis, right_positive) = half(right)?;
        ensure!(
            up_axis == down_axis
                && up_positive != down_positive
                && left_axis == right_axis
                && left_positive != right_positive
                && up_axis != left_axis,
            "Gear analog directions need opposite halves of two distinct SDL axes"
        );
        Ok(Directional::Analog {
            x_axis: left_axis,
            y_axis: up_axis,
            invert_x: left_positive,
            invert_y: up_positive,
        })
    }

    fn translate(
        calibration: &Calibration,
        profile: &EmulatorProfile,
        device: &lunchbox_controller_probe::Device,
    ) -> Result<BTreeMap<String, LogicalInput>> {
        ensure!(
            device.is_gamepad,
            "Gear selected device is not an SDL gamepad"
        );
        let resolved = device
            .resolved
            .as_ref()
            .context("Gear SDL3 resolved bindings are absent")?;
        let physical = device
            .linux_classic
            .as_ref()
            .context("Gear SDL3 classic Linux control map is absent")?;
        physical.validate_counts(resolved)?;
        let mapping = calibration.plan_profile(profile)?;
        let mut result = BTreeMap::new();
        let mut outputs = BTreeSet::new();
        for row in mapping.rows {
            let binding = row
                .input
                .as_ref()
                .context("Gear gameplay control is not calibrated")?;
            let native = binding
                .native
                .as_ref()
                .context("Gear requires measured native controls")?;
            let endpoints = binding.axis.as_ref().map(|axis| AxisEndpoints {
                released: axis.released,
                pressed: axis.pressed,
            });
            let raw = physical.digital_input(native.code, endpoints)?;
            let logical = LogicalInput::from_sdl(crate::controller_pcsx2::physical::digital(
                resolved, true, raw,
            )?)?;
            ensure!(
                outputs.insert(logical),
                "Gear target controls resolve to the same SDL input"
            );
            ensure!(
                result.insert(row.target_id, logical).is_none(),
                "Gear target control appears twice"
            );
        }
        Ok(result)
    }

    fn gearsystem_profile(
        player: u8,
        mapped: &BTreeMap<String, LogicalInput>,
    ) -> Result<crate::controller_gearsystem_standalone::PlayerProfile> {
        let action = |name: &str| -> Result<crate::controller_gearsystem_standalone::GamepadInput> {
            let (axis, index) = mapped
                .get(name)
                .copied()
                .with_context(|| format!("Gearsystem control {name} is absent"))?
                .action()?;
            Ok(if axis {
                crate::controller_gearsystem_standalone::GamepadInput::Axis(index)
            } else {
                crate::controller_gearsystem_standalone::GamepadInput::Button(index)
            })
        };
        let directional = match directional(mapped)? {
            Directional::Dpad => crate::controller_gearsystem_standalone::Directional::Dpad,
            Directional::Analog {
                x_axis,
                y_axis,
                invert_x,
                invert_y,
            } => crate::controller_gearsystem_standalone::Directional::Analog {
                x_axis,
                y_axis,
                invert_x,
                invert_y,
            },
        };
        Ok(crate::controller_gearsystem_standalone::PlayerProfile {
            player,
            directional,
            button_1: action("b")?,
            button_2: action("a")?,
            start: mapped
                .contains_key("start")
                .then(|| action("start"))
                .transpose()?,
            reset: None,
        })
    }

    fn gearcoleco_profile(
        player: u8,
        mapped: &BTreeMap<String, LogicalInput>,
    ) -> Result<crate::controller_gearcoleco_standalone::PlayerProfile> {
        let action = |name: &str| -> Result<crate::controller_gearcoleco_standalone::GamepadInput> {
            let (axis, index) = mapped
                .get(name)
                .copied()
                .with_context(|| format!("Gearcoleco control {name} is absent"))?
                .action()?;
            Ok(if axis {
                crate::controller_gearcoleco_standalone::GamepadInput::Axis(index)
            } else {
                crate::controller_gearcoleco_standalone::GamepadInput::Button(index)
            })
        };
        let directional = match directional(mapped)? {
            Directional::Dpad => crate::controller_gearcoleco_standalone::Directional::Dpad,
            Directional::Analog {
                x_axis,
                y_axis,
                invert_x,
                invert_y,
            } => crate::controller_gearcoleco_standalone::Directional::Analog {
                x_axis,
                y_axis,
                invert_x,
                invert_y,
            },
        };
        let mut buttons = [None; 16];
        for (index, target) in [
            (0, "fire1"),
            (1, "fire2"),
            (4, "key_1"),
            (5, "key_2"),
            (6, "key_3"),
            (7, "key_4"),
            (8, "key_5"),
            (9, "key_6"),
            (10, "key_7"),
            (11, "key_8"),
            (12, "key_9"),
            (13, "key_0"),
            (14, "key_star"),
            (15, "key_hash"),
        ] {
            buttons[index] = Some(action(target)?);
        }
        Ok(crate::controller_gearcoleco_standalone::PlayerProfile {
            player,
            directional,
            buttons,
        })
    }

    fn section_value<'a>(text: &'a str, section: &str, key: &str) -> Result<Option<&'a str>> {
        let mut active = false;
        let mut value = None;
        for line in text.lines() {
            let trimmed = line.trim();
            if let Some(header) = trimmed
                .strip_prefix('[')
                .and_then(|line| line.strip_suffix(']'))
            {
                active = header.trim() == section;
            } else if active
                && let Some((name, current)) = line.split_once('=')
                && name.trim() == key
            {
                ensure!(value.is_none(), "Gear config contains duplicate {key}");
                value = Some(current.trim());
            }
        }
        Ok(value)
    }

    fn patch_section(
        text: &str,
        section: &str,
        fields: &BTreeMap<String, String>,
    ) -> Result<String> {
        ensure!(!text.contains('\0'), "Gear config contains a NUL byte");
        let (bom, text) = text
            .strip_prefix('\u{feff}')
            .map_or(("", text), |text| ("\u{feff}", text));
        let newline = if text.contains("\r\n") { "\r\n" } else { "\n" };
        let mut result = bom.to_owned();
        let mut active = false;
        let mut inserted = false;
        for line in text.split_inclusive('\n') {
            if let Some(header) = line
                .trim()
                .strip_prefix('[')
                .and_then(|line| line.strip_suffix(']'))
            {
                active = header.trim() == section;
                result.push_str(line);
                if active && !inserted {
                    if !result.ends_with('\n') {
                        result.push_str(newline);
                    }
                    for (key, value) in fields {
                        result.push_str(&format!("{key}={value}{newline}"));
                    }
                    inserted = true;
                }
            } else {
                let owned = active
                    && line
                        .split_once('=')
                        .is_some_and(|(key, _)| fields.contains_key(key.trim()));
                if !owned {
                    result.push_str(line);
                }
            }
        }
        if !inserted {
            if !result.is_empty() && !result.ends_with('\n') {
                result.push_str(newline);
            }
            result.push_str(&format!("[{section}]{newline}"));
            for (key, value) in fields {
                result.push_str(&format!("{key}={value}{newline}"));
            }
        }
        Ok(result)
    }

    pub(super) fn persistence_overlay(
        text: &str,
        original_root: &Path,
        content: &Path,
    ) -> Result<(String, Vec<PathBuf>)> {
        let original_root = original_root
            .to_str()
            .context("Gear config directory is not UTF-8")?;
        ensure!(
            !original_root.chars().any(char::is_control),
            "Gear config directory contains a control character"
        );
        let mut fields = BTreeMap::new();
        let mut roots = Vec::new();
        for (option_key, path_key) in [
            ("SaveFilesDirOption", "SaveFilesPath"),
            ("SaveStatesDirOption", "SaveStatesPath"),
        ] {
            let option = section_value(text, "Emulator", option_key)?
                .unwrap_or("0")
                .parse::<u8>()
                .with_context(|| format!("Gear config has an invalid {option_key}"))?;
            let root = match option {
                0 => {
                    fields.insert(option_key.to_owned(), "2".into());
                    fields.insert(path_key.to_owned(), original_root.to_owned());
                    PathBuf::from(original_root)
                }
                1 => content
                    .parent()
                    .context("Gear content has no parent directory")?
                    .to_path_buf(),
                2 => {
                    let path = section_value(text, "Emulator", path_key)?
                        .context("Gear custom persistence option has no path")?;
                    ensure!(
                        !path.chars().any(char::is_control),
                        "Gear custom persistence path contains a control character"
                    );
                    let path = PathBuf::from(path);
                    ensure!(
                        path.is_absolute()
                            && !path
                                .components()
                                .any(|part| matches!(part, Component::ParentDir)),
                        "Gear custom persistence path must be absolute without parent traversal"
                    );
                    path
                }
                _ => anyhow::bail!("Gear config has an unsupported persistence option"),
            };
            ensure!(root.is_dir(), "Gear persistence directory is missing");
            roots.push(root);
        }
        roots.sort();
        roots.dedup();
        Ok((patch_section(text, "Emulator", &fields)?, roots))
    }

    type Routing = Vec<(String, String, Option<String>, Option<u16>, Option<String>)>;

    fn routing(snapshot: &Snapshot) -> Routing {
        snapshot
            .devices
            .iter()
            .map(|device| {
                (
                    device.path.clone().unwrap_or_default(),
                    device.guid.clone(),
                    device.gamepad_name.clone(),
                    device.gamepad_index,
                    device.mapping.clone(),
                )
            })
            .collect()
    }

    fn comparable(snapshot: &Snapshot) -> Result<serde_json::Value> {
        let mut value = serde_json::to_value(snapshot)?;
        value
            .as_object_mut()
            .context("Invalid Gear SDL snapshot shape")?
            .remove("warnings");
        Ok(value)
    }

    fn observe(
        setup: &settings::SavedSetup,
        paths: &[String],
        cancel: &AtomicBool,
    ) -> Result<Snapshot> {
        let mut command = Command::new(&setup.probe_program);
        command
            .arg("--sdl-library")
            .arg(&setup.sdl_library)
            .arg("--mapping-db")
            .arg(&setup.mapping_database)
            .arg("--hint")
            .arg("SDL_JOYSTICK_LINUX_CLASSIC=1");
        for path in paths {
            command.arg("--bindings-for-path").arg(path);
        }
        let (output, _) = capture(&mut command, cancel)?;
        let snapshot: Snapshot =
            serde_json::from_slice(&output).context("Invalid Gear SDL3 capture")?;
        ensure!(
            snapshot.library.canonicalize()? == setup.sdl_library.canonicalize()?
                && snapshot.library_sha256 == file_hash(&setup.sdl_library)?
                && snapshot.mapping_database_sha256.as_deref()
                    == Some(file_hash(&setup.mapping_database)?.as_str())
                && snapshot
                    .effective_hints
                    .get("SDL_JOYSTICK_LINUX_CLASSIC")
                    .and_then(Option::as_deref)
                    == Some("1"),
            "Gear helper inspected different SDL3 inputs"
        );
        Ok(snapshot)
    }

    struct DirectoryIdentity {
        path: PathBuf,
        canonical: PathBuf,
        device: u64,
        inode: u64,
    }

    impl DirectoryIdentity {
        fn capture(path: PathBuf) -> Result<Self> {
            let canonical = path.canonicalize()?;
            let metadata = fs::metadata(&path)?;
            Ok(Self {
                path,
                canonical,
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
                "Gear persistence directory changed"
            );
            Ok(())
        }
    }

    pub(crate) struct PreparedSession {
        directory: tempfile::TempDir,
        pub(crate) data_home: PathBuf,
        runtime_paths: Vec<String>,
        topology: InputTopology,
        snapshot: Snapshot,
        setup: settings::SavedSetup,
        hashes: BTreeMap<PathBuf, String>,
        persistence: Vec<DirectoryIdentity>,
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
            let profile = crate::controller_catalog::catalog()
                .emulator_profiles
                .iter()
                .find(|profile| profile.id == setup.profile_id)
                .context("Missing Gear native profile")?;
            let mut selected = Vec::new();
            for player in &setup.players {
                let found = inventory
                    .iter()
                    .filter(|device| device.stable_id == player.controller_id)
                    .collect::<Vec<_>>();
                ensure!(
                    found.len() == 1 && !found[0].is_virtual,
                    "Gear physical controller is missing or ambiguous"
                );
                selected.push(found[0].device_path.clone());
            }
            let topology = InputTopology::capture(&selected)?;
            let initial = observe(setup, &[], cancel)?;
            let visible_paths = initial
                .devices
                .iter()
                .filter_map(|device| device.path.as_deref())
                .collect::<Vec<_>>();
            let runtime_paths = selected
                .iter()
                .map(|path| topology.resolve_runtime_path(path, visible_paths.iter().copied()))
                .collect::<Result<Vec<_>>>()?;
            let snapshot = observe(setup, &runtime_paths, cancel)?;
            ensure!(
                routing(&initial) == routing(&snapshot),
                "Gear SDL3 routing changed during binding capture"
            );
            for (index, path) in runtime_paths.iter().enumerate() {
                ensure!(
                    snapshot.device_at_path(path)?.gamepad_index == Some(index as u16),
                    "Gear selected controllers are not the first SDL3 gamepads in player order"
                );
            }
            let mut gearsystem = Vec::new();
            let mut gearcoleco = Vec::new();
            for (player, path) in setup.players.iter().zip(&runtime_paths) {
                let mapped = translate(
                    calibrations
                        .get(&player.controller_id)
                        .context("Gear calibration disappeared")?,
                    profile,
                    snapshot.device_at_path(path)?,
                )?;
                match setup.adapter {
                    Adapter::Gearsystem => {
                        gearsystem.push(gearsystem_profile(player.player, &mapped)?)
                    }
                    Adapter::Gearcoleco => {
                        gearcoleco.push(gearcoleco_profile(player.player, &mapped)?)
                    }
                }
            }
            let baseline =
                fs::read(&setup.config_path).context("Reading the declared Gear config.ini")?;
            ensure!(baseline.len() <= 1024 * 1024, "Gear config is too large");
            let patched = match setup.adapter {
                Adapter::Gearsystem => {
                    crate::controller_gearsystem_standalone::patch_config(&baseline, &gearsystem)?
                }
                Adapter::Gearcoleco => {
                    crate::controller_gearcoleco_standalone::patch_config(&baseline, &gearcoleco)?
                }
            };
            let original_root = setup
                .config_path
                .parent()
                .context("Gear config path has no parent")?;
            let (patched, persistence) =
                persistence_overlay(&patched, original_root, &setup.content)?;
            let persistence = persistence
                .into_iter()
                .map(DirectoryIdentity::capture)
                .collect::<Result<Vec<_>>>()?;

            let directory = tempfile::Builder::new()
                .prefix("lunchbox-gear-native-")
                .tempdir()?;
            let data_home = directory.path().join("data-home");
            let target_dir = data_home.join("Geardome").join(setup.adapter.title());
            fs::create_dir_all(&target_dir)?;
            let private_config = target_dir.join("config.ini");
            fs::write(&private_config, patched)?;
            let mut hashes = BTreeMap::new();
            for path in [
                &setup.probe_program,
                &setup.sdl_library,
                &setup.mapping_database,
                &setup.content,
                &setup.config_path,
                &private_config,
            ] {
                hashes.insert(path.clone(), file_hash(path)?);
            }
            let prepared = Self {
                directory,
                data_home,
                runtime_paths,
                topology,
                snapshot,
                setup: setup.clone(),
                hashes,
                persistence,
            };
            prepared.verify(cancel)?;
            Ok(prepared)
        }

        pub(crate) fn verify(&self, cancel: &AtomicBool) -> Result<()> {
            cancelled(cancel)?;
            ensure!(
                self.directory.path().is_dir() && self.data_home.is_dir(),
                "Gear private SDL preference root disappeared"
            );
            self.topology.verify()?;
            for (path, expected) in &self.hashes {
                ensure!(file_hash(path)? == *expected, "Gear launch input changed");
            }
            for directory in &self.persistence {
                directory.verify()?;
            }
            let current = observe(&self.setup, &self.runtime_paths, cancel)?;
            ensure!(
                comparable(&current)? == comparable(&self.snapshot)?,
                "Gear SDL3 routing or resolved bindings changed before launch"
            );
            self.topology.verify()
        }

        pub(crate) fn check_health(&self) -> Result<()> {
            self.topology.verify()?;
            for directory in &self.persistence {
                directory.verify()?;
            }
            Ok(())
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
    use std::sync::atomic::AtomicBool;

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
                file_hash(&self.executable)? == self.setup.executable_sha256,
                "Gear executable differs from the saved trusted runtime"
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
                "Gear launch plan changed after preparation"
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
            anyhow::bail!("Gear calibrated launch requires native Linux");
        };
        ensure!(
            setup
                .adapter
                .core()
                .eq_ignore_ascii_case(&option.emulator_name)
                && setup.emulator_id == option.emulator_id
                && original.environment.is_empty(),
            "Gear identity differs or custom environment needs resolution"
        );
        let executable = executable.canonicalize()?;
        ensure!(
            executable == original.program.canonicalize()?,
            "Gear launch executable differs from selection"
        );
        let allowed_flag = |argument: &std::ffi::OsStr| {
            matches!(
                argument.to_str(),
                Some("-f" | "--fullscreen" | "-w" | "--windowed")
            )
        };
        ensure!(
            original
                .arguments
                .iter()
                .filter(|argument| !allowed_flag(argument))
                .eq(std::iter::once(setup.content.as_os_str())),
            "Gear calibrated launch requires the saved game and only fullscreen/windowed flags"
        );
        ensure!(
            !executable
                .parent()
                .context("Gear executable has no parent")?
                .join("portable.ini")
                .exists(),
            "Gear portable mode bypasses the private SDL preference root"
        );
        ensure!(
            setup.mapping_database.canonicalize()?
                == executable
                    .parent()
                    .context("Gear executable has no parent")?
                    .join("gamecontrollerdb.txt")
                    .canonicalize()?,
            "Gear mapping database is not the executable's sibling database"
        );
        ensure!(
            file_hash(&executable)? == setup.executable_sha256,
            "Gear executable differs from the saved trusted runtime"
        );
        let inputs = session::PreparedSession::prepare(setup, calibrations, inventory, cancel)?;
        let mut plan = original.clone();
        plan.environment.push((
            "XDG_DATA_HOME".into(),
            inputs.data_home.as_os_str().to_owned(),
        ));
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

    #[test]
    fn persistence_overlay_keeps_custom_and_rom_destinations() {
        let temporary = tempfile::tempdir().unwrap();
        let config = temporary.path().join("config");
        let roms = temporary.path().join("roms");
        let custom = temporary.path().join("custom");
        std::fs::create_dir_all(&config).unwrap();
        std::fs::create_dir_all(&roms).unwrap();
        std::fs::create_dir_all(&custom).unwrap();
        let content = roms.join("game.rom");
        std::fs::write(&content, b"rom").unwrap();
        let baseline = format!(
            "[Emulator]\nSaveFilesDirOption=1\nSaveStatesDirOption=2\nSaveStatesPath={}\n",
            custom.display()
        );
        let (patched, roots) = session::persistence_overlay(&baseline, &config, &content).unwrap();
        assert_eq!(patched, baseline);
        assert_eq!(roots, vec![custom, roms]);
    }

    #[test]
    fn persistence_overlay_pins_default_to_original_root() {
        let temporary = tempfile::tempdir().unwrap();
        let config = temporary.path().join("config");
        let roms = temporary.path().join("roms");
        std::fs::create_dir_all(&config).unwrap();
        std::fs::create_dir_all(&roms).unwrap();
        let content = roms.join("game.rom");
        let (patched, roots) =
            session::persistence_overlay("[Emulator]\nSaveSlot=2\n", &config, &content).unwrap();
        assert!(patched.contains("SaveFilesDirOption=2\n"));
        assert!(patched.contains(&format!("SaveFilesPath={}\n", config.display())));
        assert!(patched.contains("SaveStatesDirOption=2\n"));
        assert_eq!(roots, vec![config]);
    }
}
