//! Emulicious standalone-native controller profile writer.
//!
//! Pinned release: the official `Emulicious` download (archive SHA-256
//! `6e1c6d511014033bbc2668360a0194389a5bad2bf6c5ffd0fe093b84da33c0fc`).
//! The release's `platform.Emulicious$243` and `$50` classes establish the
//! portable Java-properties spelling: a selected JInput controller is stored
//! as `Gamepad<N>Key<Action>` (with `_1` for the secondary assignment), while
//! keyboard mappings use `Key<Action>`/`Key<Action>_1`.

use anyhow::{Context, Result, ensure};
use std::collections::{BTreeMap, BTreeSet};

pub(crate) const SOURCE_ARCHIVE_URL: &str =
    "https://emulicious.net/download/emulicious/?wpdmdl=205";
pub(crate) const SOURCE_ARCHIVE_SHA256: &str =
    "6e1c6d511014033bbc2668360a0194389a5bad2bf6c5ffd0fe093b84da33c0fc";
pub(crate) const PROFILE_ID: &str = "emulicious:standalone-native-jinput-v1";

/// One of the six gamepad properties consumed by Emulicious's input dialog.
#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub(crate) enum Target {
    GameBoy,
    MasterSystemA,
    MasterSystemB,
    MasterSystemButtons,
    MsxA,
    MsxB,
}

impl Target {
    fn selector_key(self) -> &'static str {
        match self {
            Self::GameBoy => "GBGamepad",
            Self::MasterSystemA => "SMSGamepadA",
            Self::MasterSystemB => "SMSGamepadB",
            Self::MasterSystemButtons => "SMSbuttonsGamepad",
            Self::MsxA => "MSXGamepadA",
            Self::MsxB => "MSXGamepadB",
        }
    }

    fn threshold_key(self) -> &'static str {
        match self {
            Self::GameBoy => "GBGamepadThreshold",
            Self::MasterSystemA => "SMSGamepadAThreshold",
            Self::MasterSystemB => "SMSGamepadBThreshold",
            Self::MasterSystemButtons => "SMSbuttonsThreshold",
            Self::MsxA => "MSXGamepadAThreshold",
            Self::MsxB => "MSXGamepadBThreshold",
        }
    }

    fn keyboard_key(self) -> &'static str {
        match self {
            Self::GameBoy => "GBGamepadKeyboard",
            Self::MasterSystemA => "SMSGamepadAKeyboard",
            Self::MasterSystemB => "SMSGamepadBKeyboard",
            Self::MasterSystemButtons => "SMSbuttonsKeyboard",
            Self::MsxA => "MSXGamepadAKeyboard",
            Self::MsxB => "MSXGamepadBKeyboard",
        }
    }
}

/// One JInput event ID. Values are component base IDs (`component * 4`) plus
/// the release's documented direction offsets; `Unassigned` serializes to -1.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum Binding {
    Unassigned,
    JInputEvent(i32),
}

impl Binding {
    fn value(self) -> Result<i32> {
        match self {
            Self::Unassigned => Ok(-1),
            Self::JInputEvent(value) => {
                ensure!(
                    (0..(268 * 4)).contains(&value),
                    "Emulicious JInput event is out of range"
                );
                Ok(value)
            }
        }
    }
}

/// A target's selection and action mappings. Action IDs are the global IDs
/// used by Emulicious (`0..267`), not an invented per-console numbering.
/// Each action has a primary and optional secondary assignment.
#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct GamepadProfile {
    pub target: Target,
    /// `None` selects keyboard (`-1`); `Some(n)` selects JInput controller n.
    pub gamepad: Option<u8>,
    pub keyboard: bool,
    /// The release's input dialog slider accepts 10..99 percent.
    pub threshold: u8,
    pub assignments: BTreeMap<u16, [Binding; 2]>,
}

fn fields(profiles: &[GamepadProfile]) -> Result<BTreeMap<String, String>> {
    ensure!(
        !profiles.is_empty() && profiles.len() <= 6,
        "Emulicious profile count is invalid"
    );
    let mut targets = BTreeSet::new();
    let mut fields = BTreeMap::new();
    for profile in profiles {
        ensure!(
            targets.insert(profile.target),
            "Emulicious target is duplicated"
        );
        ensure!(
            (10..=99).contains(&profile.threshold),
            "Emulicious threshold must be 10 through 99"
        );
        insert_field(
            &mut fields,
            profile.target.selector_key().to_owned(),
            profile
                .gamepad
                .map_or_else(|| "-1".to_owned(), |slot| slot.to_string()),
        )?;
        insert_field(
            &mut fields,
            profile.target.keyboard_key().to_owned(),
            profile.keyboard.to_string(),
        )?;
        insert_field(
            &mut fields,
            profile.target.threshold_key().to_owned(),
            profile.threshold.to_string(),
        )?;
        let (prefix, suffix) = match profile.gamepad {
            Some(slot) => (format!("Gamepad{slot}Key"), ""),
            None => ("Key".to_owned(), ""),
        };
        for (&action, bindings) in &profile.assignments {
            ensure!(action < 268, "Emulicious action ID is out of range");
            for (assignment, binding) in bindings.iter().enumerate() {
                let key = if assignment == 0 {
                    format!("{prefix}{action}{suffix}")
                } else {
                    format!("{prefix}{action}_{assignment}{suffix}")
                };
                insert_field(&mut fields, key, binding.value()?.to_string())?;
            }
        }
    }
    Ok(fields)
}

fn insert_field(fields: &mut BTreeMap<String, String>, key: String, value: String) -> Result<()> {
    if let Some(existing) = fields.get(&key) {
        ensure!(
            existing == &value,
            "Emulicious profiles conflict on property {key}"
        );
    } else {
        fields.insert(key, value);
    }
    Ok(())
}

/// Patch only the observed Emulicious Java-properties keys. Unknown settings,
/// comments, ordering, and line endings are retained. JInput controller
/// indices are process-environment order and must be verified before launch.
pub(crate) fn patch_config(baseline: &[u8], profiles: &[GamepadProfile]) -> Result<String> {
    ensure!(
        baseline.len() <= 4 * 1024 * 1024,
        "Emulicious config is too large"
    );
    let original = std::str::from_utf8(baseline).context("Emulicious config is not UTF-8")?;
    ensure!(
        !original.contains('\0'),
        "Emulicious config contains a NUL byte"
    );
    let fields = fields(profiles)?;
    let newline = if original.contains("\r\n") {
        "\r\n"
    } else {
        "\n"
    };
    let mut output = String::with_capacity(original.len() + fields.len() * 24);
    let mut seen = BTreeSet::new();
    for line in original.split_inclusive('\n') {
        let key = property_key(line).and_then(|key| fields.get_key_value(key).map(|(key, _)| key));
        if let Some(key) = key {
            ensure!(
                seen.insert(key.clone()),
                "Emulicious property is duplicated: {key}"
            );
            output.push_str(key);
            output.push('=');
            output.push_str(&fields[key]);
            output.push_str(newline);
        } else {
            output.push_str(line);
        }
    }
    if !output.is_empty() && !output.ends_with('\n') {
        output.push_str(newline);
    }
    for (key, value) in fields {
        if !seen.contains(&key) {
            output.push_str(&key);
            output.push('=');
            output.push_str(&value);
            output.push_str(newline);
        }
    }
    Ok(output)
}

fn property_key(line: &str) -> Option<&str> {
    let body = line
        .trim_end_matches(['\r', '\n'])
        .trim_start_matches('\u{feff}')
        .trim_start();
    if body.is_empty() || body.starts_with('#') || body.starts_with('!') {
        return None;
    }
    let end = body.find(|character: char| {
        character == '=' || character == ':' || character.is_whitespace()
    })?;
    let key = &body[..end];
    (!key.is_empty()).then_some(key)
}

pub(crate) fn source_boundary() -> &'static str {
    "The pinned release establishes a Java-properties writer for JInput controller indices, action IDs, thresholds, keyboard-union flags, and six system selectors. JInput indices are process-environment order rather than stable physical identity: enumerate and verify the selected controller immediately before launch. Preserve the copied Emulicious install root, ROM-relative .sav policy, sstates directory, optional BIOS paths, and all unrelated INI properties; this writer does not prove executable startup or gameplay input."
}

#[cfg(test)]
mod tests {
    use super::*;

    fn profile(gamepad: Option<u8>) -> GamepadProfile {
        GamepadProfile {
            target: Target::GameBoy,
            gamepad,
            keyboard: true,
            threshold: 50,
            assignments: BTreeMap::from([
                (0, [Binding::JInputEvent(20), Binding::Unassigned]),
                (6, [Binding::JInputEvent(24), Binding::JInputEvent(25)]),
            ]),
        }
    }

    #[test]
    fn patches_gamepad_and_secondary_properties_without_touching_other_settings() {
        let output = patch_config(
            b"#keep\nAudioSync=false\nGBGamepad=-1\nGamepad0Key0=3\nGamepad0Key6_1=-1\n",
            &[profile(Some(0))],
        )
        .unwrap();
        assert!(output.contains("AudioSync=false\n"));
        assert!(output.contains("GBGamepad=0\n"));
        assert!(output.contains("Gamepad0Key0=20\n"));
        assert!(output.contains("Gamepad0Key6=24\n"));
        assert!(output.contains("Gamepad0Key6_1=25\n"));
        assert!(!output.contains("Gamepad0Key0=3"));
    }

    #[test]
    fn keyboard_profile_uses_key_namespace_and_preserves_bom() {
        let output = patch_config(
            "\u{feff}#settings\r\nGBGamepad=-1\r\nKey0=90\r\n".as_bytes(),
            &[profile(None)],
        )
        .unwrap();
        assert!(output.starts_with("\u{feff}#settings\r\n"));
        assert!(output.contains("GBGamepad=-1\r\n"));
        assert!(output.contains("Key0=20\r\n"));
        assert!(output.contains("Key6=24\r\nKey6_1=25\r\n"));
        assert!(!output.contains("Gamepad0Key0"));
    }

    #[test]
    fn rejects_duplicate_targets_bad_thresholds_and_invalid_events() {
        assert!(patch_config(b"", &[profile(Some(0)), profile(Some(1))]).is_err());
        let mut bad = profile(Some(0));
        bad.threshold = 9;
        assert!(patch_config(b"", &[bad]).is_err());
        let mut bad = profile(Some(0));
        bad.assignments
            .insert(1, [Binding::JInputEvent(1072), Binding::Unassigned]);
        assert!(patch_config(b"", &[bad]).is_err());
        let mut other = profile(Some(0));
        other.target = Target::MsxA;
        other
            .assignments
            .insert(0, [Binding::JInputEvent(21), Binding::Unassigned]);
        assert!(patch_config(b"", &[profile(Some(0)), other]).is_err());
    }
}
