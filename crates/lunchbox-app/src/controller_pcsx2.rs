//! Standalone PCSX2 standard DualShock 2 destination contract.
//! Pinned PCSX2 98697735f1bb1a1452d975251269abd1019876d1, SIO/Pad.
use anyhow::{Result, ensure};
use std::collections::BTreeMap;

pub(crate) mod configuration;
pub(crate) mod data_tree;
pub(crate) mod folders;
pub(crate) mod game_settings;
pub(crate) mod isolation;
pub(crate) mod launch;
#[cfg(target_os = "linux")]
pub(crate) mod native_command;
pub(crate) mod physical;
pub(crate) mod prepared;
pub(crate) mod profile;
pub(crate) mod routing;
pub(crate) mod sdl;
#[cfg(target_os = "linux")]
pub(crate) mod session;
pub(crate) mod settings;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum BindingKind {
    Button,
    HalfAxis,
    Motor,
}

/// Names are native configuration keys, not libretro joypad identifiers.
pub(crate) fn bindings() -> BTreeMap<&'static str, BindingKind> {
    let mut bindings = BTreeMap::new();
    for key in [
        "Up", "Right", "Down", "Left", "Triangle", "Circle", "Cross", "Square", "Select", "Start",
        "L1", "R1", "L3", "R3", "Analog", "Pressure",
    ] {
        bindings.insert(key, BindingKind::Button);
    }
    for key in [
        "L2", "R2", "LUp", "LRight", "LDown", "LLeft", "RUp", "RRight", "RDown", "RLeft",
    ] {
        bindings.insert(key, BindingKind::HalfAxis);
    }
    for key in ["LargeMotor", "SmallMotor"] {
        bindings.insert(key, BindingKind::Motor);
    }
    bindings
}

/// Existing DualShock visual IDs translated into the standalone pad vocabulary.
/// Analog toggle and pressure modifier are additional optional native actions;
/// neither is silently assigned to a face button.
pub(crate) fn visual_routes() -> BTreeMap<&'static str, &'static str> {
    BTreeMap::from([
        ("up", "Up"),
        ("down", "Down"),
        ("left", "Left"),
        ("right", "Right"),
        ("a", "Circle"),
        ("b", "Cross"),
        ("x", "Triangle"),
        ("y", "Square"),
        ("select", "Select"),
        ("start", "Start"),
        ("l", "L1"),
        ("r", "R1"),
        ("l2", "L2"),
        ("r2", "R2"),
        ("l3", "L3"),
        ("r3", "R3"),
        ("stick_up", "LUp"),
        ("stick_down", "LDown"),
        ("stick_left", "LLeft"),
        ("stick_right", "LRight"),
        ("right_stick_up", "RUp"),
        ("right_stick_down", "RDown"),
        ("right_stick_left", "RLeft"),
        ("right_stick_right", "RRight"),
    ])
}

/// Native unified slot numbering; UI player order with multitaps is separate.
pub(crate) fn section(native_slot: u8) -> Result<String> {
    ensure!(native_slot < 8, "PCSX2 native pad slot is out of range");
    Ok(format!("Pad{}", native_slot + 1))
}
