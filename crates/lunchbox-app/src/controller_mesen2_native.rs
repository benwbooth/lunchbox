//! Mesen2 native `settings.json` NES controller mappings for the Linux
//! frontend, not libretro bindings.
//!
//! Functional contract pinned to SourMesen/Mesen2
//! b9fa69ddc6d0a331fb103fdb5eef6904305703c2:
//! - `Core/Shared/Interfaces/IKeyManager.h` — `BaseGamepadIndex = 0x1000`;
//!   `Linux/LinuxKeyManager.cpp` builds each KeyMapping UInt16 as
//!   `0x1000 + pad*0x100 + buttonIndex` with up to twenty pads.
//! - `Linux/LinuxGameController.cpp` — buttonIndex resolves straight to
//!   kernel codes: 0..13 are BTN_A/B/C/X/Y/Z, BTN_TL/TR/TL2/TR2,
//!   BTN_SELECT/START, BTN_THUMBL/THUMBR; 14..25 are the positive/negative
//!   halves of ABS_X/Y/Z and ABS_RX/RY/RZ; 26..29 are ABS_HAT0X/Y
//!   directions (BTN_DPAD_* fold into the same indices).
//! - `Linux/LinuxKeyManager.cpp CheckForGamepads` — pads register from
//!   `/dev/input/event*` in directory-iteration order, so this contract
//!   requires the selected controller to be the single qualifying device
//!   (`EV_KEY+BTN_GAMEPAD` or `EV_ABS+ABS_X`, per
//!   `LinuxGameController::GetController`) and uses pad slot 0.
//! - `Linux/LinuxGameController.cpp CheckAxis` — digital axis halves engage
//!   beyond 40% of the (default→max)/(min→default) range at the default
//!   dead-zone ratio 1.0 (`EmuSettings::GetControllerDeadzoneRatio`,
//!   ControllerDeadzoneSize 2).
//! - `UI/Utilities/JsonHelper.cs` — settings serialize with PascalCase
//!   properties, indented output and string enums; `UI/Config/InputConfig.cs`
//!   holds one UInt16 per control; `Core/Shared/SettingTypes.h` names the NES
//!   standard controller `NesController` and `ControllerType.None` for unset
//!   ports; `$XDG_DATA_HOME/Mesen2/settings.json` (ApplicationData) is the
//!   configuration file selected by an isolated XDG_DATA_HOME.
use anyhow::{Result, ensure};

#[cfg(target_os = "linux")]
pub(crate) mod native_command;
#[cfg(target_os = "linux")]
pub(crate) mod session;
pub(crate) mod settings;

/// Digital axis engagement: 40% of the measured range, plus margin.
pub(crate) const RANGE_FRACTION: f64 = 0.40;

/// NES target controls: layout id -> KeyMapping field name.
pub(crate) const CONTROLS: [(&str, &str); 8] = [
    ("a", "A"),
    ("b", "B"),
    ("select", "Select"),
    ("start", "Start"),
    ("up", "Up"),
    ("down", "Down"),
    ("left", "Left"),
    ("right", "Right"),
];

/// One native KeyMapping UInt16.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct Binding(pub(crate) u16);

impl Binding {
    /// Build from a Mesen buttonIndex on the pinned pad slot.
    pub(crate) fn button_index(pad: u32, index: u32) -> Result<Self> {
        ensure!(pad < 20, "Mesen2 pads address twenty slots");
        ensure!(index < 0x100, "Mesen2 button index overflows its byte");
        Ok(Self((0x1000 + pad * 0x100 + index) as u16))
    }

    /// Kernel KEY_* code -> buttonIndex (LinuxGameController.cpp table).
    pub(crate) fn from_key(code: u32) -> Option<Self> {
        let index = match code {
            0x130..=0x13b => code - 0x130,
            0x13d => 12,
            0x13e => 13,
            0x223 => 26,
            0x222 => 27,
            0x221 => 28,
            0x220 => 29,
            _ => return None,
        };
        Some(Self((0x1000 + index) as u16))
    }

    /// Kernel ABS_* code and direction -> buttonIndex. LinuxGameController
    /// assigns the lower index to the positive half (`CheckAxis(code, true)`).
    pub(crate) fn from_axis(code: u32, positive: bool) -> Option<Self> {
        let base = match code {
            0x00 => 14,
            0x01 => 16,
            0x02 => 18,
            0x03 => 20,
            0x04 => 22,
            0x05 => 24,
            0x10 => 26,
            0x11 => 28,
            _ => return None,
        };
        Some(Self((0x1000 + base + u32::from(!positive)) as u16))
    }
}

/// Render the private settings.json. Only the two NES ports are expressed;
/// every other value keeps Mesen2's defaults.
pub(crate) fn settings_json(mapping: &[(String, Binding)]) -> Result<String> {
    use std::fmt::Write;
    ensure!(
        mapping.len() == CONTROLS.len()
            && CONTROLS
                .iter()
                .all(|(_, field)| mapping.iter().any(|(name, _)| name == field)),
        "Mesen2 needs every standard NES control"
    );
    let mut result = String::from(
        "{\n  \"Nes\": {\n    \"Port1\": {\n      \"Type\": \"NesController\",\n      \"Mapping1\": {\n",
    );
    for (index, (control, field)) in CONTROLS.iter().enumerate() {
        let code = mapping
            .iter()
            .find(|(name, _)| name == *field)
            .map(|(_, binding)| binding.0)
            .unwrap_or_default();
        let comma = if index + 1 < CONTROLS.len() { "," } else { "" };
        let control_comment = control;
        let _ = control_comment;
        writeln!(result, "        \"{field}\": {code}{comma}").expect("in-memory write");
    }
    result.push_str("      }\n    },\n    \"Port2\": {\n      \"Type\": \"None\"\n    }\n  }\n}\n");
    Ok(result)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn key_codes_map_through_the_pinned_table() {
        assert_eq!(Binding::from_key(0x130).unwrap().0, 0x1000); // BTN_A
        assert_eq!(Binding::from_key(0x131).unwrap().0, 0x1001); // BTN_B
        assert_eq!(Binding::from_key(0x13a).unwrap().0, 0x100a); // BTN_SELECT
        assert_eq!(Binding::from_key(0x13b).unwrap().0, 0x100b); // BTN_START
        assert_eq!(Binding::from_key(0x13d).unwrap().0, 0x100c); // BTN_THUMBL
        assert_eq!(Binding::from_key(0x13e).unwrap().0, 0x100d); // BTN_THUMBR
        assert_eq!(Binding::from_key(0x223).unwrap().0, 0x101a); // DPAD right
        assert_eq!(Binding::from_key(0x222).unwrap().0, 0x101b); // left
        assert_eq!(Binding::from_key(0x221).unwrap().0, 0x101c); // down
        assert_eq!(Binding::from_key(0x220).unwrap().0, 0x101d); // up
        assert!(Binding::from_key(0x13c).is_none()); // BTN_MODE is unhandled
        assert!(Binding::from_key(0x101).is_none());
    }

    #[test]
    fn axis_codes_map_to_half_indices() {
        assert_eq!(Binding::from_axis(0x00, true).unwrap().0, 0x100e);
        assert_eq!(Binding::from_axis(0x00, false).unwrap().0, 0x100f);
        assert_eq!(Binding::from_axis(0x01, true).unwrap().0, 0x1010);
        assert_eq!(Binding::from_axis(0x01, false).unwrap().0, 0x1011);
        assert_eq!(Binding::from_axis(0x03, false).unwrap().0, 0x1015);
        assert_eq!(Binding::from_axis(0x05, true).unwrap().0, 0x1018);
        assert_eq!(Binding::from_axis(0x10, true).unwrap().0, 0x101a);
        assert_eq!(Binding::from_axis(0x11, false).unwrap().0, 0x101d);
        assert!(Binding::from_axis(0x06, true).is_none());
        assert!(Binding::from_axis(0x12, true).is_none());
    }

    #[test]
    fn pad_slots_stride_by_0x100() {
        assert_eq!(Binding::button_index(0, 5).unwrap().0, 0x1005);
        assert_eq!(Binding::button_index(2, 1).unwrap().0, 0x1201);
        assert!(Binding::button_index(20, 0).is_err());
        assert!(Binding::button_index(0, 0x100).is_err());
    }

    #[test]
    fn settings_json_uses_the_verified_shape() {
        let mapping = CONTROLS
            .iter()
            .enumerate()
            .map(|(i, (_, field))| {
                (
                    (*field).to_owned(),
                    Binding::button_index(0, i as u32 + 2).unwrap(),
                )
            })
            .collect::<Vec<_>>();
        let text = settings_json(&mapping).unwrap();
        assert!(text.starts_with("{\n  \"Nes\": {\n    \"Port1\": {\n      \"Type\": \"NesController\",\n      \"Mapping1\": {\n"));
        assert!(text.contains("        \"A\": 4098,\n"));
        assert!(text.contains("        \"Start\": 4101,\n"));
        assert!(text.ends_with(
            "      }\n    },\n    \"Port2\": {\n      \"Type\": \"None\"\n    }\n  }\n}\n"
        ));
        assert!(settings_json(&mapping[..3].to_vec()).is_err());
    }
}
