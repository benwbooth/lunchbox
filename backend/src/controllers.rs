use crate::state::{AppSettings, ControllerMappingSettings};
use serde::Serialize;
use std::collections::HashSet;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::time::Duration;

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ControllerInventory {
    pub provider: ControllerProviderStatus,
    pub controllers: Vec<ControllerDevice>,
    pub managed_devices: Vec<InputPlumberCompositeDevice>,
    pub supported_targets: Vec<InputPlumberTargetDevice>,
    pub built_in_profiles: Vec<ControllerProfileInfo>,
    pub warnings: Vec<String>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ControllerProviderStatus {
    pub provider: String,
    pub available: bool,
    pub version: Option<String>,
    pub service_accessible: bool,
    pub message: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ControllerDevice {
    pub stable_id: String,
    pub name: String,
    pub device_path: String,
    pub event_paths: Vec<String>,
    pub vendor_id: Option<String>,
    pub product_id: Option<String>,
    pub version: Option<String>,
    pub bus_type: Option<String>,
    pub physical_path: Option<String>,
    pub unique_id: Option<String>,
    pub is_virtual: bool,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct InputPlumberCompositeDevice {
    pub id: String,
    pub name: String,
    pub profile_name: Option<String>,
    pub profile_path: Option<String>,
    pub source_paths: Vec<String>,
    pub target_ids: Vec<String>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct InputPlumberTargetDevice {
    pub id: String,
    pub name: String,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ControllerProfileInfo {
    pub id: String,
    pub name: String,
    pub description: String,
}

#[derive(Debug, Default)]
pub struct ControllerLaunchSession {
    restore_entries: Vec<InputPlumberRestoreEntry>,
}

#[derive(Debug)]
struct InputPlumberRestoreEntry {
    device_id: String,
    intercept_mode: Option<String>,
    profile_path: Option<String>,
    target_ids: Vec<String>,
}

pub fn built_in_profiles() -> Vec<ControllerProfileInfo> {
    vec![ControllerProfileInfo {
        id: TWO_BUTTON_CLOCKWISE_PROFILE_ID.to_string(),
        name: "2-button clockwise diamond".to_string(),
        description:
            "Maps physical bottom/right face buttons to target left/bottom for NES-style layouts."
                .to_string(),
    }]
}

pub fn controller_inventory() -> ControllerInventory {
    let mut warnings = Vec::new();
    let controllers = list_local_controllers(&mut warnings);
    let (provider, managed_devices, supported_targets) = inputplumber_inventory(&mut warnings);

    ControllerInventory {
        provider,
        controllers,
        managed_devices,
        supported_targets,
        built_in_profiles: built_in_profiles(),
        warnings,
    }
}

pub async fn activate_for_launch(
    settings: &AppSettings,
    platform_name: Option<&str>,
    launchbox_db_id: Option<i64>,
) -> Result<ControllerLaunchSession, String> {
    let mapping = &settings.controller_mapping;
    if !mapping.enabled {
        return Ok(ControllerLaunchSession::default());
    }

    let selected_profile = resolve_profile_id(mapping, platform_name, launchbox_db_id);

    if !cfg!(target_os = "linux") {
        return Err(
            "Controller remapping is currently implemented for Linux/InputPlumber only."
                .to_string(),
        );
    }

    let provider = mapping.provider.trim();
    if !provider.is_empty() && provider != "auto" && provider != "inputplumber" {
        return Err(format!(
            "Unsupported controller mapping provider '{}'.",
            mapping.provider
        ));
    }

    if which::which("inputplumber").is_err() {
        return Err("InputPlumber is not available on PATH.".to_string());
    }

    let mut warnings = Vec::new();
    let controllers = list_local_controllers(&mut warnings);
    let mut managed_devices = list_inputplumber_composite_devices()
        .map_err(|e| format!("Failed to list InputPlumber managed devices: {e}"))?;

    if mapping.manage_all || managed_devices.is_empty() {
        run_inputplumber(&["devices", "manage-all", "--enable"])?;
        managed_devices = list_inputplumber_composite_devices()
            .map_err(|e| format!("Failed to list InputPlumber managed devices: {e}"))?;
    }

    if managed_devices.is_empty() {
        return Err(
            "InputPlumber has no managed composite devices. Enable InputPlumber management for the controller first."
                .to_string(),
        );
    }

    let profile_scope_is_all = mapping
        .profile_controller_ids
        .iter()
        .all(|id| id.trim().is_empty());
    let profile_controller_paths = if profile_scope_is_all {
        controller_source_paths(&controllers)
    } else {
        selected_controller_source_paths(&controllers, &mapping.profile_controller_ids)
    };
    let hidden_controller_paths =
        selected_controller_source_paths(&controllers, &mapping.hidden_controller_ids);
    let target_device_ids = normalize_target_ids(&mapping.output_target);
    let profile_path = if let Some(profile_id) = selected_profile.as_deref() {
        Some(resolve_profile_path(settings, profile_id)?)
    } else {
        None
    };

    let mut session = ControllerLaunchSession::default();
    let mut matched_any = false;

    for device in managed_devices {
        let matches_hidden = !hidden_controller_paths.is_empty()
            && device
                .source_paths
                .iter()
                .any(|path| hidden_controller_paths.contains(path));
        let matches_profile = profile_scope_is_all
            || device
                .source_paths
                .iter()
                .any(|path| profile_controller_paths.contains(path));

        if !matches_hidden && !matches_profile {
            continue;
        }

        matched_any = true;
        let restore = InputPlumberRestoreEntry {
            device_id: device.id.clone(),
            intercept_mode: Some(inputplumber_device_intercept_mode(&device.id)?),
            profile_path: device.profile_path.clone(),
            target_ids: device.target_ids.clone(),
        };

        session.restore_entries.push(restore);

        let apply_result = (|| {
            run_inputplumber(&["device", &device.id, "intercept", "set", "gamepad-only"])?;

            if matches_hidden {
                run_inputplumber(&["device", &device.id, "targets", "set", "null"])?;
            } else {
                run_inputplumber_with_dynamic_args(
                    ["device", &device.id, "targets", "set"],
                    &target_device_ids,
                )?;
                if let Some(path) = profile_path.as_ref() {
                    run_inputplumber(&[
                        "device",
                        &device.id,
                        "profile",
                        "load",
                        path.to_string_lossy().as_ref(),
                    ])?;
                }
            }
            Ok(())
        })();

        if let Err(e) = apply_result {
            session.restore();
            return Err(e);
        }
    }

    if !matched_any {
        return Err(
            "No InputPlumber managed device matched the selected controller list.".to_string(),
        );
    }

    Ok(session)
}

impl ControllerLaunchSession {
    pub fn is_active(&self) -> bool {
        !self.restore_entries.is_empty()
    }

    pub fn restore(self) {
        for entry in self.restore_entries {
            if !entry.target_ids.is_empty() {
                let _ = run_inputplumber_with_dynamic_args(
                    ["device", &entry.device_id, "targets", "set"],
                    &entry.target_ids,
                );
            }
            if let Some(profile_path) = entry.profile_path {
                if !profile_path.trim().is_empty() {
                    let _ = run_inputplumber(&[
                        "device",
                        &entry.device_id,
                        "profile",
                        "load",
                        &profile_path,
                    ]);
                }
            }
            if let Some(intercept_mode) = entry.intercept_mode {
                let _ = run_inputplumber(&[
                    "device",
                    &entry.device_id,
                    "intercept",
                    "set",
                    &intercept_mode,
                ]);
            }
        }
    }

    pub fn restore_when_process_exits(self, pid: u32) {
        if !self.is_active() {
            return;
        }

        std::thread::spawn(move || {
            while process_exists(pid) {
                std::thread::sleep(Duration::from_millis(750));
            }
            self.restore();
        });
    }
}

const TWO_BUTTON_CLOCKWISE_PROFILE_ID: &str = "two-button-clockwise";

fn resolve_profile_id(
    mapping: &ControllerMappingSettings,
    platform_name: Option<&str>,
    launchbox_db_id: Option<i64>,
) -> Option<String> {
    if let Some(db_id) = launchbox_db_id {
        if let Some(profile_id) = mapping.game_profile_ids.get(&db_id.to_string()) {
            return normalized_optional_string(profile_id);
        }
    }

    if let Some(platform) = platform_name {
        if let Some(profile_id) = mapping.platform_profile_ids.get(platform) {
            return normalized_optional_string(profile_id);
        }
    }

    mapping
        .default_profile_id
        .as_deref()
        .and_then(normalized_optional_string)
}

fn normalized_optional_string(value: &str) -> Option<String> {
    let trimmed = value.trim();
    if trimmed.is_empty() || trimmed == "none" {
        None
    } else {
        Some(trimmed.to_string())
    }
}

fn resolve_profile_path(settings: &AppSettings, profile_id: &str) -> Result<PathBuf, String> {
    if profile_id == TWO_BUTTON_CLOCKWISE_PROFILE_ID {
        write_two_button_clockwise_profile(settings)
    } else {
        let path = PathBuf::from(profile_id);
        if path.is_file() {
            Ok(path)
        } else {
            Err(format!(
                "Controller profile '{}' is not a built-in profile or readable file path.",
                profile_id
            ))
        }
    }
}

fn write_two_button_clockwise_profile(settings: &AppSettings) -> Result<PathBuf, String> {
    let dir = settings.get_data_directory().join("controller-profiles");
    std::fs::create_dir_all(&dir).map_err(|e| {
        format!(
            "Failed to create controller profile directory {}: {}",
            dir.display(),
            e
        )
    })?;
    let path = dir.join("two-button-clockwise.yaml");
    std::fs::write(&path, TWO_BUTTON_CLOCKWISE_PROFILE).map_err(|e| {
        format!(
            "Failed to write controller profile {}: {}",
            path.display(),
            e
        )
    })?;
    Ok(path)
}

const TWO_BUTTON_CLOCKWISE_PROFILE: &str = r#"# yaml-language-server: $schema=https://raw.githubusercontent.com/ShadowBlip/InputPlumber/main/rootfs/usr/share/inputplumber/schema/device_profile_v1.json
version: 1
kind: DeviceProfile
name: 2-button clockwise diamond
description: Maps physical bottom/right face buttons to target left/bottom for NES-style two-button layouts.

mapping:
  - name: Physical South to target West
    source_event:
      gamepad:
        button: South
    target_events:
      - gamepad:
          button: West
  - name: Physical East to target South
    source_event:
      gamepad:
        button: East
    target_events:
      - gamepad:
          button: South
  - name: Physical North to target East
    source_event:
      gamepad:
        button: North
    target_events:
      - gamepad:
          button: East
  - name: Physical West to target North
    source_event:
      gamepad:
        button: West
    target_events:
      - gamepad:
          button: North
"#;

fn normalize_target_ids(output_target: &str) -> Vec<String> {
    let targets = output_target
        .split(',')
        .map(str::trim)
        .filter(|target| !target.is_empty())
        .map(ToOwned::to_owned)
        .collect::<Vec<_>>();

    if targets.is_empty() {
        vec!["xb360".to_string()]
    } else {
        targets
    }
}

fn controller_source_paths(controllers: &[ControllerDevice]) -> HashSet<String> {
    controllers
        .iter()
        .flat_map(|controller| {
            std::iter::once(controller.device_path.clone())
                .chain(controller.event_paths.iter().cloned())
        })
        .collect()
}

fn selected_controller_source_paths(
    controllers: &[ControllerDevice],
    hidden_controller_ids: &[String],
) -> HashSet<String> {
    let selected_ids = hidden_controller_ids
        .iter()
        .map(|id| id.trim())
        .filter(|id| !id.is_empty())
        .collect::<HashSet<_>>();
    if selected_ids.is_empty() {
        return HashSet::new();
    }

    controllers
        .iter()
        .filter(|controller| selected_ids.contains(controller.stable_id.as_str()))
        .flat_map(|controller| {
            std::iter::once(controller.device_path.clone())
                .chain(controller.event_paths.iter().cloned())
        })
        .collect()
}

fn inputplumber_inventory(
    warnings: &mut Vec<String>,
) -> (
    ControllerProviderStatus,
    Vec<InputPlumberCompositeDevice>,
    Vec<InputPlumberTargetDevice>,
) {
    let version = inputplumber_version();
    let available = version.is_some();
    if !available {
        return (
            ControllerProviderStatus {
                provider: "inputplumber".to_string(),
                available: false,
                version: None,
                service_accessible: false,
                message: Some("InputPlumber is not available on PATH.".to_string()),
            },
            Vec::new(),
            Vec::new(),
        );
    }

    let managed_devices = match list_inputplumber_composite_devices() {
        Ok(devices) => devices,
        Err(e) => {
            warnings.push(e.clone());
            Vec::new()
        }
    };
    let supported_targets = match list_inputplumber_supported_targets() {
        Ok(targets) => targets,
        Err(e) => {
            warnings.push(e.clone());
            Vec::new()
        }
    };

    (
        ControllerProviderStatus {
            provider: "inputplumber".to_string(),
            available,
            version,
            service_accessible: !managed_devices.is_empty() || !supported_targets.is_empty(),
            message: None,
        },
        managed_devices,
        supported_targets,
    )
}

fn inputplumber_version() -> Option<String> {
    let output = Command::new("inputplumber")
        .arg("--version")
        .output()
        .ok()?;
    if !output.status.success() {
        return None;
    }
    let stdout = String::from_utf8_lossy(&output.stdout).trim().to_string();
    if stdout.is_empty() {
        None
    } else {
        Some(stdout)
    }
}

fn list_inputplumber_composite_devices() -> Result<Vec<InputPlumberCompositeDevice>, String> {
    let output = run_inputplumber(&["devices", "list"])?;
    let mut devices = Vec::new();

    for row in parse_box_rows(&output) {
        if row.len() < 2 || row[0] == "Id" {
            continue;
        }
        let id = row[0].clone();
        let name = row[1].clone();
        let info = inputplumber_device_info(&id).unwrap_or_default();
        let target_ids = inputplumber_device_targets(&id).unwrap_or_default();
        devices.push(InputPlumberCompositeDevice {
            id,
            name,
            profile_name: info.profile_name,
            profile_path: info.profile_path,
            source_paths: info.source_paths,
            target_ids,
        });
    }

    Ok(devices)
}

#[derive(Default)]
struct InputPlumberDeviceInfo {
    profile_name: Option<String>,
    profile_path: Option<String>,
    source_paths: Vec<String>,
}

fn inputplumber_device_info(id: &str) -> Result<InputPlumberDeviceInfo, String> {
    let output = run_inputplumber(&["device", id, "info"])?;
    let mut info = InputPlumberDeviceInfo::default();

    for row in parse_box_rows(&output) {
        if row.first().map(String::as_str) != Some(id) {
            continue;
        }
        if row.len() >= 3 && row[2] != "Profile Name" && !row[2].is_empty() {
            info.profile_name = Some(row[2].clone());
        }
        if row.len() >= 4 {
            info.source_paths = extract_quoted_paths(&row[3]);
        }
    }

    let profile_path = run_inputplumber(&["device", id, "profile", "path"]).ok();
    info.profile_path = profile_path.and_then(|output| {
        extract_first_path(&output).or_else(|| normalized_optional_string(output.trim()))
    });

    Ok(info)
}

fn inputplumber_device_targets(id: &str) -> Result<Vec<String>, String> {
    let output = run_inputplumber(&["device", id, "targets", "list"])?;
    Ok(parse_box_rows(&output)
        .into_iter()
        .filter_map(|row| {
            if row.len() >= 2 && row[0] != "Id" {
                Some(row[0].clone())
            } else {
                None
            }
        })
        .collect())
}

fn inputplumber_device_intercept_mode(id: &str) -> Result<String, String> {
    let output = run_inputplumber(&["device", id, "intercept", "get"])?;
    output
        .split(':')
        .nth(1)
        .map(str::trim)
        .filter(|mode| !mode.is_empty())
        .map(ToOwned::to_owned)
        .ok_or_else(|| format!("Failed to parse InputPlumber intercept mode for device {id}"))
}

fn list_inputplumber_supported_targets() -> Result<Vec<InputPlumberTargetDevice>, String> {
    let output = run_inputplumber(&["targets", "supported-devices"])?;
    Ok(parse_box_rows(&output)
        .into_iter()
        .filter_map(|row| {
            if row.len() >= 2 && row[0] != "Id" {
                Some(InputPlumberTargetDevice {
                    id: row[0].clone(),
                    name: row[1].clone(),
                })
            } else {
                None
            }
        })
        .collect())
}

fn run_inputplumber(args: &[&str]) -> Result<String, String> {
    run_command("inputplumber", args)
}

fn run_inputplumber_with_dynamic_args<const N: usize>(
    prefix: [&str; N],
    dynamic_args: &[String],
) -> Result<String, String> {
    let args = prefix
        .iter()
        .map(|arg| (*arg).to_string())
        .chain(dynamic_args.iter().cloned())
        .collect::<Vec<_>>();
    let refs = args.iter().map(String::as_str).collect::<Vec<_>>();
    run_inputplumber(&refs)
}

fn run_command(program: &str, args: &[&str]) -> Result<String, String> {
    let output = Command::new(program)
        .args(args)
        .output()
        .map_err(|e| format!("Failed to run {}: {}", program, e))?;

    if output.status.success() {
        Ok(String::from_utf8_lossy(&output.stdout).to_string())
    } else {
        let stderr = String::from_utf8_lossy(&output.stderr).trim().to_string();
        let stdout = String::from_utf8_lossy(&output.stdout).trim().to_string();
        let message = if stderr.is_empty() { stdout } else { stderr };
        Err(format!(
            "{} {} failed: {}",
            program,
            args.join(" "),
            message
        ))
    }
}

fn parse_box_rows(output: &str) -> Vec<Vec<String>> {
    output
        .lines()
        .filter(|line| line.contains('│'))
        .filter_map(|line| {
            let cells = line
                .split('│')
                .skip(1)
                .map(|cell| cell.trim().to_string())
                .filter(|cell| !cell.is_empty())
                .collect::<Vec<_>>();
            if cells.len() >= 2 { Some(cells) } else { None }
        })
        .collect()
}

fn extract_quoted_paths(value: &str) -> Vec<String> {
    let mut paths = Vec::new();
    let mut in_quote = false;
    let mut current = String::new();

    for ch in value.chars() {
        if ch == '"' {
            if in_quote && current.starts_with("/dev/") {
                paths.push(current.clone());
            }
            current.clear();
            in_quote = !in_quote;
            continue;
        }
        if in_quote {
            current.push(ch);
        }
    }

    paths
}

fn extract_first_path(output: &str) -> Option<String> {
    output
        .split_whitespace()
        .find(|part| part.starts_with('/'))
        .map(|part| part.trim_matches('"').to_string())
}

fn list_local_controllers(warnings: &mut Vec<String>) -> Vec<ControllerDevice> {
    if cfg!(target_os = "linux") {
        list_linux_joystick_controllers(warnings)
    } else {
        warnings.push("Controller inventory is currently implemented on Linux only.".to_string());
        Vec::new()
    }
}

fn list_linux_joystick_controllers(warnings: &mut Vec<String>) -> Vec<ControllerDevice> {
    let sys_input = Path::new("/sys/class/input");
    let entries = match std::fs::read_dir(sys_input) {
        Ok(entries) => entries,
        Err(e) => {
            warnings.push(format!("Failed to read {}: {}", sys_input.display(), e));
            return Vec::new();
        }
    };

    let mut devices = Vec::new();
    for entry in entries.flatten() {
        let name = entry.file_name().to_string_lossy().to_string();
        if !name.starts_with("js") {
            continue;
        }
        if let Some(device) = linux_controller_from_js(&name, &entry.path()) {
            if !device.is_virtual {
                devices.push(device);
            }
        }
    }

    devices.sort_by(|a, b| a.device_path.cmp(&b.device_path));
    devices
}

fn linux_controller_from_js(js_name: &str, js_sys_path: &Path) -> Option<ControllerDevice> {
    let device_dir = js_sys_path.join("device");
    let name = read_trimmed(device_dir.join("name")).unwrap_or_else(|| js_name.to_string());
    if is_likely_non_game_controller(&name) {
        return None;
    }
    let vendor_id = read_trimmed(device_dir.join("id/vendor"));
    let product_id = read_trimmed(device_dir.join("id/product"));
    let version = read_trimmed(device_dir.join("id/version"));
    let bus_type = read_trimmed(device_dir.join("id/bustype"));
    let physical_path = read_trimmed(device_dir.join("phys"));
    let unique_id = read_trimmed(device_dir.join("uniq"));
    let device_path = format!("/dev/input/{js_name}");
    let event_paths = linux_input_child_paths(&device_dir, "event");
    let canonical_device_dir = std::fs::canonicalize(&device_dir).ok();
    let stable_id = stable_controller_id(
        &name,
        vendor_id.as_deref(),
        product_id.as_deref(),
        unique_id.as_deref(),
    );
    let is_virtual = is_virtual_linux_controller(
        &name,
        physical_path.as_deref(),
        canonical_device_dir.as_deref(),
        bus_type.as_deref(),
    );

    Some(ControllerDevice {
        stable_id,
        name,
        device_path,
        event_paths,
        vendor_id,
        product_id,
        version,
        bus_type,
        physical_path,
        unique_id,
        is_virtual,
    })
}

fn is_virtual_linux_controller(
    name: &str,
    physical_path: Option<&str>,
    canonical_device_dir: Option<&Path>,
    bus_type: Option<&str>,
) -> bool {
    let lower_name = name.to_ascii_lowercase();
    if lower_name.contains("virtual") || lower_name.contains("inputplumber") {
        return true;
    }

    if physical_path
        .map(|path| path.to_ascii_lowercase().contains("virtual"))
        .unwrap_or(false)
    {
        return true;
    }

    let Some(canonical_device_dir) = canonical_device_dir else {
        return false;
    };
    let canonical = canonical_device_dir.to_string_lossy().to_ascii_lowercase();
    if canonical.contains("/devices/virtual/input/") {
        return true;
    }

    // Bluetooth HID controllers also use UHID, but report the Bluetooth bus
    // type. InputPlumber virtual targets are UHID devices that present as USB.
    if canonical.contains("/devices/virtual/misc/uhid/") {
        return !bus_type
            .map(|value| value.eq_ignore_ascii_case("0005"))
            .unwrap_or(false);
    }

    false
}

fn is_likely_non_game_controller(name: &str) -> bool {
    let lower = name.to_ascii_lowercase();
    lower.contains("keyboard")
        || lower.contains("mouse")
        || lower.contains("pointer")
        || lower.contains("trackpad")
        || lower.contains("touchpad")
        || lower.contains("motion sensors")
        || lower.contains("headset jack")
        || lower.contains("led controller")
        || lower.contains("chakram")
}

fn linux_input_child_paths(device_dir: &Path, prefix: &str) -> Vec<String> {
    let Ok(entries) = std::fs::read_dir(device_dir) else {
        return Vec::new();
    };
    let mut paths = entries
        .flatten()
        .filter_map(|entry| {
            let name = entry.file_name().to_string_lossy().to_string();
            if name.starts_with(prefix) {
                Some(format!("/dev/input/{name}"))
            } else {
                None
            }
        })
        .collect::<Vec<_>>();
    paths.sort();
    paths
}

fn stable_controller_id(
    name: &str,
    vendor_id: Option<&str>,
    product_id: Option<&str>,
    unique_id: Option<&str>,
) -> String {
    let vendor = vendor_id.unwrap_or("0000");
    let product = product_id.unwrap_or("0000");
    let unique = unique_id
        .filter(|value| !value.trim().is_empty())
        .unwrap_or(name);
    format!(
        "linux:{}:{}:{}",
        vendor.to_ascii_lowercase(),
        product.to_ascii_lowercase(),
        slugify_id(unique)
    )
}

fn slugify_id(value: &str) -> String {
    let slug = value
        .chars()
        .map(|ch| {
            if ch.is_ascii_alphanumeric() {
                ch.to_ascii_lowercase()
            } else {
                '-'
            }
        })
        .collect::<String>();
    slug.split('-')
        .filter(|part| !part.is_empty())
        .collect::<Vec<_>>()
        .join("-")
}

fn read_trimmed(path: impl AsRef<Path>) -> Option<String> {
    std::fs::read_to_string(path)
        .ok()
        .map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty())
}

fn process_exists(pid: u32) -> bool {
    if cfg!(target_os = "linux") {
        PathBuf::from(format!("/proc/{pid}")).exists()
    } else {
        false
    }
}
