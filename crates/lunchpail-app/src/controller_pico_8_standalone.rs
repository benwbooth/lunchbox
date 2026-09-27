//! Explicit refusal for PICO-8 controller config serialization.

pub(crate) const SOURCE_DOC: &str = "https://www.lexaloffle.com/dl/docs/pico-8_manual.txt";

pub(crate) fn refusal() -> anyhow::Result<()> {
    anyhow::bail!(
        "PICO-8 controller mapping is unavailable: manual documents SDL2/KEYCONFIG but not a stable patchable profile grammar"
    )
}

#[cfg(test)]
mod tests {
    #[test]
    fn proprietary_profile_grammar_is_explicitly_refused() {
        assert!(super::super::controller_pico_8_standalone::refusal().is_err());
    }
}
