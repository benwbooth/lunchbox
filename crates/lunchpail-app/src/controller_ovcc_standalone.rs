//! OVCC standalone-native Vcc.ini controller writer.
//!
//! Pinned source: `WallyZambotti/OVCC` at
//! `cc936b25be3da2c03b21a9bf1cc2ff4a494a5f09`.  `CoCo/config.c` reads and
//! writes the exact `Misc/KeyMapIndex`, `LeftJoyStick`, and `RightJoyStick`
//! INI entries.  Device numbers are SDL enumeration indices, so callers must
//! measure them in the same runtime before launching OVCC.

use anyhow::{Context, Result, ensure};

pub(crate) const SOURCE_COMMIT: &str = "cc936b25be3da2c03b21a9bf1cc2ff4a494a5f09";
pub(crate) const PROFILE_ID: &str = "ovcc:standalone-vcc-ini-input-v1";

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) struct JoystickInput {
    pub use_mouse: u8,
    /// AGAR scan-code values consumed by OVCC's SDL keyboard path.
    pub left: u16,
    pub right: u16,
    pub up: u16,
    pub down: u16,
    pub fire1: u16,
    pub fire2: u16,
    /// Zero-based SDL joystick enumeration index.
    pub device: u8,
    /// 0 is standard, 1 is Tandy hi-res, and 2 is CC-MAX.
    pub hires: u8,
}

fn fields(
    keymap: u8,
    left: JoystickInput,
    right: JoystickInput,
) -> Vec<(&'static str, String, String)> {
    let mut values = Vec::with_capacity(19);
    values.push(("Misc", "KeyMapIndex".to_owned(), keymap.to_string()));
    for (section, input) in [("LeftJoyStick", left), ("RightJoyStick", right)] {
        values.extend(
            [
                (section, "UseMouse".to_owned(), input.use_mouse.to_string()),
                (section, "Left".to_owned(), input.left.to_string()),
                (section, "Right".to_owned(), input.right.to_string()),
                (section, "Up".to_owned(), input.up.to_string()),
                (section, "Down".to_owned(), input.down.to_string()),
                (section, "Fire1".to_owned(), input.fire1.to_string()),
                (section, "Fire2".to_owned(), input.fire2.to_string()),
                (section, "DiDevice".to_owned(), input.device.to_string()),
                (section, "HiResDevice".to_owned(), input.hires.to_string()),
            ]
            .into_iter()
            .map(|(section, key, value)| (section, key, value)),
        );
    }
    values
}

fn patch_section(
    text: &str,
    section: &str,
    replacements: &[(String, String)],
    newline: &str,
) -> (String, bool) {
    let mut output = String::with_capacity(text.len() + replacements.len() * 20);
    let mut active = false;
    let mut found = false;
    let mut written = vec![false; replacements.len()];
    for raw in text.split_inclusive('\n') {
        let line = raw
            .strip_suffix('\n')
            .unwrap_or(raw)
            .strip_suffix('\r')
            .unwrap_or(raw);
        if let Some(header) = line
            .trim()
            .strip_prefix('[')
            .and_then(|s| s.strip_suffix(']'))
        {
            if active {
                for ((key, value), did_write) in replacements.iter().zip(&mut written) {
                    if !*did_write {
                        output.push_str(key);
                        output.push('=');
                        output.push_str(value);
                        output.push_str(newline);
                    }
                }
            }
            active = header.trim().eq_ignore_ascii_case(section);
            found |= active;
            output.push_str(raw);
            continue;
        }
        if active {
            if let Some((key, _)) = line.split_once('=') {
                if let Some(index) = replacements
                    .iter()
                    .position(|(candidate, _)| candidate.eq_ignore_ascii_case(key.trim()))
                {
                    output.push_str(&format!(
                        "{}={}{}",
                        replacements[index].0,
                        replacements[index].1,
                        if raw.ends_with("\r\n") {
                            "\r\n"
                        } else {
                            newline
                        }
                    ));
                    written[index] = true;
                    continue;
                }
            }
        }
        output.push_str(raw);
    }
    if active {
        if !output.ends_with(['\n', '\r']) {
            output.push_str(newline);
        }
        for ((key, value), did_write) in replacements.iter().zip(&mut written) {
            if !*did_write {
                output.push_str(key);
                output.push('=');
                output.push_str(value);
                output.push_str(newline);
            }
        }
    }
    (output, found)
}

/// Patch OVCC's source-defined joystick and keyboard entries while retaining
/// all other Vcc.ini text. Missing owned sections are appended exactly as the
/// source's INI writer would create them.
pub(crate) fn patch_config(
    baseline: &[u8],
    keymap: u8,
    left: JoystickInput,
    right: JoystickInput,
) -> Result<String> {
    ensure!(
        baseline.len() <= 4 * 1024 * 1024,
        "OVCC Vcc.ini is too large"
    );
    ensure!(keymap < 3, "OVCC keyboard layout must be 0, 1, or 2");
    ensure!(
        left.use_mouse <= 3 && right.use_mouse <= 3,
        "OVCC joystick input mode must be 0 (audio), 1 (joystick), 2 (mouse), or 3 (keyboard)"
    );
    ensure!(
        left.hires <= 2 && right.hires <= 2,
        "OVCC joystick emulation must be 0 (standard), 1 (Tandy hi-res), or 2 (CC-MAX)"
    );
    let source = std::str::from_utf8(baseline).context("OVCC Vcc.ini is not UTF-8")?;
    ensure!(!source.contains('\0'), "OVCC Vcc.ini contains a NUL byte");
    let newline = if source.contains("\r\n") {
        "\r\n"
    } else {
        "\n"
    };
    let all = fields(keymap, left, right);
    let mut output = source.to_owned();
    for section in ["Misc", "LeftJoyStick", "RightJoyStick"] {
        let replacements: Vec<_> = all
            .iter()
            .filter(|(s, _, _)| *s == section)
            .map(|(_, key, value)| (key.clone(), value.clone()))
            .collect();
        let (patched, found) = patch_section(&output, section, &replacements, newline);
        output = patched;
        if !found {
            if !output.is_empty() && !output.ends_with(['\n', '\r']) {
                output.push_str(newline);
            }
            output.push('[');
            output.push_str(section);
            output.push(']');
            output.push_str(newline);
            for (key, value) in replacements {
                output.push_str(&format!("{key}={value}{newline}"));
            }
        }
    }
    Ok(output)
}

pub(crate) fn source_boundary() -> &'static str {
    "OVCC writes AGAR scan-code values and SDL joystick enumeration indices to Vcc.ini; the device index is not a stable identity. Preserve Vcc.ini, ROMs, disk images, and module-library paths, and verify the same native frontend before launch."
}

#[cfg(test)]
mod tests {
    use super::*;

    const INPUT: JoystickInput = JoystickInput {
        use_mouse: 0,
        left: 1,
        right: 2,
        up: 3,
        down: 4,
        fire1: 5,
        fire2: 6,
        device: 1,
        hires: 0,
    };

    #[test]
    fn patches_source_ini_entries_and_preserves_unknowns() {
        let baseline =
            b"[Misc]\nKeyMapIndex=9\nkeep=yes\n[LeftJoyStick]\nLeft=99\n[Other]\nkeep=1\n";
        let output = patch_config(baseline, 2, INPUT, INPUT).unwrap();
        assert!(output.contains("[Misc]\nKeyMapIndex=2\nkeep=yes\n"));
        assert!(output.contains("[LeftJoyStick]\nLeft=1\n"));
        assert!(output.contains("Right=2\n"));
        assert!(output.contains("[RightJoyStick]\nUseMouse=0\n"));
        assert!(output.contains("[Other]\nkeep=1\n"));
    }

    #[test]
    fn accepts_source_defined_input_modes() {
        let modes = JoystickInput {
            use_mouse: 3,
            hires: 2,
            ..INPUT
        };
        let output = patch_config(b"", 0, modes, modes).unwrap();
        assert!(output.contains("UseMouse=3\n"));
        assert!(output.contains("HiResDevice=2\n"));
    }

    #[test]
    fn preserves_a_final_unterminated_owned_section_line() {
        let output = patch_config(b"[Misc]\nkeep=yes", 1, INPUT, INPUT).unwrap();
        assert!(output.starts_with("[Misc]\nkeep=yes\nKeyMapIndex=1\n"));
    }

    #[test]
    fn rejects_unsupported_layout_and_modes() {
        assert!(patch_config(b"", 3, INPUT, INPUT).is_err());
        let bad = JoystickInput {
            use_mouse: 4,
            ..INPUT
        };
        assert!(patch_config(b"", 0, bad, INPUT).is_err());
        let bad = JoystickInput { hires: 3, ..INPUT };
        assert!(patch_config(b"", 0, bad, INPUT).is_err());
    }
}
