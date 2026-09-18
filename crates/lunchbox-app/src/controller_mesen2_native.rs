//! Mesen2 native `settings.json` NES and PC Engine controller mappings for the
//! Linux frontend, not libretro bindings.
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
//!   standard controller `NesController`, the PC Engine standard controller
//!   `PceController` and `ControllerType.None` for unset
//!   ports; `$XDG_DATA_HOME/Mesen2/settings.json` (ApplicationData) is the
//!   configuration file selected by an isolated XDG_DATA_HOME.
//! - `Core/PCE/Input/PceController.h` — the PCE pad reads I from
//!   `KeyMapping.A`, II from `KeyMapping.B`, Run from `KeyMapping.Start`,
//!   Select from `KeyMapping.Select` and the dpad from the matching
//!   directions; `UI/Config/Configuration.cs` serializes the PCE section as
//!   `PcEngine` with `Port1`/`Port2` slots, mirroring the `Nes` section.
use anyhow::{Result, ensure};

#[cfg(target_os = "linux")]
pub(crate) mod native_command;
// Linux-only by source contract: the session verifies through evdev
// (`--evdev-catalog` captures) and needs the controller's event node.
// Other hosts have no evdev nodes, so no port is staged.
#[cfg(target_os = "linux")]
pub(crate) mod guided;
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

/// PC Engine target controls: layout id -> KeyMapping field name. The PCE
/// pad reads the same KeyMapping fields as NES (A=I, B=II, Start=Run).
pub(crate) const PCE_CONTROLS: [(&str, &str); 8] = [
    ("a", "A"),
    ("b", "B"),
    ("select", "Select"),
    ("start", "Start"),
    ("up", "Up"),
    ("down", "Down"),
    ("left", "Left"),
    ("right", "Right"),
];

/// System key selecting the settings section and controller type.
pub(crate) const SYSTEM_NES: &str = "nes";
pub(crate) const SYSTEM_PCE: &str = "pce";

/// One native KeyMapping UInt16.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Ord, PartialOrd)]
pub(crate) struct Binding(pub(crate) u16);

impl Binding {
    /// Build from a Mesen buttonIndex on the pinned pad slot.
    pub(crate) fn button_index(pad: u32, index: u32) -> Result<Self> {
        ensure!(pad < 20, "Mesen2 pads address twenty slots");
        ensure!(index < 0x100, "Mesen2 button index overflows its byte");
        Ok(Self((0x1000 + pad * 0x100 + index) as u16))
    }

    /// Kernel KEY_* code -> buttonIndex (LinuxGameController.cpp table) on the
    /// slot Mesen2 assigns to this pad.
    pub(crate) fn from_key(pad: u32, code: u32) -> Option<Self> {
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
        Self::button_index(pad, index).ok()
    }

    /// Kernel ABS_* code and direction -> buttonIndex on the pad's slot.
    /// LinuxGameController assigns the lower index to the positive half
    /// (`CheckAxis(code, true)`).
    pub(crate) fn from_axis(pad: u32, code: u32, positive: bool) -> Option<Self> {
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
        Self::button_index(pad, base + u32::from(!positive)).ok()
    }
}

/// Patch the target system's controller contract into a copy of the user's
/// own `settings.json`. Mesen2 deserializes the file as a whole, so every
/// unrelated section (video, audio, other systems' pads, first-run state) is
/// preserved verbatim; only this system's two ports are authored. Port two is
/// forced to `None` so no stale expansion device (Four Score, TurboTap,
/// keyboard) competes for the mapping, and the second port is not inferred
/// from the number of connected pads.
pub(crate) fn patch_settings(
    source: &serde_json::Value,
    system: &str,
) -> Result<(
    &'static str,
    &'static str,
    &'static [(&'static str, &'static str); 8],
)> {
    let _ = source;
    let (section, controller, controls) = match system {
        SYSTEM_PCE => ("PcEngine", "PceController", &PCE_CONTROLS),
        SYSTEM_NES => ("Nes", "NesController", &CONTROLS),
        other => anyhow::bail!("Mesen2 has no settings section for system {other}"),
    };
    Ok((section, controller, controls))
}

/// Codes Mesen2 stores for one mapping, keyed by KeyMapping field name.
pub(crate) fn mapping_codes(
    mapping: &[(String, Binding)],
    controls: &[(&'static str, &'static str); 8],
    missing: &str,
) -> Result<std::collections::BTreeMap<&'static str, u16>> {
    ensure!(
        mapping.len() == controls.len()
            && controls
                .iter()
                .all(|(_, field)| mapping.iter().any(|(name, _)| name == field)),
        "{missing}"
    );
    let mut codes = std::collections::BTreeMap::new();
    let mut used = std::collections::BTreeSet::new();
    for (_, field) in controls.iter() {
        let code = mapping
            .iter()
            .find(|(name, _)| name == field)
            .map(|(_, binding)| binding.0)
            .unwrap_or_default();
        ensure!(
            used.insert(code),
            "Mesen2 physical input has multiple gameplay owners"
        );
        codes.insert(*field, code);
    }
    Ok(codes)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn key_codes_map_through_the_pinned_table() {
        assert_eq!(Binding::from_key(0, 0x130).unwrap().0, 0x1000); // BTN_A
        assert_eq!(Binding::from_key(0, 0x131).unwrap().0, 0x1001); // BTN_B
        assert_eq!(Binding::from_key(0, 0x13a).unwrap().0, 0x100a); // BTN_SELECT
        assert_eq!(Binding::from_key(0, 0x13b).unwrap().0, 0x100b); // BTN_START
        assert_eq!(Binding::from_key(0, 0x13d).unwrap().0, 0x100c); // BTN_THUMBL
        assert_eq!(Binding::from_key(0, 0x13e).unwrap().0, 0x100d); // BTN_THUMBR
        assert_eq!(Binding::from_key(0, 0x223).unwrap().0, 0x101a); // DPAD right
        assert_eq!(Binding::from_key(0, 0x222).unwrap().0, 0x101b); // left
        assert_eq!(Binding::from_key(0, 0x221).unwrap().0, 0x101c); // down
        assert_eq!(Binding::from_key(0, 0x220).unwrap().0, 0x101d); // up
        assert!(Binding::from_key(0, 0x13c).is_none()); // BTN_MODE is unhandled
        assert!(Binding::from_key(0, 0x101).is_none());
    }

    #[test]
    fn axis_codes_map_to_half_indices() {
        assert_eq!(Binding::from_axis(0, 0x00, true).unwrap().0, 0x100e);
        assert_eq!(Binding::from_axis(0, 0x00, false).unwrap().0, 0x100f);
        assert_eq!(Binding::from_axis(0, 0x01, true).unwrap().0, 0x1010);
        assert_eq!(Binding::from_axis(0, 0x01, false).unwrap().0, 0x1011);
        assert_eq!(Binding::from_axis(0, 0x03, false).unwrap().0, 0x1015);
        assert_eq!(Binding::from_axis(0, 0x05, true).unwrap().0, 0x1018);
        assert_eq!(Binding::from_axis(0, 0x10, true).unwrap().0, 0x101a);
        assert_eq!(Binding::from_axis(0, 0x11, false).unwrap().0, 0x101d);
        assert!(Binding::from_axis(0, 0x06, true).is_none());
        assert!(Binding::from_axis(0, 0x12, true).is_none());
    }

    #[test]
    fn pad_slots_stride_by_0x100() {
        assert_eq!(Binding::button_index(0, 5).unwrap().0, 0x1005);
        assert_eq!(Binding::button_index(2, 1).unwrap().0, 0x1201);
        assert!(Binding::button_index(20, 0).is_err());
        assert!(Binding::button_index(0, 0x100).is_err());
    }

    /// A pad Mesen2 registers after another qualifying gamepad keeps its own
    /// slot; the binding must not stay on slot zero.
    #[test]
    fn bindings_follow_the_pad_slot() {
        assert_eq!(Binding::from_key(3, 0x130).unwrap().0, 0x1300);
        assert_eq!(Binding::from_axis(3, 0x00, true).unwrap().0, 0x130e);
        assert_eq!(Binding::from_key(2, 0x220).unwrap().0, 0x121d);
        assert!(Binding::from_key(20, 0x130).is_none());
    }

    #[test]
    fn settings_patch_writes_the_verified_shape_and_keeps_other_sections() {
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
        let (section, controller, controls) =
            patch_settings(&serde_json::json!({}), SYSTEM_NES).unwrap();
        assert_eq!((section, controller), ("Nes", "NesController"));
        let codes = mapping_codes(&mapping, controls, "nes").unwrap();
        assert_eq!(codes["A"], 4098);
        assert_eq!(codes["Start"], 4101);
        assert!(mapping_codes(&mapping[..3].to_vec(), controls, "nes").is_err());

        let (section, controller, controls) =
            patch_settings(&serde_json::json!({}), SYSTEM_PCE).unwrap();
        assert_eq!((section, controller), ("PcEngine", "PceController"));
        let pce = PCE_CONTROLS
            .iter()
            .enumerate()
            .map(|(i, (_, field))| {
                (
                    (*field).to_owned(),
                    Binding::button_index(0, i as u32 + 2).unwrap(),
                )
            })
            .collect::<Vec<_>>();
        assert_eq!(mapping_codes(&pce, controls, "pce").unwrap()["A"], 4098);
        assert!(patch_settings(&serde_json::json!({}), "snes").is_err());
    }

    #[test]
    fn settings_patch_rejects_shared_physical_inputs() {
        let mapping = CONTROLS
            .iter()
            .map(|(_, field)| ((*field).to_owned(), Binding(0x1000)))
            .collect::<Vec<_>>();
        assert!(mapping_codes(&mapping, &CONTROLS, "nes").is_err());
    }
}
