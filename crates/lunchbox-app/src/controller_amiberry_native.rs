//! Amiberry native SDL gamepad mappings for standalone Amiga joystick ports.
//!
//! This is a source-backed configuration writer, not a claim that an Amiberry
//! binary or an Amiga game has been exercised.  The source pin is
//! BlitterStudio/amiberry 06ff25093b620deef734a395189a1c564ed8beac.
//!
//! Amiberry uses SDL's `gamecontrollerdb.txt` grammar for the physical-to-
//! logical translation and the `.uae` file's `joyportN=joyN`/`joyportNmode`
//! options for port selection.  Keeping those two layers separate is
//! important: a `.uae` file cannot safely contain guessed raw SDL button
//! numbers when SDL's controller database resolves the logical gamepad.

use anyhow::{Result, ensure};
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};

pub(crate) const SOURCE_COMMIT: &str = "06ff25093b620deef734a395189a1c564ed8beac";
pub(crate) const PROFILE_ID: &str = "amiberry:standalone-amiberry-amiga";

/// The classic Amiga joystick controls represented by Amiberry's normal
/// `gamepad` port mode.  Fire, second-fire and third-fire are SDL south, east
/// and west respectively; the source maps those to Joy1/Joy2 button events.
pub(crate) const CONTROLS: [(&str, &str); 7] = [
    ("up", "dpup"),
    ("down", "dpdown"),
    ("left", "dpleft"),
    ("right", "dpright"),
    ("fire", "a"),
    ("fire2", "b"),
    ("fire3", "x"),
];

/// One raw SDL joystick control in SDL's gamecontroller database syntax.
#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub(crate) enum Binding {
    Button(u32),
    Axis { index: u32, positive: bool },
    Hat { index: u32, mask: u8 },
}

impl Binding {
    fn sdl_spec(self) -> Result<String> {
        match self {
            Self::Button(index) => {
                ensure!(index < 256, "Amiberry SDL button index is out of range");
                Ok(format!("b{index}"))
            }
            Self::Axis { index, positive } => {
                ensure!(index < 256, "Amiberry SDL axis index is out of range");
                Ok(format!("a{index}{}", if positive { "+" } else { "-" }))
            }
            Self::Hat { index, mask } => {
                ensure!(index < 64, "Amiberry SDL hat index is out of range");
                ensure!(
                    matches!(mask, 1 | 2 | 4 | 8),
                    "Amiberry SDL hat mask is not cardinal"
                );
                Ok(format!("h{index}.{mask}"))
            }
        }
    }
}

fn valid_text(value: &str, what: &str) -> Result<()> {
    ensure!(!value.is_empty(), "Amiberry {what} is empty");
    ensure!(value.len() <= 256, "Amiberry {what} is too long");
    ensure!(
        !value
            .chars()
            .any(|ch| ch == ',' || ch == '\n' || ch == '\r' || ch.is_control()),
        "Amiberry {what} contains a gamecontroller-db separator or control character"
    );
    Ok(())
}

fn valid_guid(guid: &str) -> Result<()> {
    ensure!(
        guid.len() == 32 && guid.bytes().all(|byte| byte.is_ascii_hexdigit()),
        "Amiberry SDL GUID must be 32 hexadecimal characters"
    );
    Ok(())
}

/// Render one SDL gamecontroller database record.  `bindings` uses the
/// target names in [`CONTROLS`], not SDL button numbers.  Directions are
/// either all four digital fields (buttons/hats) or two opposite axis pairs;
/// mixed or incomplete direction descriptions are rejected.
pub(crate) fn gamecontrollerdb_line(
    guid: &str,
    name: &str,
    platform: &str,
    bindings: &BTreeMap<String, Binding>,
) -> Result<String> {
    valid_guid(guid)?;
    valid_text(name, "controller name")?;
    valid_text(platform, "platform")?;
    ensure!(
        bindings.len() == CONTROLS.len(),
        "Amiberry needs all seven classic joystick controls"
    );
    let mut used = BTreeSet::new();
    for (target, _) in CONTROLS {
        ensure!(
            bindings.contains_key(target),
            "Amiberry control {target} is absent"
        );
        let binding = bindings[target];
        ensure!(used.insert(binding), "Amiberry reuses one physical input");
    }

    let directions = ["up", "down", "left", "right"];
    let direction_values: Vec<_> = directions.iter().map(|name| bindings[*name]).collect();
    let axis_pair = |negative: Binding, positive: Binding| match (negative, positive) {
        (
            Binding::Axis {
                index: first,
                positive: false,
            },
            Binding::Axis {
                index: second,
                positive: true,
            },
        ) if first == second => Some(first),
        _ => None,
    };
    let digital = direction_values
        .iter()
        .all(|value| matches!(value, Binding::Button(_) | Binding::Hat { .. }));
    let analog = axis_pair(direction_values[2], direction_values[3]).is_some()
        && axis_pair(direction_values[0], direction_values[1]).is_some()
        && axis_pair(direction_values[0], direction_values[1])
            != axis_pair(direction_values[2], direction_values[3]);
    ensure!(
        digital || analog,
        "Amiberry directions must be one digital set or two opposite SDL axes"
    );

    let mut fields: BTreeMap<&str, String> = BTreeMap::new();
    if digital {
        for (target, output) in CONTROLS {
            fields.insert(output, bindings[target].sdl_spec()?);
        }
    } else {
        fields.insert("leftx", bindings["left"].sdl_spec()?);
        fields.insert("lefty", bindings["up"].sdl_spec()?);
        for (target, output) in [("fire", "a"), ("fire2", "b"), ("fire3", "x")] {
            fields.insert(output, bindings[target].sdl_spec()?);
        }
    }

    let mut output = format!("{},{},platform:{platform}", guid.to_ascii_lowercase(), name);
    for (field, spec) in fields {
        output.push(',');
        output.push_str(field);
        output.push(':');
        output.push_str(&spec);
    }
    output.push('\n');
    Ok(output)
}

/// Render the `.uae` options selecting one SDL joystick for a classic port.
/// Amiberry's source uses zero-based `joyN` IDs and only the first two ports
/// are normal joystick/gamepad ports; ports 2 and 3 are parallel adapters.
pub(crate) fn uae_port_fragment(port: u8, device_index: u8, friendly_name: &str) -> Result<String> {
    ensure!(
        port < 2,
        "Amiberry classic gamepad mode supports ports 0 and 1"
    );
    valid_text(friendly_name, "friendly device name")?;
    Ok(format!(
        "joyport{port}=joy{device_index}\njoyport{port}mode=gamepad\njoyportfriendlyname{port}={friendly_name}\njoyportname{port}=JOY{device_index}\n"
    ))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn digital() -> BTreeMap<String, Binding> {
        [
            ("up", Binding::Hat { index: 0, mask: 1 }),
            ("down", Binding::Hat { index: 0, mask: 4 }),
            ("left", Binding::Hat { index: 0, mask: 8 }),
            ("right", Binding::Hat { index: 0, mask: 2 }),
            ("fire", Binding::Button(0)),
            ("fire2", Binding::Button(1)),
            ("fire3", Binding::Button(2)),
        ]
        .into_iter()
        .map(|(name, binding)| (name.to_owned(), binding))
        .collect()
    }

    #[test]
    fn gamecontroller_db_uses_amiberry_logical_fields() {
        let text = gamecontrollerdb_line(
            "0123456789abcdef0123456789abcdef",
            "Pad One",
            "Linux",
            &digital(),
        )
        .unwrap();
        assert!(text.contains("dpup:h0.1"));
        assert!(text.contains("a:b0"));
        assert!(text.ends_with('\n'));
    }

    #[test]
    fn uae_fragment_selects_zero_based_joy_and_gamepad_mode() {
        let text = uae_port_fragment(1, 3, "Pad One").unwrap();
        assert_eq!(
            text,
            "joyport1=joy3\njoyport1mode=gamepad\njoyportfriendlyname1=Pad One\njoyportname1=JOY3\n"
        );
        assert!(uae_port_fragment(2, 0, "Pad").is_err());
    }

    #[test]
    fn incomplete_or_mixed_directions_fail_closed() {
        let mut map = digital();
        map.remove("right");
        assert!(
            gamecontrollerdb_line("0123456789abcdef0123456789abcdef", "Pad", "Linux", &map)
                .is_err()
        );
        let mut mixed = digital();
        mixed.insert(
            "up".into(),
            Binding::Axis {
                index: 1,
                positive: false,
            },
        );
        assert!(
            gamecontrollerdb_line("0123456789abcdef0123456789abcdef", "Pad", "Linux", &mixed)
                .is_err()
        );
    }
}

/// SDL3 gamepad counts from `SDL_gamepad.h` at the pinned source's SDL3.
pub(crate) const GAMEPAD_BUTTON_COUNT: usize = 27;
pub(crate) const GAMEPAD_AXIS_COUNT: usize = 6;

/// Gamepad buttons carrying Amiga fire in `gamepad` port mode.
pub(crate) const FIRE_BUTTON: u8 = 0;
pub(crate) const FIRE2_BUTTON: u8 = 1;
pub(crate) const FIRE3_BUTTON: u8 = 2;

/// Render a `<name>.controller` mapping file: `button`/`axis` arrays map
/// SDL gamepad indices to raw SDL indices. Missing keys keep the seeded
/// identity defaults, so only the fire overrides are written.
pub(crate) fn controller_file(fire: [u32; 3]) -> Result<String> {
    for raw in fire {
        ensure!(
            raw <= 255,
            "Amiberry fire button is outside the raw SDL range"
        );
    }
    let mut buttons: Vec<u32> = (0..GAMEPAD_BUTTON_COUNT as u32).collect();
    buttons[usize::from(FIRE_BUTTON)] = fire[0];
    buttons[usize::from(FIRE2_BUTTON)] = fire[1];
    buttons[usize::from(FIRE3_BUTTON)] = fire[2];
    let axes: Vec<u32> = (0..GAMEPAD_AXIS_COUNT as u32).collect();
    Ok(format!(
        "button = {}\naxis = {}\n",
        buttons
            .iter()
            .map(u32::to_string)
            .collect::<Vec<_>>()
            .join(" "),
        axes.iter()
            .map(u32::to_string)
            .collect::<Vec<_>>()
            .join(" ")
    ))
}

/// Layout target ids covered by the native profile.
pub(crate) const ROUTES: [(&str, &str); 7] = [
    ("up", "Up"),
    ("down", "Down"),
    ("left", "Left"),
    ("right", "Right"),
    ("fire", "Fire (south)"),
    ("fire2", "Second fire (east)"),
    ("fire3", "Third fire (west)"),
];

pub(crate) mod settings {
    use super::*;
    use crate::controller_catalog::{Calibration, catalog};
    use anyhow::Context;
    use serde::{Deserialize, Serialize};
    use std::{collections::HashMap, path::PathBuf};

    #[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
    #[serde(deny_unknown_fields)]
    pub(crate) struct Player {
        pub player: u8,
        pub controller_id: String,
    }

    #[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
    #[serde(deny_unknown_fields)]
    pub(crate) struct SavedSetup {
        pub emulator_id: String,
        pub content: PathBuf,
        pub probe_program: PathBuf,
        pub sdl_library: PathBuf,
        pub executable_sha256: String,
        pub players: Vec<Player>,
    }

    impl SavedSetup {
        pub(crate) fn validate(&self) -> Result<()> {
            ensure!(
                !self.emulator_id.trim().is_empty(),
                "Amiberry setup needs an emulator identity"
            );
            for path in [&self.content, &self.probe_program, &self.sdl_library] {
                ensure!(
                    path.is_absolute()
                        && !path
                            .components()
                            .any(|part| matches!(part, std::path::Component::ParentDir)),
                    "Amiberry setup paths must be absolute without parent traversal"
                );
            }
            ensure!(
                self.executable_sha256.len() == 64
                    && self
                        .executable_sha256
                        .bytes()
                        .all(|byte| byte.is_ascii_hexdigit()),
                "Amiberry setup needs a trusted executable SHA-256"
            );
            let limit = catalog()
                .emulator_profiles
                .iter()
                .find(|profile| profile.id == PROFILE_ID)
                .and_then(|profile| profile.native_launch.as_ref())
                .context("Missing Amiberry native profile")?
                .max_players;
            ensure!(
                !self.players.is_empty() && self.players.len() <= limit,
                "Amiberry setup needs one or two players"
            );
            let mut controllers = BTreeSet::new();
            for (index, player) in self.players.iter().enumerate() {
                ensure!(
                    usize::from(player.player) == index + 1
                        && !player.controller_id.trim().is_empty()
                        && controllers.insert(&player.controller_id),
                    "Amiberry players must be distinct, contiguous, and start at player one"
                );
            }
            Ok(())
        }

        pub(crate) fn review(
            &self,
            calibrations: &HashMap<String, Calibration>,
        ) -> Result<serde_json::Value> {
            self.validate()?;
            let profile = catalog()
                .emulator_profiles
                .iter()
                .find(|profile| profile.id == PROFILE_ID)
                .context("Missing Amiberry native profile")?;
            let mut players = Vec::new();
            for player in &self.players {
                let calibration = calibrations
                    .get(&player.controller_id)
                    .context("Amiberry controller has no saved calibration")?;
                ensure!(
                    ["linux", "macos", "windows"].contains(&calibration.os.as_str()),
                    "Amiberry mapping requires Linux physical calibration"
                );
                let mapping = calibration.plan_profile(profile)?;
                ensure!(
                    mapping.rows.iter().all(|row| row.physical_id.is_some()
                        && row
                            .input
                            .as_ref()
                            .is_some_and(|input| input.native.is_some())),
                    "Amiberry needs native calibration for every Amiga control"
                );
                players.push(serde_json::json!({
                    "player": player.player,
                    "controller_id": player.controller_id,
                    "source_layout": calibration.layout,
                    "target_layout": profile.target_layout,
                    "mapping": mapping,
                }));
            }
            Ok(serde_json::json!({
                "profile_id": PROFILE_ID,
                "players": players,
                "launch_ready": false,
                "launch_integration": "partial",
                "detail": "Native Linux launch writes a private <name>.controller plus joyport fragment, then rechecks the exact SDL3 routes. Only fixed-dpad directions with raw-button fire on ports 0/1 are supported; runtime behavior remains unverified."
            }))
        }
    }

    pub(crate) fn validate_setups(setups: &[SavedSetup]) -> Result<()> {
        ensure!(setups.len() <= 1024, "Too many Amiberry saved setups");
        let mut identities = BTreeSet::new();
        for setup in setups {
            setup.validate()?;
            ensure!(
                identities.insert((&setup.emulator_id, &setup.content)),
                "Duplicate Amiberry emulator/content setup"
            );
        }
        Ok(())
    }
}

#[cfg(target_os = "linux")]
mod session {
    use super::*;
    use crate::{
        controller_bizhawk_guard::InputTopology,
        controller_catalog::Calibration,
        controller_native_process::{cancelled, capture},
        controllers::ControllerDevice,
    };
    use anyhow::Context;
    use lunchbox_controller_probe::{
        Snapshot, duckstation::DigitalInput, file_hash, linux_classic::AxisEndpoints,
    };
    use std::{
        collections::{BTreeMap, HashMap},
        fs,
        path::PathBuf,
        process::Command,
        sync::atomic::AtomicBool,
    };

    fn observe(
        setup: &settings::SavedSetup,
        paths: &[String],
        cancel: &AtomicBool,
    ) -> Result<Snapshot> {
        let mut command = Command::new(&setup.probe_program);
        command
            .arg("--sdl-library")
            .arg(&setup.sdl_library)
            .arg("--hint")
            .arg("SDL_JOYSTICK_LINUX_CLASSIC=1");
        for path in paths {
            command.arg("--bindings-for-path").arg(path);
        }
        let (output, _) = capture(&mut command, cancel)?;
        let snapshot: Snapshot =
            serde_json::from_slice(&output).context("Invalid Amiberry SDL3 capture")?;
        ensure!(
            snapshot.library.canonicalize()? == setup.sdl_library.canonicalize()?
                && snapshot.library_sha256 == file_hash(&setup.sdl_library)?
                && snapshot
                    .effective_hints
                    .get("SDL_JOYSTICK_LINUX_CLASSIC")
                    .and_then(Option::as_deref)
                    == Some("1"),
            "Amiberry helper inspected a different SDL runtime or backend"
        );
        Ok(snapshot)
    }

    fn comparable(snapshot: &Snapshot) -> Result<serde_json::Value> {
        let mut value = serde_json::to_value(snapshot)?;
        let object = value
            .as_object_mut()
            .context("Invalid Amiberry snapshot shape")?;
        object.remove("warnings");
        for device in object
            .get_mut("devices")
            .and_then(serde_json::Value::as_array_mut)
            .into_iter()
            .flatten()
        {
            let object = device
                .as_object_mut()
                .context("Invalid Amiberry device shape")?;
            // Resolved bindings and classic maps are inputs to translation,
            // not routing identity; sampled state is excluded upstream.
            object.remove("resolved");
            object.remove("linux_classic");
        }
        Ok(value)
    }

    fn raw_bindings(
        calibration: &Calibration,
        device: &lunchbox_controller_probe::Device,
    ) -> Result<BTreeMap<String, Binding>> {
        let profile = crate::controller_catalog::catalog()
            .emulator_profiles
            .iter()
            .find(|profile| profile.id == PROFILE_ID)
            .context("Missing Amiberry native profile")?;
        // SDL3 gamepad translation needs the classic joystick numbering the
        // hint selects; evdev-direct numbering is a different contract.
        let classic = device
            .linux_classic
            .as_ref()
            .context("Amiberry classic Linux control map is absent")?;
        classic.validate_counts(
            device
                .resolved
                .as_ref()
                .context("Amiberry SDL resolved bindings are absent")?,
        )?;
        let mut result = BTreeMap::new();
        for row in calibration.plan_profile(profile)?.rows {
            ensure!(
                CONTROLS.iter().any(|(target, _)| *target == row.target_id),
                "Amiberry target {} outside contract",
                row.target_id
            );
            let input = row
                .input
                .as_ref()
                .context("Amiberry Amiga control is not calibrated")?;
            let native = input
                .native
                .as_ref()
                .context("Amiberry requires measured native controls")?;
            let measured = input.axis.as_ref().map(|axis| AxisEndpoints {
                released: axis.released,
                pressed: axis.pressed,
            });
            // Release posture is proven by the saved calibration
            // endpoints, not a live sample: the SDL3 snapshot carries no
            // instantaneous state. Stale calibrations fail at review when
            // their endpoints no longer describe the gesture.
            let translated = classic.digital_input(native.code, measured)?;
            // Raw SDL joystick numbers feed the gamecontrollerdb grammar;
            // the resolved gamepad layer is only required to exist so the
            // pad is a real SDL gamepad at launch.
            let binding = match translated {
                DigitalInput::Button(index) => Binding::Button(
                    u32::try_from(index).context("Amiberry button index is too large")?,
                ),
                DigitalInput::Axis { index, .. } => Binding::Axis {
                    index: u32::try_from(index).context("Amiberry axis index is too large")?,
                    positive: native.direction > 0,
                },
                DigitalInput::Hat { index, direction } => Binding::Hat {
                    index: u32::try_from(index).context("Amiberry hat index is too large")?,
                    mask: direction,
                },
            };
            ensure!(
                result.insert(row.target_id, binding).is_none(),
                "Amiberry target control appears twice"
            );
        }
        ensure!(
            result.len() == CONTROLS.len(),
            "Amiberry native mapping is incomplete"
        );
        Ok(result)
    }

    pub(crate) struct PreparedSession {
        directory: tempfile::TempDir,
        controllers_dir: PathBuf,
        uae_path: PathBuf,
        physical_paths: Vec<String>,
        gamepad_indices: Vec<u16>,
        #[cfg(target_os = "linux")]
        topology: InputTopology,
        initial: serde_json::Value,
        setup: settings::SavedSetup,
        hashes: BTreeMap<PathBuf, String>,
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
            ensure!(
                fs::symlink_metadata(&setup.content)?.file_type().is_file()
                    && setup.content.canonicalize()? == setup.content,
                "Amiberry content must be a direct regular file with canonical ancestry"
            );
            let mut selected = Vec::new();
            for player in &setup.players {
                let found = inventory
                    .iter()
                    .filter(|device| device.stable_id == player.controller_id)
                    .collect::<Vec<_>>();
                ensure!(
                    found.len() == 1 && !found[0].is_virtual,
                    "Amiberry physical controller is missing or ambiguous"
                );
                selected.push(found[0].device_path.clone());
            }
            let topology = InputTopology::capture(&selected)?;
            let initial = comparable(&observe(setup, &[], cancel)?)?;
            let mut physical_paths = Vec::new();
            let mut gamepad_indices = Vec::new();
            let mut uae = String::new();
            let mut db_lines = String::new();
            let controllers_dir_owned = tempfile::Builder::new()
                .prefix("lunchbox-amiberry-controllers-")
                .tempdir()?;
            for (player, selected_path) in setup.players.iter().zip(&selected) {
                let path = topology.resolve_runtime_path(
                    selected_path,
                    observe(setup, &[], cancel)?
                        .devices
                        .iter()
                        .filter_map(|device| device.path.as_deref()),
                )?;
                ensure!(
                    !physical_paths.contains(&path),
                    "Amiberry players share a controller"
                );
                let captured = observe(setup, &[path.clone()], cancel)?;
                ensure!(
                    comparable(&captured)? == initial,
                    "Amiberry SDL inventory moved during preparation"
                );
                #[cfg(target_os = "linux")]
                topology.verify()?;
                let device = captured.device_at_path(&path)?;
                ensure!(
                    device.is_gamepad,
                    "Amiberry needs an SDL-recognized gamepad"
                );
                ensure!(
                    device.resolved.is_some(),
                    "Amiberry SDL resolved bindings are absent"
                );
                let calibration = calibrations
                    .get(&player.controller_id)
                    .context("Amiberry calibration disappeared")?;
                let bindings = raw_bindings(calibration, device)?;
                let name = device
                    .name
                    .as_deref()
                    .context("Amiberry SDL device has no name")?;
                db_lines.push_str(
                    &gamecontrollerdb_line(
                        &device.guid,
                        name,
                        crate::controller_native_platform::sdl_platform_name(),
                        &bindings,
                    )
                    .context("Amiberry gamecontrollerdb line failed")?,
                );
                // All devices are gamepads (proven above), so the SDL
                // gamepad index is the di_joystick position the port
                // fragment consumes as the zero-based joy ID.
                let joy_index = device
                    .gamepad_index
                    .context("Amiberry SDL gamepad index is absent")?;
                gamepad_indices.push(joy_index);
                #[cfg(not(target_os = "linux"))]
                platform::require_unique_sdl3_path(&captured.devices, &path)?;
                uae.push_str(
                    &uae_port_fragment(player.player - 1, joy_index as u8, name)
                        .context("Amiberry port fragment failed")?,
                );
                physical_paths.push(path);
            }
            // Amiberry loads gamecontrollerdb_user.txt from the controllers
            // path on top of its bundled database.
            fs::write(
                controllers_dir_owned
                    .path()
                    .join("gamecontrollerdb_user.txt"),
                &db_lines,
            )?;
            let directory = tempfile::Builder::new()
                .prefix("lunchbox-amiberry-")
                .tempdir()?;
            let uae_path = directory.path().join("lunchbox-amiberry.uae");
            fs::write(&uae_path, &uae)?;
            let mut hashes = BTreeMap::new();
            for path in [
                &setup.content,
                &setup.probe_program,
                &setup.sdl_library,
                &uae_path,
            ] {
                hashes.insert(path.clone(), file_hash(path)?);
            }
            let db_path = controllers_dir_owned
                .path()
                .join("gamecontrollerdb_user.txt");
            hashes.insert(
                db_path,
                file_hash(
                    &controllers_dir_owned
                        .path()
                        .join("gamecontrollerdb_user.txt"),
                )?,
            );
            let prepared = Self {
                directory,
                controllers_dir: controllers_dir_owned.keep(),
                uae_path,
                physical_paths,
                gamepad_indices,
                #[cfg(target_os = "linux")]
                topology,
                initial,
                setup: setup.clone(),
                hashes,
            };
            prepared.verify(cancel)?;
            Ok(prepared)
        }

        pub(crate) fn uae_path(&self) -> &std::path::Path {
            &self.uae_path
        }

        pub(crate) fn controllers_dir(&self) -> &std::path::Path {
            &self.controllers_dir
        }

        pub(crate) fn verify(&self, cancel: &AtomicBool) -> Result<()> {
            cancelled(cancel)?;
            #[cfg(target_os = "linux")]
            self.topology.verify()?;
            for (path, hash) in &self.hashes {
                ensure!(file_hash(path)? == *hash, "Amiberry launch input changed");
            }
            let fresh = comparable(&observe(&self.setup, &[], cancel)?)?;
            ensure!(
                fresh == self.initial,
                "Amiberry SDL inventory moved before launch"
            );
            for (path, index) in self.physical_paths.iter().zip(&self.gamepad_indices) {
                let captured = observe(&self.setup, &[path.clone()], cancel)?;
                ensure!(
                    comparable(&captured)? == self.initial,
                    "Amiberry SDL inventory moved before launch"
                );
                let device = captured.device_at_path(path)?;
                ensure!(
                    device.gamepad_index == Some(*index),
                    "Amiberry SDL gamepad order moved before launch"
                );
                #[cfg(not(target_os = "linux"))]
                platform::require_unique_sdl3_path(&captured.devices, path)?;
                #[cfg(target_os = "linux")]
                self.topology.verify()?;
            }
            #[cfg(target_os = "linux")]
            {
                return self.topology.verify();
            }
            #[cfg(not(target_os = "linux"))]
            {
                return Ok(());
            }
        }

        pub(crate) fn check_health(&self) -> Result<()> {
            #[cfg(target_os = "linux")]
            return self.topology.verify();
            #[cfg(not(target_os = "linux"))]
            return self.verify_health_probe();
        }

        #[cfg(not(target_os = "linux"))]
        fn verify_health_probe(&self) -> Result<()> {
            let snapshot = observe(&self.setup, &[], &AtomicBool::new(false))?;
            ensure!(
                comparable(&snapshot)? == self.initial,
                "Amiberry SDL inventory moved"
            );
            for path in &self.physical_paths {
                platform::require_unique_sdl3_path(&snapshot.devices, path)?;
            }
            Ok(())
        }
    }
}

pub(crate) mod native_command {
    use super::*;
    use crate::controller_native_process::cancelled;
    #[cfg(target_os = "linux")]
    use crate::controller_native_process::native_pid;
    use crate::{
        controller_catalog::Calibration,
        controller_native_platform as platform,
        controllers::ControllerDevice,
        emulator::{EmulatorExecutable, LaunchPlan, RomEmulatorOption},
    };
    use lunchbox_controller_probe::file_hash;
    use std::{
        collections::HashMap,
        path::PathBuf,
        sync::atomic::AtomicBool,
        time::{Duration, Instant},
    };

    pub(crate) struct NativeSession {
        inputs: session::PreparedSession,
        executable: PathBuf,
        setup: settings::SavedSetup,
        pub(crate) plan: LaunchPlan,
    }

    impl NativeSession {
        pub(crate) fn check_health(&self) -> Result<()> {
            self.inputs.check_health()
        }

        pub(crate) fn verify(&self, cancel: &AtomicBool) -> Result<()> {
            cancelled(cancel)?;
            ensure!(
                file_hash(&self.executable)?.eq_ignore_ascii_case(&self.setup.executable_sha256),
                "Amiberry executable differs from the saved trusted runtime"
            );
            self.inputs.verify(cancel)
        }

        pub(crate) fn spawn(
            &mut self,
            plan: &LaunchPlan,
            cancel: &AtomicBool,
        ) -> Result<std::process::Child> {
            ensure!(
                plan == &self.plan,
                "Amiberry launch plan changed after preparation"
            );
            self.verify(cancel)?;
            let mut child = crate::emulator::spawn_launch_plan(plan)?;
            if let Err(error) = self.confirm(&mut child, cancel) {
                let _ = child.kill();
                let _ = child.wait();
                return Err(error);
            }
            Ok(child)
        }

        fn confirm(&self, child: &mut std::process::Child, cancel: &AtomicBool) -> Result<()> {
            let deadline = Instant::now() + Duration::from_secs(20);
            loop {
                cancelled(cancel)?;
                ensure!(
                    child.try_wait()?.is_none(),
                    "Amiberry exited before controller handoff"
                );
                // Linux walks the launch tree (bubblewrap monitors); other
                // hosts check the direct child, which they spawn directly.
                #[cfg(target_os = "linux")]
                let owned = native_pid(child.id(), &self.executable)?
                    .is_some_and(|pid| self.ready(pid).unwrap_or(false));
                #[cfg(not(target_os = "linux"))]
                let owned = platform::child_exe_matches(child.id(), &self.executable)?
                    && self.ready(child.id())?;
                if owned {
                    self.inputs.check_health()?;
                    return Ok(());
                }
                ensure!(
                    Instant::now() < deadline,
                    "Amiberry did not open the selected SDL controllers before timeout"
                );
                std::thread::sleep(Duration::from_millis(25));
            }
        }

        fn ready(&self, pid: u32) -> Result<bool> {
            // Linux proves the child mapped the exact SDL library. Other
            // hosts pin the executable plus a fresh device re-probe; the
            // weaker guarantee is explicit here and in the launch text.
            if cfg!(target_os = "linux") {
                return platform::child_maps_library(pid, &self.setup.sdl_library);
            }
            if !platform::child_exe_matches(pid, &self.executable)? {
                return Ok(false);
            }
            self.inputs.check_health()?;
            Ok(true)
        }
    }

    pub(crate) fn prepare(
        setup: &settings::SavedSetup,
        calibrations: &HashMap<String, Calibration>,
        inventory: &[ControllerDevice],
        option: &RomEmulatorOption,
        original: &LaunchPlan,
        cancel: &AtomicBool,
    ) -> Result<NativeSession> {
        cancelled(cancel)?;
        setup.validate()?;
        let EmulatorExecutable::Native(executable) = &option.executable else {
            anyhow::bail!("Amiberry calibrated launch requires a native build");
        };
        ensure!(
            option.emulator_name.eq_ignore_ascii_case("Amiberry")
                && setup.emulator_id == option.emulator_id
                && original.environment.is_empty()
                && original.retroarch_content.is_none(),
            "Amiberry identity differs or custom environment needs resolution"
        );
        let executable = executable.canonicalize()?;
        ensure!(
            executable == original.program.canonicalize()?,
            "Amiberry launch executable differs from selection"
        );
        ensure!(
            original.arguments.as_slice() == [setup.content.as_os_str()],
            "Amiberry calibrated launch requires exactly the saved content argument"
        );
        ensure!(
            file_hash(&executable)?.eq_ignore_ascii_case(&setup.executable_sha256),
            "Amiberry executable differs from the saved trusted runtime"
        );
        let inputs = session::PreparedSession::prepare(setup, calibrations, inventory, cancel)?;
        let mut plan = original.clone();
        plan.program = executable.clone();
        // The private .uae carries only joyport fragments; controllers_path
        // and use_gui=no ride as -s overrides so no user file is touched.
        // Content keeps its default positional slot.
        plan.arguments = vec![
            std::ffi::OsString::from("-f"),
            inputs.uae_path().as_os_str().to_owned(),
            std::ffi::OsString::from("-s"),
            std::ffi::OsString::from(format!(
                "controllers_path={}",
                inputs.controllers_dir().display()
            )),
            std::ffi::OsString::from("-s"),
            std::ffi::OsString::from("use_gui=no"),
            setup.content.as_os_str().to_owned(),
        ];
        let session = NativeSession {
            inputs,
            executable,
            setup: setup.clone(),
            plan,
        };
        session.verify(cancel)?;
        Ok(session)
    }
}
