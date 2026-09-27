//! DOSBox Staging mapper-file preservation writer.
//!
//! Pinned source: dosbox-staging/dosbox-staging
//! `d9135010ea56c2faa0bb8062aaf30a8541bf22f3`. `src/gui/mapper.cpp` writes
//! one line per source event: `event_name "BIND" "BIND"`. Event names are
//! deliberately caller-supplied because DOS programs choose their own guest
//! controls.

use anyhow::{Context, Result, ensure};
use std::collections::BTreeSet;

pub(crate) const SOURCE_COMMIT: &str = "d9135010ea56c2faa0bb8062aaf30a8541bf22f3";
pub(crate) const PROFILE_ID: &str = "dosbox-staging:native-mapper-v1";

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct Event {
    /// Exact mapper event name discovered from the same DOSBox build/game.
    pub name: String,
    /// Source `CBind::GetConfigName` strings, e.g. `key 30` or
    /// `stick_0 button 6`.
    pub binds: Vec<String>,
}

fn valid_word(value: &str) -> bool {
    !value.is_empty() && value.len() <= 128 && !value.contains(['"', '\r', '\n'])
}

pub(crate) fn event_line(event: &Event) -> Result<String> {
    ensure!(
        valid_word(&event.name) && !event.name.contains(char::is_whitespace),
        "DOSBox Staging event name is invalid"
    );
    ensure!(
        !event.binds.is_empty() && event.binds.len() <= 8,
        "DOSBox Staging event needs one through eight binds"
    );
    for bind in &event.binds {
        ensure!(valid_word(bind), "DOSBox Staging bind is invalid");
    }
    let mut out = event.name.clone();
    for bind in &event.binds {
        out.push_str(&format!(" \"{bind}\""));
    }
    out.push('\n');
    Ok(out)
}

/// Replace explicit event records in a copied mapper file and preserve all
/// unrelated event lines/comments. The caller must provide exact event names.
pub(crate) fn patch_mapper(baseline: &[u8], events: &[Event]) -> Result<String> {
    ensure!(
        baseline.len() <= 4 * 1024 * 1024,
        "DOSBox Staging mapper is too large"
    );
    ensure!(
        !events.is_empty(),
        "DOSBox Staging needs at least one event"
    );
    let text = std::str::from_utf8(baseline).context("DOSBox Staging mapper is not UTF-8")?;
    let names = events
        .iter()
        .map(|event| {
            let _ = event_line(event)?;
            Ok(event.name.to_ascii_lowercase())
        })
        .collect::<Result<BTreeSet<_>>>()?;
    let mut lines = Vec::new();
    for line in text.lines() {
        let name = line.split_whitespace().next().map(str::to_ascii_lowercase);
        if name.is_none() || !names.contains(&name.unwrap()) {
            lines.push(line);
        }
    }
    let mut out = lines.join("\n");
    if !out.is_empty() {
        out.push('\n');
    }
    for event in events {
        out.push_str(&event_line(event)?);
    }
    Ok(out)
}

pub(crate) fn source_boundary() -> &'static str {
    "DOSBox Staging mapper event names and bind tokens are exact source/runtime vocabulary supplied by the caller; preserve the baseline mapper and DOSBox config, mounted drives, disk images and guest saves. Mapping serialization does not prove a DOS title consumes the event."
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn emits_and_replaces_mapper_event() {
        let event = Event {
            name: "jbutton_0_0".into(),
            binds: vec!["stick_0 button 6".into()],
        };
        assert_eq!(
            event_line(&event).unwrap(),
            "jbutton_0_0 \"stick_0 button 6\"\n"
        );
        let text = patch_mapper(b"# keep\njbutton_0_0 \"key 30\"\n", &[event]).unwrap();
        assert!(text.starts_with("# keep\n"));
        assert!(text.contains("stick_0 button 6"));
        assert!(!text.contains("key 30"));
    }
    #[test]
    fn rejects_untrusted_names() {
        assert!(
            event_line(&Event {
                name: "bad name".into(),
                binds: vec!["key 1".into()]
            })
            .is_err()
        );
    }
}
