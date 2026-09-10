//! Pinned SDL binary preferences, not an INI file or Rust struct dump.
use super::Bindings;
use anyhow::{Result, ensure};

/// Layout derived from v1.0.3 configuration.h for the ordinary C ABI:
/// 4-byte enums/scancodes/unsigned, 1-byte bool, 4-byte struct alignment,
/// and the explicitly packed/aligned(4) final anonymous struct.
/// The launch owner must establish the trusted executable's ABI; file length
/// alone is not proof of compatibility. Never use this on Cocoa preferences.
#[derive(Clone, Copy, Debug, Eq, PartialEq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "kebab-case")]
pub(crate) enum Abi {
    Sdl103Enums32Bool8,
}

pub(crate) fn render(original: &[u8], bindings: &Bindings, abi: Abi) -> Result<Vec<u8>> {
    let Abi::Sdl103Enums32Bool8 = abi;
    const SIZE: usize = 4460;
    const BUTTONS: usize = 224;
    const AXES: usize = 256;
    const PACKED_TAIL: usize = 4396;
    const FAUX_ANALOG: usize = PACKED_TAIL + 57;
    // Refuse partial older files rather than replacing native defaults with
    // zeroes; their missing bytes are initialized by the native frontend.
    ensure!(
        original.len() == SIZE,
        "SameBoy needs complete v1.0.3 SDL preferences for the declared ABI"
    );
    ensure!(
        bindings.buttons[8..].iter().all(|button| *button == 255),
        "SameBoy generated mapping must clear inherited controller shortcuts"
    );
    let mut result = original.to_vec();
    result[BUTTONS..BUTTONS + 32].copy_from_slice(&bindings.buttons);
    result[AXES..AXES + 2].copy_from_slice(&bindings.axes);
    // Digital axis mapping must use the native hysteresis, not frame-varying
    // faux analog inputs. Leave keyboard, audio, video and save settings intact.
    result[FAUX_ANALOG] = 0;
    Ok(result)
}
