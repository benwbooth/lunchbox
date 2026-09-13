//! Yaba Sanshiro 2 native `yabause.ini` input key grammar (Qt frontend).
//! Evidence: `emulator_details/records/yaba-sanshiro-2.json`, pinned against
//! the devmiyax/yabause v1.20.37 GPL source tarball. Config:
//! `~/.config/YabaSanshiro/qt/yabause.ini` (legacy `~/.yabause/yabause.ini`
//! copied on first run); Windows: portable `<dir-of-yabause.exe>\yabause.ini`
//! else `%APPDATA%\YabaSanshiro\yabause.ini`. All keys live under the
//! `[0.9.11]` group. Per-device identity is
//! `Input/Port/<port>/Id/<id>/Type|Device|DeviceName` and bindings are
//! `Input/Port/<port>/Id/<id>/Controller/<perType>/Key/<padKey>` holding a u32
//! host key code (`bindingKey` in `yabause/src/qt/InputPortConfig.cpp`).
//! Peripheral ids (`PERPAD` etc.) and the standard-pad button table come from
//! `yabause/src/peripheral.h` / `peripheral.c` (`PerPadNames`); unbound keys
//! read back as `PERKEY_UNBOUND`. Session, native_command, and full wiring
//! will follow in a later cycle.

/// QSettings group every Yaba Sanshiro 2 key lives under.
pub(crate) const SETTINGS_GROUP: &str = "0.9.11";

/// Config path relative to the XDG config home on Linux.
pub(crate) const CONFIG_RELATIVE: &str = "YabaSanshiro/qt/yabause.ini";

/// Legacy config path relative to `$HOME`, copied forward on first run.
pub(crate) const LEGACY_CONFIG_RELATIVE: &str = ".yabause/yabause.ini";

/// Config path relative to `%APPDATA%` on Windows (non-portable installs).
pub(crate) const WINDOWS_APPDATA_RELATIVE: &str = "YabaSanshiro/yabause.ini";

/// Peripheral-type ids from `yabause/src/peripheral.h`.
pub(crate) const PER_TYPE_PAD: u32 = 0x02;
pub(crate) const PER_TYPE_WHEEL: u32 = 0x13;
pub(crate) const PER_TYPE_MISSION_STICK: u32 = 0x15;
pub(crate) const PER_TYPE_3D_PAD: u32 = 0x16;
pub(crate) const PER_TYPE_TWIN_STICKS: u32 = 0x19;
pub(crate) const PER_TYPE_GUN: u32 = 0x25;
pub(crate) const PER_TYPE_KEYBOARD: u32 = 0x34;
pub(crate) const PER_TYPE_MOUSE: u32 = 0xE3;

/// Stored key value meaning "this Saturn button has nothing bound to it".
pub(crate) const PERKEY_UNBOUND: u32 = 0xFFFF_FFFF;

/// Standard-pad buttons: `(PERPAD_* index, PerPadNames entry)`, in source order.
pub(crate) const PAD_BUTTONS: [(u8, &str); 13] = [
    (0, "Up"),
    (1, "Right"),
    (2, "Down"),
    (3, "Left"),
    (4, "R"),
    (5, "L"),
    (6, "Start"),
    (7, "A"),
    (8, "B"),
    (9, "C"),
    (10, "X"),
    (11, "Y"),
    (12, "Z"),
];

/// Display name for a standard-pad button index, or `None` outside 0..=12.
pub(crate) fn pad_button_name(pad_key: u8) -> Option<&'static str> {
    PAD_BUTTONS
        .iter()
        .find(|(index, _)| *index == pad_key)
        .map(|(_, name)| *name)
}

/// `saturn-digital` layout target id to its `PERPAD_*` button index.
/// Explicit table: layout identity is never fuzzy-matched.
pub(crate) fn pad_key_for_target(target: &str) -> Option<u8> {
    Some(match target {
        "up" => 0,
        "right" => 1,
        "down" => 2,
        "left" => 3,
        "r" => 4,
        "l" => 5,
        "start" => 6,
        "a" => 7,
        "b" => 8,
        "c" => 9,
        "x" => 10,
        "y" => 11,
        "z" => 12,
        _ => return None,
    })
}

/// Native-target routes for the `saturn-digital` layout: layout target id to
/// the writer output (`PerPadNames` entry with its `Key` index), in pad order.
pub(crate) const SATURN_ROUTES: [(&str, &str); 13] = [
    ("up", "Up (Key 0)"),
    ("right", "Right (Key 1)"),
    ("down", "Down (Key 2)"),
    ("left", "Left (Key 3)"),
    ("r", "R (Key 4)"),
    ("l", "L (Key 5)"),
    ("start", "Start (Key 6)"),
    ("a", "A (Key 7)"),
    ("b", "B (Key 8)"),
    ("c", "C (Key 9)"),
    ("x", "X (Key 10)"),
    ("y", "Y (Key 11)"),
    ("z", "Z (Key 12)"),
];

/// Per-device identity fields under `Input/Port/<port>/Id/<id>`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum IdentityField {
    Type,
    Device,
    DeviceName,
}

impl IdentityField {
    fn as_str(self) -> &'static str {
        match self {
            Self::Type => "Type",
            Self::Device => "Device",
            Self::DeviceName => "DeviceName",
        }
    }
}

/// `Input/Port/<port>/Id/<id>/<Type|Device|DeviceName>`.
pub(crate) fn identity_key(port: u32, id: u32, field: IdentityField) -> String {
    format!("Input/Port/{port}/Id/{id}/{}", field.as_str())
}

/// `Input/Port/<port>/Id/<id>/Controller/<perType>/Key/<padKey>`.
/// All segments are numeric, so the key is always well-formed; use
/// [`pad_button_name`] to validate standard-pad button indices.
pub(crate) fn binding_key(port: u32, id: u32, per_type: u32, pad_key: u8) -> String {
    format!("Input/Port/{port}/Id/{id}/Controller/{per_type}/Key/{pad_key}")
}

/// One standard-pad binding line: the key from [`binding_key`] with
/// `PER_TYPE_PAD`, holding the u32 host code as decimal (QSettings stores
/// the `quint32` from `UIControllerSetting::setPadKey` in decimal).
pub(crate) fn binding_line(port: u32, id: u32, pad_key: u8, code: u32) -> Result<String, ()> {
    if pad_button_name(pad_key).is_none() {
        return Err(());
    }
    Ok(format!(
        "{}={code}",
        binding_key(port, id, PER_TYPE_PAD, pad_key)
    ))
}

/// Minimal `[0.9.11]` settings body carrying one port/id's pad bindings,
/// in caller order. The session stage writes this into a private-HOME
/// `yabause.ini` so the user's own config is never touched.
pub(crate) fn ini_body(port: u32, id: u32, bindings: &[(u8, u32)]) -> Result<String, ()> {
    let mut body = String::from("[0.9.11]\n");
    let mut keys = std::collections::BTreeSet::new();
    for (pad_key, code) in bindings {
        if !keys.insert(*pad_key) {
            return Err(());
        }
        body.push_str(&binding_line(port, id, *pad_key, *code)?);
        body.push('\n');
    }
    Ok(body)
}

/// Host key-code layout from `yabause/src/persdlcodes.h`: bits 0-15 payload,
/// 16-17 sub-type, 18-19 device, 20 raw axis, 21 raw hat, 22 game controller.
pub(crate) const PERSDL_DEVICE_SHIFT: u32 = 18;
pub(crate) const PERSDL_DEVICE_MASK: u32 = 0x3 << PERSDL_DEVICE_SHIFT;
pub(crate) const PERSDL_MAX_DEVICES: u32 = 4;
const PERSDL_CODE_LIMIT: u32 = 0x44_0000;

/// Game-controller codes (SDL-recognised pads).
pub(crate) const SDL_GC_BUTTON_VALUE: u32 = 0x40_0000;
pub(crate) const SDL_GC_AXIS_POS_VALUE: u32 = 0x41_0000;
pub(crate) const SDL_GC_AXIS_NEG_VALUE: u32 = 0x42_0000;
pub(crate) const SDL_GC_AXIS_ANALOG_VALUE: u32 = 0x43_0000;

/// Raw-joystick codes (devices SDL has no mapping for).
pub(crate) const SDL_MAX_AXIS_VALUE: u32 = 0x11_0000;
pub(crate) const SDL_MIN_AXIS_VALUE: u32 = 0x10_0000;
pub(crate) const SDL_HAT_VALUE: u32 = 0x20_0000;

/// Cardinal hat states accepted by the input scan.
pub(crate) const SDL_HAT_UP: u8 = 0x01;
pub(crate) const SDL_HAT_RIGHT: u8 = 0x02;
pub(crate) const SDL_HAT_DOWN: u8 = 0x04;
pub(crate) const SDL_HAT_LEFT: u8 = 0x08;

fn check_device(device: u32) -> Result<u32, ()> {
    if device >= PERSDL_MAX_DEVICES {
        return Err(());
    }
    Ok(device << PERSDL_DEVICE_SHIFT)
}

/// `SDL_GC_BUTTON_VALUE | (device << 18) | button`.
pub(crate) fn gc_button_code(device: u32, button: u16) -> Result<u32, ()> {
    Ok(SDL_GC_BUTTON_VALUE | check_device(device)? | u32::from(button))
}

/// `SDL_GC_AXIS_{POS,NEG}_VALUE | (device << 18) | axis`.
pub(crate) fn gc_axis_code(device: u32, axis: u16, positive: bool) -> Result<u32, ()> {
    let base = if positive {
        SDL_GC_AXIS_POS_VALUE
    } else {
        SDL_GC_AXIS_NEG_VALUE
    };
    Ok(base | check_device(device)? | u32::from(axis))
}

/// `SDL_GC_AXIS_ANALOG_VALUE | (device << 18) | axis`.
pub(crate) fn gc_axis_analog_code(device: u32, axis: u16) -> Result<u32, ()> {
    Ok(SDL_GC_AXIS_ANALOG_VALUE | check_device(device)? | u32::from(axis))
}

/// `(device << 18) | (button + 1)` for unmapped devices.
pub(crate) fn raw_button_code(device: u32, button: u16) -> Result<u32, ()> {
    Ok(check_device(device)? | (u32::from(button) + 1))
}

/// `(device << 18) | SDL_HAT_VALUE | (hat << 4) | index`; cardinal hats only.
pub(crate) fn raw_hat_code(device: u32, hat_index: u16, hat: u8) -> Result<u32, ()> {
    match hat {
        SDL_HAT_UP | SDL_HAT_RIGHT | SDL_HAT_DOWN | SDL_HAT_LEFT => Ok(check_device(device)?
            | SDL_HAT_VALUE
            | (u32::from(hat) << 4)
            | u32::from(hat_index)),
        _ => Err(()),
    }
}

/// `(device << 18) | SDL_{MAX,MIN}_AXIS_VALUE | axis` for unmapped devices.
pub(crate) fn raw_axis_code(device: u32, axis: u16, positive: bool) -> Result<u32, ()> {
    let base = if positive {
        SDL_MAX_AXIS_VALUE
    } else {
        SDL_MIN_AXIS_VALUE
    };
    Ok(base | check_device(device)? | u32::from(axis))
}

/// Re-point a stored binding at a different device, mirroring
/// `PERSDLRetargetCode`: unbound codes, out-of-core codes, and
/// unencodable devices come back unchanged.
pub(crate) fn retarget_code(key: u32, device: u32) -> u32 {
    if key == PERKEY_UNBOUND || device >= PERSDL_MAX_DEVICES {
        return key;
    }
    if (key & !PERSDL_DEVICE_MASK) >= PERSDL_CODE_LIMIT {
        return key;
    }
    (key & !PERSDL_DEVICE_MASK) | (device << PERSDL_DEVICE_SHIFT)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn identity_keys_follow_the_recorded_grammar() {
        assert_eq!(
            identity_key(0, 1, IdentityField::Type),
            "Input/Port/0/Id/1/Type"
        );
        assert_eq!(
            identity_key(1, 0, IdentityField::Device),
            "Input/Port/1/Id/0/Device"
        );
        assert_eq!(
            identity_key(0, 0, IdentityField::DeviceName),
            "Input/Port/0/Id/0/DeviceName"
        );
    }

    #[test]
    fn binding_key_uses_numeric_segments() {
        assert_eq!(
            binding_key(0, 0, PER_TYPE_PAD, 7),
            "Input/Port/0/Id/0/Controller/2/Key/7"
        );
        assert_eq!(
            binding_key(1, 2, PER_TYPE_MOUSE, 0),
            "Input/Port/1/Id/2/Controller/227/Key/0"
        );
    }

    #[test]
    fn pad_table_matches_perpad_source_order() {
        assert_eq!(PAD_BUTTONS.len(), 13);
        for (index, _) in PAD_BUTTONS {
            assert!(index <= 12);
        }
        assert_eq!(pad_button_name(0), Some("Up"));
        assert_eq!(pad_button_name(4), Some("R"));
        assert_eq!(pad_button_name(5), Some("L"));
        assert_eq!(pad_button_name(12), Some("Z"));
        assert_eq!(pad_button_name(13), None);
    }

    #[test]
    fn peripheral_ids_and_unbound_match_source() {
        assert_eq!(PER_TYPE_PAD, 0x02);
        assert_eq!(PER_TYPE_WHEEL, 0x13);
        assert_eq!(PER_TYPE_MISSION_STICK, 0x15);
        assert_eq!(PER_TYPE_3D_PAD, 0x16);
        assert_eq!(PER_TYPE_TWIN_STICKS, 0x19);
        assert_eq!(PER_TYPE_GUN, 0x25);
        assert_eq!(PER_TYPE_KEYBOARD, 0x34);
        assert_eq!(PER_TYPE_MOUSE, 0xE3);
        assert_eq!(PERKEY_UNBOUND, 0xFFFF_FFFF);
    }

    #[test]
    fn config_locations_match_the_capture() {
        assert_eq!(SETTINGS_GROUP, "0.9.11");
        assert_eq!(CONFIG_RELATIVE, "YabaSanshiro/qt/yabause.ini");
        assert_eq!(LEGACY_CONFIG_RELATIVE, ".yabause/yabause.ini");
        assert_eq!(WINDOWS_APPDATA_RELATIVE, "YabaSanshiro/yabause.ini");
    }

    #[test]
    fn host_codes_match_the_sdl_layout() {
        assert_eq!(gc_button_code(0, 5).unwrap(), 0x40_0000 | 5);
        assert_eq!(gc_button_code(2, 0).unwrap(), 0x40_0000 | (2 << 18));
        assert_eq!(gc_axis_code(1, 3, true).unwrap(), 0x41_0000 | (1 << 18) | 3);
        assert_eq!(
            gc_axis_code(1, 3, false).unwrap(),
            0x42_0000 | (1 << 18) | 3
        );
        assert_eq!(gc_axis_analog_code(0, 2).unwrap(), 0x43_0000 | 2);
        assert_eq!(raw_button_code(0, 0).unwrap(), 1);
        assert_eq!(raw_button_code(3, 7).unwrap(), (3 << 18) | 8);
        assert_eq!(
            raw_hat_code(0, 1, SDL_HAT_UP).unwrap(),
            0x20_0000 | (1 << 4) | 1
        );
        assert_eq!(raw_axis_code(0, 0, true).unwrap(), 0x11_0000);
        assert_eq!(raw_axis_code(0, 0, false).unwrap(), 0x10_0000);
        assert!(gc_button_code(4, 0).is_err());
        assert!(raw_hat_code(0, 0, 0x03).is_err());
        assert!(raw_hat_code(0, 0, 0x00).is_err());
    }

    #[test]
    fn retarget_follows_devices_like_the_core() {
        let code = gc_button_code(0, 5).unwrap();
        assert_eq!(retarget_code(code, 2), gc_button_code(2, 5).unwrap());
        assert_eq!(retarget_code(PERKEY_UNBOUND, 1), PERKEY_UNBOUND);
        assert_eq!(retarget_code(code, 4), code);
        assert_eq!(retarget_code(0x0100_0000, 1), 0x0100_0000);
    }

    #[test]
    fn saturn_routes_cover_the_pad_table() {
        assert_eq!(SATURN_ROUTES.len(), PAD_BUTTONS.len());
        for ((pad_key, name), (target, output)) in PAD_BUTTONS.iter().zip(SATURN_ROUTES.iter()) {
            assert_eq!(*output, format!("{name} (Key {pad_key})"));
            assert!(!target.is_empty());
        }
        let mut targets: Vec<&str> = SATURN_ROUTES.iter().map(|(t, _)| *t).collect();
        targets.sort_unstable();
        targets.dedup();
        assert_eq!(targets.len(), SATURN_ROUTES.len());
    }

    #[test]
    fn target_ids_resolve_to_pad_keys() {
        assert_eq!(pad_key_for_target("a"), Some(7));
        assert_eq!(pad_key_for_target("up"), Some(0));
        assert_eq!(pad_key_for_target("z"), Some(12));
        assert_eq!(pad_key_for_target("mode"), None);
        assert_eq!(pad_key_for_target(""), None);
        for (target, _) in SATURN_ROUTES {
            assert_eq!(pad_key_for_target(target).is_some(), true);
        }
    }

    #[test]
    fn ini_body_carries_decimal_pad_bindings() {
        let body = ini_body(0, 0, &[(7, 0x40_0005), (0, 0x40_0000)]).unwrap();
        assert_eq!(
            body,
            "[0.9.11]\nInput/Port/0/Id/0/Controller/2/Key/7=4194309\n\
             Input/Port/0/Id/0/Controller/2/Key/0=4194304\n"
        );
        assert!(ini_body(0, 0, &[(13, 0)]).is_err());
        assert_eq!(
            binding_line(1, 2, 6, 99).unwrap(),
            "Input/Port/1/Id/2/Controller/2/Key/6=99"
        );
        assert!(ini_body(0, 0, &[(7, 1), (7, 2)]).is_err());
    }
}
