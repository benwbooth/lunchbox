//! Explicit refusal for Wataroo standalone controller serialization.
//!
//! Wataroo's documented `.joy` artifact describes device-specific joystick
//! profiles, but the closed Windows binary and physical DirectInput/XInput
//! identity semantics are not safely reproducible by a generic writer.

pub(crate) const SOURCE_URL: &str = "http://tailchao.com/Wataroo/";

pub(crate) fn refusal() -> anyhow::Result<()> {
    anyhow::bail!(
        "Wataroo controller mapping is unavailable: the closed Windows runtime owns DirectInput/XInput device identity and no deterministic generic writer is implemented"
    )
}

#[cfg(test)]
mod tests {
    #[test]
    fn closed_runtime_mapping_is_explicitly_refused() {
        assert!(super::refusal().is_err());
    }
}
