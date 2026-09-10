//! Native SDL3 launch-time inventory, calibration ownership and the private
//! stella.sqlite3 writer inside the persistent -basedir. The user's own
//! Stella database is never opened.
use super::{Binding, Stick, settings};
use crate::{
    controller_bizhawk_guard::InputTopology,
    controller_catalog::Calibration,
    controller_native_process::{cancelled, capture},
    controllers::ControllerDevice,
};
use anyhow::{Context, Result, ensure};
use lunchbox_controller_probe::{Snapshot, file_hash, linux_classic::AxisEndpoints};
use std::{
    collections::{BTreeMap, HashMap},
    process::Command,
    sync::atomic::AtomicBool,
};

fn observe(
    setup: &settings::SavedSetup,
    path: Option<&str>,
    cancel: &AtomicBool,
) -> Result<Snapshot> {
    let mut command = Command::new(&setup.probe_program);
    command
        .arg("--sdl-library")
        .arg(&setup.sdl_library)
        .arg("--hint")
        .arg("SDL_JOYSTICK_LINUX_CLASSIC=1");
    if let Some(path) = path {
        command.arg("--bindings-for-path").arg(path);
    }
    let (output, _) = capture(&mut command, cancel)?;
    let snapshot: Snapshot =
        serde_json::from_slice(&output).context("Invalid Stella SDL capture")?;
    ensure!(
        snapshot.library.canonicalize()? == setup.sdl_library.canonicalize()?
            && snapshot.library_sha256 == file_hash(&setup.sdl_library)?
            && snapshot
                .effective_hints
                .get("SDL_JOYSTICK_LINUX_CLASSIC")
                .and_then(Option::as_deref)
                == Some("1"),
        "Stella helper inspected a different SDL runtime or backend"
    );
    Ok(snapshot)
}

/// Translate one player's calibrated controls into Stella mapping entries.
/// `events` maps this stick's target controls (already port-expanded) to the
/// persisted event names; console switches only appear on player one.
pub(super) fn calibrated_stick(
    calibration: &Calibration,
    snapshot: &Snapshot,
    runtime_path: &str,
    port: &'static str,
    events: &BTreeMap<&'static str, &'static str>,
) -> Result<Stick> {
    use lunchbox_controller_probe::duckstation::DigitalInput;
    ensure!(
        calibration.os == "linux",
        "Stella native calibration requires Linux"
    );
    let device = snapshot.device_at_path(runtime_path)?;
    let name = device
        .name
        .clone()
        .context("Stella stick has no SDL name")?;
    let classic = device
        .linux_classic
        .as_ref()
        .context("Stella classic joystick numbering is absent; the pinned classic backend did not capture it")?;
    let profile = crate::controller_catalog::catalog()
        .emulator_profiles
        .iter()
        .find(|profile| profile.id == settings::PROFILE_ID)
        .context("Missing native Stella profile")?;
    let mut joystick_mode: BTreeMap<&'static str, Binding> = BTreeMap::new();
    for row in calibration.plan_profile(profile)?.rows {
        let event = events.get(row.target_id.as_str()).with_context(|| {
            format!(
                "Stella target {} is outside this port's contract",
                row.target_id
            )
        })?;
        let input = row
            .input
            .context("Stella gameplay control is not calibrated")?;
        let native = input
            .native
            .as_ref()
            .context("Stella requires measured native controls")?;
        let measured = input.axis.as_ref().map(|axis| AxisEndpoints {
            released: axis.released,
            pressed: axis.pressed,
        });
        let binding = match classic.digital_input(native.code, measured)? {
            DigitalInput::Button(index) => Binding::Button(index),
            DigitalInput::Axis {
                index,
                released,
                pressed,
            } => {
                ensure!(
                    (-super::DIGITAL_THRESHOLD..super::DIGITAL_THRESHOLD)
                        .contains(&i32::from(released)),
                    "Stella axis rest would hold a digital direction pressed; recalibrate the rest position"
                );
                if i32::from(pressed) > super::DIGITAL_THRESHOLD {
                    Binding::Axis {
                        index,
                        negative: false,
                    }
                } else if i32::from(pressed) < -super::DIGITAL_THRESHOLD {
                    Binding::Axis {
                        index,
                        negative: true,
                    }
                } else {
                    anyhow::bail!(
                        "Stella's default digital dead zone (16200) cannot represent the measured axis travel"
                    );
                }
            }
            DigitalInput::Hat { index, direction } => {
                const LEFT: u8 = 8;
                const UP: u8 = 1;
                let negative_half = direction & (LEFT | UP) != 0;
                let input = index
                    .checked_mul(2)
                    .context("Stella hat index overflows the hat field")?
                    + u32::from(!negative_half);
                Binding::Hat {
                    input,
                    up: negative_half,
                }
            }
        };
        ensure!(
            joystick_mode.insert(event, binding).is_none(),
            "Duplicate Stella gameplay event"
        );
    }
    Ok(Stick {
        name,
        port,
        joystick_mode,
    })
}

/// Events for one player: joystick controls expand JoystickZero→JoystickOne
/// for the right port; console switches stay on player one only.
fn port_events(player: u8) -> BTreeMap<&'static str, &'static str> {
    super::CONTROLS
        .iter()
        .filter_map(|(control, event)| {
            let expanded: &'static str = if player == 2 && super::PLAYER_EVENTS.contains(event) {
                match *event {
                    "JoystickZeroUp" => "JoystickOneUp",
                    "JoystickZeroDown" => "JoystickOneDown",
                    "JoystickZeroLeft" => "JoystickOneLeft",
                    "JoystickZeroRight" => "JoystickOneRight",
                    "JoystickZeroFire" => "JoystickOneFire",
                    "JoystickZeroFire5" => "JoystickOneFire5",
                    "JoystickZeroFire9" => "JoystickOneFire9",
                    _ => unreachable!("player events are exactly the joystick events"),
                }
            } else if player == 2 {
                return None;
            } else {
                event
            };
            Some((*control, expanded))
        })
        .collect()
}

pub(crate) struct PreparedSession {
    setup: settings::SavedSetup,
    runtime_paths: Vec<String>,
    topology: InputTopology,
    initial: Snapshot,
    joymap: String,
}

impl PreparedSession {
    pub(crate) fn prepare(
        setup: &settings::SavedSetup,
        calibrations: &HashMap<String, Calibration>,
        inventory: &[ControllerDevice],
        cancel: &AtomicBool,
    ) -> Result<Self> {
        cancelled(cancel)?;
        setup.review(calibrations)?;
        let mut selected = Vec::new();
        for player in &setup.players {
            let mut matches = inventory
                .iter()
                .filter(|device| device.stable_id == player.controller_id);
            let device = matches
                .next()
                .context("Stella selected controller is disconnected")?;
            ensure!(
                matches.next().is_none() && !device.is_virtual,
                "Stella requires an unambiguous physical controller"
            );
            selected.push(device.device_path.clone());
        }
        let topology = InputTopology::capture(&selected)?;
        let initial = observe(setup, None, cancel)?;
        let mut sticks = Vec::new();
        let mut names: Vec<String> = Vec::new();
        let mut runtime_paths = Vec::new();
        for (player, selected) in setup.players.iter().zip(&selected) {
            let path = topology.resolve_runtime_path(
                selected,
                initial
                    .devices
                    .iter()
                    .filter_map(|device| device.path.as_deref()),
            )?;
            ensure!(
                !runtime_paths.contains(&path),
                "Stella players resolved to the same native controller"
            );
            let captured = observe(setup, Some(&path), cancel)?;
            topology.verify()?;
            let stick = calibrated_stick(
                calibrations
                    .get(&player.controller_id)
                    .context("Stella calibration disappeared")?,
                &captured,
                &path,
                if player.player == 1 { "Left" } else { "Right" },
                &port_events(player.player),
            )?;
            names.push(stick.name.clone());
            sticks.push(stick);
            runtime_paths.push(path);
        }
        // Stella renames same-name sticks when adding them in enumeration
        // order; the persisted names must match what the launch will see.
        let unique = super::unique_names(&names);
        for (stick, name) in sticks.iter_mut().zip(unique) {
            stick.name = name;
        }
        let joymap = super::joymap(&sticks)?;
        let session = Self {
            setup: setup.clone(),
            runtime_paths,
            topology,
            initial,
            joymap,
        };
        session.verify(cancel)?;
        Ok(session)
    }

    /// Write or update the private settings database. Only `event_ver` and
    /// `joymap` are owned; saves, states and other settings under the basedir
    /// are preserved.
    pub(crate) fn write_database(&self) -> Result<()> {
        use rusqlite::params;
        std::fs::create_dir_all(&self.setup.base_directory)?;
        let mut connection = rusqlite::Connection::open(self.setup.database_path())
            .with_context(|| format!("Opening {}", self.setup.database_path().display()))?;
        let transaction = connection.transaction()?;
        transaction.execute(
            "CREATE TABLE IF NOT EXISTS `settings` \
             (`setting` TEXT PRIMARY KEY, `value` TEXT) WITHOUT ROWID",
            [],
        )?;
        transaction.execute(
            "INSERT OR REPLACE INTO `settings` VALUES (?, ?)",
            params!["joymap", self.joymap],
        )?;
        transaction.execute(
            "INSERT OR REPLACE INTO `settings` VALUES (?, ?)",
            params!["event_ver", "9"],
        )?;
        transaction.commit()?;
        let stored: String = connection
            .query_row(
                "SELECT `value` FROM `settings` WHERE `setting` = 'joymap'",
                [],
                |row| row.get(0),
            )
            .context("Re-reading the staged Stella joymap")?;
        ensure!(
            stored == self.joymap,
            "Stella settings database did not retain the staged mappings"
        );
        Ok(())
    }

    pub(crate) fn verify(&self, cancel: &AtomicBool) -> Result<()> {
        cancelled(cancel)?;
        self.topology.verify()?;
        let fresh = observe(&self.setup, None, cancel)?;
        for path in &self.runtime_paths {
            fresh
                .device_at_path(path)
                .with_context(|| format!("Stella controller disappeared: {path}"))?;
        }
        self.topology.verify()
    }

    pub(crate) fn check_health(&self) -> Result<()> {
        self.topology.verify()
    }

    pub(crate) fn staged_joymap(&self) -> &str {
        &self.joymap
    }

    /// Launch arguments select the private -basedir ahead of the game.
    pub(crate) fn overlay_arguments(
        &self,
        arguments: &[std::ffi::OsString],
    ) -> Result<Vec<std::ffi::OsString>> {
        ensure!(
            self.setup.base_directory.is_absolute(),
            "Stella basedir must be absolute"
        );
        let mut result = Vec::with_capacity(arguments.len() + 2);
        result.push("-basedir".into());
        result.push(self.setup.base_directory.as_os_str().to_owned());
        result.extend_from_slice(arguments);
        Ok(result)
    }
}

/// The settings-table keys this adapter owns inside the private basedir.
pub(crate) const OWNED_KEYS: [&str; 2] = ["event_ver", "joymap"];
