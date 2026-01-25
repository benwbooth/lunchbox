//! Emulator detection, installation, and launching
//!
//! This module handles:
//! - Detecting if emulators are installed on the system
//! - Installing emulators via package managers (flatpak/winget/homebrew)
//! - Launching games with the appropriate emulator

use crate::db::schema::EmulatorInfo;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use std::process::Command;

/// Emulator with installation status for frontend display
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct EmulatorWithStatus {
    /// The base emulator info from database
    #[serde(flatten)]
    pub info: EmulatorInfo,
    /// Whether the emulator is currently installed
    pub is_installed: bool,
    /// The install method that would be used (flatpak, winget, homebrew)
    pub install_method: Option<String>,
    /// Whether this is a RetroArch core
    pub is_retroarch_core: bool,
    /// Display name (e.g., "RetroArch: mesen" for cores)
    pub display_name: String,
    /// Path to the installed emulator executable
    pub executable_path: Option<String>,
}

/// Progress event for emulator installation/launch
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", tag = "type")]
pub enum LaunchProgress {
    /// Checking if emulator is installed
    CheckingInstallation { emulator_name: String },
    /// Installing RetroArch first (for cores)
    InstallingRetroarch,
    /// Downloading emulator
    Downloading { emulator_name: String },
    /// Installing emulator
    Installing { emulator_name: String },
    /// Installing RetroArch core
    InstallingCore { core_name: String },
    /// Launching emulator
    Launching { emulator_name: String },
    /// Successfully launched
    Launched { emulator_name: String, pid: u32 },
    /// Error occurred
    Error { emulator_name: String, message: String },
}

/// Launch result with process ID or error
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LaunchResult {
    pub success: bool,
    pub pid: Option<u32>,
    pub error: Option<String>,
}

// ============================================================================
// OS Detection
// ============================================================================

/// Get the current OS as a string
pub fn current_os() -> &'static str {
    #[cfg(target_os = "windows")]
    { "Windows" }
    #[cfg(target_os = "macos")]
    { "macOS" }
    #[cfg(target_os = "linux")]
    { "Linux" }
    #[cfg(not(any(target_os = "windows", target_os = "macos", target_os = "linux")))]
    { "Unknown" }
}

/// Get the install method available for the current OS
fn get_install_method(emulator: &EmulatorInfo) -> Option<String> {
    match current_os() {
        "Linux" => {
            if emulator.flatpak_id.is_some() {
                Some("flatpak".to_string())
            } else {
                None
            }
        }
        "Windows" => {
            if emulator.winget_id.is_some() {
                Some("winget".to_string())
            } else {
                None
            }
        }
        "macOS" => {
            if emulator.homebrew_formula.is_some() {
                Some("homebrew".to_string())
            } else {
                None
            }
        }
        _ => None,
    }
}

// ============================================================================
// Installation Detection
// ============================================================================

/// Check if an emulator is installed, returning the path if found
pub fn check_installation(emulator: &EmulatorInfo) -> Option<PathBuf> {
    // If it's a RetroArch core, check for the core specifically
    if let Some(ref core_name) = emulator.retroarch_core {
        return check_retroarch_core_installed(core_name);
    }

    // Otherwise check for standalone emulator
    match current_os() {
        "Linux" => check_linux_installation(emulator),
        "Windows" => check_windows_installation(emulator),
        "macOS" => check_macos_installation(emulator),
        _ => None,
    }
}

/// Check Linux installation (flatpak or native)
fn check_linux_installation(emulator: &EmulatorInfo) -> Option<PathBuf> {
    // First check if installed via flatpak
    if let Some(ref flatpak_id) = emulator.flatpak_id {
        if is_flatpak_installed(flatpak_id) {
            // Return a pseudo-path for flatpak apps
            return Some(PathBuf::from(format!("flatpak::{}", flatpak_id)));
        }
    }

    // Check for native installation via which
    let executable_names = get_executable_names(&emulator.name);
    for name in executable_names {
        if let Ok(path) = which::which(&name) {
            return Some(path);
        }
    }

    None
}

/// Check Windows installation (winget or native)
fn check_windows_installation(emulator: &EmulatorInfo) -> Option<PathBuf> {
    // Check common installation paths first
    let executable_names = get_executable_names(&emulator.name);
    for name in &executable_names {
        // Check PATH
        if let Ok(path) = which::which(name) {
            return Some(path);
        }
    }

    // Check standard install locations
    let program_files = std::env::var("ProgramFiles").unwrap_or_default();
    let program_files_x86 = std::env::var("ProgramFiles(x86)").unwrap_or_default();
    let local_app_data = std::env::var("LOCALAPPDATA").unwrap_or_default();

    for base in [&program_files, &program_files_x86, &local_app_data] {
        if base.is_empty() {
            continue;
        }
        for exe_name in &executable_names {
            let path = PathBuf::from(base)
                .join(&emulator.name)
                .join(format!("{}.exe", exe_name));
            if path.exists() {
                return Some(path);
            }
        }
    }

    None
}

/// Check macOS installation (homebrew cask or native)
fn check_macos_installation(emulator: &EmulatorInfo) -> Option<PathBuf> {
    // Check for .app in /Applications
    let app_name = format!("{}.app", emulator.name);
    let app_path = PathBuf::from("/Applications").join(&app_name);
    if app_path.exists() {
        return Some(app_path);
    }

    // Check homebrew cask location
    let homebrew_path = PathBuf::from("/opt/homebrew/Caskroom")
        .join(emulator.homebrew_formula.as_deref().unwrap_or(&emulator.name.to_lowercase()));
    if homebrew_path.exists() {
        return Some(homebrew_path);
    }

    // Check for CLI tool in PATH
    let executable_names = get_executable_names(&emulator.name);
    for name in executable_names {
        if let Ok(path) = which::which(&name) {
            return Some(path);
        }
    }

    None
}

/// Check if a RetroArch core is installed
fn check_retroarch_core_installed(core_name: &str) -> Option<PathBuf> {
    // First check if RetroArch itself is installed
    if !is_retroarch_installed() {
        return None;
    }

    let core_dirs = get_retroarch_core_dirs();
    let core_filename = get_core_filename(core_name);

    for dir in core_dirs {
        let core_path = dir.join(&core_filename);
        if core_path.exists() {
            return Some(core_path);
        }
    }

    None
}

/// Check if RetroArch is installed
fn is_retroarch_installed() -> bool {
    match current_os() {
        "Linux" => {
            // Check flatpak first
            if is_flatpak_installed("org.libretro.RetroArch") {
                return true;
            }
            // Check native
            which::which("retroarch").is_ok()
        }
        "Windows" => {
            which::which("retroarch").is_ok() || which::which("retroarch.exe").is_ok()
        }
        "macOS" => {
            PathBuf::from("/Applications/RetroArch.app").exists()
                || which::which("retroarch").is_ok()
        }
        _ => false,
    }
}

/// Check if a flatpak app is installed
fn is_flatpak_installed(app_id: &str) -> bool {
    #[cfg(target_os = "linux")]
    {
        Command::new("flatpak")
            .args(["info", app_id])
            .output()
            .map(|o| o.status.success())
            .unwrap_or(false)
    }
    #[cfg(not(target_os = "linux"))]
    {
        let _ = app_id;
        false
    }
}

/// Get possible executable names for an emulator
fn get_executable_names(name: &str) -> Vec<String> {
    let lower = name.to_lowercase();
    let mut names = vec![lower.clone()];

    // Add common variations
    match lower.as_str() {
        "dolphin" => {
            names.extend(["dolphin-emu", "dolphin-emu-qt"].iter().map(|s| s.to_string()));
        }
        "ppsspp" => {
            names.extend(["PPSSPP", "PPSSPPQt"].iter().map(|s| s.to_string()));
        }
        "duckstation" => {
            names.extend(["duckstation-qt", "duckstation-nogui"].iter().map(|s| s.to_string()));
        }
        "mesen" => {
            names.extend(["Mesen", "mesen-x"].iter().map(|s| s.to_string()));
        }
        "ares" => {
            names.push("ares-emu".to_string());
        }
        _ => {}
    }

    names
}

/// Get RetroArch core directories for the current OS
fn get_retroarch_core_dirs() -> Vec<PathBuf> {
    let mut dirs = Vec::new();

    match current_os() {
        "Linux" => {
            // Flatpak RetroArch core location
            if let Some(home) = dirs::home_dir() {
                dirs.push(home.join(".var/app/org.libretro.RetroArch/config/retroarch/cores"));
            }
            // System-wide cores
            dirs.push(PathBuf::from("/usr/lib/libretro"));
            dirs.push(PathBuf::from("/usr/lib64/libretro"));
            // User cores
            if let Some(home) = dirs::home_dir() {
                dirs.push(home.join(".config/retroarch/cores"));
            }
        }
        "Windows" => {
            if let Some(app_data) = dirs::data_local_dir() {
                dirs.push(app_data.join("RetroArch").join("cores"));
            }
            // Also check program files
            if let Ok(pf) = std::env::var("ProgramFiles") {
                dirs.push(PathBuf::from(pf).join("RetroArch").join("cores"));
            }
        }
        "macOS" => {
            dirs.push(PathBuf::from("/Applications/RetroArch.app/Contents/Resources/cores"));
            if let Some(home) = dirs::home_dir() {
                dirs.push(home.join("Library/Application Support/RetroArch/cores"));
            }
        }
        _ => {}
    }

    dirs
}

/// Get the core filename for a core name
fn get_core_filename(core_name: &str) -> String {
    match current_os() {
        "Windows" => format!("{}_libretro.dll", core_name),
        "macOS" => format!("{}_libretro.dylib", core_name),
        _ => format!("{}_libretro.so", core_name),
    }
}

// ============================================================================
// Installation
// ============================================================================

/// Install an emulator using the appropriate package manager
/// If `as_retroarch_core` is true, install as a RetroArch core; otherwise install standalone
pub async fn install_emulator(emulator: &EmulatorInfo, as_retroarch_core: bool) -> Result<PathBuf, String> {
    tracing::info!(
        emulator = %emulator.name,
        as_retroarch_core = as_retroarch_core,
        os = current_os(),
        "Installing emulator"
    );

    // If installing as RetroArch core, use that path
    if as_retroarch_core {
        if let Some(ref core_name) = emulator.retroarch_core {
            tracing::info!(core_name = core_name, "Installing as RetroArch core");
            return install_retroarch_core(core_name).await;
        } else {
            let err = format!("{} does not have a RetroArch core", emulator.name);
            tracing::error!(error = %err);
            return Err(err);
        }
    }

    // Install standalone version
    let result = match current_os() {
        "Linux" => {
            if let Some(ref flatpak_id) = emulator.flatpak_id {
                tracing::info!(flatpak_id = flatpak_id, "Installing via flatpak");
                install_flatpak(flatpak_id).await?;
                Ok(PathBuf::from(format!("flatpak::{}", flatpak_id)))
            } else {
                Err(format!("No installation method available for {} on Linux", emulator.name))
            }
        }
        "Windows" => {
            if let Some(ref winget_id) = emulator.winget_id {
                tracing::info!(winget_id = winget_id, "Installing via winget");
                install_winget(winget_id).await?;
                // Try to find the installed executable
                check_windows_installation(emulator)
                    .ok_or_else(|| format!("Installed {} but could not find executable", emulator.name))
            } else {
                Err(format!("No installation method available for {} on Windows", emulator.name))
            }
        }
        "macOS" => {
            if let Some(ref formula) = emulator.homebrew_formula {
                tracing::info!(formula = formula, "Installing via homebrew");
                install_homebrew(formula).await?;
                check_macos_installation(emulator)
                    .ok_or_else(|| format!("Installed {} but could not find application", emulator.name))
            } else {
                Err(format!("No installation method available for {} on macOS", emulator.name))
            }
        }
        _ => Err("Unsupported operating system".to_string()),
    };

    match &result {
        Ok(path) => tracing::info!(path = ?path, "Emulator installed successfully"),
        Err(e) => tracing::error!(error = %e, "Failed to install emulator"),
    }

    result
}

/// Install a flatpak package
async fn install_flatpak(app_id: &str) -> Result<(), String> {
    #[cfg(target_os = "linux")]
    {
        let output = tokio::process::Command::new("flatpak")
            .args(["install", "-y", "flathub", app_id])
            .output()
            .await
            .map_err(|e| format!("Failed to run flatpak: {}", e))?;

        if output.status.success() {
            Ok(())
        } else {
            let stderr = String::from_utf8_lossy(&output.stderr);
            Err(format!("Flatpak install failed: {}", stderr))
        }
    }
    #[cfg(not(target_os = "linux"))]
    {
        let _ = app_id;
        Err("Flatpak is only available on Linux".to_string())
    }
}

/// Install via winget
async fn install_winget(winget_id: &str) -> Result<(), String> {
    #[cfg(target_os = "windows")]
    {
        let output = tokio::process::Command::new("winget")
            .args(["install", "--accept-package-agreements", "--accept-source-agreements", "-e", "--id", winget_id])
            .output()
            .await
            .map_err(|e| format!("Failed to run winget: {}", e))?;

        if output.status.success() {
            Ok(())
        } else {
            let stderr = String::from_utf8_lossy(&output.stderr);
            Err(format!("Winget install failed: {}", stderr))
        }
    }
    #[cfg(not(target_os = "windows"))]
    {
        let _ = winget_id;
        Err("Winget is only available on Windows".to_string())
    }
}

/// Install via homebrew
async fn install_homebrew(formula: &str) -> Result<(), String> {
    #[cfg(target_os = "macos")]
    {
        let output = tokio::process::Command::new("brew")
            .args(["install", "--cask", formula])
            .output()
            .await
            .map_err(|e| format!("Failed to run brew: {}", e))?;

        if output.status.success() {
            Ok(())
        } else {
            let stderr = String::from_utf8_lossy(&output.stderr);
            Err(format!("Homebrew install failed: {}", stderr))
        }
    }
    #[cfg(not(target_os = "macos"))]
    {
        let _ = formula;
        Err("Homebrew is only available on macOS".to_string())
    }
}

/// Install a RetroArch core
async fn install_retroarch_core(core_name: &str) -> Result<PathBuf, String> {
    // First ensure RetroArch is installed
    if !is_retroarch_installed() {
        install_retroarch().await?;
    }

    // Download the core from libretro buildbot
    let core_url = get_libretro_core_url(core_name);
    let core_dirs = get_retroarch_core_dirs();
    let core_dir = core_dirs.first()
        .ok_or_else(|| "Could not determine RetroArch cores directory".to_string())?;

    // Create cores directory if it doesn't exist
    tokio::fs::create_dir_all(core_dir)
        .await
        .map_err(|e| format!("Failed to create cores directory: {}", e))?;

    let core_filename = get_core_filename(core_name);
    let core_path = core_dir.join(&core_filename);
    let zip_filename = format!("{}.zip", core_filename.trim_end_matches(".so").trim_end_matches(".dll").trim_end_matches(".dylib"));
    let zip_path = core_dir.join(&zip_filename);

    // Download the core zip
    let client = reqwest::Client::new();
    let response = client
        .get(&core_url)
        .send()
        .await
        .map_err(|e| format!("Failed to download core: {}", e))?;

    if !response.status().is_success() {
        return Err(format!("Failed to download core: HTTP {}", response.status()));
    }

    let bytes = response.bytes().await
        .map_err(|e| format!("Failed to read core data: {}", e))?;

    // Write zip file
    tokio::fs::write(&zip_path, &bytes)
        .await
        .map_err(|e| format!("Failed to write core zip: {}", e))?;

    // Extract the core
    let zip_path_clone = zip_path.clone();
    let core_dir_clone = core_dir.clone();
    tokio::task::spawn_blocking(move || {
        let file = std::fs::File::open(&zip_path_clone)
            .map_err(|e| format!("Failed to open zip: {}", e))?;
        let mut archive = zip::ZipArchive::new(file)
            .map_err(|e| format!("Failed to read zip: {}", e))?;
        archive.extract(&core_dir_clone)
            .map_err(|e| format!("Failed to extract zip: {}", e))?;
        Ok::<_, String>(())
    })
    .await
    .map_err(|e| format!("Task failed: {}", e))??;

    // Clean up zip file
    let _ = tokio::fs::remove_file(&zip_path).await;

    if core_path.exists() {
        Ok(core_path)
    } else {
        Err(format!("Core file not found after extraction: {}", core_filename))
    }
}

/// Install RetroArch itself
async fn install_retroarch() -> Result<(), String> {
    match current_os() {
        "Linux" => install_flatpak("org.libretro.RetroArch").await,
        "Windows" => install_winget("Libretro.RetroArch").await,
        "macOS" => install_homebrew("retroarch").await,
        _ => Err("Unsupported operating system".to_string()),
    }
}

/// Get the libretro buildbot URL for a core
fn get_libretro_core_url(core_name: &str) -> String {
    let (platform, ext) = match current_os() {
        "Linux" => ("linux/x86_64", "so"),
        "Windows" => ("windows/x86_64", "dll"),
        "macOS" => ("apple/osx/x86_64", "dylib"),
        _ => ("linux/x86_64", "so"),
    };

    format!(
        "https://buildbot.libretro.com/nightly/{}/latest/{}_libretro.{}.zip",
        platform, core_name, ext
    )
}

// ============================================================================
// Launching
// ============================================================================

/// Launch an emulator (optionally with a ROM)
/// If `as_retroarch_core` is true, launch via RetroArch; otherwise launch standalone
pub fn launch_emulator(emulator: &EmulatorInfo, rom_path: Option<&str>, as_retroarch_core: bool) -> Result<u32, String> {
    tracing::info!(
        emulator = %emulator.name,
        rom = ?rom_path,
        as_retroarch_core = as_retroarch_core,
        "Launching emulator"
    );

    let result = if as_retroarch_core {
        if let Some(ref core_name) = emulator.retroarch_core {
            launch_retroarch(core_name, rom_path)
        } else {
            let err = format!("{} does not have a RetroArch core", emulator.name);
            tracing::error!(error = %err);
            return Err(err);
        }
    } else {
        launch_standalone(emulator, rom_path)
    };

    match &result {
        Ok(pid) => tracing::info!(pid = pid, "Emulator launched successfully"),
        Err(e) => tracing::error!(error = %e, "Failed to launch emulator"),
    }

    result
}

/// Launch a game with an emulator (legacy wrapper)
/// Defaults to using RetroArch core if available
pub fn launch_game(emulator: &EmulatorInfo, rom_path: &str) -> Result<u32, String> {
    let as_retroarch_core = emulator.retroarch_core.is_some();
    launch_emulator(emulator, Some(rom_path), as_retroarch_core)
}

/// Launch RetroArch with a specific core (optionally with a ROM)
fn launch_retroarch(core_name: &str, rom_path: Option<&str>) -> Result<u32, String> {
    tracing::debug!(core_name = core_name, rom = ?rom_path, "Launching RetroArch");

    let core_path = check_retroarch_core_installed(core_name);
    tracing::debug!(core_path = ?core_path, "Core installation check");

    let core_arg = core_path
        .as_ref()
        .and_then(|p| p.to_str())
        .unwrap_or(core_name);

    let child = match current_os() {
        "Linux" => {
            // Try flatpak first
            let is_flatpak = is_flatpak_installed("org.libretro.RetroArch");
            tracing::debug!(flatpak_installed = is_flatpak, "Checking RetroArch installation");

            if is_flatpak {
                let mut cmd = Command::new("flatpak");
                cmd.arg("run").arg("org.libretro.RetroArch");
                if core_path.is_some() {
                    cmd.arg("-L").arg(core_arg);
                }
                if let Some(rom) = rom_path {
                    cmd.arg(rom);
                }
                tracing::info!(command = ?cmd, "Spawning RetroArch via flatpak");
                cmd.spawn()
                    .map_err(|e| format!("Failed to launch RetroArch via flatpak: {}", e))?
            } else {
                let mut cmd = Command::new("retroarch");
                if core_path.is_some() {
                    cmd.arg("-L").arg(core_arg);
                }
                if let Some(rom) = rom_path {
                    cmd.arg(rom);
                }
                tracing::info!(command = ?cmd, "Spawning RetroArch native");
                cmd.spawn()
                    .map_err(|e| format!("Failed to launch RetroArch: {}", e))?
            }
        }
        "Windows" => {
            let retroarch_path = which::which("retroarch")
                .or_else(|_| which::which("retroarch.exe"))
                .map_err(|_| "Could not find RetroArch executable")?;
            let mut cmd = Command::new(retroarch_path);
            if core_path.is_some() {
                cmd.arg("-L").arg(core_arg);
            }
            if let Some(rom) = rom_path {
                cmd.arg(rom);
            }
            cmd.spawn()
                .map_err(|e| format!("Failed to launch RetroArch: {}", e))?
        }
        "macOS" => {
            let mut cmd = Command::new("open");
            cmd.arg("-a").arg("RetroArch");
            if core_path.is_some() || rom_path.is_some() {
                cmd.arg("--args");
                if core_path.is_some() {
                    cmd.arg("-L").arg(core_arg);
                }
                if let Some(rom) = rom_path {
                    cmd.arg(rom);
                }
            }
            cmd.spawn()
                .map_err(|e| format!("Failed to launch RetroArch: {}", e))?
        }
        _ => return Err("Unsupported operating system".to_string()),
    };

    Ok(child.id())
}

/// Launch a standalone emulator (optionally with a ROM)
fn launch_standalone(emulator: &EmulatorInfo, rom_path: Option<&str>) -> Result<u32, String> {
    tracing::debug!(emulator = %emulator.name, rom = ?rom_path, "Launching standalone emulator");

    let exe_path = check_installation(emulator)
        .ok_or_else(|| {
            let err = format!("{} is not installed", emulator.name);
            tracing::error!(error = %err, "Emulator not installed");
            err
        })?;

    tracing::debug!(exe_path = ?exe_path, "Found emulator executable");

    let child = if exe_path.to_string_lossy().starts_with("flatpak::") {
        // Flatpak app
        let app_id = exe_path.to_string_lossy().replace("flatpak::", "");
        let mut cmd = Command::new("flatpak");
        cmd.arg("run").arg(&app_id);
        if let Some(rom) = rom_path {
            cmd.arg(rom);
        }
        tracing::info!(command = ?cmd, app_id = %app_id, "Spawning via flatpak");
        cmd.spawn()
            .map_err(|e| {
                let err = format!("Failed to launch via flatpak: {}", e);
                tracing::error!(error = %err);
                err
            })?
    } else if current_os() == "macOS" && exe_path.extension().map(|e| e == "app").unwrap_or(false) {
        // macOS .app bundle
        let mut cmd = Command::new("open");
        cmd.arg("-a").arg(exe_path.to_str().unwrap_or_default());
        if let Some(rom) = rom_path {
            cmd.arg(rom);
        }
        tracing::info!(command = ?cmd, "Spawning macOS app");
        cmd.spawn()
            .map_err(|e| {
                let err = format!("Failed to launch {}: {}", emulator.name, e);
                tracing::error!(error = %err);
                err
            })?
    } else {
        // Regular executable
        let mut cmd = Command::new(&exe_path);
        if let Some(rom) = rom_path {
            cmd.arg(rom);
        }
        tracing::info!(command = ?cmd, "Spawning native executable");
        cmd.spawn()
            .map_err(|e| {
                let err = format!("Failed to launch {}: {}", emulator.name, e);
                tracing::error!(error = %err);
                err
            })?
    };

    Ok(child.id())
}

// ============================================================================
// Emulator Status Helpers
// ============================================================================

/// Add installation status to an emulator
pub fn add_status(emulator: EmulatorInfo) -> EmulatorWithStatus {
    let is_retroarch_core = emulator.retroarch_core.is_some();
    let install_path = check_installation(&emulator);
    let is_installed = install_path.is_some();
    let install_method = get_install_method(&emulator);

    let display_name = if let Some(ref core) = emulator.retroarch_core {
        format!("RetroArch: {}", core)
    } else {
        emulator.name.clone()
    };

    EmulatorWithStatus {
        info: emulator,
        is_installed,
        install_method,
        is_retroarch_core,
        display_name,
        executable_path: install_path.map(|p| p.to_string_lossy().to_string()),
    }
}

/// Add status for an emulator as a RetroArch core entry
pub fn add_status_as_retroarch(emulator: EmulatorInfo) -> EmulatorWithStatus {
    let core_name = emulator.retroarch_core.as_ref()
        .expect("add_status_as_retroarch called on emulator without retroarch_core");

    let display_name = format!("RetroArch: {}", core_name);
    let core_path = check_retroarch_core_installed(core_name);
    let is_installed = core_path.is_some();

    EmulatorWithStatus {
        info: emulator,
        is_installed,
        install_method: Some("retroarch".to_string()),
        is_retroarch_core: true,
        display_name,
        executable_path: if is_installed {
            find_retroarch_executable().map(|p| p.to_string_lossy().to_string())
        } else {
            None
        },
    }
}

/// Add status for an emulator as a standalone entry (ignoring RetroArch core)
pub fn add_status_as_standalone(emulator: EmulatorInfo) -> EmulatorWithStatus {
    let install_method = get_install_method(&emulator);
    let install_path = check_standalone_installation(&emulator);
    let is_installed = install_path.is_some();
    let display_name = emulator.name.clone();

    EmulatorWithStatus {
        info: emulator,
        is_installed,
        install_method,
        is_retroarch_core: false,
        display_name,
        executable_path: install_path.map(|p| p.to_string_lossy().to_string()),
    }
}

/// Find the RetroArch executable
fn find_retroarch_executable() -> Option<PathBuf> {
    // Check flatpak first
    if Command::new("flatpak")
        .args(["info", "org.libretro.RetroArch"])
        .output()
        .map(|o| o.status.success())
        .unwrap_or(false)
    {
        return Some(PathBuf::from("flatpak run org.libretro.RetroArch"));
    }

    // Check PATH
    if let Ok(path) = which::which("retroarch") {
        return Some(path);
    }

    None
}

/// Check if standalone emulator is installed (not via RetroArch)
fn check_standalone_installation(emulator: &EmulatorInfo) -> Option<PathBuf> {
    match current_os() {
        "Linux" => {
            if let Some(ref flatpak_id) = emulator.flatpak_id {
                // Check if flatpak app is installed
                if Command::new("flatpak")
                    .args(["info", flatpak_id])
                    .output()
                    .map(|o| o.status.success())
                    .unwrap_or(false)
                {
                    return Some(PathBuf::from(format!("flatpak run {}", flatpak_id)));
                }
            }
        }
        "Windows" => {
            if emulator.winget_id.is_some() {
                // Check common install locations or PATH
                if let Ok(path) = which::which(&emulator.name.to_lowercase()) {
                    return Some(path);
                }
            }
        }
        "macOS" => {
            if emulator.homebrew_formula.is_some() {
                // Check PATH or /Applications
                if let Ok(path) = which::which(&emulator.name.to_lowercase()) {
                    return Some(path);
                }
            }
        }
        _ => {}
    }
    None
}

/// Filter emulators to only those that can be installed on this OS
pub fn filter_installable(emulators: Vec<EmulatorInfo>) -> Vec<EmulatorInfo> {
    let os = current_os();

    emulators
        .into_iter()
        .filter(|e| {
            // Keep if it's a RetroArch core (we can install RA on any supported OS)
            if e.retroarch_core.is_some() {
                return true;
            }
            // Keep if we have an install method for this OS
            match os {
                "Linux" => e.flatpak_id.is_some(),
                "Windows" => e.winget_id.is_some(),
                "macOS" => e.homebrew_formula.is_some(),
                _ => false,
            }
        })
        .collect()
}
