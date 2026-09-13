//! Explicit refusal for MSX.emu native controller serialization.
//!
//! MSX.emu is an Android/iOS/Linux-family proprietary ExPlusAlpha build. The
//! official page does not expose a source-pinned native profile grammar.

pub(crate) const SOURCE_URL: &str = "https://www.explusalpha.com/contents/msx-emu/";
pub(crate) fn refusal() -> anyhow::Result<()> {
    anyhow::bail!(
        "MSX.emu controller mapping is unavailable: the official proprietary build does not publish a stable host-device profile format"
    )
}
#[cfg(test)]
mod tests {
    #[test]
    fn refuses_unverified_profile() {
        assert!(super::super::controller_msx_emu_standalone::refusal().is_err());
    }
}
