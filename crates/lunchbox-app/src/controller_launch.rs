//! Launch-scoped controller adapters. Never write a user's emulator config.
use std::collections::BTreeMap;
use std::ffi::OsString;
use std::path::Path;

use crate::controller_catalog::{Calibration, EmulatorProfile, NativeInput, catalog};
use crate::controllers::ControllerDevice;
use crate::emulator::{EmulatorExecutable, EmulatorRuntimeKind, LaunchPlan, RomEmulatorOption};
use crate::settings::AppSettings;
use anyhow::{Context, Result, bail, ensure};

#[cfg(all(test, target_os = "linux"))]
#[path = "controller_launch_oracle.rs"]
mod oracle;

pub struct CalibratedLaunch {
    // Keeps the private append config alive until the child exits, including errors.
    _directory: tempfile::TempDir,
    pub description: String,
}

const OUTPUTS: &[(&str, &str)] = &[
    ("South", "b"),
    ("East", "a"),
    ("West", "y"),
    ("North", "x"),
    ("Select", "select"),
    ("Start", "start"),
    ("DPadUp", "up"),
    ("DPadDown", "down"),
    ("DPadLeft", "left"),
    ("DPadRight", "right"),
    ("LeftBumper", "l"),
    ("RightBumper", "r"),
    ("LeftTrigger", "l2"),
    ("RightTrigger", "r2"),
    ("LeftStick", "l3"),
    ("RightStick", "r3"),
    ("LeftStickLeft", "l_x_minus"),
    ("LeftStickRight", "l_x_plus"),
    ("LeftStickUp", "l_y_minus"),
    ("LeftStickDown", "l_y_plus"),
    ("RightStickLeft", "r_x_minus"),
    ("RightStickRight", "r_x_plus"),
    ("RightStickUp", "r_y_minus"),
    ("RightStickDown", "r_y_plus"),
];

/// Select by the actual launched core and platform, never emulator display name.
pub fn contract(core: &str, platform: &str) -> Option<&'static EmulatorProfile> {
    catalog().launch_profile(core, platform)
}

pub fn supports_profile(profile: &EmulatorProfile) -> bool {
    (cfg!(target_os = "linux") && profile.retroarch_launch.is_some())
        || profile.transport == "ares-settings"
}

fn cfg_value(text: &str, key: &str) -> Result<Option<String>> {
    let mut result = None;
    for line in text.lines() {
        let Some((name, value)) = line.trim().split_once('=') else {
            continue;
        };
        if name.trim() != key {
            continue;
        }
        ensure!(
            name.ends_with(|c: char| c.is_ascii_whitespace()),
            "RetroArch requires whitespace before '=' in {key}"
        );
        ensure!(result.is_none(), "Duplicate {key} setting");
        let value = value.trim();
        let (value, rest) = if let Some(quoted) = value.strip_prefix('"') {
            quoted
                .split_once('"')
                .with_context(|| format!("Unterminated {key} setting"))?
        } else {
            let end = value
                .find(|c: char| c.is_ascii_whitespace() || c == '#')
                .unwrap_or(value.len());
            ensure!(end > 0, "Missing {key} value");
            (&value[..end], &value[end..])
        };
        ensure!(
            rest.trim().is_empty() || rest.trim_start().starts_with('#'),
            "Unresolved {key} setting suffix"
        );
        result = Some(value.to_owned());
    }
    Ok(result)
}

fn core_options_overlay(baseline: &str, options: &BTreeMap<String, String>) -> Result<String> {
    ensure!(
        !baseline
            .lines()
            .any(|line| line.trim_start().starts_with("#include")),
        "Included core-options files require an adapter upgrade before calibrated launch"
    );
    let mut output = baseline
        .lines()
        .filter(|line| {
            !line
                .split_once('=')
                .is_some_and(|(key, _)| options.contains_key(key.trim()))
        })
        .map(|line| format!("{line}\n"))
        .collect::<String>();
    for (key, value) in options {
        ensure!(
            key.bytes()
                .all(|b| b.is_ascii_alphanumeric() || b == b'-' || b == b'_')
                && !value.contains(['"', '\n', '\r', '\\']),
            "Invalid core option"
        );
        output.push_str(&format!("{key} = \"{value}\"\n"));
    }
    Ok(output)
}

fn read_optional(path: &Path) -> Result<String> {
    match std::fs::read_to_string(path) {
        Ok(value) => Ok(value),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(String::new()),
        Err(error) => Err(error).with_context(|| format!("Reading {}", path.display())),
    }
}

fn retroarch_base(executable: &EmulatorExecutable) -> Result<(std::path::PathBuf, String)> {
    let dirs = directories::BaseDirs::new().context("Finding RetroArch config directory")?;
    let base = match executable {
        EmulatorExecutable::Flatpak { app_id, .. } => dirs
            .home_dir()
            .join(".var/app")
            .join(app_id)
            .join("config/retroarch"),
        _ => dirs.config_dir().join("retroarch"),
    };
    let path = base.join("retroarch.cfg");
    ensure!(
        path.exists()
            || matches!(executable, EmulatorExecutable::Flatpak { .. })
            || !dirs.home_dir().join(".retroarch.cfg").exists(),
        "Legacy ~/.retroarch.cfg needs explicit configuration resolution for calibrated launch"
    );
    Ok((base, read_optional(&path)?))
}

fn configured_path(value: &str) -> Result<std::path::PathBuf> {
    let path = if let Some(relative) = value.strip_prefix("~/") {
        directories::BaseDirs::new()
            .context("Finding RetroArch home directory")?
            .home_dir()
            .join(relative)
    } else {
        std::path::PathBuf::from(value)
    };
    ensure!(
        path.is_absolute(),
        "Custom relative RetroArch paths need effective configuration resolution"
    );
    Ok(path)
}

fn emulator_arguments<'a>(
    plan: &'a LaunchPlan,
    executable: &EmulatorExecutable,
) -> Result<&'a [OsString]> {
    match executable {
        EmulatorExecutable::Flatpak { app_id, .. } => {
            let boundary = plan
                .arguments
                .iter()
                .position(|arg| arg.to_str() == Some(app_id))
                .context("Missing Flatpak app boundary")?;
            Ok(&plan.arguments[boundary + 1..])
        }
        _ => Ok(&plan.arguments),
    }
}

fn write_core_options(
    profile: &EmulatorProfile,
    plan: &LaunchPlan,
    executable: &EmulatorExecutable,
    directory: &Path,
) -> Result<String> {
    if profile.core_options.is_empty() {
        return Ok(String::new());
    }
    ensure!(
        !plan.arguments.iter().any(|arg| arg == "--config"
            || arg == "-c"
            || arg.to_string_lossy().starts_with("--config=")
            || arg.to_string_lossy().starts_with("--appendconfig")),
        "Custom RetroArch configuration arguments need core-options resolution before calibrated launch with core options"
    );
    let (base, config) = retroarch_base(executable)?;
    let baseline = read_core_options(&base, &config)?;
    write_core_options_snapshot(profile, &baseline, directory)
}

fn read_core_options(base: &Path, config: &str) -> Result<String> {
    ensure!(
        !config
            .lines()
            .any(|line| line.trim_start().starts_with("#include")),
        "Included RetroArch configs require core-options resolution before calibrated launch with core options"
    );
    let path = match cfg_value(config, "core_options_path")?.filter(|v| !v.is_empty()) {
        Some(path) => configured_path(&path)?,
        None => base.join("retroarch-core-options.cfg"),
    };
    read_optional(&path)
}

fn config_bool(config: &str, key: &str, default: bool) -> Result<bool> {
    let mut result = None;
    for line in config.lines().map(str::trim) {
        let Some((name, value)) = line.split_once('=') else {
            continue;
        };
        if name.trim() != key {
            continue;
        }
        ensure!(
            name.ends_with(|c: char| c.is_ascii_whitespace()),
            "RetroArch requires whitespace before '=' in {key}"
        );
        ensure!(result.is_none(), "Duplicate {key} setting");
        let value = value.trim();
        let value = if let Some(quoted) = value.strip_prefix('"') {
            let (value, rest) = quoted
                .split_once('"')
                .context("Unterminated boolean setting")?;
            ensure!(
                rest.trim().is_empty() || rest.trim_start().starts_with('#'),
                "Invalid boolean setting suffix"
            );
            value
        } else {
            value.split('#').next().unwrap().trim()
        };
        result = Some(match value {
            "true" => true,
            "false" => false,
            _ => bail!("Unresolved {key} boolean setting"),
        });
    }
    Ok(result.unwrap_or(default))
}

fn effective_mode_core_options(
    base: &Path,
    config: &str,
    config_directory: &Path,
    library: &str,
    content: &Path,
) -> Result<String> {
    // RetroArch 1.22.2 runloop_init_core_options_path selects ONE file, not a
    // merge: game -> folder -> per-core -> global. Its defaults enable game
    // options and per-core storage (global_core_options = false).
    let directory = config_directory.join(library);
    let mut candidates = Vec::new();
    if config_bool(config, "game_specific_options", true)? {
        let game = content
            .file_stem()
            .context("Prepared content has no game basename")?;
        let folder = content
            .parent()
            .and_then(Path::file_name)
            .context("Prepared content has no folder basename")?;
        for name in [game, folder] {
            let mut filename = name.to_os_string();
            filename.push(".opt");
            candidates.push(directory.join(filename));
        }
    }
    if !config_bool(config, "global_core_options", false)? {
        candidates.push(directory.join(format!("{library}.opt")));
    }
    for path in candidates {
        if path.exists() {
            return read_optional(&path);
        }
    }
    read_core_options(base, config)
}

fn write_core_options_snapshot(
    profile: &EmulatorProfile,
    baseline: &str,
    directory: &Path,
) -> Result<String> {
    if profile.core_options.is_empty() {
        return Ok(String::new());
    }
    let options = core_options_overlay(baseline, &profile.core_options)?;
    let output = directory.join("core-options.cfg");
    let output_text = output.to_str().context("Core-options path must be UTF-8")?;
    ensure!(
        !output_text.contains(['"', '\n', '\r', '\\']),
        "Core-options path cannot be encoded"
    );
    std::fs::write(&output, options)?;
    // RetroArch may save a per-core .opt under this directory even with
    // game_specific_options disabled. Keep that exit-time write private too.
    let private_config = directory.join("config");
    std::fs::create_dir_all(&private_config)?;
    let private_config = private_config
        .to_str()
        .context("Private config path must be UTF-8")?;
    Ok(format!(
        "game_specific_options = \"false\"\nglobal_core_options = \"false\"\ncore_options_path = \"{output_text}\"\nrgui_config_directory = \"{private_config}\"\n"
    ))
}

/// Read-only runtime numbering, independent of the labels printed on a pad.
#[derive(Debug, PartialEq, Eq)]
pub struct JoydevMap {
    pub index: usize,
    pub buttons: Vec<u16>,
    pub axes: Vec<u8>,
}

#[cfg(target_os = "linux")]
fn measured_event_path<'a>(
    joystick: &Path,
    events: &'a [std::path::PathBuf],
    sys_input: &Path,
) -> Result<(&'a Path, std::path::PathBuf)> {
    let identity = |path: &Path, prefix: &str| -> Result<std::path::PathBuf> {
        ensure!(
            path.parent() == Some(Path::new("/dev/input")),
            "Expected a physical input node"
        );
        let name = path
            .file_name()
            .and_then(|name| name.to_str())
            .context("Input node name is not UTF-8")?;
        ensure!(
            name.strip_prefix(prefix)
                .is_some_and(|n| !n.is_empty() && n.bytes().all(|b| b.is_ascii_digit())),
            "Unexpected input node name"
        );
        Ok(sys_input.join(name).join("device").canonicalize()?)
    };
    let expected = identity(joystick, "js")?;
    let mut selected = None;
    for event in events {
        if identity(event, "event")? == expected {
            ensure!(
                selected.is_none(),
                "Ambiguous physical event node for calibrated controller"
            );
            selected = Some(event.as_path());
        }
    }
    Ok((
        selected
            .context("No exact evdev node for the selected joystick; reconnect the controller")?,
        expected,
    ))
}

/// The same physical input mode that produced the gesture must still be present
/// when its per-launch mapping is written. Older button-number calibrations stay
/// readable; missing axis measurements are never fabricated for them.
fn validated_numbering(
    calibration: &Calibration,
    profile: &EmulatorProfile,
    device: &ControllerDevice,
) -> Result<JoydevMap> {
    let plan = calibration.plan(&profile.id)?;
    let numbering = JoydevMap::read(&device.device_path)?;
    let measurements = plan
        .rows
        .iter()
        .filter_map(|row| {
            let input = row.input.as_ref()?;
            Some((input.native.as_ref()?.code, input.axis.as_ref()?))
        })
        .collect::<Vec<_>>();
    if measurements.is_empty() {
        return Ok(numbering);
    }
    #[cfg(target_os = "linux")]
    {
        let sys_input = Path::new("/sys/class/input");
        let (event, identity) =
            measured_event_path(&device.device_path, &device.event_paths, sys_input)?;
        crate::controller_axis::validate_recorded_axes(event, measurements)?;
        ensure!(
            JoydevMap::read(&device.device_path)? == numbering
                && measured_event_path(&device.device_path, &device.event_paths, sys_input)?.1
                    == identity,
            "Controller changed while preparing its mapping; try launching again"
        );
        Ok(numbering)
    }
    #[cfg(not(target_os = "linux"))]
    bail!("Physical axis validation requires the recorded Linux input backend")
}

#[cfg(target_os = "linux")]
pub fn physical_button_present(path: &Path, code: u16) -> Result<bool> {
    use std::os::fd::AsRawFd;
    ensure!(code < 768, "Unsupported evdev key code");
    let file = std::fs::File::open(path)?;
    let mut keys = [0u8; 96];
    // EVIOCGBIT(EV_KEY, KEY_CNT / 8): read-only key capability query.
    let result = unsafe {
        libc::ioctl(
            file.as_raw_fd(),
            0x80604521 as libc::c_ulong,
            keys.as_mut_ptr(),
        )
    };
    ensure!(result >= 0, "Cannot verify physical D-pad capabilities");
    Ok(keys[usize::from(code) / 8] & (1 << (code % 8)) != 0)
}

pub fn numbering_probe() -> Result<()> {
    let mut warnings = Vec::new();
    for device in crate::controllers::list_local_controllers(&mut warnings) {
        let numbering = JoydevMap::read(&device.device_path)?;
        #[cfg(target_os = "linux")]
        let physical_axes = {
            let (event, _) = measured_event_path(
                &device.device_path,
                &device.event_paths,
                Path::new("/sys/class/input"),
            )?;
            numbering
                .axes
                .iter()
                .map(|code| crate::controller_axis::probe(event, *code))
                .collect::<Result<Vec<_>>>()?
        };
        #[cfg(not(target_os = "linux"))]
        let physical_axes: Option<Vec<serde_json::Value>> = None;
        println!(
            "{}",
            serde_json::json!({ "device": device.device_path, "name": device.name,
            "index": numbering.index, "buttons": numbering.buttons, "axes": numbering.axes,
            "physical_axes": physical_axes })
        );
    }
    Ok(())
}

impl JoydevMap {
    pub fn binding(&self, input: &NativeInput) -> Result<(&'static str, String)> {
        let code = (input.code & 0xffff) as u16;
        match input.code >> 16 {
            1 if input.direction == 0 => {
                let index = self
                    .buttons
                    .iter()
                    .position(|value| *value == code)
                    .context(
                        "Recorded button is not present in this device's current mode; recalibrate",
                    )?;
                ensure!(
                    index < 32,
                    "RetroArch linuxraw supports only the first 32 buttons"
                );
                Ok(("btn", index.to_string()))
            }
            3 if matches!(input.direction, -1 | 1) => {
                let index = self
                    .axes
                    .iter()
                    .position(|value| u16::from(*value) == code)
                    .context(
                        "Recorded axis is not present in this device's current mode; recalibrate",
                    )?;
                ensure!(
                    index < 32,
                    "RetroArch linuxraw supports only the first 32 axes"
                );
                Ok((
                    "axis",
                    format!("{}{index}", if input.direction < 0 { "-" } else { "+" }),
                ))
            }
            _ => bail!("Unsupported physical input encoding"),
        }
    }

    #[cfg(target_os = "linux")]
    pub fn read(path: &Path) -> Result<Self> {
        use std::os::fd::AsRawFd;
        ensure!(
            path.parent() == Some(Path::new("/dev/input")),
            "Expected a native joystick device"
        );
        let index = path
            .file_name()
            .and_then(|s| s.to_str())
            .and_then(|s| s.strip_prefix("js"))
            .context("Expected a joystick device")?
            .parse::<usize>()?;
        ensure!(
            index < 16,
            "Joystick index is outside RetroArch's supported player range"
        );
        let file = std::fs::File::open(path).context("Opening calibrated controller")?;
        let mut buttons = [0u16; 512];
        let mut axes = [0u8; 64];
        let mut button_count = 0u8;
        let mut axis_count = 0u8;
        // Linux joystick.h read-only ABI; the request sizes exactly match the
        // writable stack arrays. The FD remains owned and live for every call.
        for (request, destination) in [
            (
                0x80016a12u64,
                (&mut button_count as *mut u8).cast::<libc::c_void>(),
            ),
            (0x80016a11, (&mut axis_count as *mut u8).cast()),
            (0x84006a34, buttons.as_mut_ptr().cast()),
            (0x80406a32, axes.as_mut_ptr().cast()),
        ] {
            let result =
                unsafe { libc::ioctl(file.as_raw_fd(), request as libc::c_ulong, destination) };
            ensure!(
                result >= 0,
                "Reading controller numbering: {}",
                std::io::Error::last_os_error()
            );
        }
        ensure!(
            usize::from(axis_count) <= axes.len(),
            "Invalid joystick axis count"
        );
        Ok(Self {
            index,
            buttons: buttons[..usize::from(button_count)].to_vec(),
            axes: axes[..usize::from(axis_count)].to_vec(),
        })
    }

    #[cfg(not(target_os = "linux"))]
    pub fn read(_path: &Path) -> Result<Self> {
        bail!("Native controller numbering adapter is not available on this OS yet")
    }
}

fn player_config(
    calibration: &Calibration,
    profile: &EmulatorProfile,
    device: &JoydevMap,
    player: usize,
) -> Result<String> {
    let requested = profile
        .retroarch_launch
        .as_ref()
        .context("Preview-only controller contract")?
        .device;
    player_config_requested(calibration, profile, device, player, requested)
}

fn player_config_requested(
    calibration: &Calibration,
    profile: &EmulatorProfile,
    device: &JoydevMap,
    player: usize,
    requested_mode: u32,
) -> Result<String> {
    calibration.validate()?;
    ensure!(
        calibration.os == std::env::consts::OS && calibration.os == "linux",
        "Recalibrate on this host OS"
    );
    let launch = profile
        .retroarch_launch
        .as_ref()
        .context("Preview-only controller contract")?;
    ensure!(
        requested_mode == launch.device
            || (matches!(profile.core.as_str(), "mednafen_psx" | "mednafen_psx_hw")
                && matches!(requested_mode, 1 | 517)),
        "Requested controller mode disagrees with the binding contract"
    );
    ensure!(
        (1..=launch.max_players).contains(&player),
        "Player number exceeds this input mode's port count"
    );
    let plan = calibration.plan(&profile.id)?;
    let target = catalog().layout(&profile.target_layout).unwrap();
    let mut values = BTreeMap::new();
    // Clear BOTH sides: an inherited axis must not survive a new button bind.
    // Keyboard bindings remain available. Unused auto-config inputs are disabled.
    for (_, output) in OUTPUTS {
        for suffix in ["btn", "axis"] {
            values.insert(
                format!("input_player{player}_{output}_{suffix}"),
                "nul".to_string(),
            );
        }
    }
    for row in plan.rows {
        let Some(input) = row.input else {
            let optional = target
                .controls
                .iter()
                .find(|c| c.id == row.target_id)
                .is_some_and(|c| c.optional);
            ensure!(
                optional,
                "{} is not mapped for {}. Complete calibration or select a compatible controller.",
                row.target,
                profile.name
            );
            continue;
        };
        let native = input.native.context("This calibration predates physical-input capture. Calibrate this controller again once")?;
        let (suffix, value) = device.binding(&native)?;
        let (_, output) = OUTPUTS
            .iter()
            .find(|(logical, _)| *logical == row.output)
            .context("Unknown RetroPad output")?;
        values.insert(format!("input_player{player}_{output}_{suffix}"), value);
    }
    values.insert(
        format!("input_player{player}_joypad_index"),
        device.index.to_string(),
    );
    values.insert(format!("input_player{player}_analog_dpad_mode"), "0".into());
    values.insert(
        format!("input_libretro_device_p{player}"),
        requested_mode.to_string(),
    );
    Ok(values
        .into_iter()
        .map(|(key, value)| format!("{key} = \"{value}\"\n"))
        .collect())
}

fn append_argument(arguments: &mut Vec<OsString>, path: &Path) -> Result<()> {
    let path = path
        .to_str()
        .context("RetroArch config path is not UTF-8")?;
    ensure!(
        !path.contains('|'),
        "RetroArch config path contains its list separator"
    );
    // A custom append list must be preserved; RetroArch accepts a pipe-separated list.
    if let Some(index) = arguments.iter().position(|arg| arg == "--appendconfig") {
        let previous = arguments
            .get(index + 1)
            .and_then(|s| s.to_str())
            .context("Invalid --appendconfig")?;
        arguments[index + 1] = format!("{previous}|{path}").into();
    } else if let Some(index) = arguments
        .iter()
        .position(|arg| arg.to_string_lossy().starts_with("--appendconfig="))
    {
        let previous = arguments[index]
            .to_str()
            .context("Invalid --appendconfig")?;
        arguments[index] = format!("{previous}|{path}").into();
    } else {
        // Inserting at the front also works with a user's final `--` separator.
        arguments.splice(
            0..0,
            [OsString::from("--appendconfig"), OsString::from(path)],
        );
    }
    Ok(())
}

fn selected_devices<'a>(
    settings: &AppSettings,
    devices: &'a [ControllerDevice],
    platform: &str,
) -> Vec<&'a ControllerDevice> {
    let mapping = &settings.controller_mapping;
    if mapping.explicit_player_selection {
        // Preserve the chosen order exactly. Missing players are rejected by the
        // launch guard below, rather than silently promoting P2 to P1.
        return mapping
            .player_mappings
            .iter()
            .filter_map(|player| {
                devices
                    .iter()
                    .find(|device| player.controller_id.as_deref() == Some(&device.stable_id))
            })
            .collect();
    }
    let system = crate::controllers::system_layout(platform);
    let preferred = mapping
        .preferred_devices
        .get(crate::controllers::system_layout(platform));
    let mut devices = devices
        .iter()
        .filter(|device| {
            mapping.calibrations.contains_key(&device.stable_id)
                && !mapping.hidden_controller_ids.contains(&device.stable_id)
        })
        .collect::<Vec<_>>();
    devices.sort_by_key(|device| {
        let explicit = mapping
            .player_mappings
            .iter()
            .position(|p| p.controller_id.as_deref() == Some(&device.stable_id));
        let family = catalog()
            .layout(&mapping.calibrations[&device.stable_id].layout)
            .map(|layout| layout.family.as_str())
            .unwrap_or("");
        let fit = match (system, family) {
            ("two-button", "two-button" | "horizontal-four")
            | ("n64", "n64")
            | ("six-button", "six-button" | "three-button")
            | ("modern", "diamond") => 0,
            ("six-button", "n64") => 1,
            _ => 2,
        };
        (
            explicit.is_none(),
            explicit.unwrap_or(usize::MAX),
            preferred != Some(&device.stable_id),
            fit,
        )
    });
    devices
}

fn compatible(calibration: &Calibration, profile: &EmulatorProfile) -> bool {
    calibration
        .plan(&profile.id)
        .is_ok_and(|plan| plan.automatic_launch_ready)
}

fn restrict_players(
    devices: &mut Vec<&ControllerDevice>,
    profile: &EmulatorProfile,
) -> Result<usize> {
    let eligible_count = devices.len();
    let launch = profile
        .retroarch_launch
        .as_ref()
        .context("Preview-only controller contract")?;
    // Preference ordering chooses active players without creating nonexistent ports.
    devices.truncate(launch.max_players);
    Ok(eligible_count)
}

/// Returns None only when no saved calibrated launch was requested. Failures are
/// surfaced at the launch boundary instead of reporting a miswired game as ready.
pub fn prepare(
    settings: &AppSettings,
    platform: &str,
    option: &RomEmulatorOption,
    plan: &mut LaunchPlan,
) -> Result<Option<CalibratedLaunch>> {
    let mapping = &settings.controller_mapping;
    if !mapping.calibrated_launch || mapping.calibrations.is_empty() {
        return Ok(None);
    }
    let mut warnings = Vec::new();
    let inventory = crate::controllers::list_local_controllers(&mut warnings);
    let mut devices = selected_devices(settings, &inventory, platform);
    if settings.controller_mapping.explicit_player_selection {
        ensure!(
            devices.len() == settings.controller_mapping.player_mappings.len(),
            "A selected player's controller is disconnected. Reconnect it or change the players in Controller setup."
        );
        for (index, device) in devices.iter().enumerate() {
            ensure!(
                settings
                    .controller_mapping
                    .calibrations
                    .contains_key(&device.stable_id)
                    && !settings
                        .controller_mapping
                        .hidden_controller_ids
                        .contains(&device.stable_id),
                "Finish setting up Player {} in Controller setup before launching.",
                index + 1
            );
        }
    }
    if devices.is_empty() {
        return Ok(None);
    }
    if option.runtime_kind == EmulatorRuntimeKind::Standalone
        && option.emulator_name.eq_ignore_ascii_case("ares")
    {
        let directory = crate::controller_ares::prepare(
            settings,
            platform,
            option,
            plan,
            &devices,
            &std::sync::atomic::AtomicBool::new(false),
        )?;
        return Ok(Some(CalibratedLaunch {
            _directory: directory,
            description: format!(
                "ares: applied {} player mapping(s) to a private settings file",
                devices.len()
            ),
        }));
    }
    ensure!(
        option.runtime_kind == EmulatorRuntimeKind::RetroArch,
        "Calibrated launch adapter for {} is not implemented yet. Disable Apply saved calibrations to keep its native setup.",
        option.emulator_name
    );
    ensure!(
        matches!(
            &option.executable,
            EmulatorExecutable::Native(_) | EmulatorExecutable::Flatpak { .. }
        ),
        "Calibrated launch for Wine RetroArch is not implemented yet"
    );
    let profile = contract(&option.core_name, platform).context("No automatic controller contract for this core/platform yet; disable Apply saved calibrations to use the emulator's native setup")?;
    if catalog().launch_modes(&option.core_name, platform).len() > 1
        || profile
            .retroarch_launch
            .as_ref()
            .is_some_and(|launch| launch.player_topology.is_some())
    {
        return prepare_mode_aware(settings, platform, option, plan, &devices);
    }
    // A connected N30 must not prevent a calibrated Brawler64 from playing N64.
    // Only controllers with every required target capability enter the player list.
    devices.retain(|device| compatible(&mapping.calibrations[&device.stable_id], profile));
    ensure!(
        !devices.is_empty(),
        "No connected calibration has all required controls for {}. Complete calibration (older calibrations need physical-input capture), or disable Apply saved calibrations.",
        profile.name
    );
    let eligible_count = restrict_players(&mut devices, profile)?;
    let cache = directories::BaseDirs::new()
        .context("Finding controller launch cache")?
        .cache_dir()
        .join("lunchbox/controller-launch");
    std::fs::create_dir_all(&cache)?;
    let directory = tempfile::Builder::new()
        .prefix("session-")
        .tempdir_in(cache)?;
    let mut config = String::from(
        "# Lunchbox per-launch physical calibration. User config is never rewritten.\ninput_joypad_driver = \"linuxraw\"\ninput_autodetect_enable = \"false\"\nauto_remaps_enable = \"false\"\nauto_overrides_enable = \"false\"\nconfig_save_on_exit = \"false\"\nremap_save_on_exit = \"false\"\n",
    );
    for (index, device) in devices.iter().enumerate() {
        let calibration = &mapping.calibrations[&device.stable_id];
        let numbering = validated_numbering(calibration, profile, device)?;
        config.push_str(&player_config(calibration, profile, &numbering, index + 1)?);
    }
    // Do not leave later ports pointing at one of these same devices through an
    // inherited automatic configuration. Players are the calibrated selection.
    config.push_str(&format!("input_max_users = \"{}\"\n", devices.len()));
    let path = directory.path().join("controllers.cfg");
    config.push_str(&write_core_options(
        profile,
        plan,
        &option.executable,
        directory.path(),
    )?);
    std::fs::write(&path, config)?;
    attach_config(plan, &option.executable, &path)?;
    Ok(Some(CalibratedLaunch {
        _directory: directory,
        description: format!(
            "Applied {} of {} compatible calibrated controller(s) for {} · RetroArch automatic overrides/remaps suspended for this launch",
            devices.len(),
            eligible_count,
            profile.name
        ),
    }))
}

fn ensure_no_mode_overrides(directory: &Path, library: &str) -> Result<()> {
    if !directory.exists() {
        return Ok(());
    }
    // RetroArch opens symlinked configs normally (common with declarative
    // configuration). Follow them too; an unreadable/cyclic tree is unresolved,
    // never evidence that no device-changing override exists.
    for entry in walkdir::WalkDir::new(directory).follow_links(true) {
        let entry = entry?;
        ensure!(
            !entry.file_type().is_file()
                || !entry
                    .path()
                    .extension()
                    .is_some_and(|ext| ext == "rmp" || ext == "cfg"),
            "Saved {library} core/game overrides need effective controller-mode resolution before calibrated launch"
        );
    }
    Ok(())
}

/// Independent target modes per console port, selected from the emulator's
/// configuration, never inferred by downgrading an incompatible physical pad.
fn prepare_mode_aware(
    settings: &AppSettings,
    platform: &str,
    option: &RomEmulatorOption,
    plan: &mut LaunchPlan,
    devices: &[&ControllerDevice],
) -> Result<Option<CalibratedLaunch>> {
    let (base, config) = retroarch_base(&option.executable)?;
    ensure!(
        !plan
            .environment
            .iter()
            .any(|(key, _)| key == "XDG_CONFIG_HOME" || key == "HOME"),
        "Custom emulator environment needs effective controller-mode resolution"
    );
    let profiles = catalog().launch_modes(&option.core_name, platform);
    let first = profiles.first().context("No controller mode contract")?;
    let ports = first.retroarch_launch.as_ref().unwrap().max_players;
    ensure!(
        profiles.iter().all(
            |profile| profile.retroarch_launch.as_ref().unwrap().max_players == ports
                && profile.core_options == first.core_options
                && profile.retroarch_library == first.retroarch_library
                && profile.retroarch_launch.as_ref().unwrap().player_topology
                    == first.retroarch_launch.as_ref().unwrap().player_topology
        ),
        "Controller modes disagree on port topology, library name or core options"
    );
    let library = first
        .retroarch_library
        .as_ref()
        .context("Mode-aware core lacks its exact library name")?;
    let arguments = emulator_arguments(plan, &option.executable)?;
    ensure!(
        !arguments.iter().any(|arg| arg == "--config"
            || arg.to_string_lossy().starts_with("-c")
            || arg.to_string_lossy().starts_with("--config=")
            || arg.to_string_lossy().starts_with("--appendconfig")),
        "Custom RetroArch config arguments need effective controller-mode resolution"
    );
    // A saved core/game remap can select a device mode, not just button wiring.
    // Do not silently fall back to the base mode while those layers are unresolved.
    let remaps = cfg_value(&config, "input_remapping_directory")?
        .filter(|v| !v.is_empty())
        .map(|value| configured_path(&value))
        .transpose()?
        .unwrap_or_else(|| base.join("config/remaps"));
    let overrides = cfg_value(&config, "rgui_config_directory")?
        .filter(|v| !v.is_empty())
        .map(|value| configured_path(&value))
        .transpose()?
        .unwrap_or_else(|| base.join("config"));
    for directory in [remaps.join(library), overrides.join(library)] {
        ensure_no_mode_overrides(&directory, library)?;
    }
    let beetle_content = beetle_launch_content(
        &option.core_name,
        arguments,
        plan.retroarch_content.as_ref(),
    )?;
    let option_content = if first
        .retroarch_launch
        .as_ref()
        .unwrap()
        .player_topology
        .is_some()
    {
        let content = plan
            .retroarch_content
            .as_ref()
            .context("Option-dependent controller topology requires prepared content identity")?;
        crate::controller_launch_modes::validate_arguments(arguments, content)?;
        Some(content)
    } else {
        beetle_content
    };
    // Read once: the exact effective options determine both topology and the
    // snapshot delivered to the core; per-game settings are not flattened away.
    let baseline_options = if let Some(content) = option_content {
        effective_mode_core_options(&base, &config, &overrides, library, &content.content)?
    } else {
        read_core_options(&base, &config)?
    };
    let (active_ports, snapshot_profile) = topology_snapshot(first, &baseline_options)?;
    let modes = crate::controller_launch_modes::configured_modes(&config, arguments, active_ports)?;
    let binding_modes = if let Some(content) = beetle_content {
        crate::controller_psx::launch_binding_modes(
            content,
            &option.core_name,
            &modes,
            &baseline_options,
        )?
    } else {
        modes.clone()
    };
    let players = mode_players(
        settings,
        &option.core_name,
        platform,
        &binding_modes,
        devices,
    )?;
    let cache = directories::BaseDirs::new()
        .context("Finding controller launch cache")?
        .cache_dir()
        .join("lunchbox/controller-launch");
    std::fs::create_dir_all(&cache)?;
    let directory = tempfile::Builder::new()
        .prefix("session-")
        .tempdir_in(cache)?;
    let mut output = String::from(
        "# Lunchbox per-launch physical calibration; original configuration is unchanged.\ninput_joypad_driver = \"linuxraw\"\ninput_autodetect_enable = \"false\"\nauto_remaps_enable = \"false\"\nauto_overrides_enable = \"false\"\nconfig_save_on_exit = \"false\"\nremap_save_on_exit = \"false\"\n",
    );
    let highest_port = players.iter().map(|(port, _, _)| *port).max().unwrap();
    // Disabled gaps must not inherit a joystick already assigned to another port.
    for port in 1..=ports {
        if !players.iter().any(|(assigned, _, _)| *assigned == port) {
            for (_, control) in OUTPUTS {
                for suffix in ["btn", "axis"] {
                    output.push_str(&format!(
                        "input_player{port}_{control}_{suffix} = \"nul\"\n"
                    ));
                }
            }
            output.push_str(&format!("input_libretro_device_p{port} = \"0\"\n"));
        }
    }
    for (port, profile, device) in &players {
        let calibration = &settings.controller_mapping.calibrations[&device.stable_id];
        let numbering = validated_numbering(calibration, profile, device)?;
        output.push_str(&player_config_requested(
            calibration,
            profile,
            &numbering,
            *port,
            modes[*port - 1],
        )?);
    }
    output.push_str(&format!("input_max_users = \"{highest_port}\"\n"));
    output.push_str(&write_core_options_snapshot(
        &snapshot_profile,
        &baseline_options,
        directory.path(),
    )?);
    let path = directory.path().join("controllers.cfg");
    std::fs::write(&path, output)?;
    attach_config(plan, &option.executable, &path)?;
    Ok(Some(CalibratedLaunch {
        _directory: directory,
        description: format!(
            "Applied {} calibrated controller(s) using the selected per-port modes · RetroArch automatic overrides/remaps suspended for this launch",
            players.len()
        ),
    }))
}

/// Resolve a declared option-dependent port count without changing the selected
/// model. Freeze that same value into the private options snapshot, including
/// the declared default when the effective file omits it.
fn topology_snapshot(
    profile: &EmulatorProfile,
    baseline: &str,
) -> Result<(usize, EmulatorProfile)> {
    let launch = profile
        .retroarch_launch
        .as_ref()
        .context("Missing launch topology")?;
    let mut snapshot = profile.clone();
    let Some(topology) = &launch.player_topology else {
        return Ok((launch.max_players, snapshot));
    };
    ensure!(
        !baseline
            .lines()
            .any(|line| line.trim_start().starts_with("#include")),
        "Included core-options files require effective player-topology resolution"
    );
    let value = cfg_value(baseline, &topology.option)?.unwrap_or_else(|| topology.default.clone());
    let ports = *topology.values.get(&value).with_context(|| {
        format!(
            "Unverified controller topology for {} = {value}",
            topology.option
        )
    })?;
    snapshot.core_options.insert(topology.option.clone(), value);
    Ok((ports, snapshot))
}

/// Only Beetle needs disc identity to resolve compatibility-forced devices.
/// Other mode-aware cores retain their own argument and option contracts.
fn beetle_launch_content<'a>(
    core: &str,
    arguments: &[OsString],
    prepared: Option<&'a crate::emulator::PreparedRetroarchContent>,
) -> Result<Option<&'a crate::emulator::PreparedRetroarchContent>> {
    if !matches!(core, "mednafen_psx" | "mednafen_psx_hw") {
        return Ok(None);
    }
    let content = prepared.context("Missing prepared PlayStation content identity")?;
    crate::controller_psx::validate_arguments(arguments, content)?;
    Ok(Some(content))
}

fn mode_players<'a>(
    settings: &AppSettings,
    core: &str,
    platform: &str,
    modes: &[u32],
    devices: &[&'a ControllerDevice],
) -> Result<Vec<(usize, &'static EmulatorProfile, &'a ControllerDevice)>> {
    let mut targets = Vec::new();
    for (index, mode) in modes.iter().copied().enumerate() {
        if mode == 0 {
            continue;
        }
        let profile = catalog()
            .launch_mode(core, platform, mode)
            .with_context(|| {
                format!(
                    "No calibrated contract for {core} controller mode {mode} on port {}",
                    index + 1
                )
            })?;
        ensure!(
            index < profile.retroarch_launch.as_ref().unwrap().max_players,
            "Controller port exceeds the verified mode"
        );
        targets.push((index + 1, profile));
    }
    let candidates = targets
        .iter()
        .map(|(_, profile)| {
            devices
                .iter()
                .enumerate()
                .filter_map(|(index, device)| {
                    settings
                        .controller_mapping
                        .calibrations
                        .get(&device.stable_id)
                        .is_some_and(|calibration| compatible(calibration, profile))
                        .then_some(index)
                })
                .collect::<Vec<_>>()
        })
        .collect::<Vec<_>>();
    // Augmenting paths maximize filled ports without consuming a uniquely
    // capable pad on a port that another connected controller could serve.
    fn assign(
        port: usize,
        candidates: &[Vec<usize>],
        owners: &mut [Option<usize>],
        seen: &mut [bool],
    ) -> bool {
        // Preserve earlier ports' preferences whenever an unused compatible
        // controller is available. Reassign only to fill an otherwise empty
        // port (for example digital first, then DualShock-only second).
        for &device in &candidates[port] {
            if !seen[device] && owners[device].is_none() {
                owners[device] = Some(port);
                return true;
            }
        }
        for &device in &candidates[port] {
            if seen[device] {
                continue;
            }
            seen[device] = true;
            if owners[device].is_none_or(|other| assign(other, candidates, owners, seen)) {
                owners[device] = Some(port);
                return true;
            }
        }
        false
    }
    let mut owners = vec![None; devices.len()];
    for (index, (port, profile)) in targets.iter().enumerate() {
        if !assign(
            index,
            &candidates,
            &mut owners,
            &mut vec![false; devices.len()],
        ) && *port == 1
        {
            bail!(
                "No connected calibration supplies all controls for {}. Complete calibration or choose a compatible controller; the emulated mode was not changed",
                profile.name
            );
        }
    }
    let mut players = owners
        .iter()
        .enumerate()
        .filter_map(|(device, owner)| {
            owner.map(|target| (targets[target].0, targets[target].1, devices[device]))
        })
        .collect::<Vec<_>>();
    players.sort_by_key(|(port, _, _)| *port);
    ensure!(
        !players.is_empty(),
        "No enabled console port has a compatible calibrated controller"
    );
    Ok(players)
}

fn attach_config(
    plan: &mut LaunchPlan,
    executable: &EmulatorExecutable,
    path: &Path,
) -> Result<()> {
    match executable {
        EmulatorExecutable::Flatpak { app_id, .. } => {
            let boundary = plan
                .arguments
                .iter()
                .position(|arg| arg.to_str() == Some(app_id))
                .context("Missing Flatpak app boundary")?;
            let mut app_arguments = plan.arguments[boundary + 1..].to_vec();
            append_argument(&mut app_arguments, path)?;
            let parent = path.parent().context("Missing config directory")?;
            let mut access = OsString::from("--filesystem=");
            access.push(parent);
            // Permission applies only to this process and this private directory.
            plan.arguments.truncate(boundary + 1);
            plan.arguments.insert(boundary, access);
            plan.arguments.extend(app_arguments);
            Ok(())
        }
        EmulatorExecutable::Native(_) => append_argument(&mut plan.arguments, path),
        _ => bail!("Unsupported controller adapter transport"),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::controller_catalog::InputBinding;

    pub(super) fn calibrated_layout(layout: &str) -> (Calibration, JoydevMap) {
        let mut buttons = Vec::new();
        let mut bindings = BTreeMap::new();
        for control in &catalog().layout(layout).unwrap().controls {
            let native = if control.analog {
                let axis = match control.id.as_str() {
                    "stick_up" | "stick_down" => 1,
                    "stick_left" | "stick_right" => 0,
                    "right_stick_up" | "right_stick_down" => 3,
                    "right_stick_left" | "right_stick_right" => 2,
                    _ => panic!("Unknown test axis"),
                };
                NativeInput {
                    code: 0x30000 + axis,
                    direction: if control.id.ends_with("up") || control.id.ends_with("left") {
                        -1
                    } else {
                        1
                    },
                }
            } else {
                let code = 288 + buttons.len() as u16;
                buttons.push(code);
                NativeInput {
                    code: 0x10000 + u32::from(code),
                    direction: 0,
                }
            };
            bindings.insert(
                control.id.clone(),
                InputBinding {
                    code: native.code,
                    direction: native.direction,
                    kind: if control.analog { "axis" } else { "button" }.into(),
                    logical: "Driver label is deliberately irrelevant".into(),
                    axis: None,
                    native: Some(native),
                },
            );
        }
        buttons.reverse();
        (
            Calibration {
                target_mappings: Default::default(),
                layout: layout.into(),
                os: "linux".into(),
                backend: "gilrs-0.11".into(),
                bindings,
            },
            JoydevMap {
                index: 3,
                buttons,
                axes: vec![3, 2, 1, 0],
            },
        )
    }

    #[test]
    #[cfg(target_os = "linux")]
    fn sameboy_preserves_effective_model_and_its_frontend_port_count() {
        let profile = contract("sameboy", "Nintendo Game Boy").unwrap();
        for (model, count) in [
            ("Auto", 1),
            ("Auto (SGB)", 1),
            ("Game Boy", 1),
            ("Game Boy Pocket", 1),
            ("Game Boy Color 0", 1),
            ("Game Boy Color A", 1),
            ("Game Boy Color B", 1),
            ("Game Boy Color D", 1),
            ("Game Boy Player", 1),
            ("Game Boy Color C", 1),
            ("Game Boy Color", 1),
            ("Game Boy Advance", 1),
            ("Super Game Boy", 4),
            ("Super Game Boy PAL", 4),
            ("Super Game Boy 2", 4),
        ] {
            let baseline =
                format!("sameboy_model = \"{model}\"\nsameboy_mono_palette = \"olive\"\n");
            let (ports, snapshot) = topology_snapshot(profile, &baseline).unwrap();
            assert_eq!(ports, count, "{model}");
            assert_eq!(snapshot.core_options["sameboy_model"], model);
            let dir = tempfile::tempdir().unwrap();
            let config = write_core_options_snapshot(&snapshot, &baseline, dir.path()).unwrap();
            let output = std::fs::read_to_string(dir.path().join("core-options.cfg")).unwrap();
            assert_eq!(
                cfg_value(&output, "sameboy_model").unwrap().as_deref(),
                Some(model)
            );
            assert!(output.contains("sameboy_mono_palette = \"olive\""));
            assert!(config.contains("game_specific_options = \"false\""));
            assert!(config.contains("rgui_config_directory"));
            let modes = crate::controller_launch_modes::configured_modes("", &[], ports).unwrap();
            assert_eq!(modes, vec![1; count]);
        }
        let (ports, snapshot) = topology_snapshot(profile, "").unwrap();
        assert_eq!(ports, 1);
        assert_eq!(snapshot.core_options["sameboy_model"], "Auto");
        for bad in [
            "sameboy_model = \"Future model\"",
            "sameboy_model = \"\"",
            "sameboy_model = \"Auto\"\nsameboy_model = \"Super Game Boy\"",
            "sameboy_model=Auto",
        ] {
            assert!(topology_snapshot(profile, bad).is_err(), "{bad}");
        }
        assert!(topology_snapshot(profile, "#include \"hidden.opt\"").is_err());
        assert!(contract("sameboy", "Nintendo Game Boy Advance").is_none());
        assert!(contract("sameboy", "Super Nintendo Entertainment System").is_none());
    }

    #[test]
    #[cfg(target_os = "linux")]
    fn sameboy_topology_uses_one_effective_options_file_not_merged_defaults() {
        let root = tempfile::tempdir().unwrap();
        let base = root.path();
        let configs = base.join("config");
        let core_dir = configs.join("SameBoy");
        std::fs::create_dir_all(&core_dir).unwrap();
        let profile = contract("sameboy", "Game Boy").unwrap();
        let content = base.join("roms/Game.gb");
        let global = base.join("retroarch-core-options.cfg");
        std::fs::write(&global, "sameboy_model = \"Super Game Boy 2\"\n").unwrap();
        let resolve = || {
            let options =
                effective_mode_core_options(base, "", &configs, "SameBoy", &content).unwrap();
            topology_snapshot(profile, &options).unwrap()
        };
        assert_eq!(resolve().0, 4);
        std::fs::write(
            core_dir.join("SameBoy.opt"),
            "sameboy_model = \"Game Boy\"\n",
        )
        .unwrap();
        assert_eq!(resolve().0, 1);
        std::fs::write(
            core_dir.join("roms.opt"),
            "sameboy_model = \"Super Game Boy PAL\"\n",
        )
        .unwrap();
        assert_eq!(resolve().0, 4);
        // A game file replaces the folder/core/global file. Its absent model
        // means the core's Auto default, not the lower-priority four-port value.
        std::fs::write(
            core_dir.join("Game.opt"),
            "sameboy_mono_palette = \"olive\"\n",
        )
        .unwrap();
        let (ports, snapshot) = resolve();
        assert_eq!(ports, 1);
        assert_eq!(snapshot.core_options["sameboy_model"], "Auto");
        assert_eq!(
            std::fs::read_to_string(&global).unwrap(),
            "sameboy_model = \"Super Game Boy 2\"\n"
        );
        let identity = crate::emulator::PreparedRetroarchContent {
            core: "/cores/sameboy_libretro.so".into(),
            content,
        };
        let args = vec![
            "-L".into(),
            identity.core.clone().into_os_string(),
            identity.content.clone().into_os_string(),
        ];
        crate::controller_launch_modes::validate_arguments(&args, &identity).unwrap();
        for prefix in [
            "--subsystem=gb_link_2p",
            "--appendconfig=hidden.cfg",
            "--config=hidden.cfg",
        ] {
            let mut altered = vec![prefix.into()];
            altered.extend(args.clone());
            assert!(
                crate::controller_launch_modes::validate_arguments(&altered, &identity).is_err()
            );
        }
        let mut changed = args;
        *changed.last_mut().unwrap() = "/other/Game.gb".into();
        assert!(crate::controller_launch_modes::validate_arguments(&changed, &identity).is_err());
    }

    #[test]
    #[cfg(target_os = "linux")]
    fn sameboy_composes_brawler_inputs_and_assigns_only_model_ports() {
        let (calibration, numbering) = calibrated_layout("brawler64");
        for mode in [1, 257] {
            let profile = catalog()
                .launch_mode("sameboy", "Game Boy Color", mode)
                .unwrap();
            let config = player_config(&calibration, profile, &numbering, 1).unwrap();
            for (target, output) in [
                ("up", "up"),
                ("down", "down"),
                ("left", "left"),
                ("right", "right"),
                ("a", "a"),
                ("b", "b"),
                ("start", "start"),
                ("select", "select"),
            ] {
                let (_, button) = numbering
                    .binding(calibration.bindings[target].native.as_ref().unwrap())
                    .unwrap();
                assert_eq!(
                    cfg_value(&config, &format!("input_player1_{output}_btn")).unwrap(),
                    Some(button)
                );
            }
            assert_eq!(
                cfg_value(&config, "input_libretro_device_p1").unwrap(),
                Some(mode.to_string())
            );
            for unused in ["x", "y", "l", "r", "l2", "r2", "l3", "r3"] {
                assert_eq!(
                    cfg_value(&config, &format!("input_player1_{unused}_btn"))
                        .unwrap()
                        .as_deref(),
                    Some("nul")
                );
            }
        }
        let devices = (0..5)
            .map(|index| ControllerDevice {
                stable_id: format!("pad{index}"),
                name: "Identical name".into(),
                device_path: format!("/dev/input/js{index}").into(),
                event_paths: vec![],
                vendor_id: None,
                product_id: None,
                version: None,
                bus_type: None,
                physical_path: None,
                unique_id: None,
                is_virtual: false,
            })
            .collect::<Vec<_>>();
        let mut settings = AppSettings::default();
        for device in &devices {
            settings
                .controller_mapping
                .calibrations
                .insert(device.stable_id.clone(), calibration.clone());
        }
        let order = devices.iter().rev().collect::<Vec<_>>();
        for (model, count) in [("Auto", 1), ("Super Game Boy 2", 4)] {
            let profile = contract("sameboy", "Game Boy").unwrap();
            let (ports, _) =
                topology_snapshot(profile, &format!("sameboy_model = \"{model}\"")).unwrap();
            let modes = crate::controller_launch_modes::configured_modes(
                "input_libretro_device_p1 = 257",
                &[],
                ports,
            )
            .unwrap();
            let players = mode_players(&settings, "sameboy", "Game Boy", &modes, &order).unwrap();
            assert_eq!(players.len(), count);
            for (index, (port, _, device)) in players.iter().enumerate() {
                assert_eq!(*port, index + 1);
                assert_eq!(device.stable_id, order[index].stable_id);
            }
        }
        let players =
            mode_players(&settings, "sameboy", "Game Boy", &[257, 0, 1, 257], &order).unwrap();
        assert_eq!(players.iter().map(|p| p.0).collect::<Vec<_>>(), [1, 3, 4]);
        assert!(mode_players(&settings, "sameboy", "Game Boy", &[5], &order).is_err());
        assert!(
            crate::controller_launch_modes::configured_modes("", &["--device=2:257".into()], 1)
                .is_err()
        );
    }

    #[test]
    #[cfg(target_os = "linux")]
    fn swanstation_uses_mode_specific_physical_bindings_not_printed_letters() {
        for (mode, layout) in [
            (1, "brawler64"),
            (1, "dualshock"),
            (261, "dualshock"),
            (261, "xbox"),
        ] {
            let profile = catalog()
                .launch_mode("swanstation", "Sony Playstation", mode)
                .unwrap();
            let (calibration, numbering) = calibrated_layout(layout);
            let config = player_config(&calibration, profile, &numbering, 1).unwrap();
            assert!(compatible(&calibration, profile));
            assert!(config.contains(&format!("input_libretro_device_p1 = \"{mode}\"")));
            let physical = if layout == "brawler64" { "a" } else { "b" };
            let (_, cross) = numbering
                .binding(calibration.bindings[physical].native.as_ref().unwrap())
                .unwrap();
            assert!(config.contains(&format!("input_player1_b_btn = \"{cross}\"")));
            if mode == 261 {
                for (control, output) in [
                    ("stick_up", "l_y_minus"),
                    ("stick_right", "l_x_plus"),
                    ("right_stick_down", "r_y_plus"),
                    ("right_stick_left", "r_x_minus"),
                ] {
                    let (_, axis) = numbering
                        .binding(calibration.bindings[control].native.as_ref().unwrap())
                        .unwrap();
                    assert!(config.contains(&format!("input_player1_{output}_axis = \"{axis}\"")));
                    assert!(config.contains(&format!("input_player1_{output}_btn = \"nul\"")));
                }
            }
        }
        let analog = catalog().launch_mode("swanstation", "PSX", 261).unwrap();
        assert!(!compatible(&calibrated_layout("brawler64").0, analog));
        assert!(!compatible(&calibrated_layout("horizontal-four").0, analog));
        let mut missing_click = calibrated_layout("dualshock").0;
        missing_click.bindings.remove("l3");
        assert!(!compatible(&missing_click, analog));
        assert!(catalog().launch_mode("swanstation", "PSX", 517).is_none());
    }

    #[test]
    #[cfg(target_os = "linux")]
    fn beetle_keeps_requested_devices_separate_from_effective_bindings() {
        for core in ["mednafen_psx", "mednafen_psx_hw"] {
            let digital = catalog().launch_mode(core, "PSX", 1).unwrap();
            let analog = catalog().launch_mode(core, "PSX", 517).unwrap();
            let (brawler, numbering) = calibrated_layout("brawler64");
            let config = player_config_requested(&brawler, digital, &numbering, 1, 517).unwrap();
            assert_eq!(
                cfg_value(&config, "input_libretro_device_p1")
                    .unwrap()
                    .as_deref(),
                Some("517")
            );
            assert_eq!(
                cfg_value(&config, "input_player1_l_x_plus_axis")
                    .unwrap()
                    .as_deref(),
                Some("nul")
            );
            let (_, cross) = numbering
                .binding(brawler.bindings["a"].native.as_ref().unwrap())
                .unwrap();
            assert_eq!(
                cfg_value(&config, "input_player1_b_btn").unwrap(),
                Some(cross)
            );
            assert!(!compatible(&brawler, analog));
            let (dualshock, numbering) = calibrated_layout("dualshock");
            let config = player_config_requested(&dualshock, analog, &numbering, 1, 1).unwrap();
            assert_eq!(
                cfg_value(&config, "input_libretro_device_p1")
                    .unwrap()
                    .as_deref(),
                Some("1")
            );
            assert_ne!(
                cfg_value(&config, "input_player1_r_x_plus_axis")
                    .unwrap()
                    .as_deref(),
                Some("nul")
            );
            assert!(player_config_requested(&dualshock, analog, &numbering, 1, 261).is_err());
            let dir = tempfile::tempdir().unwrap();
            let prefix = if core.ends_with("_hw") {
                "beetle_psx_hw"
            } else {
                "beetle_psx"
            };
            let baseline = format!(
                "{prefix}_compatibility_settings = \"enabled\"\n{prefix}_analog_toggle = \"enabled\"\n"
            );
            write_core_options_snapshot(digital, &baseline, dir.path()).unwrap();
            let copy = std::fs::read_to_string(dir.path().join("core-options.cfg")).unwrap();
            assert!(copy.starts_with(&baseline));
            for port in [1, 2] {
                assert_eq!(
                    cfg_value(&copy, &format!("{prefix}_enable_multitap_port{port}"))
                        .unwrap()
                        .as_deref(),
                    Some("disabled")
                );
            }
        }
    }

    #[test]
    fn beetle_content_validation_does_not_restrict_swanstation_arguments() {
        let arguments = ["--sram-mode", "noload-nosave"].map(OsString::from);
        assert!(
            beetle_launch_content("swanstation", &arguments, None)
                .unwrap()
                .is_none()
        );
        let prepared = crate::emulator::PreparedRetroarchContent {
            core: "/cores/mednafen_psx_libretro.so".into(),
            content: "/games/title.cue".into(),
        };
        assert!(
            beetle_launch_content("swanstation", &arguments, Some(&prepared))
                .unwrap()
                .is_none()
        );
        for core in ["mednafen_psx", "mednafen_psx_hw"] {
            assert!(beetle_launch_content(core, &arguments, None).is_err());
            assert!(beetle_launch_content(core, &arguments, Some(&prepared)).is_err());
            let valid = [
                OsString::from("-L"),
                prepared.core.clone().into_os_string(),
                prepared.content.clone().into_os_string(),
            ];
            assert!(
                beetle_launch_content(core, &valid, Some(&prepared))
                    .unwrap()
                    .is_some()
            );
        }
    }

    #[test]
    fn compact_option_syntax_cannot_change_effective_file_selection() {
        for key in ["game_specific_options", "global_core_options"] {
            for value in ["true", "false", "\"true\"", "\"false\""] {
                assert!(config_bool(&format!("{key}={value}"), key, true).is_err());
                assert!(config_bool(&format!("{key}={value}"), key, false).is_err());
            }
        }
        for key in [
            "core_options_path",
            "rgui_config_directory",
            "input_remapping_directory",
        ] {
            assert!(cfg_value(&format!("{key}=/custom/path"), key).is_err());
        }
    }

    #[test]
    fn path_settings_accept_quoted_and_unquoted_values_without_silent_fallback() {
        for key in [
            "core_options_path",
            "rgui_config_directory",
            "input_remapping_directory",
        ] {
            for suffix in [
                "/custom/path",
                "\"/custom/path\"",
                "/custom/path # note",
                "\"/custom/path\" # note",
            ] {
                assert_eq!(
                    cfg_value(&format!("{key} = {suffix}"), key)
                        .unwrap()
                        .as_deref(),
                    Some("/custom/path")
                );
            }
            assert_eq!(
                cfg_value(&format!("{key} = \"/path with # hash\""), key)
                    .unwrap()
                    .as_deref(),
                Some("/path with # hash")
            );
            for value in [
                "",
                "# no value",
                "\"unterminated",
                "\"/custom/path\" extra",
                "/custom/path extra",
            ] {
                assert!(cfg_value(&format!("{key} = {value}"), key).is_err());
            }
            assert!(cfg_value(&format!("{key} = /first\n{key} = /second"), key).is_err());
            assert_eq!(cfg_value("# no setting", key).unwrap(), None);
        }
    }

    #[test]
    fn unquoted_custom_core_options_preserve_compatibility_choice() {
        let temp = tempfile::tempdir().unwrap();
        let custom = temp.path().join("custom.opt");
        let contents = "beetle_psx_compatibility_settings = \"disabled\"\n";
        std::fs::write(&custom, contents).unwrap();
        std::fs::write(
            temp.path().join("retroarch-core-options.cfg"),
            "beetle_psx_compatibility_settings = \"enabled\"\n",
        )
        .unwrap();
        let config = format!(
            "core_options_path = {}\ngame_specific_options = false\nglobal_core_options = true\n",
            custom.display()
        );
        let baseline = effective_mode_core_options(
            temp.path(),
            &config,
            &temp.path().join("config"),
            "Beetle PSX",
            &temp.path().join("games/title.cue"),
        )
        .unwrap();
        assert_eq!(baseline, contents);
        let private = tempfile::tempdir().unwrap();
        let profile = catalog().launch_mode("mednafen_psx", "PSX", 1).unwrap();
        write_core_options_snapshot(profile, &baseline, private.path()).unwrap();
        let written = std::fs::read_to_string(private.path().join("core-options.cfg")).unwrap();
        assert_eq!(
            cfg_value(&written, "beetle_psx_compatibility_settings")
                .unwrap()
                .as_deref(),
            Some("disabled")
        );
        assert_eq!(std::fs::read_to_string(custom).unwrap(), contents);
    }

    #[test]
    fn mode_options_follow_retroarch_file_precedence_without_merging() {
        let temp = tempfile::tempdir().unwrap();
        let base = temp.path();
        let configs = base.join("config");
        let directory = configs.join("Beetle PSX");
        std::fs::create_dir_all(&directory).unwrap();
        let content = base.join("games/title.cue");
        let global = base.join("retroarch-core-options.cfg");
        std::fs::write(&global, "global_only=1\n").unwrap();
        let resolve = |config: &str| {
            effective_mode_core_options(base, config, &configs, "Beetle PSX", &content).unwrap()
        };
        assert_eq!(resolve(""), "global_only=1\n");
        let core = directory.join("Beetle PSX.opt");
        std::fs::write(&core, "beetle_psx_compatibility_settings = disabled\n").unwrap();
        assert_eq!(
            resolve(""),
            "beetle_psx_compatibility_settings = disabled\n"
        );
        assert_eq!(resolve("global_core_options = true"), "global_only=1\n");
        let folder = directory.join("games.opt");
        std::fs::write(&folder, "folder_only=1\n").unwrap();
        assert_eq!(resolve("global_core_options = true"), "folder_only=1\n");
        let game = directory.join("title.opt");
        std::fs::write(&game, "game_only=1\n").unwrap();
        assert_eq!(resolve(""), "game_only=1\n");
        assert_eq!(
            resolve("game_specific_options = false"),
            "beetle_psx_compatibility_settings = disabled\n"
        );
        assert_eq!(
            resolve("game_specific_options = false\nglobal_core_options = true"),
            "global_only=1\n"
        );
        assert_eq!(std::fs::read_to_string(&game).unwrap(), "game_only=1\n");
        assert_eq!(std::fs::read_to_string(&global).unwrap(), "global_only=1\n");
        assert!(
            effective_mode_core_options(
                base,
                "game_specific_options = maybe",
                &configs,
                "Beetle PSX",
                &content
            )
            .is_err()
        );
    }

    #[test]
    #[cfg(unix)]
    fn mode_override_guard_follows_symlinked_files_and_directories() {
        use std::os::unix::fs::symlink;
        let temp = tempfile::tempdir().unwrap();
        let directory = temp.path().join("Beetle PSX");
        std::fs::create_dir(&directory).unwrap();
        let external = temp.path().join("managed");
        std::fs::create_dir(&external).unwrap();
        let options = external.join("core.opt");
        std::fs::write(&options, "beetle_psx_compatibility_settings = disabled").unwrap();
        symlink(&options, directory.join("Beetle PSX.opt")).unwrap();
        ensure_no_mode_overrides(&directory, "Beetle PSX").unwrap();
        let remap = external.join("game.rmp");
        std::fs::write(&remap, "input_libretro_device_p1=517").unwrap();
        let link = directory.join("game.rmp");
        symlink(&remap, &link).unwrap();
        assert!(ensure_no_mode_overrides(&directory, "Beetle PSX").is_err());
        std::fs::remove_file(&link).unwrap();
        symlink(&external, directory.join("nested")).unwrap();
        assert!(ensure_no_mode_overrides(&directory, "Beetle PSX").is_err());
        std::fs::remove_file(&remap).unwrap();
        ensure_no_mode_overrides(&directory, "Beetle PSX").unwrap();
        symlink(&directory, external.join("loop")).unwrap();
        assert!(ensure_no_mode_overrides(&directory, "Beetle PSX").is_err());
    }

    #[test]
    #[cfg(target_os = "linux")]
    fn gamegear_uses_two_buttons_and_start_without_requiring_select() {
        let profile = contract("genesis_plus_gx", "Sega Game Gear").unwrap();
        assert_eq!(profile.bindings.len(), 7);
        assert_eq!(profile.target_layout, "gamegear");
        assert_eq!(
            profile.core_options["genesis_plus_gx_system_hw"],
            "game gear"
        );
        for (layout, one, two) in [
            ("gamegear", "b", "a"),
            ("nes", "b", "a"),
            ("horizontal-four", "b", "a"),
            ("n30-turbo", "b", "a"),
            ("brawler64", "b", "a"),
            ("xbox", "y", "b"),
        ] {
            let (mut calibration, numbering) = calibrated_layout(layout);
            calibration.bindings.remove("select");
            assert!(compatible(&calibration, profile), "{layout}");
            let config = player_config(&calibration, profile, &numbering, 1).unwrap();
            assert!(config.contains("input_libretro_device_p1 = \"769\""));
            for (output, physical) in [("b", one), ("a", two), ("start", "start")] {
                let (_, button) = numbering
                    .binding(calibration.bindings[physical].native.as_ref().unwrap())
                    .unwrap();
                assert!(
                    config.contains(&format!("input_player1_{output}_btn = \"{button}\"")),
                    "{layout}/{output}"
                );
            }
            for output in ["select", "x", "y", "l", "r", "l2", "r2"] {
                for suffix in ["btn", "axis"] {
                    assert!(config.contains(&format!("input_player1_{output}_{suffix} = \"nul\"")));
                }
            }
            assert!(player_config(&calibration, profile, &numbering, 2).is_err());
            calibration.bindings.remove("start");
            // Richer layouts can supply Start with a spare gameplay button.
            // Two-button pads have no such spare; reserved D-pad/menu inputs
            // and hardware turbo repeats cannot stand in for independent keys.
            let has_spare = matches!(layout, "horizontal-four" | "brawler64" | "xbox");
            assert_eq!(compatible(&calibration, profile), has_spare, "{layout}");
            if has_spare {
                let config = player_config(&calibration, profile, &numbering, 1).unwrap();
                assert!(!config.contains("input_player1_start_btn = \"nul\""));
            }
        }
    }

    #[test]
    #[cfg(target_os = "linux")]
    fn mgba_distinguishes_gba_shoulders_from_gameboy_and_preserves_thumb_pairs() {
        let gba = catalog()
            .launch_profile("mgba", "Nintendo Game Boy Advance")
            .unwrap();
        let gb = catalog()
            .launch_profile("mgba", "Nintendo Game Boy Color")
            .unwrap();
        assert_eq!(gba.target_layout, "gba");
        assert_eq!(gb.target_layout, "gameboy");
        assert_eq!(gba.bindings.len(), 10);
        assert_eq!(gb.bindings.len(), 8);
        assert_eq!(gba.retroarch_launch.as_ref().unwrap().max_players, 1);
        assert!(catalog().launch_profile("mgba", "Nintendo DS").is_none());
        let (n30, numbering) = calibrated_layout("horizontal-four");
        assert!(compatible(&n30, gb));
        assert!(compatible(&n30, gba));
        let config = player_config(&n30, gba, &numbering, 1).unwrap();
        for (target, physical) in [("l", "y"), ("r", "x")] {
            let (_, button) = numbering
                .binding(n30.bindings[physical].native.as_ref().unwrap())
                .unwrap();
            assert!(config.contains(&format!("input_player1_{target}_btn = \"{button}\"")));
        }
        let (turbo, _) = calibrated_layout("n30-turbo");
        assert!(
            !compatible(&turbo, gba),
            "hardware repeats are not extra independent buttons"
        );
        for (layout, physical_a, physical_b) in [
            ("gba", "a", "b"),
            ("brawler64", "a", "b"),
            ("xbox", "b", "y"),
            ("dualshock", "b", "y"),
        ] {
            let (calibration, numbering) = calibrated_layout(layout);
            assert!(compatible(&calibration, gba));
            let config = player_config(&calibration, gba, &numbering, 1).unwrap();
            for (output, physical) in [("a", physical_a), ("b", physical_b), ("l", "l"), ("r", "r")]
            {
                let (suffix, button) = numbering
                    .binding(calibration.bindings[physical].native.as_ref().unwrap())
                    .unwrap();
                assert_eq!(suffix, "btn");
                assert!(
                    config.contains(&format!("input_player1_{output}_btn = \"{button}\"")),
                    "{layout}: {output}"
                );
            }
            // Do not accidentally inherit turbo or solar-sensor controls.
            for output in ["x", "y", "l2", "r2", "l3", "r3"] {
                assert!(config.contains(&format!("input_player1_{output}_btn = \"nul\"")));
                assert!(config.contains(&format!("input_player1_{output}_axis = \"nul\"")));
            }
        }
    }

    #[test]
    #[cfg(target_os = "linux")]
    fn mixed_console_port_modes_allocate_independent_compatible_controllers() {
        let make_device = |id: &str| ControllerDevice {
            stable_id: id.into(),
            name: "Same USB name".into(),
            device_path: format!("/dev/input/{id}").into(),
            event_paths: vec![],
            vendor_id: None,
            product_id: None,
            version: None,
            bus_type: None,
            physical_path: None,
            unique_id: None,
            is_virtual: false,
        };
        let devices = [make_device("js4"), make_device("js5")];
        let mut settings = AppSettings::default();
        settings
            .controller_mapping
            .calibrations
            .insert("js4".into(), calibrated_layout("brawler64").0);
        settings
            .controller_mapping
            .calibrations
            .insert("js5".into(), calibrated_layout("dualshock").0);
        for order in [[&devices[0], &devices[1]], [&devices[1], &devices[0]]] {
            let players = mode_players(&settings, "swanstation", "PSX", &[1, 1], &order).unwrap();
            assert_eq!(players.len(), 2);
            assert_eq!(players[0].2.stable_id, order[0].stable_id);
            assert_eq!(players[1].2.stable_id, order[1].stable_id);
        }
        let digital_first = mode_players(
            &settings,
            "swanstation",
            "PSX",
            &[1, 261],
            &[&devices[1], &devices[0]],
        )
        .unwrap();
        assert_eq!(digital_first.len(), 2);
        assert_eq!(digital_first[0].2.stable_id, "js4");
        assert_eq!(digital_first[1].2.stable_id, "js5");
        let selected = mode_players(
            &settings,
            "swanstation",
            "PSX",
            &[261, 1],
            &[&devices[0], &devices[1]],
        )
        .unwrap();
        assert_eq!(
            (selected[0].0, selected[0].2.stable_id.as_str()),
            (1, "js5")
        );
        assert_eq!(
            (selected[1].0, selected[1].2.stable_id.as_str()),
            (2, "js4")
        );
        assert!(mode_players(&settings, "swanstation", "PSX", &[261, 1], &[&devices[0]]).is_err());
        let selected =
            mode_players(&settings, "swanstation", "PSX", &[0, 1], &[&devices[0]]).unwrap();
        assert_eq!(selected[0].0, 2);
        assert!(mode_players(&settings, "swanstation", "PSX", &[517, 1], &[&devices[1]]).is_err());
    }

    #[test]
    #[cfg(target_os = "linux")]
    #[ignore = "Real RetroArch Flatpak BIOS-only smoke check; requires explicit trusted core and BIOS directory"]
    fn swanstation_flatpak_mode_startup_oracle() {
        use std::os::unix::process::CommandExt;
        use std::{
            fs,
            process::{Command, Stdio},
            time::{Duration, Instant},
        };
        let core = std::path::PathBuf::from(
            std::env::var_os("LUNCHBOX_TEST_SWANSTATION_CORE").expect("Set trusted core path"),
        )
        .canonicalize()
        .unwrap();
        let bios = std::path::PathBuf::from(
            std::env::var_os("LUNCHBOX_TEST_PSX_BIOS_DIRECTORY").expect("Set BIOS directory"),
        )
        .canonicalize()
        .unwrap();
        assert!(core.is_file() && bios.is_dir());
        let directory = tempfile::Builder::new()
            .prefix("lunchbox-swanstation-mode-oracle-")
            .tempdir_in("/tmp")
            .unwrap()
            .keep();
        for mode in [1, 261] {
            let root = directory.join(mode.to_string());
            fs::create_dir(&root).unwrap();
            for folder in ["saves", "states", "cache", "logs"] {
                fs::create_dir(root.join(folder)).unwrap();
            }
            let profile = catalog().launch_mode("swanstation", "PSX", mode).unwrap();
            let (calibration, numbering) = calibrated_layout("dualshock");
            let mut config = player_config(&calibration, profile, &numbering, 1).unwrap();
            config.push_str("menu_show_start_screen = \"false\"\nmenu_pause_libretro = \"false\"\naudio_enable = \"false\"\nhistory_list_enable = \"false\"\ngame_specific_options = \"false\"\n");
            config.push_str("config_save_on_exit = \"false\"\ninput_autodetect_enable = \"false\"\nauto_remaps_enable = \"false\"\nauto_overrides_enable = \"false\"\ninput_max_users = \"1\"\ninput_joypad_driver = \"linuxraw\"\ninput_driver = \"udev\"\nvideo_driver = \"null\"\naudio_driver = \"null\"\nvideo_vsync = \"false\"\nmenu_enable_widgets = \"false\"\npause_nonactive = \"false\"\npause_on_disconnect = \"false\"\n");
            // A real video driver is necessary for a meaningful rendered-frame
            // limit; a null display can run without advancing that counter.
            config = config.replace("video_driver = \"null\"", "video_driver = \"glcore\"");
            for (key, path) in [
                ("system_directory", bios.clone()),
                ("savefile_directory", root.join("saves")),
                ("savestate_directory", root.join("states")),
                ("cache_directory", root.join("cache")),
                ("log_dir", root.join("logs")),
                ("core_options_path", root.join("options.cfg")),
                ("rgui_config_directory", root.join("config")),
                ("content_history_path", root.join("history.lpl")),
                ("content_favorites_path", root.join("favorites.lpl")),
                ("content_image_history_path", root.join("images.lpl")),
                ("content_music_history_path", root.join("music.lpl")),
                ("content_video_history_path", root.join("videos.lpl")),
            ] {
                let path = path.to_str().unwrap();
                assert!(!path.contains(['"', '\n', '\r']));
                config.push_str(&format!("{key} = \"{path}\"\n"));
            }
            fs::write(root.join("options.cfg"), "swanstation_GPU_Renderer = \"Software\"\nswanstation_ControllerPorts_MultitapMode = \"Disabled\"\n").unwrap();
            fs::write(root.join("retroarch.cfg"), config).unwrap();
            let log = fs::File::create(root.join("startup.log")).unwrap();
            let mut child = Command::new("flatpak")
                .process_group(0)
                .arg("run")
                .arg(format!(
                    "--filesystem={}:ro",
                    core.parent().unwrap().display()
                ))
                .arg(format!("--filesystem={}:ro", bios.display()))
                .arg(format!("--filesystem={}", root.display()))
                .args(["org.libretro.RetroArch", "--verbose", "--config"])
                .arg(root.join("retroarch.cfg"))
                .arg("-L")
                .arg(&core)
                .args(["--max-frames", "8", "--sram-mode", "noload-nosave"])
                .stdout(Stdio::from(log.try_clone().unwrap()))
                .stderr(Stdio::from(log))
                .spawn()
                .unwrap();
            let started = Instant::now();
            let status = loop {
                if let Some(status) = child.try_wait().unwrap() {
                    break status;
                }
                if started.elapsed() > Duration::from_secs(30) {
                    // A Flatpak launcher can exit before its sandbox child.
                    // This test alone owns the fresh process group; never kill
                    // every RetroArch process or a shared terminal job group.
                    unsafe {
                        libc::kill(-(child.id() as i32), libc::SIGTERM);
                    }
                    std::thread::sleep(Duration::from_millis(200));
                    unsafe {
                        libc::kill(-(child.id() as i32), libc::SIGKILL);
                    }
                    child.wait().unwrap();
                    panic!("Core startup timed out; logs at {}", root.display());
                }
                std::thread::sleep(Duration::from_millis(20));
            };
            let text = fs::read_to_string(root.join("startup.log")).unwrap();
            assert!(
                status.success(),
                "Core startup failed; logs at {}",
                root.display()
            );
            assert!(
                text.contains("SwanStation"),
                "Wrong core; logs at {}",
                root.display()
            );
            println!(
                "SWANSTATION_MODE_ORACLE mode={mode} log={}",
                root.join("startup.log").display()
            );
        }
    }
    #[test]
    #[cfg(target_os = "linux")]
    fn measured_event_identity_never_falls_back_to_first_event_or_same_model() {
        use std::os::unix::fs::symlink;
        let directory = tempfile::tempdir().unwrap();
        let sys = directory.path();
        for name in ["pad-a", "pad-b", "js4", "event8", "event9", "event10"] {
            std::fs::create_dir(sys.join(name)).unwrap();
        }
        for (node, physical) in [
            ("js4", "pad-a"),
            ("event8", "pad-b"),
            ("event9", "pad-a"),
            ("event10", "pad-a"),
        ] {
            symlink(sys.join(physical), sys.join(node).join("device")).unwrap();
        }
        let joystick = Path::new("/dev/input/js4");
        let events = vec!["/dev/input/event8".into(), "/dev/input/event9".into()];
        assert_eq!(
            measured_event_path(joystick, &events, sys).unwrap().0,
            Path::new("/dev/input/event9")
        );
        assert!(measured_event_path(joystick, &events[..1], sys).is_err());
        let mut ambiguous = events;
        ambiguous.push("/dev/input/event10".into());
        assert!(measured_event_path(joystick, &ambiguous, sys).is_err());
        assert!(measured_event_path(Path::new("/tmp/js4"), &ambiguous, sys).is_err());
    }

    #[test]
    #[cfg(target_os = "linux")]
    fn snes_cores_compose_physical_inputs_through_retropad() {
        for core in ["snes9x", "bsnes"] {
            let profile = contract(core, "Nintendo - Super Nintendo Entertainment System").unwrap();
            for layout in ["snes", "xbox", "dualshock", "brawler64"] {
                let mut cal = nes();
                cal.layout = layout.into();
                cal.bindings = catalog()
                    .layout(layout)
                    .unwrap()
                    .controls
                    .iter()
                    .filter(|control| !control.analog)
                    .enumerate()
                    .map(|(index, control)| {
                        (
                            control.id.clone(),
                            InputBinding {
                                code: index as u32,
                                kind: "button".into(),
                                direction: 0,
                                logical: control.label.clone(),
                                axis: None,
                                native: Some(NativeInput {
                                    code: 0x10120 + index as u32,
                                    direction: 0,
                                }),
                            },
                        )
                    })
                    .collect();
                let map = JoydevMap {
                    index: 3,
                    buttons: (288..288 + cal.bindings.len() as u16).rev().collect(),
                    axes: vec![],
                };
                let config = player_config(&cal, profile, &map, 1).unwrap();
                assert!(compatible(&cal, profile), "{core}/{layout}");
                let face = if layout == "brawler64" {
                    [("a", "c_down"), ("b", "a"), ("x", "c_left"), ("y", "b")]
                } else {
                    [("a", "a"), ("b", "b"), ("x", "x"), ("y", "y")]
                };
                for (output, physical) in face.into_iter().chain([
                    ("l", "l"),
                    ("r", "r"),
                    ("start", "start"),
                    ("select", "select"),
                ]) {
                    let (_, number) = map
                        .binding(cal.bindings[physical].native.as_ref().unwrap())
                        .unwrap();
                    assert!(
                        config.contains(&format!("input_player1_{output}_btn = \"{number}\"")),
                        "{core}/{layout}/{output}"
                    );
                }
                assert!(config.contains("input_libretro_device_p1 = \"1\""));
                assert!(player_config(&cal, profile, &map, 3).is_err());
            }
            // Four face buttons alone do not supply the required SNES shoulders.
            let mut n30 = nes();
            n30.layout = "horizontal-four".into();
            assert!(!compatible(&n30, profile));
        }
    }

    #[test]
    fn launch_opt_in_and_port_count_come_from_the_catalog() {
        for profile in &catalog().emulator_profiles {
            assert_eq!(
                supports_profile(profile),
                (cfg!(target_os = "linux") && profile.retroarch_launch.is_some())
                    || profile.native_launch.is_some()
            );
            if let Some(launch) = &profile.retroarch_launch {
                for alias in &launch.platforms {
                    assert_eq!(
                        catalog()
                            .launch_mode(&profile.core, alias, launch.device)
                            .unwrap()
                            .id,
                        profile.id
                    );
                }
            }
        }
        let preview = catalog()
            .emulator_profiles
            .iter()
            .find(|p| p.id == "retroarch:genesis_plus_gx:md3")
            .unwrap();
        assert!(!supports_profile(preview));
        assert!(restrict_players(&mut vec![], preview).is_err());
    }
    #[test]
    #[cfg(target_os = "linux")]
    fn horizontal_and_turbo_pads_compose_pce_gameplay_without_upper_button_aliases() {
        let profile = contract("mednafen_pce_fast", "NEC TurboGrafx-16").unwrap();
        for layout in ["horizontal-four", "n30-turbo", "pce-2"] {
            let mut cal = nes();
            cal.layout = layout.into();
            let map = JoydevMap {
                index: 2,
                buttons: (288..296).rev().collect(),
                axes: vec![],
            };
            let config = player_config(&cal, profile, &map, 1).unwrap();
            for (physical, output) in [("a", "a"), ("b", "b")] {
                let (_, number) = map
                    .binding(cal.bindings[physical].native.as_ref().unwrap())
                    .unwrap();
                assert!(
                    config.contains(&format!("input_player1_{output}_btn = \"{number}\"")),
                    "{layout}"
                );
            }
            for output in ["x", "y", "l", "r", "l2"] {
                assert!(
                    config.contains(&format!("input_player1_{output}_btn = \"nul\"")),
                    "{layout}"
                );
            }
        }
        let options = core_options_overlay(
            "pce_fast_default_joypad_type_p1 = \"6 Buttons\"\npce_fast_cdspeed = \"4\"\n",
            &profile.core_options,
        )
        .unwrap();
        assert!(!options.contains("6 Buttons"));
        assert!(options.contains("pce_fast_cdspeed = \"4\""));
        for player in 1..=5 {
            assert!(options.contains(&format!(
                "pce_fast_default_joypad_type_p{player} = \"2 Buttons\""
            )));
        }
    }
    #[test]
    fn n64_independent_buttons_and_core_options_are_composed_together() {
        let profile = contract("mupen64plus_next", "Nintendo 64").unwrap();
        assert_eq!(profile.bindings["c_left"], "LeftBumper");
        assert_eq!(profile.bindings["c_right"], "RightBumper");
        assert_eq!(profile.bindings["l"], "Select");
        assert_eq!(profile.bindings["r"], "RightTrigger");
        let baseline = "mupen64plus-alt-map = \"False\"\nmupen64plus-pak1 = \"rumble\"\n";
        let options = core_options_overlay(baseline, &profile.core_options).unwrap();
        assert_eq!(options.matches("mupen64plus-alt-map").count(), 1);
        assert!(options.contains("mupen64plus-alt-map = \"True\""));
        assert!(options.contains("mupen64plus-pak1 = \"rumble\""));
        assert!(core_options_overlay("#include \"other.cfg\"", &profile.core_options).is_err());
    }
    #[test]
    fn flatpak_config_is_scoped_and_inserted_inside_app_arguments() {
        let executable = EmulatorExecutable::Flatpak {
            command: "flatpak".into(),
            app_id: "org.libretro.RetroArch".into(),
        };
        let mut plan = LaunchPlan {
            emulator_name: "RetroArch".into(),
            program: "flatpak".into(),
            arguments: vec![
                "run".into(),
                "--filesystem=/roms:ro".into(),
                "org.libretro.RetroArch".into(),
                "-L".into(),
                "/cores/fceumm_libretro.so".into(),
                "/roms/game.nes".into(),
            ],
            current_directory: "/roms".into(),
            environment: vec![],
            cleanup_paths: vec![],
            retroarch_content: None,
        };
        attach_config(
            &mut plan,
            &executable,
            Path::new("/cache/private session/controllers.cfg"),
        )
        .unwrap();
        assert_eq!(plan.arguments[2], "--filesystem=/cache/private session");
        assert_eq!(plan.arguments[3], "org.libretro.RetroArch");
        assert_eq!(plan.arguments[4], "--appendconfig");
        assert_eq!(plan.arguments[5], "/cache/private session/controllers.cfg");
        assert_eq!(plan.arguments.last().unwrap(), "/roms/game.nes");
    }
    #[test]
    fn selected_controller_order_honors_explicit_order_preference_and_hidden_devices() {
        let device = |id: &str| ControllerDevice {
            stable_id: id.into(),
            name: id.into(),
            device_path: format!("/dev/input/{id}").into(),
            event_paths: vec![],
            vendor_id: None,
            product_id: None,
            version: None,
            bus_type: None,
            physical_path: None,
            unique_id: None,
            is_virtual: false,
        };
        let devices = vec![device("pad-a"), device("pad-b")];
        let mut ordered = vec![&devices[1], &devices[0]];
        assert_eq!(
            restrict_players(
                &mut ordered,
                contract("gambatte", "Nintendo Game Boy").unwrap()
            )
            .unwrap(),
            2
        );
        assert_eq!(ordered.len(), 1);
        assert_eq!(ordered[0].stable_id, "pad-b");
        let mut settings = AppSettings::default();
        for device in &devices {
            settings
                .controller_mapping
                .calibrations
                .insert(device.stable_id.clone(), nes());
        }
        let brawler = settings
            .controller_mapping
            .calibrations
            .get_mut("pad-b")
            .unwrap();
        brawler.layout = "brawler64".into();
        brawler.bindings.retain(|id, _| id == "a" || id == "b");
        brawler.validate().unwrap();
        assert_eq!(
            selected_devices(&settings, &devices, "Nintendo 64")[0].stable_id,
            "pad-b"
        );
        assert_eq!(
            selected_devices(&settings, &devices, "Nintendo Entertainment System")[0].stable_id,
            "pad-a"
        );
        settings
            .controller_mapping
            .preferred_devices
            .insert("n64".into(), "pad-b".into());
        assert_eq!(
            selected_devices(&settings, &devices, "Nintendo 64")[0].stable_id,
            "pad-b"
        );
        settings.controller_mapping.player_mappings.push(
            crate::settings::ControllerPlayerMapping {
                controller_id: Some("pad-a".into()),
                ..Default::default()
            },
        );
        assert_eq!(
            selected_devices(&settings, &devices, "Nintendo 64")[0].stable_id,
            "pad-a"
        );
        settings
            .controller_mapping
            .hidden_controller_ids
            .push("pad-a".into());
        assert_eq!(
            selected_devices(&settings, &devices, "Nintendo 64").len(),
            1
        );
    }
    fn nes() -> Calibration {
        let bindings = catalog()
            .layout("nes")
            .unwrap()
            .controls
            .iter()
            .enumerate()
            .map(|(index, c)| {
                (
                    c.id.clone(),
                    InputBinding {
                        code: 0x10000 + index as u32,
                        kind: "button".into(),
                        direction: 0,
                        logical: c.label.clone(),
                        axis: None,
                        native: Some(NativeInput {
                            code: 0x10120 + index as u32,
                            direction: 0,
                        }),
                    },
                )
            })
            .collect();
        Calibration {
            target_mappings: Default::default(),
            layout: "nes".into(),
            os: "linux".into(),
            backend: "gilrs-0.11".into(),
            bindings,
        }
    }
    #[test]
    fn device_indices_are_not_xbox_button_numbers() {
        let map = JoydevMap {
            index: 5,
            buttons: vec![305, 304],
            axes: vec![0, 1, 16, 17],
        };
        assert_eq!(
            map.binding(&NativeInput {
                code: 0x10130,
                direction: 0
            })
            .unwrap(),
            ("btn", "1".into())
        );
        assert_eq!(
            map.binding(&NativeInput {
                code: 0x30011,
                direction: -1
            })
            .unwrap(),
            ("axis", "-3".into())
        );
        assert!(
            map.binding(&NativeInput {
                code: 0x101ff,
                direction: 0
            })
            .is_err()
        );
    }
    #[test]
    #[cfg(target_os = "linux")]
    fn exact_physical_bindings_clear_inherited_axes_and_reject_legacy_calibration() {
        let mut cal = nes();
        let map = JoydevMap {
            index: 5,
            buttons: (288..296).rev().collect(),
            axes: vec![],
        };
        let profile = contract("fceumm", "Nintendo Entertainment System").unwrap();
        let config = player_config(&cal, profile, &map, 1).unwrap();
        assert!(config.contains("input_player1_b_btn = \"7\""));
        assert!(config.contains("input_player1_b_axis = \"nul\""));
        assert!(config.contains("input_player1_joypad_index = \"5\""));
        cal.bindings.get_mut("b").unwrap().native = None;
        assert!(player_config(&cal, profile, &map, 1).is_err());
        cal.bindings.remove("b");
        assert!(player_config(&cal, profile, &map, 1).is_err());
    }
    #[test]
    fn append_config_preserves_custom_lists_and_argument_boundaries() {
        let mut arguments = vec![
            "--appendconfig".into(),
            "/tmp/user config.cfg".into(),
            "game.nes".into(),
        ];
        append_argument(&mut arguments, Path::new("/tmp/lunchbox config.cfg")).unwrap();
        assert_eq!(
            arguments[1],
            "/tmp/user config.cfg|/tmp/lunchbox config.cfg"
        );
        assert_eq!(arguments[2], "game.nes");
        let mut arguments = vec!["--".into(), "game.nes".into()];
        append_argument(&mut arguments, Path::new("/tmp/controller.cfg")).unwrap();
        assert_eq!(arguments[2], "--");
    }
    #[test]
    fn contracts_never_infer_system_from_core_name_alone() {
        assert!(contract("mednafen_pce_fast", "NEC SuperGrafx").is_none());
        assert!(contract("mednafen_pce_fast", "NEC PC-FX").is_none());
        assert!(contract("mednafen_pce", "NEC PC Engine").is_none());
        assert!(contract("mednafen_pce_fast", "NEC TurboGrafx-CD").is_some());
        assert_eq!(
            contract("mednafen_pce_fast", "NEC - PC Engine CD - TurboGrafx-CD")
                .unwrap()
                .target_layout,
            "pce-2"
        );
        assert!(contract("genesis_plus_gx", "Sega Master System").is_none());
        assert!(contract("fceumm", "Super Nintendo Entertainment System").is_none());
        assert!(contract("unknown", "Nintendo Entertainment System").is_none());
        assert_eq!(
            contract("genesis_plus_gx", "Sega Genesis")
                .unwrap()
                .target_layout,
            "genesis-6"
        );
    }
}
