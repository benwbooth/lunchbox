//! RPCS3 standard pad schema, pinned 54014a7de4b2ccec98c9c0cb7dbebec0606c5cd6.
use anyhow::{Result, ensure};
use std::collections::BTreeMap;

pub(crate) mod configuration;
pub(crate) mod isolation;
pub(crate) mod launch;
#[cfg(target_os = "linux")]
pub(crate) mod native_command;
pub(crate) mod paths;
pub(crate) mod physical;
pub(crate) mod prepared;
pub(crate) mod routing;
#[cfg(target_os = "linux")]
pub(crate) mod session;
pub(crate) mod settings;
pub(crate) mod startup;

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
        ("stick_up", "Left Stick Up"),
        ("stick_down", "Left Stick Down"),
        ("stick_left", "Left Stick Left"),
        ("stick_right", "Left Stick Right"),
        ("right_stick_up", "Right Stick Up"),
        ("right_stick_down", "Right Stick Down"),
        ("right_stick_left", "Right Stick Left"),
        ("right_stick_right", "Right Stick Right"),
    ])
}

/// RPCS3's SDL vocabulary is positional; raw SDL axis/button ordinals are not
/// accepted as mapping strings. Physical translation must resolve gamepad outputs.
pub(crate) const SDL_INPUTS: &[&str] = &[
    "South",
    "East",
    "West",
    "North",
    "Left",
    "Right",
    "Up",
    "Down",
    "LB",
    "RB",
    "Back",
    "Start",
    "LS",
    "RS",
    "Guide",
    "Misc 1",
    "Misc 2",
    "Misc 3",
    "Misc 4",
    "Misc 5",
    "Misc 6",
    "R Paddle 1",
    "L Paddle 1",
    "R Paddle 2",
    "L Paddle 2",
    "Touchpad",
    "Touch Left",
    "Touch Right",
    "Touch Up",
    "Touch Down",
    "LT",
    "RT",
    "LS X-",
    "LS X+",
    "LS Y+",
    "LS Y-",
    "RS X-",
    "RS X+",
    "RS Y+",
    "RS Y-",
];

pub(crate) fn player_key(player: u8) -> Result<String> {
    ensure!(
        (1..=7).contains(&player),
        "RPCS3 supports seven standard player entries"
    );
    Ok(format!("Player {player} Input"))
}

/// Validate single-input standard controls. PS/pressure/limiter actions may be
/// supplied explicitly; do not inherit the native Start+Back PS-button combo.
pub(crate) fn validate_controls(controls: &BTreeMap<String, String>) -> Result<()> {
    let routes = visual_routes();
    ensure!(
        routes.values().all(|name| controls.contains_key(*name)),
        "RPCS3 needs all standard pad controls"
    );
    let mut inputs = std::collections::BTreeSet::new();
    for (key, value) in controls {
        ensure!(
            routes.values().any(|name| *name == key)
                || matches!(
                    key.as_str(),
                    "PS Button" | "Pressure Intensity Button" | "Analog Limiter Button"
                ),
            "Unknown RPCS3 standard pad control"
        );
        ensure!(
            SDL_INPUTS.contains(&value.as_str()) && inputs.insert(value),
            "RPCS3 input is unknown or assigned to multiple controls"
        );
    }
    Ok(())
}
