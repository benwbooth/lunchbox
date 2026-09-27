//! Magnavody standalone-native controller boundary.
//!
//! Pinned source: `dodgyville/magnavody`
//! `b07ecc571f26ba6c934a7b9cc05557e396284984`. The Godot project has a
//! source-backed InputMap and ResourceSaver override, but joypads are selected
//! by runtime Godot indices and the `.tres` object graph is not a portable
//! controller profile. Keep the refusal explicit until a Godot fixture and
//! launch oracle exist.

pub(crate) const SOURCE_COMMIT: &str = "b07ecc571f26ba6c934a7b9cc05557e396284984";

pub(crate) fn refusal() -> anyhow::Result<()> {
    anyhow::bail!(
        "Magnavody controller mapping is unavailable: the native override is a Godot ResourceSaver .tres object graph and joypad selection is a runtime device index, not a stable physical identity"
    )
}

#[cfg(test)]
mod tests {
    #[test]
    fn godot_resource_boundary_is_explicitly_refused() {
        assert!(super::refusal().is_err());
        assert_eq!(super::SOURCE_COMMIT.len(), 40);
    }
}
