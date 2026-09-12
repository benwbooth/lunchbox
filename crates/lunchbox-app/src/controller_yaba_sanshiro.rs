//! Yaba Sanshiro 2 native `yabause.ini` input key grammar (Qt frontend).
//! Evidence: `emulator_details/records/yaba-sanshiro-2.json` (devmiyax/yabause
//! v1.20.37 source tarball role). Config: `~/.config/YabaSanshiro/qt/yabause.ini`
//! (legacy `~/.yabause/yabause.ini` copied on first run); Windows: portable
//! `<dir-of-yabause.exe>\yabause.ini` else `%APPDATA%\YabaSanshiro\yabause.ini`.
//! All keys live under the `[0.9.11]` group. Per-device identity is
//! `Input/Port/<port>/Id/<id>/Type|Device|DeviceName` and pad bindings are
//! `Input/Port/<port>/Id/<id>/Controller/<peripheral>/Key/<button>` holding a
//! u32 core key code. Peripheral-type strings and button-index tables are NOT
//! captured here: the record verifies the grammar, not the value tables, so
//! this module builds validated key paths only. Session, native_command, and
//! full wiring will follow in a later cycle.

/// QSettings group every Yaba Sanshiro 2 key lives under.
pub(crate) const SETTINGS_GROUP: &str = "0.9.11";

/// Config path relative to the XDG config home on Linux.
pub(crate) const CONFIG_RELATIVE: &str = "YabaSanshiro/qt/yabause.ini";

/// Legacy config path relative to `$HOME`, copied forward on first run.
pub(crate) const LEGACY_CONFIG_RELATIVE: &str = ".yabause/yabause.ini";

/// Config path relative to `%APPDATA%` on Windows (non-portable installs).
pub(crate) const WINDOWS_APPDATA_RELATIVE: &str = "YabaSanshiro/yabause.ini";

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

fn check_segment(segment: &str) -> Result<(), ()> {
    if segment.is_empty()
        || segment.contains('/')
        || segment.contains("..")
        || segment.chars().any(|c| c.is_control())
    {
        return Err(());
    }
    Ok(())
}

/// `Input/Port/<port>/Id/<id>/<Type|Device|DeviceName>`.
pub(crate) fn identity_key(port: u32, id: u32, field: IdentityField) -> String {
    format!("Input/Port/{port}/Id/{id}/{}", field.as_str())
}

/// `Input/Port/<port>/Id/<id>/Controller/<peripheral>/Key/<button>`.
/// `peripheral` and `button` are validated as single INI key segments;
/// their vocabularies are source-unverified and stay caller-supplied.
pub(crate) fn binding_key(
    port: u32,
    id: u32,
    peripheral: &str,
    button: &str,
) -> Result<String, ()> {
    check_segment(peripheral)?;
    check_segment(button)?;
    Ok(format!(
        "Input/Port/{port}/Id/{id}/Controller/{peripheral}/Key/{button}"
    ))
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
    fn binding_key_format_is_correct() {
        // "ExamplePad" is a format placeholder, not a verified peripheral name.
        assert_eq!(
            binding_key(0, 0, "ExamplePad", "Up").unwrap(),
            "Input/Port/0/Id/0/Controller/ExamplePad/Key/Up"
        );
    }

    #[test]
    fn binding_key_rejects_malformed_segments() {
        assert!(binding_key(0, 0, "", "Up").is_err());
        assert!(binding_key(0, 0, "ExamplePad", "").is_err());
        assert!(binding_key(0, 0, "Pad/Key", "Up").is_err());
        assert!(binding_key(0, 0, "Pad", "Up\x07").is_err());
        assert!(binding_key(0, 0, "..", "Up").is_err());
    }

    #[test]
    fn config_locations_match_the_capture() {
        assert_eq!(SETTINGS_GROUP, "0.9.11");
        assert_eq!(CONFIG_RELATIVE, "YabaSanshiro/qt/yabause.ini");
        assert_eq!(LEGACY_CONFIG_RELATIVE, ".yabause/yabause.ini");
        assert_eq!(WINDOWS_APPDATA_RELATIVE, "YabaSanshiro/yabause.ini");
    }
}
