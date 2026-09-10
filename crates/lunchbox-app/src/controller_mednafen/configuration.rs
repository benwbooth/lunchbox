//! Flat native setting replacements; not RetroArch INI syntax.
use super::{Input, profiles::Gamepad};
use anyhow::{Result, ensure};
use std::collections::BTreeMap;

/// SettingsManager::ParseSettingLine requires an ASCII space after the key;
/// values are literal, including additional leading/trailing spaces.
pub(crate) fn assignments(text: &str) -> Result<BTreeMap<String, String>> {
    ensure!(
        text.len() <= 16 * 1024 * 1024 && !text.contains('\0'),
        "Mednafen configuration is oversized or contains NUL"
    );
    let mut result = BTreeMap::new();
    for line in text.trim_start_matches('\u{feff}').split('\n') {
        let line = line.strip_suffix('\r').unwrap_or(line);
        if line.starts_with([';', '#'])
            || line
                .bytes()
                .all(|byte| matches!(byte, b' ' | b'\t' | b'\r' | b'\n' | 0x0b | 0x0c))
        {
            continue;
        }
        let split = line.bytes().position(|byte| byte <= 0x20);
        let Some(split) = split else {
            anyhow::bail!("Mednafen setting has no space-delimited value");
        };
        ensure!(
            split > 0 && line.as_bytes()[split] == b' ',
            "Mednafen setting has invalid whitespace before its value"
        );
        result.insert(line[..split].to_owned(), line[split + 1..].to_owned());
    }
    Ok(result)
}

/// Merge layers in native load order before obtaining this value.
pub(crate) fn axis_threshold(effective: &BTreeMap<String, String>) -> Result<f64> {
    let value = effective
        .get("input.joystick.axis_threshold")
        .map_or(Ok(75.0), |value| value.trim().parse::<f64>())?;
    ensure!(
        value.is_finite() && (0.0..=100.0).contains(&value),
        "Mednafen effective joystick threshold is invalid"
    );
    Ok(value)
}

/// Append complete selected mappings to a captured native settings layer.
/// The launch owner must apply this to the effective override layers too;
/// writing the global file alone does not establish per-game precedence.
pub(crate) fn render(
    original: &str,
    gamepad: Gamepad,
    native_id: &str,
    controls: &BTreeMap<String, Input>,
) -> Result<String> {
    let assignments = gamepad.assignments(native_id, controls)?;
    render_assignments(original, &assignments)
}

/// Render a complete set assembled across native ports. Keeping this separate
/// from a single controller prevents later players from replacing earlier
/// players' private layers during multi-controller preparation.
pub(crate) fn render_assignments(
    original: &str,
    replacements: &BTreeMap<String, String>,
) -> Result<String> {
    assignments(original)?;
    ensure!(
        !replacements.is_empty(),
        "Mednafen needs native mapping assignments"
    );
    let mut result = original.to_owned();
    if !result.is_empty() && !result.ends_with('\n') {
        result.push('\n');
    }
    for (key, value) in replacements {
        ensure!(
            !key.is_empty()
                && !key.bytes().any(|byte| byte <= 0x20)
                && !key.starts_with([';', '#'])
                && !value.contains(['\r', '\n', '\0']),
            "Mednafen replacement is not a single native setting"
        );
        // Empty rapid-fire values intentionally clear their native bindings.
        result.push_str(&key);
        result.push(' ');
        result.push_str(&value);
        result.push('\n');
    }
    assignments(&result)?;
    Ok(result)
}
