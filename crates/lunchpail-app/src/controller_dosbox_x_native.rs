//! DOSBox-X mapper-file preservation writer.
//!
//! Pinned source: joncampbell123/dosbox-x
//! `532909c4e84160a5ac2185fbf9c4c97dbe07f85d`. `src/gui/sdl_mapper.cpp`
//! serializes named EVENT/BIND records in the same plain mapper grammar used
//! by SDL1 and SDL2 (`mapperfile`, `mapperfile_sdl1`, `mapperfile_sdl2`).

use anyhow::{Context, Result, ensure};
use std::collections::BTreeSet;

pub(crate) const SOURCE_COMMIT: &str = "532909c4e84160a5ac2185fbf9c4c97dbe07f85d";
pub(crate) const PROFILE_ID: &str = "dosbox-x:native-mapper-v1";

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct Event {
    /// Exact DOSBox-X mapper EVENT name supplied by the caller.
    pub name: String,
    /// Source `CBind::GetConfigName` strings, such as `key 30`,
    /// `stick_0 axis 0 1`, or `stick_0 button 0`.
    pub binds: Vec<String>,
}

fn valid_word(value: &str) -> bool {
    !value.is_empty() && value.len() <= 128 && !value.contains(['"', '\r', '\n'])
}

pub(crate) fn event_line(event: &Event) -> Result<String> {
    ensure!(
        valid_word(&event.name) && !event.name.contains(char::is_whitespace),
        "DOSBox-X event name is invalid"
    );
    ensure!(
        !event.binds.is_empty() && event.binds.len() <= 8,
        "DOSBox-X event needs one through eight binds"
    );
    for bind in &event.binds {
        ensure!(valid_word(bind), "DOSBox-X bind is invalid");
    }
    let mut out = event.name.clone();
    for bind in &event.binds {
        out.push_str(&format!(" \"{bind}\""));
    }
    out.push('\n');
    Ok(out)
}

/// Replace explicitly named EVENT records in a copied DOSBox-X mapper while
/// preserving unrelated events/comments and the selected mapper path.
pub(crate) fn patch_mapper(baseline: &[u8], events: &[Event]) -> Result<String> {
    ensure!(
        baseline.len() <= 4 * 1024 * 1024,
        "DOSBox-X mapper is too large"
    );
    ensure!(!events.is_empty(), "DOSBox-X needs at least one event");
    let text = std::str::from_utf8(baseline).context("DOSBox-X mapper is not UTF-8")?;
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
    "DOSBox-X mapper EVENT names and BIND tokens are exact source/runtime vocabulary supplied by the caller; preserve the baseline mapper and DOSBox-X config, mounted drives, disk images, -savedir and guest saves. Mapping serialization does not prove a DOS title consumes the event."
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn emits_and_replaces_mapper_event() {
        let event = Event {
            name: "jaxis_0_0+".into(),
            binds: vec!["stick_0 axis 0 1".into()],
        };
        assert_eq!(
            event_line(&event).unwrap(),
            "jaxis_0_0+ \"stick_0 axis 0 1\"\n"
        );
        let text = patch_mapper(b"# keep\njaxis_0_0+ \"key 30\"\n", &[event]).unwrap();
        assert!(text.starts_with("# keep\n"));
        assert!(text.contains("stick_0 axis 0 1"));
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
