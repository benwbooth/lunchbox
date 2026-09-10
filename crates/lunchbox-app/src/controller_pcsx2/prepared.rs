//! Saved physical sources -> measured SDL3 controls -> complete native profile.
use super::{profile, settings::SavedSetup};
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
    pub sdl_player: u8,
}

pub(crate) fn controller_profile(
    setup: &SavedSetup,
    calibrations: &HashMap<String, Calibration>,
    controllers: &[Controller<'_>],
) -> Result<String> {
    setup.review(calibrations)?;
    ensure!(
        controllers.len() == setup.players.len(),
        "PCSX2 resolved controller count differs from saved setup"
    );
    let mut paths = BTreeSet::new();
    let mut identities = BTreeSet::new();
    for controller in controllers {
        let path = controller
            .device
            .path
            .as_deref()
            .context("PCSX2 physical device path is missing")?;
        ensure!(
            !path.is_empty() && paths.insert(path) && identities.insert(controller.controller_id),
            "PCSX2 controller identity or physical path is duplicated"
        );
    }
    let routes = super::visual_routes();
    let mut mapped = Vec::new();
    for player in &setup.players {
        let controller = controllers
            .iter()
            .find(|controller| controller.controller_id == player.controller_id)
            .context("PCSX2 saved controller is absent")?;
        let calibration = &calibrations[&player.controller_id];
        let device = controller.device;
        let gamepad = device
            .resolved
            .as_ref()
            .context("PCSX2 SDL3 resolved bindings are missing")?;
        let physical = device
            .linux_classic
            .as_ref()
            .context("PCSX2 physical backend needs verified classic-axis mapping")?;
        physical.validate_counts(gamepad)?;
        let mut controls = BTreeMap::new();
        let mut stick_axes = BTreeMap::new();
        for (target, source) in &player.source_controls {
            let binding = &calibration.bindings[source];
            let native = binding
                .native
                .as_ref()
                .context("PCSX2 native control identity is absent")?;
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
                        "PCSX2 stick directions require proportional axes, not buttons or hats"
                    );
                };
                if stick {
                    stick_axes.insert(target.as_str(), (index, released, pressed));
                }
                super::physical::analog(
                    gamepad,
                    device.is_gamepad,
                    AnalogInput {
                        index,
                        released,
                        extent: pressed,
                    },
                )?
            } else {
                super::physical::digital(gamepad, device.is_gamepad, input)?
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
                "PCSX2 sticks require four distinct axes with matching opposite-direction calibration"
            );
        }
        mapped.push((player.player, controller.sdl_player, controls));
    }
    let players = mapped
        .iter()
        .map(|(player, sdl_player, controls)| profile::Player {
            player: *player,
            sdl_player: *sdl_player,
            controls,
        })
        .collect::<Vec<_>>();
    profile::controller_profile(setup.multitaps, &players)
}
