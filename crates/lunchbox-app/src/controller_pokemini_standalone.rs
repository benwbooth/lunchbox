//! Explicit refusal for PokeMini controller serialization.

pub(crate) const ARTIFACT_SHA256: &str =
    "19de65332abe6d203c7065a70764202f28c974fa851b67cf0fe689a5d5c1f17e";

pub(crate) fn refusal() -> anyhow::Result<()> {
    anyhow::bail!(
        "PokeMini controller mapping is unavailable: v0.60 exposes fixed keyboard defaults and launch-time joystick selection, not a persistent profile grammar"
    )
}

#[cfg(test)]
mod tests {
    #[test]
    fn launch_only_mapping_is_explicitly_refused() {
        assert!(super::super::controller_pokemini_standalone::refusal().is_err());
    }
}
