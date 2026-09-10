//! Standalone MAME controller defaults, not libretro mappings.
//! MAME 0.280, ec9abd86c6c9029f67e9cf4908ef5426b78d3eab,
//! docs/source/advanced/ctrlr_config.rst. No cabinet inspection is required.
use anyhow::{Result, ensure};
use std::collections::{BTreeMap, BTreeSet};

pub(crate) mod config_copy;
pub(crate) mod configuration;
pub(crate) mod launch;
#[cfg(target_os = "linux")]
pub(crate) mod native_command;
pub(crate) mod overrides;
pub(crate) mod prepared;
pub(crate) mod sdl;
#[cfg(target_os = "linux")]
pub(crate) mod session;
pub(crate) mod settings;
pub(crate) mod tokens;

pub(crate) struct DeviceSlot<'a> {
    /// Full ID reported by the chosen MAME input provider, not a display name.
    pub native_id: &'a str,
    pub number: u8,
}

/// The caller supplies a current inventory from the same MAME input provider.
/// MAME permits substring matching: require a full ID and reject ambiguous
/// matches, including duplicate IDs and another ID containing this full ID.
pub(crate) fn mapped_controller_xml(
    players: &[Player<'_>],
    devices: &[DeviceSlot<'_>],
    observed_joystick_ids: &[String],
) -> Result<String> {
    ensure!(
        !devices.is_empty() && devices.len() <= 8,
        "Invalid MAME device mapping count"
    );
    let mut slots = BTreeSet::new();
    let mut ids = BTreeSet::new();
    let mut mappings = String::new();
    for device in devices {
        ensure!(
            (1..=8).contains(&device.number)
                && slots.insert(device.number)
                && ids.insert(device.native_id),
            "Duplicate or invalid MAME device mapping"
        );
        ensure!(
            !device.native_id.trim().is_empty()
                && device.native_id.len() <= 4096
                && device
                    .native_id
                    .chars()
                    .all(|ch| !ch.is_control() && ch != '\u{fffe}' && ch != '\u{ffff}'),
            "MAME device ID cannot be represented safely in XML"
        );
        ensure!(
            observed_joystick_ids
                .iter()
                .any(|id| id == device.native_id)
                && observed_joystick_ids
                    .iter()
                    .filter(|id| id.contains(device.native_id))
                    .count()
                    == 1,
            "MAME device ID is missing, partial or ambiguous in the native provider inventory"
        );
        let escaped = device
            .native_id
            .replace('&', "&amp;")
            .replace('"', "&quot;")
            .replace('<', "&lt;")
            .replace('>', "&gt;")
            .replace('\'', "&apos;");
        mappings.push_str(&format!(
            "      <mapdevice device=\"{escaped}\" controller=\"JOYCODE_{}\" />\n",
            device.number
        ));
    }
    for player in players {
        for token in player.tokens.values() {
            if let Some(rest) = token.strip_prefix("JOYCODE_") {
                let number: u8 = rest.split('_').next().unwrap_or("").parse()?;
                ensure!(
                    slots.contains(&number),
                    "MAME input references an unmapped native joystick slot"
                );
            }
        }
    }
    let xml = controller_xml(players)?;
    Ok(xml.replacen("    <input>\n", &format!("    <input>\n{mappings}"), 1))
}

/// Convert provider-resolved native items for the XML writer. Supplying a MAME
/// device number does not establish physical controller ownership by itself.
pub(crate) fn joystick_tokens(
    native_device_number: u8,
    controls: &BTreeMap<String, tokens::Item>,
) -> Result<BTreeMap<String, String>> {
    controls
        .iter()
        .map(|(control, item)| Ok((control.clone(), item.token(native_device_number)?)))
        .collect()
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "kebab-case")]
pub(crate) enum Panel {
    Six,
    Eight,
}

impl Panel {
    pub(crate) fn buttons(self) -> u8 {
        if self == Self::Six { 6 } else { 8 }
    }

    pub(crate) fn routes(self, player: u8) -> BTreeMap<String, String> {
        let mut types = BTreeMap::from([
            ("up".into(), format!("P{player}_JOYSTICK_UP")),
            ("down".into(), format!("P{player}_JOYSTICK_DOWN")),
            ("left".into(), format!("P{player}_JOYSTICK_LEFT")),
            ("right".into(), format!("P{player}_JOYSTICK_RIGHT")),
            ("start".into(), format!("START{player}")),
            ("coin".into(), format!("COIN{player}")),
        ]);
        for button in 1..=self.buttons() {
            types.insert(
                format!("button{button}"),
                format!("P{player}_BUTTON{button}"),
            );
        }
        types
    }
}

/// Tokens must already have been resolved against MAME's selected native input
/// provider. This writer deliberately does not guess SDL or joydev numbering.
pub(crate) struct Player<'a> {
    pub number: u8,
    pub panel: Panel,
    pub tokens: &'a BTreeMap<String, String>,
}

/// Only default type assignments are emitted. Native per-game assignments can
/// override these; the launch adapter must disclose or isolate those layers.
pub(crate) fn controller_xml(players: &[Player<'_>]) -> Result<String> {
    ensure!(
        !players.is_empty() && players.len() <= 8,
        "Invalid MAME player count"
    );
    let mut ports = BTreeSet::new();
    let mut xml = String::from(
        "<?xml version=\"1.0\"?>\n<mameconfig version=\"10\">\n  <system name=\"default\">\n    <input>\n",
    );
    for player in players {
        ensure!(
            (1..=8).contains(&player.number) && ports.insert(player.number),
            "Invalid or duplicate MAME player"
        );
        let types = player.panel.routes(player.number);
        ensure!(
            player.tokens.len() == types.len()
                && types.keys().all(|key| player.tokens.contains_key(key)),
            "MAME panel needs directions, Start, Coin and all selected buttons"
        );
        let mut owners = BTreeSet::new();
        for (control, native_type) in types {
            let token = &player.tokens[&control];
            // A single resolved token, never a sequence expression or XML.
            // Native token existence is the provider resolver's responsibility.
            ensure!(
                !token.is_empty()
                    && token.len() <= 128
                    && token.bytes().all(|byte| byte.is_ascii_uppercase()
                        || byte.is_ascii_digit()
                        || byte == b'_')
                    && (token.starts_with("JOYCODE_") || token.starts_with("KEYCODE_")),
                "MAME binding must be one resolved joystick or keyboard token"
            );
            ensure!(owners.insert(token), "MAME panel controls share an input");
            xml.push_str(&format!("      <port type=\"{native_type}\"><newseq type=\"standard\">{token}</newseq></port>\n"));
        }
    }
    xml.push_str("    </input>\n  </system>\n</mameconfig>\n");
    Ok(xml)
}
