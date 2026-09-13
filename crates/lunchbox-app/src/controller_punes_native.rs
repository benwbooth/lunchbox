//! puNES standalone-native input.cfg writer.
//!
//! Pinned source: `punesemu/puNES`
//! `b1758ec26a3d6da10abf50dc184fa837236c5f61`.  The native Qt frontend
//! persists exact joystick GUIDs as `P?J GUID` and keyboard fallbacks as
//! `P?K ...`.  A GUID selects the source's compiled joystick database entry;
//! this writer therefore never fabricates per-button SDL mappings.

use anyhow::{Context, Result, ensure};
use std::collections::BTreeSet;

pub(crate) const SOURCE_COMMIT: &str = "b1758ec26a3d6da10abf50dc184fa837236c5f61";
pub(crate) const PROFILE_ID: &str = "punes:standalone-native-input-v1";
pub(crate) const KEY_CONTROLS: [&str; 10] = [
    "A", "B", "Select", "Start", "Up", "Down", "Left", "Right", "TurboA", "TurboB",
];

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct PortConfig {
    /// puNES ports are one-based and source supports 1 through 4.
    pub port: u8,
    /// `js_guid_to_string()` output, measured from the same native frontend.
    /// Both Windows and POSIX builds use `{8-4-4-4-12}` hexadecimal text.
    pub guid: String,
    /// Optional Qt key names. Empty means retain the baseline keyboard values.
    pub keyboard: std::collections::BTreeMap<String, String>,
}

fn valid_guid(guid: &str) -> Result<()> {
    let bytes = guid.as_bytes();
    ensure!(
        bytes.len() == 38 && bytes[0] == b'{' && bytes[37] == b'}',
        "puNES joystick GUID must use 38-character braced text"
    );
    for (index, byte) in bytes.iter().enumerate() {
        if matches!(index, 0 | 9 | 14 | 19 | 24 | 37) {
            continue;
        }
        ensure!(
            byte.is_ascii_hexdigit(),
            "puNES joystick GUID contains a non-hex character"
        );
    }
    for index in [9, 14, 19, 24] {
        ensure!(
            bytes[index] == b'-',
            "puNES joystick GUID has an invalid separator"
        );
    }
    Ok(())
}

fn valid_value(value: &str, what: &str) -> Result<()> {
    ensure!(
        !value.is_empty() && value.len() <= 128,
        "puNES {what} is empty or too long"
    );
    ensure!(
        !value
            .chars()
            .any(|ch| ch == '\n' || ch == '\r' || ch == '\0'),
        "puNES {what} contains a control character"
    );
    Ok(())
}

fn validate(config: &PortConfig) -> Result<()> {
    ensure!((1..=4).contains(&config.port), "puNES port must be 1..4");
    valid_guid(&config.guid)?;
    if config.keyboard.is_empty() {
        return Ok(());
    }
    ensure!(
        config.keyboard.len() == KEY_CONTROLS.len(),
        "puNES keyboard overlay must contain all ten controls"
    );
    let expected: BTreeSet<_> = KEY_CONTROLS.into_iter().map(str::to_owned).collect();
    ensure!(
        config.keyboard.keys().cloned().collect::<BTreeSet<_>>() == expected,
        "puNES keyboard overlay contains an unknown control"
    );
    for value in config.keyboard.values() {
        valid_value(value, "keyboard key")?;
    }
    Ok(())
}

fn rendered_lines(config: &PortConfig) -> Result<Vec<String>> {
    validate(config)?;
    let n = config.port;
    let mut lines = vec![
        format!("controller {n}=standard"),
        format!("pad {n} type=auto"),
        format!("P{n}J GUID={}", config.guid.to_ascii_uppercase()),
    ];
    for key in KEY_CONTROLS {
        if let Some(value) = config.keyboard.get(key) {
            lines.push(format!("P{n}K {key}={value}"));
        }
    }
    Ok(lines)
}

/// Render an isolated source-shaped `[port N]` block.
pub(crate) fn config_text(config: &PortConfig) -> Result<String> {
    let lines = rendered_lines(config)?;
    Ok(format!("[port {}]\n{}\n", config.port, lines.join("\n")))
}

fn header_name(line: &str) -> Option<&str> {
    let trimmed = line.trim();
    trimmed
        .strip_prefix('[')
        .and_then(|s| s.strip_suffix(']'))
        .map(str::trim)
}

fn owned_key(port: u8, key: &str) -> bool {
    key == format!("controller {port}")
        || key == format!("pad {port} type")
        || key == format!("P{port}J GUID")
        || KEY_CONTROLS
            .iter()
            .any(|control| key == format!("P{port}K {control}"))
}

/// Patch only selected `[port N]` fields while preserving comments, unrelated
/// settings, ROM/media paths, and all other port sections.
pub(crate) fn patch_input_cfg(baseline: &[u8], configs: &[PortConfig]) -> Result<String> {
    ensure!(
        baseline.len() <= 4 * 1024 * 1024,
        "puNES input.cfg is too large"
    );
    ensure!(
        !configs.is_empty() && configs.len() <= 4,
        "puNES port configuration count is invalid"
    );
    let mut ports = BTreeSet::new();
    let mut rendered = std::collections::BTreeMap::new();
    for config in configs {
        ensure!(ports.insert(config.port), "puNES port is duplicated");
        rendered.insert(config.port, rendered_lines(config)?);
    }
    let text = std::str::from_utf8(baseline).context("puNES input.cfg is not UTF-8")?;
    let newline = if text.contains("\r\n") { "\r\n" } else { "\n" };
    let mut output = String::new();
    let mut active = None;
    let mut inserted = BTreeSet::new();
    let mut insert_active = |port: u8, output: &mut String| {
        if inserted.insert(port) {
            if !output.is_empty() && !output.ends_with(newline) {
                output.push_str(newline);
            }
            for line in &rendered[&port] {
                output.push_str(line);
                output.push_str(newline);
            }
        }
    };
    for raw in text.split_inclusive('\n') {
        let line = raw
            .strip_suffix('\n')
            .unwrap_or(raw)
            .strip_suffix('\r')
            .unwrap_or(raw);
        if let Some(header) = header_name(line) {
            if let Some(port) = active.take() {
                insert_active(port, &mut output);
            }
            active = header
                .strip_prefix("port ")
                .and_then(|value| value.parse::<u8>().ok())
                .filter(|port| rendered.contains_key(port));
        }
        if active.is_some_and(|port| {
            raw.split_once('=')
                .is_some_and(|(key, _)| owned_key(port, key.trim()))
        }) {
            continue;
        }
        output.push_str(raw);
    }
    if let Some(port) = active {
        insert_active(port, &mut output);
    }
    for port in ports {
        if !inserted.contains(&port) {
            if !output.is_empty() {
                if !output.ends_with(newline) {
                    output.push_str(newline);
                }
                output.push_str(newline);
            }
            output.push_str(&format!("[port {port}]{newline}"));
            for line in &rendered[&port] {
                output.push_str(line);
                output.push_str(newline);
            }
        }
    }
    Ok(output)
}

pub(crate) fn source_boundary() -> &'static str {
    "Use the pinned puNES native executable and measured SDL joystick GUID. The GUID selects puNES's compiled jstick_db defaults; it is not a portable arbitrary per-control mapper. Preserve input.cfg/puNES.cfg, XDG data battery and save roots, FDS disksys.rom, and the native-versus-Flatpak boundary. A rendered config is not proof of startup or effective gameplay input."
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::BTreeMap;
    fn config(port: u8) -> PortConfig {
        PortConfig {
            port,
            guid: "{01234567-89AB-CDEF-0123-456789ABCDEF}".into(),
            keyboard: BTreeMap::new(),
        }
    }

    #[test]
    fn emits_guid_and_source_port_selection() {
        let text = config_text(&config(1)).unwrap();
        assert!(text.contains("[port 1]"));
        assert!(text.contains("controller 1=standard"));
        assert!(text.contains("P1J GUID={01234567-89AB-CDEF-0123-456789ABCDEF}"));
    }

    #[test]
    fn patches_guid_and_keyboard_without_touching_other_ports() {
        let mut c = config(1);
        c.keyboard = KEY_CONTROLS
            .into_iter()
            .map(|key| (key.into(), "K".into()))
            .collect();
        let baseline = b"[port 1]\ncontroller 1=disable\nP1J GUID=NULL\nP1K A=S\n[port 2]\ncontroller 2=disable\n";
        let text = patch_input_cfg(baseline, &[c]).unwrap();
        assert!(text.contains("controller 1=standard\n"));
        assert!(text.contains("P1K A=K\n"));
        assert!(text.contains("[port 2]\ncontroller 2=disable\n"));
        assert!(!text.contains("P1J GUID=NULL"));
    }

    #[test]
    fn rejects_bad_guid_partial_keyboard_and_duplicate_ports() {
        let mut bad = config(1);
        bad.guid = "NULL".into();
        assert!(config_text(&bad).is_err());
        let mut partial = config(1);
        partial.keyboard.insert("A".into(), "K".into());
        assert!(config_text(&partial).is_err());
        assert!(patch_input_cfg(b"", &[config(1), config(1)]).is_err());
    }
}
