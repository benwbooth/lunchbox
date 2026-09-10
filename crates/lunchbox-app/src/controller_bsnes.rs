//! bsnes v115+ native `settings.bml` controller mappings (SDL joypad driver),
//! not libretro RetroPad bindings.
//!
//! Functional contract pinned to bsnes-emu/bsnes
//! 7d5aa1e656b9171524d01b1b22917197d8121cb4:
//! - `target-bsnes/input/input.cpp`: assignment grammar `0x{id}/{group}/{input}`
//!   with optional `/Lo`, `/Hi` qualifiers; four assignments per control joined
//!   by `;`; digital axis/hat thresholds at ±16384; node paths built as
//!   `{System}/{Port}/{Device}/{Input}` with spaces removed.
//! - `nall/hid.hpp`: Joypad GroupID order Axis=0, Hat=1, Trigger=2, Button=3;
//!   SDL joypads use generic vendor 0x0000/product 0x0003 ids, so a device id
//!   is `sdl_joystick_index << 32 | 3`.
//! - `ruby/input/joypad/sdl.cpp`: device index is SDL's enumeration index; hat
//!   input `2*hat+0` is X (Left=Lo, Right=Hi) and `2*hat+1` is Y (Up=Lo,
//!   Down=Hi); axis and button indices follow SDL joystick order.
//! - `target-bsnes/bsnes.cpp`: `--settings=<file>` selects a private settings
//!   file, a bare path argument loads the game, and the file is rewritten at
//!   startup, so the user's own settings.bml is never touched.
//! - `sfc/interface/interface.cpp`: port devices None/Gamepad; both controller
//!   ports default to Gamepad, so an unused port is set to None explicitly.
use anyhow::{Result, ensure};
use std::collections::BTreeMap;

#[cfg(target_os = "linux")]
pub(crate) mod native_command;
#[cfg(target_os = "linux")]
pub(crate) mod session;
pub(crate) mod settings;

/// Standard SNES gamepad controls: target layout id -> settings.bml node name.
pub(crate) const CONTROLS: [(&str, &str); 12] = [
    ("up", "Up"),
    ("down", "Down"),
    ("left", "Left"),
    ("right", "Right"),
    ("b", "B"),
    ("a", "A"),
    ("y", "Y"),
    ("x", "X"),
    ("l", "L"),
    ("r", "R"),
    ("select", "Select"),
    ("start", "Start"),
];

/// HID::Joypad::GroupID values in nall's declaration order.
pub(crate) const GROUP_AXIS: u32 = 0;
pub(crate) const GROUP_HAT: u32 = 1;
pub(crate) const GROUP_BUTTON: u32 = 3;

/// The runtime's digital threshold for axis/hat qualifiers.
pub(crate) const DIGITAL_THRESHOLD: i32 = 16384;

/// Device id for an SDL joypad at `index` in bsnes's own enumeration.
pub(crate) fn device_id(index: u32) -> u64 {
    (u64::from(index) << 32) | 3
}

/// One native assignment. Hat inputs carry the full `2*hat + component` index.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Binding {
    Button(u32),
    AxisLo(u32),
    AxisHi(u32),
    HatLo(u32),
    HatHi(u32),
}

impl Binding {
    pub(crate) fn assignment(&self, id: u64) -> String {
        match *self {
            Binding::Button(input) => format!("0x{id:x}/{GROUP_BUTTON}/{input}"),
            Binding::AxisLo(input) => format!("0x{id:x}/{GROUP_AXIS}/{input}/Lo"),
            Binding::AxisHi(input) => format!("0x{id:x}/{GROUP_AXIS}/{input}/Hi"),
            Binding::HatLo(input) => format!("0x{id:x}/{GROUP_HAT}/{input}/Lo"),
            Binding::HatHi(input) => format!("0x{id:x}/{GROUP_HAT}/{input}/Hi"),
        }
    }
}

/// One player's finished Gamepad mapping: node name -> `;`-joined assignments.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub(crate) struct Gamepad {
    pub(crate) assignments: BTreeMap<&'static str, String>,
}

impl Gamepad {
    pub(crate) fn build(index: u32, controls: &BTreeMap<&'static str, Binding>) -> Result<Self> {
        ensure!(
            controls.len() == CONTROLS.len()
                && CONTROLS
                    .iter()
                    .all(|(control, _)| controls.contains_key(control)),
            "bsnes needs every standard SNES gamepad control"
        );
        let id = device_id(index);
        let mut assignments = BTreeMap::new();
        for (control, node) in CONTROLS {
            assignments.insert(node, controls[control].assignment(id));
        }
        Ok(Self { assignments })
    }
}

/// Render a complete private settings.bml. Everything absent keeps bsnes's
/// own defaults; only the input section is expressed.
pub(crate) fn settings_bml(players: &[Option<Gamepad>]) -> String {
    let mut result = String::from("SuperFamicom\n");
    for (port, player) in players.iter().enumerate() {
        let port = port + 1;
        match player {
            Some(gamepad) => {
                result.push_str(&format!("  ControllerPort{port}: Gamepad\n    Gamepad\n"));
                for (_, node) in CONTROLS {
                    if let Some(value) = gamepad.assignments.get(node) {
                        result.push_str(&format!("      {node}: {value}\n"));
                    }
                }
            }
            None => result.push_str(&format!("  ControllerPort{port}: None\n")),
        }
    }
    result.push_str("  ExpansionPort: None\n");
    result
}

#[cfg(test)]
mod tests {
    use super::*;

    fn snes_controls() -> BTreeMap<&'static str, Binding> {
        CONTROLS
            .iter()
            .enumerate()
            .map(|(i, (control, _))| {
                (
                    *control,
                    match i {
                        0 => Binding::AxisLo(0), // stick left
                        1 => Binding::AxisHi(0), // stick right
                        2 => Binding::AxisLo(1), // stick up
                        3 => Binding::AxisHi(1), // stick down
                        4.. => Binding::Button(u32::try_from(i + 4).unwrap()),
                    },
                )
            })
            .collect()
    }

    #[test]
    fn assignments_use_the_pinned_grammar() {
        assert_eq!(format!("0x{:x}", device_id(0)), "0x3");
        assert_eq!(format!("0x{:x}", device_id(1)), "0x100000003");
        assert_eq!(Binding::Button(5).assignment(device_id(0)), "0x3/3/5");
        assert_eq!(
            Binding::AxisHi(2).assignment(device_id(1)),
            "0x100000003/0/2/Hi"
        );
        assert_eq!(Binding::AxisLo(0).assignment(device_id(0)), "0x3/0/0/Lo");
        assert_eq!(
            Binding::HatLo(4).assignment(device_id(2)),
            "0x200000003/1/4/Lo"
        );
        assert_eq!(Binding::HatHi(1).assignment(device_id(0)), "0x3/1/1/Hi");
    }

    #[test]
    fn settings_bml_names_every_snes_control_once() {
        let players = vec![Some(Gamepad::build(0, &snes_controls()).unwrap()), None];
        let text = settings_bml(&players);
        assert!(text.starts_with("SuperFamicom\n  ControllerPort1: Gamepad\n    Gamepad\n"));
        assert!(text.contains("      Up: 0x3/0/0/Lo\n"));
        assert!(text.contains("      Start: 0x3/3/15\n"));
        assert!(text.contains("  ControllerPort2: None\n"));
        assert!(text.contains("  ExpansionPort: None\n"));
        for (_, node) in CONTROLS {
            assert_eq!(text.matches(&format!("{node}: ")).count(), 1, "{node}");
        }
    }

    #[test]
    fn partial_control_sets_are_rejected() {
        let mut controls = snes_controls();
        controls.remove("start");
        assert!(Gamepad::build(0, &controls).is_err());
    }
}
