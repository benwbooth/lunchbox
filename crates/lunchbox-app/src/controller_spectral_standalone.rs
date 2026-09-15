//! Explicit refusal for Spectral gamepad serialization.
//!
//! Pinned source: r-lyeh/Spectral
//! `fd652c16ff0f82a918d39f8f62d23983c5ea69d7`. The `gamepad3` reader in
//! `sys_gamepad.h` returns zero on Linux ("Lubuntu16: current 3rd_gamepad
//! lib is returning noisy reads"), so no Linux gamepad path exists to stage.

pub(crate) const SOURCE_COMMIT: &str = "fd652c16ff0f82a918d39f8f62d23983c5ea69d7";

pub(crate) fn refusal() -> anyhow::Result<()> {
    anyhow::bail!(
        "Spectral gamepad mapping is unavailable: the pinned source disables gamepad reads on Linux"
    )
}

#[cfg(test)]
mod tests {
    #[test]
    fn linux_disabled_gamepads_are_explicitly_refused() {
        assert!(super::refusal().is_err());
        assert_eq!(super::SOURCE_COMMIT.len(), 40);
    }
}
