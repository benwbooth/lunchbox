//! Gopher64 native JSON input profiles, not Mupen64Plus/libretro mappings.
//!
//! Functional contract pinned to gopher64/gopher64
//! 0bb9fbba638f5cebe3b8a3c1c245abcfec8b0132 (v1.1.36-12):
//! - `src/ui/config.rs` serializes `config.json`; `InputProfile.inputs` is
//!   nineteen pairs of optional externally tagged `InputItem` values.
//! - `src/ui/input_profile.rs` fixes indices 0..17 as D-pad right/left/down/up,
//!   Start, Z, B, A, C right/left/down/up, R, L and stick right/left/down/up;
//!   index 18 is the hotkey activator.
//! - `src/ui/input.rs` opens the exact path stored in `controller_assignment`,
//!   chooses SDL3 gamepad input when `dinput` is false, and selects one named
//!   profile per port through `input_profile_binding`.
//! - `src/ui.rs` uses `$XDG_CONFIG_HOME/gopher64/config.json` separately from
//!   `$XDG_DATA_HOME/gopher64/{saves,states}`. A private config home therefore
//!   leaves native saves and states in their ordinary persistent data root.
//! - a `portable.txt` beside the executable overrides XDG roots, so portable
//!   mode is rejected rather than touching the user's live configuration.
use anyhow::{Context, Result, ensure};
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet, HashMap};
use std::path::{Component, Path, PathBuf};

use crate::controller_pcsx2::sdl::{AxisRange, Input as SdlInput};

pub(crate) const PROFILE_ID: &str = "gopher64:standalone-n64";

/// N64 target control -> Gopher64's `InputProfile.inputs` index.
pub(crate) const CONTROLS: [(&str, usize); 18] = [
    ("right", 0),
    ("left", 1),
    ("down", 2),
    ("up", 3),
    ("start", 4),
    ("z", 5),
    ("b", 6),
    ("a", 7),
    ("c_right", 8),
    ("c_left", 9),
    ("c_down", 10),
    ("c_up", 11),
    ("r", 12),
    ("l", 13),
    ("stick_right", 14),
    ("stick_left", 15),
    ("stick_down", 16),
    ("stick_up", 17),
];

#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
enum MappedInput {
    Button(u8),
    Axis { index: u8, positive: bool },
}

/// The pinned Gopher64 input schema.  `Config::new` first tries the complete
/// config and then falls back to deserializing just `input`; either path must
/// accept the patched input or Gopher64 silently starts with defaults.
#[allow(dead_code)]
#[derive(Deserialize)]
struct Gopher64InputButton {
    id: i32,
}

#[allow(dead_code)]
#[derive(Deserialize)]
struct Gopher64InputAxis {
    id: i32,
    axis: i16,
    initial_state: i16,
}

#[allow(dead_code)]
#[derive(Deserialize)]
struct Gopher64InputHat {
    id: i32,
    direction: u8,
}

#[allow(dead_code)]
#[derive(Deserialize)]
enum Gopher64InputItem {
    Key(Gopher64InputButton),
    ControllerButton(Gopher64InputButton),
    ControllerAxis(Gopher64InputAxis),
    JoystickButton(Gopher64InputButton),
    JoystickHat(Gopher64InputHat),
    JoystickAxis(Gopher64InputAxis),
}

#[derive(Deserialize)]
struct Gopher64InputProfile {
    inputs: [[Option<Gopher64InputItem>; 2]; 19],
    dinput: bool,
    deadzone: i32,
}

#[derive(Deserialize)]
struct Gopher64Input {
    input_profiles: BTreeMap<String, Gopher64InputProfile>,
    input_profile_binding: [String; 4],
    controller_assignment: [Option<String>; 4],
    controller_enabled: [bool; 4],
    transfer_pak: [bool; 4],
    gb_rom_path: [String; 4],
    gb_ram_path: [String; 4],
    emulate_vru: bool,
}

impl MappedInput {
    fn json(self) -> serde_json::Value {
        match self {
            Self::Button(id) => serde_json::json!({"ControllerButton":{"id":id}}),
            Self::Axis { index, positive } => serde_json::json!({
                "ControllerAxis":{"id":index,"axis":if positive { 1 } else { -1 },"initial_state":0}
            }),
        }
    }

    fn from_sdl(input: SdlInput) -> Result<Self> {
        match input {
            SdlInput::GamepadButton { index } => Ok(Self::Button(index)),
            SdlInput::GamepadAxis { index, range } => Ok(Self::Axis {
                index,
                positive: match range {
                    AxisRange::Positive => true,
                    AxisRange::Negative => false,
                    AxisRange::Full => {
                        anyhow::bail!("Gopher64 directional profile entries need one SDL axis half")
                    }
                },
            }),
            _ => anyhow::bail!(
                "Gopher64's gamepad profile cannot consume an unmapped raw SDL control"
            ),
        }
    }
}

fn check_range(value: MappedInput) -> Result<()> {
    match value {
        MappedInput::Button(id) => {
            ensure!(id < 26, "Gopher64 SDL gamepad button is out of range")
        }
        MappedInput::Axis { index, .. } => {
            ensure!(index < 6, "Gopher64 SDL gamepad axis is out of range")
        }
    }
    Ok(())
}

fn profile_json(
    mapped: &BTreeMap<String, MappedInput>,
    secondary: &BTreeMap<String, MappedInput>,
) -> Result<serde_json::Value> {
    ensure!(
        mapped.len() == CONTROLS.len(),
        "Gopher64 needs every N64 gameplay control"
    );
    ensure!(
        mapped
            .keys()
            .all(|key| CONTROLS.iter().any(|(control, _)| key == control)),
        "Gopher64 mapping contains an unknown N64 control"
    );
    let mut inputs = vec![serde_json::json!([null, null]); 19];
    let mut used = BTreeSet::new();
    for (control, index) in CONTROLS {
        let value = *mapped
            .get(control)
            .with_context(|| format!("Gopher64 control {control} is absent"))?;
        ensure!(
            used.insert(value),
            "Gopher64 cannot assign one SDL output to multiple N64 controls"
        );
        check_range(value)?;
        inputs[index] = serde_json::json!([value.json(), null]);
    }
    // Twin inputs (N64 Z on twin-trigger pads) share their target through
    // the profile's second slot instead of displacing the primary binding.
    for (control, value) in secondary {
        let (_, index) = CONTROLS
            .iter()
            .find(|(target, _)| target == control)
            .context("Gopher64 twin target is outside the N64 contract")?;
        ensure!(
            mapped.contains_key(control.as_str()),
            "Gopher64 twin needs its bound primary control"
        );
        check_range(*value)?;
        inputs[*index] = serde_json::json!([mapped[control].json(), value.json()]);
    }
    let axis = |name: &str| match mapped.get(name) {
        Some(MappedInput::Axis { index, positive }) => Ok((*index, *positive)),
        _ => anyhow::bail!("Gopher64 {name} needs a proportional SDL gamepad axis"),
    };
    let left = axis("stick_left")?;
    let right = axis("stick_right")?;
    let up = axis("stick_up")?;
    let down = axis("stick_down")?;
    ensure!(
        left.0 == right.0 && left.1 != right.1,
        "Gopher64 stick left/right must be opposite halves of one SDL axis"
    );
    ensure!(
        up.0 == down.0 && up.1 != down.1 && up.0 != left.0,
        "Gopher64 stick up/down must be opposite halves of a distinct SDL axis"
    );
    Ok(serde_json::json!({"inputs":inputs,"dinput":false,"deadzone":5}))
}

fn patched_config(
    baseline: &[u8],
    profiles: &[(u8, String, serde_json::Value)],
) -> Result<Vec<u8>> {
    ensure!(
        baseline.len() <= 4 * 1024 * 1024,
        "Gopher64 config is too large"
    );
    let mut root: serde_json::Value =
        serde_json::from_slice(baseline).context("Parsing Gopher64 config.json")?;
    let root_object = root
        .as_object_mut()
        .context("Gopher64 config root must be a JSON object")?;
    let input = root_object
        .entry("input")
        .or_insert_with(|| serde_json::json!({}))
        .as_object_mut()
        .context("Gopher64 input config must be a JSON object")?;
    let saved_profiles = input
        .entry("input_profiles")
        .or_insert_with(|| serde_json::json!({}))
        .as_object_mut()
        .context("Gopher64 input_profiles must be a JSON object")?;

    let mut bindings = ["default", "default", "default", "default"].map(str::to_owned);
    let mut assignments: [Option<String>; 4] = std::array::from_fn(|_| None);
    let mut enabled = [false; 4];
    for (port, path, profile) in profiles {
        ensure!((1..=4).contains(port), "Gopher64 port is out of range");
        let index = usize::from(*port - 1);
        let name = format!("Lunchbox Player {port}");
        ensure!(
            assignments[index].replace(path.clone()).is_none(),
            "Gopher64 port is duplicated"
        );
        bindings[index].clone_from(&name);
        enabled[index] = true;
        saved_profiles.insert(name, profile.clone());
    }
    input.insert(
        "input_profile_binding".into(),
        serde_json::to_value(bindings)?,
    );
    input.insert(
        "controller_assignment".into(),
        serde_json::to_value(assignments)?,
    );
    input.insert("controller_enabled".into(), serde_json::to_value(enabled)?);
    input
        .entry("transfer_pak")
        .or_insert_with(|| serde_json::json!([false, false, false, false]));
    input
        .entry("gb_rom_path")
        .or_insert_with(|| serde_json::json!(["", "", "", ""]));
    input
        .entry("gb_ram_path")
        .or_insert_with(|| serde_json::json!(["", "", "", ""]));
    input
        .entry("emulate_vru")
        .or_insert(serde_json::Value::Bool(false));
    serde_json::from_value::<Gopher64Input>(serde_json::Value::Object(input.clone()))
        .context("Patched Gopher64 input config does not match the pinned schema")?;
    serde_json::to_vec_pretty(&root).context("Serializing private Gopher64 config")
}

/// Launch-time Gopher64 setup discovery. First launches work without a
/// hand-written JSON setup: the ROM, config baseline, probe, SDL runtime,
/// and executable trust are all resolved from the launch itself, following
/// the mgba guided-discovery precedent. Nothing here persists; the setup
/// lives only for the launch.
pub(crate) mod guided {
    use super::settings::SavedSetup;
    use crate::{
        controller_native_process::{cancelled, capture},
        emulator::{EmulatorExecutable, LaunchPlan, RomEmulatorOption},
        platform_process,
    };
    use anyhow::{Context, Result, ensure};
    use std::{
        path::{Path, PathBuf},
        process::Command,
        sync::atomic::AtomicBool,
    };

    /// Probe helper: explicit override for integration tests, else the
    /// sibling controller-probe binary beside the Lunchbox executable.
    pub(crate) fn helper() -> Result<PathBuf> {
        if let Some(path) = std::env::var_os("LUNCHBOX_CONTROLLER_PROBE") {
            let path = PathBuf::from(path);
            ensure!(
                path.is_file(),
                "LUNCHBOX_CONTROLLER_PROBE does not name a file"
            );
            return Ok(path);
        }
        let path = std::env::current_exe()?
            .parent()
            .context("Missing Lunchbox application directory")?
            .join(if cfg!(windows) {
                "lunchbox-controller-probe.exe"
            } else {
                "lunchbox-controller-probe"
            });
        ensure!(
            path.is_file(),
            "This Lunchbox installation is missing lunchbox-controller-probe; install the full package"
        );
        Ok(path)
    }

    pub(crate) fn flatpak_info(command: &Path, app_id: &str, flag: &str) -> Result<String> {
        let (output, _) = capture(
            Command::new(command).args(["info", flag, app_id]),
            &AtomicBool::new(false),
        )?;
        Ok(String::from_utf8(output)
            .context("Gopher64 Flatpak info was not UTF-8")?
            .trim()
            .to_owned())
    }

    /// Trust anchor: content hash for native builds, Flatpak commit for
    /// sandboxed apps. Both are 64 lowercase hex characters.
    pub(crate) fn executable_trust(
        option: &RomEmulatorOption,
        cancel: &AtomicBool,
    ) -> Result<String> {
        match &option.executable {
            EmulatorExecutable::Native(program) => {
                Ok(lunchbox_controller_probe::file_hash(program)?)
            }
            EmulatorExecutable::Flatpak { command, app_id } => {
                let _ = cancel;
                let commit = flatpak_info(command, app_id, "--show-commit")?;
                ensure!(
                    commit.len() == 64 && commit.bytes().all(|byte| byte.is_ascii_hexdigit()),
                    "Gopher64 Flatpak commit is not a trusted SHA-256"
                );
                Ok(commit)
            }
            EmulatorExecutable::Wine { .. } => {
                anyhow::bail!(
                    "Gopher64 calibrated launch needs a native or Flatpak build, not Wine"
                )
            }
        }
    }

    /// SDL3 library the emulator process will load, so the host probe
    /// enumerates devices through the identical runtime.
    pub(crate) fn sdl_library(
        option: &RomEmulatorOption,
        cancel: &AtomicBool,
    ) -> Result<(PathBuf, PathBuf)> {
        match &option.executable {
            EmulatorExecutable::Native(program) => {
                let directory = program
                    .parent()
                    .context("Missing Gopher64 executable directory")?;
                let names = ["SDL3.dll", "libSDL3.dylib", "libSDL3.so.0"];
                let mut candidates: Vec<_> = [
                    directory.to_path_buf(),
                    directory.join("../lib"),
                    directory.join("../Frameworks"),
                ]
                .into_iter()
                .flat_map(|dir| names.iter().map(move |name| dir.join(name)))
                .filter(|path| path.is_file())
                .collect();
                for relative in [
                    "../Frameworks/SDL3.framework/SDL3",
                    "../Frameworks/SDL3.framework/Versions/A/SDL3",
                ] {
                    let framework = directory.join(relative);
                    if framework.is_file() {
                        candidates.push(framework);
                    }
                }
                #[cfg(target_os = "linux")]
                if candidates.is_empty() {
                    let (out, _) =
                        capture(platform_process::host_command("ldd").arg(program), cancel)?;
                    let out = std::str::from_utf8(&out).context("ldd output was not UTF-8")?;
                    for line in out.lines().filter(|line| line.contains("libSDL3.so")) {
                        if let Some(path) = line.split_whitespace().find(|s| s.starts_with('/')) {
                            candidates.push(path.into());
                        }
                    }
                }
                let library = candidates
                    .into_iter()
                    .find(|path| path.is_file())
                    .context("Could not locate the SDL3 library used by Gopher64")?;
                let dir = library.parent().unwrap_or(Path::new(".")).to_path_buf();
                Ok((library, dir))
            }
            EmulatorExecutable::Flatpak { command, app_id } => {
                let runtime = flatpak_info(command, app_id, "--show-runtime")?;
                let root = PathBuf::from(flatpak_info(command, &runtime, "--show-location")?)
                    .join("files");
                let mut dirs = vec![root.join("lib")];
                if let Ok(entries) = std::fs::read_dir(root.join("lib")) {
                    dirs.extend(
                        entries
                            .filter_map(Result::ok)
                            .map(|entry| entry.path())
                            .filter(|path| path.is_dir()),
                    );
                }
                let library = dirs
                    .iter()
                    .map(|dir| dir.join("libSDL3.so.0"))
                    .find(|path| path.is_file())
                    .context("Gopher64's Flatpak SDL3 library could not be located")?;
                let dir = library.parent().unwrap_or(Path::new(".")).to_path_buf();
                Ok((library, dir))
            }
            EmulatorExecutable::Wine { .. } => {
                anyhow::bail!(
                    "Gopher64 calibrated launch needs a native or Flatpak build, not Wine"
                )
            }
        }
    }

    /// Live user config backing the private session. Flatpak forces
    /// XDG_CONFIG_HOME inside the sandbox (verified), so the session patches
    /// the real per-app config with backup/restore instead of a private dir.
    pub(crate) fn config_base(option: &RomEmulatorOption) -> Result<PathBuf> {
        let dirs = directories::BaseDirs::new().context("Missing user directories")?;
        match &option.executable {
            EmulatorExecutable::Native(_) => Ok(dirs.config_dir().join("gopher64/config.json")),
            EmulatorExecutable::Flatpak { app_id, .. } => {
                ensure!(
                    !app_id.is_empty() && !app_id.contains('/') && !app_id.contains(".."),
                    "Gopher64 Flatpak app identity is not a plain app id"
                );
                Ok(dirs
                    .home_dir()
                    .join(".var/app")
                    .join(app_id)
                    .join("config/gopher64/config.json"))
            }
            EmulatorExecutable::Wine { .. } => {
                anyhow::bail!(
                    "Gopher64 calibrated launch needs a native or Flatpak build, not Wine"
                )
            }
        }
    }

    /// The single ROM argument, skipping launcher wrappers and flags.
    /// Anything else is a guess about which file to play: fail instead.
    pub(crate) fn content_argument(
        plan: &LaunchPlan,
        option: &RomEmulatorOption,
    ) -> Result<PathBuf> {
        let args: Vec<&std::ffi::OsString> = match &option.executable {
            EmulatorExecutable::Native(_) => {
                ensure!(
                    plan.arguments.len() == 1,
                    "Gopher64 native launch needs exactly the game argument"
                );
                plan.arguments.iter().collect()
            }
            EmulatorExecutable::Flatpak { app_id, .. } => {
                let files: Vec<&std::ffi::OsString> = plan
                    .arguments
                    .iter()
                    .filter(|arg| {
                        *arg != "run"
                            && !arg.to_string_lossy().starts_with("--")
                            && arg.to_string_lossy() != *app_id
                    })
                    .collect();
                ensure!(
                    files.len() == 1,
                    "Gopher64 Flatpak launch needs exactly one game argument"
                );
                files
            }
            EmulatorExecutable::Wine { .. } => {
                anyhow::bail!(
                    "Gopher64 calibrated launch needs a native or Flatpak build, not Wine"
                )
            }
        };
        let content = PathBuf::from(args[0]);
        ensure!(
            content.is_absolute() && content.is_file(),
            "Gopher64 needs the selected ROM's absolute file path"
        );
        Ok(content)
    }

    pub(crate) fn discover(
        option: &RomEmulatorOption,
        plan: &LaunchPlan,
        ids: &[String],
        cancel: &AtomicBool,
    ) -> Result<SavedSetup> {
        cancelled(cancel)?;
        ensure!(
            !ids.is_empty() && ids.len() <= 4,
            "Gopher64 supports one to four controller ports; assign players in Controller setup"
        );
        let content = content_argument(plan, option)?;
        let setup = SavedSetup {
            emulator_id: option.emulator_id.clone(),
            content,
            config_path: config_base(option)?,
            probe_program: helper()?,
            sdl_library: sdl_library(option, cancel)?.0,
            executable_sha256: executable_trust(option, cancel)?,
            players: ids
                .iter()
                .enumerate()
                .map(|(index, id)| super::settings::Player {
                    player: (index + 1) as u8,
                    controller_id: id.clone(),
                })
                .collect(),
        };
        setup.validate()?;
        cancelled(cancel)?;
        Ok(setup)
    }
}

pub(crate) mod settings {
    use super::*;
    use crate::controller_catalog::{Calibration, catalog};

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
        /// Existing user config copied and patched inside the private XDG root.
        pub config_path: PathBuf,
        pub probe_program: PathBuf,
        pub sdl_library: PathBuf,
        pub executable_sha256: String,
        pub players: Vec<Player>,
    }

    impl SavedSetup {
        pub(crate) fn validate(&self) -> Result<()> {
            ensure!(
                !self.emulator_id.trim().is_empty(),
                "Gopher64 needs an emulator identity"
            );
            for path in [
                &self.content,
                &self.config_path,
                &self.probe_program,
                &self.sdl_library,
            ] {
                ensure!(
                    path.is_absolute()
                        && !path
                            .components()
                            .any(|part| matches!(part, Component::ParentDir)),
                    "Gopher64 setup paths must be absolute without parent traversal"
                );
            }
            ensure!(
                self.config_path.file_name().and_then(|name| name.to_str()) == Some("config.json"),
                "Gopher64 config_path must name config.json"
            );
            ensure!(
                self.executable_sha256.len() == 64
                    && self
                        .executable_sha256
                        .bytes()
                        .all(|byte| byte.is_ascii_hexdigit()),
                "Gopher64 needs a trusted executable SHA-256"
            );
            ensure!(
                (1..=4).contains(&self.players.len()),
                "Gopher64 supports one to four controller ports"
            );
            let mut ports = BTreeSet::new();
            let mut controllers = BTreeSet::new();
            for player in &self.players {
                ensure!(
                    (1..=4).contains(&player.player)
                        && ports.insert(player.player)
                        && !player.controller_id.trim().is_empty()
                        && controllers.insert(&player.controller_id),
                    "Gopher64 players and physical controllers must be distinct"
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
                .context("Missing native Gopher64 profile")?;
            let mut players = Vec::new();
            for player in &self.players {
                let calibration = calibrations
                    .get(&player.controller_id)
                    .context("Gopher64 controller has no saved calibration")?;
                ensure!(
                    ["linux", "macos", "windows"].contains(&calibration.os.as_str()),
                    "Gopher64 native mapping requires a desktop calibration"
                );
                let mapping = calibration.plan_profile(profile)?;
                ensure!(
                    mapping.rows.iter().all(|row| row.physical_id.is_some()
                        && row
                            .input
                            .as_ref()
                            .is_some_and(|input| input.native.is_some())),
                    "Gopher64 needs native calibration for every N64 gameplay control"
                );
                players.push(serde_json::json!({
                    "port":player.player,"controller_id":player.controller_id,
                    "source_layout":calibration.layout,"target_layout":profile.target_layout,
                    "mapping":mapping
                }));
            }
            Ok(serde_json::json!({
                "players":players,"launch_ready":false,"launch_integration":"partial",
                "detail":"Gopher64 JSON dispatch is connected but runtime-untested. Launch copies the declared config into a private XDG_CONFIG_HOME, patches only controller profiles/assignments, and leaves XDG_DATA_HOME untouched so native saves and states remain persistent. The target SDL3 runtime resolves measured Linux controls. Transfer Pak/VRU configuration is preserved from the copied config but not authored by this adapter; portable mode is rejected."
            }))
        }
    }

    pub(crate) fn validate_setups(setups: &[SavedSetup]) -> Result<()> {
        ensure!(setups.len() <= 1024, "Too many Gopher64 saved setups");
        let mut identities = BTreeSet::new();
        for setup in setups {
            setup.validate()?;
            ensure!(
                identities.insert((&setup.emulator_id, &setup.content)),
                "Duplicate Gopher64 emulator/content setup"
            );
        }
        Ok(())
    }
}

pub(crate) mod session {
    use super::*;
    #[cfg(target_os = "linux")]
    use crate::controller_bizhawk_guard::InputTopology;
    #[cfg(not(target_os = "linux"))]
    use crate::controller_native_platform as platform;
    use crate::{
        controller_catalog::Calibration,
        controller_native_process::{cancelled, capture},
        controllers::ControllerDevice,
    };
    use lunchbox_controller_probe::{Snapshot, file_hash, linux_classic::AxisEndpoints};
    use std::{process::Command, sync::atomic::AtomicBool};

    fn observe(
        setup: &settings::SavedSetup,
        paths: &[String],
        cancel: &AtomicBool,
    ) -> Result<Snapshot> {
        // Load the emulator's own SDL and its runtime dependencies, not
        // Lunchbox's potentially different SDL version (ares precedent).
        let mut command = crate::platform_process::host_command("env");
        #[cfg(target_os = "linux")]
        if let Some(dir) = setup.sdl_library.parent() {
            command.arg(format!("LD_LIBRARY_PATH={}", dir.display()));
        }
        command.arg(&setup.probe_program);
        command
            .arg("--sdl-library")
            .arg(&setup.sdl_library)
            .arg("--hint")
            .arg("SDL_JOYSTICK_LINUX_CLASSIC=1");
        if cfg!(target_os = "linux")
            && let Some(dir) = setup.sdl_library.parent()
            && dir.join("libudev.so.1").is_file()
        {
            command
                .arg("--runtime-library")
                .arg(dir.join("libudev.so.1"));
        }
        for path in paths {
            command.arg("--bindings-for-path").arg(path);
        }
        let (output, _) = capture(&mut command, cancel)?;
        let snapshot: Snapshot =
            serde_json::from_slice(&output).context("Invalid Gopher64 SDL3 capture")?;
        ensure!(
            snapshot.library.canonicalize()? == setup.sdl_library.canonicalize()?
                && snapshot.library_sha256 == file_hash(&setup.sdl_library)?
                && snapshot
                    .effective_hints
                    .get("SDL_JOYSTICK_LINUX_CLASSIC")
                    .and_then(Option::as_deref)
                    == Some("1"),
            "Gopher64 helper inspected a different SDL runtime or backend"
        );
        Ok(snapshot)
    }

    fn comparable(snapshot: &Snapshot, bindings: bool) -> Result<serde_json::Value> {
        let mut value = serde_json::to_value(snapshot)?;
        let object = value
            .as_object_mut()
            .context("Invalid Gopher64 snapshot shape")?;
        object.remove("warnings");
        if let Some(report) = object
            .get_mut("player_probe")
            .and_then(serde_json::Value::as_object_mut)
        {
            report.remove("events_processed");
        }
        if !bindings
            && let Some(devices) = object
                .get_mut("devices")
                .and_then(serde_json::Value::as_array_mut)
        {
            for device in devices {
                let object = device
                    .as_object_mut()
                    .context("Invalid Gopher64 device shape")?;
                object.remove("resolved");
                object.remove("linux_classic");
            }
        }
        Ok(value)
    }

    fn mapped_profile(
        calibration: &Calibration,
        device: &lunchbox_controller_probe::Device,
    ) -> Result<serde_json::Value> {
        ensure!(
            device.is_gamepad,
            "Gopher64 needs an SDL3-recognized gamepad"
        );
        let gamepad = device
            .resolved
            .as_ref()
            .context("Gopher64 SDL3 resolved bindings are absent")?;
        let physical = device
            .linux_classic
            .as_ref()
            .context("Gopher64 classic Linux control map is absent")?;
        physical.validate_counts(gamepad)?;
        let profile = crate::controller_catalog::catalog()
            .emulator_profiles
            .iter()
            .find(|profile| profile.id == PROFILE_ID)
            .context("Missing native Gopher64 profile")?;
        let rows = calibration.plan_profile(profile)?.rows;
        let mut mapped = BTreeMap::new();
        for row in &rows {
            let binding = row
                .input
                .as_ref()
                .context("Gopher64 gameplay control is not calibrated")?;
            let native = binding
                .native
                .as_ref()
                .context("Gopher64 requires measured native controls")?;
            let endpoints = binding.axis.as_ref().map(|axis| AxisEndpoints {
                released: axis.released,
                pressed: axis.pressed,
            });
            let raw = physical.digital_input(native.code, endpoints)?;
            // Proportional axes (sticks, analog triggers) resolve through
            // the analog path; buttons and hats through the digital path.
            let translated = match &raw {
                lunchbox_controller_probe::duckstation::DigitalInput::Axis {
                    index,
                    released,
                    pressed,
                } => crate::controller_pcsx2::physical::analog(
                    gamepad,
                    true,
                    lunchbox_controller_probe::duckstation::AnalogInput {
                        index: *index,
                        released: *released,
                        extent: *pressed,
                    },
                )?,
                _ => crate::controller_pcsx2::physical::digital(gamepad, true, raw)?,
            };
            ensure!(
                CONTROLS
                    .iter()
                    .any(|(control, _)| *control == row.target_id),
                "Gopher64 target is outside the N64 contract"
            );
            ensure!(
                mapped
                    .insert(row.target_id.clone(), MappedInput::from_sdl(translated)?)
                    .is_none(),
                "Gopher64 target appears twice"
            );
        }
        let mut secondary = BTreeMap::new();
        for twin in crate::controller_ares::twin_routes(calibration, profile, &rows) {
            let twin_binding = calibration
                .bindings
                .get(&twin.physical_id)
                .context("Gopher64 twin input disappeared")?;
            let twin_native = twin_binding
                .native
                .as_ref()
                .context("Gopher64 twin needs a measured native control")?;
            let twin_endpoints = twin_binding.axis.as_ref().map(|axis| AxisEndpoints {
                released: axis.released,
                pressed: axis.pressed,
            });
            let twin_raw = physical.digital_input(twin_native.code, twin_endpoints)?;
            let twin_translated = match &twin_raw {
                lunchbox_controller_probe::duckstation::DigitalInput::Axis {
                    index,
                    released,
                    pressed,
                } => crate::controller_pcsx2::physical::analog(
                    gamepad,
                    true,
                    lunchbox_controller_probe::duckstation::AnalogInput {
                        index: *index,
                        released: *released,
                        extent: *pressed,
                    },
                )?,
                _ => crate::controller_pcsx2::physical::digital(gamepad, true, twin_raw)?,
            };
            ensure!(
                secondary
                    .insert(twin.target_id, MappedInput::from_sdl(twin_translated)?)
                    .is_none(),
                "Gopher64 twin target appears twice"
            );
        }
        profile_json(&mapped, &secondary)
    }

    fn copy_companion(source_dir: &Path, target_dir: &Path, name: &str) -> Result<Option<PathBuf>> {
        let source = source_dir.join(name);
        if !source.exists() {
            return Ok(None);
        }
        ensure!(source.is_file(), "Gopher64 config companion must be a file");
        let bytes = std::fs::read(&source)?;
        ensure!(
            bytes.len() <= 1024 * 1024,
            "Gopher64 config companion is too large"
        );
        let target = target_dir.join(name);
        std::fs::write(&target, bytes)?;
        Ok(Some(target))
    }

    /// Backup/restore guard for the live Flatpak config. Flatpak forces
    /// XDG_CONFIG_HOME inside the sandbox (verified: --env is ignored), so a
    /// private config dir can never reach the app. The session instead swaps
    /// the live config file aside, runs with the patched file in place, and
    /// restores on drop. The backup always refreshes from the live file at
    /// prepare time, so no edit path can lose user data; a lock file refuses
    /// concurrent sessions, treating dead PIDs as stale.
    struct LiveConfigGuard {
        live: PathBuf,
        backup: PathBuf,
        lock: PathBuf,
    }

    impl LiveConfigGuard {
        fn lock_path(live: &Path) -> PathBuf {
            live.parent()
                .unwrap_or(Path::new("."))
                .join(".lunchbox-gopher64-session.lock")
        }

        fn backup_path(live: &Path) -> PathBuf {
            let mut backup = live.as_os_str().to_owned();
            backup.push(".lunchbox-backup");
            PathBuf::from(backup)
        }

        fn acquire(live: &Path) -> Result<Self> {
            let lock = Self::lock_path(live);
            if let Ok(contents) = std::fs::read_to_string(&lock) {
                let mut parts = contents.split_whitespace();
                let pid = parts.next().and_then(|pid| pid.parse::<u32>().ok());
                let exe = parts.next().map(PathBuf::from).unwrap_or_default();
                let alive = match (pid, exe.as_os_str().is_empty()) {
                    (Some(pid), false) => {
                        crate::controller_native_platform::child_exe_matches(pid, &exe)
                            .unwrap_or(true)
                    }
                    _ => false,
                };
                ensure!(
                    !alive,
                    "Another Lunchbox Gopher64 session is active; refusing to swap its live config (lock {})",
                    lock.display()
                );
            }
            let me = format!(
                "{} {}",
                std::process::id(),
                std::env::current_exe()
                    .map(|path| path.display().to_string())
                    .unwrap_or_default()
            );
            if let Some(parent) = lock.parent() {
                std::fs::create_dir_all(parent).context("Preparing the Gopher64 session lock")?;
            }
            std::fs::write(&lock, me).context("Recording the Gopher64 session lock")?;
            let backup = Self::backup_path(live);
            if live.is_file() {
                std::fs::copy(live, &backup).context("Backing up the live Gopher64 config")?;
            } else if backup.is_file() {
                std::fs::remove_file(&backup).ok();
            }
            Ok(Self {
                live: live.to_path_buf(),
                backup,
                lock,
            })
        }
    }

    impl Drop for LiveConfigGuard {
        fn drop(&mut self) {
            if self.backup.is_file() {
                let _ = std::fs::copy(&self.backup, &self.live);
                let _ = std::fs::remove_file(&self.backup);
            } else {
                let _ = std::fs::remove_file(&self.live);
            }
            let _ = std::fs::remove_file(&self.lock);
        }
    }

    fn sessions_dir() -> Result<PathBuf> {
        // Home-backed storage is visible to both Lunchbox and an emulator
        // Flatpak; /tmp is private in each sandbox and cannot stage handoffs.
        let sessions = directories::BaseDirs::new()
            .context("Missing application data directory")?
            .data_local_dir()
            .join("lunchbox/controller-sessions");
        std::fs::create_dir_all(&sessions)?;
        Ok(sessions)
    }

    pub(crate) struct PreparedSession {
        directory: Option<tempfile::TempDir>,
        pub(crate) config_home: PathBuf,
        runtime_paths: Vec<String>,
        #[cfg(target_os = "linux")]
        topology: InputTopology,
        snapshot: Snapshot,
        setup: settings::SavedSetup,
        hashes: BTreeMap<PathBuf, String>,
        live_guard: Option<LiveConfigGuard>,
    }

    impl PreparedSession {
        pub(crate) fn prepare(
            setup: &settings::SavedSetup,
            calibrations: &HashMap<String, Calibration>,
            inventory: &[ControllerDevice],
            flatpak_app_id: Option<&str>,
            cancel: &AtomicBool,
        ) -> Result<Self> {
            cancelled(cancel)?;
            setup.review(calibrations)?;
            let mut selected = Vec::new();
            for player in &setup.players {
                let found = inventory
                    .iter()
                    .filter(|device| device.stable_id == player.controller_id)
                    .collect::<Vec<_>>();
                ensure!(
                    found.len() == 1 && !found[0].is_virtual,
                    "Gopher64 physical controller is missing or ambiguous"
                );
                selected.push(found[0].device_path.clone());
            }
            #[cfg(target_os = "linux")]
            let topology = InputTopology::capture(&selected)?;
            let initial = observe(setup, &[], cancel)?;
            let mut runtime_paths = Vec::new();
            for path in &selected {
                // Linux resolves through the sysfs topology; other hosts
                // match the SDL device-interface path and require uniqueness.
                #[cfg(target_os = "linux")]
                let runtime = topology.resolve_runtime_path(
                    path,
                    initial
                        .devices
                        .iter()
                        .filter_map(|device| device.path.as_deref()),
                )?;
                #[cfg(not(target_os = "linux"))]
                let runtime = {
                    let path_string = path.to_string_lossy().into_owned();
                    let candidates = initial
                        .devices
                        .iter()
                        .filter(|device| device.path.as_deref() == Some(path_string.as_str()))
                        .collect::<Vec<_>>();
                    ensure!(
                        candidates.len() == 1,
                        "Gopher64 physical controller is missing or ambiguous in SDL"
                    );
                    path_string
                };
                runtime_paths.push(runtime);
            }
            ensure!(
                runtime_paths.iter().collect::<BTreeSet<_>>().len() == runtime_paths.len(),
                "Gopher64 players resolved to the same SDL device"
            );
            let snapshot = observe(setup, &runtime_paths, cancel)?;
            ensure!(
                comparable(&initial, false)? == comparable(&snapshot, false)?,
                "Gopher64 SDL routing changed during binding capture"
            );
            let mut profiles = Vec::new();
            for (player, path) in setup.players.iter().zip(&runtime_paths) {
                profiles.push((
                    player.player,
                    path.clone(),
                    mapped_profile(
                        calibrations
                            .get(&player.controller_id)
                            .context("Gopher64 calibration disappeared")?,
                        snapshot.device_at_path(path)?,
                    )?,
                ));
            }
            let baseline = match std::fs::read(&setup.config_path) {
                Ok(bytes) => bytes,
                Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
                    // First runs have no user config yet. The patched file
                    // carries the complete validated input subtree, which
                    // Gopher64 explicitly accepts as a fallback shape.
                    b"{}".to_vec()
                }
                Err(error) => {
                    return Err(error).context("Reading the declared Gopher64 config.json");
                }
            };
            // Flatpak forces XDG_CONFIG_HOME inside the sandbox (verified),
            // so the session patches the live per-app config with
            // backup/restore instead of an invisible private dir. Native
            // builds keep an auto-cleaned tempdir session.
            let (directory, config_home, live_guard) = match flatpak_app_id {
                Some(_) => {
                    let live = &setup.config_path;
                    let guard = LiveConfigGuard::acquire(live)?;
                    (
                        None,
                        live.parent()
                            .context("Gopher64 config path has no parent")?
                            .to_path_buf(),
                        Some(guard),
                    )
                }
                None => {
                    let directory: tempfile::TempDir = tempfile::Builder::new()
                        .prefix("lunchbox-gopher64-")
                        .tempdir()?;
                    let config_home = directory.path().join("config-home");
                    (Some(directory), config_home, None)
                }
            };
            // Flatpak sessions patch the declared live file in place (the
            // sandbox mounts this exact path); native sessions stage a
            // private copy under the tempdir config home.
            let (target_dir, private_config) = match flatpak_app_id {
                Some(_) => (config_home.clone(), setup.config_path.clone()),
                None => {
                    let staged = config_home.join("gopher64");
                    (staged.clone(), staged.join("config.json"))
                }
            };
            std::fs::create_dir_all(&target_dir)?;
            std::fs::write(&private_config, patched_config(&baseline, &profiles)?)?;
            let source_dir = setup
                .config_path
                .parent()
                .context("Gopher64 config path has no parent")?;
            let mut companions = Vec::new();
            for name in ["cheats.json", "retroachievements.json"] {
                // Flatpak sessions already live in place; copying a file
                // onto itself only risks permission noise.
                if source_dir == target_dir {
                    break;
                }
                if let Some(path) = copy_companion(source_dir, &target_dir, name)? {
                    companions.push(path);
                }
            }

            let mut hashes = BTreeMap::new();
            for path in [
                &setup.probe_program,
                &setup.sdl_library,
                &setup.content,
                &setup.config_path,
                &private_config,
            ] {
                hashes.insert(path.clone(), file_hash(path)?);
            }
            for path in companions {
                hashes.insert(path.clone(), file_hash(&path)?);
            }
            let session = Self {
                directory,
                config_home,
                runtime_paths,
                #[cfg(target_os = "linux")]
                topology,
                snapshot,
                setup: setup.clone(),
                hashes,
                live_guard,
            };
            session.verify(cancel)?;
            Ok(session)
        }

        pub(crate) fn verify(&self, cancel: &AtomicBool) -> Result<()> {
            cancelled(cancel)?;
            #[cfg(target_os = "linux")]
            self.topology.verify()?;
            if let Some(directory) = &self.directory {
                ensure!(directory.path().is_dir(), "Gopher64 session disappeared");
            }
            for (path, expected) in &self.hashes {
                ensure!(
                    file_hash(path)? == *expected,
                    "Gopher64 launch input changed"
                );
            }
            let current = observe(&self.setup, &self.runtime_paths, cancel)?;
            ensure!(
                comparable(&current, true)? == comparable(&self.snapshot, true)?,
                "Gopher64 SDL routing or resolved bindings changed before launch"
            );
            #[cfg(not(target_os = "linux"))]
            for path in &self.runtime_paths {
                let count = current
                    .devices
                    .iter()
                    .filter(|device| device.path.as_deref() == Some(path.as_str()))
                    .count();
                ensure!(
                    count == 1,
                    "Gopher64 SDL device path is missing or ambiguous"
                );
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
                comparable(&current, true)? == comparable(&self.snapshot, true)?,
                "Gopher64 SDL routing or resolved bindings changed"
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
    use std::sync::atomic::AtomicBool;

    pub(crate) struct NativeSession {
        pub(crate) inputs: session::PreparedSession,
        executable: PathBuf,
        flatpak_app_id: Option<String>,
        setup: settings::SavedSetup,
        pub(crate) plan: LaunchPlan,
    }

    impl NativeSession {
        pub(crate) fn check_health(&self) -> Result<()> {
            self.inputs.check_health()
        }

        pub(crate) fn verify(&self, cancel: &AtomicBool) -> Result<()> {
            cancelled(cancel)?;
            match &self.flatpak_app_id {
                None => ensure!(
                    file_hash(&self.executable)? == self.setup.executable_sha256,
                    "Gopher64 executable differs from the saved trusted runtime"
                ),
                Some(app_id) => {
                    let commit =
                        super::guided::flatpak_info(&self.executable, app_id, "--show-commit")?;
                    ensure!(
                        commit == self.setup.executable_sha256,
                        "Gopher64 Flatpak runtime changed; review the controller setup again"
                    );
                }
            }
            self.inputs.verify(cancel)
        }

        pub(crate) fn spawn(
            &mut self,
            plan: &LaunchPlan,
            cancel: &AtomicBool,
        ) -> Result<std::process::Child> {
            ensure!(
                plan == &self.plan,
                "Gopher64 launch plan changed after preparation"
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
        ensure!(
            setup.emulator_id == option.emulator_id && original.environment.is_empty(),
            "Gopher64 identity differs or custom environment needs resolution"
        );
        let (executable, flatpak_app_id) = match &option.executable {
            EmulatorExecutable::Native(program) => {
                let executable = program.canonicalize()?;
                ensure!(
                    executable == original.program.canonicalize()?,
                    "Gopher64 launch executable differs from selection"
                );
                ensure!(
                    !executable
                        .parent()
                        .context("Gopher64 executable has no parent")?
                        .join("portable.txt")
                        .exists(),
                    "Gopher64 portable mode overrides XDG_CONFIG_HOME and is not safely supported"
                );
                ensure!(
                    file_hash(&executable)? == setup.executable_sha256,
                    "Gopher64 executable differs from the saved trusted runtime"
                );
                (executable, None)
            }
            EmulatorExecutable::Flatpak { command, app_id } => {
                let commit = super::guided::flatpak_info(command, app_id, "--show-commit")?;
                ensure!(
                    commit == setup.executable_sha256,
                    "Gopher64 Flatpak runtime changed; review the controller setup again"
                );
                (command.clone(), Some(app_id.clone()))
            }
            EmulatorExecutable::Wine { .. } => {
                anyhow::bail!(
                    "Gopher64 calibrated launch needs a native or Flatpak build, not Wine"
                )
            }
        };
        let content = super::guided::content_argument(original, option)?;
        ensure!(
            content == setup.content,
            "Gopher64 calibrated launch currently requires exactly the saved game argument"
        );
        let inputs = session::PreparedSession::prepare(
            setup,
            calibrations,
            inventory,
            flatpak_app_id.as_deref(),
            cancel,
        )?;
        let mut plan = original.clone();
        if let Some(app_id) = &flatpak_app_id {
            // The sandbox sees neither /tmp sessions nor the ROM library by
            // default: bind the live config home plus the ROM directory
            // read-only, and repeat the probe's SDL backend hint. XDG paths
            // inside resolve identically because mounts keep host paths.
            let position = plan
                .arguments
                .iter()
                .position(|arg| arg == app_id.as_str())
                .context("Missing Gopher64 Flatpak app argument")?;
            let rom_parent = setup
                .content
                .parent()
                .context("Gopher64 game argument has no parent directory")?;
            ensure!(
                rom_parent != Path::new("/"),
                "Gopher64 Flatpak launch refuses to expose the filesystem root"
            );
            let mut grants: Vec<std::ffi::OsString> = vec![
                std::ffi::OsString::from(format!("--filesystem={}", inputs.config_home.display())),
                std::ffi::OsString::from("--env=SDL_JOYSTICK_LINUX_CLASSIC=1"),
            ];
            // The planner usually mounts the ROM root already; only add a
            // read-only grant when it is missing.
            let rom_grant = format!("--filesystem={}:ro", rom_parent.display());
            let covered = plan.arguments.iter().any(|arg| {
                let text = arg.to_string_lossy();
                text == format!("--filesystem={}", rom_parent.display()) || text == rom_grant
            });
            if !covered {
                grants.push(std::ffi::OsString::from(rom_grant));
            }
            plan.arguments.splice(position..position, grants);
        } else {
            plan.environment.push((
                "XDG_CONFIG_HOME".into(),
                inputs.config_home.as_os_str().to_owned(),
            ));
            plan.environment
                .push(("SDL_JOYSTICK_LINUX_CLASSIC".into(), "1".into()));
        }
        let session = NativeSession {
            inputs,
            executable,
            flatpak_app_id,
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

    fn empty_secondary() -> BTreeMap<String, MappedInput> {
        BTreeMap::new()
    }

    fn complete_profile() -> BTreeMap<String, MappedInput> {
        CONTROLS
            .iter()
            .enumerate()
            .map(|(n, (control, _))| {
                let input = match *control {
                    "stick_left" => MappedInput::Axis {
                        index: 0,
                        positive: false,
                    },
                    "stick_right" => MappedInput::Axis {
                        index: 0,
                        positive: true,
                    },
                    "stick_up" => MappedInput::Axis {
                        index: 1,
                        positive: false,
                    },
                    "stick_down" => MappedInput::Axis {
                        index: 1,
                        positive: true,
                    },
                    _ => MappedInput::Button(n as u8 + 2),
                };
                ((*control).to_owned(), input)
            })
            .collect()
    }

    #[test]
    fn renders_pinned_profile_order_and_axis_schema() {
        let profile = profile_json(&complete_profile(), &empty_secondary()).unwrap();
        serde_json::from_value::<Gopher64InputProfile>(profile.clone()).unwrap();
        let inputs = profile["inputs"].as_array().unwrap();
        assert_eq!(inputs.len(), 19);
        assert_eq!(inputs[7][0]["ControllerButton"]["id"], 9);
        assert_eq!(inputs[15][0]["ControllerAxis"]["id"], 0);
        assert_eq!(inputs[15][0]["ControllerAxis"]["axis"], -1);
        assert!(inputs[18][0].is_null());
        assert_eq!(profile["dinput"], false);
        assert_eq!(profile["deadzone"], 5);
    }

    #[test]
    fn twin_trigger_occupies_the_profile_second_slot() {
        let mut secondary = BTreeMap::new();
        secondary.insert("z".into(), MappedInput::Button(9));
        let profile = profile_json(&complete_profile(), &secondary).unwrap();
        let inputs = profile["inputs"].as_array().unwrap();
        assert_eq!(inputs[5][0]["ControllerButton"]["id"], 7);
        assert_eq!(inputs[5][1]["ControllerButton"]["id"], 9);
        assert!(inputs[18][0].is_null() && inputs[18][1].is_null());
        serde_json::from_value::<Gopher64InputProfile>(profile).unwrap();

        let mut orphan = BTreeMap::new();
        orphan.insert("not_n64".into(), MappedInput::Button(0));
        assert!(profile_json(&complete_profile(), &orphan).is_err());
    }

    #[test]
    fn discover_extracts_exact_rom_arguments_without_guessing() {
        use crate::emulator::{EmulatorExecutable, LaunchPlan, RomEmulatorOption};
        fn launch_plan(arguments: Vec<&str>) -> LaunchPlan {
            LaunchPlan {
                emulator_name: "Gopher64".into(),
                program: "/usr/bin/flatpak".into(),
                arguments: arguments.into_iter().map(Into::into).collect(),
                current_directory: "/tmp".into(),
                environment: Vec::new(),
                cleanup_paths: Vec::new(),
                retroarch_content: None,
            }
        }
        fn option() -> RomEmulatorOption {
            RomEmulatorOption::standalone(
                "gopher64".into(),
                "Gopher64".into(),
                EmulatorExecutable::Flatpak {
                    command: "/usr/bin/flatpak".into(),
                    app_id: "io.github.gopher64.gopher64".into(),
                },
            )
        }
        let rom = std::path::PathBuf::from("/games/mario.n64");
        let plan = launch_plan(vec![
            "run",
            "--filesystem=/home/u/.var/app/x",
            "--env=A=1",
            "io.github.gopher64.gopher64",
            rom.to_str().unwrap(),
        ]);
        // Path must exist for discovery; missing files fail closed.
        assert!(guided::content_argument(&plan, &option()).is_err());
        let _ = rom;
        let existing = std::env::current_exe().unwrap();
        let plan = launch_plan(vec![
            "run",
            "io.github.gopher64.gopher64",
            existing.to_str().unwrap(),
        ]);
        assert_eq!(
            guided::content_argument(&plan, &option()).unwrap(),
            existing
        );
        // Two game files is ambiguous, never a guess.
        let plan = launch_plan(vec![
            "run",
            "io.github.gopher64.gopher64",
            existing.to_str().unwrap(),
            existing.to_str().unwrap(),
        ]);
        assert!(guided::content_argument(&plan, &option()).is_err());
    }

    #[test]
    fn private_patch_preserves_unowned_config_and_transfer_pak() {
        let mut custom = profile_json(&complete_profile(), &empty_secondary()).unwrap();
        custom["future"] = serde_json::json!(1);
        let baseline = serde_json::to_vec(&serde_json::json!({
          "input":{"input_profiles":{"custom":custom},
            "input_profile_binding":["custom","custom","custom","custom"],
            "controller_assignment":["old",null,null,null],
            "controller_enabled":[true,false,false,false],
            "transfer_pak":[true,false,false,false],"gb_rom_path":["gb", "", "", ""],
            "gb_ram_path":["ram", "", "", ""],"emulate_vru":true,"future_input":7},
          "video":{"future_video":8},"future_root":9
        }))
        .unwrap();
        let players = vec![(
            1,
            "/dev/input/js0".into(),
            profile_json(&complete_profile(), &empty_secondary()).unwrap(),
        )];
        let value: serde_json::Value =
            serde_json::from_slice(&patched_config(&baseline, &players).unwrap()).unwrap();
        assert_eq!(value["future_root"], 9);
        assert_eq!(value["video"]["future_video"], 8);
        assert_eq!(value["input"]["future_input"], 7);
        assert_eq!(value["input"]["transfer_pak"][0], true);
        assert_eq!(value["input"]["gb_rom_path"][0], "gb");
        assert_eq!(value["input"]["controller_assignment"][0], "/dev/input/js0");
        assert_eq!(
            value["input"]["input_profile_binding"][0],
            "Lunchbox Player 1"
        );
        assert_eq!(value["input"]["input_profiles"]["custom"]["future"], 1);
    }

    #[test]
    fn rejects_duplicate_or_mismatched_stick_outputs() {
        let mut duplicate = complete_profile();
        duplicate.insert("b".into(), duplicate["a"]);
        assert!(profile_json(&duplicate, &empty_secondary()).is_err());
        let mut mismatched = complete_profile();
        mismatched.insert(
            "stick_right".into(),
            MappedInput::Axis {
                index: 2,
                positive: true,
            },
        );
        assert!(profile_json(&mismatched, &empty_secondary()).is_err());
    }

    #[test]
    fn rejects_unknown_controls_and_invalid_sdl_indices() {
        let mut unknown = complete_profile();
        unknown.remove("a");
        unknown.insert("not_n64".into(), MappedInput::Button(0));
        assert!(profile_json(&unknown, &empty_secondary()).is_err());

        let mut button = complete_profile();
        button.insert("a".into(), MappedInput::Button(26));
        assert!(profile_json(&button, &empty_secondary()).is_err());

        let mut axis = complete_profile();
        axis.insert(
            "stick_left".into(),
            MappedInput::Axis {
                index: 6,
                positive: false,
            },
        );
        assert!(profile_json(&axis, &empty_secondary()).is_err());
    }

    #[test]
    fn rejects_malformed_preserved_input_fields() {
        let result = patched_config(br#"{"input":{"transfer_pak":"not-an-array"}}"#, &[]);
        assert!(result.is_err());
    }

    #[test]
    fn setup_validation_rejects_traversal_and_duplicate_players() {
        let dir = tempfile::tempdir().unwrap();
        let root = dir.path().to_path_buf();
        let _guard = dir;
        let base = settings::SavedSetup {
            emulator_id: "gopher64".into(),
            content: root.join("mario.n64"),
            config_path: root.join("config/gopher64/config.json"),
            probe_program: root.join("opt/lunchbox-probe"),
            sdl_library: root.join("usr/lib/libSDL3.so"),
            executable_sha256: "a".repeat(64),
            players: vec![settings::Player {
                player: 1,
                controller_id: "pad-1".into(),
            }],
        };
        assert!(base.validate().is_ok());

        let mut traversal = base.clone();
        traversal.content = root.join("../secret.n64");
        assert!(traversal.validate().is_err());

        let mut duplicate = base;
        duplicate.players.push(settings::Player {
            player: 1,
            controller_id: "pad-2".into(),
        });
        assert!(duplicate.validate().is_err());
    }
}
