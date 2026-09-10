//! Remove only selected input-type overrides from a private MAME cfg copy.
//! Caller supplies exact Pn_JOYSTICK_*, Pn_BUTTONn, STARTn and COINn types.
use anyhow::{Result, ensure};
use quick_xml::{Reader, Writer, events::Event};
use std::collections::BTreeSet;

pub(crate) fn without_panel_overrides(source: &[u8], types: &BTreeSet<String>) -> Result<Vec<u8>> {
    ensure!(
        source.len() <= 16 * 1024 * 1024,
        "MAME config exceeds preparation limit"
    );
    std::str::from_utf8(source)?;
    let mut reader = Reader::from_reader(source);
    let mut writer = Writer::new(Vec::with_capacity(source.len()));
    let mut stack: Vec<Vec<u8>> = Vec::new();
    let mut skip_depth: Option<usize> = None;
    let mut roots = 0;
    loop {
        let event = reader.read_event()?;
        match &event {
            Event::Start(element) | Event::Empty(element) => {
                let name = element.name().as_ref().to_vec();
                if stack.is_empty() {
                    roots += 1;
                    ensure!(
                        roots == 1 && name == b"mameconfig",
                        "Expected one MAME configuration root"
                    );
                    let version = element
                        .try_get_attribute("version")?
                        .ok_or_else(|| anyhow::anyhow!("MAME configuration version is missing"))?;
                    ensure!(
                        version.unescape_value()?.as_ref() == "10",
                        "Unsupported MAME configuration version"
                    );
                }
                let at_port = stack.len() == 3
                    && stack[0] == b"mameconfig"
                    && stack[1] == b"system"
                    && stack[2] == b"input"
                    && name == b"port";
                let selected = if at_port {
                    element
                        .try_get_attribute("type")?
                        .map(|attribute| {
                            attribute
                                .unescape_value()
                                .map(|value| types.contains(value.as_ref()))
                        })
                        .transpose()?
                        .unwrap_or(false)
                } else {
                    false
                };
                if skip_depth.is_none() && !selected {
                    writer.write_event(event.clone())?;
                }
                if matches!(event, Event::Start(_)) {
                    stack.push(name);
                    ensure!(
                        stack.len() <= 128,
                        "MAME config nesting exceeds preparation limit"
                    );
                    if selected && skip_depth.is_none() {
                        skip_depth = Some(stack.len());
                    }
                }
            }
            Event::End(_) => {
                ensure!(!stack.is_empty(), "Unbalanced MAME configuration");
                if skip_depth.is_none() {
                    writer.write_event(event.clone())?;
                }
                if skip_depth == Some(stack.len()) {
                    skip_depth = None;
                }
                stack.pop();
            }
            Event::DocType(_) => anyhow::bail!("MAME configuration DTD is unsupported"),
            Event::Eof => {
                ensure!(
                    roots == 1 && stack.is_empty() && skip_depth.is_none(),
                    "Incomplete MAME configuration"
                );
                break;
            }
            Event::Text(text) if stack.is_empty() => {
                ensure!(
                    text.unescape()?.trim().is_empty(),
                    "Text outside MAME configuration root"
                );
                writer.write_event(event.clone())?;
            }
            _ => {
                if skip_depth.is_none() {
                    writer.write_event(event.clone())?;
                }
            }
        }
    }
    Ok(writer.into_inner())
}
