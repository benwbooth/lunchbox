//! Gearsystem standalone SDL3 controller-profile writer.
//!
//! Pinned source: drhelius/Gearsystem commit
//! 253752954d5237a30b60c40789117ded345bcd48. The desktop frontend reads
//! `config.ini` through mINI and defines the exact `[InputA]` / `[InputB]`
//! fields in `platforms/shared/desktop/config_definitions.inc.h`.
//! `gamepad.cpp` assigns the first two SDL gamepads to player slots at runtime;
//! the config values are SDL logical controls, not physical-device indices.

use anyhow::{Context, Result, ensure};
use std::collections::BTreeMap;

pub(crate) const SOURCE_COMMIT: &str = "253752954d5237a30b60c40789117ded345bcd48";

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum GamepadInput {
    Disabled,
    /// SDL_GamepadButton 0 through 26 in the pinned SDL3 API.
    Button(u8),
    /// The frontend's `GAMEPAD_VBTN_AXIS_BASE + SDL_GamepadAxis` convention.
    Axis(u8),
}

impl GamepadInput {
    fn value(self) -> Result<i16> {
        match self {
            Self::Disabled => Ok(-1),
            Self::Button(button) => {
                ensure!(button <= 26, "Gearsystem SDL3 button is out of range");
                Ok(button as i16)
            }
            Self::Axis(axis) => {
                ensure!(axis <= 5, "Gearsystem SDL3 axis button is out of range");
                Ok(1000 + axis as i16)
            }
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum Directional {
    Dpad,
    Analog {
        x_axis: u8,
        y_axis: u8,
        invert_x: bool,
        invert_y: bool,
    },
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) struct PlayerProfile {
    pub player: u8,
    pub directional: Directional,
    pub button_1: GamepadInput,
    pub button_2: GamepadInput,
    pub start: GamepadInput,
    pub reset: GamepadInput,
}

fn fields(profile: PlayerProfile) -> Result<BTreeMap<String, String>> {
    ensure!(
        (1..=2).contains(&profile.player),
        "Gearsystem player must be 1 or 2"
    );
    let mut fields = BTreeMap::from([
        ("AllowUpDown".into(), "false".into()),
        ("Gamepad".into(), "true".into()),
        ("Gamepad1".into(), profile.button_1.value()?.to_string()),
        ("Gamepad2".into(), profile.button_2.value()?.to_string()),
        ("GamepadReset".into(), profile.reset.value()?.to_string()),
        ("GamepadStart".into(), profile.start.value()?.to_string()),
    ]);
    match profile.directional {
        Directional::Dpad => {
            fields.insert("GamepadDirectional".into(), "0".into());
            fields.insert("GamepadInvertX".into(), "false".into());
            fields.insert("GamepadInvertY".into(), "false".into());
            fields.insert("GamepadX".into(), "0".into());
            fields.insert("GamepadY".into(), "1".into());
        }
        Directional::Analog {
            x_axis,
            y_axis,
            invert_x,
            invert_y,
        } => {
            ensure!(
                x_axis <= 5 && y_axis <= 5,
                "Gearsystem SDL3 axis is out of range"
            );
            ensure!(
                x_axis != y_axis,
                "Gearsystem directional axes must be distinct"
            );
            fields.insert("GamepadDirectional".into(), "1".into());
            fields.insert("GamepadInvertX".into(), invert_x.to_string());
            fields.insert("GamepadInvertY".into(), invert_y.to_string());
            fields.insert("GamepadX".into(), x_axis.to_string());
            fields.insert("GamepadY".into(), y_axis.to_string());
        }
    }
    Ok(fields)
}

/// Patch the source-defined input sections in a copied `config.ini`. Device
/// order is deliberately not serialized: the launch layer must probe SDL3,
/// constrain/verify the first two gamepads, and recheck the selected physical
/// identities before starting Gearsystem.
pub(crate) fn patch_config(baseline: &[u8], profiles: &[PlayerProfile]) -> Result<String> {
    ensure!(
        baseline.len() <= 1024 * 1024,
        "Gearsystem config is too large"
    );
    ensure!(
        !profiles.is_empty() && profiles.len() <= 2,
        "Gearsystem profile count is invalid"
    );
    let mut seen = [false; 2];
    let original = std::str::from_utf8(baseline).context("Gearsystem config is not UTF-8")?;
    let mut result = original.to_owned();
    for profile in profiles {
        ensure!(
            (1..=2).contains(&profile.player),
            "Gearsystem player must be 1 or 2"
        );
        ensure!(
            !seen[(profile.player - 1) as usize],
            "Gearsystem player is duplicated"
        );
        seen[(profile.player - 1) as usize] = true;
        result = patch_section(
            &result,
            if profile.player == 1 {
                "InputA"
            } else {
                "InputB"
            },
            &fields(*profile)?,
        )?;
    }
    Ok(result)
}

fn patch_section(
    original: &str,
    section: &str,
    fields: &BTreeMap<String, String>,
) -> Result<String> {
    ensure!(
        !original.contains('\0'),
        "Gearsystem config contains a NUL byte"
    );
    let (bom, text) = original
        .strip_prefix('\u{feff}')
        .map_or(("", original), |text| ("\u{feff}", text));
    let newline = if text.contains("\r\n") { "\r\n" } else { "\n" };
    let mut result = bom.to_owned();
    let mut active = false;
    let mut inserted = false;
    for line in text.split_inclusive('\n') {
        if let Some(header) = line
            .trim_start()
            .strip_prefix('[')
            .and_then(|line| line.split_once(']').map(|(name, _)| name.trim()))
        {
            active = header == section;
            result.push_str(line);
            if active && !inserted {
                if !result.ends_with('\n') {
                    result.push_str(newline);
                }
                for (key, value) in fields {
                    result.push_str(&format!("{key}={value}{newline}"));
                }
                inserted = true;
            }
        } else {
            let owned = active
                && line
                    .split_once('=')
                    .is_some_and(|(key, _)| fields.keys().any(|known| key.trim() == known));
            if !owned {
                result.push_str(line);
            }
        }
    }
    if !inserted {
        if !result.is_empty() && !result.ends_with('\n') {
            result.push_str(newline);
        }
        result.push_str(&format!("[{section}]{newline}"));
        for (key, value) in fields {
            result.push_str(&format!("{key}={value}{newline}"));
        }
    }
    Ok(result)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn patches_logical_sdl_controls_without_device_index() {
        let output = patch_config(
            b"[Video]\nScale=3\n[InputA]\nGamepad=false\nKeyLeft=80\nGamepad1=9\n",
            &[PlayerProfile {
                player: 1,
                directional: Directional::Analog {
                    x_axis: 2,
                    y_axis: 3,
                    invert_x: false,
                    invert_y: true,
                },
                button_1: GamepadInput::Button(0),
                button_2: GamepadInput::Button(1),
                start: GamepadInput::Button(6),
                reset: GamepadInput::Disabled,
            }],
        )
        .unwrap();
        assert!(output.contains("[InputA]\nAllowUpDown=false\nGamepad=true\n"));
        assert!(output.contains("GamepadDirectional=1\n"));
        assert!(output.contains("GamepadX=2\nGamepadY=3\n"));
        assert!(output.contains("KeyLeft=80\n"));
        assert!(output.contains("[Video]\nScale=3\n"));
        assert!(!output.contains("Gamepad=false"));
    }

    #[test]
    fn validates_players_and_sdl_ranges() {
        let profile = PlayerProfile {
            player: 0,
            directional: Directional::Dpad,
            button_1: GamepadInput::Button(27),
            button_2: GamepadInput::Button(1),
            start: GamepadInput::Axis(6),
            reset: GamepadInput::Disabled,
        };
        assert!(patch_config(b"", &[profile]).is_err());
    }
}
