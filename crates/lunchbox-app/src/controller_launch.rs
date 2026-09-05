//! Launch-scoped controller adapters. Never write a user's emulator config.
use std::collections::BTreeMap;
use std::ffi::OsString;
use std::path::Path;

use crate::controller_catalog::{Calibration, EmulatorProfile, NativeInput, catalog};
use crate::controllers::ControllerDevice;
use crate::emulator::{EmulatorExecutable, EmulatorRuntimeKind, LaunchPlan, RomEmulatorOption};
use crate::settings::AppSettings;
use anyhow::{Context, Result, bail, ensure};

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
    let platform = platform.to_ascii_lowercase();
    if core == "mupen64plus_next" && (platform.contains("nintendo 64") || platform == "n64") {
        return catalog()
            .emulator_profiles
            .iter()
            .find(|p| p.id == "retroarch:mupen64plus_next:n64-independent");
    }
    let target = match core {
        "fceumm"
            if platform.contains("entertainment system") && !platform.contains("super")
                || matches!(
                    platform.as_str(),
                    "nes" | "nintendo nes" | "nintendo famicom"
                ) =>
        {
            "nes"
        }
        "gambatte" if platform.contains("game boy") && !platform.contains("advance") => "gameboy",
        "mednafen_pce_fast"
            if matches!(
                platform.as_str(),
                "nec pc engine"
                    | "pc engine"
                    | "nec turbografx-16"
                    | "turbografx-16"
                    | "nec turbografx-16 cd"
                    | "turbografx-cd"
                    | "nec turbografx-cd"
                    | "nec pc engine cd"
                    | "pc engine cd"
                    | "nec - pc engine - turbografx 16"
                    | "nec - pc engine cd - turbografx-cd"
            ) =>
        {
            "pce-2"
        }
        "genesis_plus_gx" if platform.contains("genesis") || platform.contains("mega drive") => {
            "genesis-6"
        }
        _ => return None,
    };
    catalog()
        .emulator_profiles
        .iter()
        .find(|p| p.core == core && p.target_layout == target)
}

pub fn supports_profile(profile: &EmulatorProfile) -> bool {
    cfg!(target_os = "linux")
        && matches!(
            profile.id.as_str(),
            "retroarch:fceumm:nes"
                | "retroarch:mednafen_pce_fast:pce2"
                | "retroarch:gambatte:gameboy"
                | "retroarch:genesis_plus_gx:md6"
                | "retroarch:mupen64plus_next:n64-independent"
        )
}

fn cfg_value(text: &str, key: &str) -> Option<String> {
    text.lines()
        .filter_map(|line| line.trim().split_once('='))
        .filter(|(name, _)| name.trim() == key)
        .filter_map(|(_, value)| {
            value
                .trim()
                .strip_prefix('"')
                .and_then(|value| value.split_once('"'))
                .map(|(value, _)| value.to_string())
        })
        .next()
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
    let dirs = directories::BaseDirs::new().context("Finding RetroArch config directory")?;
    let base = match executable {
        EmulatorExecutable::Flatpak { app_id, .. } => dirs
            .home_dir()
            .join(".var/app")
            .join(app_id)
            .join("config/retroarch"),
        _ => dirs.config_dir().join("retroarch"),
    };
    let config = read_optional(&base.join("retroarch.cfg"))?;
    ensure!(
        !config
            .lines()
            .any(|line| line.trim_start().starts_with("#include")),
        "Included RetroArch configs require core-options resolution before calibrated launch with core options"
    );
    let path = match cfg_value(&config, "core_options_path").filter(|v| !v.is_empty()) {
        Some(path) => {
            let path = std::path::PathBuf::from(path);
            ensure!(
                path.is_absolute(),
                "Custom relative core-options path is not supported yet"
            );
            path
        }
        None => base.join("retroarch-core-options.cfg"),
    };
    let options = core_options_overlay(&read_optional(&path)?, &profile.core_options)?;
    let output = directory.join("core-options.cfg");
    let output_text = output.to_str().context("Core-options path must be UTF-8")?;
    ensure!(
        !output_text.contains(['"', '\n', '\r', '\\']),
        "Core-options path cannot be encoded"
    );
    std::fs::write(&output, options)?;
    Ok(format!(
        "game_specific_options = \"false\"\ncore_options_path = \"{output_text}\"\n"
    ))
}

/// Read-only runtime numbering, independent of the labels printed on a pad.
#[derive(Debug)]
pub struct JoydevMap {
    pub index: usize,
    pub buttons: Vec<u16>,
    pub axes: Vec<u8>,
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
        println!(
            "{}",
            serde_json::json!({ "device": device.device_path, "name": device.name,
            "index": numbering.index, "buttons": numbering.buttons, "axes": numbering.axes })
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
    calibration.validate()?;
    ensure!(
        calibration.os == std::env::consts::OS && calibration.os == "linux",
        "Recalibrate on this host OS"
    );
    ensure!((1..=16).contains(&player), "Invalid player number");
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
                .find(|c| c.label == row.target)
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
        if profile.target_layout == "genesis-6" {
            "513"
        } else {
            "1"
        }
        .into(),
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
    if devices.is_empty() {
        return Ok(None);
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
    // A connected N30 must not prevent a calibrated Brawler64 from playing N64.
    // Only controllers with every required target capability enter the player list.
    devices.retain(|device| compatible(&mapping.calibrations[&device.stable_id], profile));
    ensure!(
        !devices.is_empty(),
        "No connected calibration has all required controls for {}. Complete calibration (older calibrations need physical-input capture), or disable Apply saved calibrations.",
        profile.name
    );
    ensure!(devices.len() <= 16, "Too many calibrated controllers");
    ensure!(
        profile.core != "mednafen_pce_fast" || devices.len() <= 5,
        "Beetle PCE Fast supports at most five controller ports; hide extra controllers in Settings"
    );
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
        let numbering = JoydevMap::read(&device.device_path)?;
        config.push_str(&player_config(
            &mapping.calibrations[&device.stable_id],
            profile,
            &numbering,
            index + 1,
        )?);
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
            "Applied {} calibrated controller(s) for {} · RetroArch automatic overrides/remaps suspended for this launch",
            devices.len(),
            profile.name
        ),
    }))
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
                        native: Some(NativeInput {
                            code: 0x10120 + index as u32,
                            direction: 0,
                        }),
                    },
                )
            })
            .collect();
        Calibration {
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
        assert!(contract("genesis_plus_gx", "Sega Game Gear").is_none());
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
