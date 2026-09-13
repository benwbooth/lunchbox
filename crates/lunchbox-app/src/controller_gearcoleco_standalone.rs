//! Gearcoleco standalone SDL3 controller-profile writer.
//!
//! Pinned source: drhelius/Gearcoleco commit
//! 8ad5f92c45e7ca616535a057495557c2352a9115. The desktop frontend declares
//! the complete `[InputA]` / `[InputB]` grammar in
//! `platforms/shared/desktop/config_definitions.inc.h`; `events.cpp` consumes
//! those values as SDL logical buttons/axes. `gamepad.cpp` assigns the first
//! two SDL gamepads to player slots at runtime and never reads a persisted
//! physical-device index.

use anyhow::{Context, Result, ensure};
use std::collections::BTreeMap;

pub(crate) const SOURCE_COMMIT: &str = "8ad5f92c45e7ca616535a057495557c2352a9115";

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
                ensure!(button <= 26, "Gearcoleco SDL3 button is out of range");
                Ok(button as i16)
            }
            Self::Axis(axis) => {
                ensure!(axis <= 5, "Gearcoleco SDL3 axis button is out of range");
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

/// Button order mirrors `events.cpp`: left fire, right fire, blue, purple,
/// keypad 1..9, keypad 0, asterisk, hash.
pub(crate) const BUTTON_FIELDS: [&str; 16] = [
    "GamepadLeft",
    "GamepadRight",
    "GamepadBlue",
    "GamepadPurple",
    "Gamepad1",
    "Gamepad2",
    "Gamepad3",
    "Gamepad4",
    "Gamepad5",
    "Gamepad6",
    "Gamepad7",
    "Gamepad8",
    "Gamepad9",
    "Gamepad0",
    "GamepadAsterisk",
    "GamepadHash",
];

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) struct PlayerProfile {
    pub player: u8,
    pub directional: Directional,
    pub buttons: [GamepadInput; 16],
}

fn fields(profile: PlayerProfile) -> Result<BTreeMap<String, String>> {
    ensure!(
        (1..=2).contains(&profile.player),
        "Gearcoleco player must be 1 or 2"
    );
    let mut fields = BTreeMap::from([
        ("AllowUpDown".into(), "false".into()),
        ("Gamepad".into(), "true".into()),
    ]);
    for (name, binding) in BUTTON_FIELDS.iter().zip(profile.buttons) {
        fields.insert((*name).into(), binding.value()?.to_string());
    }
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
                "Gearcoleco SDL3 axis is out of range"
            );
            ensure!(
                x_axis != y_axis,
                "Gearcoleco directional axes must be distinct"
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

/// Patch the complete source-defined gamepad surface in copied `config.ini`
/// input sections while preserving keyboard, BIOS, media, save, state and
/// display settings. The launch layer must still probe SDL3 and verify that
/// the selected physical devices are the first two gamepads before launch.
pub(crate) fn patch_config(baseline: &[u8], profiles: &[PlayerProfile]) -> Result<String> {
    ensure!(
        baseline.len() <= 1024 * 1024,
        "Gearcoleco config is too large"
    );
    ensure!(
        !profiles.is_empty() && profiles.len() <= 2,
        "Gearcoleco profile count is invalid"
    );
    let mut seen = [false; 2];
    let original = std::str::from_utf8(baseline).context("Gearcoleco config is not UTF-8")?;
    let mut result = original.to_owned();
    for profile in profiles {
        ensure!(
            (1..=2).contains(&profile.player),
            "Gearcoleco player must be 1 or 2"
        );
        ensure!(
            !seen[(profile.player - 1) as usize],
            "Gearcoleco player is duplicated"
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
        "Gearcoleco config contains a NUL byte"
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
    fn writes_all_keypad_and_fire_bindings() {
        let mut buttons = [GamepadInput::Disabled; 16];
        for (index, binding) in buttons.iter_mut().enumerate() {
            *binding = GamepadInput::Button((index % 15) as u8);
        }
        buttons[2] = GamepadInput::Axis(4);
        let output = patch_config(
            b"[Emulator]\nSaveSlot=2\n[InputA]\nGamepad=false\nKeyLeft=80\n",
            &[PlayerProfile {
                player: 1,
                directional: Directional::Dpad,
                buttons,
            }],
        )
        .unwrap();
        assert!(output.contains("[InputA]\nAllowUpDown=false\nGamepad=true\n"));
        assert!(output.contains("GamepadBlue=1004\n"));
        assert!(output.contains("GamepadAsterisk=14\nGamepadBlue=1004\n"));
        assert!(output.contains("GamepadHash=0\n"));
        assert!(output.contains("KeyLeft=80\n"));
        assert!(output.contains("[Emulator]\nSaveSlot=2\n"));
    }

    #[test]
    fn rejects_invalid_player_and_logical_control() {
        let profile = PlayerProfile {
            player: 3,
            directional: Directional::Analog {
                x_axis: 0,
                y_axis: 0,
                invert_x: false,
                invert_y: false,
            },
            buttons: [GamepadInput::Button(27); 16],
        };
        assert!(patch_config(b"", &[profile]).is_err());
    }
}
