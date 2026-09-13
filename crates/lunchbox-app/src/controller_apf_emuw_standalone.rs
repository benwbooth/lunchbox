//! APF_EMUW keyboard/joystick override writer.
//!
//! The official APF_EMUW 2.0.1 Windows archive is an old standalone binary,
//! not a source project. Its bundled `APFEMU-Eng.txt` documents the only
//! portable input override: `APF_EMU.KYS`, whose lines contain the Windows
//! key value followed by the APF key value. The emulator's physical joystick
//! identity and its `apf_emuw.ini` settings are outside this writer.

use anyhow::{Result, ensure};
use std::collections::BTreeSet;

pub(crate) const OFFICIAL_ARCHIVE_URL: &str =
    "https://orphanedgames.com/APF/apf_emulation/apf_emuw.zip";
pub(crate) const ARCHIVE_SHA256: &str =
    "adb6c061094478d8b2b44d51b8ee08858f60e61e786daba89721266ec6b41629";
pub(crate) const PROFILE_ID: &str = "apf-emuw:standalone-kys-v1";

/// Render APF_EMUW's documented `APF_EMU.KYS` two-column override file.
/// Values are intentionally opaque: the upstream manual says the first is
/// the value produced by Windows for the key and the second is the matching
/// APF key value. The caller must obtain those values from the same Windows
/// keyboard layout (the manual specifically recommends exporting them from
/// the emulator first).
pub(crate) fn key_overrides(overrides: &[(u16, u16)]) -> Result<String> {
    ensure!(!overrides.is_empty(), "APF_EMUW key override list is empty");
    ensure!(
        overrides.len() <= 256,
        "APF_EMUW key override list is too large"
    );
    let mut output = String::with_capacity(overrides.len() * 14);
    let mut windows_values = BTreeSet::new();
    for &(windows_value, apf_value) in overrides {
        ensure!(
            windows_values.insert(windows_value),
            "APF_EMUW Windows key value is duplicated"
        );
        output.push_str(&format!("{windows_value} {apf_value}\r\n"));
    }
    Ok(output)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn renders_documented_two_number_kys_lines() {
        assert_eq!(
            key_overrides(&[(30, 1), (31, 2)]).unwrap(),
            "30 1\r\n31 2\r\n"
        );
    }

    #[test]
    fn refuses_empty_override_file() {
        assert!(key_overrides(&[]).is_err());
        assert!(key_overrides(&[(30, 1), (30, 2)]).is_err());
    }
}
