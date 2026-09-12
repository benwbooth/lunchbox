//! Azahar native Linux SDL input-profile contract.
//!
//! Azahar stores profiles in the QSettings `Controls/profiles` array and
//! stores each control as a serialized `Common::ParamPackage`.  XDG config
//! may be isolated while XDG data is left untouched, preserving NAND, SDMC,
//! keys and user save data.

use anyhow::{Result, ensure};
use std::collections::BTreeMap;

pub(crate) const PROFILE_ID: &str = "azahar:standalone-3ds";
pub(crate) const CONTROLS: [(&str, &str); 16] = [
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
    ("zl", "button_zl"),
    ("zr", "button_zr"),
    ("start", "button_start"),
    ("select", "button_select"),
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
    Axis {
        guid: String,
        port: u32,
        x: u32,
        y: u32,
    },
}

impl Binding {
    fn param(&self) -> String {
        match self {
            Self::Button { guid, port, index } => format!(
                "engine:sdl,guid:{guid},port:{port},api:controller,button:{index},maptype:guid+port"
            ),
            Self::Axis { guid, port, x, y } => format!(
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
        !name.is_empty() && !name.contains(['=', '\n', '\r', '[', ']']),
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
        if let Binding::Button { guid, .. } | Binding::Axis { guid, .. } = binding {
            ensure!(
                valid_guid(guid),
                "Azahar SDL GUID must be 32 hex characters"
            );
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
            map.insert(
                name.to_owned(),
                Binding::Button {
                    guid: "0123456789abcdef0123456789abcdef".into(),
                    port: 0,
                    index: map.len() as u32,
                },
            );
        }
        let ini = profile_ini("Lunchbox", &map).unwrap();
        assert!(ini.contains("profiles\\size=1"));
        assert!(ini.contains("engine:sdl,guid:0123456789abcdef0123456789abcdef,port:0,api:controller,button:0,maptype:guid+port"));
    }
}
