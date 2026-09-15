//! Explicit refusal for Geolith controller serialization.
//!
//! Pinned source: jgemu/geolith
//! `3c3038cc08c56851e7fefff1d5c7ac91d6a15768`. Geolith is a `jg` core
//! (`jg.c`): it exposes input through `jg_input` callbacks to a frontend
//! and ships no standalone controller persistence grammar of its own.
//! Controller selection lives in the frontend, so this catalog treats it
//! as frontend-bound like the libretro queue.

pub(crate) const SOURCE_COMMIT: &str = "3c3038cc08c56851e7fefff1d5c7ac91d6a15768";

pub(crate) fn refusal() -> anyhow::Result<()> {
    anyhow::bail!(
        "Geolith controller mapping is unavailable: jg core with frontend-owned input and no standalone grammar"
    )
}

#[cfg(test)]
mod tests {
    #[test]
    fn frontend_owned_input_is_explicitly_refused() {
        assert!(super::refusal().is_err());
        assert_eq!(super::SOURCE_COMMIT.len(), 40);
    }
}
