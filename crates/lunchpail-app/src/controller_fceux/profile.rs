//! Private FCEUX Qt native NES profile text, with explicit alternate banks.
use super::Input;
use anyhow::{Result, ensure};
use std::collections::{BTreeMap, BTreeSet};

pub(crate) const CONTROLS: [&str; 8] = [
    "a", "b", "back", "start", "dpup", "dpdown", "dpleft", "dpright",
];

pub(crate) fn render(guid: &str, name: &str, bindings: &BTreeMap<String, Input>) -> Result<String> {
    ensure!(
        guid.len() == 32 && guid.bytes().all(|byte| byte.is_ascii_hexdigit()),
        "FCEUX needs the native SDL joystick GUID, not a device name"
    );
    ensure!(
        !name.is_empty()
            && name.len() <= 48
            && name
                .bytes()
                .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'_' | b'-')),
        "FCEUX profile name must be a short filename-safe basename"
    );
    ensure!(
        bindings.len() == CONTROLS.len() && CONTROLS.iter().all(|key| bindings.contains_key(*key)),
        "FCEUX standard NES profile requires all eight controls"
    );
    let mut owners = BTreeSet::new();
    for input in bindings.values() {
        ensure!(
            owners.insert(input.packed()?),
            "FCEUX native input has competing gameplay owners"
        );
    }
    let mut result = String::new();
    for bank in 0..4 {
        // Fields are comma-delimited and config must precede the button fields.
        // A trailing comma is required by native parseMapping's field count.
        let mut line = format!("{guid},{name},config:{bank},");
        for key in CONTROLS {
            let value = if bank == 0 {
                bindings[key].as_setting()?
            } else {
                String::new()
            };
            line.push_str(&format!("{key}:{value},"));
        }
        line.push_str("turboA:,turboB:,\n");
        // getMapFromFile reads fgets into 256 bytes; never split one logical
        // record into independently parsed fragments in that fixed buffer.
        ensure!(
            line.len() <= 255,
            "FCEUX profile record exceeds native line buffer"
        );
        result.push_str(&line);
    }
    // No copied per-profile hotkeys: the caller stages a new session profile,
    // not an edit of the user's original profile or keyboard shortcut registry.
    Ok(result)
}
