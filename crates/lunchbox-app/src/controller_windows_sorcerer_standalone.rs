//! Explicit refusal for Windows Sorcerer controller serialization.
//!
//! The preserved distribution documents DirectX joystick polling and an INI
//! file, but also says that the joystick is not configurable and keyboard
//! configuration is not working.  A Lunchbox writer must therefore not guess
//! a DirectInput identity or mutate the obsolete INI grammar.

pub(crate) const ARTIFACT_SHA256: &str =
    "b3171b5cf2020884c66cf5f3dee6481d230737492d914cc3797e6733e2b6c693";

pub(crate) fn refusal() -> anyhow::Result<()> {
    anyhow::bail!(
        "Windows Sorcerer controller mapping is unavailable: the pinned Windows95/98 artifact documents a DirectX joystick but says it is not configurable and provides no verified controller-profile serializer"
    )
}

#[cfg(test)]
mod tests {
    #[test]
    fn obsolete_directx_boundary_is_explicitly_refused() {
        assert!(super::refusal().is_err());
        assert_eq!(super::ARTIFACT_SHA256.len(), 64);
    }
}
