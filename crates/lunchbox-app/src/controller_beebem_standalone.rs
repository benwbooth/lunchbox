//! BeebEm Windows persistence helpers (Preferences.cfg and BeebState slots).

use anyhow::{Result, ensure};
use std::path::{Path, PathBuf};

pub(crate) const DEFAULT_DATA_DIR: &str = "Documents/BeebEm";
pub(crate) const QUICK_SAVE: &str = "BeebState/quicksave.uefstate";
pub(crate) const PREFERENCES_TOKEN: &str = "*** BeebEm Preferences ***";

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum StickMode {
    Disabled,
    Joystick,
    AnalogueMouseStick,
    DigitalMouseStick,
}

impl StickMode {
    fn source_value(self) -> &'static str {
        match self {
            Self::Disabled => "Disabled",
            Self::Joystick => "Joystick",
            Self::AnalogueMouseStick => "AnalogueMouseStick",
            Self::DigitalMouseStick => "DigitalMouseStick",
        }
    }
}

/// BeebEm's Preferences.cfg is token/key=value text.  Keep unknown tokens so
/// a scoped adapter update does not erase unrelated user settings.
pub(crate) fn set_preference(text: &str, key: &str, value: &str) -> Result<String> {
    ensure!(text.len() <= 4 * 1024 * 1024, "BeebEm config is too large");
    ensure!(!text.contains('\0'), "BeebEm config contains a NUL byte");
    ensure!(
        text.lines().next() == Some(PREFERENCES_TOKEN),
        "BeebEm preferences token is missing"
    );
    ensure!(
        !key.is_empty() && !key.contains(['=', '\n', '\r']),
        "invalid preference key"
    );
    ensure!(!value.contains(['\n', '\r']), "invalid preference value");
    let newline = if text.contains("\r\n") { "\r\n" } else { "\n" };
    let mut found = false;
    let mut out = String::new();
    for raw in text.split_inclusive('\n') {
        let line = raw.trim_end_matches(['\r', '\n']);
        if line
            .split_once('=')
            .is_some_and(|(name, _)| name.trim() == key)
        {
            if !found {
                out.push_str(key);
                out.push('=');
                out.push_str(value);
                out.push_str(newline);
                found = true;
            }
        } else {
            out.push_str(raw);
        }
    }
    if !found {
        if !out.ends_with(['\n', '\r']) {
            out.push_str(newline);
        }
        out.push_str(key);
        out.push('=');
        out.push_str(value);
        out.push_str(newline);
    }
    Ok(out)
}

pub(crate) fn patch_stick_mode(baseline: &[u8], mode: StickMode) -> Result<String> {
    let text = std::str::from_utf8(baseline)?;
    set_preference(text, "Sticks", mode.source_value())
}

pub(crate) fn quicksave_path(data_root: &Path, slot: Option<u8>) -> Result<PathBuf> {
    if let Some(slot) = slot {
        ensure!(
            (1..=9).contains(&slot),
            "BeebEm quicksave backup must be 1 through 9"
        );
    }
    let name = slot.map_or_else(
        || "quicksave.uefstate".to_string(),
        |n| format!("quicksave{n}.uefstate"),
    );
    Ok(data_root.join("BeebState").join(name))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn stick_mode_patch_preserves_unknowns_and_crlf() {
        let baseline = b"*** BeebEm Preferences ***\r\n# keep\r\n Sticks = Disabled\r\nOther=yes\r\nSticks=DigitalMouseStick\r\n";
        let output = patch_stick_mode(baseline, StickMode::Joystick).unwrap();
        assert_eq!(output.matches("Sticks=").count(), 1);
        assert!(output.contains("# keep\r\nSticks=Joystick\r\nOther=yes\r\n"));
        assert!(!output.contains("\nSticks=DigitalMouseStick"));
    }

    #[test]
    fn rejects_invalid_config_and_slot() {
        assert!(patch_stick_mode(b"Sticks=Disabled\n", StickMode::Joystick).is_err());
        assert!(quicksave_path(Path::new("root"), Some(0)).is_err());
        assert_eq!(
            quicksave_path(Path::new("root"), Some(9)).unwrap(),
            PathBuf::from("root/BeebState/quicksave9.uefstate")
        );
    }
}
