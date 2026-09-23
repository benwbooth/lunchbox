//! ares v148's SDL joystick / VirtualPad contract. Never modify the user's file.
//! Sources: ares v148 desktop-ui/{input/input.cpp,settings/settings.cpp,
//! emulator/*.cpp} and ruby/input/joypad/sdl.cpp. SDL's raw joystick numbers
//! are deliberately distinct from gamepad enums and kernel evdev codes.
use crate::controller_catalog::{Calibration, Catalog, EmulatorProfile, InputBinding};
use anyhow::{Context, Result, ensure};
use lunchbox_controller_probe::{
    Device, Snapshot,
    bindings::{Input, Output},
};
use serde::Serialize;
use std::collections::{BTreeMap, BTreeSet};

mod runtime;
pub use runtime::prepare;

pub const PAD_KEYS: &[&str] = &[
    "Pad.Up",
    "Pad.Down",
    "Pad.Left",
    "Pad.Right",
    "Select",
    "Start",
    "A..South",
    "B..East",
    "X..West",
    "Y..North",
    "L-Bumper",
    "R-Bumper",
    "L-Trigger",
    "R-Trigger",
    "L-Stick..Click",
    "R-Stick..Click",
    "L-Up",
    "L-Down",
    "L-Left",
    "L-Right",
    "R-Up",
    "R-Down",
    "R-Left",
    "R-Right",
    "Rumble",
];

pub fn valid_output(profile: &EmulatorProfile, output: &str) -> bool {
    profile.core == "ares" && profile.native_launch.is_some() && PAD_KEYS.contains(&output)
}

/// Only default gamepad modes actually attached by ares on startup. Alternate
/// devices (e.g. PS DualShock, PCE multitap, MD six-button) need a separate
/// attachment contract; listing their artwork must not imply they were enabled.
pub fn add_profiles(db: &mut Catalog) -> Result<()> {
    let specs: &[(&str, usize, &[&str])] = &[
        ("arcade-six-button", 2, &["Arcade"]),
        ("arcade-eight-button", 2, &["Arcade"]),
        ("n64", 4, &["Nintendo 64", "Nintendo 64DD"]),
        (
            "nes",
            2,
            &[
                "Nintendo Entertainment System",
                "Nintendo Famicom Disk System",
            ],
        ),
        ("snes", 2, &["Super Nintendo Entertainment System"]),
        (
            "gameboy",
            1,
            &["Nintendo Game Boy", "Nintendo Game Boy Color"],
        ),
        ("gba", 1, &["Nintendo Game Boy Advance"]),
        ("gamegear", 1, &["Sega Game Gear"]),
        ("master-system", 2, &["Sega Master System", "Sega SG-1000"]),
        (
            "genesis-3",
            2,
            &["Sega Genesis", "Sega Mega Drive", "Sega CD", "Sega 32X"],
        ),
        ("playstation-digital", 2, &["Sony Playstation"]),
        (
            "pce-2",
            1,
            &[
                "NEC TurboGrafx-16",
                "NEC TurboGrafx-CD",
                "NEC PC Engine SuperGrafx",
                "PC Engine",
            ],
        ),
        (
            "ngp",
            1,
            &["SNK Neo Geo Pocket", "SNK Neo Geo Pocket Color"],
        ),
    ];
    for &(layout_id, players, platforms) in specs {
        let layout = db.layout(layout_id).context("Missing ares target layout")?;
        let mut bindings = BTreeMap::new();
        for control in &layout.controls {
            let output = match (layout_id, control.id.as_str()) {
                ("arcade-six-button" | "arcade-eight-button", "button1") => "X..West",
                ("arcade-six-button" | "arcade-eight-button", "button2") => "A..South",
                ("arcade-six-button" | "arcade-eight-button", "button3") => "B..East",
                ("arcade-six-button" | "arcade-eight-button", "button4") => "Y..North",
                ("arcade-six-button" | "arcade-eight-button", "button5") => "L-Bumper",
                ("arcade-six-button" | "arcade-eight-button", "button6") => "R-Bumper",
                ("arcade-eight-button", "button7") => "L-Trigger",
                ("arcade-eight-button", "button8") => "R-Trigger",
                (_, "up") => "Pad.Up",
                (_, "down") => "Pad.Down",
                (_, "left") => "Pad.Left",
                (_, "right") => "Pad.Right",
                (_, "start") => "Start",
                (_, "select") => "Select",
                (_, "l") => "L-Bumper",
                (_, "r") => "R-Bumper",
                (_, "l2") => "L-Trigger",
                (_, "r2") => "R-Trigger",
                (_, "stick_up") => "L-Up",
                (_, "stick_down") => "L-Down",
                (_, "stick_left") => "L-Left",
                (_, "stick_right") => "L-Right",
                ("n64", "a") | ("ngp", "a") => "A..South",
                ("n64", "b") => "X..West",
                ("ngp", "b") => "B..East",
                ("n64", "c_up") => "R-Up",
                ("n64", "c_down") => "R-Down",
                ("n64", "c_left") => "R-Left",
                ("n64", "c_right") => "R-Right",
                ("n64", "z") => "R-Trigger",
                ("genesis-3", "a") => "X..West",
                ("genesis-3", "c") => "B..East",
                (_, "b") => "A..South",
                (_, "a") => "B..East",
                (_, "y") => "X..West",
                (_, "x") => "Y..North",
                _ if control.optional => continue,
                _ => anyhow::bail!("Unreviewed ares control {layout_id}/{}", control.id),
            };
            bindings.insert(control.id.clone(), output);
        }
        db.emulator_profiles.push(serde_json::from_value(serde_json::json!({
            "id": format!("ares-{layout_id}"), "name": format!("ares · {}", layout.name),
            "core": "ares", "target_layout": layout_id, "transport": "ares-settings",
            "native_launch": {"platforms": platforms, "max_players": players},
            "status": "documented", "source": "https://github.com/ares-emulator/ares/tree/v148/desktop-ui/emulator",
            "conditions": ["ares 148 or newer, SDL input driver; saved calibration and player order are applied to a private settings file."],
            "bindings": bindings
        }))?);
    }
    Ok(())
}

pub fn profile(platform: &str) -> Option<&'static EmulatorProfile> {
    crate::controller_catalog::catalog()
        .emulator_profiles
        .iter()
        .find(|p| {
            p.core == "ares"
                && p.native_launch.as_ref().is_some_and(|n| {
                    n.platforms
                        .iter()
                        .any(|s| s.eq_ignore_ascii_case(platform.trim()))
                })
        })
}

/// Mirrors ares's GUID + same-GUID slot, not the process-local SDL instance ID.
/// Exact paths first select a physical controller; GUID alone never does so.
fn identifier(snapshot: &Snapshot, selected: &Device) -> Result<String> {
    let identity = |device: &Device| {
        if device.guid.len() == 32
            && device.guid.bytes().all(|b| b.is_ascii_hexdigit())
            && device.guid != "00000000000000000000000000000000"
        {
            device.guid.clone()
        } else {
            format!(
                "VID:{}|PID:{}",
                device.vendor,
                if device.product == 0 {
                    3
                } else {
                    device.product
                }
            )
        }
    };
    let position = snapshot
        .devices
        .iter()
        .position(|d| std::ptr::eq(d, selected))
        .context("Selected device is not from this runtime snapshot")?;
    let id = identity(selected);
    let slot = snapshot.devices[..position]
        .iter()
        .filter(|d| identity(d) == id)
        .count();
    Ok(format!("{id}/{slot}"))
}

#[cfg(target_os = "linux")]
fn physical_input(input: &InputBinding, device: &Device) -> Result<(u32, u32, i8)> {
    use lunchbox_controller_probe::{duckstation::DigitalInput, linux_classic};
    let native = input
        .native
        .as_ref()
        .context("Record this controller's physical buttons first")?;
    let path = device
        .path
        .as_ref()
        .context("SDL did not report a physical device path")?;
    let map = linux_classic::read(std::path::Path::new(path))?;
    if let Some(resolved) = &device.resolved {
        map.validate_counts(resolved)?;
    }
    let measured = input.axis.as_ref().map(|a| linux_classic::AxisEndpoints {
        released: a.released,
        pressed: a.pressed,
    });
    match map.digital_input(native.code, measured)? {
        DigitalInput::Button(index) => Ok((3, index, 0)),
        DigitalInput::Axis {
            index,
            released,
            pressed,
        } => {
            ensure!(
                (pressed < -16384 && released >= -16384) || (pressed > 16384 && released <= 16384),
                "This trigger's resting position cannot be represented by ares's axis threshold; record a digital button instead"
            );
            Ok((0, index, if pressed < 0 { -1 } else { 1 }))
        }
        DigitalInput::Hat { index, direction } => match direction {
            1 => Ok((1, index * 2 + 1, -1)),
            2 => Ok((1, index * 2, 1)),
            4 => Ok((1, index * 2 + 1, 1)),
            8 => Ok((1, index * 2, -1)),
            _ => anyhow::bail!("A diagonal hat cannot represent one ares direction"),
        },
    }
}

#[cfg(not(target_os = "linux"))]
fn physical_input(_: &InputBinding, _: &Device) -> Result<(u32, u32, i8)> {
    anyhow::bail!("This ares adapter needs an SDL3 calibration on this operating system")
}

fn sdl_input(input: &InputBinding, device: &Device) -> Result<(u32, u32, i8)> {
    ensure!(
        crate::controller_sdl3::valid_binding(input),
        "Invalid SDL3 saved button"
    );
    let resolved = device.resolved.as_ref().context("The emulator's SDL does not recognize this controller; update the emulator or record its raw buttons")?;
    let mut matches = resolved.bindings.iter().filter_map(|binding| {
        let direction = match binding.output {
            Output::Button { index } if input.kind == "button" && index == input.code & 0xffff => 1,
            Output::Axis { index, min, max }
                if input.kind == "axis" && index == input.code & 0xffff =>
            {
                // Invert the source when the SDL mapping reverses this axis.
                i32::from(input.direction) * (max - min).signum()
            }
            _ => return None,
        };
        Some((|| match binding.input {
            Input::Button { index } => Ok((3, index, 0)),
            Input::Axis { index, min, max } => {
                let endpoint = if direction > 0 { max } else { min };
                ensure!(
                    endpoint.abs() > 16384,
                    "SDL half-axis cannot reach ares's threshold"
                );
                Ok((0, index, if endpoint < 0 { -1 } else { 1 }))
            }
            Input::Hat { index, mask } => match mask {
                1 => Ok((1, index * 2 + 1, -1)),
                2 => Ok((1, index * 2, 1)),
                4 => Ok((1, index * 2 + 1, 1)),
                8 => Ok((1, index * 2, -1)),
                _ => anyhow::bail!("Unsupported SDL hat mapping"),
            },
        })())
    });
    let value = matches
        .next()
        .context("No matching raw SDL input for this saved button")??;
    ensure!(
        matches.next().is_none(),
        "Ambiguous raw SDL mapping for this button"
    );
    Ok(value)
}

pub fn player_bindings(
    calibration: &Calibration,
    profile: &EmulatorProfile,
    snapshot: &Snapshot,
    path: &str,
) -> Result<BTreeMap<String, String>> {
    ensure!(
        calibration.os == std::env::consts::OS,
        "Record this controller on this operating system first"
    );
    let device = snapshot.device_at_path(path)?;
    let identity = identifier(snapshot, device)?;
    let layout = crate::controller_catalog::catalog()
        .layout(&profile.target_layout)
        .unwrap();
    let rows = calibration.plan(&profile.id)?.rows;
    let resolve = |input: &InputBinding| {
        if calibration.backend == crate::controller_sdl3::BACKEND {
            sdl_input(input, device)
        } else {
            physical_input(input, device)
        }
    };
    let mut result = BTreeMap::new();
    for row in &rows {
        let Some(input) = row.input.as_ref() else {
            ensure!(
                layout
                    .controls
                    .iter()
                    .any(|c| c.id == row.target_id && c.optional),
                "No button assigned to {}",
                row.target
            );
            continue;
        };
        let (group, index, direction) = resolve(input)?;
        let suffix = match direction {
            -1 => "/Lo",
            1 => "/Hi",
            _ => "",
        };
        result.insert(
            row.output.clone(),
            format!("{identity}/{group}/{index}{suffix}"),
        );
    }
    append_twin_z(
        calibration,
        &rows,
        &resolve,
        &identity,
        profile,
        &mut result,
    )?;
    Ok(result)
}

/// A recorded input that shares its target with another physical control
/// instead of owning a plan row (N64 twin Z triggers). Device-free so both
/// the launch writer and the mapping preview share one rule.
#[derive(Debug, Clone, Serialize)]
pub struct TwinRoute {
    pub target_id: String,
    pub physical_id: String,
    pub output: String,
}

pub fn twin_routes(
    calibration: &Calibration,
    profile: &EmulatorProfile,
    rows: &[crate::controller_catalog::MappingRow],
) -> Vec<TwinRoute> {
    if profile.target_layout != "n64" {
        return Vec::new();
    }
    // Twins share the N64 Z target identity, not any one profile's output
    // vocabulary: ares calls it R-Trigger while other writers key the same
    // target id directly.
    let Some(primary) = rows
        .iter()
        .find(|row| row.target_id == "z" && row.input.is_some())
    else {
        return Vec::new();
    };
    ["z", "z_left", "z_right"]
        .into_iter()
        .filter(|secondary| {
            Some(*secondary) != primary.physical_id.as_deref()
                && !rows.iter().any(|row| {
                    row.input.is_some() && row.physical_id.as_deref() == Some(*secondary)
                })
                && calibration.bindings.contains_key(*secondary)
        })
        .map(|secondary| TwinRoute {
            target_id: primary.target_id.clone(),
            physical_id: secondary.to_owned(),
            output: primary.output.clone(),
        })
        .collect()
}

/// Nintendo 64 hardware exposes a single Z trigger while twin-trigger pads
/// (Brawler64 Z plus Z right) record two distinct inputs. The plan binds one;
/// an unassigned recorded twin is appended to the same R-Trigger list so
/// both triggers stay live. ares values are ';'-separated binding lists;
/// ares's own settings files store empty slots as ";;".
fn append_twin_z(
    calibration: &Calibration,
    rows: &[crate::controller_catalog::MappingRow],
    resolve: &dyn Fn(&InputBinding) -> Result<(u32, u32, i8)>,
    identity: &str,
    profile: &EmulatorProfile,
    result: &mut BTreeMap<String, String>,
) -> Result<()> {
    for twin in twin_routes(calibration, profile, rows) {
        let Some(recorded) = calibration.bindings.get(twin.physical_id.as_str()) else {
            continue;
        };
        let (group, index, direction) = resolve(recorded)?;
        let suffix = match direction {
            -1 => "/Lo",
            1 => "/Hi",
            _ => "",
        };
        let combined = format!(
            "{};{identity}/{group}/{index}{suffix}",
            result[twin.output.as_str()]
        );
        result.insert(twin.output, combined);
    }
    Ok(())
}

/// Small BML editing primitive: preserve all untouched lines verbatim, including
/// firmware paths, save directories, renderer choices and unknown settings.
fn remove_nodes(text: &str, roots: &BTreeSet<String>) -> Result<String> {
    let mut stack: Vec<(usize, String)> = Vec::new();
    let mut skip = None;
    let mut result = String::new();
    for line in text.lines() {
        ensure!(
            !line.starts_with('\t'),
            "Unsupported tab-indented ares settings"
        );
        let indent = line.len() - line.trim_start_matches(' ').len();
        let body = line.trim();
        if body.is_empty() || body.starts_with("//") {
            if skip.is_none() {
                result.push_str(line);
                result.push('\n');
            }
            continue;
        }
        while stack.last().is_some_and(|(level, _)| *level >= indent) {
            stack.pop();
        }
        let name = body.split_once(':').map_or(body, |(key, _)| key).trim();
        stack.push((indent, name.to_owned()));
        if skip.is_some_and(|level| indent <= level) {
            skip = None;
        }
        let path = stack
            .iter()
            .map(|(_, name)| name.as_str())
            .collect::<Vec<_>>()
            .join("/");
        if roots.contains(&path) {
            skip = Some(indent);
        }
        if skip.is_none() {
            result.push_str(line);
            result.push('\n');
        }
    }
    Ok(result)
}

pub fn configuration(base: &str, players: &[BTreeMap<String, String>]) -> Result<String> {
    ensure!(
        !players.is_empty() && players.len() <= 5,
        "Invalid ares player count"
    );
    // Per-core overrides take priority over VirtualPad. Suspend them only in
    // this private file. Also remove controller hotkeys that could fire on play.
    let mut roots: BTreeSet<String> = (1..=5).map(|p| format!("VirtualPad{p}")).collect();
    for line in base
        .lines()
        .filter(|line| !line.starts_with(' ') && !line.trim().is_empty())
    {
        let name = line.split(':').next().unwrap().trim();
        if name != "Input" {
            roots.insert(format!("{name}/Input"));
        }
    }
    roots.insert("Hotkey".into());
    let mut result = remove_nodes(base, &roots)?;
    // Keep keyboard shortcuts (ares assigns keyboard HID ID 1 on every OS),
    // but prevent old controller hotkeys from also firing on gameplay buttons.
    let mut hotkeys = false;
    for line in base.lines() {
        if !line.starts_with(' ') {
            hotkeys = line.trim() == "Hotkey";
            if hotkeys {
                result.push_str("Hotkey\n");
            }
        } else if hotkeys && let Some((key, bindings)) = line.trim().split_once(':') {
            let keyboard: Vec<_> = bindings
                .trim()
                .split(';')
                .filter(|binding| {
                    let mut parts = binding.split('/');
                    parts
                        .next()
                        .and_then(|id| id.strip_prefix("0x"))
                        .and_then(|id| u64::from_str_radix(id, 16).ok())
                        == Some(1)
                        && parts.next() == Some("0")
                })
                .collect();
            result.push_str(&format!(" {key}: {}\n", keyboard.join(";")));
        }
    }
    for port in 1..=5 {
        result.push_str(&format!("VirtualPad{port}\n"));
        for key in PAD_KEYS {
            let value = players
                .get(port - 1)
                .and_then(|p| p.get(*key))
                .map_or("", String::as_str);
            ensure!(
                !value.contains(['\n', '\r', '\0']),
                "Invalid ares binding value"
            );
            result.push_str(&format!(" {key}: {value}\n"));
        }
    }
    Ok(result)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn snapshot() -> Snapshot {
        serde_json::from_value(serde_json::json!({
            "schema_version":4, "host_os":std::env::consts::OS, "library":"/test/SDL3",
            "library_sha256":"", "sdl_version":3004012, "requested_hints":{}, "effective_hints":{},
            "mapping_database_sha256":null, "warnings":[], "devices": [
                {"instance_id":19,"name":"Identical pad","path":"/first","guid":"0123456789abcdef0123456789abcdef","vendor":1118,"product":654,"product_version":1,"is_gamepad":true,"reported_player_index":-1,"mapping":null},
                {"instance_id":3,"name":"Identical pad","path":"/second","guid":"0123456789abcdef0123456789abcdef","vendor":1118,"product":654,"product_version":1,"is_gamepad":true,"reported_player_index":-1,"mapping":null}
            ]
        })).unwrap()
    }

    #[test]
    fn device_identity_uses_exact_path_then_guid_slot_not_sdl_instance() {
        let snapshot = snapshot();
        let selected = snapshot.device_at_path("/second").unwrap();
        assert_eq!(
            identifier(&snapshot, selected).unwrap(),
            "0123456789abcdef0123456789abcdef/1"
        );
        assert!(snapshot.device_at_path("Identical pad").is_err());
    }

    #[test]
    fn private_settings_preserve_other_preferences_and_clear_conflicts() {
        let base = "Video\n Driver: Vulkan\n Scale: 3\nPaths\n Saves: C:/Games/My Saves/\nNintendo64\n Quality: SD\n Input\n  Controller.Port.1\n   Gamepad\n    A: old-binding\nVirtualPad1\n A..South: old-button\nVirtualPad4\n Start: stale-player\nHotkey\n Reset: old-button\n";
        let players = vec![BTreeMap::from([(
            "A..South".into(),
            "0123456789abcdef0123456789abcdef/0/3/5".into(),
        )])];
        let result = configuration(base, &players).unwrap();
        assert!(
            result.contains(
                " Scale: 3\nPaths\n Saves: C:/Games/My Saves/\nNintendo64\n Quality: SD\n"
            )
        );
        assert!(
            !result.contains("old-binding")
                && !result.contains("old-button")
                && !result.contains("stale-player")
        );
        assert!(result.contains("VirtualPad4\n Pad.Up: \n"));
        assert_eq!(configuration(&result, &players).unwrap(), result);
    }

    #[test]
    fn system_filter_and_port_limits_come_from_native_contracts() {
        assert_eq!(
            profile("Nintendo 64")
                .unwrap()
                .native_launch
                .as_ref()
                .unwrap()
                .max_players,
            4
        );
        assert_eq!(
            profile("Nintendo Game Boy")
                .unwrap()
                .native_launch
                .as_ref()
                .unwrap()
                .max_players,
            1
        );
        assert!(profile("Sony Playstation 2").is_none());
        let n64 = profile("Nintendo 64").unwrap();
        assert_eq!(n64.bindings["b"], "X..West");
        assert_eq!(n64.bindings["a"], "A..South");
        assert_eq!(n64.bindings["z"], "R-Trigger");
        assert_eq!(n64.bindings["c_left"], "R-Left");
    }

    #[test]
    fn sdl_gamepad_enums_are_translated_to_raw_joystick_inputs() {
        let mut snapshot = snapshot();
        snapshot.devices[0].resolved = Some(lunchbox_controller_probe::bindings::ResolvedGamepad {
            joystick_axes: 4,
            joystick_buttons: 12,
            joystick_hats: 1,
            bindings: vec![
                lunchbox_controller_probe::bindings::Binding {
                    input: Input::Button { index: 7 },
                    output: Output::Button { index: 0 },
                },
                lunchbox_controller_probe::bindings::Binding {
                    input: Input::Axis {
                        index: 3,
                        min: 32767,
                        max: -32768,
                    },
                    output: Output::Axis {
                        index: 0,
                        min: -32768,
                        max: 32767,
                    },
                },
                lunchbox_controller_probe::bindings::Binding {
                    input: Input::Hat { index: 0, mask: 1 },
                    output: Output::Button { index: 11 },
                },
            ],
        });
        let device = &snapshot.devices[0];
        assert_eq!(
            sdl_input(&crate::controller_sdl3::binding(0, 0), device).unwrap(),
            (3, 7, 0)
        );
        assert_eq!(
            sdl_input(&crate::controller_sdl3::binding(0, 1), device).unwrap(),
            (0, 3, -1)
        );
        assert_eq!(
            sdl_input(&crate::controller_sdl3::binding(11, 0), device).unwrap(),
            (1, 1, -1)
        );
        assert!(sdl_input(&crate::controller_sdl3::binding(5, 0), device).is_err());
    }

    #[test]
    fn twin_z_triggers_share_one_n64_z_output() {
        use crate::controller_sdl3;
        use lunchbox_controller_probe::bindings::{
            Binding, Input as ProbeInput, Output as ProbeOutput, ResolvedGamepad,
        };

        let layout = crate::controller_catalog::catalog()
            .layout("brawler64")
            .expect("brawler64 layout");
        // Every recorded control gets a distinct canonical SDL input; the
        // stick directions get distinct axes so resolution stays unambiguous.
        let mut axis = 0;
        let mut button = 0;
        let mut bindings = BTreeMap::new();
        for control in &layout.controls {
            let binding = if control.analog {
                let binding = controller_sdl3::binding(axis, if axis % 2 == 0 { -1 } else { 1 });
                axis += 1;
                binding
            } else {
                let binding = controller_sdl3::binding(button, 0);
                button += 1;
                binding
            };
            bindings.insert(control.id.clone(), binding);
        }
        let mut choices = BTreeMap::new();
        for target in [
            "a",
            "b",
            "c_down",
            "c_left",
            "c_right",
            "c_up",
            "down",
            "l",
            "left",
            "r",
            "right",
            "start",
            "up",
            "z",
            "stick_down",
            "stick_left",
            "stick_right",
            "stick_up",
        ] {
            choices.insert(target.to_owned(), target.to_owned());
        }
        let calibration = Calibration {
            target_mappings: BTreeMap::from([("ares-n64".to_owned(), choices)]),
            layout: "brawler64".into(),
            os: std::env::consts::OS.into(),
            backend: controller_sdl3::BACKEND.into(),
            bindings,
        };
        let profile = profile("Nintendo 64").expect("ares n64 profile");
        assert_eq!(profile.id, "ares-n64");

        let mut resolved = Vec::new();
        for index in 0..button {
            let index = u32::try_from(index).unwrap();
            resolved.push(Binding {
                input: ProbeInput::Button { index: 100 + index },
                output: ProbeOutput::Button { index },
            });
        }
        for index in 0..axis {
            let index = u32::try_from(index).unwrap();
            resolved.push(Binding {
                input: ProbeInput::Axis {
                    index: 100 + index,
                    min: -32767,
                    max: 32767,
                },
                output: ProbeOutput::Axis {
                    index,
                    min: -32768,
                    max: 32767,
                },
            });
        }
        let mut snapshot = snapshot();
        snapshot.devices[0].resolved = Some(ResolvedGamepad {
            joystick_axes: 8,
            joystick_buttons: 200,
            joystick_hats: 0,
            bindings: resolved,
        });
        let bindings = player_bindings(&calibration, profile, &snapshot, "/first").unwrap();
        let trigger = &bindings["R-Trigger"];
        let guid = "0123456789abcdef0123456789abcdef/0";
        assert!(
            trigger.starts_with(&format!("{guid}/3/")),
            "primary Z binding first, got {trigger}"
        );
        assert_eq!(
            trigger.split(';').count(),
            2,
            "both Z triggers live on one output, got {trigger}"
        );
        assert!(
            trigger.contains(';'),
            "twin binding uses the ';' list grammar, got {trigger}"
        );
    }

    #[test]
    fn twin_routes_cover_an_unassigned_recorded_twin_only() {
        use crate::controller_catalog::{Calibration, InputBinding, MappingRow};
        fn input() -> InputBinding {
            InputBinding {
                code: 1,
                kind: "button".into(),
                direction: 0,
                logical: "X".into(),
                native: None,
                axis: None,
            }
        }
        fn row(target: &str, physical: Option<&str>, output: &str) -> MappingRow {
            MappingRow {
                target_id: target.into(),
                target: target.into(),
                physical_id: physical.map(str::to_owned),
                physical: physical.unwrap_or_default().into(),
                input: physical.map(|_| input()),
                alternate_physical_id: None,
                alternate_input: None,
                output: output.into(),
                reason: String::new(),
            }
        }
        fn calibration() -> Calibration {
            Calibration {
                target_mappings: Default::default(),
                layout: "brawler64".into(),
                os: std::env::consts::OS.into(),
                backend: "gilrs-0.11".into(),
                bindings: BTreeMap::from([("z".into(), input()), ("z_right".into(), input())]),
            }
        }
        let n64 = profile("Nintendo 64").expect("ares n64 profile");
        let rows = vec![row("z", Some("z"), "R-Trigger")];
        let twins = twin_routes(&calibration(), n64, &rows);
        assert_eq!(twins.len(), 1);
        assert_eq!(twins[0].target_id, "z");
        assert_eq!(twins[0].physical_id, "z_right");
        assert_eq!(twins[0].output, "R-Trigger");

        // An already-consumed twin stays out.
        let rows = vec![
            row("z", Some("z"), "R-Trigger"),
            row("z2", Some("z_right"), "R-Trigger"),
        ];
        assert!(twin_routes(&calibration(), n64, &rows).is_empty());

        // Other systems never twin.
        let snes = profile("Super Nintendo Entertainment System").expect("ares snes profile");
        assert!(twin_routes(&calibration(), snes, &rows).is_empty());
    }

    #[test]
    fn no_empty_or_injected_player_mapping_files() {
        assert!(configuration("", &[]).is_err());
        assert!(configuration("", &vec![BTreeMap::new(); 6]).is_err());
        assert!(
            configuration(
                "",
                &[BTreeMap::from([(
                    "Start".into(),
                    "bad\nVirtualPad2".into()
                )])]
            )
            .is_err()
        );
    }
}
