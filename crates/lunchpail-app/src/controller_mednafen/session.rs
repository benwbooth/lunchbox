//! Native Linux controller identity and config lifetime for a saved setup.
use super::{inventory::Inventory, isolation::PreparedConfig, settings::SavedSetup};
use crate::{
    controller_bizhawk_guard::InputTopology, controller_catalog::Calibration,
    controller_native_process::cancelled, controllers::ControllerDevice,
};
use anyhow::{Context, Result, ensure};
use std::{collections::HashMap, sync::atomic::AtomicBool};

pub(crate) struct PreparedSession {
    pub(crate) configuration: PreparedConfig,
    pub(crate) runtime_paths: Vec<String>,
    inventory: Inventory,
    topology: InputTopology,
    nes_patch_absence: Option<super::ips::Snapshot>,
}

impl PreparedSession {
    pub(crate) fn prepare(
        setup: &SavedSetup,
        file_base: &str,
        calibrations: &HashMap<String, Calibration>,
        controllers: &[ControllerDevice],
        cancel: &AtomicBool,
    ) -> Result<Self> {
        cancelled(cancel)?;
        setup.review(calibrations)?;
        // Native LoadIPS uses the outside ROM path plus .ips before module
        // loading. Retain the patch or its absence through child startup.
        let nes_patch_absence = if setup.gamepad.system() == "nes" {
            Some(super::ips::Snapshot::capture(&setup.content)?)
        } else {
            None
        };
        let mut selected_paths = Vec::new();
        for player in &setup.players {
            let mut matches = controllers
                .iter()
                .filter(|device| device.stable_id == player.controller_id);
            let device = matches
                .next()
                .context("Mednafen selected controller is disconnected")?;
            ensure!(
                matches.next().is_none() && !device.is_virtual,
                "Mednafen requires unambiguous physical controllers"
            );
            selected_paths.push(device.device_path.clone());
        }
        let topology = InputTopology::capture(&selected_paths)?;
        let inventory = Inventory::capture(cancel)?;
        let discovered = super::discovery::Discovered::capture(
            &setup.base_directory,
            setup.gamepad.system(),
            file_base,
        )?;
        let threshold = super::configuration::axis_threshold(&discovered.effective()?)?;
        let mut assignments = std::collections::BTreeMap::new();
        let mut native_owners = std::collections::BTreeSet::new();
        let mut runtime_paths = Vec::new();
        let mut snes_players = Vec::new();
        let mut md_players = Vec::new();
        let mut saturn_players = Vec::new();
        let mut psx_players = Vec::new();
        for (player, selected_path) in setup.players.iter().zip(&selected_paths) {
            cancelled(cancel)?;
            let runtime = topology.resolve_runtime_path(
                selected_path,
                inventory
                    .devices
                    .iter()
                    .filter_map(|device| device.path.to_str()),
            )?;
            let device = inventory
                .devices
                .iter()
                .find(|device| device.path.to_str() == Some(runtime.as_str()))
                .context("Mednafen selected native joystick disappeared")?;
            ensure!(
                native_owners.insert(device.native_id.clone()),
                "Mednafen players resolve to the same native joystick"
            );
            let state = super::state::capture(&device.identity, &device.map)?;
            let gamepad = setup.player_gamepad(player)?;
            let controls = device.map.calibrated(
                calibrations
                    .get(&player.controller_id)
                    .context("Mednafen calibration disappeared")?,
                gamepad,
                &device.native_id,
                &state,
                threshold,
            )?;
            if gamepad.system() == "psx" {
                psx_players.push((player.player, gamepad, device.native_id.clone(), controls));
            } else if gamepad.system() == "ss" {
                saturn_players.push((player.player, device.native_id.clone(), controls));
            } else if gamepad.system() == "md" {
                md_players.push((player.player, gamepad, device.native_id.clone(), controls));
            } else if matches!(
                gamepad,
                super::profiles::Gamepad::Snes | super::profiles::Gamepad::SnesFaust
            ) {
                snes_players.push((player.player, device.native_id.clone(), controls));
            } else {
                for (key, value) in
                    gamepad.assignments_for_player(player.player, &device.native_id, &controls)?
                {
                    ensure!(
                        assignments.insert(key, value).is_none(),
                        "Mednafen native port assignment collision"
                    );
                }
            }
            runtime_paths.push(runtime);
            runtime_paths.push(
                device
                    .event
                    .to_str()
                    .context("Mednafen event path is not UTF-8")?
                    .to_owned(),
            );
        }
        if setup.gamepad.system() == "psx" {
            let players: Vec<_> = psx_players
                .iter()
                .map(|(port, gamepad, native_id, controls)| {
                    (
                        *gamepad == super::profiles::Gamepad::PlayStationDualAnalog,
                        super::psx::Player {
                            port: *port,
                            native_id,
                            controls,
                        },
                    )
                })
                .collect();
            assignments =
                super::psx::mixed_assignments(setup.psx_multitaps.unwrap_or([false; 2]), &players)?;
        } else if setup.gamepad.system() == "ss" {
            let players: Vec<_> = saturn_players
                .iter()
                .map(|(port, native_id, controls)| super::saturn::Player {
                    port: *port,
                    native_id,
                    controls,
                })
                .collect();
            assignments =
                super::saturn::assignments(setup.saturn_multitaps.unwrap_or([false; 2]), &players)?;
        } else if setup.gamepad.system() == "md" {
            let players: Vec<_> = md_players
                .iter()
                .map(|(port, gamepad, native_id, controls)| super::md::Player {
                    port: *port,
                    pad: if *gamepad == super::profiles::Gamepad::MdThree {
                        super::md::Pad::Three
                    } else {
                        super::md::Pad::Six
                    },
                    native_id,
                    controls,
                })
                .collect();
            assignments =
                super::md::assignments(setup.md_tap.unwrap_or(super::md::Tap::None), &players)?;
        } else if matches!(
            setup.gamepad,
            super::profiles::Gamepad::Snes | super::profiles::Gamepad::SnesFaust
        ) {
            let players: Vec<_> = snes_players
                .iter()
                .map(|(port, native_id, controls)| super::snes::Player {
                    port: *port,
                    native_id,
                    controls,
                })
                .collect();
            assignments = if setup.gamepad == super::profiles::Gamepad::SnesFaust {
                super::snes::faust_assignments(&players)?
            } else {
                super::snes::assignments(&players)?
            };
        } else {
            for player in 1..=setup.gamepad.max_players() {
                if !setup.players.iter().any(|saved| saved.player == player) {
                    assignments.extend(setup.gamepad.unused_port(player)?);
                }
            }
        }
        if setup.gamepad.system() == "pce" {
            assignments.insert(
                "pce.input.multitap".into(),
                if setup.players.iter().any(|player| player.player > 1) {
                    "1"
                } else {
                    "0"
                }
                .into(),
            );
        }
        if setup.gamepad.system() == "nes" {
            use super::profiles::Gamepad;
            use std::io::Read;
            for port in 1..=4 {
                assignments
                    .entry(format!("nes.input.port{port}"))
                    .or_insert_with(|| "none".into());
            }
            assignments.insert(
                "nes.nofs".into(),
                if setup.gamepad == Gamepad::NesFourScore {
                    "0"
                } else {
                    "1"
                }
                .into(),
            );
            assignments.insert(
                "nes.input.fcexp".into(),
                if setup.gamepad == Gamepad::NesFamicomFour {
                    "4player"
                } else {
                    "none"
                }
                .into(),
            );
            let file = std::fs::File::open(&setup.content)?;
            ensure!(
                file.metadata()?.is_file(),
                "NES content must be a regular ROM file"
            );
            let mut bytes = Vec::new();
            file.take(16 * 1024 * 1024 + 1).read_to_end(&mut bytes)?;
            ensure!(
                bytes.len() <= 16 * 1024 * 1024,
                "NES ROM exceeds preparation limit"
            );
            let patched = nes_patch_absence
                .as_ref()
                .context("NES patch capture missing")?
                .apply(&bytes)?;
            let desired = super::nes_content::desired(&patched)?;
            super::nes_desired::reconcile_unused_pads(&desired, &mut assignments)?;
        }
        inventory.verify(cancel)?;
        topology.verify()?;
        let configuration = PreparedConfig::prepare_assignments(discovered, &assignments)?;
        let session = Self {
            configuration,
            runtime_paths,
            inventory,
            topology,
            nes_patch_absence,
        };
        session.verify(cancel)?;
        Ok(session)
    }

    pub(crate) fn verify(&self, cancel: &AtomicBool) -> Result<()> {
        cancelled(cancel)?;
        self.topology.verify()?;
        if let Some(absence) = &self.nes_patch_absence {
            absence.verify()?;
        }
        self.inventory.verify(cancel)?;
        self.configuration.verify()
    }

    pub(crate) fn check_health(&self) -> Result<()> {
        self.topology.verify()?;
        self.inventory.check_health()
    }
}
