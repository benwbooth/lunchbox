//! PicoDrive native input.cfg writer.
//!
//! Pinned source: notaz/picodrive@26ecb2b6358fefba24e3d68b9eb2efba7f10d5ee.
//! `platform/common/config_file.c` writes `binddev = ...` and
//! `bind <key> = playerN <action>` lines. This renderer follows that grammar
//! and refuses ambiguous device/action text.

use anyhow::{Result, ensure};
use std::collections::BTreeSet;

pub(crate) const SOURCE_COMMIT: &str = "26ecb2b6358fefba24e3d68b9eb2efba7f10d5ee";
pub(crate) const PROFILE_ID: &str = "picodrive:native-bind-v1";
pub(crate) const PLAYER_ACTIONS: [&str; 15] = [
    "UP", "DOWN", "LEFT", "RIGHT", "A", "B", "C", "START", "MODE", "X", "Y", "Z", "A turbo",
    "B turbo", "C turbo",
];

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct Binding {
    pub host_key: String,
    pub player: u8,
    pub action: String,
}

fn valid_atom(value: &str, what: &str) -> Result<()> {
    ensure!(
        !value.is_empty() && value.len() <= 64,
        "PicoDrive {what} is empty or too long"
    );
    ensure!(
        !value.chars().any(|c| c == '\n' || c == '\r' || c == '\0'),
        "PicoDrive {what} contains a control character"
    );
    Ok(())
}

fn escaped_key(key: &str) -> Result<String> {
    valid_atom(key, "host key")?;
    Ok(match key {
        "#" => "\\x23".to_owned(),
        "=" => "\\x3d".to_owned(),
        _ => key.to_owned(),
    })
}

pub(crate) fn config_text(device: &str, bindings: &[Binding]) -> Result<String> {
    valid_atom(device, "device name")?;
    ensure!(
        !device.contains(['#', '=']),
        "PicoDrive device name conflicts with the source line grammar"
    );
    ensure!(
        !bindings.is_empty() && bindings.len() <= 128,
        "PicoDrive binding count is invalid"
    );
    // `config_file.c` uses CRLF on the non-MSVC path and LF on MSVC.
    let newline = "\r\n";
    let mut out = format!("binddev = {device}{newline}");
    let mut seen = BTreeSet::new();
    for binding in bindings {
        ensure!(
            (1..=4).contains(&binding.player),
            "PicoDrive player must be 1..4"
        );
        ensure!(
            PLAYER_ACTIONS.contains(&binding.action.as_str()),
            "PicoDrive action is not in the source action table"
        );
        ensure!(
            seen.insert((&binding.host_key, binding.player, &binding.action)),
            "Duplicate PicoDrive binding"
        );
        let key = escaped_key(&binding.host_key)?;
        out.push_str(&format!(
            "bind {key} = player{} {}{newline}",
            binding.player, binding.action
        ));
    }
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn renders_source_bind_grammar_and_escapes_reserved_keys() {
        let text = config_text(
            "SDL Controller",
            &[
                Binding {
                    host_key: "#".into(),
                    player: 1,
                    action: "A".into(),
                },
                Binding {
                    host_key: "=".into(),
                    player: 2,
                    action: "START".into(),
                },
            ],
        )
        .unwrap();
        assert_eq!(
            text,
            "binddev = SDL Controller\r\nbind \\x23 = player1 A\r\nbind \\x3d = player2 START\r\n"
        );
    }

    #[test]
    fn rejects_unknown_actions_and_injection() {
        assert!(
            config_text(
                "pad",
                &[Binding {
                    host_key: "A\n".into(),
                    player: 1,
                    action: "A".into()
                }]
            )
            .is_err()
        );
        assert!(
            config_text(
                "pad",
                &[Binding {
                    host_key: "A".into(),
                    player: 1,
                    action: "reset".into()
                }]
            )
            .is_err()
        );
        assert!(
            config_text(
                "pad#comment",
                &[Binding {
                    host_key: "A".into(),
                    player: 1,
                    action: "A".into()
                }]
            )
            .is_err()
        );
        let duplicate = Binding {
            host_key: "A".into(),
            player: 1,
            action: "A".into(),
        };
        assert!(config_text("pad", &[duplicate.clone(), duplicate]).is_err());
    }
}
