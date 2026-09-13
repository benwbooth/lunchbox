//! Vita3K native SDL3 controller configuration.
//!
//! Pinned to Vita3K/Vita3K `84184a363aa99c7f331a7e75bdd75f43ff63db08`.
//! Vita3K stores the SDL gamepad button and axis enum values in the YAML
//! `controller-binds` and `controller-axis-binds` vectors.  `--config-location`
//! accepts a YAML file; `pref-path` keeps the installed Vita filesystem (and
//! therefore firmware and saves) separate from the staged config.

use anyhow::{Result, ensure};

pub(crate) const SOURCE_COMMIT: &str = "84184a363aa99c7f331a7e75bdd75f43ff63db08";
pub(crate) const PROFILE_ID: &str = "vita3k:standalone-vita";

/// Vita3K's `get_controller_bindings` order in ctrl/src/ctrl.cpp.
pub(crate) const BUTTON_KEYS: [&str; 15] = [
    "select", "start", "up", "right", "down", "left", "l", "r", "triangle", "circle", "cross",
    "square", "psbutton", "l3", "r3",
];

/// SDL3 Gamepad button values; caller supplies the measured SDL enum values.
pub(crate) fn config_yaml(
    controller_binds: &[i16],
    controller_axis_binds: &[i16],
    vita_fs_path: &str,
) -> Result<String> {
    ensure!(
        controller_binds.len() == BUTTON_KEYS.len(),
        "Vita3K requires all 15 controller button binds"
    );
    ensure!(
        controller_axis_binds.len() == 6,
        "Vita3K requires six controller axis binds"
    );
    ensure!(
        controller_binds
            .iter()
            .all(|value| (0..=31).contains(value)),
        "Vita3K SDL button enum is out of range"
    );
    ensure!(
        controller_axis_binds
            .iter()
            .all(|value| (0..=5).contains(value)),
        "Vita3K SDL axis enum is out of range"
    );
    ensure!(
        !vita_fs_path.is_empty() && !vita_fs_path.chars().any(char::is_control),
        "Vita3K Vita filesystem path is invalid"
    );
    let mut out = String::from("pref-path: \"");
    out.push_str(&vita_fs_path.replace('\\', "\\\\").replace('"', "\\\""));
    out.push_str("\"\ncontroller-binds:\n");
    for value in controller_binds {
        out.push_str(&format!("  - {value}\n"));
    }
    out.push_str("controller-axis-binds:\n");
    for value in controller_axis_binds {
        out.push_str(&format!("  - {value}\n"));
    }
    out.push_str("controller-analog-multiplier: 1.0\n");
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn emits_source_vector_order_and_data_root() {
        let text = config_yaml(&[0; 15], &[0, 1, 2, 3, 4, 5], "/srv/vita data").unwrap();
        assert!(text.starts_with("pref-path: \"/srv/vita data\""));
        assert_eq!(text.matches("  - 0\n").count(), 16);
        assert!(text.contains("controller-axis-binds:\n  - 0\n  - 1\n"));
    }
}
