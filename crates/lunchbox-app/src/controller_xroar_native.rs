//! XRoar standalone SDL3 controller-profile writer.
//!
//! Pinned source: xroar/xroar commit
//! 0229f97a636c3c80d51fd27e7d145d792f0a8932. `src/joystick.c` prints
//! `joy` profile blocks, while `src/sdl3/joystick_sdl3.c` defines the exact
//! `physical:<device>,<axis>` and `physical:<device>,%<button-mask>` grammar.
//! The device component is a runtime enumeration index and must be measured
//! and rechecked for the exact child process immediately before launch.

use anyhow::{Context, Result, ensure};
use serde::{Deserialize, Serialize};
use std::collections::BTreeSet;

pub(crate) const SOURCE_COMMIT: &str = "0229f97a636c3c80d51fd27e7d145d792f0a8932";
pub(crate) const PROFILE_ID: &str = "xroar:standalone-xroar-analog-joystick";
pub(crate) const CONTROLS: [(&str, &str); 6] = [
    ("up", "joy-axis Y negative"),
    ("down", "joy-axis Y positive"),
    ("left", "joy-axis X negative"),
    ("right", "joy-axis X positive"),
    ("b", "joy-button 0"),
    ("a", "joy-button 1"),
];

#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub(crate) enum GuestPort {
    Right,
    Left,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) struct PhysicalAxis {
    /// SDL_GamepadAxis value (0 through 5).
    pub index: u8,
    /// XRoar's `-` control prefix, applied before reading the axis.
    pub inverted: bool,
}

impl GuestPort {
    fn profile_name(self) -> &'static str {
        match self {
            Self::Right => "lunchbox-right",
            Self::Left => "lunchbox-left",
        }
    }

    fn selector(self) -> &'static str {
        match self {
            Self::Right => "joy-right",
            Self::Left => "joy-left",
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) struct PortProfile {
    pub port: GuestPort,
    /// SDL3 enumeration index measured for this launch.
    pub runtime_index: u8,
    /// X and Y logical SDL_GamepadAxis values.
    pub axes: [PhysicalAxis; 2],
    /// Two non-empty masks over SDL_GamepadButton values 0 through 26.
    pub button_masks: [u32; 2],
}

fn validate(profile: PortProfile) -> Result<()> {
    ensure!(
        profile.runtime_index <= 31,
        "XRoar SDL runtime index is out of range"
    );
    ensure!(
        profile.axes.iter().all(|axis| axis.index <= 5)
            && profile.axes[0].index != profile.axes[1].index,
        "XRoar axes must be distinct SDL3 axes 0 through 5"
    );
    const VALID_BUTTONS: u32 = (1_u32 << 27) - 1;
    ensure!(
        profile
            .button_masks
            .iter()
            .all(|mask| *mask != 0 && (*mask & !VALID_BUTTONS) == 0),
        "XRoar button masks must select SDL3 buttons 0 through 26"
    );
    Ok(())
}

/// Append source-shaped joystick profile declarations to a copied
/// `xroar.conf`. XRoar processes declarations in order, so these final
/// `lunchbox-right` / `lunchbox-left` profiles and selectors supersede earlier
/// declarations without deleting machine, ROM, tape, disk or snapshot config.
pub(crate) fn patch_config(baseline: &[u8], profiles: &[PortProfile]) -> Result<String> {
    ensure!(baseline.len() <= 1024 * 1024, "XRoar config is too large");
    ensure!(
        !profiles.is_empty() && profiles.len() <= 2,
        "XRoar profile count is invalid"
    );
    let original = std::str::from_utf8(baseline).context("XRoar config is not UTF-8")?;
    ensure!(!original.contains('\0'), "XRoar config contains a NUL byte");
    let newline = if original.contains("\r\n") {
        "\r\n"
    } else {
        "\n"
    };
    let mut ports = BTreeSet::new();
    let mut indices = BTreeSet::new();
    let mut output = original.to_owned();
    if !output.is_empty() && !output.ends_with('\n') {
        output.push_str(newline);
    }
    output.push_str(&format!(
        "# Lunchbox measured SDL3 joystick profiles{newline}"
    ));
    for profile in profiles {
        validate(*profile)?;
        ensure!(ports.insert(profile.port), "XRoar guest port is duplicated");
        ensure!(
            indices.insert(profile.runtime_index),
            "XRoar SDL runtime index is duplicated"
        );
        let name = profile.port.profile_name();
        output.push_str(&format!("joy {name}{newline}"));
        output.push_str(&format!(
            "  joy-desc 'Lunchbox measured SDL3 controller'{newline}"
        ));
        for (axis_number, physical_axis) in profile.axes.iter().enumerate() {
            let inversion = if physical_axis.inverted { "-" } else { "" };
            output.push_str(&format!(
                "  joy-axis {axis_number}='physical:{},{inversion}{}'{newline}",
                profile.runtime_index, physical_axis.index
            ));
        }
        for (button_number, mask) in profile.button_masks.iter().enumerate() {
            output.push_str(&format!(
                "  joy-button {button_number}='physical:{},%{}'{newline}",
                profile.runtime_index, mask
            ));
        }
        output.push_str(newline);
        output.push_str(&format!("{} {name}{newline}", profile.port.selector()));
    }
    Ok(output)
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
        pub profile_id: String,
        pub emulator_id: String,
        pub content: PathBuf,
        /// Existing user configuration; it is copied, never edited in place.
        pub config_path: PathBuf,
        pub probe_program: PathBuf,
        /// Exact SDL3 library used by the selected XRoar build.
        pub sdl_library: PathBuf,
        /// Mapping database supplied to both the probe and XRoar.
        pub mapping_database: PathBuf,
        pub executable_sha256: String,
        pub players: Vec<Player>,
    }

    impl SavedSetup {
        pub(crate) fn validate(&self) -> Result<()> {
            ensure!(
                self.profile_id == PROFILE_ID,
                "XRoar setup has the wrong native profile"
            );
            ensure!(
                !self.emulator_id.trim().is_empty(),
                "XRoar setup needs an emulator identity"
            );
            for path in [
                &self.content,
                &self.config_path,
                &self.probe_program,
                &self.sdl_library,
                &self.mapping_database,
            ] {
                ensure!(
                    path.is_absolute()
                        && !path
                            .components()
                            .any(|part| matches!(part, std::path::Component::ParentDir)),
                    "XRoar setup paths must be absolute without parent traversal"
                );
            }
            ensure!(
                self.config_path.file_name().and_then(|name| name.to_str()) == Some("xroar.conf"),
                "XRoar config_path must name xroar.conf"
            );
            ensure!(
                self.executable_sha256.len() == 64
                    && self
                        .executable_sha256
                        .bytes()
                        .all(|byte| byte.is_ascii_hexdigit()),
                "XRoar setup needs a trusted executable SHA-256"
            );
            let limit = catalog()
                .emulator_profiles
                .iter()
                .find(|profile| profile.id == self.profile_id)
                .and_then(|profile| profile.native_launch.as_ref())
                .context("Missing XRoar native profile")?
                .max_players;
            ensure!(
                !self.players.is_empty() && self.players.len() <= limit,
                "XRoar setup has an invalid player count"
            );
            let mut controllers = BTreeSet::new();
            for (index, player) in self.players.iter().enumerate() {
                ensure!(
                    usize::from(player.player) == index + 1
                        && !player.controller_id.trim().is_empty()
                        && controllers.insert(&player.controller_id),
                    "XRoar players must be distinct, contiguous, and start at player one"
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
                .find(|profile| profile.id == self.profile_id)
                .context("Missing XRoar native profile")?;
            let mut players = Vec::new();
            for player in &self.players {
                let calibration = calibrations
                    .get(&player.controller_id)
                    .context("XRoar controller has no saved calibration")?;
                ensure!(
                    ["linux", "macos", "windows"].contains(&calibration.os.as_str())
                        && calibration.backend != crate::controller_sdl3::BACKEND,
                    "XRoar mapping needs a desktop physical calibration"
                );
                let mapping = calibration.plan_profile(profile)?;
                ensure!(
                    mapping.rows.iter().all(|row| {
                        row.physical_id.is_some()
                            && row
                                .input
                                .as_ref()
                                .is_some_and(|input| input.native.is_some())
                    }),
                    "XRoar mapping needs every joystick control calibrated"
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
                "profile_id": self.profile_id,
                "players": players,
                "launch_ready": false,
                "launch_integration": "partial",
                "detail": "Native Linux launch copies xroar.conf, supplies it with first-option -c, disables config auto-save, and rechecks the exact SDL3 joystick order and bindings. Each directional pair must resolve to one SDL gamepad axis. Runtime behavior is unverified."
            }))
        }
    }

    pub(crate) fn validate_setups(setups: &[SavedSetup]) -> Result<()> {
        ensure!(setups.len() <= 1024, "Too many XRoar saved setups");
        let mut identities = BTreeSet::new();
        for setup in setups {
            setup.validate()?;
            ensure!(
                identities.insert((&setup.emulator_id, &setup.content)),
                "Duplicate XRoar emulator/content setup"
            );
        }
        Ok(())
    }
}

mod session {
    use super::*;
    #[cfg(target_os = "linux")]
    use crate::controller_bizhawk_guard::InputTopology;
    #[cfg(not(target_os = "linux"))]
    use crate::controller_native_platform as platform;
    use crate::{
        controller_catalog::{Calibration, EmulatorProfile},
        controller_native_process::{cancelled, capture},
        controller_pcsx2::sdl::{AxisRange, Input as SdlInput},
        controllers::ControllerDevice,
    };
    use anyhow::bail;
    use lunchbox_controller_probe::{Snapshot, file_hash, linux_classic::AxisEndpoints};
    use std::{
        collections::{BTreeMap, HashMap},
        fs,
        path::{Path, PathBuf},
        process::Command,
        sync::atomic::AtomicBool,
    };

    #[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
    enum LogicalInput {
        Button(u8),
        Axis { index: u8, range: AxisRange },
    }

    impl LogicalInput {
        fn from_sdl(input: SdlInput) -> Result<Self> {
            match input {
                SdlInput::GamepadButton { index } => Ok(Self::Button(index)),
                SdlInput::GamepadAxis { index, range } => Ok(Self::Axis { index, range }),
                _ => bail!("XRoar requires a resolved SDL gamepad control"),
            }
        }
    }

    fn translate(
        calibration: &Calibration,
        profile: &EmulatorProfile,
        device: &lunchbox_controller_probe::Device,
    ) -> Result<BTreeMap<String, LogicalInput>> {
        ensure!(
            device.is_gamepad,
            "XRoar selected device is not an SDL gamepad"
        );
        let resolved = device
            .resolved
            .as_ref()
            .context("XRoar SDL3 resolved bindings are absent")?;
        let physical = device
            .linux_classic
            .as_ref()
            .context("XRoar SDL3 classic Linux control map is absent")?;
        physical.validate_counts(resolved)?;
        let mapping = calibration.plan_profile(profile)?;
        let mut result = BTreeMap::new();
        let mut outputs = BTreeSet::new();
        for row in mapping.rows {
            let binding = row
                .input
                .as_ref()
                .context("XRoar joystick control is not calibrated")?;
            let native = binding
                .native
                .as_ref()
                .context("XRoar requires measured native controls")?;
            let endpoints = binding.axis.as_ref().map(|axis| AxisEndpoints {
                released: axis.released,
                pressed: axis.pressed,
            });
            let raw = physical.digital_input(native.code, endpoints)?;
            let logical = LogicalInput::from_sdl(crate::controller_pcsx2::physical::digital(
                resolved, true, raw,
            )?)?;
            ensure!(
                outputs.insert(logical),
                "XRoar target controls resolve to the same SDL input"
            );
            ensure!(
                result.insert(row.target_id, logical).is_none(),
                "XRoar target control appears twice"
            );
        }
        Ok(result)
    }

    fn axis_pair(
        negative: LogicalInput,
        positive: LogicalInput,
        name: &str,
    ) -> Result<PhysicalAxis> {
        match (negative, positive) {
            (
                LogicalInput::Axis {
                    index: first,
                    range: first_range,
                },
                LogicalInput::Axis {
                    index: second,
                    range: second_range,
                },
            ) if first == second
                && first_range != AxisRange::Full
                && second_range != AxisRange::Full
                && first_range != second_range =>
            {
                Ok(PhysicalAxis {
                    index: first,
                    inverted: first_range == AxisRange::Positive,
                })
            }
            _ => bail!(
                "XRoar {name} directions must resolve to opposite halves of one SDL gamepad axis"
            ),
        }
    }

    fn port_profile(
        player: u8,
        runtime_index: u8,
        mapped: &BTreeMap<String, LogicalInput>,
    ) -> Result<PortProfile> {
        let input = |name: &str| {
            mapped
                .get(name)
                .copied()
                .with_context(|| format!("XRoar control {name} is absent"))
        };
        let button = |name: &str| match input(name)? {
            LogicalInput::Button(index) if index <= 26 => Ok(1_u32 << index),
            _ => bail!("XRoar fire controls must resolve to SDL gamepad buttons"),
        };
        Ok(PortProfile {
            port: match player {
                1 => GuestPort::Right,
                2 => GuestPort::Left,
                _ => bail!("XRoar supports only the right and left joystick ports"),
            },
            runtime_index,
            axes: [
                axis_pair(input("left")?, input("right")?, "horizontal")?,
                axis_pair(input("up")?, input("down")?, "vertical")?,
            ],
            button_masks: [button("b")?, button("a")?],
        })
    }

    type Routing = Vec<(String, String, Option<String>, Option<u16>, Option<String>)>;

    fn routing(snapshot: &Snapshot) -> Routing {
        snapshot
            .devices
            .iter()
            .map(|device| {
                (
                    device.path.clone().unwrap_or_default(),
                    device.guid.clone(),
                    device.gamepad_name.clone(),
                    device.gamepad_index,
                    device.mapping.clone(),
                )
            })
            .collect()
    }

    fn comparable(snapshot: &Snapshot) -> Result<serde_json::Value> {
        let mut value = serde_json::to_value(snapshot)?;
        value
            .as_object_mut()
            .context("Invalid XRoar SDL snapshot shape")?
            .remove("warnings");
        Ok(value)
    }

    fn observe(
        setup: &settings::SavedSetup,
        paths: &[String],
        cancel: &AtomicBool,
    ) -> Result<Snapshot> {
        let mut command = Command::new(&setup.probe_program);
        command
            .arg("--sdl-library")
            .arg(&setup.sdl_library)
            .arg("--mapping-db")
            .arg(&setup.mapping_database)
            .arg("--hint")
            .arg("SDL_JOYSTICK_LINUX_CLASSIC=1");
        for path in paths {
            command.arg("--bindings-for-path").arg(path);
        }
        let (output, _) = capture(&mut command, cancel)?;
        let snapshot: Snapshot =
            serde_json::from_slice(&output).context("Invalid XRoar SDL3 capture")?;
        ensure!(
            snapshot.library.canonicalize()? == setup.sdl_library.canonicalize()?
                && snapshot.library_sha256 == file_hash(&setup.sdl_library)?
                && snapshot.mapping_database_sha256.as_deref()
                    == Some(file_hash(&setup.mapping_database)?.as_str())
                && snapshot
                    .effective_hints
                    .get("SDL_JOYSTICK_LINUX_CLASSIC")
                    .and_then(Option::as_deref)
                    == Some("1"),
            "XRoar helper inspected different SDL3 inputs"
        );
        Ok(snapshot)
    }

    fn xconfig_string(path: &Path) -> Result<String> {
        let value = path.to_str().context("XRoar path is not UTF-8")?;
        ensure!(
            !value.chars().any(char::is_control),
            "XRoar path contains a control character"
        );
        let mut quoted = String::from("\"");
        for character in value.chars() {
            match character {
                '\\' => quoted.push_str("\\\\"),
                '"' => quoted.push_str("\\\""),
                other => quoted.push(other),
            }
        }
        quoted.push('"');
        Ok(quoted)
    }

    fn patch_runtime_config(
        baseline: &[u8],
        mapping_database: &Path,
        profiles: &[PortProfile],
    ) -> Result<String> {
        let mut patched = patch_config(baseline, profiles)?;
        patched.push_str(&format!(
            "joy-db-file {}\n",
            xconfig_string(mapping_database)?
        ));
        Ok(patched)
    }

    pub(crate) struct PreparedSession {
        directory: tempfile::TempDir,
        pub(crate) private_config: PathBuf,
        runtime_paths: Vec<String>,
        #[cfg(target_os = "linux")]
        topology: InputTopology,
        snapshot: Snapshot,
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
            let profile = crate::controller_catalog::catalog()
                .emulator_profiles
                .iter()
                .find(|profile| profile.id == setup.profile_id)
                .context("Missing XRoar native profile")?;
            let mut selected = Vec::new();
            for player in &setup.players {
                let found = inventory
                    .iter()
                    .filter(|device| device.stable_id == player.controller_id)
                    .collect::<Vec<_>>();
                ensure!(
                    found.len() == 1 && !found[0].is_virtual,
                    "XRoar physical controller is missing or ambiguous"
                );
                selected.push(found[0].device_path.clone());
            }
            #[cfg(target_os = "linux")]
            let topology = InputTopology::capture(&selected)?;
            let initial = observe(setup, &[], cancel)?;
            let visible_paths = initial
                .devices
                .iter()
                .filter_map(|device| device.path.as_deref())
                .collect::<Vec<_>>();
            // Linux resolves through the sysfs topology; other hosts
            // match the SDL device-interface path and require uniqueness.
            #[cfg(target_os = "linux")]
            let runtime_paths = selected
                .iter()
                .map(|path| topology.resolve_runtime_path(path, visible_paths.iter().copied()))
                .collect::<Result<Vec<_>>>()?;
            #[cfg(not(target_os = "linux"))]
            let runtime_paths = selected
                .iter()
                .map(|path| {
                    let path_string = path.to_string_lossy().into_owned();
                    let candidates = initial
                        .devices
                        .iter()
                        .filter(|device| device.path.as_deref() == Some(path_string.as_str()))
                        .collect::<Vec<_>>();
                    ensure!(
                        candidates.len() == 1,
                        "XRoar physical controller is missing or ambiguous in SDL"
                    );
                    Ok(path_string)
                })
                .collect::<Result<Vec<_>>>()?;
            let snapshot = observe(setup, &runtime_paths, cancel)?;
            ensure!(
                routing(&initial) == routing(&snapshot),
                "XRoar SDL3 routing changed during binding capture"
            );
            let mut profiles = Vec::new();
            for (player, path) in setup.players.iter().zip(&runtime_paths) {
                let device = snapshot.device_at_path(path)?;
                let runtime_index = snapshot
                    .devices
                    .iter()
                    .position(|candidate| std::ptr::eq(candidate, device))
                    .context("XRoar measured device disappeared")?;
                let runtime_index = u8::try_from(runtime_index)
                    .context("XRoar SDL joystick index is out of range")?;
                let mapped = translate(
                    calibrations
                        .get(&player.controller_id)
                        .context("XRoar calibration disappeared")?,
                    profile,
                    device,
                )?;
                profiles.push(port_profile(player.player, runtime_index, &mapped)?);
            }
            let baseline =
                fs::read(&setup.config_path).context("Reading the declared XRoar configuration")?;
            let patched = patch_runtime_config(&baseline, &setup.mapping_database, &profiles)?;
            let directory = tempfile::Builder::new()
                .prefix("lunchbox-xroar-native-")
                .tempdir()?;
            let private_config = directory.path().join("xroar.conf");
            fs::write(&private_config, patched)?;
            let mut hashes = BTreeMap::new();
            for path in [
                &setup.probe_program,
                &setup.sdl_library,
                &setup.mapping_database,
                &setup.content,
                &setup.config_path,
                &private_config,
            ] {
                hashes.insert(path.clone(), file_hash(path)?);
            }
            let prepared = Self {
                directory,
                private_config,
                runtime_paths,
                #[cfg(target_os = "linux")]
                topology,
                snapshot,
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
                "XRoar private configuration disappeared"
            );
            #[cfg(target_os = "linux")]
            self.topology.verify()?;
            for (path, expected) in &self.hashes {
                ensure!(file_hash(path)? == *expected, "XRoar launch input changed");
            }
            let current = observe(&self.setup, &self.runtime_paths, cancel)?;
            ensure!(
                comparable(&current)? == comparable(&self.snapshot)?,
                "XRoar SDL3 routing or resolved bindings changed before launch"
            );
            #[cfg(not(target_os = "linux"))]
            for path in &self.runtime_paths {
                let count = current
                    .devices
                    .iter()
                    .filter(|device| device.path.as_deref() == Some(path.as_str()))
                    .count();
                ensure!(count == 1, "XRoar SDL device path is missing or ambiguous");
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
            let current = observe(&self.setup, &self.runtime_paths, &AtomicBool::new(false))?;
            ensure!(
                comparable(&current)? == comparable(&self.snapshot)?,
                "XRoar SDL3 routing or resolved bindings changed"
            );
            Ok(())
        }
    }
}

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
                file_hash(&self.executable)? == self.setup.executable_sha256,
                "XRoar executable differs from the saved trusted runtime"
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
                "XRoar launch plan changed after preparation"
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
            anyhow::bail!("XRoar calibrated launch requires a native build");
        };
        ensure!(
            option.emulator_name.eq_ignore_ascii_case("xroar")
                && setup.emulator_id == option.emulator_id
                && original.environment.is_empty(),
            "XRoar identity differs or custom environment needs resolution"
        );
        let executable = executable.canonicalize()?;
        ensure!(
            executable == original.program.canonicalize()?,
            "XRoar launch executable differs from selection"
        );
        ensure!(
            original.arguments.as_slice() == [setup.content.as_os_str()],
            "XRoar calibrated launch requires exactly the saved game argument"
        );
        ensure!(
            file_hash(&executable)? == setup.executable_sha256,
            "XRoar executable differs from the saved trusted runtime"
        );
        let inputs = session::PreparedSession::prepare(setup, calibrations, inventory, cancel)?;
        let mut plan = original.clone();
        plan.arguments = vec![
            "-c".into(),
            inputs.private_config.as_os_str().to_owned(),
            "-no-config-auto-save".into(),
            setup.content.as_os_str().to_owned(),
        ];
        plan.environment
            .push(("SDL_JOYSTICK_LINUX_CLASSIC".into(), "1".into()));
        plan.environment.push((
            "SDL_GAMECONTROLLERCONFIG_FILE".into(),
            setup.mapping_database.as_os_str().to_owned(),
        ));
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
    fn appends_exact_physical_profile_grammar() {
        let output = patch_config(
            b"machine dragon64\nrompath /opt/roms\n",
            &[PortProfile {
                port: GuestPort::Right,
                runtime_index: 2,
                axes: [
                    PhysicalAxis {
                        index: 0,
                        inverted: false,
                    },
                    PhysicalAxis {
                        index: 1,
                        inverted: true,
                    },
                ],
                button_masks: [(1 << 0) | (1 << 9), 1 << 1],
            }],
        )
        .unwrap();
        assert!(output.contains("machine dragon64\nrompath /opt/roms\n"));
        assert!(output.contains("joy lunchbox-right\n"));
        assert!(output.contains("joy-axis 0='physical:2,0'\n"));
        assert!(output.contains("joy-axis 1='physical:2,-1'\n"));
        assert!(output.contains("joy-button 0='physical:2,%513'\n"));
        assert!(output.ends_with("joy-right lunchbox-right\n"));
    }

    #[test]
    fn rejects_duplicate_ports_indices_and_invalid_controls() {
        let base = PortProfile {
            port: GuestPort::Left,
            runtime_index: 0,
            axes: [
                PhysicalAxis {
                    index: 0,
                    inverted: false,
                },
                PhysicalAxis {
                    index: 1,
                    inverted: false,
                },
            ],
            button_masks: [1, 2],
        };
        assert!(patch_config(b"", &[base, base]).is_err());
        let invalid = PortProfile {
            port: GuestPort::Right,
            runtime_index: 1,
            axes: [
                PhysicalAxis {
                    index: 6,
                    inverted: false,
                },
                PhysicalAxis {
                    index: 1,
                    inverted: false,
                },
            ],
            button_masks: [1, 1 << 27],
        };
        assert!(patch_config(b"", &[invalid]).is_err());
    }
}
