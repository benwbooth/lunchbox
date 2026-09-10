//! Mednafen native input expressions, distinct from Beetle/libretro mappings.
//! Format: https://mednafen.github.io/documentation/index.html#Section_input_mapping_format
//! Source pin: f0ee9d595db68ad5247ba5ac6a8367fdced9c3fc, drivers/input.cpp.
use anyhow::{Result, ensure};

pub(crate) mod configuration;
pub(crate) mod disc;
pub(crate) mod discovery;
#[cfg(target_os = "linux")]
pub(crate) mod enumeration;
pub(crate) mod identity;
#[cfg(target_os = "linux")]
pub(crate) mod inventory;
pub(crate) mod ips;
pub(crate) mod isolation;
pub(crate) mod layers;
pub(crate) mod md;
#[cfg(target_os = "linux")]
pub(crate) mod native_command;
pub(crate) mod nes;
pub(crate) mod nes_content;
pub(crate) mod nes_desired;
pub(crate) mod paths;
pub(crate) mod pce;
pub(crate) mod physical;
pub(crate) mod prepared;
pub(crate) mod profiles;
pub(crate) mod psx;
pub(crate) mod saturn;
#[cfg(target_os = "linux")]
pub(crate) mod session;
pub(crate) mod settings;
pub(crate) mod snes;
#[cfg(target_os = "linux")]
pub(crate) mod state;
#[cfg(target_os = "linux")]
pub(crate) mod sysfs;

#[derive(Clone, Copy, Debug, Eq, PartialEq, Ord, PartialOrd)]
pub(crate) enum Polarity {
    Positive,
    Negative,
    Full,
    FullReversed,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Ord, PartialOrd)]
pub(crate) enum Input {
    Button(u16),
    Absolute { index: u16, polarity: Polarity },
}

impl Input {
    pub(crate) fn as_setting(self) -> Result<String> {
        Ok(match self {
            Self::Button(index) => {
                ensure!(index <= 1023, "Mednafen native button index exceeds 1023");
                format!("button_{index}")
            }
            Self::Absolute { index, polarity } => {
                ensure!(index <= 1023, "Mednafen native axis index exceeds 1023");
                let suffix = match polarity {
                    Polarity::Positive => "+",
                    Polarity::Negative => "-",
                    Polarity::Full => "-+",
                    Polarity::FullReversed => "+-",
                };
                format!("abs_{index}{suffix}")
            }
        })
    }
}

/// The 128-bit ID must come from Mednafen's native inventory. SDL GUIDs and
/// kernel paths are not interchangeable with it, even when they look similar.
pub(crate) fn binding(native_id: &str, input: Input, scale: u16) -> Result<String> {
    let id = native_id.strip_prefix("0x").unwrap_or(native_id);
    ensure!(
        id.len() == 32 && id.bytes().all(|byte| byte.is_ascii_hexdigit()),
        "Mednafen requires a complete native 128-bit joystick ID"
    );
    ensure!(scale > 0, "Mednafen gameplay binding scale must be nonzero");
    let input = input.as_setting()?;
    // Native default is 4.12 fixed-point unity (4096), not a percentage.
    let scale = if scale == 4096 {
        String::new()
    } else {
        format!(" {scale}")
    };
    Ok(format!(
        "joystick 0x{} {input}{scale}",
        id.to_ascii_lowercase()
    ))
}
