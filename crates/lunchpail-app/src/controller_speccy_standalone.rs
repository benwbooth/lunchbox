//! Explicit refusal for Speccy controller serialization.
//!
//! Speccy by Marat Fayzullin is proprietary: the official site offers the
//! source code under a commercial license only
//! (`https://fms.komkon.org/Speccy/`). No inspectable persistence grammar
//! exists, so no writer is fabricated.

pub(crate) const SOURCE_DOC: &str = "https://fms.komkon.org/Speccy/";

pub(crate) fn refusal() -> anyhow::Result<()> {
    anyhow::bail!(
        "Speccy controller mapping is unavailable: proprietary with no published binding grammar"
    )
}

#[cfg(test)]
mod tests {
    #[test]
    fn proprietary_grammar_is_explicitly_refused() {
        assert!(super::refusal().is_err());
    }
}
