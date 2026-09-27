//! Xenia SDL GameControllerDB mapping writer.
//!
//! Pinned to xenia-project/xenia
//! `95a5c3ee250f80c3b9d139658649d9ffb6db3eec`. The native HID SDL driver
//! exposes the source `SDL.mappings_file` path (default
//! `gamecontrollerdb.txt`) and loads standard SDL GameControllerDB lines with
//! `SDL_GameControllerAddMappingsFromRW`. Xenia's logical Xbox controls are
//! therefore authorable through this exact SDL database surface; no guessed
//! Xenia-specific TOML keys are emitted.

use anyhow::{Context, Result, ensure};
use std::collections::BTreeSet;

pub(crate) const SOURCE_COMMIT: &str = "95a5c3ee250f80c3b9d139658649d9ffb6db3eec";
pub(crate) const PROFILE_ID: &str = "xenia:native-sdl-gamecontrollerdb-v1";
pub(crate) const DEFAULT_MAPPINGS_FILE: &str = "gamecontrollerdb.txt";

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct Mapping {
    /// SDL's 32-hex-digit controller GUID, measured from the same host SDL.
    pub guid: String,
    pub name: String,
    /// SDL mapping fields such as `a:b0`, `leftx:a0`, or `dpup:h0.1`.
    pub bindings: Vec<(String, String)>,
}

fn valid_field(field: &str) -> bool {
    matches!(
        field,
        "a" | "b"
            | "x"
            | "y"
            | "back"
            | "guide"
            | "start"
            | "leftstick"
            | "rightstick"
            | "leftshoulder"
            | "rightshoulder"
            | "lefttrigger"
            | "righttrigger"
            | "dpup"
            | "dpdown"
            | "dpleft"
            | "dpright"
            | "leftx"
            | "lefty"
            | "rightx"
            | "righty"
    )
}

const REQUIRED_FIELDS: [&str; 20] = [
    "a",
    "b",
    "x",
    "y",
    "back",
    "start",
    "leftstick",
    "rightstick",
    "leftshoulder",
    "rightshoulder",
    "lefttrigger",
    "righttrigger",
    "dpup",
    "dpdown",
    "dpleft",
    "dpright",
    "leftx",
    "lefty",
    "rightx",
    "righty",
];

fn decimal(value: &str, limit: u16) -> bool {
    !value.is_empty()
        && value.bytes().all(|byte| byte.is_ascii_digit())
        && value.parse::<u16>().is_ok_and(|number| number < limit)
}

fn valid_value(value: &str) -> bool {
    if let Some(button) = value.strip_prefix('b') {
        return decimal(button, 256);
    }
    if let Some(hat) = value.strip_prefix('h') {
        return hat.split_once('.').is_some_and(|(index, mask)| {
            decimal(index, 64) && matches!(mask, "1" | "2" | "4" | "8")
        });
    }
    let axis = value.strip_suffix('~').unwrap_or(value);
    let axis = axis
        .strip_prefix('+')
        .or_else(|| axis.strip_prefix('-'))
        .unwrap_or(axis);
    axis.strip_prefix('a')
        .is_some_and(|index| decimal(index, 256))
}

fn mapping_line(mapping: &Mapping) -> Result<String> {
    ensure!(
        mapping.guid.len() == 32 && mapping.guid.bytes().all(|b| b.is_ascii_hexdigit()),
        "Xenia SDL GUID must contain 32 hex digits"
    );
    ensure!(
        !mapping.name.is_empty()
            && mapping.name.len() <= 128
            && !mapping.name.contains([',', '\n', '\r']),
        "Xenia SDL controller name is invalid"
    );
    ensure!(
        !mapping.bindings.is_empty(),
        "Xenia SDL mapping needs at least one binding"
    );
    let mut fields = BTreeSet::new();
    let mut out = format!("{},{}", mapping.guid.to_ascii_lowercase(), mapping.name);
    for (field, value) in &mapping.bindings {
        ensure!(valid_field(field), "Xenia SDL mapping field is unsupported");
        ensure!(valid_value(value), "Xenia SDL mapping value is invalid");
        ensure!(
            fields.insert(field.clone()),
            "Xenia SDL mapping field is duplicated"
        );
        out.push(',');
        out.push_str(field);
        out.push(':');
        out.push_str(value);
    }
    for required in REQUIRED_FIELDS {
        ensure!(
            fields.contains(required),
            "Xenia SDL mapping is missing {required}"
        );
    }
    out.push('\n');
    Ok(out)
}

pub(crate) fn gamecontrollerdb_line(mapping: &Mapping) -> Result<String> {
    mapping_line(mapping)
}

/// Replace mappings with matching GUIDs in a copied SDL database and append
/// the requested source-shaped records. Comments and unrelated devices are
/// preserved. Xenia receives the file via `--SDL.mappings_file=<path>` (or a
/// build-specific equivalent for its cvar parser).
pub(crate) fn patch_mappings(baseline: &[u8], mappings: &[Mapping]) -> Result<String> {
    ensure!(
        baseline.len() <= 4 * 1024 * 1024,
        "Xenia mappings file is too large"
    );
    ensure!(!mappings.is_empty(), "Xenia needs at least one SDL mapping");
    let text = std::str::from_utf8(baseline).context("Xenia mappings file is not UTF-8")?;
    let mut lines = Vec::new();
    let mut guids = BTreeSet::new();
    for mapping in mappings {
        let _ = mapping_line(mapping)?;
        ensure!(
            guids.insert(mapping.guid.to_ascii_lowercase()),
            "Xenia SDL mapping GUID is duplicated"
        );
    }
    for line in text.lines() {
        let guid = line
            .split_once(',')
            .map(|(guid, _)| guid.trim().to_ascii_lowercase());
        if guid.is_none() || !guids.contains(&guid.unwrap()) {
            lines.push(line);
        }
    }
    let mut out = lines.join("\n");
    if !out.is_empty() {
        out.push('\n');
    }
    for mapping in mappings {
        out.push_str(&mapping_line(mapping)?);
    }
    Ok(out)
}

pub(crate) fn source_boundary() -> &'static str {
    "Xenia's SDL GameControllerDB is the exact authorable controller surface; use a measured SDL GUID and pass the patched file through SDL.mappings_file. This does not establish an official packaged Linux runtime or gameplay parity."
}

#[cfg(test)]
mod tests {
    use super::*;

    fn mapping() -> Mapping {
        Mapping {
            guid: "0123456789abcdef0123456789abcdef".into(),
            name: "Measured Pad".into(),
            bindings: vec![
                ("a".into(), "b0".into()),
                ("b".into(), "b1".into()),
                ("x".into(), "b2".into()),
                ("y".into(), "b3".into()),
                ("back".into(), "b4".into()),
                ("start".into(), "b5".into()),
                ("leftstick".into(), "b6".into()),
                ("rightstick".into(), "b7".into()),
                ("leftshoulder".into(), "b8".into()),
                ("rightshoulder".into(), "b9".into()),
                ("lefttrigger".into(), "a4".into()),
                ("righttrigger".into(), "a5".into()),
                ("dpup".into(), "h0.1".into()),
                ("dpdown".into(), "h0.4".into()),
                ("dpleft".into(), "h0.8".into()),
                ("dpright".into(), "h0.2".into()),
                ("leftx".into(), "a0~".into()),
                ("lefty".into(), "a1".into()),
                ("rightx".into(), "a2".into()),
                ("righty".into(), "a3".into()),
            ],
        }
    }

    #[test]
    fn emits_sdl_database_line_and_replaces_guid() {
        let line = gamecontrollerdb_line(&mapping()).unwrap();
        assert_eq!(
            line,
            "0123456789abcdef0123456789abcdef,Measured Pad,a:b0,b:b1,x:b2,y:b3,back:b4,start:b5,leftstick:b6,rightstick:b7,leftshoulder:b8,rightshoulder:b9,lefttrigger:a4,righttrigger:a5,dpup:h0.1,dpdown:h0.4,dpleft:h0.8,dpright:h0.2,leftx:a0~,lefty:a1,rightx:a2,righty:a3\n"
        );
        let patched = patch_mappings(
            b"# keep\n0123456789ABCDEF0123456789ABCDEF,old,a:b1\n",
            &[mapping()],
        )
        .unwrap();
        assert!(patched.starts_with("# keep\n"));
        assert_eq!(patched.lines().count(), 2);
        assert_eq!(
            patched
                .lines()
                .filter(|line| line.starts_with("0123456789abcdef0123456789abcdef,"))
                .count(),
            1
        );
        assert!(!patched.contains("0123456789ABCDEF0123456789ABCDEF"));
    }

    #[test]
    fn rejects_duplicate_or_invalid_fields() {
        let mut bad = mapping();
        bad.bindings.push(("a".into(), "b1".into()));
        assert!(gamecontrollerdb_line(&bad).is_err());
        bad.bindings[0].0 = "not-a-control".into();
        bad.bindings.pop();
        assert!(gamecontrollerdb_line(&bad).is_err());
        let mut bad = mapping();
        bad.bindings[0].1 = "button zero".into();
        assert!(gamecontrollerdb_line(&bad).is_err());
    }

    #[test]
    fn rejects_duplicate_guids() {
        assert!(patch_mappings(b"", &[mapping(), mapping()]).is_err());
    }
}
