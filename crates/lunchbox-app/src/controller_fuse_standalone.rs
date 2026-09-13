//! Fuse standalone physical-joystick logical mapping writer.
//!
//! Pinned source: fuse-emulator/fuse commit
//! 5ba7804a44483466d7403a6e646a228da562ed5d. `settings.dat` and
//! `settings.pl` define both key/value and libxml2 XML spellings such as
//! `joystick1output` and `joystick1fire1`; the SDL2 frontend opens runtime
//! slots 0 and 1 and maps their axes/hats plus 15 buttons. The physical slots
//! cannot be reordered in config, so they must be measured for the exact
//! process immediately before launch.

use anyhow::{Context, Result, ensure};
use quick_xml::{
    Reader, Writer,
    events::{BytesEnd, BytesStart, BytesText, Event},
};
use std::collections::{BTreeMap, BTreeSet};

pub(crate) const SOURCE_COMMIT: &str = "5ba7804a44483466d7403a6e646a228da562ed5d";
pub(crate) const PROFILE_ID: &str = "fuse:standalone-native-fixed-sdl-v1";

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
#[repr(u8)]
pub(crate) enum JoystickType {
    Cursor = 1,
    Kempston = 2,
    Sinclair1 = 3,
    Sinclair2 = 4,
    Timex1 = 5,
    Timex2 = 6,
    Fuller = 7,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum FireTarget {
    None,
    Space,
    Digit(u8),
    Letter(char),
    Enter,
    CapsShift,
    SymbolShift,
    JoystickFire,
}

impl FireTarget {
    fn value(self) -> Result<u16> {
        match self {
            Self::None => Ok(0),
            Self::Space => Ok(0x20),
            Self::Digit(digit) => {
                ensure!(digit <= 9, "Fuse Spectrum digit must be 0 through 9");
                Ok(u16::from(b'0' + digit))
            }
            Self::Letter(letter) => {
                ensure!(
                    letter.is_ascii_alphabetic(),
                    "Fuse Spectrum letter is invalid"
                );
                Ok(letter.to_ascii_lowercase() as u16)
            }
            Self::Enter => Ok(0x100),
            Self::CapsShift => Ok(0x101),
            Self::SymbolShift => Ok(0x102),
            Self::JoystickFire => Ok(0x1000),
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct PortProfile {
    /// The pinned SDL frontend recognizes only physical slots 0 and 1.
    pub physical_slot: u8,
    pub output: JoystickType,
    /// Physical buttons 1 through 15 in source order.
    pub fire_targets: [FireTarget; 15],
}

fn fields(profiles: &[PortProfile]) -> Result<BTreeMap<String, String>> {
    ensure!(
        !profiles.is_empty() && profiles.len() <= 2,
        "Fuse needs one or two physical joystick profiles"
    );
    let mut slots = BTreeSet::new();
    let mut fields = BTreeMap::new();
    for profile in profiles {
        ensure!(profile.physical_slot <= 1, "Fuse SDL slot must be 0 or 1");
        ensure!(
            slots.insert(profile.physical_slot),
            "Fuse SDL slot is duplicated"
        );
        let number = profile.physical_slot + 1;
        fields.insert(
            format!("joystick{number}output"),
            (profile.output as u8).to_string(),
        );
        for (index, target) in profile.fire_targets.iter().enumerate() {
            fields.insert(
                format!("joystick{number}fire{}", index + 1),
                target.value()?.to_string(),
            );
        }
    }
    Ok(fields)
}

/// Patch a copied Fuse `fuserc`/`fuse.cfg`, supporting both source-defined
/// key/value and libxml2 `<settings>` encodings. ROM, media, UI, machine and
/// snapshot settings remain untouched.
pub(crate) fn patch_config(baseline: &[u8], profiles: &[PortProfile]) -> Result<String> {
    ensure!(
        baseline.len() <= 4 * 1024 * 1024,
        "Fuse config is too large"
    );
    let original = std::str::from_utf8(baseline).context("Fuse config is not UTF-8")?;
    ensure!(!original.contains('\0'), "Fuse config contains a NUL byte");
    let fields = fields(profiles)?;
    if original.trim_start().starts_with('<') {
        patch_xml(original, &fields)
    } else {
        patch_key_value(original, &fields)
    }
}

fn patch_key_value(original: &str, fields: &BTreeMap<String, String>) -> Result<String> {
    let newline = if original.contains("\r\n") {
        "\r\n"
    } else {
        "\n"
    };
    let mut output = String::new();
    let mut seen = BTreeSet::new();
    for line in original.split_inclusive('\n') {
        let owned = line.split_once('=').and_then(|(key, _)| {
            fields
                .keys()
                .find(|known| key.trim() == known.as_str())
                .cloned()
        });
        if let Some(key) = owned {
            ensure!(seen.insert(key.clone()), "Fuse config key is duplicated");
            output.push_str(&format!("{key}={}{newline}", fields[&key]));
        } else {
            output.push_str(line);
        }
    }
    if !output.is_empty() && !output.ends_with('\n') {
        output.push_str(newline);
    }
    for (key, value) in fields {
        if !seen.contains(key) {
            output.push_str(&format!("{key}={value}{newline}"));
        }
    }
    Ok(output)
}

fn patch_xml(original: &str, fields: &BTreeMap<String, String>) -> Result<String> {
    let mut reader = Reader::from_str(original);
    let mut writer = Writer::new(Vec::with_capacity(original.len() + fields.len() * 40));
    let mut stack: Vec<Vec<u8>> = Vec::new();
    let mut roots = 0;
    let mut seen = BTreeSet::new();
    loop {
        let event = reader.read_event()?;
        match event {
            Event::Start(ref element) => {
                let name = element.name().as_ref().to_vec();
                if stack.is_empty() {
                    roots += 1;
                    ensure!(
                        roots == 1 && name == b"settings",
                        "Fuse XML root must be settings"
                    );
                }
                stack.push(name);
                ensure!(stack.len() <= 8, "Fuse XML nesting is too deep");
                writer.write_event(event.into_owned())?;
            }
            Event::Empty(ref element) if stack.len() == 1 && stack[0] == b"settings" => {
                let key = std::str::from_utf8(element.name().as_ref())?.to_owned();
                if let Some(value) = fields.get(&key) {
                    ensure!(seen.insert(key.clone()), "Fuse XML key is duplicated");
                    writer.write_event(Event::Start(BytesStart::new(&key)))?;
                    writer.write_event(Event::Text(BytesText::new(value)))?;
                    writer.write_event(Event::End(BytesEnd::new(&key)))?;
                } else {
                    writer.write_event(event.into_owned())?;
                }
            }
            Event::Text(_) if stack.len() == 2 && stack[0] == b"settings" => {
                let key = std::str::from_utf8(&stack[1])?;
                if let Some(value) = fields.get(key) {
                    ensure!(seen.insert(key.to_owned()), "Fuse XML key is duplicated");
                    writer.write_event(Event::Text(BytesText::new(value)))?;
                } else {
                    writer.write_event(event.into_owned())?;
                }
            }
            Event::End(ref element) => {
                ensure!(
                    stack
                        .last()
                        .is_some_and(|name| name == element.name().as_ref()),
                    "Fuse XML is unbalanced"
                );
                if stack.len() == 1 && stack[0] == b"settings" {
                    for (key, value) in fields {
                        if !seen.contains(key) {
                            writer.write_event(Event::Start(BytesStart::new(key)))?;
                            writer.write_event(Event::Text(BytesText::new(value)))?;
                            writer.write_event(Event::End(BytesEnd::new(key)))?;
                        }
                    }
                }
                writer.write_event(event.into_owned())?;
                stack.pop();
            }
            Event::DocType(_) => anyhow::bail!("Fuse XML DTD is unsupported"),
            Event::Eof => {
                ensure!(roots == 1 && stack.is_empty(), "Fuse XML is incomplete");
                break;
            }
            _ => writer.write_event(event.into_owned())?,
        }
    }
    String::from_utf8(writer.into_inner()).context("Fuse XML output is not UTF-8")
}

pub(crate) fn source_boundary() -> &'static str {
    "Fuse config controls guest joystick type and all 15 physical fire-button targets, but the pinned SDL2 frontend fixes physical devices to runtime slots 0 and 1. Verify those slots against the intended pads for the exact child immediately before launch. Preserve ROM directories, writable media, snapshots and recordings; native Flatpak remains unverified."
}

#[cfg(test)]
mod tests {
    use super::*;

    fn profile() -> PortProfile {
        let mut targets = [FireTarget::None; 15];
        targets[0] = FireTarget::JoystickFire;
        targets[1] = FireTarget::Enter;
        targets[2] = FireTarget::Letter('m');
        PortProfile {
            physical_slot: 0,
            output: JoystickType::Kempston,
            fire_targets: targets,
        }
    }

    #[test]
    fn patches_key_value_and_xml_grammars() {
        let plain = patch_config(
            b"machine=48\njoystick1output=0\nvolumeay=80\n",
            &[profile()],
        )
        .unwrap();
        assert!(plain.contains("joystick1output=2\n"));
        assert!(plain.contains("joystick1fire1=4096\n"));
        assert!(plain.contains("joystick1fire2=256\n"));
        assert!(plain.contains("volumeay=80\n"));

        let xml = patch_config(
            b"<?xml version=\"1.0\"?><settings><machine>48</machine><joystick1output>0</joystick1output></settings>",
            &[profile()],
        )
        .unwrap();
        assert!(xml.contains("<machine>48</machine>"));
        assert!(xml.contains("<joystick1output>2</joystick1output>"));
        assert!(xml.contains("<joystick1fire1>4096</joystick1fire1>"));
    }

    #[test]
    fn rejects_unrecognized_physical_slots_and_targets() {
        let mut bad = profile();
        bad.physical_slot = 2;
        assert!(patch_config(b"", &[bad]).is_err());
        let mut bad = profile();
        bad.fire_targets[0] = FireTarget::Digit(10);
        assert!(patch_config(b"", &[bad]).is_err());
    }
}
