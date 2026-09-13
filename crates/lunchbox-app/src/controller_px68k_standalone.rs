//! Explicit refusal for PX68k standalone-native controller serialization.
//!
//! PX68k is a libretro core. Its `system/keropi/config` file persists browser
//! and media paths, while controller type/layout is selected by core options
//! and runtime RETRO_DEVICE callbacks. It is not a physical-device profile.

pub(crate) const SOURCE_COMMIT: &str = "0ad84d7058a12b7db4f7f7a906e87fad4e2f26f6";

pub(crate) fn refusal() -> anyhow::Result<()> {
    anyhow::bail!(
        "PX68k controller mapping is unavailable: libretro callbacks and core options own input, while system/keropi/config is media/browser persistence rather than a controller profile"
    )
}

#[cfg(test)]
mod tests {
    #[test]
    fn libretro_owned_mapping_is_explicitly_refused() {
        assert!(super::refusal().is_err());
    }
}
