//! Private configuration for DuckStation 0a53bc47c. This is not a launch adapter:
//! the caller must still verify runtime routing, game identity and SDL bindings.
//! No source configuration is written, and the owner must outlive the emulator.
use anyhow::{Context, Result, bail, ensure};
use std::{
    collections::BTreeMap,
    fs,
    path::{Path, PathBuf},
};

const FOLDERS: &[(&str, &str, &str)] = &[
    ("BIOS", "SearchDirectory", "bios"),
    ("Folders", "Cache", "cache"),
    ("Folders", "Cheats", "cheats"),
    ("Folders", "Covers", "covers"),
    ("Folders", "GameIcons", "gameicons"),
    ("Folders", "GameSettings", "gamesettings"),
    ("Folders", "InputProfiles", "inputprofiles"),
    ("MemoryCards", "Directory", "memcards"),
    ("Folders", "Patches", "patches"),
    ("Folders", "SaveStates", "savestates"),
    ("Folders", "Screenshots", "screenshots"),
    ("Folders", "Shaders", "shaders"),
    ("Folders", "Subchannels", "subchannels"),
    ("Folders", "Textures", "textures"),
    ("Folders", "UserResources", "resources"),
    ("Folders", "Videos", "videos"),
];

/// Exact identity from the target emulator's database/disc, never a title match.
/// `None` means the game is verified not to belong to a disc set, not "unknown".
pub struct GameIdentity<'a> {
    pub serial: &'a str,
    pub first_disc_serial: Option<&'a str>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum InputLayer {
    Global,
    Game(String),
    Profile(String),
}

/// Line-preserving editor for the revision's single-line, multi-key SimpleIni
/// format. Unrelated repeated keys, comments, BOM and line endings survive.
/// Scalar reads reject duplicates rather than guessing at an ambiguous setting.
#[derive(Clone)]
struct Ini {
    text: String,
}

impl Ini {
    fn new(text: String) -> Result<Self> {
        ensure!(!text.contains('\0'), "NUL in DuckStation configuration");
        Ok(Self { text })
    }

    fn entries(&self, section: &str, key: &str) -> Vec<(std::ops::Range<usize>, &str)> {
        let mut current = "";
        let mut offset = 0;
        let mut found = Vec::new();
        for line in self.text.split_inclusive('\n') {
            let content = line.trim_start_matches('\u{feff}').trim();
            if let Some(name) = content.strip_prefix('[').and_then(|s| s.split_once(']')) {
                current = name.0.trim();
            } else if !content.starts_with([';', '#'])
                && current.eq_ignore_ascii_case(section)
                && let Some((name, value)) = content.split_once('=')
                && name.trim().eq_ignore_ascii_case(key)
            {
                found.push((offset..offset + line.len(), value.trim()));
            }
            offset += line.len();
        }
        found
    }

    fn get(&self, section: &str, key: &str) -> Result<Option<&str>> {
        let found = self.entries(section, key);
        ensure!(found.len() <= 1, "Ambiguous [{section}] {key}");
        Ok(found.first().map(|(_, value)| *value))
    }

    fn boolean(&self, section: &str, key: &str, default: bool) -> Result<bool> {
        match self
            .get(section, key)?
            .map(str::to_ascii_lowercase)
            .as_deref()
        {
            None => Ok(default),
            Some("true" | "yes" | "on" | "1") => Ok(true),
            Some("false" | "no" | "off" | "0") => Ok(false),
            Some(_) => bail!("Unrecognized [{section}] {key} boolean"),
        }
    }

    fn set(&mut self, section: &str, key: &str, value: &str) -> Result<()> {
        ensure!(
            !value.contains(['\n', '\r', '\0']) && value.trim() == value,
            "Value cannot be represented as a single-line DuckStation setting"
        );
        let newline = if self.text.contains("\r\n") {
            "\r\n"
        } else {
            "\n"
        };
        // Remove *all* old list members for a replaced binding, not just the first.
        let ranges: Vec<_> = self
            .entries(section, key)
            .into_iter()
            .map(|(range, _)| range)
            .collect();
        let existing = ranges.first().map(|r| r.start);
        for range in ranges.into_iter().rev() {
            self.text.replace_range(range, "");
        }
        let mut insertion = existing;
        if insertion.is_none() {
            let mut current_matches = false;
            let mut offset = 0;
            for line in self.text.split_inclusive('\n') {
                if let Some((name, _)) = line
                    .trim_start_matches('\u{feff}')
                    .trim()
                    .strip_prefix('[')
                    .and_then(|s| s.split_once(']'))
                {
                    if current_matches {
                        insertion = Some(offset);
                    }
                    current_matches = name.trim().eq_ignore_ascii_case(section);
                }
                offset += line.len();
            }
            if current_matches {
                insertion = Some(offset);
            }
        }
        if let Some(offset) = insertion {
            let prefix = if offset > 0 && !self.text[..offset].ends_with('\n') {
                newline
            } else {
                ""
            };
            self.text
                .insert_str(offset, &format!("{prefix}{key} = {value}{newline}"));
        } else {
            if !self.text.is_empty() && !self.text.ends_with('\n') {
                self.text.push_str(newline);
            }
            self.text
                .push_str(&format!("[{section}]{newline}{key} = {value}{newline}"));
        }
        Ok(())
    }
}

pub struct LaunchConfig {
    directory: tempfile::TempDir,
    pub input_layer: InputLayer,
    documents: BTreeMap<PathBuf, Ini>,
    originals: BTreeMap<PathBuf, Vec<u8>>,
}

fn safe_name(name: &str) -> Result<()> {
    // Accepted names need no platform-specific filename sanitization. Reject
    // unsafe names rather than accidentally choosing a different game's INI.
    ensure!(
        !name.is_empty()
            && name != "."
            && name != ".."
            && name.trim() == name
            && !name.ends_with('.')
            && !name
                .chars()
                .any(|c| c.is_control() || "/\\:<>\"|?*".contains(c)),
        "Unsupported DuckStation configuration filename"
    );
    Ok(())
}

impl LaunchConfig {
    /// `source_root` is the emulator's resolved DataRoot in the launch namespace.
    /// A missing game is only valid for a no-game startup, not an unknown ROM.
    pub fn stage(source_root: &Path, game: Option<GameIdentity<'_>>) -> Result<Self> {
        ensure!(
            source_root.is_absolute(),
            "DuckStation data root must be absolute"
        );
        let directory = tempfile::Builder::new()
            .prefix("lunchbox-duckstation-config-")
            .tempdir()?;
        let mut result = Self {
            directory,
            input_layer: InputLayer::Global,
            documents: BTreeMap::new(),
            originals: BTreeMap::new(),
        };
        let baseline = result.read_ini(&source_root.join("settings.ini"))?;
        ensure!(
            baseline.get("Main", "SettingsVersion")? == Some("3"),
            "DuckStation settings version differs from the verified contract"
        );
        let mut global = baseline.clone();
        for (section, key, default) in FOLDERS {
            let value = baseline
                .get(section, key)?
                .filter(|s| !s.is_empty())
                .unwrap_or(default);
            let source = source_root.join(value);
            let destination = if matches!(*key, "GameSettings" | "InputProfiles") {
                let private = result.data_root().join(default);
                fs::create_dir_all(&private)?;
                match fs::read_dir(&source) {
                    Ok(entries) => {
                        for entry in entries {
                            let entry = entry?;
                            if entry
                                .path()
                                .extension()
                                .is_some_and(|s| s.eq_ignore_ascii_case("ini"))
                            {
                                let ini = result.read_ini(&entry.path())?;
                                result
                                    .documents
                                    .insert(private.join(entry.file_name()), ini);
                            }
                        }
                    }
                    Err(error) if error.kind() == std::io::ErrorKind::NotFound => (),
                    Err(error) => {
                        return Err(error).context("Reading DuckStation configuration directory");
                    }
                }
                private
            } else {
                source
            };
            global.set(
                section,
                key,
                destination
                    .to_str()
                    .context("Non-UTF8 DuckStation folder")?,
            )?;
        }
        fs::create_dir_all(result.data_root())?;
        let mapping = source_root.join("gamecontrollerdb.txt");
        match fs::read(&mapping) {
            Ok(bytes) => {
                fs::write(result.data_root().join("gamecontrollerdb.txt"), &bytes)?;
                result.originals.insert(mapping, bytes);
            }
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => (),
            Err(error) => return Err(error).context("Reading DuckStation SDL mapping database"),
        }
        result.documents.insert(result.settings_path(), global);
        if baseline.boolean("Main", "ApplyGameSettings", true)?
            && let Some(game) = game
        {
            safe_name(game.serial)?;
            let first = game.first_disc_serial.unwrap_or(game.serial);
            safe_name(first)?;
            let mut selected = first;
            if let Some(ini) = result.documents.get(&result.game_path(first)) {
                if first != game.serial
                    && ini.boolean("Main", "UseSeparateConfigForDiscSet", false)?
                {
                    selected = game.serial;
                }
                if let Some(ini) = result.documents.get(&result.game_path(selected)) {
                    if ini.boolean("ControllerPorts", "UseGameSettingsForController", false)? {
                        result.input_layer = InputLayer::Game(selected.to_owned());
                    } else if let Some(name) = ini
                        .get("ControllerPorts", "InputProfileName")?
                        .filter(|s| !s.is_empty())
                    {
                        safe_name(name)?;
                        if result.documents.contains_key(&result.profile_path(name)) {
                            result.input_layer = InputLayer::Profile(name.to_owned());
                        }
                    }
                }
            }
        }
        result.flush()?;
        Ok(result)
    }

    fn read_ini(&mut self, path: &Path) -> Result<Ini> {
        let bytes = fs::read(path).with_context(|| format!("Reading {}", path.display()))?;
        let ini = Ini::new(
            String::from_utf8(bytes.clone()).context("Non-UTF8 DuckStation configuration")?,
        )?;
        self.originals.insert(path.to_owned(), bytes);
        Ok(ini)
    }

    pub fn root(&self) -> &Path {
        self.directory.path()
    }
    pub fn config_home(&self) -> PathBuf {
        self.root().join("config")
    }
    pub fn data_root(&self) -> PathBuf {
        self.config_home().join("duckstation")
    }
    pub fn settings_path(&self) -> PathBuf {
        self.data_root().join("settings.ini")
    }
    fn game_path(&self, name: &str) -> PathBuf {
        self.data_root()
            .join("gamesettings")
            .join(format!("{name}.ini"))
    }
    fn profile_path(&self, name: &str) -> PathBuf {
        self.data_root()
            .join("inputprofiles")
            .join(format!("{name}.ini"))
    }
    pub fn input_path(&self) -> PathBuf {
        match &self.input_layer {
            InputLayer::Global => self.settings_path(),
            InputLayer::Game(name) => self.game_path(name),
            InputLayer::Profile(name) => self.profile_path(name),
        }
    }

    /// The type comes from the whole selected layer, not a per-key global fallback.
    pub fn controller_type(&self, pad: u8) -> Result<String> {
        ensure!((1..=8).contains(&pad), "Invalid DuckStation pad slot");
        if pad >= 3 {
            let mode = self.documents[&self.input_path()]
                .get("ControllerPorts", "MultitapMode")?
                .unwrap_or("Disabled");
            // Settings slots 1/2 are the two normal ports; 3..5 belong to the
            // first multitap and 6..8 to the second, not two sequential groups.
            let enabled = mode.eq_ignore_ascii_case("BothPorts")
                || mode.eq_ignore_ascii_case(if pad <= 5 { "Port1Only" } else { "Port2Only" });
            if !enabled {
                return Ok("None".into());
            }
        }
        Ok(self.documents[&self.input_path()]
            .get(&format!("Pad{pad}"), "Type")?
            .unwrap_or(if pad == 1 { "AnalogController" } else { "None" })
            .to_owned())
    }

    /// Applies already-validated digital bindings. This never changes Type and
    /// never claims digital calibration is sufficient for an analog controller.
    /// Missing entries intentionally clear that control; caller must explain any
    /// missing physical capabilities before accepting the launch plan.
    pub fn apply_digital(&mut self, pad: u8, bindings: &BTreeMap<String, String>) -> Result<()> {
        const KEYS: &[&str] = &[
            "Up", "Down", "Left", "Right", "Select", "Start", "Cross", "Circle", "Square",
            "Triangle", "L1", "L2", "R1", "R2",
        ];
        ensure!(
            self.controller_type(pad)?
                .eq_ignore_ascii_case("DigitalController"),
            "Digital bindings cannot replace the selected controller type"
        );
        self.patch_inputs(pad, bindings, KEYS)
    }

    /// Gameplay bindings for the selected AnalogController. Keeps its type,
    /// dead zones, sensitivity, analog-mode policy, toggle and motor routing.
    /// Toggle/motor assignment is separate from calibrated gameplay inputs.
    pub fn apply_analog(&mut self, pad: u8, bindings: &BTreeMap<String, String>) -> Result<()> {
        const KEYS: &[&str] = &[
            "Up", "Down", "Left", "Right", "Select", "Start", "Cross", "Circle", "Square",
            "Triangle", "L1", "L2", "R1", "R2", "L3", "R3", "LLeft", "LRight", "LUp", "LDown",
            "RLeft", "RRight", "RUp", "RDown",
        ];
        ensure!(
            self.controller_type(pad)?
                .eq_ignore_ascii_case("AnalogController"),
            "Analog bindings cannot replace the selected controller type"
        );
        self.patch_inputs(pad, bindings, KEYS)
    }

    fn patch_inputs(
        &mut self,
        pad: u8,
        bindings: &BTreeMap<String, String>,
        keys: &[&str],
    ) -> Result<()> {
        ensure!(
            bindings.keys().all(|key| keys.contains(&key.as_str())),
            "Unknown gameplay controller binding"
        );
        // Validate the entire change in memory before replacing the staged file.
        let path = self.input_path();
        let mut input = self.documents[&path].clone();
        for key in keys {
            input.set(
                &format!("Pad{pad}"),
                key,
                bindings.get(*key).map(String::as_str).unwrap_or(""),
            )?;
        }
        fs::write(&path, &input.text)?;
        self.documents.insert(path, input);
        Ok(())
    }

    /// Diagnostic-only options for the no-game startup oracle.
    pub fn enable_startup_diagnostics(&mut self) -> Result<()> {
        let path = self.settings_path();
        let global = self
            .documents
            .get_mut(&path)
            .expect("staged global settings");
        for (section, key, value) in [
            ("Logging", "LogLevel", "Dev"),
            ("Logging", "LogToConsole", "true"),
            ("Logging", "LogTimestamps", "false"),
            ("AutoUpdater", "CheckAtStartup", "false"),
        ] {
            global.set(section, key, value)?;
        }
        self.flush()
    }

    pub fn verify_originals_unchanged(&self) -> Result<()> {
        for (path, bytes) in &self.originals {
            ensure!(
                fs::read(path)? == *bytes,
                "Source configuration changed: {}",
                path.display()
            );
        }
        Ok(())
    }

    /// Checks the installed emulator's Dev log, not just our generated INI.
    /// This covers data-folder routing at startup, not game input interpretation.
    pub fn verify_startup_routing(&self, log: &str) -> Result<()> {
        ensure!(
            log.contains(&format!(
                "Loading config from {}.",
                self.settings_path().display()
            )),
            "DuckStation did not confirm the private settings path"
        );
        let labels = [
            "BIOS",
            "Cache",
            "Cheats",
            "Covers",
            "Game Icons",
            "Game Settings",
            "Input Profile",
            "MemoryCards",
            "Patches",
            "SaveStates",
            "Screenshots",
            "Shaders",
            "Subchannels",
            "Textures",
            "User Resources",
            "Videos",
        ];
        let global = &self.documents[&self.settings_path()];
        for ((section, key, _), label) in FOLDERS.iter().zip(labels) {
            let expected =
                PathBuf::from(global.get(section, key)?.context("Missing staged folder")?);
            let expected = expected.canonicalize().unwrap_or(expected);
            // Color reset follows the path. Include it or end of line so a
            // different directory sharing a prefix cannot satisfy the check.
            let marker = format!("{label} Directory: {}", expected.display());
            ensure!(
                log.lines().any(|line| line
                    .split_once(&marker)
                    .is_some_and(|(_, tail)| tail.is_empty() || tail.starts_with("\x1b["))),
                "DuckStation did not confirm the expected {label} directory"
            );
        }
        Ok(())
    }

    fn flush(&self) -> Result<()> {
        for (path, ini) in &self.documents {
            fs::write(path, &ini.text)?;
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn fixture(extra: &str) -> tempfile::TempDir {
        let dir = tempfile::tempdir().unwrap();
        fs::write(
            dir.path().join("settings.ini"),
            format!("[Main]\nSettingsVersion=3\n{extra}"),
        )
        .unwrap();
        for name in ["gamesettings", "inputprofiles"] {
            fs::create_dir(dir.path().join(name)).unwrap();
        }
        dir
    }

    fn game() -> Option<GameIdentity<'static>> {
        Some(GameIdentity {
            serial: "SLUS-00002",
            first_disc_serial: Some("SLUS-00001"),
        })
    }

    #[test]
    fn preserves_unrelated_bytes_and_replaces_all_binding_list_members() {
        let original = "\u{feff}; comment\r\n[Pad1]\r\nCross=old\r\ncross=other\r\n[Paths]\r\nPath=one\r\nPath=two\r\n# end";
        let mut ini = Ini::new(original.into()).unwrap();
        ini.set("Pad1", "Cross", "SDL-2/A").unwrap();
        assert_eq!(
            ini.text,
            "\u{feff}; comment\r\n[Pad1]\r\nCross = SDL-2/A\r\n[Paths]\r\nPath=one\r\nPath=two\r\n# end"
        );
        let once = ini.text.clone();
        ini.set("Pad1", "Cross", "SDL-2/A").unwrap();
        assert_eq!(ini.text, once);
        assert_eq!(ini.get("Pad1", "Cross").unwrap(), Some("SDL-2/A"));
        assert!(ini.get("Paths", "Path").is_err());
        assert!(ini.set("Pad1", "Cross", "x\nType=None").is_err());
    }

    #[test]
    fn redirects_only_configuration_and_keeps_save_locations() {
        let source = fixture(
            "[MemoryCards]\nDirectory=../My Cards\nCard1Path=card.mcd\n[Pad1]\nType=AnalogController\n[Hotkeys]\nPause=Keyboard/Space\n",
        );
        fs::write(source.path().join("gamecontrollerdb.txt"), "local mapping").unwrap();
        let staged = LaunchConfig::stage(source.path(), None).unwrap();
        let global = &staged.documents[&staged.settings_path()];
        assert_eq!(
            global.get("MemoryCards", "Directory").unwrap(),
            source.path().join("../My Cards").to_str()
        );
        assert_eq!(
            global.get("MemoryCards", "Card1Path").unwrap(),
            Some("card.mcd")
        );
        assert_eq!(
            global.get("BIOS", "SearchDirectory").unwrap(),
            source.path().join("bios").to_str()
        );
        assert_eq!(
            global.get("Folders", "GameSettings").unwrap(),
            staged.data_root().join("gamesettings").to_str()
        );
        assert_eq!(staged.controller_type(1).unwrap(), "AnalogController");
        assert_eq!(
            fs::read(staged.data_root().join("gamecontrollerdb.txt")).unwrap(),
            b"local mapping"
        );
        staged.verify_originals_unchanged().unwrap();
        let root = staged.root().to_owned();
        drop(staged);
        assert!(!root.exists());
        assert!(source.path().join("settings.ini").exists());
    }

    #[test]
    fn patches_selected_profile_not_global_or_other_game_options() {
        let source = fixture("[Pad1]\nType=AnalogController\nCross=global\n");
        fs::write(
            source.path().join("gamesettings/SLUS-00001.ini"),
            "[ControllerPorts]\nInputProfileName=My pad\n[GPU]\nResolutionScale=4\n",
        )
        .unwrap();
        fs::write(source.path().join("inputprofiles/My pad.ini"), "[Pad1]\nType=DigitalController\nCross=old\nCross=second\n[Hotkeys]\nPause=Keyboard/P\n").unwrap();
        let mut staged = LaunchConfig::stage(source.path(), game()).unwrap();
        assert_eq!(staged.input_layer, InputLayer::Profile("My pad".into()));
        staged
            .apply_digital(1, &BTreeMap::from([("Cross".into(), "SDL-1/A".into())]))
            .unwrap();
        let profile = &staged.documents[&staged.input_path()];
        assert_eq!(profile.get("Pad1", "Cross").unwrap(), Some("SDL-1/A"));
        assert_eq!(profile.get("Pad1", "Circle").unwrap(), Some(""));
        assert_eq!(profile.get("Hotkeys", "Pause").unwrap(), Some("Keyboard/P"));
        assert_eq!(
            staged.documents[&staged.settings_path()]
                .get("Pad1", "Cross")
                .unwrap(),
            Some("global")
        );
        assert_eq!(
            staged.documents[&staged.game_path("SLUS-00001")]
                .get("GPU", "ResolutionScale")
                .unwrap(),
            Some("4")
        );
        staged.verify_originals_unchanged().unwrap();
    }

    #[test]
    fn game_input_policy_wins_and_has_no_global_type_fallback() {
        let source = fixture("[Pad1]\nType=DigitalController\n");
        fs::write(
            source.path().join("gamesettings/SLUS-00001.ini"),
            "[ControllerPorts]\nUseGameSettingsForController=true\nInputProfileName=ignored\n",
        )
        .unwrap();
        let mut staged = LaunchConfig::stage(source.path(), game()).unwrap();
        assert_eq!(staged.input_layer, InputLayer::Game("SLUS-00001".into()));
        assert_eq!(staged.controller_type(1).unwrap(), "AnalogController");
        assert!(staged.apply_digital(1, &BTreeMap::new()).is_err());
        assert_eq!(staged.controller_type(2).unwrap(), "None");
    }

    #[test]
    fn separate_disc_missing_is_global_not_group_fallback() {
        let source = fixture("");
        let first = source.path().join("gamesettings/SLUS-00001.ini");
        fs::write(&first, "[Main]\nUseSeparateConfigForDiscSet=true\n[ControllerPorts]\nUseGameSettingsForController=true\n").unwrap();
        assert_eq!(
            LaunchConfig::stage(source.path(), game())
                .unwrap()
                .input_layer,
            InputLayer::Global
        );
        fs::write(
            source.path().join("gamesettings/SLUS-00002.ini"),
            "[ControllerPorts]\nUseGameSettingsForController=true\n",
        )
        .unwrap();
        assert_eq!(
            LaunchConfig::stage(source.path(), game())
                .unwrap()
                .input_layer,
            InputLayer::Game("SLUS-00002".into())
        );
        fs::remove_file(first).unwrap();
        assert_eq!(
            LaunchConfig::stage(source.path(), game())
                .unwrap()
                .input_layer,
            InputLayer::Global
        );
    }

    #[test]
    fn disabled_game_settings_and_missing_profile_use_global() {
        let source = fixture("ApplyGameSettings=false\n");
        fs::write(
            source.path().join("gamesettings/SLUS-00001.ini"),
            "[ControllerPorts]\nUseGameSettingsForController=true\n",
        )
        .unwrap();
        assert_eq!(
            LaunchConfig::stage(source.path(), game())
                .unwrap()
                .input_layer,
            InputLayer::Global
        );
        fs::write(
            source.path().join("settings.ini"),
            "[Main]\nSettingsVersion=3\n",
        )
        .unwrap();
        fs::write(
            source.path().join("gamesettings/SLUS-00001.ini"),
            "[ControllerPorts]\nInputProfileName=absent\n",
        )
        .unwrap();
        assert_eq!(
            LaunchConfig::stage(source.path(), game())
                .unwrap()
                .input_layer,
            InputLayer::Global
        );
    }

    #[test]
    fn rejects_unsafe_identity_and_profile_without_source_writes() {
        let source = fixture("");
        assert!(
            LaunchConfig::stage(
                source.path(),
                Some(GameIdentity {
                    serial: "../escape",
                    first_disc_serial: None
                })
            )
            .is_err()
        );
        fs::write(
            source.path().join("gamesettings/SLUS-00001.ini"),
            "[ControllerPorts]\nInputProfileName=../outside\n",
        )
        .unwrap();
        assert!(LaunchConfig::stage(source.path(), game()).is_err());
        assert!(LaunchConfig::stage(Path::new("relative"), None).is_err());
    }

    #[test]
    fn insertion_into_missing_section_and_unterminated_existing_section_is_idempotent() {
        for text in [
            "",
            "[Other]\nA=1",
            "[Pad1]\nType=DigitalController",
            "\u{feff}[Pad1]\r\nType=DigitalController\r\n",
        ] {
            let mut ini = Ini::new(text.into()).unwrap();
            ini.set("Pad1", "Cross", "SDL-0/A").unwrap();
            let once = ini.text.clone();
            ini.set("Pad1", "Cross", "SDL-0/A").unwrap();
            assert_eq!(ini.text, once);
            assert_eq!(ini.get("Pad1", "Cross").unwrap(), Some("SDL-0/A"));
        }
    }

    #[test]
    fn changed_source_is_detected_and_bad_patch_is_atomic() {
        let source = fixture("[Pad1]\nType=DigitalController\nCross=old\n");
        let mut staged = LaunchConfig::stage(source.path(), None).unwrap();
        let before = fs::read(staged.input_path()).unwrap();
        assert!(
            staged
                .apply_digital(
                    1,
                    &BTreeMap::from([("Triangle".into(), "bad\nvalue".into())])
                )
                .is_err()
        );
        assert_eq!(before, fs::read(staged.input_path()).unwrap());
        fs::write(
            source.path().join("settings.ini"),
            "changed by another process",
        )
        .unwrap();
        assert!(staged.verify_originals_unchanged().is_err());
        assert!(staged.verify_startup_routing("unrelated log").is_err());
    }

    #[test]
    fn disabled_multitap_slots_cannot_receive_an_ineffective_patch() {
        for (mode, first, second) in [
            ("Disabled", "None", "None"),
            ("Port1Only", "DigitalController", "None"),
            ("Port2Only", "None", "DigitalController"),
            ("BothPorts", "DigitalController", "DigitalController"),
        ] {
            let source = fixture(&format!(
                "[ControllerPorts]\nMultitapMode={mode}\n[Pad3]\nType=DigitalController\n[Pad5]\nType=DigitalController\n[Pad6]\nType=DigitalController\n[Pad8]\nType=DigitalController\n"
            ));
            let mut staged = LaunchConfig::stage(source.path(), None).unwrap();
            for pad in [3, 5] {
                assert_eq!(staged.controller_type(pad).unwrap(), first);
            }
            for pad in [6, 8] {
                assert_eq!(staged.controller_type(pad).unwrap(), second);
            }
            assert_eq!(
                staged.apply_digital(3, &BTreeMap::new()).is_ok(),
                first != "None"
            );
        }
    }

    #[test]
    fn analog_patch_preserves_mode_and_non_gameplay_settings() {
        let source = fixture(
            "[Pad1]\nType=AnalogController\nLUp=old\nLUp=other\nAnalogDeadzone=0.12\nAnalogSensitivity=1.15\nForceAnalogOnReset=false\nAnalog=Keyboard/Tab\nLargeMotor=SDL-0/LargeMotor\n[Hotkeys]\nPause=Keyboard/P\n",
        );
        let mut staged = LaunchConfig::stage(source.path(), None).unwrap();
        let bindings = BTreeMap::from([
            ("LUp".into(), "SDL-2/-LeftY".into()),
            ("Cross".into(), "SDL-2/A".into()),
        ]);
        staged.apply_analog(1, &bindings).unwrap();
        let first = fs::read(staged.input_path()).unwrap();
        staged.apply_analog(1, &bindings).unwrap();
        assert_eq!(fs::read(staged.input_path()).unwrap(), first);
        let input = &staged.documents[&staged.input_path()];
        assert_eq!(input.get("Pad1", "LUp").unwrap(), Some("SDL-2/-LeftY"));
        assert_eq!(input.get("Pad1", "RUp").unwrap(), Some(""));
        for (key, value) in [
            ("Type", "AnalogController"),
            ("AnalogDeadzone", "0.12"),
            ("AnalogSensitivity", "1.15"),
            ("ForceAnalogOnReset", "false"),
            ("Analog", "Keyboard/Tab"),
            ("LargeMotor", "SDL-0/LargeMotor"),
        ] {
            assert_eq!(input.get("Pad1", key).unwrap(), Some(value));
        }
        assert!(staged.apply_analog(2, &BTreeMap::new()).is_err());
        staged.verify_originals_unchanged().unwrap();
    }
}
