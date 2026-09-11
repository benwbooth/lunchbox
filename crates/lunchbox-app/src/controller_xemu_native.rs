//! xemu native `xemu.toml` controller mappings for the SDL3 gamepad input,
//! not libretro bindings.
//!
//! Functional contract pinned to mborgerson/xemu
//! fd0ae0c0a189d56e87f8e46073b15b287e4a1e1a:
//! - `ui/xemu.c` — `-config_path <file>` selects the configuration file
//!   (loaded instead of the user's and auto-saved back at exit) and
//!   `ui/xemu-settings.cc` resolves `sys.files.dvd_path` from it, mounting
//!   the XISO at boot (`ui/xemu.c` dvd handling via
//!   `xemu_settings_set_string(&g_config.sys.files.dvd_path, ...)`).
//! - `ui/xemu-input.c` — `input.bindings.port1..4` hold SDL GUID strings
//!   (from `SDL_GUIDToString`); `input.auto_bind` must be disabled so an
//!   empty port does not steal a device; the per-controller
//!   `input.gamepad_mappings[].controller_mapping` maps SDL_Gamepad
//!   standard button/axis indices to Xbox functions, validated at runtime
//!   against `SDL_GAMEPAD_*_COUNT` (`xemu_input_bindings_set_in_range`).
//! - `config_spec.yml` — the complete schema including the standard-index
//!   defaults (a=0…dpad_right=14; axes leftx=0…righttrigger=5) and the
//!   `invert_axis_*` flags.
//! - Numbering: the probe and the child both run with
//!   `SDL_JOYSTICK_LINUX_CLASSIC=1`, so SDL's classic /dev/input/js*
//!   backend defines the raw indices; the calibration's evdev codes are
//!   translated through the verified classic map and matched against SDL's
//!   own resolved gamepad bindings to obtain the standard indices xemu
//!   consumes.
use anyhow::{Result, ensure};

#[cfg(target_os = "linux")]
pub(crate) mod native_command;
#[cfg(target_os = "linux")]
pub(crate) mod session;
pub(crate) mod settings;

/// Xbox layout control -> controller_mapping field. Digital subset.
pub(crate) const BUTTON_FIELDS: [(&str, &str); 12] = [
    ("a", "a"),
    ("b", "b"),
    ("x", "x"),
    ("y", "y"),
    ("select", "back"),
    ("start", "start"),
    ("l", "lshoulder"),
    ("r", "rshoulder"),
    ("up", "dpad_up"),
    ("down", "dpad_down"),
    ("left", "dpad_left"),
    ("right", "dpad_right"),
];

/// Optional stick-click controls.
pub(crate) const OPTIONAL_BUTTON_FIELDS: [(&str, &str); 2] =
    [("l3", "lstick_btn"), ("r3", "rstick_btn")];

/// Analog pairs: (negative control, positive control, axis field, invert
/// flag field).
pub(crate) const AXIS_PAIRS: [(&str, &str, &str, &str); 4] = [
    (
        "stick_left",
        "stick_right",
        "axis_left_x",
        "invert_axis_left_x",
    ),
    (
        "stick_up",
        "stick_down",
        "axis_left_y",
        "invert_axis_left_y",
    ),
    (
        "right_stick_left",
        "right_stick_right",
        "axis_right_x",
        "invert_axis_right_x",
    ),
    (
        "right_stick_up",
        "right_stick_down",
        "axis_right_y",
        "invert_axis_right_y",
    ),
];

/// Analog triggers: (control, field).
pub(crate) const TRIGGER_FIELDS: [(&str, &str); 2] =
    [("l2", "axis_trigger_left"), ("r2", "axis_trigger_right")];

/// One resolved standard index (SDL_Gamepad button or axis number).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Standard {
    Button(u32),
    Axis(u32),
}

/// Render the private xemu.toml. `ports` pairs the port number with the
/// bound GUID and its mapping fields.
pub(crate) fn config_toml(
    ports: &[(usize, &str, &BTreeMapRef)],
    dvd_path: &std::path::Path,
    mcpx_bootrom: &std::path::Path,
    flashrom: &std::path::Path,
) -> Result<String> {
    use std::fmt::Write;
    ensure!(
        !ports.is_empty() && ports.len() <= 4,
        "xemu binds up to four ports"
    );
    let mut out = String::new();
    writeln!(out, "[sys.files]").expect("in-memory write");
    writeln!(
        out,
        "dvd_path = {}",
        toml_value(&dvd_path.to_string_lossy())
    )
    .expect("in-memory write");
    writeln!(
        out,
        "bootrom_path = {}",
        toml_value(&mcpx_bootrom.to_string_lossy())
    )
    .expect("in-memory write");
    writeln!(
        out,
        "flashrom_path = {}",
        toml_value(&flashrom.to_string_lossy())
    )
    .expect("in-memory write");
    out.push_str("\n[input.bindings]\nauto_bind = false\n");
    for (port, guid, _) in ports {
        writeln!(out, "port{port} = {}", toml_value(guid)).expect("in-memory write");
        writeln!(out, "port{port}_driver = \"usb-xbox-gamepad\"").expect("in-memory write");
    }
    for (port, guid, mapping) in ports {
        writeln!(out, "\n[[input.gamepad_mappings]]").expect("in-memory write");
        writeln!(out, "gamepad_id = {}", toml_value(guid)).expect("in-memory write");
        writeln!(out, "enable_rumble = true").expect("in-memory write");
        let _ = port;
        out.push_str("\n  [input.gamepad_mappings.controller_mapping]\n");
        for key in [
            "a",
            "b",
            "x",
            "y",
            "back",
            "guide",
            "start",
            "lstick_btn",
            "rstick_btn",
            "lshoulder",
            "rshoulder",
            "dpad_up",
            "dpad_down",
            "dpad_left",
            "dpad_right",
        ] {
            if let Some(value) = mapping.buttons.get(key) {
                writeln!(out, "  {key} = {value}").expect("in-memory write");
            }
        }
        for key in [
            "axis_left_x",
            "axis_left_y",
            "axis_right_x",
            "axis_right_y",
            "axis_trigger_left",
            "axis_trigger_right",
        ] {
            if let Some(value) = mapping.axes.get(key) {
                writeln!(out, "  {key} = {value}").expect("in-memory write");
            }
        }
        for key in [
            "invert_axis_left_x",
            "invert_axis_left_y",
            "invert_axis_right_x",
            "invert_axis_right_y",
        ] {
            writeln!(out, "  {key} = {}", mapping.inverted.contains(key)).expect("in-memory write");
        }
    }
    Ok(out)
}

fn toml_value(text: &str) -> String {
    format!("{text:?}")
}

/// The resolved per-controller mapping fields.
#[derive(Clone, Debug, Default, PartialEq)]
pub(crate) struct BTreeMapRef {
    pub(crate) buttons: std::collections::BTreeMap<String, u32>,
    pub(crate) axes: std::collections::BTreeMap<String, u32>,
    pub(crate) inverted: std::collections::BTreeSet<String>,
}

#[cfg(test)]
mod tests {
    use super::*;

    fn mapping() -> BTreeMapRef {
        let mut map = BTreeMapRef::default();
        map.buttons.insert("a".into(), 0);
        map.buttons.insert("dpad_up".into(), 11);
        map.axes.insert("axis_left_x".into(), 0);
        map.axes.insert("axis_trigger_left".into(), 4);
        map.inverted.insert("invert_axis_left_x".into());
        map
    }

    #[test]
    fn config_toml_uses_the_pinned_schema() {
        let text = config_toml(
            &[(1, "0300abcd", &mapping())],
            std::path::Path::new("/games/game.iso"),
            std::path::Path::new("/bios/mcpx.bin"),
            std::path::Path::new("/bios/flash.bin"),
        )
        .unwrap();
        assert!(text.contains("[sys.files]\n"));
        assert!(text.contains("dvd_path = \"/games/game.iso\"\n"));
        assert!(text.contains("bootrom_path = \"/bios/mcpx.bin\"\n"));
        assert!(text.contains("flashrom_path = \"/bios/flash.bin\"\n"));
        assert!(text.contains("\n[input.bindings]\nauto_bind = false\n"));
        assert!(text.contains("port1 = \"0300abcd\"\n"));
        assert!(text.contains("port1_driver = \"usb-xbox-gamepad\"\n"));
        assert!(text.contains("\n[[input.gamepad_mappings]]\n"));
        assert!(text.contains("gamepad_id = \"0300abcd\"\n"));
        assert!(text.contains("  [input.gamepad_mappings.controller_mapping]\n"));
        assert!(text.contains("  a = 0\n"));
        assert!(text.contains("  dpad_up = 11\n"));
        assert!(text.contains("  axis_left_x = 0\n"));
        assert!(text.contains("  axis_trigger_left = 4\n"));
        assert!(text.contains("  invert_axis_left_x = true\n"));
        assert!(text.contains("  invert_axis_left_y = false\n"));
    }

    #[test]
    fn port_count_is_validated() {
        let none: Vec<(usize, &str, &BTreeMapRef)> = Vec::new();
        assert!(
            config_toml(
                &none,
                std::path::Path::new("/g"),
                std::path::Path::new("/m"),
                std::path::Path::new("/f")
            )
            .is_err()
        );
    }
}
