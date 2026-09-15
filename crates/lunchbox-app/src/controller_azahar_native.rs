//! Azahar native Linux SDL input-profile contract.
//!
//! Pinned source: azahar-emu/azahar commit
//! `ec8201d42cd3d8e2ec1d69d5832be0389490ea47`.
//!
//! Azahar stores profiles in the QSettings `Controls/profiles` array and
//! stores each control as a serialized `Common::ParamPackage`.  XDG config
//! may be isolated while XDG data is left untouched, preserving NAND, SDMC,
//! keys and user save data.

use anyhow::{Context, Result, ensure};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

pub(crate) const PROFILE_ID: &str = "azahar:standalone-azahar-3ds";
pub(crate) const CONTROLS: [(&str, &str); 20] = [
    ("a", "button_a"),
    ("b", "button_b"),
    ("x", "button_x"),
    ("y", "button_y"),
    ("up", "button_up"),
    ("down", "button_down"),
    ("left", "button_left"),
    ("right", "button_right"),
    ("l", "button_l"),
    ("r", "button_r"),
    ("start", "button_start"),
    ("select", "button_select"),
    ("debug", "button_debug"),
    ("gpio14", "button_gpio14"),
    ("zl", "button_zl"),
    ("zr", "button_zr"),
    ("home", "button_home"),
    ("power", "button_power"),
    ("circle_pad", "circle_pad"),
    ("c_stick", "c_stick"),
];

#[derive(Clone, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub(crate) enum Binding {
    Button {
        guid: String,
        port: u32,
        index: u32,
    },
    Trigger {
        guid: String,
        port: u32,
        axis: u32,
    },
    Analog {
        guid: String,
        port: u32,
        x: u32,
        y: u32,
    },
}

impl Binding {
    fn param(&self) -> String {
        // SDL_JoystickGetGUIDString, which Azahar uses for its joystick map,
        // emits lower-case hexadecimal.  Canonicalizing here prevents an
        // otherwise valid upper-case caller value from missing that lookup.
        let guid = match self {
            Self::Button { guid, .. } | Self::Trigger { guid, .. } | Self::Analog { guid, .. } => {
                guid.to_ascii_lowercase()
            }
        };
        match self {
            Self::Button { port, index, .. } => format!(
                "engine:sdl,guid:{guid},port:{port},api:controller,button:{index},maptype:guid+port"
            ),
            Self::Trigger { port, axis, .. } => format!(
                "engine:sdl,guid:{guid},port:{port},api:controller,axis:{axis},direction:+,threshold:0.5,maptype:guid+port"
            ),
            Self::Analog { port, x, y, .. } => format!(
                "engine:sdl,guid:{guid},port:{port},api:controller,axis_x:{x},axis_y:{y},maptype:guid+port"
            ),
        }
    }
}

fn valid_guid(guid: &str) -> bool {
    guid.len() == 32 && guid.bytes().all(|b| b.is_ascii_hexdigit())
}

/// Render a complete QSettings INI containing one input profile.  Existing
/// unrelated settings can be prepended by the caller; this output is limited
/// to the source's `Controls` keys and is therefore safe to stage privately.
pub(crate) fn profile_ini(name: &str, mappings: &BTreeMap<String, Binding>) -> Result<String> {
    ensure!(
        !name.is_empty()
            && !name.chars().any(char::is_control)
            && !name.contains(['=', '\n', '\r', '[', ']']),
        "Azahar profile name is invalid"
    );
    ensure!(
        mappings.len() == CONTROLS.len(),
        "Azahar needs every 3DS gameplay mapping"
    );
    let mut out = String::from("[Controls]\nprofile=0\nprofiles\\size=1\nprofiles\\1\\name=");
    out.push_str(name);
    out.push_str("\nprofiles\\1\\input_maptype=2\n");
    let mut used = std::collections::BTreeSet::new();
    for (target, key) in CONTROLS {
        let binding = mappings
            .get(target)
            .ok_or_else(|| anyhow::anyhow!("Azahar mapping {target} is absent"))?;
        let guid = match binding {
            Binding::Button { guid, .. }
            | Binding::Trigger { guid, .. }
            | Binding::Analog { guid, .. } => guid,
        };
        ensure!(
            valid_guid(guid),
            "Azahar SDL GUID must be 32 hex characters"
        );
        let port = match binding {
            Binding::Button { port, .. }
            | Binding::Trigger { port, .. }
            | Binding::Analog { port, .. } => port,
        };
        ensure!(
            *port <= i32::MAX as u32,
            "Azahar SDL controller port is out of range"
        );
        match binding {
            Binding::Button { index, .. } => {
                ensure!(*index < 32, "Azahar SDL gamepad button is out of range");
            }
            Binding::Trigger { axis, .. } => {
                ensure!(*axis < 6, "Azahar SDL gamepad axis is out of range");
                ensure!(*axis >= 4, "Azahar trigger must use a trigger axis");
            }
            Binding::Analog { x, y, .. } => {
                ensure!(
                    *x < 6 && *y < 6 && *x != *y,
                    "Azahar analog axes are invalid"
                );
            }
        }
        let value = binding.param();
        ensure!(used.insert(value.clone()), "Azahar mapping is reused");
        out.push_str("profiles\\1\\");
        out.push_str(key);
        out.push('=');
        out.push_str(&value);
        out.push('\n');
    }
    out.push_str("profiles\\1\\motion_device=engine:motion_emu,update_period:100,sensitivity:0.01,tilt_clamp:90.0\nprofiles\\1\\touch_device=engine:emu_window\nprofiles\\1\\use_touchpad=false\n");
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn emits_qsettings_array_and_guid_port_mapping() {
        let mut map = BTreeMap::new();
        for (name, _) in CONTROLS {
            let binding = match name {
                "zl" => Binding::Trigger {
                    guid: "0123456789abcdef0123456789abcdef".into(),
                    port: 0,
                    axis: 4,
                },
                "zr" => Binding::Trigger {
                    guid: "0123456789abcdef0123456789abcdef".into(),
                    port: 0,
                    axis: 5,
                },
                "circle_pad" => Binding::Analog {
                    guid: "0123456789abcdef0123456789abcdef".into(),
                    port: 0,
                    x: 0,
                    y: 1,
                },
                "c_stick" => Binding::Analog {
                    guid: "0123456789abcdef0123456789abcdef".into(),
                    port: 0,
                    x: 2,
                    y: 3,
                },
                _ => Binding::Button {
                    guid: "0123456789abcdef0123456789abcdef".into(),
                    port: 0,
                    index: map.len() as u32,
                },
            };
            map.insert(name.to_owned(), binding);
        }
        let ini = profile_ini("Lunchbox", &map).unwrap();
        assert!(ini.contains("profiles\\size=1"));
        assert!(ini.contains("engine:sdl,guid:0123456789abcdef0123456789abcdef,port:0,api:controller,button:0,maptype:guid+port"));
        assert!(ini.contains("axis:4,direction:+,threshold:0.5"));
        assert!(ini.contains("axis_x:0,axis_y:1"));
    }

    #[test]
    fn canonicalizes_uppercase_sdl_guid() {
        let mut map = BTreeMap::new();
        for (name, _) in CONTROLS {
            let binding = if name == "circle_pad" {
                Binding::Analog {
                    guid: "ABCDEF0123456789ABCDEF0123456789".into(),
                    port: 0,
                    x: 0,
                    y: 1,
                }
            } else {
                Binding::Button {
                    guid: "ABCDEF0123456789ABCDEF0123456789".into(),
                    port: 0,
                    index: map.len() as u32,
                }
            };
            map.insert(name.to_owned(), binding);
        }
        let ini = profile_ini("Lunchbox", &map).unwrap();
        assert!(ini.contains("guid:abcdef0123456789abcdef0123456789"));
        assert!(!ini.contains("guid:ABCDEF0123456789ABCDEF0123456789"));
    }
}

/// Registered catalog profile id (`{core}:standalone-{layout}` convention).
/// Private input profile name staged in `qt-config.ini`.
pub(crate) const SESSION_PROFILE_NAME: &str = "Lunchbox";

/// Layout target ids covered by the native profile: the 16 game controls,
/// triggers from the New 3DS shoulders, four console buttons, and both
/// analog pairs from stick directions.
pub(crate) const ROUTES: [(&str, &str); 26] = [
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
    ("home", "Home"),
    ("power", "Power"),
    ("debug", "Debug"),
    ("gpio14", "GPIO14"),
    ("stick_up", "Circle pad up"),
    ("stick_down", "Circle pad down"),
    ("stick_left", "Circle pad left"),
    ("stick_right", "Circle pad right"),
    ("right_stick_up", "C-stick up"),
    ("right_stick_down", "C-stick down"),
    ("right_stick_left", "C-stick left"),
    ("right_stick_right", "C-stick right"),
];

/// Writer control each layout target feeds.
pub(crate) const TARGET_KEYS: [(&str, &str); 26] = [
    ("a", "a"),
    ("b", "b"),
    ("x", "x"),
    ("y", "y"),
    ("up", "up"),
    ("down", "down"),
    ("left", "left"),
    ("right", "right"),
    ("l", "l"),
    ("r", "r"),
    ("start", "start"),
    ("select", "select"),
    ("zl", "zl"),
    ("zr", "zr"),
    ("home", "home"),
    ("power", "power"),
    ("debug", "debug"),
    ("gpio14", "gpio14"),
    ("stick_up", "circle_pad"),
    ("stick_down", "circle_pad"),
    ("stick_left", "circle_pad"),
    ("stick_right", "circle_pad"),
    ("right_stick_up", "c_stick"),
    ("right_stick_down", "c_stick"),
    ("right_stick_left", "c_stick"),
    ("right_stick_right", "c_stick"),
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
        /// The user's real Azahar user directory (`user/`); its data entries
        /// are shared through symlinks while the config file stays private.
        pub user_dir: PathBuf,
        pub probe_program: PathBuf,
        pub sdl_library: PathBuf,
        pub executable_sha256: String,
        pub players: Vec<Player>,
    }

    impl SavedSetup {
        pub(crate) fn validate(&self) -> Result<()> {
            ensure!(
                !self.emulator_id.trim().is_empty(),
                "Azahar setup needs an emulator identity"
            );
            for path in [&self.content, &self.probe_program, &self.sdl_library] {
                ensure!(
                    path.is_absolute()
                        && !path
                            .components()
                            .any(|part| matches!(part, std::path::Component::ParentDir)),
                    "Azahar setup paths must be absolute without parent traversal"
                );
            }
            ensure!(
                self.user_dir.is_absolute()
                    && !self
                        .user_dir
                        .components()
                        .any(|part| matches!(part, std::path::Component::ParentDir)),
                "Azahar user directory must be absolute without parent traversal"
            );
            ensure!(
                self.executable_sha256.len() == 64
                    && self
                        .executable_sha256
                        .bytes()
                        .all(|byte| byte.is_ascii_hexdigit()),
                "Azahar setup needs a trusted executable SHA-256"
            );
            let limit = catalog()
                .emulator_profiles
                .iter()
                .find(|profile| profile.id == PROFILE_ID)
                .and_then(|profile| profile.native_launch.as_ref())
                .context("Missing Azahar native profile")?
                .max_players;
            ensure!(
                self.players.len() == 1 && self.players[0].player == 1,
                "Azahar supports exactly player one"
            );
            ensure!(
                self.players.len() <= limit && !self.players[0].controller_id.trim().is_empty(),
                "Azahar player needs a saved controller identity"
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
                .context("Missing Azahar native profile")?;
            let player = &self.players[0];
            let calibration = calibrations
                .get(&player.controller_id)
                .context("Azahar controller has no saved calibration")?;
            ensure!(
                ["linux", "macos", "windows"].contains(&calibration.os.as_str()),
                "Azahar mapping requires Linux physical calibration"
            );
            let mapping = calibration.plan_profile(profile)?;
            ensure!(
                mapping.rows.iter().all(|row| row.physical_id.is_some()
                    && row
                        .input
                        .as_ref()
                        .is_some_and(|input| input.native.is_some())),
                "Azahar needs native calibration for every 3DS control"
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
                "detail": "Native launch runs in a session directory with a private qt-config.ini, then rechecks the exact SDL2 routes. Only the single 3DS pad on a unique GUID is supported; runtime behavior remains unverified."
            }))
        }
    }

    pub(crate) fn validate_setups(setups: &[SavedSetup]) -> Result<()> {
        ensure!(setups.len() <= 1024, "Too many Azahar saved setups");
        let mut identities = std::collections::BTreeSet::new();
        for setup in setups {
            setup.validate()?;
            ensure!(
                identities.insert((&setup.emulator_id, &setup.content)),
                "Duplicate Azahar emulator/content setup"
            );
        }
        Ok(())
    }
}

/// SDL2 `SDL_GameControllerButton` indices in enum order.
const SDL_BUTTONS: [&str; 21] = [
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
    "misc1",
    "paddle1",
    "paddle2",
    "paddle3",
    "paddle4",
    "touchpad",
];

/// SDL2 game-controller axis order.
const SDL_AXES: [&str; 6] = [
    "leftx",
    "lefty",
    "rightx",
    "righty",
    "lefttrigger",
    "righttrigger",
];

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
        os::unix::fs::symlink,
        path::{Path, PathBuf},
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
            serde_json::from_slice(&output).context("Invalid Azahar SDL2 capture")?;
        ensure!(
            snapshot.version[0] == 2
                && snapshot.library.canonicalize()? == setup.sdl_library.canonicalize()?
                && snapshot.library_sha256 == file_hash(&setup.sdl_library)?,
            "Azahar helper inspected a different SDL2 runtime"
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

    /// Parse an SDL2 game-controller mapping string into output→input pairs.
    fn mapping_fields(mapping: &str) -> BTreeMap<String, String> {
        let mut fields = BTreeMap::new();
        for entry in mapping.split(',').skip(2) {
            if let Some((key, value)) = entry.split_once(':') {
                fields.insert(key.trim().to_owned(), value.trim().to_owned());
            }
        }
        fields
    }

    /// Raw SDL2 button index for a game-controller button output.
    fn mapped_button(fields: &BTreeMap<String, String>, output: &str) -> Result<u32> {
        let input = fields
            .get(output)
            .with_context(|| format!("Azahar gamepad entry {output} is unmapped"))?;
        let input = input.trim_end_matches('~');
        let rest = input
            .strip_prefix('b')
            .context("Azahar gamepad entry is not a button")?;
        rest.parse().context("Azahar mapping button is invalid")
    }

    /// Translate one calibrated control to its SDL2 game-controller element:
    /// button index, hat-driven dpad index, or axis index with polarity.
    enum GamepadElement {
        Button(u32),
        Axis { index: u32, positive: bool },
    }

    fn gamepad_element(
        fields: &BTreeMap<String, String>,
        translated: DigitalInput,
        positive: bool,
    ) -> Result<GamepadElement> {
        Ok(match translated {
            DigitalInput::Button(index) => {
                let raw = u32::try_from(index).context("Azahar button index is too large")?;
                let mut found = None;
                for output in SDL_BUTTONS {
                    if let Ok(candidate) = mapped_button(fields, output)
                        && candidate == raw
                    {
                        found = Some(
                            SDL_BUTTONS
                                .iter()
                                .position(|name| *name == output)
                                .context("Azahar SDL button is unknown")?,
                        );
                        break;
                    }
                }
                GamepadElement::Button(
                    u32::try_from(found.context("Azahar raw button has no gamepad mapping")?)
                        .context("Azahar gamepad button is too large")?,
                )
            }
            DigitalInput::Hat { direction, .. } => {
                let want = match direction {
                    0x01 => "dpup",
                    0x04 => "dpdown",
                    0x08 => "dpleft",
                    0x02 => "dpright",
                    _ => anyhow::bail!("Azahar hat direction is not cardinal"),
                };
                // The hat must actually drive this dpad entry in the mapping.
                let input = fields
                    .get(want)
                    .with_context(|| format!("Azahar dpad entry {want} is unmapped"))?;
                ensure!(
                    input.trim_end_matches('~').starts_with('h'),
                    "Azahar dpad entry is not a hat"
                );
                GamepadElement::Button(
                    u32::try_from(
                        SDL_BUTTONS
                            .iter()
                            .position(|name| *name == want)
                            .context("Azahar SDL dpad is unknown")?,
                    )
                    .context("Azahar gamepad button is too large")?,
                )
            }
            DigitalInput::Axis { index, .. } => {
                let raw = u32::try_from(index).context("Azahar axis index is too large")?;
                let mut found = None;
                for output in SDL_AXES {
                    if let Some(input) = fields.get(output) {
                        let axis: u32 = input
                            .trim_start_matches(['+', '-'])
                            .trim_end_matches('~')
                            .strip_prefix('a')
                            .context("Azahar stick entry is not an axis")?
                            .parse()
                            .context("Azahar stick axis is invalid")?;
                        if axis == raw {
                            found = Some(
                                SDL_AXES
                                    .iter()
                                    .position(|name| *name == output)
                                    .context("Azahar SDL axis is unknown")?,
                            );
                            break;
                        }
                    }
                }
                GamepadElement::Axis {
                    index: u32::try_from(found.context("Azahar raw axis has no stick mapping")?)
                        .context("Azahar gamepad axis is too large")?,
                    positive,
                }
            }
        })
    }

    /// Mirror one filesystem entry as a symlink, recursing into directories.
    /// The caller skips the private config file itself.
    fn mirror_entry(source: &Path, link: &Path) -> Result<()> {
        let kind = fs::symlink_metadata(source)?.file_type();
        if kind.is_symlink() {
            symlink(fs::read_link(source)?, link)?;
        } else if kind.is_dir() {
            fs::create_dir(link)?;
            for entry in fs::read_dir(source)? {
                let entry = entry?;
                mirror_entry(&entry.path(), &link.join(entry.file_name()))?;
            }
        } else if kind.is_file() {
            symlink(source, link)?;
        } else {
            anyhow::bail!("Azahar user entry is not a file, directory, or symlink");
        }
        Ok(())
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
                "Azahar content must be a direct regular file with canonical ancestry"
            );
            ensure!(
                setup.user_dir.is_dir() && setup.user_dir.canonicalize()? == setup.user_dir,
                "Azahar user directory must be a canonical directory"
            );
            let player = &setup.players[0];
            let found = inventory
                .iter()
                .filter(|device| device.stable_id == player.controller_id)
                .collect::<Vec<_>>();
            ensure!(
                found.len() == 1 && !found[0].is_virtual,
                "Azahar physical controller is missing or ambiguous"
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
                    "azahar physical controller is missing or ambiguous in SDL"
                );
                selected_string
            };
            let captured = observe(setup, Some(&physical_path), cancel)?;
            initial.ensure_same_routing(&routing(captured.clone()))?;
            #[cfg(target_os = "linux")]
            topology.verify()?;
            let device = captured.device_at_path(&physical_path)?;
            let guid = device.guid.to_ascii_lowercase();
            ensure!(
                guid.len() == 32 && guid.bytes().all(|b| b.is_ascii_hexdigit()),
                "Azahar SDL GUID is invalid"
            );
            // `port` counts same-GUID joysticks; duplicates would make the
            // profile ambiguous, so the session requires a unique GUID.
            ensure!(
                initial
                    .devices
                    .iter()
                    .filter(|other| other.guid.to_ascii_lowercase() == guid)
                    .count()
                    == 1,
                "Azahar SDL GUID is duplicated; unplug the twin pad"
            );
            let fields = mapping_fields(
                device
                    .mapping
                    .as_deref()
                    .context("Azahar SDL mapping is absent")?,
            );
            let calibration = calibrations
                .get(&player.controller_id)
                .context("Azahar calibration disappeared")?;
            let profile = crate::controller_catalog::catalog()
                .emulator_profiles
                .iter()
                .find(|profile| profile.id == PROFILE_ID)
                .context("Missing Azahar native profile")?;
            let physical = PhysicalMap::from_device(device)?;
            let state = device
                .sampled_state
                .as_ref()
                .context("Azahar SDL released state is missing")?;
            state.validate(
                device
                    .controls
                    .as_ref()
                    .context("Azahar SDL control counts are missing")?,
            )?;
            // Every ParamPackage must be distinct, so every calibrated
            // control must resolve to a distinct raw element; halves pair
            // into shared stick axes.
            let mut raw_seen = std::collections::BTreeSet::new();
            let mut buttons: BTreeMap<String, u32> = BTreeMap::new();
            let mut sticks: BTreeMap<String, (u32, bool)> = BTreeMap::new();
            for row in calibration.plan_profile(profile)?.rows {
                ensure!(
                    ROUTES.iter().any(|(target, _)| *target == row.target_id),
                    "Azahar target {} outside contract",
                    row.target_id
                );
                let input = row
                    .input
                    .as_ref()
                    .context("Azahar 3DS control is not calibrated")?;
                let native = input
                    .native
                    .as_ref()
                    .context("Azahar requires measured native controls")?;
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
                    "Release the Azahar controls before launch preparation"
                );
                let element = gamepad_element(&fields, translated, native.direction > 0)?;
                match element {
                    GamepadElement::Button(gamepad) => {
                        ensure!(
                            raw_seen.insert(format!("b{gamepad}")),
                            "Azahar controls share a gamepad button; give debug/gpio14/home/power their own buttons"
                        );
                        ensure!(
                            buttons.insert(row.target_id, gamepad).is_none(),
                            "Azahar control appears twice"
                        );
                    }
                    GamepadElement::Axis { index, positive } => {
                        ensure!(
                            sticks
                                .insert(row.target_id.clone(), (index, positive))
                                .is_none(),
                            "Azahar stick direction appears twice"
                        );
                    }
                }
            }
            let pair = |negative: &str, positive: &str| {
                let (neg_index, neg_dir) = sticks.get(negative).with_context(|| {
                    format!("Azahar stick direction {negative} is not calibrated")
                })?;
                let (pos_index, pos_dir) = sticks.get(positive).with_context(|| {
                    format!("Azahar stick direction {positive} is not calibrated")
                })?;
                ensure!(
                    neg_index == pos_index && !neg_dir && *pos_dir,
                    "Azahar stick halves must share one axis with opposite polarity"
                );
                Ok::<u32, anyhow::Error>(*neg_index)
            };
            let circle_x = pair("stick_left", "stick_right")?;
            let circle_y = pair("stick_up", "stick_down")?;
            let c_x = pair("right_stick_left", "right_stick_right")?;
            let c_y = pair("right_stick_up", "right_stick_down")?;
            let mut mappings = BTreeMap::new();
            for (target, gamepad) in &buttons {
                let (_, key) = TARGET_KEYS
                    .iter()
                    .find(|(known, _)| known == target)
                    .context("Azahar target is outside the 3DS profile")?;
                if matches!(*key, "zl" | "zr" | "circle_pad" | "c_stick") {
                    continue;
                }
                mappings.insert(
                    (*key).to_owned(),
                    Binding::Button {
                        guid: guid.clone(),
                        port: 0,
                        index: *gamepad,
                    },
                );
            }
            // zl/zr resolve through trigger axes and the analogs through
            // the paired stick axes.
            mappings.insert(
                "zl".to_owned(),
                Binding::Trigger {
                    guid: guid.clone(),
                    port: 0,
                    axis: trigger_axis(buttons.get("zl"), &sticks, "zl")?,
                },
            );
            mappings.insert(
                "zr".to_owned(),
                Binding::Trigger {
                    guid: guid.clone(),
                    port: 0,
                    axis: trigger_axis(buttons.get("zr"), &sticks, "zr")?,
                },
            );
            mappings.insert(
                "circle_pad".to_owned(),
                Binding::Analog {
                    guid: guid.clone(),
                    port: 0,
                    x: circle_x,
                    y: circle_y,
                },
            );
            mappings.insert(
                "c_stick".to_owned(),
                Binding::Analog {
                    guid: guid.clone(),
                    port: 0,
                    x: c_x,
                    y: c_y,
                },
            );
            ensure!(
                (circle_x, circle_y) != (c_x, c_y),
                "Azahar circle pad and C-stick share axes; map them to distinct sticks"
            );
            ensure!(
                mappings.len() == CONTROLS.len(),
                "Azahar 3DS mapping is incomplete"
            );
            let directory = tempfile::Builder::new()
                .prefix("lunchbox-azahar-")
                .tempdir()?;
            // The user directory resolves as <cwd>/user; mirror every data
            // entry through symlinks so saves survive, but keep the config
            // file private.
            let user_dir = directory.path().join("user");
            fs::create_dir(&user_dir)?;
            for entry in fs::read_dir(&setup.user_dir)? {
                let entry = entry?;
                if entry.file_name() == "config" {
                    continue;
                }
                mirror_entry(&entry.path(), &user_dir.join(entry.file_name()))?;
            }
            let config_dir = user_dir.join("config");
            fs::create_dir(&config_dir)?;
            let real_config = setup.user_dir.join("config");
            if real_config.is_dir() {
                for entry in fs::read_dir(&real_config)? {
                    let entry = entry?;
                    if entry.file_name() == "qt-config.ini" {
                        continue;
                    }
                    mirror_entry(&entry.path(), &config_dir.join(entry.file_name()))?;
                }
            }
            let config_path = config_dir.join("qt-config.ini");
            fs::write(&config_path, profile_ini(SESSION_PROFILE_NAME, &mappings)?)?;
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
                ensure!(file_hash(path)? == *hash, "Azahar launch input changed");
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

    /// Trigger axis for zl/zr: the calibration must land on a trigger axis
    /// in the mapping, never a button.
    fn trigger_axis(
        button_hit: Option<&u32>,
        sticks: &BTreeMap<String, (u32, bool)>,
        target: &str,
    ) -> Result<u32> {
        // If the target resolved as a button, the user mapped a button.
        if let Some(raw) = button_hit {
            anyhow::bail!("Azahar {target} must be a trigger axis, not a button ({raw})");
        }
        // Otherwise it resolved as an axis half; find which gamepad axis.
        let mut found = None;
        for (name, (index, _)) in sticks {
            if name == target {
                found = Some(*index);
                break;
            }
        }
        let index = found.context("Azahar trigger is not calibrated")?;
        // Triggers are axes 4/5 in game-controller order.
        ensure!(
            index == 4 || index == 5,
            "Azahar trigger must use a trigger axis"
        );
        Ok(index)
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
                "Azahar executable differs from the saved trusted runtime"
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
                "Azahar launch plan changed after preparation"
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
                    "Azahar exited before controller handoff"
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
                    "Azahar did not open the selected SDL controller before timeout"
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
            anyhow::bail!("Azahar calibrated launch requires a native build");
        };
        ensure!(
            option.emulator_name.eq_ignore_ascii_case("Azahar")
                && setup.emulator_id == option.emulator_id
                && original.environment.is_empty()
                && original.retroarch_content.is_none(),
            "Azahar identity differs or custom environment needs resolution"
        );
        let executable = executable.canonicalize()?;
        ensure!(
            executable == original.program.canonicalize()?,
            "Azahar launch executable differs from selection"
        );
        ensure!(
            original.arguments.as_slice() == [setup.content.as_os_str()],
            "Azahar calibrated launch requires exactly the saved content argument"
        );
        ensure!(
            file_hash(&executable)?.eq_ignore_ascii_case(&setup.executable_sha256),
            "Azahar executable differs from the saved trusted runtime"
        );
        let inputs = session::PreparedSession::prepare(setup, calibrations, inventory, cancel)?;
        let mut plan = original.clone();
        plan.program = executable.clone();
        // The user directory resolves as <cwd>/user; running there selects
        // the private config while saves survive through symlinks. The game
        // keeps its default positional slot.
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
