//! Explicit refusal for QUASI88 standalone-native controller serialization.
//!
//! QUASI88's libretro port consumes RetroPad/RetroKeyboard callbacks and
//! leaves profile persistence to the frontend. Its config-file initializer is
//! stubbed, so no native controller grammar can be safely inferred.

pub(crate) const SOURCE_COMMIT: &str = "459bbc6e90caa3dc392ae8e64a9b0881b1e5ef77";

pub(crate) fn refusal() -> anyhow::Result<()> {
    anyhow::bail!(
        "QUASI88 controller mapping is unavailable: the libretro frontend owns RetroPad/RetroKeyboard profiles and the core has no stable physical-device config grammar"
    )
}

#[cfg(test)]
mod tests {
    #[test]
    fn frontend_owned_mapping_is_explicitly_refused() {
        assert!(super::refusal().is_err());
    }
}
