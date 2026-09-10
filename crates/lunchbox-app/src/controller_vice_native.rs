//! VICE 3.x+ native `.vjm` joystick mappings and joy-device binding for the
//! SDL2 UI, not libretro bindings.
//!
//! Functional contract pinned to the VICE trunk snapshot mirrored at
//! VICE-Team/svn-mirror d322f7a8d6c269b97162c74e73214c58eaad9a71 (r46226):
//! - `joyport/joystick.c` `mapping_dump_header`/`mapping_dump_map`: a .vjm
//!   file holds `#` comments, `!CLEAR`, and entries
//!   `<device> <inputtype> <inputindex> <action> <params>`.
//!   inputtype 0=axis, 1=button, 2=hat. Buttons use the plain button index;
//!   hats use `hat*4 + (0=up,1=down,2=left,3=right)`; axes acting as digital
//!   joystick inputs use `axis*2 + (0=positive,1=negative)`.
//!   Action 1 (joystick) takes the pin bitmask 1=up, 2=down, 4=left, 8=right,
//!   16=fire, 32=fire2, 64=fire3.
//! - `joyport/joystick.c` `set_joystick_device`: resource `JoyDevice1`/`JoyDevice2`
//!   binds a host device to an emulated control port with the value
//!   `JOYDEV_REALJOYSTICK_MIN (=4) + host device index`; config files are
//!   `ResourceName=ResourceValue` lines (`resources.c`).
//! - `arch/sdl/joy.c`: `-joymap <file>` selects the mapping, `-config <file>`
//!   selects a private config file, `-joythreshold` (default 10000) and
//!   `-joyfuzz` (default 1000) drive digital axis engagement at |value| >
//!   10000 in SDL2 axis units; host devices register in SDL enumeration
//!   order, so the .vjm device column is the SDL joystick index.
use anyhow::{Context, Result};

#[cfg(target_os = "linux")]
pub(crate) mod native_command;
#[cfg(target_os = "linux")]
pub(crate) mod session;
pub(crate) mod settings;

/// Joystick pin bitmask values for the digital joystick action.
pub(crate) const PINS: [(&str, u32, bool); 7] = [
    ("up", 1, true),
    ("down", 2, true),
    ("left", 4, true),
    ("right", 8, true),
    ("fire", 16, true),
    ("fire2", 32, false),
    ("fire3", 64, false),
];

/// Default JoyThreshold; digital axes engage beyond ±10000, and the release
/// window adds JoyFuzz (1000), so travel must clear 11000 to be robust.
pub(crate) const DIGITAL_THRESHOLD: i32 = 10000;
pub(crate) const RELEASE_MARGIN: i32 = 1000;

/// One native .vjm entry line.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Binding {
    Button(u32),
    AxisPositive(u32),
    AxisNegative(u32),
    HatUp(u32),
    HatDown(u32),
    HatLeft(u32),
    HatRight(u32),
}

impl Binding {
    pub(crate) fn line(&self, device: u32, pin: u32) -> String {
        let (input_type, input) = match *self {
            Binding::Button(button) => (1, button),
            Binding::AxisPositive(axis) => (0, axis * 2),
            Binding::AxisNegative(axis) => (0, axis * 2 + 1),
            Binding::HatUp(hat) => (2, hat * 4),
            Binding::HatDown(hat) => (2, hat * 4 + 1),
            Binding::HatLeft(hat) => (2, hat * 4 + 2),
            Binding::HatRight(hat) => (2, hat * 4 + 3),
        };
        format!("{device} {input_type} {input} 1 {pin}")
    }
}

/// Render a complete .vjm body for the given per-device pin assignments.
pub(crate) fn joymap(assignments: &[(u32, &[(u32, Binding)])]) -> String {
    let mut result = String::from("!CLEAR\n");
    for (device, pins) in assignments {
        for (pin, binding) in *pins {
            result.push_str(&binding.line(*device, *pin));
            result.push('\n');
        }
    }
    result
}

/// Render the private config selecting which host device feeds each port.
pub(crate) fn config(devices: &[u32]) -> String {
    let mut result = String::new();
    for (port, device) in devices.iter().enumerate() {
        result.push_str(&format!(
            "JoyDevice{}={}\n",
            port + 1,
            4 + u64::from(*device)
        ));
    }
    result
}

pub(crate) fn pin(control: &str) -> Result<u32> {
    PINS.iter()
        .find(|(id, _, _)| *id == control)
        .map(|(_, pin, _)| *pin)
        .with_context(|| format!("VICE target {control} is outside the joystick contract"))
}

pub(crate) fn required_controls() -> impl Iterator<Item = &'static str> {
    PINS.iter()
        .filter(|(_, _, required)| *required)
        .map(|(id, _, _)| *id)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn entries_use_the_pinned_grammar() {
        assert_eq!(Binding::Button(5).line(0, 16), "0 1 5 1 16");
        assert_eq!(Binding::AxisPositive(1).line(2, 8), "2 0 2 1 8");
        assert_eq!(Binding::AxisNegative(0).line(0, 4), "0 0 1 1 4");
        assert_eq!(Binding::HatUp(0).line(1, 1), "1 2 0 1 1");
        assert_eq!(Binding::HatDown(1).line(1, 2), "1 2 5 1 2");
        assert_eq!(Binding::HatLeft(0).line(0, 4), "0 2 2 1 4");
        assert_eq!(Binding::HatRight(2).line(0, 8), "0 2 11 1 8");
    }

    #[test]
    fn joymap_clears_then_maps_every_pin() {
        let text = joymap(&[
            (
                0,
                &[(1, Binding::AxisNegative(1)), (16, Binding::Button(0))],
            ),
            (1, &[(8, Binding::HatRight(0))]),
        ]);
        assert!(text.starts_with("!CLEAR\n"));
        assert_eq!(text, "!CLEAR\n0 0 3 1 1\n0 1 0 1 16\n1 2 3 1 8\n");
    }

    #[test]
    fn config_binds_devices_through_the_real_joystick_offset() {
        assert_eq!(config(&[0]), "JoyDevice1=4\n");
        assert_eq!(config(&[0, 2]), "JoyDevice1=4\nJoyDevice2=6\n");
    }

    #[test]
    fn pins_cover_the_digital_joystick_actions() {
        assert_eq!(pin("up").unwrap(), 1);
        assert_eq!(pin("fire3").unwrap(), 64);
        assert!(pin("paddle").is_err());
        assert_eq!(required_controls().count(), 5);
    }
}
