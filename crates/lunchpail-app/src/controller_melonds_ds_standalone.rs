//! Explicit refusal for melonDS DS as a native standalone target.
//!
//! Pinned upstream: JesseTG/melonds-ds@bc4e4b67d2d470d7c682810a1e892cafd6f9082b.
//! This project publishes a libretro core, not a standalone executable or
//! native config file. Controller input is the libretro RETRO_DEVICE_JOYPAD
//! port; RetroArch's existing core/input adapter owns host-device mapping.

pub(crate) const SOURCE_COMMIT: &str = "bc4e4b67d2d470d7c682810a1e892cafd6f9082b";

pub(crate) fn refusal() -> anyhow::Result<()> {
    anyhow::bail!(
        "melonDS DS standalone controller mapping is unavailable: upstream publishes a libretro core whose host mapping belongs to the libretro frontend"
    )
}

#[cfg(test)]
mod tests {
    #[test]
    fn libretro_only_target_is_explicitly_refused() {
        assert!(super::super::controller_melonds_ds_standalone::refusal().is_err());
    }
}
