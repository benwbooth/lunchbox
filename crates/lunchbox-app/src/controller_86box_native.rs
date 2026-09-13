//! 86Box source-shaped input-device configuration writer.
//!
//! Pinned source: 86Box/86Box `189d9d003ad9670853cec6edac8db7d6ff63550f`.
//! `src/config.c` loads and saves the `[Input devices]` section. Joystick
//! topology is selected by `joystick_type`; each emulated joystick slot uses
//! `joystick_N_nr`, `joystick_N_axis_M`, `joystick_N_button_M`, and
//! `joystick_N_pov_M` (the POV value is `x, y`).

use anyhow::{Context, Result, ensure};

pub(crate) const SOURCE_COMMIT: &str = "189d9d003ad9670853cec6edac8db7d6ff63550f";
pub(crate) const PROFILE_ID: &str = "86box:native-input-devices-v1";

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct Joystick {
    /// Emulated joystick slot, zero-based; topology determines its meaning.
    pub slot: u8,
    /// Native platform joystick number, one-based (`0` means absent in 86Box).
    pub host_number: u8,
    pub axis_mapping: Vec<i32>,
    pub button_mapping: Vec<i32>,
    pub pov_mapping: Vec<(i32, i32)>,
}

fn valid_name(name: &str) -> bool {
    !name.is_empty()
        && name.len() <= 64
        && name
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || b"_-".contains(&b))
}

/// Patch the source-defined `[Input devices]` keys in a copied 86box.cfg.
/// Existing text and unrelated settings are retained. The caller supplies
/// machine topology and measured host joystick numbering explicitly.
pub(crate) fn patch_config(
    baseline: &[u8],
    joystick_type: &str,
    joysticks: &[Joystick],
) -> Result<String> {
    ensure!(
        baseline.len() <= 4 * 1024 * 1024,
        "86Box config is too large"
    );
    ensure!(valid_name(joystick_type), "86Box joystick type is invalid");
    ensure!(
        !joysticks.is_empty() && joysticks.len() <= 16,
        "86Box needs one through sixteen joystick slots"
    );
    let text = std::str::from_utf8(baseline).context("86Box config is not UTF-8")?;
    ensure!(!text.contains('\0'), "86Box config contains a NUL byte");
    let mut seen = [false; 16];
    let newline = if text.contains("\r\n") { "\r\n" } else { "\n" };
    let mut fields = vec![("joystick_type".to_owned(), joystick_type.to_owned())];
    for joystick in joysticks {
        ensure!(
            usize::from(joystick.slot) < seen.len(),
            "86Box joystick slot is out of range"
        );
        ensure!(
            !seen[usize::from(joystick.slot)],
            "86Box joystick slot is duplicated"
        );
        seen[usize::from(joystick.slot)] = true;
        ensure!(
            joystick.host_number > 0,
            "86Box host joystick number is one-based and nonzero"
        );
        ensure!(
            joystick.axis_mapping.len() <= 16
                && joystick.button_mapping.len() <= 32
                && joystick.pov_mapping.len() <= 8,
            "86Box joystick topology exceeds source limits"
        );
        let n = joystick.slot;
        fields.push((format!("joystick_{n}_nr"), joystick.host_number.to_string()));
        for (index, value) in joystick.axis_mapping.iter().enumerate() {
            fields.push((format!("joystick_{n}_axis_{index}"), value.to_string()));
        }
        for (index, value) in joystick.button_mapping.iter().enumerate() {
            fields.push((format!("joystick_{n}_button_{index}"), value.to_string()));
        }
        for (index, (x, y)) in joystick.pov_mapping.iter().enumerate() {
            fields.push((format!("joystick_{n}_pov_{index}"), format!("{x}, {y}")));
        }
    }

    let append_fields = |out: &mut String| {
        for (key, value) in &fields {
            out.push_str(&format!("{key}={value}{newline}"));
        }
    };
    let mut out = String::new();
    let mut active = false;
    let mut found = false;
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
                append_fields(&mut out);
                active = false;
            }
            if header.trim().eq_ignore_ascii_case("Input devices") {
                ensure!(!found, "86Box config has duplicate Input devices sections");
                found = true;
                active = true;
            }
            out.push_str(raw);
            continue;
        }
        if active {
            let owned = line.split_once('=').is_some_and(|(key, _)| {
                let key = key.trim();
                key.eq_ignore_ascii_case("joystick_type")
                    || key
                        .get(..9)
                        .is_some_and(|prefix| prefix.eq_ignore_ascii_case("joystick_"))
            });
            if owned {
                continue;
            }
        }
        out.push_str(raw);
    }
    if active {
        if !out.is_empty() && !out.ends_with(['\n', '\r']) {
            out.push_str(newline);
        }
        append_fields(&mut out);
    } else if !found {
        if !out.is_empty() && !out.ends_with(['\n', '\r']) {
            out.push_str(newline);
        }
        out.push_str(&format!("[Input devices]{newline}"));
        append_fields(&mut out);
    }
    Ok(out)
}

pub(crate) fn source_boundary() -> &'static str {
    "86Box joystick_N_nr is a measured one-based host SDL/raw-input slot and joystick_N_* indices are machine-topology dependent. Preserve the copied machine config, ROM directory, disk images, and guest-save roots; a writer does not establish that the selected guest program reads the device."
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn emits_input_devices_and_pov_grammar() {
        let text = patch_config(
            b"[Machine]\nromset=keep\n[Input devices]\nscancode_1=2\njoystick_type=old\njoystick_4_nr=9\n[Other]\nkeep=yes\n",
            "2axis_2button",
            &[Joystick {
                slot: 0,
                host_number: 2,
                axis_mapping: vec![1, 0],
                button_mapping: vec![3, 4],
                pov_mapping: vec![(0, 1)],
            }],
        )
        .unwrap();
        assert!(text.contains("[Machine]\nromset=keep"));
        assert!(text.contains("[Input devices]\nscancode_1=2\njoystick_type=2axis_2button\njoystick_0_nr=2\njoystick_0_axis_0=1"));
        assert!(text.contains("joystick_0_pov_0=0, 1\n[Other]\nkeep=yes"));
        assert!(!text.contains("joystick_type=old"));
        assert!(!text.contains("joystick_4_nr=9"));
    }
    #[test]
    fn rejects_zero_host_or_duplicate_slots() {
        let bad = Joystick {
            slot: 0,
            host_number: 0,
            axis_mapping: vec![],
            button_mapping: vec![],
            pov_mapping: vec![],
        };
        assert!(patch_config(b"", "2axis_2button", &[bad]).is_err());
        let one = Joystick {
            slot: 0,
            host_number: 1,
            axis_mapping: vec![],
            button_mapping: vec![],
            pov_mapping: vec![],
        };
        assert!(patch_config(b"", "2axis_2button", &[one.clone(), one]).is_err());
        assert!(
            patch_config(
                b"[Input devices]\n[Input devices]\n",
                "2axis_2button",
                &[Joystick {
                    slot: 0,
                    host_number: 1,
                    axis_mapping: vec![],
                    button_mapping: vec![],
                    pov_mapping: vec![],
                }]
            )
            .is_err()
        );
    }
}
