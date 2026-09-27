//! Nuance Resurrection controller mapping writer.
//!
//! Pinned source: andkrau/NuanceResurrection@700d28553f3a06b16385ed9e48f194377e131b03.
//! nuance.cfg defines [Controller1Mappings] with source-owned action names
//! and KEY_/JOYBUT_/JOYAXIS_/JOYPOV_ binding values.

use anyhow::{Context, Result, ensure};
use std::collections::BTreeMap;

pub(crate) const SOURCE_COMMIT: &str = "700d28553f3a06b16385ed9e48f194377e131b03";
const ACTIONS: [&str; 14] = [
    "CPAD_UP",
    "CPAD_RIGHT",
    "CPAD_DOWN",
    "CPAD_LEFT",
    "A",
    "B",
    "L",
    "R",
    "DPAD_UP",
    "DPAD_RIGHT",
    "DPAD_DOWN",
    "DPAD_LEFT",
    "NUON",
    "START",
];

/// Patch a complete copied nuance.cfg, preserving unrelated sections and
/// comments. Values must use the source's KEY_/JOYBUT_/JOYAXIS_/JOYPOV_ form.
pub(crate) fn patch_config(baseline: &[u8], bindings: &BTreeMap<String, String>) -> Result<String> {
    let text = std::str::from_utf8(baseline).context("Nuance config is not UTF-8")?;
    ensure!(!text.contains('\0'), "Nuance config contains a NUL byte");
    ensure!(
        bindings.len() == ACTIONS.len(),
        "Nuance profile must contain every Controller1Mappings action"
    );
    for action in ACTIONS {
        let value = bindings
            .get(action)
            .context("Nuance profile is missing an action")?;
        ensure!(
            value.starts_with("KEY_")
                || value.starts_with("JOYBUT_")
                || value.starts_with("JOYAXIS_")
                || value.starts_with("JOYPOV_"),
            "Nuance binding has an unknown source prefix"
        );
    }
    let nl = if text.contains("\r\n") { "\r\n" } else { "\n" };
    let mut out = String::with_capacity(text.len() + 256);
    let mut active = false;
    let mut seen = BTreeMap::new();
    for raw in text.split_inclusive('\n') {
        let line = raw
            .strip_suffix('\n')
            .unwrap_or(raw)
            .strip_suffix('\r')
            .unwrap_or(raw);
        if let Some(sec) = line
            .trim()
            .strip_prefix('[')
            .and_then(|x| x.strip_suffix(']'))
        {
            active = sec.eq_ignore_ascii_case("Controller1Mappings");
            out.push_str(raw);
            continue;
        }
        if active {
            if let Some((key, _)) = line.split_once('=') {
                let key = key.trim();
                if ACTIONS.iter().any(|a| a.eq_ignore_ascii_case(key)) {
                    let action = ACTIONS
                        .iter()
                        .find(|a| a.eq_ignore_ascii_case(key))
                        .unwrap();
                    ensure!(
                        !seen.insert((*action).to_owned(), true).is_some(),
                        "Nuance config has duplicate action"
                    );
                    out.push_str(&format!("{action} = {}{nl}", bindings[*action]));
                    continue;
                }
            }
        }
        out.push_str(raw);
    }
    ensure!(
        seen.len() == ACTIONS.len(),
        "Nuance config is missing Controller1Mappings actions"
    );
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;
    fn binds() -> BTreeMap<String, String> {
        ACTIONS
            .into_iter()
            .map(|a| (a.to_owned(), "JOYBUT_0_0".to_owned()))
            .collect()
    }
    #[test]
    fn patches() {
        let mut b = String::from("[Controller1Mappings]\n");
        for a in ACTIONS {
            b.push_str(a);
            b.push_str(" = KEY_1_0\n");
        }
        let o = patch_config(b.as_bytes(), &binds()).unwrap();
        assert!(o.contains("CPAD_UP = JOYBUT_0_0\n"));
    }
    #[test]
    fn rejects_missing() {
        assert!(patch_config(b"[Controller1Mappings]\nA=KEY_1_0\n", &binds()).is_err());
    }
}
