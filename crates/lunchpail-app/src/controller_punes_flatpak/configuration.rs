//! Source-shaped configuration for the audited puNES 0.111 Flatpak.

use anyhow::{Context, Result, ensure};
use std::collections::{BTreeMap, BTreeSet};

pub(crate) const CONTROLS: [(&str, &str); 8] = [
    ("a", "A"),
    ("b", "B"),
    ("select", "Select"),
    ("start", "Start"),
    ("up", "Up"),
    ("down", "Down"),
    ("left", "Left"),
    ("right", "Right"),
];

const JSC_VALUES: [(&str, &str); 10] = [
    ("A", "BTN01"),
    ("B", "BTN02"),
    ("Select", "BTN11"),
    ("Start", "BTN12"),
    ("Up", "BTN16"),
    ("Down", "BTN17"),
    ("Left", "BTN18"),
    ("Right", "BTN19"),
    ("TurboA", "NULL"),
    ("TurboB", "NULL"),
];

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct Pad {
    pub(crate) player: u8,
    pub(crate) guid: String,
}

fn valid_guid(guid: &str) -> Result<()> {
    let bytes = guid.as_bytes();
    ensure!(
        bytes.len() == 38 && bytes.first() == Some(&b'{') && bytes.last() == Some(&b'}'),
        "puNES joystick GUID must use 38-character braced text"
    );
    for (index, byte) in bytes.iter().enumerate() {
        if matches!(index, 0 | 37) {
            continue;
        }
        if matches!(index, 9 | 14 | 19 | 24) {
            ensure!(*byte == b'-', "puNES joystick GUID separator is invalid");
        } else {
            ensure!(
                byte.is_ascii_hexdigit(),
                "puNES joystick GUID contains a non-hex character"
            );
        }
    }
    Ok(())
}

pub(crate) fn validate_pads(pads: &[Pad]) -> Result<()> {
    ensure!(
        matches!(pads.len(), 1 | 2),
        "puNES standard-pad mode needs one or two players"
    );
    let mut guids = BTreeSet::new();
    for (index, pad) in pads.iter().enumerate() {
        ensure!(
            usize::from(pad.player) == index + 1,
            "puNES players must be contiguous from player one"
        );
        valid_guid(&pad.guid)?;
        ensure!(
            guids.insert(pad.guid.to_ascii_uppercase()),
            "puNES players need distinct target GUIDs"
        );
    }
    Ok(())
}

fn section(line: &str) -> Option<String> {
    line.trim()
        .strip_prefix('[')
        .and_then(|value| value.strip_suffix(']'))
        .map(|value| value.trim().to_ascii_lowercase())
}

fn key(line: &str) -> Option<String> {
    let trimmed = line.trim();
    (!trimmed.starts_with('#'))
        .then(|| {
            trimmed
                .split_once('=')
                .map(|(key, _)| key.trim().to_owned())
        })
        .flatten()
}

fn render_sections(
    baseline: &[u8],
    replacements: &BTreeMap<String, BTreeMap<String, String>>,
) -> Result<String> {
    ensure!(
        baseline.len() <= 4 * 1024 * 1024,
        "puNES config is too large"
    );
    let text = std::str::from_utf8(baseline).context("puNES config is not UTF-8")?;
    ensure!(!text.contains('\0'), "puNES config contains NUL");
    let newline = if text.contains("\r\n") { "\r\n" } else { "\n" };
    let mut output = String::new();
    let mut current = None::<String>;
    let mut emitted = BTreeSet::<(String, String)>::new();
    let emit_missing =
        |name: &str, output: &mut String, emitted: &mut BTreeSet<(String, String)>| {
            for (key, value) in &replacements[name] {
                if emitted.insert((name.to_owned(), key.to_owned())) {
                    output.push_str(key);
                    output.push('=');
                    output.push_str(value);
                    output.push_str(newline);
                }
            }
        };
    for raw in text.split_inclusive('\n') {
        ensure!(
            raw.len() <= 64 * 1024,
            "puNES config contains an oversized line"
        );
        if let Some(name) = section(raw.trim_end_matches(['\r', '\n'])) {
            if let Some(previous) = current.take()
                && replacements.contains_key(&previous)
            {
                emit_missing(&previous, &mut output, &mut emitted);
            }
            current = Some(name);
        }
        let owned = current
            .as_ref()
            .and_then(|name| replacements.get(name))
            .and_then(|fields| key(raw).and_then(|key| fields.get_key_value(&key)))
            .map(|(key, value)| (key.to_owned(), value.to_owned()));
        if let Some((key, value)) = owned {
            let name = current.as_ref().unwrap();
            if emitted.insert((name.clone(), key.clone())) {
                output.push_str(&key);
                output.push('=');
                output.push_str(&value);
                output.push_str(newline);
            }
        } else {
            output.push_str(raw);
        }
    }
    if let Some(previous) = current
        && replacements.contains_key(&previous)
    {
        emit_missing(&previous, &mut output, &mut emitted);
    }
    let missing = replacements
        .keys()
        .filter(|name| {
            replacements[*name]
                .keys()
                .any(|key| !emitted.contains(&(name.to_string(), key.to_owned())))
        })
        .cloned()
        .collect::<Vec<_>>();
    for name in &missing {
        if !output.is_empty() && !output.ends_with(newline) {
            output.push_str(newline);
        }
        if !output.is_empty() {
            output.push_str(newline);
        }
        output.push('[');
        output.push_str(name);
        output.push(']');
        output.push_str(newline);
        emit_missing(name, &mut output, &mut emitted);
    }
    Ok(output)
}

/// Preserve the user's main config except for process-isolation behavior and
/// features that can inject external firmware into ordinary cartridge runs.
pub(crate) fn render_main(baseline: &[u8]) -> Result<String> {
    render_sections(
        baseline,
        &BTreeMap::from([
            (
                "system".into(),
                BTreeMap::from([
                    ("cheat mode".into(), "disabled".into()),
                    ("game genie rom file".into(), String::new()),
                    ("save settings on exit".into(), "no".into()),
                ]),
            ),
            (
                "gui".into(),
                BTreeMap::from([(
                    "allow multiple instances of the emulator".into(),
                    "yes".into(),
                )]),
            ),
        ]),
    )
}

/// Select only the session-owned GUIDs and ordinary NES controller topology.
pub(crate) fn render_input(baseline: &[u8], pads: &[Pad]) -> Result<String> {
    validate_pads(pads)?;
    let mut replacements = BTreeMap::from([
        (
            "shortcuts".into(),
            BTreeMap::from([
                ("joystick GUID".into(), "NULL".into()),
                ("save state".into(), "F1,NULL".into()),
                ("load state".into(), "F4,NULL".into()),
            ]),
        ),
        (
            "expansion port".into(),
            BTreeMap::from([("expansion port".into(), "standard".into())]),
        ),
        (
            "system".into(),
            BTreeMap::from([
                ("controller mode".into(), "nes".into()),
                ("permit up+down left+right".into(), "no".into()),
            ]),
        ),
    ]);
    for player in 1..=4u8 {
        let pad = pads.iter().find(|pad| pad.player == player);
        replacements.insert(
            format!("port {player}"),
            BTreeMap::from([
                (
                    format!("controller {player}"),
                    if pad.is_some() { "standard" } else { "disable" }.into(),
                ),
                (format!("pad {player} type"), "auto".into()),
                (
                    format!("P{player}J GUID"),
                    pad.map_or_else(|| "NULL".into(), |pad| pad.guid.to_ascii_uppercase()),
                ),
            ]),
        );
    }
    render_sections(baseline, &replacements)
}

pub(crate) fn jsc_file_name(guid: &str) -> Result<String> {
    valid_guid(guid)?;
    Ok(guid
        .chars()
        .filter(|ch| ch.is_ascii_hexdigit())
        .flat_map(char::to_uppercase)
        .collect::<String>()
        + ".jsc")
}

/// Explicitly bind all ordinary controls. This avoids depending on the
/// compiled fallback database for a project-owned virtual VID/PID.
pub(crate) fn render_jsc() -> String {
    let mut text = String::from("[standard controller]\n");
    for (key, value) in JSC_VALUES {
        text.push_str(key);
        text.push('=');
        text.push_str(value);
        text.push('\n');
    }
    text.push_str(
        "\n\n[system]\nDeadzone=40\nButtons enabled=0xFFFFFFFFFFFFFFFF\nAxes enabled=0xFFFFFF\n",
    );
    text
}

#[cfg(test)]
mod tests {
    use super::*;

    fn pads() -> Vec<Pad> {
        vec![
            Pad {
                player: 1,
                guid: "{FE12FF9C-1209-1141-4C51-4B250001FE71}".into(),
            },
            Pad {
                player: 2,
                guid: "{FE12FF9C-1209-1141-4C52-4B260001FE71}".into(),
            },
        ]
    }

    #[test]
    fn private_configs_preserve_unowned_fields_and_force_topology() {
        let input = b"[shortcuts]\nopen=Alt+O,NULL\njoystick GUID={BAD}\n\n[port 1]\ncontroller 1=disable\nP1J GUID=NULL\nP1K A=S\n\n[port 3]\ncontroller 3=standard\nP3J GUID={OTHER}\n\n[unrelated]\npath=/keep/me\n";
        let rendered = render_input(input, &pads()).unwrap();
        assert!(rendered.contains("open=Alt+O,NULL"));
        assert!(rendered.contains("P1K A=S"));
        assert!(rendered.contains("path=/keep/me"));
        assert!(rendered.contains("controller 1=standard"));
        assert!(rendered.contains("P1J GUID={FE12FF9C-1209-1141-4C51-4B250001FE71}"));
        assert!(rendered.contains("controller 3=disable"));
        assert!(rendered.contains("P3J GUID=NULL"));
        assert!(rendered.contains("controller mode=nes"));
        assert!(rendered.contains("expansion port=standard"));
    }

    #[test]
    fn main_and_jsc_are_source_shaped() {
        let main = render_main(b"[system]\npreferred mode=auto\ncheat mode=gamegenie\ngame genie rom file=/untrusted/gamegenie.rom\n\n[GUI]\nallow multiple instances of the emulator=no\n").unwrap();
        assert!(main.contains("preferred mode=auto"));
        assert!(main.contains("cheat mode=disabled"));
        assert!(main.contains("game genie rom file=\n"));
        assert!(!main.contains("/untrusted/gamegenie.rom"));
        assert!(main.contains("save settings on exit=no"));
        assert!(main.contains("allow multiple instances of the emulator=yes"));
        assert_eq!(
            jsc_file_name(&pads()[0].guid).unwrap(),
            "FE12FF9C120911414C514B250001FE71.jsc"
        );
        let jsc = render_jsc();
        for field in [
            "A=BTN01",
            "B=BTN02",
            "Up=BTN16",
            "Right=BTN19",
            "TurboA=NULL",
        ] {
            assert!(jsc.contains(field));
        }
        assert_eq!(
            jsc,
            "[standard controller]\n\
A=BTN01\n\
B=BTN02\n\
Select=BTN11\n\
Start=BTN12\n\
Up=BTN16\n\
Down=BTN17\n\
Left=BTN18\n\
Right=BTN19\n\
TurboA=NULL\n\
TurboB=NULL\n\
\n\
\n\
[system]\n\
Deadzone=40\n\
Buttons enabled=0xFFFFFFFFFFFFFFFF\n\
Axes enabled=0xFFFFFF\n"
        );
    }

    #[test]
    fn main_rewrites_canonical_keys_in_place_without_whitespace_drift() {
        let baseline = b"[system]\n\
# possible values: disabled, gamegenie, cheatslist\n\
cheat mode=gamegenie\n\
\n\
# possible values: [PATH/NAME]\n\
game genie rom file=/untrusted/gamegenie.rom\n\
\n\
# possible values: yes, no\n\
save settings on exit=yes\n\
\n\
\n\
[GUI]\n\
# possible values: yes, no\n\
allow multiple instances of the emulator=no\n\
\n\
\n";
        let expected = "[system]\n\
# possible values: disabled, gamegenie, cheatslist\n\
cheat mode=disabled\n\
\n\
# possible values: [PATH/NAME]\n\
game genie rom file=\n\
\n\
# possible values: yes, no\n\
save settings on exit=no\n\
\n\
\n\
[GUI]\n\
# possible values: yes, no\n\
allow multiple instances of the emulator=yes\n\
\n\
\n";
        let rendered = render_main(baseline).unwrap();
        assert_eq!(rendered, expected);
        assert_eq!(render_main(rendered.as_bytes()).unwrap(), expected);
    }

    #[test]
    fn rejects_non_contiguous_or_duplicate_pads() {
        let mut value = pads();
        value[1].player = 1;
        assert!(validate_pads(&value).is_err());
        let mut value = pads();
        value[1].guid = value[0].guid.clone();
        assert!(validate_pads(&value).is_err());
        assert!(render_main(b"[system]\nvalue=bad\0tail\n").is_err());
        let oversized = vec![b'x'; 64 * 1024 + 1];
        assert!(render_main(&oversized).is_err());
    }
}
