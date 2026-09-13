//! Explicit refusal for NES.emu controller serialization without a verified
//! runtime/config path.

pub(crate) const SOURCE_COMMIT: &str = "1c12fac5ce49badaadff2e2f210dcc30b89f4943";

pub(crate) fn refusal() -> anyhow::Result<()> {
    anyhow::bail!(
        "NES.emu controller mapping is unavailable: upstream defines config serialization, but this catalog has no verified runtime path or device fixture"
    )
}

#[cfg(test)]
mod tests {
    #[test]
    fn android_only_nes_emu_is_explicitly_refused() {
        assert!(super::super::controller_nes_emu_standalone::refusal().is_err());
    }
}
