//! Explicit refusal for DREAMM controller serialization.
//!
//! DREAMM is closed-source (see `https://dreamm.aarongiles.com/docs/v40/#faq-opensource`).
//! The official docs describe SDL game-controller/joystick support with a
//! "Test Controller Mapping" screen, but publish no stable hand-authored
//! binding grammar for `config.json`. Per project policy, no writer is
//! fabricated for a closed artifact without a published grammar.

pub(crate) const SOURCE_DOC: &str = "https://dreamm.aarongiles.com/docs/v40/";

pub(crate) fn refusal() -> anyhow::Result<()> {
    anyhow::bail!(
        "DREAMM controller mapping is unavailable: closed-source with no published binding grammar"
    )
}

#[cfg(test)]
mod tests {
    #[test]
    fn closed_source_grammar_is_explicitly_refused() {
        assert!(super::refusal().is_err());
    }
}
