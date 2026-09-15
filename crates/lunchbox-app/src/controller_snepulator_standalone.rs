//! Explicit refusal for Snepulator gamepad serialization.
//!
//! Pinned source: JoppyFurr/Snepulator
//! `54d20962a5747698df9527ed1e2f2fa5f1ce4fb1`. The `gamepad-<uuid>`
//! section grammar (`type-key-direction` per `sms-*` key) is pinned, and
//! device identity is the stable SDL GUID. But player-device assignment is
//! runtime GUI state only: `gamepad_init` binds player 1 to keyboard on
//! every startup, and no config key persists a player-1 UUID. Staging a
//! UUID section alone cannot route player 1 to the pad without user
//! interaction, so no launch adapter is wired.

pub(crate) const SOURCE_COMMIT: &str = "54d20962a5747698df9527ed1e2f2fa5f1ce4fb1";

pub(crate) fn refusal() -> anyhow::Result<()> {
    anyhow::bail!(
        "Snepulator gamepad mapping is unavailable: player-device assignment is runtime-only and player 1 defaults to keyboard; no config key persists it"
    )
}

#[cfg(test)]
mod tests {
    #[test]
    fn runtime_only_player_binding_is_explicitly_refused() {
        assert!(super::refusal().is_err());
        assert_eq!(super::SOURCE_COMMIT.len(), 40);
    }
}
