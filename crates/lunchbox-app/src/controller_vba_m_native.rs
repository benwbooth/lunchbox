//! VBA-M standalone Qt/wx controller mapping and guarded native Linux launch.
//!
//! Functional contract pinned to visualboyadvance-m/visualboyadvance-m commit
//! fd13034143c128c8b68133a7a18bc785178ec4e4. Both frontends persist the same
//! `Joypad/<player>/<control>` values. `JoyN` is the one-based SDL joystick
//! enumeration slot. The private launch config disables SDL GameController
//! translation so these values unambiguously name raw SDL joystick controls.

use anyhow::{Context, Result, ensure};
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};

pub(crate) const SOURCE_COMMIT: &str = "fd13034143c128c8b68133a7a18bc785178ec4e4";
pub(crate) const PROFILE_ID: &str = "vba-m:standalone-vba-m-gba-controller";
pub(crate) const UPSTREAM_URL: &str = "https://github.com/visualboyadvance-m/visualboyadvance-m";
const AXIS_THRESHOLD: i32 = 0x1fff;

pub(crate) const CONTROLS: [(&str, &str); 10] = [
    ("up", "Up"),
    ("down", "Down"),
    ("left", "Left"),
    ("right", "Right"),
    ("a", "A"),
    ("b", "B"),
    ("l", "L"),
    ("r", "R"),
    ("select", "Select"),
    ("start", "Start"),
];

#[derive(Clone, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub(crate) enum Binding {
    Keyboard(String),
    JoystickButton {
        device: u8,
        button: u8,
    },
    JoystickAxis {
        device: u8,
        axis: u8,
        positive: bool,
    },
    JoystickHat {
        device: u8,
        hat: u8,
        direction: HatDirection,
    },
}

#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub(crate) enum HatDirection {
    North,
    South,
    West,
    East,
}

impl Binding {
    fn config_string(&self) -> Result<String> {
        match self {
            Self::Keyboard(value) => {
                ensure!(!value.is_empty(), "VBA-M keyboard binding is empty");
                ensure!(
                    !value
                        .chars()
                        .any(|ch| ch == ',' || ch == '\n' || ch == '\r' || ch.is_control()),
                    "VBA-M keyboard binding contains a separator or control character"
                );
                Ok(value.clone())
            }
            Self::JoystickButton { device, button } => {
                Ok(format!("Joy{}-Button{}", u16::from(*device) + 1, button))
            }
            Self::JoystickAxis {
                device,
                axis,
                positive,
            } => Ok(format!(
                "Joy{}-Axis{}{}",
                u16::from(*device) + 1,
                axis,
                if *positive { '+' } else { '-' }
            )),
            Self::JoystickHat {
                device,
                hat,
                direction,
            } => Ok(format!(
                "Joy{}-Hat{}{}",
                u16::from(*device) + 1,
                hat,
                match direction {
                    HatDirection::North => 'N',
                    HatDirection::South => 'S',
                    HatDirection::West => 'W',
                    HatDirection::East => 'E',
                }
            )),
        }
    }
}

fn validated_values(bindings: &BTreeMap<String, Binding>) -> Result<Vec<(&'static str, String)>> {
    ensure!(
        bindings.len() == CONTROLS.len(),
        "VBA-M requires all ten first-player gameplay controls"
    );
    let mut used = BTreeSet::new();
    let mut values = Vec::with_capacity(CONTROLS.len());
    for (name, key) in CONTROLS {
        let binding = bindings
            .get(name)
            .ok_or_else(|| anyhow::anyhow!("VBA-M control {name} is absent"))?;
        let value = binding.config_string()?;
        ensure!(
            used.insert(value.clone()),
            "VBA-M reuses one physical input"
        );
        values.push((key, value));
    }
    Ok(values)
}

pub(crate) fn input_ini(bindings: &BTreeMap<String, Binding>) -> Result<String> {
    let mut output = String::from("[Joypad]\nSDLGameControllerMode=false\n");
    for (key, value) in validated_values(bindings)? {
        output.push_str("1\\");
        output.push_str(key);
        output.push('=');
        output.push_str(&value);
        output.push('\n');
    }
    Ok(output)
}

pub(crate) fn wx_input_ini(bindings: &BTreeMap<String, Binding>) -> Result<String> {
    let mut output = String::from("[Joypad]\nSDLGameControllerMode=false\n[Joypad/1]\n");
    for (key, value) in validated_values(bindings)? {
        output.push_str(key);
        output.push('=');
        output.push_str(&value);
        output.push('\n');
    }
    Ok(output)
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub(crate) enum Frontend {
    Qt,
    Wx,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub(crate) enum SdlApi {
    Sdl2,
    Sdl3,
}

fn patch_sections(
    baseline: &[u8],
    replacements: &BTreeMap<String, BTreeMap<String, String>>,
) -> Result<String> {
    ensure!(baseline.len() <= 4 * 1024 * 1024, "VBA-M INI is too large");
    let text = std::str::from_utf8(baseline).context("VBA-M INI is not UTF-8")?;
    ensure!(!text.contains('\0'), "VBA-M INI contains a NUL byte");
    let newline = if text.contains("\r\n") { "\r\n" } else { "\n" };
    let mut output = String::new();
    let mut active = None::<String>;
    let mut seen_sections = BTreeSet::new();
    let mut seen_keys = BTreeMap::<String, BTreeSet<String>>::new();
    let append_missing =
        |output: &mut String, section: &str, seen_keys: &BTreeMap<String, BTreeSet<String>>| {
            if let Some(fields) = replacements.get(section) {
                let seen = seen_keys.get(section);
                for (key, value) in fields {
                    if seen.is_none_or(|keys| !keys.contains(key)) {
                        output.push_str(key);
                        output.push('=');
                        output.push_str(value);
                        output.push_str(newline);
                    }
                }
            }
        };
    for raw in text.split_inclusive('\n') {
        let line = raw.strip_suffix('\n').unwrap_or(raw);
        let line = line.strip_suffix('\r').unwrap_or(line);
        if let Some(header) = line
            .trim()
            .strip_prefix('[')
            .and_then(|value| value.strip_suffix(']'))
        {
            if let Some(section) = active.take() {
                append_missing(&mut output, &section, &seen_keys);
            }
            let canonical = replacements
                .keys()
                .find(|known| known.eq_ignore_ascii_case(header.trim()))
                .cloned();
            if let Some(section) = &canonical {
                ensure!(
                    seen_sections.insert(section.clone()),
                    "VBA-M controller section {section} is duplicated"
                );
            }
            active = canonical;
            output.push_str(raw);
            continue;
        }
        if let Some(section) = &active
            && let Some((name, _)) = line.split_once('=')
            && let Some((canonical, value)) = replacements[section]
                .iter()
                .find(|(known, _)| known.eq_ignore_ascii_case(name.trim()))
        {
            ensure!(
                seen_keys
                    .entry(section.clone())
                    .or_default()
                    .insert(canonical.clone()),
                "VBA-M controller key {section}/{canonical} is duplicated"
            );
            output.push_str(canonical);
            output.push('=');
            output.push_str(value);
            output.push_str(newline);
            continue;
        }
        output.push_str(raw);
    }
    if let Some(section) = active {
        append_missing(&mut output, &section, &seen_keys);
    }
    for (section, fields) in replacements {
        if seen_sections.contains(section) {
            continue;
        }
        if !output.is_empty() && !output.ends_with(newline) {
            output.push_str(newline);
        }
        if !output.is_empty() {
            output.push_str(newline);
        }
        output.push('[');
        output.push_str(section);
        output.push(']');
        output.push_str(newline);
        for (key, value) in fields {
            output.push_str(key);
            output.push('=');
            output.push_str(value);
            output.push_str(newline);
        }
    }
    Ok(output)
}

pub(crate) fn patch_config(
    baseline: &[u8],
    frontend: Frontend,
    bindings: &BTreeMap<String, Binding>,
) -> Result<String> {
    let values = validated_values(bindings)?;
    let mut sections = BTreeMap::<String, BTreeMap<String, String>>::new();
    sections
        .entry("Joypad".into())
        .or_default()
        .insert("SDLGameControllerMode".into(), "false".into());
    let (section, prefix) = match frontend {
        Frontend::Qt => ("Joypad", "1\\"),
        Frontend::Wx => ("Joypad/1", ""),
    };
    for (key, value) in values {
        sections
            .entry(section.into())
            .or_default()
            .insert(format!("{prefix}{key}"), value);
    }
    patch_sections(baseline, &sections)
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
        pub config_path: PathBuf,
        pub frontend: Frontend,
        pub sdl_api: SdlApi,
        pub probe_program: PathBuf,
        pub sdl_library: PathBuf,
        pub executable_sha256: String,
        pub players: Vec<Player>,
    }

    impl SavedSetup {
        pub(crate) fn validate(&self) -> Result<()> {
            ensure!(
                !self.emulator_id.trim().is_empty(),
                "VBA-M setup needs an emulator identity"
            );
            for path in [
                &self.content,
                &self.config_path,
                &self.probe_program,
                &self.sdl_library,
            ] {
                ensure!(
                    path.is_absolute()
                        && !path
                            .components()
                            .any(|part| matches!(part, std::path::Component::ParentDir)),
                    "VBA-M setup paths must be absolute without parent traversal"
                );
            }
            let expected = match self.frontend {
                Frontend::Qt => "vbam-qt.ini",
                Frontend::Wx => "vbam.ini",
            };
            ensure!(
                self.config_path.file_name().and_then(|name| name.to_str()) == Some(expected),
                "VBA-M config filename does not match the selected frontend"
            );
            ensure!(
                self.content
                    .extension()
                    .and_then(|value| value.to_str())
                    .is_some_and(|value| value.eq_ignore_ascii_case("gba")),
                "VBA-M native e-Reader setup requires uncompressed GBA content"
            );
            ensure!(
                self.executable_sha256.len() == 64
                    && self
                        .executable_sha256
                        .bytes()
                        .all(|byte| byte.is_ascii_hexdigit()),
                "VBA-M setup needs a trusted executable SHA-256"
            );
            ensure!(
                self.players.len() == 1
                    && self.players[0].player == 1
                    && !self.players[0].controller_id.trim().is_empty(),
                "VBA-M setup needs exactly one nonempty player-one controller"
            );
            let limit = catalog()
                .emulator_profiles
                .iter()
                .find(|profile| profile.id == PROFILE_ID)
                .and_then(|profile| profile.native_launch.as_ref())
                .context("Missing VBA-M native profile")?
                .max_players;
            ensure!(
                limit == 1,
                "VBA-M native profile has an invalid player limit"
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
                .context("Missing VBA-M native profile")?;
            let calibration = calibrations
                .get(&self.players[0].controller_id)
                .context("VBA-M controller has no saved calibration")?;
            ensure!(
                ["linux", "macos", "windows"].contains(&calibration.os.as_str()),
                "VBA-M mapping requires a desktop physical calibration"
            );
            let mapping = calibration.plan_profile(profile)?;
            ensure!(
                mapping.rows.iter().all(|row| row.physical_id.is_some()
                    && row
                        .input
                        .as_ref()
                        .is_some_and(|input| input.native.is_some())),
                "VBA-M needs native calibration for every GBA control"
            );
            Ok(serde_json::json!({
                "players": [{"player": 1, "controller_id": self.players[0].controller_id, "source_layout": calibration.layout, "target_layout": profile.target_layout, "mapping": mapping}],
                "launch_ready": false,
                "launch_integration": "partial",
                "detail": "Native launch copies the selected Qt/wx INI, disables logical GameController translation, writes only first-player GBA controls, and rechecks the exact SDL runtime, raw control numbering, save/state roots, optional active GBA BIOS, content and executable. Runtime behavior is unverified."
            }))
        }
    }

    pub(crate) fn validate_setups(setups: &[SavedSetup]) -> Result<()> {
        ensure!(setups.len() <= 1024, "Too many VBA-M saved setups");
        let mut identities = BTreeSet::new();
        for setup in setups {
            setup.validate()?;
            ensure!(
                identities.insert((&setup.emulator_id, &setup.content)),
                "Duplicate VBA-M emulator/content setup"
            );
        }
        Ok(())
    }
}

#[cfg(target_os = "linux")]
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
        Snapshot as Sdl3Snapshot,
        duckstation::DigitalInput,
        file_hash,
        linux_classic::{AxisEndpoints, ClassicMap},
        sdl2::{ControlCounts, Snapshot as Sdl2Snapshot},
        sdl2_evdev::EvdevMap,
        sdl2_physical::PhysicalMap,
    };
    use std::{
        fs,
        path::{Path, PathBuf},
        process::Command,
        sync::atomic::AtomicBool,
    };

    fn observe_sdl2(
        setup: &settings::SavedSetup,
        path: Option<&str>,
        cancel: &AtomicBool,
    ) -> Result<Sdl2Snapshot> {
        let mut command = Command::new(&setup.probe_program);
        command
            .arg("--sdl2-inventory")
            .arg("--sdl-library")
            .arg(&setup.sdl_library);
        if let Some(path) = path {
            command.arg("--sdl2-controls-for-path").arg(path);
        }
        let (output, _) = capture(&mut command, cancel)?;
        let snapshot: Sdl2Snapshot =
            serde_json::from_slice(&output).context("Invalid VBA-M SDL2 capture")?;
        ensure!(
            snapshot.version[0] == 2
                && snapshot.library.canonicalize()? == setup.sdl_library.canonicalize()?
                && snapshot.library_sha256 == file_hash(&setup.sdl_library)?,
            "VBA-M helper inspected a different SDL2 runtime"
        );
        Ok(snapshot)
    }

    fn sdl2_routing(mut snapshot: Sdl2Snapshot) -> Sdl2Snapshot {
        for device in &mut snapshot.devices {
            device.controls = None;
            device.linux_classic = None;
            device.linux_evdev = None;
            device.sampled_state = None;
        }
        snapshot
    }

    fn observe_sdl3(
        setup: &settings::SavedSetup,
        paths: &[String],
        cancel: &AtomicBool,
    ) -> Result<Sdl3Snapshot> {
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
        let snapshot: Sdl3Snapshot =
            serde_json::from_slice(&output).context("Invalid VBA-M SDL3 capture")?;
        ensure!(
            snapshot.sdl_version / 1_000_000 == 3
                && snapshot.library.canonicalize()? == setup.sdl_library.canonicalize()?
                && snapshot.library_sha256 == file_hash(&setup.sdl_library)?
                && snapshot
                    .effective_hints
                    .get("SDL_JOYSTICK_LINUX_CLASSIC")
                    .and_then(Option::as_deref)
                    == Some("1"),
            "VBA-M helper inspected a different SDL3 runtime or backend"
        );
        Ok(snapshot)
    }

    fn comparable_sdl3(snapshot: &Sdl3Snapshot) -> Result<serde_json::Value> {
        let mut value = serde_json::to_value(snapshot)?;
        value
            .as_object_mut()
            .context("Invalid VBA-M SDL3 snapshot shape")?
            .remove("warnings");
        Ok(value)
    }

    fn binding(input: DigitalInput, slot: u8) -> Result<Binding> {
        match input {
            DigitalInput::Button(index) => Ok(Binding::JoystickButton {
                device: slot,
                button: u8::try_from(index).context("VBA-M button index is out of range")?,
            }),
            DigitalInput::Hat { index, direction } => Ok(Binding::JoystickHat {
                device: slot,
                hat: u8::try_from(index).context("VBA-M hat index is out of range")?,
                direction: match direction {
                    1 => HatDirection::North,
                    2 => HatDirection::East,
                    4 => HatDirection::South,
                    8 => HatDirection::West,
                    _ => anyhow::bail!("VBA-M hat direction must be cardinal"),
                },
            }),
            DigitalInput::Axis {
                index,
                released,
                pressed,
            } => {
                let released = i32::from(released);
                let pressed = i32::from(pressed);
                let positive = pressed > 0;
                ensure!(
                    if positive {
                        released <= AXIS_THRESHOLD && pressed > AXIS_THRESHOLD
                    } else {
                        released >= -AXIS_THRESHOLD && pressed < -AXIS_THRESHOLD
                    },
                    "VBA-M axis calibration does not cross the source threshold from a released half"
                );
                Ok(Binding::JoystickAxis {
                    device: slot,
                    axis: u8::try_from(index).context("VBA-M axis index is out of range")?,
                    positive,
                })
            }
        }
    }

    fn mapped_bindings<F>(
        calibration: &Calibration,
        slot: u8,
        mut translate: F,
    ) -> Result<BTreeMap<String, Binding>>
    where
        F: FnMut(u32, Option<AxisEndpoints>) -> Result<DigitalInput>,
    {
        let profile = crate::controller_catalog::catalog()
            .emulator_profiles
            .iter()
            .find(|profile| profile.id == PROFILE_ID)
            .context("Missing VBA-M native profile")?;
        let mut result = BTreeMap::new();
        for row in calibration.plan_profile(profile)?.rows {
            let input = row
                .input
                .as_ref()
                .context("VBA-M GBA control is not calibrated")?;
            let native = input
                .native
                .as_ref()
                .context("VBA-M requires measured native controls")?;
            let endpoints = input.axis.as_ref().map(|axis| AxisEndpoints {
                released: axis.released,
                pressed: axis.pressed,
            });
            ensure!(
                result
                    .insert(
                        row.target_id,
                        binding(translate(native.code, endpoints)?, slot)?
                    )
                    .is_none(),
                "VBA-M target control appears twice"
            );
        }
        ensure!(
            result.len() == CONTROLS.len()
                && CONTROLS
                    .iter()
                    .all(|(control, _)| result.contains_key(*control)),
            "VBA-M mapping is incomplete"
        );
        Ok(result)
    }

    #[derive(Debug)]
    struct DirectoryIdentity {
        path: PathBuf,
        canonical: PathBuf,
        identity: (u64, u64),
    }
    impl DirectoryIdentity {
        fn capture(path: PathBuf, purpose: &str) -> Result<Self> {
            ensure!(path.is_dir(), "VBA-M {purpose} directory is missing");
            ensure!(
                !fs::metadata(&path)?.permissions().readonly(),
                "VBA-M {purpose} directory is not writable"
            );
            let identity = crate::controller_native_platform::file_identity(&path)?;
            Ok(Self {
                canonical: path.canonicalize()?,
                path,
                identity,
            })
        }
        fn verify(&self) -> Result<()> {
            ensure!(
                self.path.canonicalize()? == self.canonical
                    && crate::controller_native_platform::file_identity(&self.path)?
                        == self.identity
                    && !fs::metadata(&self.path)?.permissions().readonly(),
                "VBA-M persistence directory changed"
            );
            Ok(())
        }
    }

    fn ini_value(text: &str, wanted_section: &str, wanted_key: &str) -> Result<Option<String>> {
        let mut active = false;
        let mut found_section = false;
        let mut value = None;
        for line in text.lines() {
            let line = line.trim();
            if let Some(section) = line
                .strip_prefix('[')
                .and_then(|value| value.strip_suffix(']'))
            {
                active = section.trim().eq_ignore_ascii_case(wanted_section);
                if active {
                    ensure!(
                        !found_section,
                        "VBA-M section {wanted_section} is duplicated"
                    );
                    found_section = true;
                }
                continue;
            }
            if active
                && let Some((key, current)) = line.split_once('=')
                && key.trim().eq_ignore_ascii_case(wanted_key)
            {
                ensure!(
                    value.is_none(),
                    "VBA-M key {wanted_section}/{wanted_key} is duplicated"
                );
                value = Some(current.trim().to_owned());
            }
        }
        Ok(value)
    }

    fn configured_directory(
        text: &str,
        key: &str,
        content: &Path,
        purpose: &str,
    ) -> Result<DirectoryIdentity> {
        let configured = ini_value(text, "General", key)?
            .unwrap_or_default()
            .replace("%s", "GameBoy Advance");
        let path = if configured.is_empty() {
            content
                .parent()
                .context("VBA-M content has no parent directory")?
                .to_path_buf()
        } else {
            let path = PathBuf::from(configured);
            ensure!(
                path.is_absolute(),
                "VBA-M relative {purpose} directory has frontend-dependent resolution; use an absolute path"
            );
            path
        };
        DirectoryIdentity::capture(path, purpose)
    }

    fn active_bios(text: &str, launch_directory: &Path) -> Result<Option<PathBuf>> {
        let enabled = ini_value(text, "preferences", "BootRomEn")?.unwrap_or_default();
        let enabled = match enabled.trim().to_ascii_lowercase().as_str() {
            "" | "0" | "false" | "no" | "off" => false,
            "1" | "true" | "yes" | "on" => true,
            _ => anyhow::bail!("VBA-M GBA BIOS enable value is invalid"),
        };
        if !enabled {
            return Ok(None);
        }
        let configured = ini_value(text, "GBA", "BiosFile")?
            .filter(|value| !value.is_empty())
            .context("VBA-M enables the GBA BIOS without a configured file")?;
        let path = PathBuf::from(configured);
        let path = if path.is_absolute() {
            path
        } else {
            launch_directory.join(path)
        };
        ensure!(
            path.is_file() && fs::metadata(&path)?.len() == 0x4000,
            "VBA-M active GBA BIOS must be a readable 16 KiB file"
        );
        Ok(Some(path.canonicalize()?))
    }

    enum Routing {
        Sdl2 {
            initial: Sdl2Snapshot,
            runtime_path: String,
            classic: Option<ClassicMap>,
            evdev: Option<EvdevMap>,
            controls: Option<ControlCounts>,
        },
        Sdl3 {
            initial: Sdl3Snapshot,
            runtime_path: String,
            physical: ClassicMap,
        },
    }
    impl Routing {
        fn verify(&self, setup: &settings::SavedSetup, cancel: &AtomicBool) -> Result<()> {
            match self {
                Self::Sdl2 {
                    initial,
                    runtime_path,
                    classic,
                    evdev,
                    controls,
                } => {
                    let fresh = observe_sdl2(setup, Some(runtime_path), cancel)?;
                    initial.ensure_same_routing(&sdl2_routing(fresh.clone()))?;
                    let device = fresh.device_at_path(runtime_path)?;
                    ensure!(
                        device.linux_classic == *classic
                            && device.linux_evdev == *evdev
                            && device.controls == *controls,
                        "VBA-M SDL2 raw control numbering changed"
                    );
                }
                Self::Sdl3 {
                    initial,
                    runtime_path,
                    physical,
                } => {
                    let fresh = observe_sdl3(setup, std::slice::from_ref(runtime_path), cancel)?;
                    ensure!(
                        comparable_sdl3(initial)? == comparable_sdl3(&fresh)?,
                        "VBA-M SDL3 routing changed before launch"
                    );
                    fresh.device_at_path(runtime_path)?;
                    ensure!(
                        lunchbox_controller_probe::linux_classic::read(Path::new(runtime_path))?
                            == *physical,
                        "VBA-M SDL3 classic control numbering changed"
                    );
                }
            }
            Ok(())
        }
    }

    pub(crate) struct PreparedSession {
        directory: tempfile::TempDir,
        pub(crate) private_config: PathBuf,
        device_index: Option<u32>,
        #[cfg(target_os = "linux")]
        topology: InputTopology,
        routing: Routing,
        setup: settings::SavedSetup,
        hashes: BTreeMap<PathBuf, String>,
        battery: DirectoryIdentity,
        states: DirectoryIdentity,
    }
    impl PreparedSession {
        pub(crate) fn prepare(
            setup: &settings::SavedSetup,
            calibrations: &std::collections::HashMap<String, Calibration>,
            inventory: &[ControllerDevice],
            launch_directory: &Path,
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
                "VBA-M physical controller is missing or ambiguous"
            );
            let selected = found[0].device_path.clone();
            let topology = InputTopology::capture(std::slice::from_ref(&selected))?;
            let calibration = calibrations
                .get(&player.controller_id)
                .context("VBA-M calibration disappeared")?;
            let mut prepared_index = None;
            let (routing, bindings) = match setup.sdl_api {
                SdlApi::Sdl2 => {
                    let initial = sdl2_routing(observe_sdl2(setup, None, cancel)?);
                    // Linux resolves through the sysfs topology; other hosts
                    // match the SDL device-interface path and require
                    // uniqueness.
                    #[cfg(target_os = "linux")]
                    let runtime_path = topology.resolve_runtime_path(
                        &selected,
                        initial
                            .devices
                            .iter()
                            .filter_map(|device| device.path.as_deref()),
                    )?;
                    #[cfg(not(target_os = "linux"))]
                    let runtime_path = {
                        let selected_string = selected.to_string_lossy().into_owned();
                        let candidates = initial
                            .devices
                            .iter()
                            .filter(|device| {
                                device.path.as_deref() == Some(selected_string.as_str())
                            })
                            .collect::<Vec<_>>();
                        ensure!(
                            candidates.len() == 1,
                            "VBA-M physical controller is missing or ambiguous in SDL"
                        );
                        selected_string
                    };
                    let captured = observe_sdl2(setup, Some(&runtime_path), cancel)?;
                    initial.ensure_same_routing(&sdl2_routing(captured.clone()))?;
                    #[cfg(target_os = "linux")]
                    topology.verify()?;
                    let device = captured.device_at_path(&runtime_path)?;
                    #[cfg(not(target_os = "linux"))]
                    platform::require_unique_device_path(
                        &captured.devices,
                        &runtime_path,
                        device.device_index,
                    )?;
                    let slot = u8::try_from(device.device_index)
                        .context("VBA-M SDL2 joystick slot is out of range")?;
                    prepared_index = Some(device.device_index);
                    let physical = PhysicalMap::from_device(device)?;
                    let classic = device.linux_classic.clone();
                    let evdev = device.linux_evdev.clone();
                    let controls = device.controls.clone();
                    let bindings = mapped_bindings(calibration, slot, |code, endpoints| {
                        physical.digital_input(code, endpoints)
                    })?;
                    (
                        Routing::Sdl2 {
                            initial,
                            runtime_path,
                            classic,
                            evdev,
                            controls,
                        },
                        bindings,
                    )
                }
                // The SDL3 path needs the classic Linux joydev backend
                // (`/dev/input/js*` plus direct kernel reads); other hosts
                // use the SDL2 API above.
                #[cfg(target_os = "linux")]
                SdlApi::Sdl3 => {
                    let initial = observe_sdl3(setup, &[], cancel)?;
                    let runtime_path = topology.resolve_runtime_path(
                        &selected,
                        initial
                            .devices
                            .iter()
                            .filter_map(|device| device.path.as_deref()),
                    )?;
                    ensure!(
                        runtime_path.starts_with("/dev/input/js"),
                        "VBA-M SDL3 needs the classic /dev/input/js* backend"
                    );
                    let captured =
                        observe_sdl3(setup, std::slice::from_ref(&runtime_path), cancel)?;
                    ensure!(
                        comparable_sdl3(&initial)? == comparable_sdl3(&captured)?,
                        "VBA-M SDL3 routing changed during capture"
                    );
                    topology.verify()?;
                    captured.device_at_path(&runtime_path)?;
                    let slot = captured
                        .devices
                        .iter()
                        .position(|device| device.path.as_deref() == Some(&runtime_path))
                        .context("VBA-M selected SDL3 joystick has no enumeration slot")?;
                    let slot =
                        u8::try_from(slot).context("VBA-M SDL3 joystick slot is out of range")?;
                    let physical =
                        lunchbox_controller_probe::linux_classic::read(Path::new(&runtime_path))?;
                    let bindings = mapped_bindings(calibration, slot, |code, endpoints| {
                        physical.digital_input(code, endpoints)
                    })?;
                    (
                        Routing::Sdl3 {
                            initial,
                            runtime_path,
                            physical,
                        },
                        bindings,
                    )
                }
                #[cfg(not(target_os = "linux"))]
                SdlApi::Sdl3 => {
                    anyhow::bail!(
                        "VBA-M SDL3 needs the classic Linux joydev backend; use the SDL2 API on this host"
                    )
                }
            };
            let baseline =
                fs::read(&setup.config_path).context("Reading declared VBA-M configuration")?;
            let baseline_text = std::str::from_utf8(&baseline).context("VBA-M INI is not UTF-8")?;
            let battery =
                configured_directory(baseline_text, "BatteryDir", &setup.content, "battery-save")?;
            let states = configured_directory(baseline_text, "StateDir", &setup.content, "state")?;
            let bios = active_bios(baseline_text, launch_directory)?;
            let directory = tempfile::Builder::new()
                .prefix("lunchbox-vba-m-native-")
                .tempdir()?;
            let private_config = directory.path().join(match setup.frontend {
                Frontend::Qt => "vbam-qt.ini",
                Frontend::Wx => "vbam.ini",
            });
            fs::write(
                &private_config,
                patch_config(&baseline, setup.frontend, &bindings)?,
            )?;
            let mut hashes = BTreeMap::new();
            for path in [
                &setup.content,
                &setup.config_path,
                &setup.probe_program,
                &setup.sdl_library,
                &private_config,
            ] {
                hashes.insert(path.clone(), file_hash(path)?);
            }
            if let Some(path) = bios {
                hashes.insert(path.clone(), file_hash(&path)?);
            }
            let session = Self {
                directory,
                private_config,
                device_index: prepared_index,
                #[cfg(target_os = "linux")]
                topology,
                routing,
                setup: setup.clone(),
                hashes,
                battery,
                states,
            };
            session.verify(cancel)?;
            Ok(session)
        }

        pub(crate) fn verify(&self, cancel: &AtomicBool) -> Result<()> {
            cancelled(cancel)?;
            ensure!(
                self.directory.path().is_dir() && self.private_config.is_file(),
                "VBA-M private configuration disappeared"
            );
            #[cfg(target_os = "linux")]
            self.topology.verify()?;
            self.battery.verify()?;
            self.states.verify()?;
            for (path, expected) in &self.hashes {
                ensure!(
                    file_hash(path)? == *expected,
                    "VBA-M launch input changed: {}",
                    path.display()
                );
            }
            self.routing.verify(&self.setup, cancel)?;
            // SDL2 sessions additionally pin the device path plus index;
            // SDL3 sessions only exist on Linux (classic backend).
            #[cfg(not(target_os = "linux"))]
            if let Some(index) = self.device_index {
                let Routing::Sdl2 { runtime_path, .. } = &self.routing else {
                    anyhow::bail!("VBA-M SDL3 sessions need Linux");
                };
                let fresh = observe_sdl2(&self.setup, Some(runtime_path), cancel)?;
                platform::require_unique_device_path(&fresh.devices, runtime_path, index)?;
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
            self.battery.verify()?;
            self.states.verify()?;
            #[cfg(target_os = "linux")]
            return self.topology.verify();
            #[cfg(not(target_os = "linux"))]
            return Ok(());
        }
    }

    #[cfg(test)]
    mod tests {
        use super::*;

        #[test]
        fn resolves_guarded_persistence_and_active_bios_paths() {
            let root = tempfile::tempdir().unwrap();
            let content_dir = root.path().join("content");
            let configured = root.path().join("GameBoy Advance");
            fs::create_dir(&content_dir).unwrap();
            fs::create_dir(&configured).unwrap();
            let content = content_dir.join("reader.gba");

            let blank = configured_directory(
                "[General]\nBatteryDir=\n",
                "BatteryDir",
                &content,
                "battery-save",
            )
            .unwrap();
            assert_eq!(blank.canonical, content_dir.canonicalize().unwrap());

            let ini = format!("[General]\nStateDir={}/%s\n", root.path().to_string_lossy());
            let expanded = configured_directory(&ini, "StateDir", &content, "state").unwrap();
            assert_eq!(expanded.canonical, configured.canonicalize().unwrap());
            assert!(
                configured_directory(
                    "[General]\nStateDir=relative\n",
                    "StateDir",
                    &content,
                    "state",
                )
                .is_err()
            );

            let bios = root.path().join("gba.bin");
            fs::write(&bios, vec![0u8; 0x4000]).unwrap();
            let active = active_bios(
                "[preferences]\nBootRomEn=true\n[GBA]\nBiosFile=gba.bin\n",
                root.path(),
            )
            .unwrap()
            .unwrap();
            assert_eq!(active, bios.canonicalize().unwrap());
        }
    }
}

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
                "VBA-M executable differs from the saved trusted runtime"
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
                "VBA-M launch plan changed after preparation"
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
            anyhow::bail!("VBA-M calibrated launch requires a native build")
        };
        ensure!(
            option.emulator_name.eq_ignore_ascii_case("VBA-M")
                && setup.emulator_id == option.emulator_id
                && original.environment.is_empty(),
            "VBA-M identity differs or custom environment needs resolution"
        );
        let executable = executable.canonicalize()?;
        ensure!(
            executable == original.program.canonicalize()?,
            "VBA-M executable differs from selection"
        );
        ensure!(
            original
                .arguments
                .iter()
                .filter(|argument| *argument == setup.content.as_os_str())
                .count()
                == 1
                && !original.arguments.iter().any(|argument| {
                    let value = argument.to_string_lossy();
                    value == "-c" || value == "--config" || value.starts_with("--config=")
                }),
            "VBA-M calibrated launch needs the saved content exactly once and no competing config option"
        );
        ensure!(
            file_hash(&executable)?.eq_ignore_ascii_case(&setup.executable_sha256),
            "VBA-M executable differs from the saved trusted runtime"
        );
        let inputs = session::PreparedSession::prepare(
            setup,
            calibrations,
            inventory,
            &original.current_directory,
            cancel,
        )?;
        let mut plan = original.clone();
        let mut arguments = vec![
            "--config".into(),
            inputs.private_config.as_os_str().to_owned(),
        ];
        arguments.extend(original.arguments.iter().cloned());
        plan.arguments = arguments;
        if setup.sdl_api == SdlApi::Sdl3 {
            plan.environment
                .push(("SDL_JOYSTICK_LINUX_CLASSIC".into(), "1".into()));
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

#[cfg(test)]
mod tests {
    use super::*;
    fn bindings() -> BTreeMap<String, Binding> {
        CONTROLS
            .iter()
            .enumerate()
            .map(|(index, (name, _))| {
                (
                    (*name).to_owned(),
                    Binding::JoystickButton {
                        device: 0,
                        button: index as u8,
                    },
                )
            })
            .collect()
    }

    #[test]
    fn patches_qt_baseline_and_preserves_unrelated_values() {
        let baseline = b"[General]\nBatteryDir=/saves\nStateDir=/states\n[Joypad]\nSDLGameControllerMode=true\n1\\Up=Space\n1\\MotionUp=Joy2-Axis3-\n2\\A=Joy2-Button4\n[preferences]\nBootRomEn=true\n[GBA]\nBiosFile=/firmware/gba.bin\n";
        let output = patch_config(baseline, Frontend::Qt, &bindings()).unwrap();
        assert!(output.contains("SDLGameControllerMode=false\n"));
        assert!(output.contains("1\\Up=Joy1-Button0\n"));
        assert!(output.contains("1\\Start=Joy1-Button9\n"));
        assert!(output.contains("1\\MotionUp=Joy2-Axis3-\n"));
        assert!(output.contains("2\\A=Joy2-Button4\n"));
        assert!(output.contains("BatteryDir=/saves\nStateDir=/states\n"));
        assert!(output.contains("BiosFile=/firmware/gba.bin\n"));
    }

    #[test]
    fn patches_wx_baseline_and_preserves_crlf() {
        let baseline = b"[Joypad]\r\nSDLGameControllerMode=1\r\n[Joypad/1]\r\nUp=Space\r\nMotionUp=Joy2-Axis3-\r\n[Joypad/2]\r\nA=Joy2-Button4\r\n";
        let output = patch_config(baseline, Frontend::Wx, &bindings()).unwrap();
        assert!(output.contains("SDLGameControllerMode=false\r\n"));
        assert!(output.contains("[Joypad/1]\r\nUp=Joy1-Button0\r\n"));
        assert!(output.contains("Start=Joy1-Button9\r\n"));
        assert!(output.contains("MotionUp=Joy2-Axis3-\r\n"));
        assert!(output.contains("[Joypad/2]\r\nA=Joy2-Button4\r\n"));
    }

    #[test]
    fn emits_axis_hat_and_rejects_duplicate_or_malformed_input() {
        let mut map = bindings();
        map.insert(
            "up".into(),
            Binding::JoystickAxis {
                device: 1,
                axis: 2,
                positive: false,
            },
        );
        map.insert(
            "down".into(),
            Binding::JoystickHat {
                device: 1,
                hat: 0,
                direction: HatDirection::South,
            },
        );
        let output = input_ini(&map).unwrap();
        assert!(output.contains("1\\Up=Joy2-Axis2-\n"));
        assert!(output.contains("1\\Down=Joy2-Hat0S\n"));
        let mut duplicate = bindings();
        duplicate.insert("b".into(), duplicate["a"].clone());
        assert!(patch_config(b"", Frontend::Qt, &duplicate).is_err());
        assert!(patch_config(b"[Joypad]\n[Joypad]\n", Frontend::Qt, &bindings()).is_err());
        assert!(patch_config(b"[Joypad]\n1\\A=x\n1\\A=y\n", Frontend::Qt, &bindings()).is_err());
    }
}
