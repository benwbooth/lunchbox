//! 86Box source-shaped input-device configuration writer.
//!
//! Pinned source: 86Box/86Box `189d9d003ad9670853cec6edac8db7d6ff63550f`.
//! `src/config.c` loads and saves the `[Input devices]` section. Joystick
//! topology is selected by `joystick_type`; each emulated joystick slot uses
//! `joystick_N_nr`, `joystick_N_axis_M`, `joystick_N_button_M`, and
//! `joystick_N_pov_M` (the POV value is `x, y`).

use anyhow::{Context, Result, ensure};
use serde::{Deserialize, Serialize};
use std::collections::BTreeSet;

pub(crate) const SOURCE_COMMIT: &str = "189d9d003ad9670853cec6edac8db7d6ff63550f";
pub(crate) const PROFILE_ID: &str = "86box:standalone-86box-2axis-2button";
pub(crate) const CONTROLS: [(&str, &str); 6] = [
    ("up", "Y axis negative"),
    ("down", "Y axis positive"),
    ("left", "X axis negative"),
    ("right", "X axis positive"),
    ("a", "Button 1"),
    ("b", "Button 2"),
];
const POV_X: u32 = 0x8000_0000;
const POV_Y: u32 = 0x4000_0000;
const MAX_PLAT_JOYSTICKS: u8 = 8;
const MAX_JOYSTICKS: usize = 4;
const MAX_AXES: usize = 16;
const MAX_BUTTONS: usize = 32;
const MAX_POVS: usize = 4;

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct Joystick {
    /// Emulated joystick slot, zero-based; topology determines its meaning.
    pub slot: u8,
    /// Native platform joystick number, one-based (`0` means absent in 86Box).
    pub host_number: u8,
    pub axis_mapping: Vec<i32>,
    pub button_mapping: Vec<i32>,
    pub pov_mapping: Vec<(i32, i32)>,
}

fn valid_name(name: &str) -> bool {
    !name.is_empty()
        && name.len() <= 64
        && name
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || b"_-".contains(&b))
}

fn valid_axis_mapping(value: i32) -> bool {
    let value = value.cast_unsigned();
    if value < MAX_AXES as u32 {
        return true;
    }
    let kind = value & (POV_X | POV_Y);
    matches!(kind, POV_X | POV_Y) && value & !(POV_X | POV_Y | (MAX_POVS as u32 - 1)) == 0
}

/// Patch the source-defined `[Input devices]` keys in a copied 86box.cfg.
/// Existing text and unrelated settings are retained. The caller supplies
/// machine topology and measured host joystick numbering explicitly.
pub(crate) fn patch_config(
    baseline: &[u8],
    joystick_type: &str,
    joysticks: &[Joystick],
) -> Result<String> {
    ensure!(
        baseline.len() <= 4 * 1024 * 1024,
        "86Box config is too large"
    );
    ensure!(valid_name(joystick_type), "86Box joystick type is invalid");
    ensure!(
        !joysticks.is_empty() && joysticks.len() <= MAX_JOYSTICKS,
        "86Box needs one through four joystick slots"
    );
    let text = std::str::from_utf8(baseline).context("86Box config is not UTF-8")?;
    ensure!(!text.contains('\0'), "86Box config contains a NUL byte");
    let mut seen = [false; MAX_JOYSTICKS];
    let newline = if text.contains("\r\n") { "\r\n" } else { "\n" };
    let mut fields = vec![("joystick_type".to_owned(), joystick_type.to_owned())];
    for joystick in joysticks {
        ensure!(
            usize::from(joystick.slot) < seen.len(),
            "86Box joystick slot is out of range"
        );
        ensure!(
            !seen[usize::from(joystick.slot)],
            "86Box joystick slot is duplicated"
        );
        seen[usize::from(joystick.slot)] = true;
        ensure!(
            (1..=MAX_PLAT_JOYSTICKS).contains(&joystick.host_number),
            "86Box host joystick number must be one through eight"
        );
        ensure!(
            joystick.axis_mapping.len() <= MAX_AXES
                && joystick.button_mapping.len() <= MAX_BUTTONS
                && joystick.pov_mapping.len() <= MAX_POVS,
            "86Box joystick topology exceeds source limits"
        );
        ensure!(
            joystick
                .axis_mapping
                .iter()
                .chain(joystick.pov_mapping.iter().flat_map(|(x, y)| [x, y]),)
                .all(|value| valid_axis_mapping(*value)),
            "86Box axis mapping is outside the raw-axis/POV grammar"
        );
        ensure!(
            joystick
                .button_mapping
                .iter()
                .all(|value| (0..MAX_BUTTONS as i32).contains(value)),
            "86Box button mapping is outside the raw-button range"
        );
        let n = joystick.slot;
        fields.push((format!("joystick_{n}_nr"), joystick.host_number.to_string()));
        for (index, value) in joystick.axis_mapping.iter().enumerate() {
            fields.push((format!("joystick_{n}_axis_{index}"), value.to_string()));
        }
        for (index, value) in joystick.button_mapping.iter().enumerate() {
            fields.push((format!("joystick_{n}_button_{index}"), value.to_string()));
        }
        for (index, (x, y)) in joystick.pov_mapping.iter().enumerate() {
            fields.push((format!("joystick_{n}_pov_{index}"), format!("{x}, {y}")));
        }
    }

    let append_fields = |out: &mut String| {
        for (key, value) in &fields {
            out.push_str(&format!("{key}={value}{newline}"));
        }
    };
    let mut out = String::new();
    let mut active = false;
    let mut found = false;
    for raw in text.split_inclusive('\n') {
        let line = raw
            .strip_suffix('\n')
            .unwrap_or(raw)
            .strip_suffix('\r')
            .unwrap_or(raw);
        if let Some(header) = line
            .trim()
            .strip_prefix('[')
            .and_then(|value| value.strip_suffix(']'))
        {
            if active {
                append_fields(&mut out);
                active = false;
            }
            if header.trim().eq_ignore_ascii_case("Input devices") {
                ensure!(!found, "86Box config has duplicate Input devices sections");
                found = true;
                active = true;
            }
            out.push_str(raw);
            continue;
        }
        if active {
            let owned = line.split_once('=').is_some_and(|(key, _)| {
                let key = key.trim();
                key.eq_ignore_ascii_case("joystick_type")
                    || key
                        .get(..9)
                        .is_some_and(|prefix| prefix.eq_ignore_ascii_case("joystick_"))
            });
            if owned {
                continue;
            }
        }
        out.push_str(raw);
    }
    if active {
        if !out.is_empty() && !out.ends_with(['\n', '\r']) {
            out.push_str(newline);
        }
        append_fields(&mut out);
    } else if !found {
        if !out.is_empty() && !out.ends_with(['\n', '\r']) {
            out.push_str(newline);
        }
        out.push_str(&format!("[Input devices]{newline}"));
        append_fields(&mut out);
    }
    Ok(out)
}

pub(crate) fn source_boundary() -> &'static str {
    "86Box joystick_N_nr is a measured one-based host SDL/raw-input slot and joystick_N_* indices are machine-topology dependent. Preserve the copied machine config, ROM directory, disk images, and guest-save roots; a writer does not establish that the selected guest program reads the device."
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub(crate) enum SdlApi {
    Sdl2,
}

pub(crate) mod settings {
    use super::*;
    use crate::controller_catalog::{Calibration, catalog};
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
        /// The selected game entry is the machine's exact `86box.cfg`.
        pub content: PathBuf,
        pub sdl_api: SdlApi,
        pub probe_program: PathBuf,
        pub sdl_library: PathBuf,
        pub bubblewrap_program: PathBuf,
        pub executable_sha256: String,
        pub players: Vec<Player>,
    }

    impl SavedSetup {
        pub(crate) fn validate(&self) -> Result<()> {
            ensure!(
                !self.emulator_id.trim().is_empty(),
                "86Box setup needs an emulator identity"
            );
            for path in [
                &self.content,
                &self.probe_program,
                &self.sdl_library,
                &self.bubblewrap_program,
            ] {
                ensure!(
                    path.is_absolute()
                        && !path
                            .components()
                            .any(|part| matches!(part, std::path::Component::ParentDir)),
                    "86Box setup paths must be absolute without parent traversal"
                );
            }
            ensure!(
                self.content.file_name().and_then(|name| name.to_str()) == Some("86box.cfg"),
                "86Box content must be an exact machine 86box.cfg"
            );
            ensure!(
                self.sdl_api == SdlApi::Sdl2,
                "86Box native setup requires the SDL2 backend"
            );
            ensure!(
                self.executable_sha256.len() == 64
                    && self
                        .executable_sha256
                        .bytes()
                        .all(|byte| byte.is_ascii_hexdigit()),
                "86Box setup needs a trusted executable SHA-256"
            );
            let limit = catalog()
                .emulator_profiles
                .iter()
                .find(|profile| profile.id == PROFILE_ID)
                .and_then(|profile| profile.native_launch.as_ref())
                .context("Missing 86Box native profile")?
                .max_players;
            ensure!(
                !self.players.is_empty() && self.players.len() <= limit,
                "86Box setup needs one or two players"
            );
            let mut controllers = BTreeSet::new();
            for (index, player) in self.players.iter().enumerate() {
                ensure!(
                    usize::from(player.player) == index + 1
                        && !player.controller_id.trim().is_empty()
                        && controllers.insert(&player.controller_id),
                    "86Box players must be distinct, contiguous, and start at player one"
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
                .context("Missing 86Box native profile")?;
            let mut players = Vec::new();
            for player in &self.players {
                let calibration = calibrations
                    .get(&player.controller_id)
                    .context("86Box controller has no saved calibration")?;
                ensure!(
                    calibration.os == "linux",
                    "86Box mapping requires Linux physical calibration"
                );
                let mapping = calibration.plan_profile(profile)?;
                ensure!(
                    mapping.rows.iter().all(|row| row.physical_id.is_some()
                        && row
                            .input
                            .as_ref()
                            .is_some_and(|input| input.native.is_some())),
                    "86Box needs native calibration for every gameport control"
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
                "profile_id": self.players.first().map(|_| PROFILE_ID),
                "players": players,
                "launch_ready": false,
                "launch_integration": "partial",
                "detail": "Native Linux launch overlays the selected machine 86box.cfg at its original path, selects the exact 2-axis/2-button gameport topology, and rechecks SDL2 raw controls and device order. Guest disk saves and ROM paths remain native. Runtime behavior is unverified."
            }))
        }
    }

    pub(crate) fn validate_setups(setups: &[SavedSetup]) -> Result<()> {
        ensure!(setups.len() <= 1024, "Too many 86Box saved setups");
        let mut identities = BTreeSet::new();
        for setup in setups {
            setup.validate()?;
            ensure!(
                identities.insert((&setup.emulator_id, &setup.content)),
                "Duplicate 86Box emulator/machine setup"
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
    use lunchbox_controller_probe::{
        duckstation::DigitalInput, file_hash, linux_classic::AxisEndpoints, sdl2::Snapshot,
        sdl2_physical::PhysicalMap,
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
        path: Option<&str>,
        cancel: &AtomicBool,
    ) -> Result<Snapshot> {
        ensure!(
            setup.sdl_api == SdlApi::Sdl2,
            "86Box native capture requires SDL2"
        );
        let mut command = Command::new(&setup.probe_program);
        command
            .arg("--sdl2-inventory")
            .arg("--sdl-library")
            .arg(&setup.sdl_library);
        if let Some(path) = path {
            command.arg("--sdl2-controls-for-path").arg(path);
        }
        let (output, _) = capture(&mut command, cancel)?;
        let snapshot: Snapshot =
            serde_json::from_slice(&output).context("Invalid 86Box SDL2 capture")?;
        ensure!(
            snapshot.version[0] == 2
                && snapshot.library.canonicalize()? == setup.sdl_library.canonicalize()?
                && snapshot.library_sha256 == file_hash(&setup.sdl_library)?,
            "86Box helper inspected a different SDL2 runtime"
        );
        Ok(snapshot)
    }

    fn routing(mut snapshot: Snapshot) -> Snapshot {
        for device in &mut snapshot.devices {
            device.controls = None;
            device.linux_classic = None;
            device.linux_evdev = None;
            device.sampled_state = None;
        }
        snapshot
    }

    fn inputs(
        calibration: &Calibration,
        snapshot: &Snapshot,
        runtime_path: &str,
    ) -> Result<BTreeMap<String, DigitalInput>> {
        let physical = PhysicalMap::from_device(snapshot.device_at_path(runtime_path)?)?;
        let profile = crate::controller_catalog::catalog()
            .emulator_profiles
            .iter()
            .find(|profile| profile.id == PROFILE_ID)
            .context("Missing 86Box native profile")?;
        let mut result = BTreeMap::new();
        for row in calibration.plan_profile(profile)?.rows {
            let input = row
                .input
                .as_ref()
                .context("86Box gameport control is not calibrated")?;
            let native = input
                .native
                .as_ref()
                .context("86Box requires measured native controls")?;
            let endpoints = input.axis.as_ref().map(|axis| AxisEndpoints {
                released: axis.released,
                pressed: axis.pressed,
            });
            ensure!(
                result
                    .insert(
                        row.target_id,
                        physical.digital_input(native.code, endpoints)?,
                    )
                    .is_none(),
                "86Box target control appears twice"
            );
        }
        Ok(result)
    }

    fn axis_mapping(
        negative: &DigitalInput,
        positive: &DigitalInput,
        negative_hat: u8,
        positive_hat: u8,
        flag: u32,
        name: &str,
    ) -> Result<i32> {
        if let (
            DigitalInput::Axis {
                index: first,
                released: first_rest,
                pressed: first_press,
            },
            DigitalInput::Axis {
                index: second,
                released: second_rest,
                pressed: second_press,
            },
        ) = (negative, positive)
            && first == second
            && i32::from(*first_rest).abs() < 10_000
            && i32::from(*second_rest).abs() < 10_000
            && *first_press < -10_000
            && *second_press > 10_000
        {
            return i32::try_from(*first).context("86Box SDL axis index is out of range");
        }
        if let (
            DigitalInput::Hat {
                index: first,
                direction: first_direction,
            },
            DigitalInput::Hat {
                index: second,
                direction: second_direction,
            },
        ) = (negative, positive)
            && first == second
            && *first <= 3
            && *first_direction == negative_hat
            && *second_direction == positive_hat
        {
            return Ok((flag | *first).cast_signed());
        }
        anyhow::bail!(
            "86Box {name} directions must be opposite halves of one raw SDL axis or one cardinal hat"
        )
    }

    fn joystick(
        slot: u8,
        host_number: u8,
        mapped: &BTreeMap<String, DigitalInput>,
    ) -> Result<Joystick> {
        let get = |name: &str| {
            mapped
                .get(name)
                .with_context(|| format!("86Box control {name} is absent"))
        };
        let button = |name: &str| match get(name)? {
            DigitalInput::Button(index) if *index < 32 => Ok(*index as i32),
            _ => anyhow::bail!("86Box {name} must resolve to a raw SDL button"),
        };
        let first = button("a")?;
        let second = button("b")?;
        ensure!(first != second, "86Box gameport buttons must be distinct");
        Ok(Joystick {
            slot,
            host_number,
            axis_mapping: vec![
                axis_mapping(get("left")?, get("right")?, 8, 2, POV_X, "horizontal")?,
                axis_mapping(get("up")?, get("down")?, 1, 4, POV_Y, "vertical")?,
            ],
            button_mapping: vec![first, second],
            pov_mapping: vec![],
        })
    }

    pub(crate) struct PreparedSession {
        directory: tempfile::TempDir,
        pub(crate) private_config: PathBuf,
        runtime_paths: Vec<String>,
        captures: Vec<Snapshot>,
        topology: InputTopology,
        initial: Snapshot,
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
                "86Box machine config must be a direct regular file with canonical ancestry"
            );
            let mut selected = Vec::new();
            for player in &setup.players {
                let matches = inventory
                    .iter()
                    .filter(|device| device.stable_id == player.controller_id)
                    .collect::<Vec<_>>();
                ensure!(
                    matches.len() == 1 && !matches[0].is_virtual,
                    "86Box physical controller is missing or ambiguous"
                );
                selected.push(matches[0].device_path.clone());
            }
            let topology = InputTopology::capture(&selected)?;
            let initial = routing(observe(setup, None, cancel)?);
            ensure!(
                initial.devices.len() <= usize::from(MAX_PLAT_JOYSTICKS),
                "86Box SDL2 runtime supports at most eight enumerated joysticks"
            );
            let mut runtime_paths = Vec::new();
            let mut captures = Vec::new();
            let mut joysticks = Vec::new();
            for (slot, (player, selected_path)) in setup.players.iter().zip(&selected).enumerate() {
                let runtime_path = topology.resolve_runtime_path(
                    selected_path,
                    initial
                        .devices
                        .iter()
                        .filter_map(|device| device.path.as_deref()),
                )?;
                ensure!(
                    !runtime_paths.contains(&runtime_path),
                    "86Box players resolved to the same controller"
                );
                let captured = observe(setup, Some(&runtime_path), cancel)?;
                initial.ensure_same_routing(&routing(captured.clone()))?;
                topology.verify()?;
                let device = captured.device_at_path(&runtime_path)?;
                let host_number = device
                    .device_index
                    .checked_add(1)
                    .and_then(|number| u8::try_from(number).ok())
                    .context("86Box SDL joystick number is out of range")?;
                ensure!(
                    host_number <= MAX_PLAT_JOYSTICKS,
                    "86Box SDL joystick number exceeds its eight-device array"
                );
                joysticks.push(joystick(
                    u8::try_from(slot)?,
                    host_number,
                    &inputs(
                        calibrations
                            .get(&player.controller_id)
                            .context("86Box calibration disappeared")?,
                        &captured,
                        &runtime_path,
                    )?,
                )?);
                runtime_paths.push(runtime_path);
                captures.push(captured);
            }
            let baseline = fs::read(&setup.content).context("Reading 86Box machine config")?;
            let directory = tempfile::Builder::new()
                .prefix("lunchbox-86box-native-")
                .tempdir()?;
            let private_config = directory.path().join("86box.cfg");
            fs::write(
                &private_config,
                patch_config(&baseline, "2axis_2button", &joysticks)?,
            )?;
            let mut hashes = BTreeMap::new();
            for path in [
                &setup.content,
                &setup.probe_program,
                &setup.sdl_library,
                &setup.bubblewrap_program,
                &private_config,
            ] {
                hashes.insert(path.clone(), file_hash(path)?);
            }
            let prepared = Self {
                directory,
                private_config,
                runtime_paths,
                captures,
                topology,
                initial,
                setup: setup.clone(),
                hashes,
            };
            prepared.verify(cancel)?;
            Ok(prepared)
        }

        pub(crate) fn verify(&self, cancel: &AtomicBool) -> Result<()> {
            cancelled(cancel)?;
            ensure!(
                self.directory.path().is_dir() && self.private_config.is_file(),
                "86Box private configuration disappeared"
            );
            self.topology.verify()?;
            for (path, expected) in &self.hashes {
                ensure!(file_hash(path)? == *expected, "86Box launch input changed");
            }
            let fresh = routing(observe(&self.setup, None, cancel)?);
            self.initial.ensure_same_routing(&fresh)?;
            ensure!(
                fresh.devices.len() <= usize::from(MAX_PLAT_JOYSTICKS),
                "86Box SDL2 runtime supports at most eight enumerated joysticks"
            );
            for (path, expected) in self.runtime_paths.iter().zip(&self.captures) {
                let captured = observe(&self.setup, Some(path), cancel)?;
                expected.ensure_same_routing(&captured)?;
                self.initial.ensure_same_routing(&routing(captured))?;
                self.topology.verify()?;
            }
            self.topology.verify()
        }

        pub(crate) fn check_health(&self) -> Result<()> {
            self.topology.verify()
        }
    }

    #[cfg(test)]
    mod tests {
        use super::*;

        fn axis(index: u32, released: i16, pressed: i16) -> DigitalInput {
            DigitalInput::Axis {
                index,
                released,
                pressed,
            }
        }

        #[test]
        fn raw_axes_and_cardinal_hats_encode_source_grammar() {
            assert_eq!(
                axis_mapping(&axis(3, 0, -32768), &axis(3, 0, 32767), 8, 2, POV_X, "x").unwrap(),
                3
            );
            assert_eq!(
                axis_mapping(
                    &DigitalInput::Hat {
                        index: 2,
                        direction: 1,
                    },
                    &DigitalInput::Hat {
                        index: 2,
                        direction: 4,
                    },
                    1,
                    4,
                    POV_Y,
                    "y",
                )
                .unwrap(),
                (POV_Y | 2).cast_signed()
            );
        }

        #[test]
        fn direction_pairs_and_buttons_must_be_unambiguous() {
            assert!(
                axis_mapping(
                    &axis(0, i16::MIN, -32768),
                    &axis(0, 0, 32767),
                    8,
                    2,
                    POV_X,
                    "x",
                )
                .is_err()
            );
            assert!(
                axis_mapping(
                    &DigitalInput::Hat {
                        index: 0,
                        direction: 8,
                    },
                    &DigitalInput::Hat {
                        index: 1,
                        direction: 2,
                    },
                    8,
                    2,
                    POV_X,
                    "x",
                )
                .is_err()
            );
            let mapped = BTreeMap::from([
                ("left".into(), axis(0, 0, -32768)),
                ("right".into(), axis(0, 0, 32767)),
                ("up".into(), axis(1, 0, -32768)),
                ("down".into(), axis(1, 0, 32767)),
                ("a".into(), DigitalInput::Button(4)),
                ("b".into(), DigitalInput::Button(4)),
            ]);
            assert!(joystick(0, 1, &mapped).is_err());
        }
    }
}

#[cfg(target_os = "linux")]
pub(crate) mod native_command {
    use super::*;
    use crate::{
        controller_catalog::Calibration,
        controller_native_process::cancelled,
        controllers::ControllerDevice,
        emulator::{EmulatorExecutable, LaunchPlan, RomEmulatorOption},
    };
    use lunchbox_controller_probe::file_hash;
    use std::{collections::HashMap, path::PathBuf, sync::atomic::AtomicBool};

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
                "86Box executable differs from the saved trusted runtime"
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
                "86Box launch plan changed after preparation"
            );
            self.verify(cancel)?;
            crate::emulator::spawn_launch_plan(plan)
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
            anyhow::bail!("86Box calibrated launch requires native Linux");
        };
        ensure!(
            option.emulator_name.eq_ignore_ascii_case("86Box")
                && setup.emulator_id == option.emulator_id
                && original.environment.is_empty(),
            "86Box identity differs or custom environment needs resolution"
        );
        let executable = executable.canonicalize()?;
        ensure!(
            executable == original.program.canonicalize()?,
            "86Box launch executable differs from selection"
        );
        ensure!(
            original.arguments.as_slice() == [setup.content.as_os_str()],
            "86Box calibrated launch requires exactly the saved machine config argument"
        );
        ensure!(
            file_hash(&executable)?.eq_ignore_ascii_case(&setup.executable_sha256),
            "86Box executable differs from the saved trusted runtime"
        );
        let inputs = session::PreparedSession::prepare(setup, calibrations, inventory, cancel)?;
        let cwd = original.current_directory.canonicalize()?;
        let arguments = vec![
            "--die-with-parent".into(),
            "--bind".into(),
            "/".into(),
            "/".into(),
            "--bind".into(),
            inputs.private_config.as_os_str().to_owned(),
            setup.content.as_os_str().to_owned(),
            "--chdir".into(),
            cwd.into_os_string(),
            "--".into(),
            executable.as_os_str().to_owned(),
            "-C".into(),
            setup.content.as_os_str().to_owned(),
        ];
        let mut plan = original.clone();
        plan.program = setup.bubblewrap_program.clone();
        plan.arguments = arguments;
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

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn emits_input_devices_and_pov_grammar() {
        let text = patch_config(
            b"[Machine]\nromset=keep\n[Input devices]\nscancode_1=2\njoystick_type=old\njoystick_4_nr=9\n[Other]\nkeep=yes\n",
            "2axis_2button",
            &[Joystick {
                slot: 0,
                host_number: 2,
                axis_mapping: vec![1, 0],
                button_mapping: vec![3, 4],
                pov_mapping: vec![(0, 1)],
            }],
        )
        .unwrap();
        assert!(text.contains("[Machine]\nromset=keep"));
        assert!(text.contains("[Input devices]\nscancode_1=2\njoystick_type=2axis_2button\njoystick_0_nr=2\njoystick_0_axis_0=1"));
        assert!(text.contains("joystick_0_pov_0=0, 1\n[Other]\nkeep=yes"));
        assert!(!text.contains("joystick_type=old"));
        assert!(!text.contains("joystick_4_nr=9"));
    }
    #[test]
    fn rejects_zero_host_or_duplicate_slots() {
        let bad = Joystick {
            slot: 0,
            host_number: 0,
            axis_mapping: vec![],
            button_mapping: vec![],
            pov_mapping: vec![],
        };
        assert!(patch_config(b"", "2axis_2button", &[bad]).is_err());
        let one = Joystick {
            slot: 0,
            host_number: 1,
            axis_mapping: vec![],
            button_mapping: vec![],
            pov_mapping: vec![],
        };
        assert!(patch_config(b"", "2axis_2button", &[one.clone(), one]).is_err());
        assert!(
            patch_config(
                b"[Input devices]\n[Input devices]\n",
                "2axis_2button",
                &[Joystick {
                    slot: 0,
                    host_number: 1,
                    axis_mapping: vec![],
                    button_mapping: vec![],
                    pov_mapping: vec![],
                }]
            )
            .is_err()
        );
    }

    #[test]
    fn rejects_source_array_overflow_mappings() {
        let joystick = |axis_mapping, button_mapping, pov_mapping| Joystick {
            slot: 0,
            host_number: 1,
            axis_mapping,
            button_mapping,
            pov_mapping,
        };
        assert!(patch_config(b"", "2axis_2button", &[joystick(vec![16], vec![], vec![])]).is_err());
        assert!(
            patch_config(
                b"",
                "2axis_2button",
                &[joystick(vec![POV_X.cast_signed() | 4], vec![], vec![])]
            )
            .is_err()
        );
        assert!(patch_config(b"", "2axis_2button", &[joystick(vec![], vec![32], vec![])]).is_err());
        assert!(
            patch_config(
                b"",
                "2axis_2button",
                &[Joystick {
                    slot: 0,
                    host_number: 9,
                    axis_mapping: vec![],
                    button_mapping: vec![],
                    pov_mapping: vec![],
                }]
            )
            .is_err()
        );
    }
}
