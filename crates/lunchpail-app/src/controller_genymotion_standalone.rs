//! Explicit refusal for Genymotion controller serialization.
//!
//! Genymotion is a commercial Android virtual-device product
//! (`https://www.genymotion.com/help/desktop/`). Input is Android-level
//! touch/sensor mapping with no published hand-authored gamepad binding
//! grammar, so no writer is fabricated.

pub(crate) const SOURCE_DOC: &str = "https://www.genymotion.com/help/desktop/";

pub(crate) fn refusal() -> anyhow::Result<()> {
    anyhow::bail!(
        "Genymotion controller mapping is unavailable: commercial product with no published binding grammar"
    )
}

#[cfg(test)]
mod tests {
    #[test]
    fn commercial_grammar_is_explicitly_refused() {
        assert!(super::refusal().is_err());
    }
}
