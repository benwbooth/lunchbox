//! LRPS2 PAD contract at de0da87d20617d711aed1cbca975f8a51c998d36.
//! This guards the implemented mode, not standalone PCSX2 or USB peripherals.
use crate::controller_catalog::{EmulatorProfile, Layout};
use anyhow::{Context, Result, ensure};

pub(crate) fn validate_profile(profile: &EmulatorProfile, target: &Layout) -> Result<()> {
    let launch = profile
        .retroarch_launch
        .as_ref()
        .context("LRPS2 requires a launch contract")?;
    ensure!(
        profile.core == "pcsx2"
            && profile.id == "retroarch:pcsx2:dualshock2-pressure"
            && profile.target_layout == "dualshock2-pressure"
            && target.id == profile.target_layout
            && profile.transport == "retropad"
            && profile.retroarch_library.as_deref() == Some("LRPS2")
            && profile.explicit_selection
            && profile.requires_fresh_start,
        "LRPS2 requires its explicit fresh-start measured-pressure contract"
    );
    ensure!(
        launch.device == 1
            && launch.max_players == 2
            && launch.player_topology.is_none()
            && profile.frontend_port_count() == 2
            && profile.port_devices.is_empty()
            && profile.port_controls.is_empty()
            && profile.port_layouts.is_empty()
            && profile.port_bindings.is_empty(),
        "LRPS2 polls two DualShock 2 ports; USB and multitap routing is not implemented"
    );
    for port in 1..=2 {
        for (key, expected) in [
            ("axis_scale", "100%"),
            ("axis_deadzone", "0%"),
            ("button_deadzone", "0%"),
            ("invert_left_stick", "disabled"),
            ("invert_right_stick", "disabled"),
            ("analog_mode", "enabled"),
            ("enable_rumble", "disabled"),
        ] {
            let key = format!("pcsx2_{key}{port}");
            ensure!(
                profile.core_options.get(&key).map(String::as_str) == Some(expected),
                "LRPS2 pressure contract requires {key} = {expected}"
            );
        }
    }
    // Semantic ID, native RetroPad output, ergonomic group, analog, pressure.
    let expected = [
        ("up", "DPadUp", "dpad", true, true),
        ("down", "DPadDown", "dpad", true, true),
        ("left", "DPadLeft", "dpad", true, true),
        ("right", "DPadRight", "dpad", true, true),
        ("b", "South", "face", true, true),
        ("a", "East", "face", true, true),
        ("y", "West", "face", true, true),
        ("x", "North", "face", true, true),
        ("l", "LeftBumper", "shoulder", true, true),
        ("r", "RightBumper", "shoulder", true, true),
        ("l2", "LeftTrigger", "shoulder", true, true),
        ("r2", "RightTrigger", "shoulder", true, true),
        ("select", "Select", "menu", false, false),
        ("start", "Start", "menu", false, false),
        ("l3", "LeftStick", "stick", false, false),
        ("r3", "RightStick", "stick", false, false),
        ("stick_up", "LeftStickUp", "stick", true, false),
        ("stick_down", "LeftStickDown", "stick", true, false),
        ("stick_left", "LeftStickLeft", "stick", true, false),
        ("stick_right", "LeftStickRight", "stick", true, false),
        ("right_stick_up", "RightStickUp", "stick", true, false),
        ("right_stick_down", "RightStickDown", "stick", true, false),
        ("right_stick_left", "RightStickLeft", "stick", true, false),
        ("right_stick_right", "RightStickRight", "stick", true, false),
    ];
    ensure!(
        target.controls.len() == expected.len() && profile.bindings.len() == expected.len(),
        "LRPS2 requires the complete pressure, digital and stick control set"
    );
    for (id, output, group, analog, pressure) in expected {
        let control = target
            .controls
            .iter()
            .find(|control| control.id == id)
            .with_context(|| format!("Missing LRPS2 control {id}"))?;
        ensure!(
            profile.bindings.get(id).map(String::as_str) == Some(output)
                && control.group == group
                && control.analog == analog
                && control.is_pressure() == pressure
                && !control.optional
                && control.repeat_of.is_none(),
            "LRPS2 control {id} does not match its required native signal"
        );
    }
    Ok(())
}
