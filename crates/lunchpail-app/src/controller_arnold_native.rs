//! Arnold Unix `.arnold` persistence contract.
//!
//! The Windows port persists settings in HKCU registry values, so this module
//! only exposes the portable key=value writer for the Unix port. Joystick
//! device identity remains runtime SDL state and is deliberately not invented.

use anyhow::{Result, ensure};
use std::path::{Path, PathBuf};

pub(crate) const CONFIG_FILE: &str = ".arnold";
pub(crate) const SOURCE_COMMIT: &str = "e5dce08964f94add100f7db992a6d0e49fe74f01";

pub(crate) fn config_path(home: &Path) -> PathBuf {
    home.join(CONFIG_FILE)
}

/// Update one Arnold key while preserving unknown settings and comments.
pub(crate) fn set_key(text: &str, key: &str, value: &str) -> Result<String> {
    ensure!(
        !key.is_empty()
            && key
                .bytes()
                .all(|byte| byte.is_ascii_alphanumeric() || byte == b'_'),
        "invalid Arnold key"
    );
    ensure!(
        !value.is_empty() && !value.contains(['\0', '\n', '\r']),
        "invalid Arnold value"
    );
    ensure!(
        key.len() + value.len() + 2 <= 1023,
        "Arnold key/value exceeds its source line buffer"
    );
    ensure!(text.len() <= 4 * 1024 * 1024, "Arnold config is too large");
    ensure!(!text.contains('\0'), "Arnold config contains a NUL byte");
    let mut found = false;
    let mut output = String::new();
    for line in text.lines() {
        if line
            .split_once('=')
            .is_some_and(|(name, _)| name.trim() == key)
        {
            if !found {
                output.push_str(key);
                output.push('=');
                output.push_str(value);
                output.push('\n');
                found = true;
            }
        } else {
            output.push_str(line);
            output.push('\n');
        }
    }
    if !found {
        output.push_str(key);
        output.push('=');
        output.push_str(value);
        output.push('\n');
    }
    Ok(output)
}

pub(crate) fn controller_refusal() -> Result<()> {
    anyhow::bail!(
        "Arnold does not persist physical controller identity or mappings: the pinned Unix SDL frontend opens runtime indices 0 and 1, while the Windows frontend enumerates runtime WinMM ids"
    )
}

pub(crate) fn snapshot_path(path: &Path) -> PathBuf {
    path.with_extension("sna")
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn key_writer_preserves_unknown_lines() {
        let out = set_key("rom1=old\ncustom=keep\nrom1=stale", "rom1", "/tmp/new.rom").unwrap();
        assert!(out.contains("rom1=/tmp/new.rom"));
        assert!(out.contains("custom=keep"));
        assert_eq!(out.matches("rom1=").count(), 1);
        assert!(controller_refusal().is_err());
    }
}
