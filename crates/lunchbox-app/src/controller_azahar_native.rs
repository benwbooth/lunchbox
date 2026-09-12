//! Azahar native Linux SDL input-profile contract.
//!
//! Pinned source: azahar-emu/azahar commit
//! `ec8201d42cd3d8e2ec1d69d5832be0389490ea47`.
//!
//! Azahar stores profiles in the QSettings `Controls/profiles` array and
//! stores each control as a serialized `Common::ParamPackage`.  XDG config
//! may be isolated while XDG data is left untouched, preserving NAND, SDMC,
//! keys and user save data.

use anyhow::{Result, ensure};
use std::collections::BTreeMap;

pub(crate) const PROFILE_ID: &str = "azahar:standalone-3ds";
pub(crate) const CONTROLS: [(&str, &str); 20] = [
    ("a", "button_a"),
    ("b", "button_b"),
    ("x", "button_x"),
    ("y", "button_y"),
    ("up", "button_up"),
    ("down", "button_down"),
    ("left", "button_left"),
    ("right", "button_right"),
    ("l", "button_l"),
    ("r", "button_r"),
    ("start", "button_start"),
    ("select", "button_select"),
    ("debug", "button_debug"),
    ("gpio14", "button_gpio14"),
    ("zl", "button_zl"),
    ("zr", "button_zr"),
    ("home", "button_home"),
    ("power", "button_power"),
    ("circle_pad", "circle_pad"),
    ("c_stick", "c_stick"),
];

#[derive(Clone, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub(crate) enum Binding {
    Button {
        guid: String,
        port: u32,
        index: u32,
    },
    Trigger {
        guid: String,
        port: u32,
        axis: u32,
    },
    Analog {
        guid: String,
        port: u32,
        x: u32,
        y: u32,
    },
}

impl Binding {
    fn param(&self) -> String {
        // SDL_JoystickGetGUIDString, which Azahar uses for its joystick map,
        // emits lower-case hexadecimal.  Canonicalizing here prevents an
        // otherwise valid upper-case caller value from missing that lookup.
        let guid = match self {
            Self::Button { guid, .. } | Self::Trigger { guid, .. } | Self::Analog { guid, .. } => {
                guid.to_ascii_lowercase()
            }
        };
        match self {
            Self::Button { port, index, .. } => format!(
                "engine:sdl,guid:{guid},port:{port},api:controller,button:{index},maptype:guid+port"
            ),
            Self::Trigger { port, axis, .. } => format!(
                "engine:sdl,guid:{guid},port:{port},api:controller,axis:{axis},direction:+,threshold:0.5,maptype:guid+port"
            ),
            Self::Analog { port, x, y, .. } => format!(
                "engine:sdl,guid:{guid},port:{port},api:controller,axis_x:{x},axis_y:{y},maptype:guid+port"
            ),
        }
    }
}

fn valid_guid(guid: &str) -> bool {
    guid.len() == 32 && guid.bytes().all(|b| b.is_ascii_hexdigit())
}

/// Render a complete QSettings INI containing one input profile.  Existing
/// unrelated settings can be prepended by the caller; this output is limited
/// to the source's `Controls` keys and is therefore safe to stage privately.
pub(crate) fn profile_ini(name: &str, mappings: &BTreeMap<String, Binding>) -> Result<String> {
    ensure!(
        !name.is_empty()
            && !name.chars().any(char::is_control)
            && !name.contains(['=', '\n', '\r', '[', ']']),
        "Azahar profile name is invalid"
    );
    ensure!(
        mappings.len() == CONTROLS.len(),
        "Azahar needs every 3DS gameplay mapping"
    );
    let mut out = String::from("[Controls]\nprofile=0\nprofiles\\size=1\nprofiles\\1\\name=");
    out.push_str(name);
    out.push_str("\nprofiles\\1\\input_maptype=2\n");
    let mut used = std::collections::BTreeSet::new();
    for (target, key) in CONTROLS {
        let binding = mappings
            .get(target)
            .ok_or_else(|| anyhow::anyhow!("Azahar mapping {target} is absent"))?;
        let guid = match binding {
            Binding::Button { guid, .. }
            | Binding::Trigger { guid, .. }
            | Binding::Analog { guid, .. } => guid,
        };
        ensure!(
            valid_guid(guid),
            "Azahar SDL GUID must be 32 hex characters"
        );
        let port = match binding {
            Binding::Button { port, .. }
            | Binding::Trigger { port, .. }
            | Binding::Analog { port, .. } => port,
        };
        ensure!(
            *port <= i32::MAX as u32,
            "Azahar SDL controller port is out of range"
        );
        match binding {
            Binding::Button { index, .. } => {
                ensure!(*index < 32, "Azahar SDL gamepad button is out of range");
            }
            Binding::Trigger { axis, .. } => {
                ensure!(*axis < 6, "Azahar SDL gamepad axis is out of range");
                ensure!(*axis >= 4, "Azahar trigger must use a trigger axis");
            }
            Binding::Analog { x, y, .. } => {
                ensure!(
                    *x < 6 && *y < 6 && *x != *y,
                    "Azahar analog axes are invalid"
                );
            }
        }
        let value = binding.param();
        ensure!(used.insert(value.clone()), "Azahar mapping is reused");
        out.push_str("profiles\\1\\");
        out.push_str(key);
        out.push('=');
        out.push_str(&value);
        out.push('\n');
    }
    out.push_str("profiles\\1\\motion_device=engine:motion_emu,update_period:100,sensitivity:0.01,tilt_clamp:90.0\nprofiles\\1\\touch_device=engine:emu_window\nprofiles\\1\\use_touchpad=false\n");
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn emits_qsettings_array_and_guid_port_mapping() {
        let mut map = BTreeMap::new();
        for (name, _) in CONTROLS {
            let binding = match name {
                "zl" => Binding::Trigger {
                    guid: "0123456789abcdef0123456789abcdef".into(),
                    port: 0,
                    axis: 4,
                },
                "zr" => Binding::Trigger {
                    guid: "0123456789abcdef0123456789abcdef".into(),
                    port: 0,
                    axis: 5,
                },
                "circle_pad" => Binding::Analog {
                    guid: "0123456789abcdef0123456789abcdef".into(),
                    port: 0,
                    x: 0,
                    y: 1,
                },
                "c_stick" => Binding::Analog {
                    guid: "0123456789abcdef0123456789abcdef".into(),
                    port: 0,
                    x: 2,
                    y: 3,
                },
                _ => Binding::Button {
                    guid: "0123456789abcdef0123456789abcdef".into(),
                    port: 0,
                    index: map.len() as u32,
                },
            };
            map.insert(name.to_owned(), binding);
        }
        let ini = profile_ini("Lunchbox", &map).unwrap();
        assert!(ini.contains("profiles\\size=1"));
        assert!(ini.contains("engine:sdl,guid:0123456789abcdef0123456789abcdef,port:0,api:controller,button:0,maptype:guid+port"));
        assert!(ini.contains("axis:4,direction:+,threshold:0.5"));
        assert!(ini.contains("axis_x:0,axis_y:1"));
    }

    #[test]
    fn canonicalizes_uppercase_sdl_guid() {
        let mut map = BTreeMap::new();
        for (name, _) in CONTROLS {
            let binding = if name == "circle_pad" {
                Binding::Analog {
                    guid: "ABCDEF0123456789ABCDEF0123456789".into(),
                    port: 0,
                    x: 0,
                    y: 1,
                }
            } else {
                Binding::Button {
                    guid: "ABCDEF0123456789ABCDEF0123456789".into(),
                    port: 0,
                    index: map.len() as u32,
                }
            };
            map.insert(name.to_owned(), binding);
        }
        let ini = profile_ini("Lunchbox", &map).unwrap();
        assert!(ini.contains("guid:abcdef0123456789abcdef0123456789"));
        assert!(!ini.contains("guid:ABCDEF0123456789ABCDEF0123456789"));
    }
}
