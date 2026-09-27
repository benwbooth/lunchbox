//! Explicit refusal for WASM-4 native controller serialization.
//!
//! The native WASM-4 runtime has fixed keyboard bindings and stores cartridge
//! data beside the cartridge. It has no stable host controller-profile file.

pub(crate) const SOURCE_COMMIT: &str = "9d6c962785cfe3719d0245fd279ecbe98a4dbb63";

pub(crate) fn refusal() -> anyhow::Result<()> {
    anyhow::bail!(
        "WASM-4 native controller mapping is unavailable: the reviewed runtime uses fixed keyboard bindings and exposes no deterministic profile grammar"
    )
}

#[cfg(test)]
mod tests {
    #[test]
    fn fixed_runtime_mapping_is_explicitly_refused() {
        assert!(super::refusal().is_err());
    }
}
