//! Oricutron standalone SDL joystick-selection writer.
//!
//! Pinned source: pete-gordon/oricutron commit
//! 002279fce9fa756d1d63cdc40ae97939eb7de7ed. `main.c` parses
//! `joystick_a`, `joystick_b`, `telejoy_a`, and `telejoy_b` as `sdljoy0`
//! through `sdljoy9`; `joystick.c` opens the corresponding runtime SDL index.
//! The writer therefore accepts only a same-launch measured index and leaves
//! physical identity verification to the launch guard.

use anyhow::{Context, Result, ensure};
use std::collections::BTreeMap;

pub(crate) const SOURCE_COMMIT: &str = "002279fce9fa756d1d63cdc40ae97939eb7de7ed";

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum AtmosInterface {
    None,
    AltaiPase,
    Ijk,
}

impl AtmosInterface {
    fn value(self) -> &'static str {
        match self {
            Self::None => "none",
            Self::AltaiPase => "altai",
            Self::Ijk => "ijk",
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) struct ControllerSelection {
    pub atmos_interface: AtmosInterface,
    pub atmos_a: Option<u8>,
    pub atmos_b: Option<u8>,
    pub telestrat_a: Option<u8>,
    pub telestrat_b: Option<u8>,
}

fn selector(index: Option<u8>) -> Result<String> {
    match index {
        Some(index) => {
            ensure!(
                index <= 9,
                "Oricutron SDL joystick index must be 0 through 9"
            );
            Ok(format!("sdljoy{index}"))
        }
        None => Ok("none".into()),
    }
}

/// Patch all source-defined physical joystick selectors in a copied
/// `oricutron.cfg`. Keyboard joystick mappings, ROMs, media, autosave and
/// machine settings are preserved. Because Oricutron compares SDL event
/// instance numbers with the stored enumeration number, the launch layer must
/// re-probe and recheck each selected physical device immediately before
/// starting the exact executable.
pub(crate) fn patch_config(baseline: &[u8], selection: ControllerSelection) -> Result<String> {
    ensure!(
        baseline.len() <= 1024 * 1024,
        "Oricutron config is too large"
    );
    ensure!(
        selection.atmos_a.is_none()
            || selection.atmos_b.is_none()
            || selection.atmos_a != selection.atmos_b,
        "Oricutron Atmos ports cannot share one SDL runtime index"
    );
    ensure!(
        selection.telestrat_a.is_none()
            || selection.telestrat_b.is_none()
            || selection.telestrat_a != selection.telestrat_b,
        "Oricutron Telestrat ports cannot share one SDL runtime index"
    );
    ensure!(
        selection.atmos_interface != AtmosInterface::None
            || (selection.atmos_a.is_none() && selection.atmos_b.is_none()),
        "Oricutron Atmos controller requires an enabled joystick interface"
    );
    let fields = BTreeMap::from([
        ("joyinterface", selection.atmos_interface.value().to_owned()),
        ("joystick_a", selector(selection.atmos_a)?),
        ("joystick_b", selector(selection.atmos_b)?),
        ("telejoy_a", selector(selection.telestrat_a)?),
        ("telejoy_b", selector(selection.telestrat_b)?),
    ]);
    let original = std::str::from_utf8(baseline).context("Oricutron config is not UTF-8")?;
    ensure!(
        !original.contains('\0'),
        "Oricutron config contains a NUL byte"
    );
    let newline = if original.contains("\r\n") {
        "\r\n"
    } else {
        "\n"
    };
    let mut output = String::new();
    let mut found = BTreeMap::new();
    for line in original.split_inclusive('\n') {
        let owned = line.split_once('=').and_then(|(key, _)| {
            let key = key.trim();
            fields.get_key_value(key)
        });
        if let Some((key, value)) = owned {
            ensure!(
                !found.contains_key(key),
                "Oricutron config key is duplicated"
            );
            found.insert(key.clone(), ());
            output.push_str(&format!("{key} = {value}{newline}"));
        } else {
            output.push_str(line);
        }
    }
    for (key, value) in &fields {
        if !found.contains_key(key) {
            if !output.is_empty() && !output.ends_with('\n') {
                output.push_str(newline);
            }
            output.push_str(&format!("{key} = {value}{newline}"));
        }
    }
    Ok(output)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn writes_measured_slots_and_preserves_unrelated_config() {
        let output = patch_config(
            b"; controller\njoystick_a = kbjoy1\njoystick_b = none\ndiskautosave = yes\n",
            ControllerSelection {
                atmos_interface: AtmosInterface::Ijk,
                atmos_a: Some(2),
                atmos_b: Some(3),
                telestrat_a: None,
                telestrat_b: None,
            },
        )
        .unwrap();
        assert!(output.contains("joystick_a = sdljoy2\n"));
        assert!(output.contains("joystick_b = sdljoy3\n"));
        assert!(output.contains("joyinterface = ijk\n"));
        assert!(output.contains("diskautosave = yes\n"));
    }

    #[test]
    fn rejects_out_of_range_or_ambiguous_slots() {
        let mut selection = ControllerSelection {
            atmos_interface: AtmosInterface::AltaiPase,
            atmos_a: Some(10),
            atmos_b: None,
            telestrat_a: None,
            telestrat_b: None,
        };
        assert!(patch_config(b"", selection).is_err());
        selection.atmos_a = Some(1);
        selection.atmos_b = Some(1);
        assert!(patch_config(b"", selection).is_err());
    }
}
