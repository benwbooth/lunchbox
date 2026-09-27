//! Mu (Palm emulator) Qt key-binding writer.
//!
//! Pinned source: meepingsnesroms/Mu@4ac406874ccdc33ca3282299fda412f15ec544ad.
//! The desktop Qt frontend uses QSettings IniFormat at ~/MuCfg.txt and stores
//! eleven Palm hardware-button bindings as palmButton0Key..palmButton10Key.

use anyhow::{Context, Result, ensure};

pub(crate) const SOURCE_COMMIT: &str = "4ac406874ccdc33ca3282299fda412f15ec544ad";
pub(crate) const PROFILE_ID: &str = "mu:qt-palm-key-bindings-v1";
const BUTTONS: usize = 11;

/// Patch the exact QSettings keys used by Mu's SettingsManager. Unknown keys,
/// comments, ordering and line endings are retained; a complete baseline is
/// required so QSettings defaults cannot silently replace a requested bind.
pub(crate) fn patch_config(baseline: &[u8], keycodes: [i32; BUTTONS]) -> Result<String> {
    let text = std::str::from_utf8(baseline).context("Mu config is not UTF-8")?;
    ensure!(!text.contains('\0'), "Mu config contains a NUL byte");
    ensure!(
        keycodes.iter().all(|key| *key >= 0),
        "Mu keycodes must be nonnegative"
    );
    let newline = if text.contains("\r\n") { "\r\n" } else { "\n" };
    let mut out = String::with_capacity(text.len() + 256);
    let mut seen = [false; BUTTONS];
    for raw in text.split_inclusive('\n') {
        let line = raw
            .strip_suffix('\n')
            .unwrap_or(raw)
            .strip_suffix('\r')
            .unwrap_or(raw);
        let found = line.split_once('=').and_then(|(k, _)| {
            let k = k.trim();
            (0..BUTTONS).find(|i| k.eq_ignore_ascii_case(&format!("palmButton{i}Key")))
        });
        if let Some(index) = found {
            ensure!(!seen[index], "Mu config contains duplicate palm button key");
            out.push_str(&format!(
                "palmButton{index}Key={}{newline}",
                keycodes[index]
            ));
            seen[index] = true;
        } else {
            out.push_str(raw);
        }
    }
    ensure!(
        seen.iter().all(|v| *v),
        "Mu config is missing a Palm button key"
    );
    Ok(out)
}

pub(crate) fn source_boundary() -> &'static str {
    "Mu's native desktop profile stores Palm button-to-host-key Qt keycodes in ~/MuCfg.txt; it is keyboard binding, not a gamepad identity contract. Preserve ROM/RAM/SD roots and named state directories, and verify the executable reads the isolated QSettings file before claiming runtime success."
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn patches_all_source_button_keys() {
        let mut baseline = String::from("unknown=yes\n");
        for i in 0..BUTTONS {
            baseline.push_str(&format!("palmButton{i}Key=0\n"));
        }
        let out = patch_config(baseline.as_bytes(), [1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11]).unwrap();
        assert!(out.contains("palmButton0Key=1\n"));
        assert!(out.contains("palmButton10Key=11\n"));
        assert!(out.contains("unknown=yes\n"));
    }
    #[test]
    fn rejects_incomplete_or_negative_profiles() {
        assert!(patch_config(b"palmButton0Key=0\n", [0; BUTTONS]).is_err());
        let mut baseline = String::new();
        for i in 0..BUTTONS {
            baseline.push_str(&format!("palmButton{i}Key=0\n"));
        }
        let mut keys = [0; BUTTONS];
        keys[3] = -1;
        assert!(patch_config(baseline.as_bytes(), keys).is_err());
    }
}
