//! Source-faithful B-em (Allegro) persistence helpers and joymap writer.

use anyhow::{Context, Result, ensure};
use std::{
    collections::{BTreeMap, BTreeSet},
    path::{Path, PathBuf},
};

pub(crate) const SOURCE_COMMIT: &str = "ccddf9fbe47cb18204f2c90f9504cbac6784b419";
pub(crate) const CONFIG_FILE: &str = "b-em/b-em.cfg";
pub(crate) const SNAPSHOT_EXTENSION: &str = "snp";
pub(crate) const SNAPSHOT_MAGIC: &[u8] = b"BEMSNAP";

/// Resolve the Linux config path using the same XDG fallback as B-em.
pub(crate) fn linux_config_path(xdg_config_home: Option<&Path>, home: &Path) -> PathBuf {
    xdg_config_home
        .map(Path::to_path_buf)
        .unwrap_or_else(|| home.join(".config"))
        .join(CONFIG_FILE)
}

/// B-em always writes the snapshot extension, even when the chooser omitted
/// one or supplied another extension.
pub(crate) fn snapshot_path(selected: &Path) -> PathBuf {
    selected.with_extension(SNAPSHOT_EXTENSION)
}

/// Recognise all currently loadable BEMSNAP revisions (1 through 3).
pub(crate) fn snapshot_revision(bytes: &[u8]) -> Option<u8> {
    if bytes.len() < SNAPSHOT_MAGIC.len() + 1 || !bytes.starts_with(SNAPSHOT_MAGIC) {
        return None;
    }
    let revision = bytes[SNAPSHOT_MAGIC.len()];
    (1..=3).contains(&revision).then_some(revision)
}

#[derive(Clone, Debug, PartialEq)]
pub(crate) struct AxisMapping {
    pub stick: u16,
    pub axis: u16,
    /// BBC ADC channel 0..4; zero explicitly disables analog routing.
    pub adc_channel: Option<u8>,
    /// Nonzero finite multiplier applied before the ADC/key thresholds.
    pub scale: Option<f32>,
    pub negative_key: Option<String>,
    pub positive_key: Option<String>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct ButtonMapping {
    pub button: u16,
    /// Raw source value consumed by `(value & button_mask) + 1`.
    pub emulated_button: Option<u8>,
    pub key: Option<String>,
}

#[derive(Clone, Debug, PartialEq)]
pub(crate) struct JoyMap {
    /// Exact `al_get_joystick_name` value measured for this launch.
    pub allegro_name: String,
    /// Measured number of axes in each Allegro stick.
    pub axis_counts: Vec<u16>,
    pub button_count: u16,
    pub axes: Vec<AxisMapping>,
    pub buttons: Vec<ButtonMapping>,
}

fn valid_name(value: &str, what: &str) -> Result<()> {
    ensure!(
        !value.trim().is_empty() && value.len() <= 256,
        "B-em {what} is empty or too long"
    );
    ensure!(
        !value.contains(['\0', '\r', '\n', '[', ']']),
        "B-em {what} contains a config delimiter"
    );
    Ok(())
}

fn fields(map: &JoyMap) -> Result<BTreeMap<String, String>> {
    valid_name(&map.allegro_name, "Allegro device name")?;
    ensure!(
        !map.axis_counts.is_empty() && map.axis_counts.len() <= 64,
        "B-em measured stick topology is invalid"
    );
    ensure!(map.button_count <= 256, "B-em button topology is too large");
    ensure!(
        !map.axes.is_empty() || !map.buttons.is_empty(),
        "B-em joymap needs at least one mapping"
    );
    let mut out = BTreeMap::new();
    let mut axes = BTreeSet::new();
    for axis in &map.axes {
        let stick = usize::from(axis.stick);
        ensure!(
            stick < map.axis_counts.len() && axis.axis < map.axis_counts[stick],
            "B-em axis is outside the measured Allegro topology"
        );
        ensure!(
            axes.insert((axis.stick, axis.axis)),
            "B-em axis mapping is duplicated"
        );
        ensure!(
            axis.adc_channel.is_some()
                || axis.negative_key.is_some()
                || axis.positive_key.is_some(),
            "B-em axis mapping has no target"
        );
        let prefix = format!("stick{}axis{}", axis.stick, axis.axis);
        if let Some(channel) = axis.adc_channel {
            ensure!(channel <= 4, "B-em ADC channel must be 0 through 4");
            out.insert(format!("{prefix}adc"), channel.to_string());
        }
        if let Some(scale) = axis.scale {
            ensure!(
                scale.is_finite() && scale != 0.0,
                "B-em axis scale must be finite and nonzero"
            );
            out.insert(format!("{prefix}scale"), scale.to_string());
        }
        if let Some(key) = &axis.negative_key {
            valid_name(key, "negative key name")?;
            out.insert(format!("{prefix}nkey"), key.clone());
        }
        if let Some(key) = &axis.positive_key {
            valid_name(key, "positive key name")?;
            out.insert(format!("{prefix}pkey"), key.clone());
        }
    }
    let mut buttons = BTreeSet::new();
    for button in &map.buttons {
        ensure!(
            button.button < map.button_count,
            "B-em button is outside the measured Allegro topology"
        );
        ensure!(
            buttons.insert(button.button),
            "B-em button mapping is duplicated"
        );
        ensure!(
            button.emulated_button.is_some() || button.key.is_some(),
            "B-em button mapping has no target"
        );
        if let Some(target) = button.emulated_button {
            out.insert(format!("button{}btn", button.button), target.to_string());
        }
        if let Some(key) = &button.key {
            valid_name(key, "button key name")?;
            out.insert(format!("button{}key", button.button), key.clone());
        }
    }
    Ok(out)
}

fn mapping_key(key: &str) -> bool {
    if let Some(rest) = key.strip_prefix("stick")
        && let Some((stick, rest)) = rest.split_once("axis")
    {
        let split = rest
            .find(|character: char| !character.is_ascii_digit())
            .unwrap_or(rest.len());
        let (axis, suffix) = rest.split_at(split);
        return !stick.is_empty()
            && stick.bytes().all(|byte| byte.is_ascii_digit())
            && !axis.is_empty()
            && axis.bytes().all(|byte| byte.is_ascii_digit())
            && matches!(suffix, "adc" | "scale" | "nkey" | "pkey");
    }
    if let Some(rest) = key.strip_prefix("button") {
        let split = rest
            .find(|character: char| !character.is_ascii_digit())
            .unwrap_or(rest.len());
        let (button, suffix) = rest.split_at(split);
        return !button.is_empty()
            && button.bytes().all(|byte| byte.is_ascii_digit())
            && matches!(suffix, "btn" | "key");
    }
    false
}

fn patch_one(original: &str, map: &JoyMap) -> Result<String> {
    let section = format!("joymap {}", map.allegro_name);
    let fields = fields(map)?;
    let newline = if original.contains("\r\n") {
        "\r\n"
    } else {
        "\n"
    };
    let mut output = String::new();
    let mut active = false;
    let mut found = false;
    for raw in original.split_inclusive('\n') {
        let line = raw.trim_end_matches(['\r', '\n']);
        if let Some(header) = line
            .trim()
            .strip_prefix('[')
            .and_then(|value| value.strip_suffix(']'))
        {
            if active {
                for (key, value) in &fields {
                    output.push_str(&format!("{key}={value}{newline}"));
                }
                active = false;
            }
            let matching = header.trim().eq_ignore_ascii_case(&section);
            if matching {
                ensure!(!found, "B-em joymap section is duplicated");
                found = true;
                active = true;
            }
            output.push_str(raw);
            continue;
        }
        if active
            && line
                .split_once('=')
                .is_some_and(|(key, _)| mapping_key(key.trim()))
        {
            continue;
        }
        output.push_str(raw);
    }
    if active {
        if !output.ends_with(['\n', '\r']) {
            output.push_str(newline);
        }
        for (key, value) in &fields {
            output.push_str(&format!("{key}={value}{newline}"));
        }
    } else if !found {
        if !output.is_empty() && !output.ends_with(['\n', '\r']) {
            output.push_str(newline);
        }
        output.push_str(&format!("[{section}]{newline}"));
        for (key, value) in &fields {
            output.push_str(&format!("{key}={value}{newline}"));
        }
    }
    Ok(output)
}

/// Patch complete source-shaped joymaps into a copied `b-em.cfg`. B-em still
/// opens only the first two live Allegro devices, so the launch layer must
/// verify their exact names and measured topology immediately before exec.
pub(crate) fn patch_joymaps(baseline: &[u8], maps: &[JoyMap]) -> Result<String> {
    ensure!(
        baseline.len() <= 4 * 1024 * 1024,
        "B-em config is too large"
    );
    ensure!(
        !maps.is_empty() && maps.len() <= 2,
        "B-em needs one or two joymaps"
    );
    let original = std::str::from_utf8(baseline).context("B-em config is not UTF-8")?;
    ensure!(!original.contains('\0'), "B-em config contains a NUL byte");
    let mut names = BTreeSet::new();
    let mut output = original.to_owned();
    for map in maps {
        ensure!(
            names.insert(map.allegro_name.to_ascii_lowercase()),
            "B-em Allegro device name is duplicated"
        );
        output = patch_one(&output, map)?;
    }
    Ok(output)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn snapshot_extension_is_normalized() {
        assert_eq!(
            snapshot_path(Path::new("slot.foo")),
            PathBuf::from("slot.snp")
        );
    }
    #[test]
    fn only_known_snapshot_revisions_are_accepted() {
        assert_eq!(snapshot_revision(b"BEMSNAP\x03rest"), Some(3));
        assert_eq!(snapshot_revision(b"BEMSNAP\x04rest"), None);
    }

    fn map() -> JoyMap {
        JoyMap {
            allegro_name: "Measured Pad".into(),
            axis_counts: vec![2],
            button_count: 2,
            axes: vec![
                AxisMapping {
                    stick: 0,
                    axis: 0,
                    adc_channel: Some(1),
                    scale: Some(-1.0),
                    negative_key: None,
                    positive_key: None,
                },
                AxisMapping {
                    stick: 0,
                    axis: 1,
                    adc_channel: Some(2),
                    scale: None,
                    negative_key: None,
                    positive_key: None,
                },
            ],
            buttons: vec![ButtonMapping {
                button: 0,
                emulated_button: Some(0),
                key: Some("Return".into()),
            }],
        }
    }

    #[test]
    fn patches_measured_joymap_and_removes_stale_owned_keys() {
        let output = patch_joymaps(
            b"[machine]\nname=keep\n[joymap Measured Pad]\nstick9axis9adc=4\nnote=keep\nbutton8key=Escape\n[next]\nvalue=yes\n",
            &[map()],
        )
        .unwrap();
        assert!(output.contains("[machine]\nname=keep\n"));
        assert!(output.contains("[joymap Measured Pad]\nnote=keep\n"));
        assert!(output.contains("stick0axis0adc=1\n"));
        assert!(output.contains("stick0axis0scale=-1\n"));
        assert!(output.contains("button0btn=0\nbutton0key=Return\n"));
        assert!(!output.contains("stick9axis9adc"));
        assert!(!output.contains("button8key"));
        assert!(output.contains("[next]\nvalue=yes\n"));
    }

    #[test]
    fn rejects_indices_outside_measured_topology() {
        let mut bad = map();
        bad.axes[0].axis = 2;
        assert!(patch_joymaps(b"", &[bad]).is_err());
    }

    #[test]
    fn rejects_duplicate_sections_and_device_names() {
        let baseline = b"[joymap Measured Pad]\n[joymap measured pad]\n";
        assert!(patch_joymaps(baseline, &[map()]).is_err());
        assert!(patch_joymaps(b"", &[map(), map()]).is_err());
    }
}
