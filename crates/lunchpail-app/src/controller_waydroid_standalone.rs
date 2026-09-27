//! Explicit refusal for Waydroid standalone-native controller serialization.
//!
//! Waydroid's INI configuration selects Android images and property
//! overrides. Android app input and compositor/container forwarding are
//! runtime- and frontend-owned, so `waydroid.cfg` is not a controller profile.

pub(crate) const SOURCE_COMMIT: &str = "5a51271131bfca8b7ee75ed067d09b26460f3a7b";

pub(crate) fn refusal() -> anyhow::Result<()> {
    anyhow::bail!(
        "Waydroid controller mapping is unavailable: waydroid.cfg configures images and Android properties, while app/controller input is runtime- and frontend-owned"
    )
}

#[cfg(test)]
mod tests {
    #[test]
    fn android_container_input_boundary_is_explicitly_refused() {
        assert!(super::refusal().is_err());
        assert_eq!(super::SOURCE_COMMIT.len(), 40);
    }
}
