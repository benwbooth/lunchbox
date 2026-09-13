//! ADAM+ Qt-settings controller writer.
//!
//! Pinned source: dvdh1961/ADAMP commit
//! f82570765bc97a2b95138cbeb33df57172d7ab94.  `maingui.cpp`/`maingui_org.cpp`
//! use `settings.ini` beside the executable, while `joypadwindow.cpp` and
//! `inputwidget.cpp` consume `input/p1/{0..17}`, `input/p2/{0..17}` and
//! `controller/joystickType`.  QSettings serializes these as an `[input]`
//! section with `p1/N`/`p2/N` keys and a `[controller]` section.

use anyhow::{Context, Result, ensure};

pub(crate) const PROFILE_ID: &str = "adam-plus:qt-settings-input-v1";
pub(crate) const SOURCE_COMMIT: &str = "f82570765bc97a2b95138cbeb33df57172d7ab94";

pub(crate) const INPUT_COUNT: usize = 18;

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct PlayerMapping {
    /// Qt key values or ADAM+'s `0x10000 + joystick-button` values.
    pub keys: [i32; INPUT_COUNT],
}

fn fields(joystick_type: u8, players: &[PlayerMapping; 2]) -> Vec<(String, String)> {
    let mut fields = Vec::with_capacity(39);
    fields.push(("controller/joystickType".into(), joystick_type.to_string()));
    for (player, mapping) in players.iter().enumerate() {
        fields.push((format!("input/p{}/type", player + 1), "0".into()));
        for (index, key) in mapping.keys.iter().enumerate() {
            fields.push((format!("input/p{}/{}", player + 1, index), key.to_string()));
        }
    }
    fields
}

fn section_and_key(path: &str, nested: bool) -> (&str, &str) {
    if path == "controller/joystickType" {
        return ("controller", "joystickType");
    }
    let (_, rest) = path.split_once('/').expect("ADAM+ field has a section");
    if nested {
        let (_player, key) = rest.split_once('/').expect("ADAM+ input field has a key");
        (
            if path.starts_with("input/p1/") {
                "input/p1"
            } else {
                "input/p2"
            },
            key,
        )
    } else {
        ("input", rest)
    }
}

fn header(line: &str) -> Option<&str> {
    let trimmed = line.trim_start();
    trimmed
        .strip_prefix('[')
        .and_then(|rest| rest.split_once(']').map(|(name, _)| name.trim()))
}

fn full_path(section: &str, key: &str) -> Option<String> {
    match section.to_ascii_lowercase().as_str() {
        "controller" if key.eq_ignore_ascii_case("joysticktype") => {
            Some("controller/joystickType".into())
        }
        "input" if key.to_ascii_lowercase().starts_with("p1/") => {
            Some(format!("input/p1/{}", &key[3..]))
        }
        "input" if key.to_ascii_lowercase().starts_with("p2/") => {
            Some(format!("input/p2/{}", &key[3..]))
        }
        "input/p1" => Some(format!("input/p1/{key}")),
        "input/p2" => Some(format!("input/p2/{key}")),
        _ => None,
    }
}

fn emit_missing(
    fields: &[(String, String)],
    seen: &mut [bool],
    section: &str,
    nested: bool,
    newline: &str,
    output: &mut String,
) {
    let needs_any = fields.iter().enumerate().any(|(index, (path, _))| {
        section_and_key(path, nested)
            .0
            .eq_ignore_ascii_case(section)
            && !seen[index]
    });
    if needs_any && !output.is_empty() && !output.ends_with(['\n', '\r']) {
        output.push_str(newline);
    }
    for (index, (path, value)) in fields.iter().enumerate() {
        let (field_section, key) = section_and_key(path, nested);
        if field_section.eq_ignore_ascii_case(section) && !seen[index] {
            output.push_str(&format!("{key}={value}{newline}"));
            seen[index] = true;
        }
    }
}

/// Patch only ADAM+'s source-defined controller fields in a copied
/// `settings.ini`. Existing comments, sections, and unrelated settings are
/// retained. The physical joystick is not persisted by ADAM+: its frontend
/// starts polling runtime index 0, so the caller must measure and verify that
/// device immediately before the same launch.
pub(crate) fn patch_config(
    baseline: &[u8],
    joystick_type: u8,
    players: [PlayerMapping; 2],
) -> Result<String> {
    ensure!(
        baseline.len() <= 4 * 1024 * 1024,
        "ADAM+ settings.ini is too large"
    );
    ensure!(joystick_type <= 2, "ADAM+ joystick type must be 0, 1, or 2");
    let original = std::str::from_utf8(baseline).context("ADAM+ settings.ini is not UTF-8")?;
    ensure!(
        !original.contains('\0'),
        "ADAM+ settings.ini contains a NUL byte"
    );
    for player in &players {
        for key in player.keys {
            ensure!(
                key >= 0,
                "ADAM+ key values must be non-negative Qt/button codes"
            );
        }
    }

    let fields = fields(joystick_type, &players);
    let newline = if original.contains("\r\n") {
        "\r\n"
    } else {
        "\n"
    };
    let mut controller_sections = 0usize;
    let mut flat_input_sections = 0usize;
    let mut p1_sections = 0usize;
    let mut p2_sections = 0usize;
    for line in original.split_inclusive('\n') {
        if let Some(section) = header(line) {
            if section.eq_ignore_ascii_case("controller") {
                controller_sections += 1;
            } else if section.eq_ignore_ascii_case("input") {
                flat_input_sections += 1;
            } else if section.eq_ignore_ascii_case("input/p1") {
                p1_sections += 1;
            } else if section.eq_ignore_ascii_case("input/p2") {
                p2_sections += 1;
            }
        }
    }
    ensure!(
        controller_sections <= 1
            && flat_input_sections <= 1
            && p1_sections <= 1
            && p2_sections <= 1,
        "ADAM+ settings.ini contains a duplicate owned section"
    );
    ensure!(
        flat_input_sections == 0 || (p1_sections == 0 && p2_sections == 0),
        "ADAM+ settings.ini mixes flat and nested input sections"
    );
    let nested = p1_sections > 0 || p2_sections > 0;
    let mut seen = vec![false; fields.len()];
    let mut output = String::with_capacity(original.len() + fields.len() * 24);
    let mut active = String::new();
    let mut inserted_controller = false;
    let has_input_section = flat_input_sections > 0 || nested;

    for line in original.split_inclusive('\n') {
        if let Some(new_section) = header(line) {
            if active.eq_ignore_ascii_case("controller") && !inserted_controller {
                emit_missing(
                    &fields,
                    &mut seen,
                    "controller",
                    nested,
                    newline,
                    &mut output,
                );
                inserted_controller = true;
            }
            if active.eq_ignore_ascii_case("input")
                || active.eq_ignore_ascii_case("input/p1")
                || active.eq_ignore_ascii_case("input/p2")
            {
                emit_missing(&fields, &mut seen, &active, nested, newline, &mut output);
            }
            active = new_section.to_owned();
            output.push_str(line);
            continue;
        }

        if let Some((key, _)) = line.split_once('=') {
            if let Some(path) = full_path(&active, key.trim()) {
                if let Some(index) = fields.iter().position(|(candidate, _)| candidate == &path) {
                    ensure!(
                        !seen[index],
                        "ADAM+ settings.ini contains a duplicate controller key"
                    );
                    let (_, value) = &fields[index];
                    output.push_str(&format!("{key}={value}{newline}"));
                    seen[index] = true;
                    continue;
                }
            }
        }
        output.push_str(line);
    }

    if active.eq_ignore_ascii_case("controller") && !inserted_controller {
        emit_missing(
            &fields,
            &mut seen,
            "controller",
            nested,
            newline,
            &mut output,
        );
        inserted_controller = true;
    }
    if active.eq_ignore_ascii_case("input")
        || active.eq_ignore_ascii_case("input/p1")
        || active.eq_ignore_ascii_case("input/p2")
    {
        emit_missing(&fields, &mut seen, &active, nested, newline, &mut output);
    }

    if !inserted_controller {
        if !output.is_empty() && !output.ends_with(['\n', '\r']) {
            output.push_str(newline);
        }
        output.push_str("[controller]");
        output.push_str(newline);
        emit_missing(
            &fields,
            &mut seen,
            "controller",
            nested,
            newline,
            &mut output,
        );
    }
    if !has_input_section {
        if !output.is_empty() && !output.ends_with(['\n', '\r']) {
            output.push_str(newline);
        }
        output.push_str("[input]");
        output.push_str(newline);
        emit_missing(&fields, &mut seen, "input", nested, newline, &mut output);
    } else if nested {
        for section in ["input/p1", "input/p2"] {
            let missing = fields
                .iter()
                .enumerate()
                .any(|(index, (path, _))| section_and_key(path, true).0 == section && !seen[index]);
            if missing {
                if !output.is_empty() && !output.ends_with(['\n', '\r']) {
                    output.push_str(newline);
                }
                output.push_str(&format!("[{section}]{newline}"));
                emit_missing(&fields, &mut seen, section, true, newline, &mut output);
            }
        }
    }
    ensure!(
        seen.iter().all(|value| *value),
        "ADAM+ input patch is incomplete"
    );
    Ok(output)
}

pub(crate) fn source_boundary() -> &'static str {
    "ADAM+ stores logical Qt/button codes but no physical-device identity; its pinned frontend always starts SimpleJoystick polling at runtime index 0. Re-enumerate and verify the caller-selected device in the same launch, and preserve the copied executable directory, settings.ini, ROM/disk/tape roots, BIOS overrides, and .sta state directory."
}

#[cfg(test)]
mod tests {
    use super::*;

    fn players() -> [PlayerMapping; 2] {
        [
            PlayerMapping {
                keys: [10; INPUT_COUNT],
            },
            PlayerMapping {
                keys: [20; INPUT_COUNT],
            },
        ]
    }

    #[test]
    fn patches_qsettings_input_and_preserves_unrelated_values() {
        let output = patch_config(
            b"[General]\nkeep=yes\n[controller]\njoystickType=2\n[input]\np1/0=1\n",
            1,
            players(),
        )
        .unwrap();
        assert!(output.contains("keep=yes\n"));
        assert!(output.contains("joystickType=1\n"));
        assert!(output.contains("p1/0=10\n"));
        assert!(output.contains("p2/17=20\n"));
    }

    #[test]
    fn accepts_nested_qsettings_sections_and_rejects_bad_type_or_duplicate() {
        let output = patch_config(b"[input/p1]\n0=1\n[input/p2]\n", 0, players()).unwrap();
        assert!(output.contains("[input/p1]\n0=10\n"));
        assert!(output.contains("[input/p2]\ntype=0\n0=20\n"));
        assert!(patch_config(b"", 3, players()).is_err());
        assert!(
            patch_config(
                b"[controller]\njoystickType=0\njoystickType=1\n",
                0,
                players()
            )
            .is_err()
        );
        assert!(patch_config(b"[input]\n[input/p1]\n", 0, players()).is_err());
    }

    #[test]
    fn completes_a_single_nested_section_and_handles_no_final_newline() {
        let output = patch_config(b"[input/p1]\nkeep=yes", 0, players()).unwrap();
        assert!(output.contains("keep=yes\ntype=0\n0=10\n"));
        assert!(output.contains("[input/p2]\ntype=0\n0=20\n"));
    }
}
