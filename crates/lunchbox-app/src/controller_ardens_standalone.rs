//! Ardens standalone-native controller boundary.
//!
//! Pinned source: `tiberiusbrown/Ardens`
//! `661a7dd4febc8d00e790a4ecde44b936295adf97`.  The SDL desktop frontend
//! polls the first SDL gamepad and feeds its standard south/east/d-pad
//! buttons directly to ImGui keys; the six Arduboy controls are then read
//! from fixed keyboard keys in `common.cpp`. Ardens has no persisted host
//! controller identity or binding/profile schema.

pub(crate) const SOURCE_COMMIT: &str = "661a7dd4febc8d00e790a4ecde44b936295adf97";
pub(crate) const PROFILE_ID: &str = "ardens:standalone-native-fixed-input";

/// Mapping is intentionally refused: the upstream settings INI persists UI
/// and display choices only, while SDL gamepad selection and the keyboard
/// layout are runtime/fixed behavior.
pub(crate) fn refusal() -> anyhow::Result<()> {
    anyhow::bail!(
        "Ardens controller mapping is unavailable: upstream has fixed keyboard controls and runtime SDL gamepad polling, but no persistent controller profile or device selector"
    )
}

#[cfg(test)]
mod tests {
    #[test]
    fn fixed_input_boundary_is_explicitly_refused() {
        assert!(super::refusal().is_err());
        assert_eq!(super::SOURCE_COMMIT.len(), 40);
    }
}
