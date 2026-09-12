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
}
