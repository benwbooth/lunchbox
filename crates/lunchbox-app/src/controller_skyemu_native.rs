//! SkyEmu standalone-native SDL controller settings.
//!
//! Pinned to skylersaleh/SkyEmu commit 01516d6798e3652b583e6a366085bb51c43b528d.
//! The native SDL frontend stores each selected controller's key and analog
//! maps as a raw host-endian `int32_t[64 * 2]` file (little-endian on the
//! supported desktop hosts) named
//! `<SDL_GetPrefPath("Sky", "SkyEmu")><controller-name>-bindings.bin`.

use anyhow::{Result, ensure};
use std::collections::BTreeMap;

pub(crate) const SOURCE_COMMIT: &str = "01516d6798e3652b583e6a366085bb51c43b528d";
pub(crate) const PROFILE_ID: &str = "skyemu:standalone-native-sdl-controller";
pub(crate) const UPSTREAM_URL: &str = "https://github.com/skylersaleh/SkyEmu";
pub(crate) const SDL_PREF_ORGANIZATION: &str = "Sky";
pub(crate) const SDL_PREF_APPLICATION: &str = "SkyEmu";
pub(crate) const BINDING_SLOTS: usize = 64;

/// The controller key-map indices from `sb_types.h`.
pub(crate) const KEY_CONTROLS: [(&str, usize); 36] = [
    ("a", 0),
    ("b", 1),
    ("x", 2),
    ("y", 3),
    ("up", 4),
    ("down", 5),
    ("left", 6),
    ("right", 7),
    ("l", 8),
    ("r", 9),
    ("start", 10),
    ("select", 11),
    ("fold_screen", 12),
    ("pen_down", 13),
    ("pause", 14),
    ("rewind", 15),
    ("fast_forward_2x", 16),
    ("fast_forward_max", 17),
    ("capture_state_0", 18),
    ("restore_state_0", 19),
    ("capture_state_1", 20),
    ("restore_state_1", 21),
    ("capture_state_2", 22),
    ("restore_state_2", 23),
    ("capture_state_3", 24),
    ("restore_state_3", 25),
    ("reset_game", 26),
    ("turbo_a", 27),
    ("turbo_b", 28),
    ("turbo_x", 29),
    ("turbo_y", 30),
    ("turbo_l", 31),
    ("turbo_r", 32),
    ("solar_sensor_plus", 33),
    ("solar_sensor_minus", 34),
    ("toggle_fullscreen", 35),
];

/// The analog-map indices from `main.c`.
pub(crate) const ANALOG_CONTROLS: [(&str, usize); 4] =
    [("up_down", 0), ("left_right", 1), ("l", 2), ("r", 3)];

/// A raw SDL controller binding as encoded by `se_get_sdl_key_bind`.
#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub(crate) enum Binding {
    Button(u8),
    Axis { index: u8, negative: bool },
    Hat { index: u8, direction: HatDirection },
}

#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub(crate) enum HatDirection {
    Up = 1,
    Down = 4,
    Left = 8,
    Right = 2,
}

impl Binding {
    fn encoded(self) -> i32 {
        match self {
            Self::Button(index) => i32::from(index),
            Self::Axis { index, negative } => {
                i32::from(index) | if negative { 1 << 18 } else { 1 << 17 }
            }
            Self::Hat { index, direction } => {
                (1 << 16) | (i32::from(index) << 8) | direction as i32
            }
        }
    }
}

fn valid_controller_name(name: &str) -> Result<()> {
    ensure!(
        !name.is_empty() && name.len() <= 127,
        "SkyEmu controller name is invalid"
    );
    ensure!(
        !name.chars().any(|ch| {
            ch == '/' || ch == '\\' || ch == '\0' || ch == '\n' || ch == '\r' || ch.is_control()
        }),
        "SkyEmu controller name contains a path separator or control character"
    );
    Ok(())
}

/// Return the exact per-controller filename SkyEmu derives from SDL's
/// preference directory. `pref_path` must be the SDL-returned path, including
/// its trailing separator.
pub(crate) fn binding_path(pref_path: &str, controller_name: &str) -> Result<String> {
    valid_controller_name(controller_name)?;
    ensure!(
        !pref_path.contains('\0'),
        "SkyEmu preference path contains NUL"
    );
    ensure!(
        pref_path.ends_with('/') || pref_path.ends_with('\\'),
        "SkyEmu SDL preference path must include its trailing separator"
    );
    Ok(format!("{pref_path}{controller_name}-bindings.bin"))
}

/// Render the exact 512-byte key/analog binding file. Supported desktop hosts
/// are little-endian, matching the source's raw `int32_t` write. Unspecified
/// slots retain SkyEmu's `-1` sentinel; unrelated user settings and save-state
/// files are not touched by this mapping-only writer.
pub(crate) fn controller_bindings(
    controller_name: &str,
    key_bindings: &BTreeMap<String, Binding>,
    analog_bindings: &BTreeMap<String, u8>,
) -> Result<Vec<u8>> {
    valid_controller_name(controller_name)?;
    ensure!(
        key_bindings
            .keys()
            .all(|key| KEY_CONTROLS.iter().any(|(name, _)| name == key)),
        "SkyEmu key binding name is outside the source contract"
    );
    ensure!(
        analog_bindings
            .keys()
            .all(|key| ANALOG_CONTROLS.iter().any(|(name, _)| name == key)),
        "SkyEmu analog binding name is outside the source contract"
    );
    ensure!(
        !key_bindings.is_empty() || !analog_bindings.is_empty(),
        "SkyEmu controller binding map is empty"
    );

    let mut slots = [-1_i32; BINDING_SLOTS * 2];
    for (name, binding) in key_bindings {
        let index = KEY_CONTROLS
            .iter()
            .find(|(known, _)| *known == name)
            .map(|(_, index)| *index)
            .expect("validated SkyEmu key binding name");
        slots[index] = binding.encoded();
    }
    for (name, axis) in analog_bindings {
        let index = ANALOG_CONTROLS
            .iter()
            .find(|(known, _)| *known == name)
            .map(|(_, index)| *index)
            .expect("validated SkyEmu analog binding name");
        slots[BINDING_SLOTS + index] = i32::from(*axis);
    }

    let mut output = Vec::with_capacity(slots.len() * 4);
    for value in slots {
        output.extend_from_slice(&value.to_le_bytes());
    }
    Ok(output)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn emits_source_binary_layout_and_sentinels() {
        let keys = BTreeMap::from([
            ("a".into(), Binding::Button(0)),
            (
                "up".into(),
                Binding::Hat {
                    index: 0,
                    direction: HatDirection::Up,
                },
            ),
        ]);
        let analog = BTreeMap::from([("left_right".into(), 1_u8)]);
        let bytes = controller_bindings("Pad One", &keys, &analog).unwrap();
        assert_eq!(bytes.len(), 512);
        assert_eq!(&bytes[0..4], &[0, 0, 0, 0]);
        assert_eq!(&bytes[16..20], &[1, 0, 1, 0]);
        assert_eq!(&bytes[64 * 4 + 4..64 * 4 + 8], &[1, 0, 0, 0]);
        assert_eq!(&bytes[12..16], &[255, 255, 255, 255]);
        assert_eq!(
            binding_path("/tmp/SkyEmu/", "Pad One").unwrap(),
            "/tmp/SkyEmu/Pad One-bindings.bin"
        );
        assert!(binding_path("/tmp/SkyEmu", "Pad One").is_err());
        assert_eq!(SOURCE_COMMIT.len(), 40);
        assert_eq!(UPSTREAM_URL, "https://github.com/skylersaleh/SkyEmu");
    }

    #[test]
    fn encodes_axis_and_hat_masks() {
        let keys = BTreeMap::from([
            (
                "a".into(),
                Binding::Axis {
                    index: 2,
                    negative: true,
                },
            ),
            (
                "b".into(),
                Binding::Hat {
                    index: 1,
                    direction: HatDirection::Right,
                },
            ),
        ]);
        let bytes = controller_bindings("Pad", &keys, &BTreeMap::new()).unwrap();
        assert_eq!(i32::from_le_bytes(bytes[0..4].try_into().unwrap()), 0x40002);
        assert_eq!(i32::from_le_bytes(bytes[4..8].try_into().unwrap()), 0x10102);
    }

    #[test]
    fn invalid_names_and_unknown_bindings_fail_closed() {
        let keys = BTreeMap::from([("a".into(), Binding::Button(0))]);
        assert!(binding_path("/tmp/", "../Pad").is_err());
        assert!(
            controller_bindings(
                "Pad",
                &BTreeMap::from([("unknown".into(), Binding::Button(0))]),
                &BTreeMap::new()
            )
            .is_err()
        );
        assert!(controller_bindings("Pad", &keys, &BTreeMap::new()).is_ok());
    }
}
