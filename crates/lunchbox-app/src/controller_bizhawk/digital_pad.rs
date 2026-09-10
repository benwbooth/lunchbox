//! Shared native digital-pad translation; core encoders retain their own contracts.
use anyhow::{Context, Result, ensure};
use std::collections::BTreeMap;

/// Shared semantic resolution for saved-input validation and native translation.
pub(super) fn resolve_pad(
    target_id: &str,
    calibration: &crate::controller_catalog::Calibration,
    logical: Option<&super::LogicalCalibration>,
) -> Result<crate::controller_layout::Resolution> {
    let catalog = crate::controller_catalog::catalog();
    let source = catalog
        .layout(&calibration.layout)
        .context("Unknown physical layout")?;
    let target = catalog
        .layout(target_id)
        .context("Missing native digital layout")?;
    let available = super::calibrated_control_ids(calibration, logical, None);
    let requested = super::requested_control_ids(target, false);
    super::guided::resolve(calibration, source, target, &available, &requested)
}

/// Saved-input completeness only; this does not prove native SDL eligibility.
pub(crate) fn validate_saved_pad(
    target_id: &str,
    native_buttons: &[&str],
    calibration: &crate::controller_catalog::Calibration,
) -> Result<()> {
    calibration.validate()?;
    ensure!(
        calibration.os == "linux",
        "Native digital-pad requires Linux calibration"
    );
    let resolution = resolve_pad(target_id, calibration, None)?;
    for &name in native_buttons {
        let assigned = resolution
            .assignments
            .get(&name.to_ascii_lowercase())
            .with_context(|| format!("No calibrated {target_id} assignment for {name}"))?;
        ensure!(
            calibration
                .bindings
                .get(assigned)
                .and_then(|input| input.native.as_ref())
                .is_some(),
            "{target_id} {name} source {assigned} has no native input identity"
        );
    }
    Ok(())
}

/// Translate against the actual probed SDL device, never a saved device index.
pub(crate) fn translate_pad(
    target_id: &str,
    native_buttons: &[&str],
    calibration: &crate::controller_catalog::Calibration,
    logical: Option<&super::LogicalCalibration>,
    inventory: &lunchbox_controller_probe::sdl2::Snapshot,
    path: &str,
) -> Result<(BTreeMap<String, String>, Vec<String>)> {
    calibration.validate()?;
    ensure!(
        calibration.os == "linux" && std::env::consts::OS == "linux",
        "Native digital-pad translation requires Linux calibration"
    );
    let device = inventory.device_at_path(path)?;
    let logical = if device.is_game_controller {
        let logical =
            logical.context("Capture logical SDL bindings for this recognized controller first")?;
        logical.validate()?;
        logical.context.ensure_current(inventory, path)?;
        ensure!(
            logical.layout == calibration.layout,
            "Physical and logical SDL layouts disagree"
        );
        Some(logical)
    } else {
        None
    };
    let resolution = resolve_pad(target_id, calibration, logical)?;
    let physical = if logical.is_none() {
        Some(super::PhysicalMap::from_device(device)?)
    } else {
        None
    };
    let mut buttons = BTreeMap::new();
    let mut native_sources = std::collections::BTreeSet::new();
    let mut warnings = Vec::new();
    for &name in native_buttons {
        let id = name.to_ascii_lowercase();
        let assigned = resolution
            .assignments
            .get(&id)
            .with_context(|| format!("No calibrated {target_id} assignment for {name}"))?;
        let input = if let Some(logical) = logical {
            let gesture = logical
                .bindings
                .get(assigned)
                .context("Resolved SDL gesture is absent")?;
            super::logical_digital_source(device, gesture)?
        } else {
            let input = calibration
                .bindings
                .get(assigned)
                .context("Resolved physical input is absent")?;
            let raw = super::raw_digital_from_calibrated_input(
                device,
                physical.context("Missing SDL physical mapping")?,
                input,
            )?;
            super::raw_digital_source(device, raw)?
        };
        ensure!(
            native_sources.insert(input.clone()),
            "{target_id} controls share native input {input}"
        );
        buttons.insert(name.to_owned(), input);
        if let Some(rule) = resolution.rules.get(&id) {
            if !matches!(rule, crate::controller_layout::Rule::Identity) {
                warnings.push(format!("{name} uses {assigned}: {}", rule.description()));
            }
        }
    }
    Ok((buttons, warnings))
}
