//! Saved physical sources -> measured SDL3 controls -> complete native profile.
use super::{configuration, settings::SavedSetup};
use crate::controller_catalog::Calibration;
use anyhow::{Context, Result, ensure};
use lunchbox_controller_probe::{
    Device,
    duckstation::{AnalogInput, DigitalInput},
    linux_classic::AxisEndpoints,
};
use std::collections::{BTreeMap, BTreeSet, HashMap};

pub(crate) struct Controller<'a> {
    pub controller_id: &'a str,
    pub device: &'a Device,
    pub native_device: &'a str,
}

pub(crate) fn controller_profile(
    setup: &SavedSetup,
    calibrations: &HashMap<String, Calibration>,
    controllers: &[Controller<'_>],
) -> Result<String> {
    setup.review(calibrations)?;
    ensure!(
        controllers.len() == setup.players.len(),
        "RPCS3 resolved controller count differs from saved setup"
    );
    let mut paths = BTreeSet::new();
    let mut identities = BTreeSet::new();
    for controller in controllers {
        let path = controller
            .device
            .path
            .as_deref()
            .context("RPCS3 physical device path is missing")?;
        ensure!(
            !path.is_empty() && paths.insert(path) && identities.insert(controller.controller_id),
            "RPCS3 controller identity or physical path is duplicated"
        );
    }
    let routes = super::visual_routes();
    let mut mapped = Vec::new();
    for player in &setup.players {
        let controller = controllers
            .iter()
            .find(|controller| controller.controller_id == player.controller_id)
            .context("RPCS3 saved controller is absent")?;
        let calibration = &calibrations[&player.controller_id];
        let device = controller.device;
        ensure!(
            device.is_gamepad,
            "RPCS3 SDL handler requires a recognized gamepad"
        );
        let gamepad = device
            .resolved
            .as_ref()
            .context("RPCS3 SDL3 resolved bindings are missing")?;
        let physical = device
            .linux_classic
            .as_ref()
            .context("RPCS3 physical backend needs verified classic-axis mapping")?;
        physical.validate_counts(gamepad)?;
        let mut controls = BTreeMap::new();
        let mut stick_axes = BTreeMap::new();
        for (target, source) in &player.source_controls {
            let binding = &calibration.bindings[source];
            let native = binding
                .native
                .as_ref()
                .context("RPCS3 native control identity is absent")?;
            let endpoints = binding.axis.as_ref().map(|axis| AxisEndpoints {
                released: axis.released,
                pressed: axis.pressed,
            });
            let input = physical.digital_input(native.code, endpoints)?;
            let stick = target.starts_with("stick_") || target.starts_with("right_stick_");
            let proportional_trigger = matches!(target.as_str(), "l2" | "r2")
                && matches!(input, DigitalInput::Axis { .. });
            let translated = if stick || proportional_trigger {
                let DigitalInput::Axis {
                    index,
                    released,
                    pressed,
                } = input
                else {
                    anyhow::bail!(
                        "RPCS3 stick directions require proportional axes, not buttons or hats"
                    );
                };
                if stick {
                    stick_axes.insert(target.as_str(), (index, released, pressed));
                }
                super::physical::analog(
                    gamepad,
                    AnalogInput {
                        index,
                        released,
                        extent: pressed,
                    },
                )?
            } else {
                super::physical::digital(gamepad, input)?
            };
            controls.insert(routes[target.as_str()].to_owned(), translated);
        }
        let mut axes = BTreeSet::new();
        for (negative, positive) in [
            ("stick_left", "stick_right"),
            ("stick_up", "stick_down"),
            ("right_stick_left", "right_stick_right"),
            ("right_stick_up", "right_stick_down"),
        ] {
            let a = stick_axes[negative];
            let b = stick_axes[positive];
            ensure!(
                a.0 == b.0
                    && a.1 == b.1
                    && (i32::from(a.2) - i32::from(a.1)).signum()
                        != (i32::from(b.2) - i32::from(b.1)).signum()
                    && axes.insert(a.0),
                "RPCS3 sticks require four distinct axes with matching opposite-direction calibration"
            );
        }
        mapped.push((player.player, controller.native_device, controls));
    }
    let players = mapped
        .iter()
        .map(|(player, native_device, controls)| configuration::Player {
            player: *player,
            device: *native_device,
            controls,
        })
        .collect::<Vec<_>>();
    configuration::render(&players, configuration::AnalogSettings::default())
}
