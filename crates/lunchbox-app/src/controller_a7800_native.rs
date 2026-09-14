//! A7800 standalone native MAME-controller XML writer.
//!
//! Pinned source: 7800-devtools/a7800 commit
//! 7a2afdc1ea08fc331b16b750d8c1f02d4ef62fc8. The target identifies itself as
//! `a7800` and explicitly retains MAME configuration syntax. Its controller
//! devices define the generic MAME input items `IPT_JOYSTICK_*` and
//! `IPT_BUTTON1/2`; this module emits only those source-backed input names.
//! The caller must resolve each `JOYCODE_*`/`KEYCODE_*` token from the same
//! MAME input provider inventory.

use anyhow::{Context, Result, ensure};
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};

pub(crate) const SOURCE_COMMIT: &str = "7a2afdc1ea08fc331b16b750d8c1f02d4ef62fc8";
pub(crate) const PROFILE_ID: &str = "a7800:standalone-atari7800";
pub(crate) const CONTROLS: [(&str, &str); 6] = [
    ("up", "Up"),
    ("down", "Down"),
    ("left", "Left"),
    ("right", "Right"),
    ("a", "Left fire button"),
    ("b", "Right fire button"),
];
const MAX_JOYSTICKS: usize = 8;

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub(crate) enum Machine {
    Ntsc,
    Pal,
}

impl Machine {
    fn basename(self) -> &'static str {
        match self {
            Self::Ntsc => "a7800",
            Self::Pal => "a7800p",
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum ControllerKind {
    /// `vcs_joystick` exposes four directions and IPT_BUTTON1.
    VcsJoystick,
    /// `proline_joystick` exposes four directions and IPT_BUTTON1/2.
    ProlineJoystick,
}

impl ControllerKind {
    fn button_count(self) -> u8 {
        match self {
            Self::VcsJoystick => 1,
            Self::ProlineJoystick => 2,
        }
    }
}

pub(crate) struct Port<'a> {
    /// Emulated controller port number, one-based.
    pub player: u8,
    pub kind: ControllerKind,
    /// Provider-resolved MAME tokens, keyed by `up`, `down`, `left`, `right`,
    /// and `button1`/`button2` as required by `kind`.
    pub tokens: &'a BTreeMap<String, String>,
}

fn token_is_resolved(token: &str) -> bool {
    !token.is_empty()
        && token.len() <= 128
        && token
            .bytes()
            .all(|byte| byte.is_ascii_uppercase() || byte.is_ascii_digit() || byte == b'_')
        && (token.starts_with("JOYCODE_") || token.starts_with("KEYCODE_"))
}

/// Produce a MAME v10 `default.cfg` input section for explicitly resolved
/// A7800 joystick ports. Console switches, paddles, lightguns, keypads,
/// trackballs and mice are intentionally outside this writer until each is
/// separately measured and declared by the caller.
pub(crate) fn controller_xml(ports: &[Port<'_>]) -> Result<String> {
    ensure!(
        !ports.is_empty() && ports.len() <= 2,
        "A7800 has two controller ports"
    );
    let mut players = BTreeSet::new();
    let mut xml = String::from(
        "<?xml version=\"1.0\"?>\n<mameconfig version=\"10\">\n  <system name=\"default\">\n    <input>\n",
    );
    let mut owners = BTreeSet::new();
    for port in ports {
        ensure!(
            (1..=2).contains(&port.player) && players.insert(port.player),
            "A7800 port is duplicated or invalid"
        );
        let mut controls = vec![
            ("up", format!("P{}_JOYSTICK_UP", port.player)),
            ("down", format!("P{}_JOYSTICK_DOWN", port.player)),
            ("left", format!("P{}_JOYSTICK_LEFT", port.player)),
            ("right", format!("P{}_JOYSTICK_RIGHT", port.player)),
        ];
        for button in 1..=port.kind.button_count() {
            controls.push((
                if button == 1 { "button1" } else { "button2" },
                format!("P{}_BUTTON{}", port.player, button),
            ));
        }
        ensure!(
            port.tokens.len() == controls.len()
                && controls
                    .iter()
                    .all(|(name, _)| port.tokens.contains_key(*name)),
            "A7800 port is missing a direction or fire binding"
        );
        for (control, native_type) in controls {
            let token = &port.tokens[control];
            ensure!(
                token_is_resolved(token),
                "A7800 binding is not a resolved MAME token"
            );
            ensure!(owners.insert(token), "A7800 controls share an input");
            xml.push_str(&format!("      <port type=\"{native_type}\"><newseq type=\"standard\">{token}</newseq></port>\n"));
        }
    }
    xml.push_str("    </input>\n  </system>\n</mameconfig>\n");
    Ok(xml)
}

fn mapped_controller_xml(
    ports: &[Port<'_>],
    devices: &[(&str, u8)],
    observed_ids: &[String],
) -> Result<String> {
    ensure!(
        !devices.is_empty() && devices.len() <= MAX_JOYSTICKS,
        "A7800 device mapping count is invalid"
    );
    let mut slots = BTreeSet::new();
    let mut ids = BTreeSet::new();
    let mut mappings = String::new();
    for (native_id, number) in devices {
        ensure!(
            (1..=8).contains(number) && slots.insert(*number) && ids.insert(*native_id),
            "A7800 native device mapping is duplicated or invalid"
        );
        ensure!(
            !native_id.is_empty()
                && native_id.len() <= 4096
                && native_id
                    .chars()
                    .all(|ch| !ch.is_control() && ch != '\u{fffe}' && ch != '\u{ffff}'),
            "A7800 native device name cannot be represented safely in XML"
        );
        ensure!(
            observed_ids.iter().any(|id| id == native_id)
                && observed_ids
                    .iter()
                    .filter(|id| id.contains(native_id))
                    .count()
                    == 1,
            "A7800 native device name is missing, partial, or ambiguous"
        );
        let escaped = native_id
            .replace('&', "&amp;")
            .replace('"', "&quot;")
            .replace('<', "&lt;")
            .replace('>', "&gt;")
            .replace('\'', "&apos;");
        mappings.push_str(&format!(
            "      <mapdevice device=\"{escaped}\" controller=\"JOYCODE_{number}\" />\n"
        ));
    }
    for port in ports {
        for token in port.tokens.values() {
            if let Some(rest) = token.strip_prefix("JOYCODE_") {
                let number: u8 = rest.split('_').next().unwrap_or("").parse()?;
                ensure!(
                    slots.contains(&number),
                    "A7800 input uses an unmapped joystick"
                );
            }
        }
    }
    Ok(controller_xml(ports)?.replacen("    <input>\n", &format!("    <input>\n{mappings}"), 1))
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
        pub content: PathBuf,
        pub machine: Machine,
        pub cfg_directory: PathBuf,
        pub probe_program: PathBuf,
        pub sdl_library: PathBuf,
        pub executable_sha256: String,
        #[serde(default = "default_threshold")]
        pub threshold_basis_points: u16,
        pub players: Vec<Player>,
    }

    fn default_threshold() -> u16 {
        3000
    }

    impl SavedSetup {
        pub(crate) fn validate(&self) -> Result<()> {
            ensure!(
                !self.emulator_id.trim().is_empty(),
                "A7800 setup needs an emulator identity"
            );
            for path in [
                &self.content,
                &self.cfg_directory,
                &self.probe_program,
                &self.sdl_library,
            ] {
                ensure!(
                    path.is_absolute()
                        && !path
                            .components()
                            .any(|part| matches!(part, std::path::Component::ParentDir)),
                    "A7800 setup paths must be absolute without parent traversal"
                );
            }
            ensure!(
                self.executable_sha256.len() == 64
                    && self
                        .executable_sha256
                        .bytes()
                        .all(|byte| byte.is_ascii_hexdigit()),
                "A7800 setup needs a trusted executable SHA-256"
            );
            ensure!(
                self.threshold_basis_points <= 10_000,
                "A7800 joystick threshold must be 0 through 10000 basis points"
            );
            let limit = catalog()
                .emulator_profiles
                .iter()
                .find(|profile| profile.id == PROFILE_ID)
                .and_then(|profile| profile.native_launch.as_ref())
                .context("Missing A7800 native profile")?
                .max_players;
            ensure!(
                !self.players.is_empty() && self.players.len() <= limit,
                "A7800 setup needs one or two players"
            );
            let mut controllers = BTreeSet::new();
            for (index, player) in self.players.iter().enumerate() {
                ensure!(
                    usize::from(player.player) == index + 1
                        && !player.controller_id.trim().is_empty()
                        && controllers.insert(&player.controller_id),
                    "A7800 players must be distinct, contiguous, and start at player one"
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
                .context("Missing A7800 native profile")?;
            let mut players = Vec::new();
            for player in &self.players {
                let calibration = calibrations
                    .get(&player.controller_id)
                    .context("A7800 controller has no saved calibration")?;
                ensure!(
                    calibration.os == "linux",
                    "A7800 mapping requires Linux physical calibration"
                );
                let mapping = calibration.plan_profile(profile)?;
                ensure!(
                    mapping.rows.iter().all(|row| row.physical_id.is_some()
                        && row
                            .input
                            .as_ref()
                            .is_some_and(|input| input.native.is_some())),
                    "A7800 needs native calibration for every Pro-Line control"
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
                "machine": self.machine,
                "players": players,
                "launch_ready": false,
                "launch_integration": "partial",
                "detail": "Native Linux launch builds a private MAME v10 controller profile and filtered cfg directory, then rechecks the exact SDL2 routes. Only the base NTSC/PAL machines and Pro-Line digital controls are supported; runtime behavior remains unverified."
            }))
        }
    }

    pub(crate) fn validate_setups(setups: &[SavedSetup]) -> Result<()> {
        ensure!(setups.len() <= 1024, "Too many A7800 saved setups");
        let mut identities = BTreeSet::new();
        for setup in setups {
            setup.validate()?;
            ensure!(
                identities.insert((&setup.emulator_id, &setup.content)),
                "Duplicate A7800 emulator/content setup"
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
        duckstation::DigitalInput,
        file_hash,
        linux_classic::AxisEndpoints,
        sdl2::{Device, Snapshot},
        sdl2_physical::PhysicalMap,
    };
    use std::{
        collections::{BTreeMap, BTreeSet, HashMap},
        fs,
        io::Read,
        path::{Path, PathBuf},
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
            .arg("--sdl2-inventory")
            .arg("--sdl-library")
            .arg(&setup.sdl_library);
        if let Some(path) = path {
            command.arg("--sdl2-controls-for-path").arg(path);
        }
        let (output, _) = capture(&mut command, cancel)?;
        let snapshot: Snapshot =
            serde_json::from_slice(&output).context("Invalid A7800 SDL2 capture")?;
        ensure!(
            snapshot.version[0] == 2
                && snapshot.library.canonicalize()? == setup.sdl_library.canonicalize()?
                && snapshot.library_sha256 == file_hash(&setup.sdl_library)?,
            "A7800 helper inspected a different SDL2 runtime"
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

    fn normalized_name(device: &Device) -> Result<String> {
        let name = device
            .name
            .as_deref()
            .context("A7800 SDL device has no name")?;
        ensure!(
            !name.is_empty()
                && name.len() <= 4096
                && name
                    .chars()
                    .all(|ch| ch.is_ascii() && !ch.is_ascii_control()),
            "A7800 SDL device name is unsafe"
        );
        let normalized: String = name
            .chars()
            .filter(|ch| !ch.is_ascii_whitespace())
            .collect();
        ensure!(
            !normalized.is_empty(),
            "A7800 SDL device name becomes empty"
        );
        Ok(normalized)
    }

    fn native_name(device: &Device, devices: &[Device]) -> Result<String> {
        let normalized = normalized_name(device)?;
        let names = devices
            .iter()
            .map(normalized_name)
            .collect::<Result<Vec<_>>>()?;
        ensure!(
            names
                .iter()
                .filter(|other| other.contains(&normalized))
                .count()
                == 1,
            "A7800 whitespace-stripped SDL device name is duplicated or a substring"
        );
        Ok(normalized)
    }

    fn mapped_items(
        calibration: &Calibration,
        device: &Device,
        threshold: f32,
    ) -> Result<BTreeMap<String, crate::controller_mame_native::tokens::Item>> {
        let profile = crate::controller_catalog::catalog()
            .emulator_profiles
            .iter()
            .find(|profile| profile.id == PROFILE_ID)
            .context("Missing A7800 native profile")?;
        let physical = PhysicalMap::from_device(device)?;
        let state = device
            .sampled_state
            .as_ref()
            .context("A7800 SDL released state is missing")?;
        state.validate(
            device
                .controls
                .as_ref()
                .context("A7800 SDL control counts are missing")?,
        )?;
        let mut result = BTreeMap::new();
        for row in calibration.plan_profile(profile)?.rows {
            let input = row
                .input
                .as_ref()
                .context("A7800 Pro-Line control is not calibrated")?;
            let native = input
                .native
                .as_ref()
                .context("A7800 requires measured native controls")?;
            let measured = input.axis.as_ref().map(|axis| AxisEndpoints {
                released: axis.released,
                pressed: axis.pressed,
            });
            let translated = crate::controller_mame_native::sdl::switch_item(
                device,
                native.code,
                measured,
                threshold,
            )?;
            let released = match physical.digital_input(native.code, measured)? {
                DigitalInput::Button(index) => state.buttons.get(&index) == Some(&false),
                DigitalInput::Hat { index, direction } => state
                    .hats
                    .get(&index)
                    .is_some_and(|mask| mask & direction == 0),
                DigitalInput::Axis {
                    index, released, ..
                } => state.axes.get(&index) == Some(&released),
            };
            ensure!(
                released,
                "Release the A7800 controls before launch preparation"
            );
            ensure!(
                result.insert(row.target_id, translated).is_none(),
                "A7800 target control appears twice"
            );
        }
        ensure!(
            result.len() == CONTROLS.len(),
            "A7800 native mapping is incomplete"
        );
        Ok(result)
    }

    fn read_optional(path: &Path) -> Result<Option<Vec<u8>>> {
        let mut file = match fs::File::open(path) {
            Ok(file) => file,
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(None),
            Err(error) => return Err(error.into()),
        };
        ensure!(
            file.metadata()?.is_file(),
            "A7800 cfg is not a regular file"
        );
        let mut bytes = Vec::new();
        file.by_ref()
            .take(16 * 1024 * 1024 + 1)
            .read_to_end(&mut bytes)?;
        ensure!(bytes.len() <= 16 * 1024 * 1024, "A7800 cfg is too large");
        Ok(Some(bytes))
    }

    struct SourceCopy {
        source: PathBuf,
        canonical: Option<PathBuf>,
        bytes: Option<Vec<u8>>,
        copy: Option<(PathBuf, Vec<u8>)>,
    }

    pub(crate) struct PreparedSession {
        profile_directory: tempfile::TempDir,
        profile: PathBuf,
        profile_bytes: Vec<u8>,
        cfg_directory: tempfile::TempDir,
        configs: Vec<SourceCopy>,
        physical_paths: Vec<String>,
        native_names: Vec<String>,
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
                "A7800 content must be a direct regular file with canonical ancestry"
            );
            let mut selected = Vec::new();
            for player in &setup.players {
                let found = inventory
                    .iter()
                    .filter(|device| device.stable_id == player.controller_id)
                    .collect::<Vec<_>>();
                ensure!(
                    found.len() == 1 && !found[0].is_virtual,
                    "A7800 physical controller is missing or ambiguous"
                );
                selected.push(found[0].device_path.clone());
            }
            let topology = InputTopology::capture(&selected)?;
            let initial = routing(observe(setup, None, cancel)?);
            ensure!(
                initial.devices.len() <= MAX_JOYSTICKS,
                "A7800 SDL2 runtime supports at most eight joysticks"
            );
            let threshold = f32::from(setup.threshold_basis_points) / 10_000.0;
            let mut physical_paths = Vec::new();
            let mut captures = Vec::new();
            let mut names = Vec::new();
            let mut token_maps = Vec::new();
            for (player, selected_path) in setup.players.iter().zip(&selected) {
                let path = topology.resolve_runtime_path(
                    selected_path,
                    initial
                        .devices
                        .iter()
                        .filter_map(|device| device.path.as_deref()),
                )?;
                ensure!(
                    !physical_paths.contains(&path),
                    "A7800 players share a controller"
                );
                let captured = observe(setup, Some(&path), cancel)?;
                initial.ensure_same_routing(&routing(captured.clone()))?;
                topology.verify()?;
                let device = captured.device_at_path(&path)?;
                names.push(native_name(device, &captured.devices)?);
                token_maps.push(mapped_items(
                    calibrations
                        .get(&player.controller_id)
                        .context("A7800 calibration disappeared")?,
                    device,
                    threshold,
                )?);
                physical_paths.push(path);
                captures.push(captured);
            }
            let observed_names = initial
                .devices
                .iter()
                .map(normalized_name)
                .collect::<Result<Vec<_>>>()?;
            let tokens = token_maps
                .iter()
                .enumerate()
                .map(|(index, items)| {
                    items
                        .iter()
                        .map(|(control, item)| {
                            let control = match control.as_str() {
                                "a" => "button1",
                                "b" => "button2",
                                other => other,
                            };
                            Ok((control.to_owned(), item.token(u8::try_from(index + 1)?)?))
                        })
                        .collect::<Result<BTreeMap<_, _>>>()
                })
                .collect::<Result<Vec<_>>>()?;
            let ports = tokens
                .iter()
                .enumerate()
                .map(|(index, tokens)| Port {
                    player: u8::try_from(index + 1).expect("two players fit in u8"),
                    kind: ControllerKind::ProlineJoystick,
                    tokens,
                })
                .collect::<Vec<_>>();
            let devices = names
                .iter()
                .enumerate()
                .map(|(index, name)| (name.as_str(), u8::try_from(index + 1).unwrap()))
                .collect::<Vec<_>>();
            let xml = mapped_controller_xml(&ports, &devices, &observed_names)?;
            let profile_directory = tempfile::Builder::new()
                .prefix("lunchbox-a7800-ctrlr-")
                .tempdir()?;
            ensure!(
                !profile_directory.path().to_string_lossy().contains(';'),
                "A7800 controller path contains a multipath separator"
            );
            let profile = profile_directory.path().join("lunchbox-a7800.cfg");
            let profile_bytes = xml.into_bytes();
            fs::write(&profile, &profile_bytes)?;

            let mut types = BTreeSet::new();
            for player in &setup.players {
                for direction in ["UP", "DOWN", "LEFT", "RIGHT"] {
                    types.insert(format!("P{}_JOYSTICK_{direction}", player.player));
                }
                types.insert(format!("P{}_BUTTON1", player.player));
                types.insert(format!("P{}_BUTTON2", player.player));
            }
            let cfg_directory = tempfile::Builder::new()
                .prefix("lunchbox-a7800-cfg-")
                .tempdir()?;
            let mut configs = Vec::new();
            for name in [
                "default.cfg".to_owned(),
                format!("{}.cfg", setup.machine.basename()),
            ] {
                let source = setup.cfg_directory.join(&name);
                let bytes = read_optional(&source)?;
                let canonical = bytes.as_ref().map(|_| source.canonicalize()).transpose()?;
                let copy = if let Some(bytes) = &bytes {
                    let filtered =
                        crate::controller_mame_native::overrides::without_panel_overrides(
                            bytes, &types,
                        )?;
                    let path = cfg_directory.path().join(name);
                    fs::write(&path, &filtered)?;
                    Some((path, filtered))
                } else {
                    None
                };
                configs.push(SourceCopy {
                    source,
                    canonical,
                    bytes,
                    copy,
                });
            }
            let mut hashes = BTreeMap::new();
            for path in [
                &setup.content,
                &setup.probe_program,
                &setup.sdl_library,
                &profile,
            ] {
                hashes.insert(path.clone(), file_hash(path)?);
            }
            let prepared = Self {
                profile_directory,
                profile,
                profile_bytes,
                cfg_directory,
                configs,
                physical_paths,
                native_names: names,
                captures,
                topology,
                initial,
                setup: setup.clone(),
                hashes,
            };
            prepared.verify(cancel)?;
            Ok(prepared)
        }

        pub(crate) fn arguments(&self) -> Vec<std::ffi::OsString> {
            let threshold = format!(
                "{}.{:04}",
                self.setup.threshold_basis_points / 10_000,
                self.setup.threshold_basis_points % 10_000
            );
            let mut result = vec![
                self.setup.machine.basename().into(),
                "-cart".into(),
                self.setup.content.as_os_str().to_owned(),
                "-ctrlrpath".into(),
                self.profile_directory.path().as_os_str().to_owned(),
                "-ctrlr".into(),
                "lunchbox-a7800".into(),
                "-cfg_directory".into(),
                self.cfg_directory.path().as_os_str().to_owned(),
                "-joystickprovider".into(),
                "sdl".into(),
                "-joystick".into(),
                "-nosixaxis".into(),
                "-joystick_threshold".into(),
                threshold.into(),
            ];
            for (player, native_name) in self.setup.players.iter().zip(&self.native_names) {
                result.push(format!("-joy{}", player.player).into());
                result.push("proline_joystick".into());
                result.push(format!("-joy_idx{}", player.player).into());
                result.push(native_name.into());
            }
            result
        }

        pub(crate) fn verify(&self, cancel: &AtomicBool) -> Result<()> {
            cancelled(cancel)?;
            self.topology.verify()?;
            ensure!(
                fs::symlink_metadata(&self.profile)?.is_file()
                    && fs::read(&self.profile)? == self.profile_bytes,
                "A7800 private controller profile changed"
            );
            for (path, hash) in &self.hashes {
                ensure!(file_hash(path)? == *hash, "A7800 launch input changed");
            }
            for config in &self.configs {
                ensure!(
                    read_optional(&config.source)? == config.bytes,
                    "A7800 source cfg changed during preparation"
                );
                if let Some(canonical) = &config.canonical {
                    ensure!(
                        config.source.canonicalize()? == *canonical,
                        "A7800 cfg was redirected"
                    );
                }
                if let Some((path, bytes)) = &config.copy {
                    ensure!(
                        fs::symlink_metadata(path)?.is_file() && fs::read(path)? == *bytes,
                        "A7800 private cfg changed before startup"
                    );
                }
            }
            let fresh = routing(observe(&self.setup, None, cancel)?);
            self.initial.ensure_same_routing(&fresh)?;
            for (path, expected) in self.physical_paths.iter().zip(&self.captures) {
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

        pub(crate) fn physical_paths(&self) -> &[String] {
            &self.physical_paths
        }
    }

    #[cfg(test)]
    mod tests {
        use super::*;

        fn device(name: &str) -> Device {
            Device {
                device_index: 0,
                instance_id: 1,
                path: Some("/dev/input/js0".into()),
                name: Some(name.into()),
                is_game_controller: false,
                guid: String::new(),
                mapping: None,
                controls: None,
                linux_classic: None,
                linux_evdev: None,
                sampled_state: None,
            }
        }

        #[test]
        fn source_style_names_strip_whitespace_and_reject_substrings() {
            let first = device("Game Pad");
            assert_eq!(
                native_name(&first, std::slice::from_ref(&first)).unwrap(),
                "GamePad"
            );
            let second = device("Game Pad Pro");
            assert!(native_name(&first, &[first.clone(), second]).is_err());
        }
    }
}

#[cfg(target_os = "linux")]
pub(crate) mod native_command {
    use super::*;
    use crate::{
        controller_catalog::Calibration,
        controller_native_process::{cancelled, native_pid},
        controllers::ControllerDevice,
        emulator::{EmulatorExecutable, LaunchPlan, RomEmulatorOption},
    };
    use lunchbox_controller_probe::file_hash;
    use std::{
        collections::{BTreeSet, HashMap},
        os::unix::fs::MetadataExt,
        path::{Path, PathBuf},
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
                "A7800 executable differs from the saved trusted runtime"
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
                "A7800 launch plan changed after preparation"
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
                    "A7800 exited before controller handoff"
                );
                if let Some(pid) = native_pid(child.id(), &self.executable)?
                    && self.ready(pid)?
                {
                    self.inputs.check_health()?;
                    return Ok(());
                }
                ensure!(
                    Instant::now() < deadline,
                    "A7800 did not open the selected SDL2 controllers before timeout"
                );
                std::thread::sleep(Duration::from_millis(25));
            }
        }

        fn ready(&self, pid: u32) -> Result<bool> {
            let expected_sdl = self.setup.sdl_library.canonicalize()?;
            let maps = std::fs::read_to_string(format!("/proc/{pid}/maps"))?;
            if !maps.lines().any(|line| {
                let path = line
                    .split_whitespace()
                    .skip(5)
                    .collect::<Vec<_>>()
                    .join(" ")
                    .replace("\\040", " ");
                Path::new(&path) == expected_sdl
            }) {
                return Ok(false);
            }
            let mut opened = BTreeSet::new();
            for (index, entry) in std::fs::read_dir(format!("/proc/{pid}/fd"))?.enumerate() {
                ensure!(
                    index < 4096,
                    "A7800 open descriptor inventory exceeds limit"
                );
                match std::fs::metadata(entry?.path()) {
                    Ok(metadata) => {
                        opened.insert((metadata.dev(), metadata.ino(), metadata.rdev()));
                    }
                    Err(error) if error.kind() == std::io::ErrorKind::NotFound => continue,
                    Err(error) => return Err(error.into()),
                }
            }
            for path in self.inputs.physical_paths() {
                let metadata = std::fs::metadata(path)?;
                if !opened.contains(&(metadata.dev(), metadata.ino(), metadata.rdev())) {
                    return Ok(false);
                }
            }
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
            anyhow::bail!("A7800 calibrated launch requires native Linux");
        };
        ensure!(
            option.emulator_name.eq_ignore_ascii_case("A7800")
                && setup.emulator_id == option.emulator_id
                && original.environment.is_empty()
                && original.retroarch_content.is_none(),
            "A7800 identity differs or custom environment needs resolution"
        );
        let executable = executable.canonicalize()?;
        ensure!(
            executable == original.program.canonicalize()?,
            "A7800 launch executable differs from selection"
        );
        ensure!(
            original.arguments.as_slice() == [setup.content.as_os_str()],
            "A7800 calibrated launch requires exactly the saved content argument"
        );
        ensure!(
            file_hash(&executable)?.eq_ignore_ascii_case(&setup.executable_sha256),
            "A7800 executable differs from the saved trusted runtime"
        );
        let inputs = session::PreparedSession::prepare(setup, calibrations, inventory, cancel)?;
        let mut plan = original.clone();
        plan.program = executable.clone();
        plan.arguments = inputs.arguments();
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
    fn emits_only_declared_proline_controls() {
        let tokens = BTreeMap::from([
            ("up".into(), "JOYCODE_1_UP".into()),
            ("down".into(), "JOYCODE_1_DOWN".into()),
            ("left".into(), "JOYCODE_1_LEFT".into()),
            ("right".into(), "JOYCODE_1_RIGHT".into()),
            ("button1".into(), "JOYCODE_1_BUTTON1".into()),
            ("button2".into(), "JOYCODE_1_BUTTON2".into()),
        ]);
        let xml = controller_xml(&[Port {
            player: 1,
            kind: ControllerKind::ProlineJoystick,
            tokens: &tokens,
        }])
        .unwrap();
        assert!(xml.contains("P1_JOYSTICK_UP"));
        assert!(xml.contains("P1_BUTTON2"));
        assert!(!xml.contains("START1"));
    }

    #[test]
    fn vcs_requires_no_second_button() {
        let tokens = BTreeMap::from([
            ("up".into(), "KEYCODE_UP".into()),
            ("down".into(), "KEYCODE_DOWN".into()),
            ("left".into(), "KEYCODE_LEFT".into()),
            ("right".into(), "KEYCODE_RIGHT".into()),
            ("button1".into(), "KEYCODE_Z".into()),
        ]);
        assert!(
            controller_xml(&[Port {
                player: 1,
                kind: ControllerKind::VcsJoystick,
                tokens: &tokens
            }])
            .is_ok()
        );
    }

    #[test]
    fn writer_rejects_shared_controls() {
        let tokens = BTreeMap::from([
            ("up".into(), "JOYCODE_1_UP".into()),
            ("down".into(), "JOYCODE_1_DOWN".into()),
            ("left".into(), "JOYCODE_1_LEFT".into()),
            ("right".into(), "JOYCODE_1_RIGHT".into()),
            ("button1".into(), "JOYCODE_1_BUTTON1".into()),
            ("button2".into(), "JOYCODE_1_BUTTON1".into()),
        ]);
        assert!(
            controller_xml(&[Port {
                player: 1,
                kind: ControllerKind::ProlineJoystick,
                tokens: &tokens,
            }])
            .is_err()
        );
    }

    #[test]
    fn mapped_xml_rejects_substring_names() {
        let tokens = BTreeMap::from([
            ("up".into(), "JOYCODE_1_UP".into()),
            ("down".into(), "JOYCODE_1_DOWN".into()),
            ("left".into(), "JOYCODE_1_LEFT".into()),
            ("right".into(), "JOYCODE_1_RIGHT".into()),
            ("button1".into(), "JOYCODE_1_BUTTON1".into()),
            ("button2".into(), "JOYCODE_1_BUTTON2".into()),
        ]);
        let ports = [Port {
            player: 1,
            kind: ControllerKind::ProlineJoystick,
            tokens: &tokens,
        }];
        assert!(
            mapped_controller_xml(
                &ports,
                &[("GamePad", 1)],
                &["GamePad".into(), "GamePadPro".into()],
            )
            .is_err()
        );
        let xml = mapped_controller_xml(&ports, &[("Pad&One", 1)], &["Pad&One".into()]).unwrap();
        assert!(xml.contains("device=\"Pad&amp;One\" controller=\"JOYCODE_1\""));
    }
}
