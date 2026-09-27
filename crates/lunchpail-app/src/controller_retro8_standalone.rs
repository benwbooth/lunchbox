//! Explicit refusal for Retro8 controller serialization.

pub(crate) const SOURCE_COMMIT: &str = "ddc06a142398ee9755894b3f0bb17c8dc428151d";

pub(crate) fn refusal() -> anyhow::Result<()> {
    anyhow::bail!(
        "Retro8 controller mapping is unavailable: the pinned project exposes runtime libretro joypad callbacks but no persistent host-controller profile grammar"
    )
}

#[cfg(test)]
mod tests {
    #[test]
    fn libretro_runtime_input_boundary_is_refused() {
        assert!(super::refusal().is_err());
        assert_eq!(super::SOURCE_COMMIT.len(), 40);
    }
}
