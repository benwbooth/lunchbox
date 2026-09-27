//! Explicit refusal for SheepShaver gamepad serialization.
//!
//! Pinned source: cebix/macemu
//! `a68ae59238dafdbd279ca0fd1b0c2f7e684f58d4`. The Unix frontend exposes
//! keyboard and mouse only; no joystick/gamepad path exists in the pinned
//! source (`SheepShaver/src/Unix/` has no joystick handling).

pub(crate) const SOURCE_COMMIT: &str = "a68ae59238dafdbd279ca0fd1b0c2f7e684f58d4";

pub(crate) fn refusal() -> anyhow::Result<()> {
    anyhow::bail!(
        "SheepShaver gamepad mapping is unavailable: the pinned Unix source has keyboard/mouse only and no gamepad path"
    )
}

#[cfg(test)]
mod tests {
    #[test]
    fn keyboard_only_boundary_is_explicitly_refused() {
        assert!(super::refusal().is_err());
        assert_eq!(super::SOURCE_COMMIT.len(), 40);
    }
}
