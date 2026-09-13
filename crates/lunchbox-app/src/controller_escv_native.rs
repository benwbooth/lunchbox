//! eSCV's Win32 joystick persistence format.
//!
//! Pinned to the official Common Source Code Project `source.7z` archive
//! (`sha256=63d598a29cabf2d9212bd8a1699d8bcdddfc2416e711c20581a0d665c96f2399`).
//! The eSCV Win32 build reads/writes `scv.ini` in its current directory and
//! serializes `Input/JoyButtonsEx{1..8}_{1..16}` as signed integer values.
//! Values are either negative Win32 virtual-key codes or `(joystick << 5) |
//! button/axis-code`.  This module only patches a complete source-generated
//! INI; physical joystick identity remains a same-launch WinMM measurement.

use anyhow::{Context, Result, ensure};

pub(crate) const SOURCE_ARCHIVE_SHA256: &str =
    "63d598a29cabf2d9212bd8a1699d8bcdddfc2416e711c20581a0d665c96f2399";
pub(crate) const PROFILE_ID: &str = "escv:win32-joystick-v1";

/// The source writes eight rows of sixteen assignments, even though eSCV's
/// runtime update loop consumes only joystick indices 0 through 3.  Keep the
/// full native shape so unknown rows are not silently discarded.
pub(crate) const JOYSTICK_ROWS: usize = 8;
pub(crate) const BUTTONS_PER_ROW: usize = 16;

fn field(row: usize, button: usize) -> String {
    format!("JoyButtonsEx{}_{}", row + 1, button + 1)
}

fn valid_binding(value: i32) -> bool {
    // Negative values are Win32 virtual-key codes.  Positive values encode a
    // joystick number and one of the source's 32 button/axis/HAT positions.
    // eSCV's update_joystick() only polls four joystick rows.
    (-(255)..=-1).contains(&value) || (0..=127).contains(&value)
}

/// Replace all source-defined joystick assignments while retaining comments,
/// unknown INI fields, ordering, and line endings.  Requiring every field
/// prevents accidentally treating a hand-written partial file as the
/// executable's persisted schema.
pub(crate) fn patch_config(
    baseline: &[u8],
    bindings: [[i32; BUTTONS_PER_ROW]; JOYSTICK_ROWS],
) -> Result<String> {
    ensure!(
        baseline.len() <= 4 * 1024 * 1024,
        "eSCV config is too large"
    );
    let original = std::str::from_utf8(baseline).context("eSCV config is not UTF-8")?;
    ensure!(!original.contains('\0'), "eSCV config contains a NUL byte");
    ensure!(
        original
            .lines()
            .any(|line| line.trim().eq_ignore_ascii_case("[Input]")),
        "eSCV config is missing the source-defined [Input] section"
    );
    for row in bindings {
        for value in row {
            ensure!(
                valid_binding(value),
                "eSCV joystick binding is outside Win32/source range"
            );
        }
    }

    let expected: Vec<String> = (0..JOYSTICK_ROWS)
        .flat_map(|row| (0..BUTTONS_PER_ROW).map(move |button| field(row, button)))
        .collect();
    let mut seen = vec![false; expected.len()];
    let mut output = String::with_capacity(original.len() + expected.len() * 8);
    let mut in_input = false;

    for line in original.split_inclusive('\n') {
        let (content, ending) = line.strip_suffix('\n').map_or((line, ""), |line| {
            line.strip_suffix('\r')
                .map_or((line, "\n"), |line| (line, "\r\n"))
        });
        let trimmed = content.trim();
        if trimmed.starts_with('[') && trimmed.ends_with(']') {
            in_input = trimmed.eq_ignore_ascii_case("[Input]");
        }
        if in_input {
            if let Some((key, _)) = content.split_once('=') {
                if let Some(index) = expected
                    .iter()
                    .position(|candidate| candidate.eq_ignore_ascii_case(key.trim()))
                {
                    ensure!(
                        !seen[index],
                        "eSCV config contains duplicate {}",
                        expected[index]
                    );
                    let row = index / BUTTONS_PER_ROW;
                    let button = index % BUTTONS_PER_ROW;
                    output.push_str(&expected[index]);
                    output.push('=');
                    output.push_str(&bindings[row][button].to_string());
                    output.push_str(ending);
                    seen[index] = true;
                    continue;
                }
            }
        }
        output.push_str(line);
    }

    ensure!(
        seen.iter().all(|value| *value),
        "eSCV config is missing JoyButtonsEx fields"
    );
    Ok(output)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn baseline() -> String {
        let mut text = String::from("[Input]\r\nunknown=keep\r\n");
        for row in 0..JOYSTICK_ROWS {
            for button in 0..BUTTONS_PER_ROW {
                text.push_str(&format!("JoyButtonsEx{}_{}=0\r\n", row + 1, button + 1));
            }
        }
        text.push_str("[Other]\r\nvalue=preserved\r\n");
        text
    }

    #[test]
    fn patches_source_shape_and_keeps_unknown_ini() {
        let mut bindings = [[0i32; BUTTONS_PER_ROW]; JOYSTICK_ROWS];
        bindings[0][0] = 0x21;
        bindings[1][3] = -0x41;
        let result = patch_config(baseline().as_bytes(), bindings).unwrap();
        assert!(result.contains("unknown=keep\r\n"));
        assert!(result.contains("JoyButtonsEx1_1=33\r\n"));
        assert!(result.contains("JoyButtonsEx2_4=-65\r\n"));
        assert!(result.contains("value=preserved\r\n"));
    }

    #[test]
    fn rejects_partial_or_out_of_range_config() {
        let mut bindings = [[0i32; BUTTONS_PER_ROW]; JOYSTICK_ROWS];
        bindings[0][0] = -256;
        assert!(patch_config(baseline().as_bytes(), bindings).is_err());
        assert!(
            patch_config(
                b"[Input]\nJoyButtonsEx1_1=0\n",
                [[0; BUTTONS_PER_ROW]; JOYSTICK_ROWS]
            )
            .is_err()
        );
    }
}
