//! Explicit reviewed per-content setup; never match machines by display title.
use anyhow::{Result, ensure};
use std::collections::{BTreeMap, BTreeSet};
use std::path::{Component, PathBuf};

#[derive(Clone, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct NativeLaunchSettings {
    pub emulator_id: String,
    pub core: PathBuf,
    pub core_sha256: String,
    pub content: PathBuf,
    pub content_sha256: String,
    pub library: String,
    pub machine: String,
    pub inputs: Vec<super::InspectionInput>,
    pub persistent: super::PersistentPaths,
    pub frontend_save: PathBuf,
    pub frontend_state: PathBuf,
    pub players: BTreeMap<usize, String>,
    /// Missing in older setups: retain their original fixed-channel geometry.
    #[serde(default)]
    pub digital_layout: super::DigitalLayout,
    /// Per-source-player geometry; omitted ports inherit the shared preset.
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub player_digital_layouts: BTreeMap<usize, super::DigitalLayout>,
    /// Explicit native absolute-axis choices. Empty preserves digital-only setups.
    /// These are not physical calibration evidence or permission to auto-route.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub analog_assignments: Vec<super::AnalogAssignment>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub digital_assignments: Vec<super::DigitalAssignment>,
    /// Exact native relative fields and identified physical sources. Presence
    /// does not imply that the selected runtime has a relative launch adapter.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub relative_assignments: Vec<super::RelativeAssignment>,
    /// Stored planning data; staging/launch remain gated until mouse-button
    /// routing and native UI isolation are connected end to end.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub relative_button_assignments: Vec<super::relative_buttons::RelativeButtonAssignment>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub relative_sources: Vec<super::RelativeSource>,
    pub reviewed_snapshot: super::ActiveFieldSnapshot,
}

impl NativeLaunchSettings {
    pub(crate) fn selected_players(&self) -> BTreeSet<usize> {
        self.players
            .keys()
            .copied()
            .chain(
                self.relative_sources
                    .iter()
                    .map(|source| source.source_player),
            )
            .collect()
    }
    pub(crate) fn digital_layout_for(&self, player: usize) -> super::DigitalLayout {
        self.player_digital_layouts
            .get(&player)
            .copied()
            .unwrap_or(self.digital_layout)
    }
    pub(crate) fn identity_key(&self) -> String {
        serde_json::to_string(&(&self.emulator_id, &self.core, &self.content))
            .expect("MAME identity serializes")
    }
    pub(crate) fn validate(&self) -> Result<()> {
        // Staging, review and launch must never accept a retained legacy snapshot.
        self.reviewed_snapshot.validate(&self.machine)?;
        if !self.relative_button_assignments.is_empty() {
            self.reviewed_snapshot.require_game_mouse_mode()?;
        }
        ensure!(
            (self.relative_assignments.is_empty() && self.relative_button_assignments.is_empty())
                || cfg!(all(target_os = "linux", target_pointer_width = "64")),
            "Physical MAME relative input requires 64-bit Linux"
        );
        self.validate_stored()
    }

    /// Settings persistence may retain known legacy snapshots for reinspection.
    /// Runtime consumers must use validate/validate_review instead.
    pub(crate) fn validate_stored(&self) -> Result<()> {
        self.validate_structure(true)
    }

    /// Read-only review reports destination-profile failures per player. All
    /// native identities and assignment contracts still need full validation.
    pub(crate) fn validate_review_inputs(&self) -> Result<()> {
        self.reviewed_snapshot.validate(&self.machine)?;
        self.validate_structure(false)
    }

    fn validate_structure(&self, require_profiles: bool) -> Result<()> {
        ensure!(
            !self.emulator_id.is_empty() && self.emulator_id.len() <= 512,
            "Invalid MAME emulator identity"
        );
        for hash in [&self.core_sha256, &self.content_sha256] {
            ensure!(
                hash.len() == 64 && hash.bytes().all(|byte| byte.is_ascii_hexdigit()),
                "Invalid MAME identity hash"
            );
        }
        ensure!(
            !self.library.is_empty()
                && self.library.len() <= 128
                && self.library != "."
                && self.library != ".."
                && !self
                    .library
                    .chars()
                    .any(|c| c.is_control() || "/\\".contains(c)),
            "Invalid MAME library identity"
        );
        for path in [
            &self.core,
            &self.content,
            &self.frontend_save,
            &self.frontend_state,
        ]
        .into_iter()
        .map(|path| path.as_path())
        .chain(self.persistent.directories())
        {
            ensure!(
                path.is_absolute()
                    && path
                        .components()
                        .all(|part| matches!(part, Component::RootDir | Component::Normal(_))),
                "MAME setup paths must be normalized and absolute"
            );
        }
        ensure!(
            !self.selected_players().is_empty() && self.selected_players().len() <= 8,
            "MAME setup needs one to eight players"
        );
        let mut devices = BTreeSet::new();
        ensure!(
            self.player_digital_layouts
                .keys()
                .all(|port| self.players.contains_key(port)),
            "MAME layout overrides refer to an unselected player"
        );
        for (port, id) in &self.players {
            ensure!(
                (1..=8).contains(port) && !id.is_empty() && id.len() <= 1024 && devices.insert(id),
                "Invalid or duplicate MAME player assignment"
            );
        }
        ensure!(
            !self.inputs.is_empty() && self.inputs.len() <= 4096,
            "Invalid MAME dependency count"
        );
        let mut destinations = BTreeSet::new();
        for input in &self.inputs {
            ensure!(
                input.source.is_absolute(),
                "MAME dependency source must be absolute"
            );
            let parts: Vec<_> = input.destination.components().collect();
            ensure!(
                parts.len() >= 2
                    && parts
                        .iter()
                        .all(|part| matches!(part, Component::Normal(_)))
                    && matches!(
                        parts[0].as_os_str().to_str(),
                        Some("roms" | "cfg" | "ctrlr" | "system" | "nvram" | "diff")
                    )
                    && destinations.insert(&input.destination),
                "Invalid or duplicate MAME dependency destination"
            );
        }
        self.reviewed_snapshot.validate_stored(&self.machine)?;
        ensure!(
            self.relative_assignments.len() <= 32768
                && self.relative_button_assignments.len() <= 32768,
            "Too many MAME relative assignments"
        );
        if !self.relative_assignments.is_empty()
            || !self.relative_button_assignments.is_empty()
            || !self.relative_sources.is_empty()
        {
            super::relative_buttons::plan_relative_button_fields(
                &self.reviewed_snapshot,
                None,
                None,
                &self.selected_players(),
                &self.analog_assignments,
                &self.digital_assignments,
                &self.relative_assignments,
                &self.relative_button_assignments,
            )?;
            super::relative_buttons::prepare_sources(
                &self.relative_assignments,
                &self.relative_button_assignments,
                &self.relative_sources,
            )?;
        }
        ensure!(
            self.digital_assignments.len() <= 32768,
            "Too many MAME digital assignments"
        );
        if !self.digital_assignments.is_empty() {
            super::plan_explicit_fields(
                &self.reviewed_snapshot,
                None,
                None,
                &self.selected_players(),
                &self.analog_assignments,
                &self.digital_assignments,
            )?;
        }
        ensure!(
            self.analog_assignments.len() <= 32768,
            "Too many MAME analog assignments"
        );
        if !self.analog_assignments.is_empty() {
            ensure!(
                !self.reviewed_snapshot.needs_reinspection(),
                "MAME analog assignments require a current field inspection"
            );
            ensure!(
                self.analog_assignments
                    .iter()
                    .all(|assignment| self.players.contains_key(&assignment.source_player)),
                "MAME analog assignments refer to an unselected source player"
            );
            // Reuse exact-field/type/routing and duplicate validation, not a
            // weaker parallel validation of the saved representation.
            super::analog::controller_xml(&self.reviewed_snapshot, &self.analog_assignments)?;
        }
        ensure!(
            self.reviewed_snapshot.joystick_enabled,
            "Reviewed MAME joystick input is disabled"
        );
        // Keep current-schema profile checks intact. A legacy snapshot cannot
        // produce profiles until a new native inspection replaces it.
        if require_profiles && !self.reviewed_snapshot.needs_reinspection() {
            let selected = self.selected_players();
            let profile_snapshot = super::relative_buttons::gamepad_snapshot(
                &self.reviewed_snapshot,
                &self.relative_button_assignments,
            )?;
            for player in selected
                .iter()
                .filter(|player| !self.players.contains_key(player))
            {
                ensure!(
                    super::explicit_profile(
                        &profile_snapshot,
                        &selected,
                        &self.analog_assignments,
                        &self.digital_assignments,
                        *player,
                        super::DigitalLayout::Automatic
                    )?
                    .is_none(),
                    "Relative-only player {player} still requires gamepad channels; assign a gamepad or explicitly route those actions to another gamepad player"
                );
            }
            for player in self.players.keys() {
                // The common composer also handles zero overrides and empty
                // editable drafts, while checking final output completeness.
                // Staging and launch separately require usable profiles.
                super::explicit_profile(
                    &profile_snapshot,
                    &selected,
                    &self.analog_assignments,
                    &self.digital_assignments,
                    *player,
                    self.digital_layout_for(*player),
                )?;
            }
        }
        Ok(())
    }

    pub(crate) fn validate_review(&self, actual: &super::ActiveFieldSnapshot) -> Result<()> {
        self.validate()?;
        actual.validate(&self.machine)?;
        let normalize = |snapshot: &super::ActiveFieldSnapshot| {
            let mut snapshot = snapshot.clone();
            snapshot.joysticks.sort_by(|a, b| a.id.cmp(&b.id));
            if let Some(mouse) = &mut snapshot.mouse_state {
                mouse.devices.sort_by(|a, b| a.id.cmp(&b.id));
            }
            snapshot.analog_states.sort_by(|a, b| {
                (
                    &a.field.tag,
                    a.field.mask,
                    &a.field.input_type,
                    a.field.defvalue,
                )
                    .cmp(&(
                        &b.field.tag,
                        b.field.mask,
                        &b.field.input_type,
                        b.field.defvalue,
                    ))
            });
            snapshot.field_labels.sort_by(|a, b| {
                (
                    &a.field.tag,
                    a.field.mask,
                    &a.field.input_type,
                    a.field.defvalue,
                )
                    .cmp(&(
                        &b.field.tag,
                        b.field.mask,
                        &b.field.input_type,
                        b.field.defvalue,
                    ))
            });
            if let Some(keyboard) = &mut snapshot.keyboard_state {
                keyboard.devices.sort_by(|a, b| a.tag.cmp(&b.tag));
                keyboard.field_owners.sort_by(|a, b| {
                    (
                        &a.field.tag,
                        a.field.mask,
                        &a.field.input_type,
                        a.field.defvalue,
                    )
                        .cmp(&(
                            &b.field.tag,
                            b.field.mask,
                            &b.field.input_type,
                            b.field.defvalue,
                        ))
                });
            }
            snapshot.fields.sort_by(|a, b| {
                (&a.tag, &a.input_type, a.mask, a.defvalue, a.analog).cmp(&(
                    &b.tag,
                    &b.input_type,
                    b.mask,
                    b.defvalue,
                    b.analog,
                ))
            });
            snapshot
        };
        ensure!(
            normalize(actual) == normalize(&self.reviewed_snapshot),
            "MAME active fields or native routing changed; review the controller setup again"
        );
        Ok(())
    }
}
