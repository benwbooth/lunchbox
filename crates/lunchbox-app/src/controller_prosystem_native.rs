//! ProSystem 1.3 native Windows input overlay.
//!
//! Pinned source: gstanton/ProSystem1_3 commit
//! 3040295934c68024ecf89b45b50107ea1c271e66. `Win/Configuration.cpp`
//! reads and writes the WinAPI INI `[Input]` keys `Key0..Key16` and
//! `Device0..Device16`; `Win/Input.cpp` defines the first twelve entries as
//! controller 1/2 directions and fire buttons and entries 12..16 as console
//! switches. Device values are DirectInput enumeration values, so this
//! writer accepts only an explicitly captured value and never invents a
//! controller identity or axis/button layout.

use anyhow::{Context, Result, ensure};
use std::collections::BTreeSet;

pub(crate) const SOURCE_COMMIT: &str = "3040295934c68024ecf89b45b50107ea1c271e66";

/// One ProSystem `Input` array entry. `device == 0` means Keyboard in the
/// upstream UI; nonzero values are the DirectInput enumeration values shown
/// by that same UI. `key` is either a DirectInput keyboard scan code or one of
/// the `e_joy_value` values from `Win/Input.h`, as supplied by the caller.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) struct Mapping {
    pub index: u8,
    pub key: u8,
    pub device: u8,
}

fn validate(mapping: Mapping) -> Result<()> {
    ensure!(mapping.index < 17, "ProSystem input index must be 0..16");
    // Win/Input.h defines JOY_AXIS_UP through JOY_BUTTON_12 as 0..19. The
    // keyboard and device tables are byte-valued in the native program, so
    // the adapter's u8 fields represent the complete persisted domain.
    Ok(())
}

fn append_missing(
    output: &mut String,
    fields: &[(String, u8)],
    seen: &BTreeSet<String>,
    newline: &str,
) {
    for (name, value) in fields {
        if !seen.contains(name) {
            output.push_str(&format!("{name}={value}{newline}"));
        }
    }
}

/// Replace only explicitly supplied `KeyN`/`DeviceN` entries in the `[Input]`
/// section. Existing display, emulation, BIOS, database, recent-ROM and
/// save-path settings remain byte-for-byte represented; no other section is
/// modified. The caller must pass a copied session-local INI.
pub(crate) fn patch_input_section(baseline: &[u8], mappings: &[Mapping]) -> Result<String> {
    ensure!(baseline.len() <= 1024 * 1024, "ProSystem INI is too large");
    ensure!(
        !mappings.is_empty() && mappings.len() <= 17,
        "ProSystem mapping count is invalid"
    );
    let mut indices = BTreeSet::new();
    for mapping in mappings {
        validate(*mapping)?;
        ensure!(
            indices.insert(mapping.index),
            "ProSystem input index is duplicated"
        );
    }

    let text = std::str::from_utf8(baseline).context("ProSystem INI is not UTF-8")?;
    ensure!(!text.contains('\0'), "ProSystem INI contains a NUL byte");
    let newline = if text.contains("\r\n") { "\r\n" } else { "\n" };
    let mut ordered_mappings = mappings.to_vec();
    ordered_mappings.sort_by_key(|mapping| mapping.index);
    let fields: Vec<_> = ordered_mappings
        .into_iter()
        .flat_map(|mapping| {
            [
                (format!("Key{}", mapping.index), mapping.key),
                (format!("Device{}", mapping.index), mapping.device),
            ]
        })
        .collect();

    let mut output = String::new();
    let mut active = false;
    let mut found_input = false;
    let mut seen = BTreeSet::new();
    for raw in text.split_inclusive('\n') {
        let line = raw
            .strip_suffix('\n')
            .unwrap_or(raw)
            .strip_suffix('\r')
            .unwrap_or(raw);
        if let Some(header) = line
            .trim()
            .strip_prefix('[')
            .and_then(|value| value.strip_suffix(']'))
        {
            if active {
                append_missing(&mut output, &fields, &seen, newline);
            }
            let matching = header.trim().eq_ignore_ascii_case("Input");
            if matching {
                ensure!(!found_input, "ProSystem Input section is duplicated");
                found_input = true;
            }
            active = matching;
            output.push_str(raw);
            continue;
        }

        if active {
            if let Some((name, _)) = line.split_once('=') {
                let canonical = fields
                    .iter()
                    .find(|(known, _)| known.eq_ignore_ascii_case(name.trim()))
                    .map(|(known, _)| known.clone());
                if let Some(canonical) = canonical {
                    ensure!(
                        seen.insert(canonical.clone()),
                        "ProSystem input key is duplicated"
                    );
                    let value = fields
                        .iter()
                        .find(|(known, _)| *known == canonical)
                        .map(|(_, value)| value)
                        .expect("canonical field must have a value");
                    output.push_str(&format!("{canonical}={value}{newline}"));
                    continue;
                }
            }
        }
        output.push_str(raw);
    }

    if active {
        append_missing(&mut output, &fields, &seen, newline);
    }
    if !found_input {
        if !output.is_empty() && !output.ends_with(newline) {
            output.push_str(newline);
        }
        if !output.is_empty() {
            output.push_str(newline);
        }
        output.push_str(&format!("[Input]{newline}"));
        append_missing(&mut output, &fields, &seen, newline);
    }
    Ok(output)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn patches_measured_entries_and_preserves_other_sections() {
        let baseline = b"[Display]\nZoom=2\n[Input]\nKey0=203\nDevice0=0\nKey1=30\nDevice1=1\n[Console]\nSave.Path=C:\\\\states\n";
        let output = patch_input_section(
            baseline,
            &[Mapping {
                index: 0,
                key: 8,
                device: 1,
            }],
        )
        .unwrap();
        assert!(output.contains("Zoom=2\n"));
        assert!(output.contains("Key1=30\nDevice1=1\n"));
        assert!(output.contains("Key0=8\nDevice0=1\n"));
        assert!(!output.contains("Key0=203\n"));
        assert!(output.contains("Save.Path=C:\\\\states\n"));
    }

    #[test]
    fn rejects_duplicate_or_out_of_range_indices() {
        let duplicate = [
            Mapping {
                index: 0,
                key: 1,
                device: 0,
            },
            Mapping {
                index: 0,
                key: 2,
                device: 0,
            },
        ];
        assert!(patch_input_section(b"[Input]\n", &duplicate).is_err());
        assert!(
            patch_input_section(
                b"[Input]\n",
                &[Mapping {
                    index: 17,
                    key: 1,
                    device: 0
                }]
            )
            .is_err()
        );
    }

    #[test]
    fn patches_existing_input_without_duplicate_section_and_preserves_crlf() {
        let baseline = b"[Display]\r\nZoom=2\r\n[Input]\r\nKey0=203\r\nDevice0=0\r\nKey1=30\r\nDevice1=1\r\n[Console]\r\nSave.Path=C:\\\\states\r\n";
        let output = patch_input_section(
            baseline,
            &[Mapping {
                index: 0,
                key: 8,
                device: 1,
            }],
        )
        .unwrap();
        assert_eq!(output.matches("[Input]").count(), 1);
        assert!(output.contains("Key0=8\r\nDevice0=1\r\n"));
        assert!(output.contains("Key1=30\r\nDevice1=1\r\n"));
        assert!(output.contains("Save.Path=C:\\\\states\r\n"));
    }

    #[test]
    fn creates_input_section_when_baseline_has_none() {
        let output = patch_input_section(
            b"[Display]\nZoom=2\n",
            &[Mapping {
                index: 12,
                key: 14,
                device: 0,
            }],
        )
        .unwrap();
        assert!(output.ends_with("[Input]\nKey12=14\nDevice12=0\n"));
    }

    #[test]
    fn rejects_duplicate_input_sections() {
        assert!(
            patch_input_section(
                b"[Input]\n[Input]\n",
                &[Mapping {
                    index: 0,
                    key: 8,
                    device: 1,
                }]
            )
            .is_err()
        );
    }
}
