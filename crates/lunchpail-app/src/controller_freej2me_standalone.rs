//! Explicit refusal for FreeJ2ME-Plus gamepad serialization.
//!
//! Pinned source: TASEmulators/freej2me-plus
//! `e1dee0bd33ce8842cb335ff1c31c10b30191eaae`. Gamepad support lives in
//! `src/win32pad/` (XInput/DirectInput); there is no Linux gamepad path,
//! so no native Linux adapter can be staged.

pub(crate) const SOURCE_COMMIT: &str = "e1dee0bd33ce8842cb335ff1c31c10b30191eaae";

pub(crate) fn refusal() -> anyhow::Result<()> {
    anyhow::bail!(
        "FreeJ2ME-Plus gamepad mapping is unavailable: gamepad support is Windows-only with no Linux path"
    )
}

#[cfg(test)]
mod tests {
    #[test]
    fn windows_only_gamepads_are_explicitly_refused() {
        assert!(super::refusal().is_err());
        assert_eq!(super::SOURCE_COMMIT.len(), 40);
    }
}
