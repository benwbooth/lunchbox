//! ScummVM native ini keymapper bindings for the SDL2 gamepad input, not
//! libretro bindings.
//!
//! Functional contract pinned to scummvm/scummvm
//! 3f6428df202e2c044ef53208acba0f665ea92096:
//! - `common/config-manager.cpp` — the configuration is an ini file whose
//!   `[keymapper]` domain and per-target game domains persist keymap
//!   entries `keymap_<keymap-id>_<action-id> = <hw input ids...>`
//!   (space separated; `backends/keymapper/keymap.cpp` loadMappings/
//!   saveMappings with `KEYMAP_KEY_PREFIX "keymap_"`).
//! - `engines/metaengine.cpp initKeymaps` — every engine without custom
//!   keymaps gets the `engine-default` game keymap whose actions (LCLK,
//!   RCLK, PAUSE, MENU, SKIP, SKLI, PIND, RETURN, UP, DOWN, LEFT, RIGHT)
//!   default to `JOY_A`, `JOY_B`, `JOY_LEFT_SHOULDER`, `JOY_Y`, `JOY_X`
//!   and the D-pad ids; game keymaps load from the active target's domain.
//! - `backends/keymapper/hardware-input.cpp` — joystick hardware input ids
//!   are `JOY_A/B/X/Y/BACK/GUIDE/START/LEFT_STICK/RIGHT_STICK/
//!   LEFT_SHOULDER/RIGHT_SHOULDER/UP/DOWN/LEFT/RIGHT`, matching
//!   `backends/events/sdl/sdl2-events.cpp mapSDLControllerButtonToOSystem`
//!   which reads SDL's standard gamepad button order.
//! - `base/commandLine.cpp` — `joystick_num` (default 0) selects the SDL
//!   device index; `-c <file>` selects the configuration file.
//! - `backends/platform/sdl/posix/posix.cpp` — the configuration lives at
//!   `$XDG_CONFIG_HOME/scummvm/scummvm.ini`, so a private XDG_CONFIG_HOME
//!   isolates every read and write.
use anyhow::{Context, Result, ensure};

#[cfg(target_os = "linux")]
pub(crate) mod native_command;
#[cfg(target_os = "linux")]
pub(crate) mod session;
pub(crate) mod settings;

/// Target controls: layout id -> (action id, default hw id label).
pub(crate) const CONTROLS: [(&str, &str); 10] = [
    ("a", "LCLK"),
    ("b", "RCLK"),
    ("x", "SKLI"),
    ("y", "SKIP"),
    ("l", "MENU"),
    ("start", "RETURN"),
    ("up", "UP"),
    ("down", "DOWN"),
    ("left", "LEFT"),
    ("right", "RIGHT"),
];

/// The default game keymap id every engine inherits.
pub(crate) const KEYMAP_ID: &str = "engine-default";

/// SDL standard gamepad field name -> ScummVM hardware input id, in the
/// button order mapSDLControllerButtonToOSystem consumes.
pub(crate) fn joy_id(standard: &str) -> Option<&'static str> {
    Some(match standard {
        "a" => "JOY_A",
        "b" => "JOY_B",
        "x" => "JOY_X",
        "y" => "JOY_Y",
        "back" => "JOY_BACK",
        "guide" => "JOY_GUIDE",
        "start" => "JOY_START",
        "leftstick" => "JOY_LEFT_STICK",
        "rightstick" => "JOY_RIGHT_STICK",
        "leftshoulder" => "JOY_LEFT_SHOULDER",
        "rightshoulder" => "JOY_RIGHT_SHOULDER",
        "dpup" => "JOY_UP",
        "dpdown" => "JOY_DOWN",
        "dpleft" => "JOY_LEFT",
        "dpright" => "JOY_RIGHT",
        _ => return None,
    })
}

/// One raw joystick backing spec from an SDL mapping string.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) enum Raw {
    Button(u32),
    Axis { index: u32, positive: bool },
    Hat { hat: u32, mask: u8 },
}

impl Raw {
    fn parse(spec: &str) -> Option<Self> {
        let spec = spec.strip_prefix('~').unwrap_or(spec);
        if let Some(button) = spec.strip_prefix('b') {
            return button.parse().ok().map(Raw::Button);
        }
        if let Some(axis) = spec.strip_prefix('a') {
            let (axis, positive) = match axis.strip_suffix('+') {
                Some(axis) => (axis, true),
                None => (axis.strip_suffix('-')?, false),
            };
            return Some(Raw::Axis {
                index: axis.parse().ok()?,
                positive,
            });
        }
        if let Some(hat) = spec.strip_prefix('h') {
            let (hat, mask) = hat.split_once('.')?;
            return Some(Raw::Hat {
                hat: hat.parse().ok()?,
                mask: mask.parse().ok()?,
            });
        }
        None
    }
}

/// Parse an SDL2 gamecontroller mapping string into standard-name → raw
/// backing pairs. The string is `guid,name[,platform:...],field:spec,...`.
pub(crate) fn parse_mapping(mapping: &str) -> Result<std::collections::BTreeMap<String, Raw>> {
    let mut fields = mapping.split(',');
    let guid = fields.next().context("Mapping string lacks a GUID")?;
    ensure!(!guid.is_empty() && guid.len() <= 64, "Invalid mapping GUID");
    let name = fields.next().context("Mapping string lacks a name")?;
    ensure!(!name.is_empty(), "Mapping string has an empty name");
    let mut result = std::collections::BTreeMap::new();
    for field in fields {
        if field.is_empty() {
            continue;
        }
        let Some((standard, spec)) = field.split_once(':') else {
            continue;
        };
        if let Some(raw) = Raw::parse(spec) {
            ensure!(
                result.insert(standard.to_owned(), raw).is_none(),
                "Duplicate mapping field {standard}"
            );
        }
    }
    ensure!(!result.is_empty(), "Mapping string carries no fields");
    Ok(result)
}

/// Render the private ini's target section carrying the game path and the
/// engine-default keymap entries. `bindings` pairs action ids with hardware
/// input ids.
pub(crate) fn target_section(
    target: &str,
    game_dir: &std::path::Path,
    bindings: &[(String, String)],
) -> Result<String> {
    ensure!(
        !target.is_empty()
            && target
                .bytes()
                .all(|b| b.is_ascii_alphanumeric() || b == b'-' || b == b'_'),
        "ScummVM target names must be plain identifiers"
    );
    let path = game_dir
        .to_str()
        .context("ScummVM game path is not UTF-8")?;
    ensure!(
        !path.contains(['\r', '\n']),
        "ScummVM game path has a newline"
    );
    let mut out = format!("[{target}]\npath={path}\n");
    let mut written = std::collections::BTreeSet::new();
    for (action, hw) in bindings {
        ensure!(
            !hw.contains(char::is_whitespace),
            "ScummVM hardware input ids carry no spaces"
        );
        ensure!(written.insert(action.clone()), "Duplicate action {action}");
        out.push_str(&format!("keymap_{KEYMAP_ID}_{action}={hw}\n"));
    }
    for (_, action) in CONTROLS {
        ensure!(
            written.contains(action),
            "ScummVM needs the {action} binding"
        );
    }
    Ok(out)
}

/// Render the application section selecting the first SDL device.
pub(crate) fn app_section() -> String {
    "[scummvm]\njoystick_num=0\n".into()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn mapping_fields_parse_into_raw_specs() {
        let mapping = "0300abcd,Fancy Pad,platform:Linux,a:b5,b:b6,x:b7,y:b8,back:b4,\
guide:b13,start:b3,leftshoulder:b9,rightshoulder:b10,dpup:h0.1,dpdown:h0.4,\
dpleft:h0.8,dpright:h0.2,leftx:a0,lefty:a1,";
        let fields = parse_mapping(mapping).unwrap();
        assert_eq!(fields["a"], Raw::Button(5));
        assert_eq!(fields["leftshoulder"], Raw::Button(9));
        assert_eq!(fields["dpup"], Raw::Hat { hat: 0, mask: 1 });
        assert_eq!(fields["dpleft"], Raw::Hat { hat: 0, mask: 8 });
        assert!(!fields.contains_key("leftx"));
        assert!(!fields.contains_key("lefty"));
        assert_eq!(fields.len(), 13);
        assert!(parse_mapping("onlyonefield").is_err());
        assert!(parse_mapping("guid,name,").is_err());
    }

    #[test]
    fn joy_ids_cover_the_sdl_button_order() {
        assert_eq!(joy_id("a").unwrap(), "JOY_A");
        assert_eq!(joy_id("leftshoulder").unwrap(), "JOY_LEFT_SHOULDER");
        assert_eq!(joy_id("dpup").unwrap(), "JOY_UP");
        assert_eq!(joy_id("dpright").unwrap(), "JOY_RIGHT");
        assert!(joy_id("leftx").is_none());
    }

    #[test]
    fn target_section_renders_path_and_every_binding() {
        let bindings: Vec<_> = CONTROLS
            .iter()
            .map(|(_, action)| ((*action).to_owned(), "JOY_A".to_owned()))
            .collect();
        let text = target_section(
            "lunchbox_game",
            std::path::Path::new("/games/monkey"),
            &bindings,
        )
        .unwrap();
        assert!(text.starts_with("[lunchbox_game]\npath=/games/monkey\n"));
        assert!(text.contains("keymap_engine-default_LCLK=JOY_A\n"));
        assert!(text.contains("keymap_engine-default_RETURN=JOY_A\n"));
        assert!(text.contains("keymap_engine-default_RIGHT=JOY_A\n"));
        let missing: Vec<(String, String)> = CONTROLS
            .iter()
            .skip(1)
            .map(|(_, a)| ((*a).to_owned(), "JOY_A".to_owned()))
            .collect();
        assert!(target_section("t", std::path::Path::new("/g"), &missing).is_err());
        assert!(target_section("bad name!", std::path::Path::new("/g"), &bindings).is_err());
    }

    #[test]
    fn app_section_selects_the_first_device() {
        assert_eq!(app_section(), "[scummvm]\njoystick_num=0\n");
    }

    #[test]
    fn mapping_parser_rejects_duplicate_standard_fields() {
        assert!(parse_mapping("guid,name,a:b0,a:b1").is_err());
    }
}
