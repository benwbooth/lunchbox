//! Edit a private copy of melonDS's native TOML; never write the source config.
use anyhow::{Context, Result, ensure};
use std::collections::{BTreeMap, BTreeSet};
use toml_edit::{DocumentMut, Item, Table, value};

const HOTKEYS: &[&str] = &[
    "Lid",
    "Mic",
    "Pause",
    "Reset",
    "FastForward",
    "FrameLimitToggle",
    "FullscreenToggle",
    "SwapScreens",
    "SwapScreenEmphasis",
    "SolarSensorDecrease",
    "SolarSensorIncrease",
    "FrameStep",
    "PowerButton",
    "VolumeUp",
    "VolumeDown",
    "AudioMuteToggle",
    "SlowMo",
    "FastForwardToggle",
    "SlowMoToggle",
    "GuitarGripGreen",
    "GuitarGripRed",
    "GuitarGripYellow",
    "GuitarGripBlue",
];

pub(crate) fn render(
    original: &str,
    joystick_index: u16,
    controls: &BTreeMap<String, super::Input>,
) -> Result<String> {
    ensure!(
        original.len() <= 4 * 1024 * 1024,
        "melonDS source configuration exceeds limit"
    );
    let routes = super::visual_routes();
    ensure!(
        controls.len() == routes.len() && routes.values().all(|key| controls.contains_key(*key)),
        "melonDS requires all twelve native standard button keys"
    );
    let mut document = original
        .parse::<DocumentMut>()
        .context("Invalid melonDS TOML configuration")?;
    let instance = document
        .entry("Instance0")
        .or_insert(Item::Table(Table::new()))
        .as_table_mut()
        .context("melonDS Instance0 must be a table")?;
    instance.insert("JoystickID", value(i64::from(joystick_index)));
    let joystick = instance
        .entry("Joystick")
        .or_insert(Item::Table(Table::new()))
        .as_table_mut()
        .context("melonDS joystick configuration must be a table")?;
    let mut used = BTreeSet::new();
    for (key, input) in controls {
        let encoded = input.encode()?;
        ensure!(
            used.insert(encoded),
            "melonDS standard controls must have distinct inputs"
        );
        joystick.insert(key, value(i64::from(encoded)));
    }
    // Existing keyboard shortcuts remain available. Clear native joystick
    // hotkeys so a game button cannot also pause, reset or change emulation.
    for key in HOTKEYS {
        joystick.insert(&format!("HK_{key}"), value(-1));
    }
    Ok(document.to_string())
}
