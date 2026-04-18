//! Emulator detection, installation, and launching
//!
//! This module handles:
//! - Detecting if emulators are installed on the system
//! - Installing emulators via package managers (flatpak/winget/homebrew)
//! - Launching games with the appropriate emulator

use crate::db::schema::EmulatorInfo;
use crate::firmware::FirmwareStatus;
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet, HashSet};
use std::io::{Read, Write};
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

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
    /// Firmware requirements/status for this runtime, if any
    #[serde(default)]
    pub firmware_statuses: Vec<FirmwareStatus>,
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
    Error {
        emulator_name: String,
        message: String,
    },
}

/// Launch result with process ID or error
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LaunchResult {
    pub success: bool,
    pub pid: Option<u32>,
    pub error: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum LaunchArg {
    Literal(String),
    Path(String),
}

#[derive(Debug, Default)]
struct PreparedRomLaunch {
    rom_path: Option<String>,
    cleanup_paths: Vec<PathBuf>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ArchiveKind {
    Zip,
    SevenZip,
    Rar,
    Tar,
    TarGz,
    TarBz2,
    TarXz,
    Gz,
    Bz2,
    Xz,
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
struct ArchiveEntry {
    path: PathBuf,
    is_dir: bool,
}

// ============================================================================
// OS Detection
// ============================================================================

/// Get the current OS as a string
pub fn current_os() -> &'static str {
    #[cfg(target_os = "windows")]
    {
        "Windows"
    }
    #[cfg(target_os = "macos")]
    {
        "macOS"
    }
    #[cfg(target_os = "linux")]
    {
        "Linux"
    }
    #[cfg(not(any(target_os = "windows", target_os = "macos", target_os = "linux")))]
    {
        "Unknown"
    }
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
    check_standalone_installation(emulator)
}

/// Check for standalone emulator installation (ignores RetroArch core)
fn check_standalone_installation(emulator: &EmulatorInfo) -> Option<PathBuf> {
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
    let homebrew_path = PathBuf::from("/opt/homebrew/Caskroom").join(
        emulator
            .homebrew_formula
            .as_deref()
            .unwrap_or(&emulator.name.to_lowercase()),
    );
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
        "Windows" => which::which("retroarch").is_ok() || which::which("retroarch.exe").is_ok(),
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
            names.extend(
                ["dolphin-emu", "dolphin-emu-qt"]
                    .iter()
                    .map(|s| s.to_string()),
            );
        }
        "ppsspp" => {
            names.extend(["PPSSPP", "PPSSPPQt"].iter().map(|s| s.to_string()));
        }
        "duckstation" => {
            names.extend(
                ["duckstation-qt", "duckstation-nogui"]
                    .iter()
                    .map(|s| s.to_string()),
            );
        }
        "mesen" => {
            names.extend(["Mesen", "mesen-x"].iter().map(|s| s.to_string()));
        }
        "ares" => {
            names.push("ares-emu".to_string());
        }
        "pcbox" => {
            names.extend(["PCBox", "pcbox"].iter().map(|s| s.to_string()));
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
            dirs.push(PathBuf::from(
                "/Applications/RetroArch.app/Contents/Resources/cores",
            ));
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
pub async fn install_emulator(
    emulator: &EmulatorInfo,
    as_retroarch_core: bool,
) -> Result<PathBuf, String> {
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
                Err(format!(
                    "No installation method available for {} on Linux",
                    emulator.name
                ))
            }
        }
        "Windows" => {
            if let Some(ref winget_id) = emulator.winget_id {
                tracing::info!(winget_id = winget_id, "Installing via winget");
                install_winget(winget_id).await?;
                // Try to find the installed executable
                check_windows_installation(emulator).ok_or_else(|| {
                    format!("Installed {} but could not find executable", emulator.name)
                })
            } else {
                Err(format!(
                    "No installation method available for {} on Windows",
                    emulator.name
                ))
            }
        }
        "macOS" => {
            if let Some(ref formula) = emulator.homebrew_formula {
                tracing::info!(formula = formula, "Installing via homebrew");
                install_homebrew(formula).await?;
                check_macos_installation(emulator).ok_or_else(|| {
                    format!("Installed {} but could not find application", emulator.name)
                })
            } else {
                Err(format!(
                    "No installation method available for {} on macOS",
                    emulator.name
                ))
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
            .args([
                "install",
                "--accept-package-agreements",
                "--accept-source-agreements",
                "-e",
                "--id",
                winget_id,
            ])
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
    let core_dir = core_dirs
        .first()
        .ok_or_else(|| "Could not determine RetroArch cores directory".to_string())?;

    // Create cores directory if it doesn't exist
    tokio::fs::create_dir_all(core_dir)
        .await
        .map_err(|e| format!("Failed to create cores directory: {}", e))?;

    let core_filename = get_core_filename(core_name);
    let core_path = core_dir.join(&core_filename);
    let zip_filename = format!(
        "{}.zip",
        core_filename
            .trim_end_matches(".so")
            .trim_end_matches(".dll")
            .trim_end_matches(".dylib")
    );
    let zip_path = core_dir.join(&zip_filename);

    // Download the core zip
    let client = reqwest::Client::new();
    let response = client
        .get(&core_url)
        .send()
        .await
        .map_err(|e| format!("Failed to download core: {}", e))?;

    if !response.status().is_success() {
        return Err(format!(
            "Failed to download core: HTTP {}",
            response.status()
        ));
    }

    let bytes = response
        .bytes()
        .await
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
        let mut archive =
            zip::ZipArchive::new(file).map_err(|e| format!("Failed to read zip: {}", e))?;
        archive
            .extract(&core_dir_clone)
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
        Err(format!(
            "Core file not found after extraction: {}",
            core_filename
        ))
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

/// Spawn a command and check if it crashes immediately.
/// Returns the pid on success, or an error message if it fails/crashes.
fn spawn_and_verify(
    mut cmd: Command,
    name: &str,
    cleanup_paths: Vec<PathBuf>,
) -> Result<u32, String> {
    // Capture stdout/stderr so we can provide useful diagnostics when startup fails.
    cmd.stderr(Stdio::piped());
    cmd.stdout(Stdio::piped());

    let mut child = cmd
        .spawn()
        .map_err(|e| format!("Failed to launch {}: {}", name, e))?;

    let pid = child.id();

    // Wait a brief moment to see if the process crashes immediately
    std::thread::sleep(std::time::Duration::from_millis(500));

    // Check if the process is still running
    match child.try_wait() {
        Ok(Some(status)) => {
            // Process exited - capture available output for diagnostics.
            let stdout_output = if let Some(mut stdout) = child.stdout.take() {
                let mut buf = Vec::new();
                let _ = stdout.read_to_end(&mut buf);
                String::from_utf8_lossy(&buf).to_string()
            } else {
                String::new()
            };

            let stderr_output = if let Some(mut stderr) = child.stderr.take() {
                let mut buf = Vec::new();
                let _ = stderr.read_to_end(&mut buf);
                String::from_utf8_lossy(&buf).to_string()
            } else {
                String::new()
            };

            let mut combined = String::new();
            if !stdout_output.trim().is_empty() {
                combined.push_str(stdout_output.trim());
            }
            if !stderr_output.trim().is_empty() {
                if !combined.is_empty() {
                    combined.push('\n');
                }
                combined.push_str(stderr_output.trim());
            }

            let mut error_msg = format!("{} exited immediately with status: {}", name, status);

            if name == "RetroArch" {
                let hint = retroarch_startup_hint(&combined);
                if !hint.is_empty() {
                    error_msg.push_str("\n");
                    error_msg.push_str(&hint);
                }
            }

            let tail = tail_lines(&combined, 14);
            if !tail.is_empty() {
                error_msg.push_str("\n\nProcess output:\n");
                error_msg.push_str(&tail);
            }

            tracing::error!(error = %error_msg, "Process crashed immediately");
            cleanup_extracted_roms(&cleanup_paths);
            Err(error_msg)
        }
        Ok(None) => {
            tracing::debug!(pid = pid, "Process is running");
            std::thread::spawn(move || {
                if let Some(mut stdout) = child.stdout.take() {
                    std::thread::spawn(move || {
                        let mut sink = std::io::sink();
                        let _ = std::io::copy(&mut stdout, &mut sink);
                    });
                }
                if let Some(mut stderr) = child.stderr.take() {
                    std::thread::spawn(move || {
                        let mut sink = std::io::sink();
                        let _ = std::io::copy(&mut stderr, &mut sink);
                    });
                }

                let _ = child.wait();
                cleanup_extracted_roms(&cleanup_paths);
            });
            Ok(pid)
        }
        Err(e) => {
            tracing::error!(error = %e, "Failed to check process status");
            cleanup_extracted_roms(&cleanup_paths);
            Err(format!("Failed to check {} status: {}", name, e))
        }
    }
}

fn tail_lines(text: &str, max_lines: usize) -> String {
    let lines: Vec<&str> = text
        .lines()
        .map(str::trim_end)
        .filter(|line| !line.is_empty())
        .collect();
    if lines.is_empty() {
        return String::new();
    }
    let start = lines.len().saturating_sub(max_lines);
    lines[start..].join("\n")
}

fn retroarch_startup_hint(output: &str) -> String {
    let lower = output.to_ascii_lowercase();

    if lower.contains("not an ines file") && lower.contains("failed to load content") {
        if lower.contains("fds bios rom image missing") {
            return "RetroArch could not load this ROM as NES content and then attempted FDS fallback (disksys.rom missing). This often means the file format is incompatible (for example unheadered .unh dumps). Try importing a headered .nes/.unf/.unif ROM.".to_string();
        }
        return "RetroArch could not load this ROM content. This often means the file format is incompatible with the selected core (for example unheadered .unh dumps). Try importing a headered .nes/.unf/.unif ROM.".to_string();
    }

    if lower.contains("failed to load content") {
        return "RetroArch failed to load the ROM content. Verify the ROM file is valid for the selected core.".to_string();
    }

    if lower.contains("failed to load dynamic libretro core")
        || lower.contains("core is not installed")
    {
        return "RetroArch failed to load the selected core. Reinstall the core and try again."
            .to_string();
    }

    String::new()
}

fn cleanup_extracted_roms(paths: &[PathBuf]) {
    for path in paths {
        if let Err(error) = std::fs::remove_file(path) {
            if error.kind() != std::io::ErrorKind::NotFound {
                tracing::warn!(path = %path.display(), error = %error, "Failed to remove extracted ROM");
            }
        }
    }
}

fn prepare_rom_for_launch(rom_path: Option<&str>) -> Result<PreparedRomLaunch, String> {
    let Some(rom_path) = rom_path else {
        return Ok(PreparedRomLaunch::default());
    };

    let path = Path::new(rom_path);
    let Some(kind) = archive_kind_for_path(path) else {
        return Ok(PreparedRomLaunch {
            rom_path: Some(rom_path.to_string()),
            cleanup_paths: Vec::new(),
        });
    };

    extract_launchable_content_from_archive(path, kind)
}

fn archive_kind_for_path(path: &Path) -> Option<ArchiveKind> {
    let name = path.file_name()?.to_string_lossy().to_ascii_lowercase();
    if name.ends_with(".tar.gz") || name.ends_with(".tgz") {
        return Some(ArchiveKind::TarGz);
    }
    if name.ends_with(".tar.bz2") || name.ends_with(".tbz2") || name.ends_with(".tbz") {
        return Some(ArchiveKind::TarBz2);
    }
    if name.ends_with(".tar.xz") || name.ends_with(".txz") {
        return Some(ArchiveKind::TarXz);
    }

    match path
        .extension()
        .and_then(|ext| ext.to_str())
        .map(|ext| ext.to_ascii_lowercase())
        .as_deref()
    {
        Some("zip") => Some(ArchiveKind::Zip),
        Some("7z") => Some(ArchiveKind::SevenZip),
        Some("rar") => Some(ArchiveKind::Rar),
        Some("tar") => Some(ArchiveKind::Tar),
        Some("gz") => Some(ArchiveKind::Gz),
        Some("bz2") => Some(ArchiveKind::Bz2),
        Some("xz") => Some(ArchiveKind::Xz),
        _ => None,
    }
}

fn extract_launchable_content_from_archive(
    archive_path: &Path,
    kind: ArchiveKind,
) -> Result<PreparedRomLaunch, String> {
    match kind {
        ArchiveKind::Gz | ArchiveKind::Bz2 | ArchiveKind::Xz => {
            return extract_single_file_archive(archive_path, kind);
        }
        _ => {}
    }

    let parent_dir = archive_path.parent().ok_or_else(|| {
        format!(
            "ROM archive {} does not have a parent directory",
            archive_path.display()
        )
    })?;

    let entries = list_archive_entries(archive_path, kind)?;
    let launch_entry = select_launch_entry(&entries).ok_or_else(|| {
        format!(
            "ROM archive {} does not contain a supported ROM file",
            archive_path.display()
        )
    })?;
    let selected_entries =
        collect_entries_needed_for_launch(archive_path, kind, &entries, &launch_entry)?;
    let extracted_paths =
        extract_archive_entries(archive_path, kind, parent_dir, &selected_entries)?;
    let launch_path = if selected_entries.len() == 1 {
        extracted_paths[0].clone()
    } else {
        parent_dir.join(&launch_entry)
    };

    tracing::info!(
        archive = %archive_path.display(),
        launch = %launch_path.display(),
        extracted_count = extracted_paths.len(),
        "Prepared archive-backed ROM launch"
    );

    Ok(PreparedRomLaunch {
        rom_path: Some(launch_path.to_string_lossy().to_string()),
        cleanup_paths: extracted_paths,
    })
}

fn list_archive_entries(
    archive_path: &Path,
    kind: ArchiveKind,
) -> Result<Vec<ArchiveEntry>, String> {
    match kind {
        ArchiveKind::Zip => list_zip_entries(archive_path),
        ArchiveKind::SevenZip | ArchiveKind::Rar => list_7z_entries(archive_path),
        ArchiveKind::Tar | ArchiveKind::TarGz | ArchiveKind::TarBz2 | ArchiveKind::TarXz => {
            list_tar_entries(archive_path)
        }
        ArchiveKind::Gz | ArchiveKind::Bz2 | ArchiveKind::Xz => Err(format!(
            "Archive {} should be handled as a single-file compressed ROM",
            archive_path.display()
        )),
    }
}

fn list_zip_entries(archive_path: &Path) -> Result<Vec<ArchiveEntry>, String> {
    let archive_file = std::fs::File::open(archive_path).map_err(|e| {
        format!(
            "Failed to open ROM archive {}: {}",
            archive_path.display(),
            e
        )
    })?;
    let mut archive = zip::ZipArchive::new(archive_file).map_err(|e| {
        format!(
            "Failed to read ROM archive {}: {}",
            archive_path.display(),
            e
        )
    })?;

    let mut entries = Vec::new();
    for index in 0..archive.len() {
        let entry = archive.by_index(index).map_err(|e| {
            format!(
                "Failed to read entry {} from ROM archive {}: {}",
                index,
                archive_path.display(),
                e
            )
        })?;
        let Some(enclosed) = entry.enclosed_name() else {
            continue;
        };
        if enclosed.as_os_str().is_empty() {
            continue;
        }
        entries.push(ArchiveEntry {
            path: enclosed.to_path_buf(),
            is_dir: entry.is_dir(),
        });
    }
    Ok(entries)
}

fn list_tar_entries(archive_path: &Path) -> Result<Vec<ArchiveEntry>, String> {
    let output = Command::new("tar")
        .arg("-tf")
        .arg(archive_path)
        .output()
        .map_err(|e| {
            format!(
                "Failed to list archive {} with tar: {}",
                archive_path.display(),
                e
            )
        })?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(format!(
            "Failed to list archive {} with tar: {}",
            archive_path.display(),
            stderr.trim()
        ));
    }

    let mut entries = Vec::new();
    for line in String::from_utf8_lossy(&output.stdout).lines() {
        let trimmed = line.trim();
        if trimmed.is_empty() {
            continue;
        }
        let is_dir = trimmed.ends_with('/');
        let raw = trimmed.trim_end_matches('/');
        if let Some(path) = normalize_archive_relative_path(Path::new(raw)) {
            entries.push(ArchiveEntry { path, is_dir });
        }
    }
    Ok(entries)
}

fn list_7z_entries(archive_path: &Path) -> Result<Vec<ArchiveEntry>, String> {
    let output = Command::new("7z")
        .args(["l", "-slt"])
        .arg(archive_path)
        .output()
        .map_err(|e| {
            format!(
                "Failed to list archive {} with 7z: {}",
                archive_path.display(),
                e
            )
        })?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(format!(
            "Failed to list archive {} with 7z: {}",
            archive_path.display(),
            stderr.trim()
        ));
    }

    let text = String::from_utf8_lossy(&output.stdout);
    let mut entries = Vec::new();
    let mut current_path: Option<String> = None;
    let mut current_is_dir = false;
    let mut in_entries = false;

    for line in text.lines() {
        if line.starts_with("----------") {
            in_entries = true;
            current_path = None;
            current_is_dir = false;
            continue;
        }
        if !in_entries {
            continue;
        }
        if line.is_empty() {
            if let Some(path) = current_path.take() {
                if let Some(path) = normalize_archive_relative_path(Path::new(&path)) {
                    entries.push(ArchiveEntry {
                        path,
                        is_dir: current_is_dir,
                    });
                }
            }
            current_is_dir = false;
            continue;
        }
        if let Some(path) = line.strip_prefix("Path = ") {
            current_path = Some(path.to_string());
            continue;
        }
        if let Some(folder) = line.strip_prefix("Attributes = ") {
            current_is_dir = folder.starts_with("D_");
            continue;
        }
    }

    if let Some(path) = current_path.take() {
        if let Some(path) = normalize_archive_relative_path(Path::new(&path)) {
            entries.push(ArchiveEntry {
                path,
                is_dir: current_is_dir,
            });
        }
    }

    Ok(entries)
}

fn normalize_archive_relative_path(path: &Path) -> Option<PathBuf> {
    if path.as_os_str().is_empty() || path.is_absolute() {
        return None;
    }

    let mut normalized = PathBuf::new();
    for component in path.components() {
        match component {
            std::path::Component::CurDir => {}
            std::path::Component::Normal(part) => normalized.push(part),
            std::path::Component::ParentDir
            | std::path::Component::RootDir
            | std::path::Component::Prefix(_) => return None,
        }
    }

    if normalized.as_os_str().is_empty() {
        None
    } else {
        Some(normalized)
    }
}

fn select_launch_entry(entries: &[ArchiveEntry]) -> Option<PathBuf> {
    entries
        .iter()
        .filter(|entry| !entry.is_dir)
        .filter_map(|entry| {
            launch_entry_priority(&entry.path).map(|priority| (priority, entry.path.clone()))
        })
        .min_by(|(priority_a, path_a), (priority_b, path_b)| {
            priority_a
                .cmp(priority_b)
                .then_with(|| {
                    path_a
                        .components()
                        .count()
                        .cmp(&path_b.components().count())
                })
                .then_with(|| path_a.cmp(path_b))
        })
        .map(|(_, path)| path)
}

fn launch_entry_priority(path: &Path) -> Option<u8> {
    let ext = path
        .extension()
        .and_then(|ext| ext.to_str())?
        .to_ascii_lowercase();

    if matches!(ext.as_str(), "cue" | "m3u" | "gdi" | "ccd" | "mds") {
        return Some(0);
    }
    if !crate::scanner::file_scanner::is_recognized_rom_extension(&ext) {
        return None;
    }

    Some(match ext.as_str() {
        "chd" | "iso" | "cso" | "gcz" | "rvz" | "wbfs" | "pbp" | "pkg" => 1,
        "bin" | "img" | "sub" | "mdf" => 3,
        _ => 2,
    })
}

fn collect_entries_needed_for_launch(
    archive_path: &Path,
    kind: ArchiveKind,
    entries: &[ArchiveEntry],
    launch_entry: &Path,
) -> Result<Vec<PathBuf>, String> {
    let available: HashSet<PathBuf> = entries
        .iter()
        .filter(|entry| !entry.is_dir)
        .map(|entry| entry.path.clone())
        .collect();
    let mut selected = BTreeSet::new();
    let mut pending = vec![launch_entry.to_path_buf()];

    while let Some(entry_path) = pending.pop() {
        if !selected.insert(entry_path.clone()) {
            continue;
        }

        let ext = entry_path
            .extension()
            .and_then(|ext| ext.to_str())
            .map(|ext| ext.to_ascii_lowercase())
            .unwrap_or_default();

        match ext.as_str() {
            "cue" => {
                let content = read_archive_entry_text(archive_path, kind, &entry_path)?;
                for reference in parse_cue_references(&content, &entry_path) {
                    push_entry_reference(&available, &mut pending, &entry_path, reference)?;
                }
            }
            "m3u" => {
                let content = read_archive_entry_text(archive_path, kind, &entry_path)?;
                for reference in parse_m3u_references(&content, &entry_path) {
                    push_entry_reference(&available, &mut pending, &entry_path, reference)?;
                }
            }
            "gdi" => {
                let content = read_archive_entry_text(archive_path, kind, &entry_path)?;
                for reference in parse_gdi_references(&content, &entry_path) {
                    push_entry_reference(&available, &mut pending, &entry_path, reference)?;
                }
            }
            "ccd" => {
                add_same_stem_sidecars(&available, &mut pending, &entry_path, &["img", "sub"]);
            }
            "mds" => {
                add_same_stem_sidecars(&available, &mut pending, &entry_path, &["mdf"]);
            }
            _ => {}
        }
    }

    Ok(selected.into_iter().collect())
}

fn push_entry_reference(
    available: &HashSet<PathBuf>,
    pending: &mut Vec<PathBuf>,
    owner_entry: &Path,
    reference: PathBuf,
) -> Result<(), String> {
    if !available.contains(&reference) {
        return Err(format!(
            "Archive entry {} references missing sidecar {}",
            owner_entry.display(),
            reference.display()
        ));
    }
    pending.push(reference);
    Ok(())
}

fn add_same_stem_sidecars(
    available: &HashSet<PathBuf>,
    pending: &mut Vec<PathBuf>,
    entry_path: &Path,
    extensions: &[&str],
) {
    for ext in extensions {
        let candidate = entry_path.with_extension(ext);
        if available.contains(&candidate) {
            pending.push(candidate);
        }
    }
}

fn parse_cue_references(content: &str, entry_path: &Path) -> Vec<PathBuf> {
    content
        .lines()
        .filter_map(|line| {
            let trimmed = line.trim();
            if !trimmed.to_ascii_uppercase().starts_with("FILE ") {
                return None;
            }
            let remainder = trimmed[4..].trim();
            let file_name = if let Some(rest) = remainder.strip_prefix('"') {
                let end = rest.find('"')?;
                &rest[..end]
            } else {
                remainder.split_whitespace().next()?
            };
            normalize_archive_reference(entry_path, file_name)
        })
        .collect()
}

fn parse_m3u_references(content: &str, entry_path: &Path) -> Vec<PathBuf> {
    content
        .lines()
        .filter_map(|line| {
            let trimmed = line.trim();
            if trimmed.is_empty() || trimmed.starts_with('#') {
                return None;
            }
            normalize_archive_reference(entry_path, trimmed)
        })
        .collect()
}

fn parse_gdi_references(content: &str, entry_path: &Path) -> Vec<PathBuf> {
    content
        .lines()
        .skip(1)
        .filter_map(|line| {
            let trimmed = line.trim();
            if trimmed.is_empty() {
                return None;
            }
            let parts: Vec<&str> = trimmed.split_whitespace().collect();
            let file_name = parts.get(4)?.trim_matches('"');
            normalize_archive_reference(entry_path, file_name)
        })
        .collect()
}

fn normalize_archive_reference(owner_entry: &Path, reference: &str) -> Option<PathBuf> {
    let base_dir = owner_entry.parent().unwrap_or_else(|| Path::new(""));
    normalize_archive_relative_path(&base_dir.join(reference))
}

fn read_archive_entry_text(
    archive_path: &Path,
    kind: ArchiveKind,
    entry_path: &Path,
) -> Result<String, String> {
    let bytes = read_archive_entry_bytes(archive_path, kind, entry_path)?;
    Ok(String::from_utf8_lossy(&bytes).into_owned())
}

fn read_archive_entry_bytes(
    archive_path: &Path,
    kind: ArchiveKind,
    entry_path: &Path,
) -> Result<Vec<u8>, String> {
    match kind {
        ArchiveKind::Zip => read_zip_entry_bytes(archive_path, entry_path),
        ArchiveKind::SevenZip | ArchiveKind::Rar => read_7z_entry_bytes(archive_path, entry_path),
        ArchiveKind::Tar | ArchiveKind::TarGz | ArchiveKind::TarBz2 | ArchiveKind::TarXz => {
            read_tar_entry_bytes(archive_path, entry_path)
        }
        ArchiveKind::Gz | ArchiveKind::Bz2 | ArchiveKind::Xz => Err(format!(
            "Archive {} does not expose multiple named entries",
            archive_path.display()
        )),
    }
}

fn read_zip_entry_bytes(archive_path: &Path, entry_path: &Path) -> Result<Vec<u8>, String> {
    let archive_file = std::fs::File::open(archive_path).map_err(|e| {
        format!(
            "Failed to open ROM archive {}: {}",
            archive_path.display(),
            e
        )
    })?;
    let mut archive = zip::ZipArchive::new(archive_file).map_err(|e| {
        format!(
            "Failed to read ROM archive {}: {}",
            archive_path.display(),
            e
        )
    })?;
    let mut entry = archive
        .by_name(&path_to_archive_name(entry_path))
        .map_err(|e| {
            format!(
                "Failed to read {} from {}: {}",
                entry_path.display(),
                archive_path.display(),
                e
            )
        })?;
    let mut bytes = Vec::new();
    entry.read_to_end(&mut bytes).map_err(|e| {
        format!(
            "Failed to read ROM archive entry {} from {}: {}",
            entry_path.display(),
            archive_path.display(),
            e
        )
    })?;
    Ok(bytes)
}

fn read_tar_entry_bytes(archive_path: &Path, entry_path: &Path) -> Result<Vec<u8>, String> {
    let output = Command::new("tar")
        .arg("-xOf")
        .arg(archive_path)
        .arg(path_to_archive_name(entry_path))
        .output()
        .map_err(|e| {
            format!(
                "Failed to read {} from {} with tar: {}",
                entry_path.display(),
                archive_path.display(),
                e
            )
        })?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(format!(
            "Failed to read {} from {} with tar: {}",
            entry_path.display(),
            archive_path.display(),
            stderr.trim()
        ));
    }

    Ok(output.stdout)
}

fn read_7z_entry_bytes(archive_path: &Path, entry_path: &Path) -> Result<Vec<u8>, String> {
    let output = Command::new("7z")
        .args(["x", "-bd", "-y", "-so"])
        .arg(archive_path)
        .arg(path_to_archive_name(entry_path))
        .output()
        .map_err(|e| {
            format!(
                "Failed to read {} from {} with 7z: {}",
                entry_path.display(),
                archive_path.display(),
                e
            )
        })?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(format!(
            "Failed to read {} from {} with 7z: {}",
            entry_path.display(),
            archive_path.display(),
            stderr.trim()
        ));
    }

    Ok(output.stdout)
}

fn extract_archive_entries(
    archive_path: &Path,
    kind: ArchiveKind,
    parent_dir: &Path,
    entry_paths: &[PathBuf],
) -> Result<Vec<PathBuf>, String> {
    if entry_paths.is_empty() {
        return Err(format!(
            "Archive {} did not provide any entries to extract",
            archive_path.display()
        ));
    }

    if entry_paths.len() == 1 {
        let entry_path = &entry_paths[0];
        let bytes = read_archive_entry_bytes(archive_path, kind, entry_path)?;
        let output_path = write_single_extracted_entry(parent_dir, entry_path, &bytes)?;
        return Ok(vec![output_path]);
    }

    let output_paths = planned_output_paths(parent_dir, entry_paths, false)?;
    match kind {
        ArchiveKind::Zip => extract_zip_entries(archive_path, parent_dir, entry_paths)?,
        ArchiveKind::SevenZip | ArchiveKind::Rar => {
            extract_7z_entries(archive_path, parent_dir, entry_paths)?
        }
        ArchiveKind::Tar | ArchiveKind::TarGz | ArchiveKind::TarBz2 | ArchiveKind::TarXz => {
            extract_tar_entries(archive_path, parent_dir, entry_paths)?
        }
        ArchiveKind::Gz | ArchiveKind::Bz2 | ArchiveKind::Xz => unreachable!(),
    }
    Ok(output_paths)
}

fn extract_zip_entries(
    archive_path: &Path,
    parent_dir: &Path,
    entry_paths: &[PathBuf],
) -> Result<(), String> {
    let archive_file = std::fs::File::open(archive_path).map_err(|e| {
        format!(
            "Failed to open ROM archive {}: {}",
            archive_path.display(),
            e
        )
    })?;
    let mut archive = zip::ZipArchive::new(archive_file).map_err(|e| {
        format!(
            "Failed to read ROM archive {}: {}",
            archive_path.display(),
            e
        )
    })?;

    for entry_path in entry_paths {
        let mut entry = archive
            .by_name(&path_to_archive_name(entry_path))
            .map_err(|e| {
                format!(
                    "Failed to read {} from {}: {}",
                    entry_path.display(),
                    archive_path.display(),
                    e
                )
            })?;
        let output_path = prepare_output_path(parent_dir, entry_path, false)?;
        let mut output_file = std::fs::File::create(&output_path).map_err(|e| {
            format!(
                "Failed to create extracted ROM {}: {}",
                output_path.display(),
                e
            )
        })?;
        std::io::copy(&mut entry, &mut output_file).map_err(|e| {
            format!(
                "Failed to extract ROM {} from {}: {}",
                entry_path.display(),
                archive_path.display(),
                e
            )
        })?;
        output_file.flush().map_err(|e| {
            format!(
                "Failed to flush extracted ROM {}: {}",
                output_path.display(),
                e
            )
        })?;
    }

    Ok(())
}

fn extract_tar_entries(
    archive_path: &Path,
    parent_dir: &Path,
    entry_paths: &[PathBuf],
) -> Result<(), String> {
    let mut cmd = Command::new("tar");
    cmd.arg("-xf")
        .arg(archive_path)
        .arg("-C")
        .arg(parent_dir)
        .arg("--");
    for entry_path in entry_paths {
        cmd.arg(path_to_archive_name(entry_path));
    }
    let output = cmd.output().map_err(|e| {
        format!(
            "Failed to extract {} with tar: {}",
            archive_path.display(),
            e
        )
    })?;
    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(format!(
            "Failed to extract {} with tar: {}",
            archive_path.display(),
            stderr.trim()
        ));
    }
    Ok(())
}

fn extract_7z_entries(
    archive_path: &Path,
    parent_dir: &Path,
    entry_paths: &[PathBuf],
) -> Result<(), String> {
    let mut cmd = Command::new("7z");
    cmd.args(["x", "-bd", "-y", &format!("-o{}", parent_dir.display())])
        .arg(archive_path);
    for entry_path in entry_paths {
        cmd.arg(path_to_archive_name(entry_path));
    }
    let output = cmd.output().map_err(|e| {
        format!(
            "Failed to extract {} with 7z: {}",
            archive_path.display(),
            e
        )
    })?;
    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(format!(
            "Failed to extract {} with 7z: {}",
            archive_path.display(),
            stderr.trim()
        ));
    }
    Ok(())
}

fn extract_single_file_archive(
    archive_path: &Path,
    kind: ArchiveKind,
) -> Result<PreparedRomLaunch, String> {
    let parent_dir = archive_path.parent().ok_or_else(|| {
        format!(
            "ROM archive {} does not have a parent directory",
            archive_path.display()
        )
    })?;
    let output_name = stripped_single_file_archive_name(archive_path, kind)?;
    let output_rel = PathBuf::from(output_name);
    let ext = output_rel
        .extension()
        .and_then(|ext| ext.to_str())
        .ok_or_else(|| {
            format!(
                "Cannot determine ROM type after decompressing {}",
                archive_path.display()
            )
        })?;
    if !crate::scanner::file_scanner::is_recognized_rom_extension(ext)
        && !matches!(
            ext.to_ascii_lowercase().as_str(),
            "cue" | "m3u" | "gdi" | "ccd" | "mds"
        )
    {
        return Err(format!(
            "Decompressed file {} from {} is not a supported ROM type",
            output_rel.display(),
            archive_path.display()
        ));
    }

    let output_path = prepare_output_path(parent_dir, &output_rel, true)?;
    let output_file = std::fs::File::create(&output_path).map_err(|e| {
        format!(
            "Failed to create extracted ROM {}: {}",
            output_path.display(),
            e
        )
    })?;

    match kind {
        ArchiveKind::Gz => {
            let input = std::fs::File::open(archive_path).map_err(|e| {
                format!(
                    "Failed to open ROM archive {}: {}",
                    archive_path.display(),
                    e
                )
            })?;
            let mut decoder = flate2::read::GzDecoder::new(input);
            let mut writer = std::io::BufWriter::new(output_file);
            std::io::copy(&mut decoder, &mut writer)
                .map_err(|e| format!("Failed to decompress {}: {}", archive_path.display(), e))?;
            writer.flush().map_err(|e| {
                format!(
                    "Failed to flush extracted ROM {}: {}",
                    output_path.display(),
                    e
                )
            })?;
        }
        ArchiveKind::Bz2 => {
            let input = std::fs::File::open(archive_path).map_err(|e| {
                format!(
                    "Failed to open ROM archive {}: {}",
                    archive_path.display(),
                    e
                )
            })?;
            let mut decoder = bzip2::read::BzDecoder::new(input);
            let mut writer = std::io::BufWriter::new(output_file);
            std::io::copy(&mut decoder, &mut writer)
                .map_err(|e| format!("Failed to decompress {}: {}", archive_path.display(), e))?;
            writer.flush().map_err(|e| {
                format!(
                    "Failed to flush extracted ROM {}: {}",
                    output_path.display(),
                    e
                )
            })?;
        }
        ArchiveKind::Xz => {
            let input = std::fs::File::open(archive_path).map_err(|e| {
                format!(
                    "Failed to open ROM archive {}: {}",
                    archive_path.display(),
                    e
                )
            })?;
            let mut decoder = xz2::read::XzDecoder::new(input);
            let mut writer = std::io::BufWriter::new(output_file);
            std::io::copy(&mut decoder, &mut writer)
                .map_err(|e| format!("Failed to decompress {}: {}", archive_path.display(), e))?;
            writer.flush().map_err(|e| {
                format!(
                    "Failed to flush extracted ROM {}: {}",
                    output_path.display(),
                    e
                )
            })?;
        }
        _ => unreachable!(),
    }

    Ok(PreparedRomLaunch {
        rom_path: Some(output_path.to_string_lossy().to_string()),
        cleanup_paths: vec![output_path],
    })
}

fn stripped_single_file_archive_name(
    archive_path: &Path,
    kind: ArchiveKind,
) -> Result<String, String> {
    let file_name = archive_path
        .file_name()
        .and_then(|name| name.to_str())
        .ok_or_else(|| format!("Invalid archive path {}", archive_path.display()))?;
    let stripped = match kind {
        ArchiveKind::Gz => file_name.strip_suffix(".gz"),
        ArchiveKind::Bz2 => file_name.strip_suffix(".bz2"),
        ArchiveKind::Xz => file_name.strip_suffix(".xz"),
        _ => None,
    }
    .ok_or_else(|| {
        format!(
            "Could not derive output name from {}",
            archive_path.display()
        )
    })?;
    Ok(stripped.to_string())
}

fn planned_output_paths(
    parent_dir: &Path,
    entry_paths: &[PathBuf],
    allow_unique_single: bool,
) -> Result<Vec<PathBuf>, String> {
    entry_paths
        .iter()
        .map(|entry_path| prepare_output_path(parent_dir, entry_path, allow_unique_single))
        .collect()
}

fn write_single_extracted_entry(
    parent_dir: &Path,
    entry_path: &Path,
    bytes: &[u8],
) -> Result<PathBuf, String> {
    let output_path = prepare_output_path(parent_dir, entry_path, true)?;
    let mut output_file = std::fs::File::create(&output_path).map_err(|e| {
        format!(
            "Failed to create extracted ROM {}: {}",
            output_path.display(),
            e
        )
    })?;
    output_file.write_all(bytes).map_err(|e| {
        format!(
            "Failed to write extracted ROM {}: {}",
            output_path.display(),
            e
        )
    })?;
    output_file.flush().map_err(|e| {
        format!(
            "Failed to flush extracted ROM {}: {}",
            output_path.display(),
            e
        )
    })?;
    Ok(output_path)
}

fn prepare_output_path(
    parent_dir: &Path,
    entry_path: &Path,
    allow_unique_single: bool,
) -> Result<PathBuf, String> {
    let relative_parent = entry_path.parent().unwrap_or_else(|| Path::new(""));
    let output_dir = parent_dir.join(relative_parent);
    std::fs::create_dir_all(&output_dir).map_err(|e| {
        format!(
            "Failed to create extracted ROM directory {}: {}",
            output_dir.display(),
            e
        )
    })?;

    let file_name = entry_path.file_name().ok_or_else(|| {
        format!(
            "Archive entry {} does not have a file name",
            entry_path.display()
        )
    })?;
    let base_output = output_dir.join(file_name);
    if !base_output.exists() {
        return Ok(base_output);
    }
    if allow_unique_single {
        return Ok(unique_extracted_rom_path(&output_dir, Path::new(file_name)));
    }
    Err(format!(
        "Refusing to extract archive over existing file {}",
        base_output.display()
    ))
}

fn path_to_archive_name(path: &Path) -> String {
    path.components()
        .filter_map(|component| match component {
            std::path::Component::Normal(part) => Some(part.to_string_lossy().into_owned()),
            _ => None,
        })
        .collect::<Vec<_>>()
        .join("/")
}

fn unique_extracted_rom_path(parent_dir: &Path, entry_name: &Path) -> PathBuf {
    let candidate = parent_dir.join(entry_name);
    if !candidate.exists() {
        return candidate;
    }

    let stem = entry_name
        .file_stem()
        .and_then(|stem| stem.to_str())
        .unwrap_or("rom");
    let ext = entry_name
        .extension()
        .and_then(|ext| ext.to_str())
        .unwrap_or_default();
    let suffix = uuid::Uuid::new_v4().to_string();

    let filename = if ext.is_empty() {
        format!("{}.lunchbox-{}", stem, suffix)
    } else {
        format!("{}.lunchbox-{}.{}", stem, suffix, ext)
    };

    parent_dir.join(filename)
}

/// Launch an emulator (optionally with a ROM)
/// If `as_retroarch_core` is true, launch via RetroArch; otherwise launch standalone
pub fn launch_emulator(
    emulator: &EmulatorInfo,
    rom_path: Option<&str>,
    as_retroarch_core: bool,
    launch_args: &[LaunchArg],
) -> Result<u32, String> {
    tracing::info!(
        emulator = %emulator.name,
        rom = ?rom_path,
        as_retroarch_core = as_retroarch_core,
        "Launching emulator"
    );

    let prepared_rom = prepare_rom_for_launch(rom_path)?;

    let result = if as_retroarch_core {
        if let Some(ref core_name) = emulator.retroarch_core {
            launch_retroarch(
                core_name,
                prepared_rom.rom_path.as_deref(),
                prepared_rom.cleanup_paths,
                launch_args,
            )
        } else {
            cleanup_extracted_roms(&prepared_rom.cleanup_paths);
            let err = format!("{} does not have a RetroArch core", emulator.name);
            tracing::error!(error = %err);
            return Err(err);
        }
    } else {
        launch_standalone(
            emulator,
            prepared_rom.rom_path.as_deref(),
            prepared_rom.cleanup_paths,
            launch_args,
        )
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
    launch_emulator(emulator, Some(rom_path), as_retroarch_core, &[])
}

const DEFAULT_SCUMMVM_CONFIG: &str = r#"[scummvm]
filtering=false
autosave_period=300
mute=false
speech_volume=192
native_mt32=false
mt32_device=mt32
kbdmouse_speed=3
talkspeed=60
midi_gain=100
subtitles=false
multi_midi=false
fullscreen=false
updates_check=2628000
gui_browser_show_hidden=false
gm_device=null
sfx_volume=192
music_volume=192
speech_mute=false
music_driver=auto
opl_driver=auto
aspect_ratio=false
gui_theme=SCUMMMODERN
enable_gs=false
"#;

pub fn launch_prepared_install(
    emulator: &EmulatorInfo,
    collection: crate::exo::ExoCollection,
    install_root: &Path,
    launch_config_path: &Path,
    as_retroarch_core: bool,
) -> Result<u32, String> {
    match collection {
        crate::exo::ExoCollection::Dos | crate::exo::ExoCollection::Win3x => {
            launch_dosbox_prepared_install(
                emulator,
                install_root,
                launch_config_path,
                as_retroarch_core,
            )
        }
        crate::exo::ExoCollection::Win9x => launch_win9x_prepared_install(
            emulator,
            install_root,
            launch_config_path,
            as_retroarch_core,
        ),
    }
}

fn launch_dosbox_prepared_install(
    emulator: &EmulatorInfo,
    install_root: &Path,
    launch_config_path: &Path,
    as_retroarch_core: bool,
) -> Result<u32, String> {
    let exception_script = launch_config_path.parent().and_then(|dir| {
        let linux = dir.join("exception.bsh");
        if linux.exists() {
            Some(linux)
        } else {
            let windows = dir.join("exception.bat");
            windows.exists().then_some(windows)
        }
    });
    let mut cleanup_paths = Vec::new();
    let mut effective_launch_config_path = launch_config_path.to_path_buf();
    if let Some(path) = exception_script.as_ref() {
        let is_linux_exception = path
            .extension()
            .and_then(|ext| ext.to_str())
            .map(|ext| ext.eq_ignore_ascii_case("bsh"))
            .unwrap_or(false);
        if is_linux_exception {
            if let Ok(scummvm_plan) = crate::exo::load_linux_scummvm_exception_plan(path) {
                return launch_scummvm_install(
                    emulator,
                    install_root,
                    &scummvm_plan,
                    as_retroarch_core,
                );
            }
            let exception_plan =
                crate::exo::load_linux_dosbox_exception_plan(path).map_err(|e| {
                    format!(
                        "This eXo title uses a Linux-specific exception launcher ({}) that Lunchbox does not support yet: {}",
                        path.display(),
                        e
                    )
                })?;
            effective_launch_config_path = path
                .parent()
                .map(|dir| dir.join(&exception_plan.launch_config_name))
                .ok_or_else(|| {
                    format!(
                        "Linux eXo exception launcher {} does not have a parent directory.",
                        path.display()
                    )
                })?;
            if !effective_launch_config_path.exists() {
                return Err(format!(
                    "Linux eXo exception launcher {} refers to missing config {}.",
                    path.display(),
                    effective_launch_config_path.display()
                ));
            }
        } else {
            let exception_plan = crate::exo::load_dosbox_exception_plan(path).map_err(|e| {
                format!(
                    "This eXo title uses a custom exception launcher ({}) that Lunchbox does not support yet: {}",
                    path.display(),
                    e
                )
            })?;
            cleanup_paths.extend(prepare_dosbox_exception_files(
                install_root,
                &exception_plan,
            )?);
        }
    }

    if as_retroarch_core {
        return Err(
            "eXo DOS/Win3x installs currently require a standalone DOSBox emulator. Use DOSBox-X or DOSBox Staging."
                .to_string(),
        );
    }

    let emulator_name = emulator.name.to_ascii_lowercase();
    if !emulator_name.contains("dosbox") {
        return Err(format!(
            "{} is not supported for eXo DOS/Win3x installs yet. Use DOSBox-X or DOSBox Staging.",
            emulator.name
        ));
    }

    let shared_options_path = if effective_launch_config_path
        .file_name()
        .and_then(|name| name.to_str())
        .map(|name| name.to_ascii_lowercase().ends_with("_linux.conf"))
        .unwrap_or(false)
    {
        let linux_options = install_root.join("emulators/dosbox/options_linux.conf");
        if linux_options.exists() {
            Some(linux_options)
        } else {
            let fallback = install_root.join("emulators/dosbox/options.conf");
            fallback.exists().then_some(fallback)
        }
    } else {
        let windows_options = install_root.join("emulators/dosbox/options.conf");
        if windows_options.exists() {
            Some(windows_options)
        } else {
            let fallback = install_root.join("emulators/dosbox/options_linux.conf");
            fallback.exists().then_some(fallback)
        }
    };

    let exe_path = check_standalone_installation(emulator)
        .ok_or_else(|| format!("{} standalone is not installed", emulator.name))?;

    if exe_path.to_string_lossy().starts_with("flatpak::") {
        let app_id = exe_path.to_string_lossy().replace("flatpak::", "");
        let mut cmd = Command::new("flatpak");
        cmd.arg("run");
        add_flatpak_filesystem_args(&mut cmd, vec![(install_root, false)]);
        cmd.arg(&app_id);
        cmd.arg("-conf").arg(map_path_for_flatpak(
            &effective_launch_config_path.to_string_lossy(),
        ));
        if let Some(ref shared_options_path) = shared_options_path {
            cmd.arg("-conf")
                .arg(map_path_for_flatpak(&shared_options_path.to_string_lossy()));
        }
        cmd.current_dir(install_root);
        tracing::info!(
            command = ?cmd,
            install_root = %install_root.display(),
            config = %effective_launch_config_path.display(),
            "Spawning DOSBox install via flatpak"
        );
        return spawn_and_verify(cmd, &emulator.name, cleanup_paths);
    }

    let mut cmd = Command::new(&exe_path);
    cmd.arg("-conf").arg(&effective_launch_config_path);
    if let Some(ref shared_options_path) = shared_options_path {
        cmd.arg("-conf").arg(shared_options_path);
    }
    cmd.current_dir(install_root);
    tracing::info!(
        command = ?cmd,
        install_root = %install_root.display(),
        config = %effective_launch_config_path.display(),
        "Spawning DOSBox install natively"
    );
    spawn_and_verify(cmd, &emulator.name, cleanup_paths)
}

fn launch_win9x_prepared_install(
    emulator: &EmulatorInfo,
    install_root: &Path,
    launch_config_path: &Path,
    as_retroarch_core: bool,
) -> Result<u32, String> {
    if as_retroarch_core {
        return Err(
            "eXoWin9x installs currently require standalone DOSBox-X, not RetroArch.".to_string(),
        );
    }

    let launcher_kind =
        detect_win9x_launcher_kind(launch_config_path.parent().ok_or_else(|| {
            format!(
                "Prepared eXoWin9x config {} does not have a parent directory.",
                launch_config_path.display()
            )
        })?)?;

    match launcher_kind {
        Win9xLauncherKind::DosboxX => {
            let emulator_name = emulator.name.to_ascii_lowercase();
            if !emulator_name.contains("dosbox-x") {
                return Err(format!(
                    "{} is not supported for this eXoWin9x title. Use DOSBox-X.",
                    emulator.name
                ));
            }

            let shared_options_path = install_root.join("emulators/dosbox/options9x.conf");
            if !shared_options_path.exists() {
                return Err(format!(
                    "Prepared eXoWin9x install is missing {}.",
                    shared_options_path.display()
                ));
            }

            let exe_path = check_standalone_installation(emulator)
                .ok_or_else(|| format!("{} standalone is not installed", emulator.name))?;

            if exe_path.to_string_lossy().starts_with("flatpak::") {
                let app_id = exe_path.to_string_lossy().replace("flatpak::", "");
                let mut cmd = Command::new("flatpak");
                cmd.arg("run");
                add_flatpak_filesystem_args(&mut cmd, vec![(install_root, false)]);
                cmd.arg(&app_id);
                cmd.arg("-conf")
                    .arg(map_path_for_flatpak(&launch_config_path.to_string_lossy()));
                cmd.arg("-conf")
                    .arg(map_path_for_flatpak(&shared_options_path.to_string_lossy()));
                cmd.arg("-nomenu");
                cmd.arg("-noconsole");
                cmd.current_dir(install_root);
                tracing::info!(
                    command = ?cmd,
                    install_root = %install_root.display(),
                    config = %launch_config_path.display(),
                    "Spawning eXoWin9x DOSBox-X install via flatpak"
                );
                return spawn_and_verify(cmd, &emulator.name, Vec::new());
            }

            let mut cmd = Command::new(&exe_path);
            cmd.arg("-conf").arg(launch_config_path);
            cmd.arg("-conf").arg(&shared_options_path);
            cmd.arg("-nomenu");
            cmd.arg("-noconsole");
            cmd.current_dir(install_root);
            tracing::info!(
                command = ?cmd,
                install_root = %install_root.display(),
                config = %launch_config_path.display(),
                "Spawning eXoWin9x DOSBox-X install natively"
            );
            spawn_and_verify(cmd, &emulator.name, Vec::new())
        }
        Win9xLauncherKind::EightySixBox(plan) => {
            let emulator_name = emulator.name.to_ascii_lowercase();
            if !emulator_name.contains("86box") {
                return Err(format!(
                    "{} is not supported for this eXoWin9x title. Use 86Box.",
                    emulator.name
                ));
            }

            let vm_root = install_root.join("emulators/86Box98");
            let vm_config_path = vm_root.join("86box.cfg");
            prepare_86box_vm_root(
                install_root,
                launch_config_path,
                &plan,
                &vm_root,
                &vm_config_path,
            )?;

            let exe_path = check_standalone_installation(emulator)
                .ok_or_else(|| format!("{} standalone is not installed", emulator.name))?;

            if exe_path.to_string_lossy().starts_with("flatpak::") {
                let app_id = exe_path.to_string_lossy().replace("flatpak::", "");
                let mut cmd = Command::new("flatpak");
                cmd.arg("run");
                add_flatpak_filesystem_args(&mut cmd, vec![(install_root, false)]);
                cmd.arg(&app_id);
                cmd.arg("-P")
                    .arg(map_path_for_flatpak(&vm_root.to_string_lossy()));
                cmd.current_dir(install_root);
                tracing::info!(
                    command = ?cmd,
                    install_root = %install_root.display(),
                    vm_root = %vm_root.display(),
                    "Spawning eXoWin9x 86Box install via flatpak"
                );
                return spawn_and_verify(cmd, &emulator.name, Vec::new());
            }

            let mut cmd = Command::new(&exe_path);
            cmd.arg("-P").arg(&vm_root);
            cmd.current_dir(install_root);
            tracing::info!(
                command = ?cmd,
                install_root = %install_root.display(),
                vm_root = %vm_root.display(),
                "Spawning eXoWin9x 86Box install natively"
            );
            spawn_and_verify(cmd, &emulator.name, Vec::new())
        }
        Win9xLauncherKind::PcBox(plan) => {
            let emulator_name = emulator.name.to_ascii_lowercase();
            if !emulator_name.contains("86box") && !emulator_name.contains("pcbox") {
                return Err(format!(
                    "{} is not supported for this eXoWin9x title. Use 86Box.",
                    emulator.name
                ));
            }

            let vm_root = install_root.join("emulators/PCBox");
            let exe_path = check_standalone_installation(emulator)
                .ok_or_else(|| format!("{} standalone is not installed", emulator.name))?;
            let use_native_pcbox = emulator_name.contains("pcbox");
            let vm_config_path = if use_native_pcbox {
                vm_root.join("play.cfg")
            } else {
                vm_root.join("86box.cfg")
            };
            prepare_pcbox_vm_root(launch_config_path, &plan, &vm_root, &vm_config_path)?;

            if use_native_pcbox {
                if exe_path.to_string_lossy().starts_with("flatpak::") {
                    return Err(
                        "PCBox launches currently require a native PCBox install, not Flatpak."
                            .to_string(),
                    );
                }

                let mut cmd = Command::new(&exe_path);
                cmd.arg("-c").arg(&vm_config_path);
                cmd.current_dir(install_root);
                tracing::info!(
                    command = ?cmd,
                    install_root = %install_root.display(),
                    vm_root = %vm_root.display(),
                    "Spawning eXoWin9x PCBox install natively"
                );
                return spawn_and_verify(cmd, &emulator.name, Vec::new());
            }

            if exe_path.to_string_lossy().starts_with("flatpak::") {
                let app_id = exe_path.to_string_lossy().replace("flatpak::", "");
                let mut cmd = Command::new("flatpak");
                cmd.arg("run");
                add_flatpak_filesystem_args(&mut cmd, vec![(install_root, false)]);
                cmd.arg(&app_id);
                cmd.arg("-P")
                    .arg(map_path_for_flatpak(&vm_root.to_string_lossy()));
                cmd.current_dir(install_root);
                tracing::info!(
                    command = ?cmd,
                    install_root = %install_root.display(),
                    vm_root = %vm_root.display(),
                    "Spawning eXoWin9x PCBox-profile install via 86Box flatpak"
                );
                return spawn_and_verify(cmd, &emulator.name, Vec::new());
            }

            let mut cmd = Command::new(&exe_path);
            cmd.arg("-P").arg(&vm_root);
            cmd.current_dir(install_root);
            tracing::info!(
                command = ?cmd,
                install_root = %install_root.display(),
                vm_root = %vm_root.display(),
                "Spawning eXoWin9x PCBox-profile install via 86Box natively"
            );
            spawn_and_verify(cmd, &emulator.name, Vec::new())
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
enum Win9xLauncherKind {
    DosboxX,
    EightySixBox(Win9xEightySixBoxPlan),
    PcBox(Win9xPcBoxPlan),
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct Win9xEightySixBoxPlan {
    config_name: &'static str,
    parent_vhd_name: &'static str,
    child_vhd_name: &'static str,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct Win9xPcBoxPlan {
    config_name: &'static str,
    parent_vhd_name: &'static str,
    child_vhd_name: &'static str,
}

fn detect_win9x_launcher_kind(metadata_dir: &Path) -> Result<Win9xLauncherKind, String> {
    let mut launcher_script = None;

    for entry in std::fs::read_dir(metadata_dir)
        .map_err(|e| format!("Failed to read {}: {}", metadata_dir.display(), e))?
    {
        let entry = entry.map_err(|e| format!("Failed to read metadata entry: {}", e))?;
        let path = entry.path();
        if !path
            .extension()
            .and_then(|ext| ext.to_str())
            .map(|ext| ext.eq_ignore_ascii_case("bat"))
            .unwrap_or(false)
        {
            continue;
        }

        let file_name = path
            .file_name()
            .and_then(|name| name.to_str())
            .unwrap_or("");
        if file_name.eq_ignore_ascii_case("install.bat") {
            continue;
        }

        launcher_script = Some(path);
        break;
    }

    let Some(launcher_script) = launcher_script else {
        return Ok(Win9xLauncherKind::DosboxX);
    };

    let contents = std::fs::read_to_string(&launcher_script).map_err(|e| {
        format!(
            "Failed to read eXoWin9x launcher {}: {}",
            launcher_script.display(),
            e
        )
    })?;
    let lower = contents.replace("\r\n", "\n").to_ascii_lowercase();

    if lower.contains("9xlaunchpcbox") {
        return Ok(Win9xLauncherKind::PcBox(Win9xPcBoxPlan {
            config_name: "Play.cfg",
            parent_vhd_name: "W98-P.vhd",
            child_vhd_name: "W98-C.vhd",
        }));
    }

    if lower.contains("9xlaunch86boxnethost") {
        return Ok(Win9xLauncherKind::EightySixBox(Win9xEightySixBoxPlan {
            config_name: "Host.cfg",
            parent_vhd_name: "W98-NetHost.vhd",
            child_vhd_name: "W98-Host.vhd",
        }));
    }

    if lower.contains("9xlaunch86boxnetjoin") {
        return Ok(Win9xLauncherKind::EightySixBox(Win9xEightySixBoxPlan {
            config_name: "Join.cfg",
            parent_vhd_name: "W98-NetJoin.vhd",
            child_vhd_name: "W98-Join.vhd",
        }));
    }

    if lower.contains("9xlaunch86boxme") {
        return Ok(Win9xLauncherKind::EightySixBox(Win9xEightySixBoxPlan {
            config_name: "Play.cfg",
            parent_vhd_name: "ME-P.vhd",
            child_vhd_name: "ME-C.vhd",
        }));
    }

    if lower.contains("9xlaunch86box") {
        return Ok(Win9xLauncherKind::EightySixBox(Win9xEightySixBoxPlan {
            config_name: "Play.cfg",
            parent_vhd_name: "W98-P.vhd",
            child_vhd_name: "W98-C.vhd",
        }));
    }

    Ok(Win9xLauncherKind::DosboxX)
}

fn prepare_86box_vm_root(
    _install_root: &Path,
    launch_config_path: &Path,
    plan: &Win9xEightySixBoxPlan,
    vm_root: &Path,
    vm_config_path: &Path,
) -> Result<(), String> {
    std::fs::create_dir_all(vm_root)
        .map_err(|e| format!("Failed to create {}: {}", vm_root.display(), e))?;

    let parent_vhd_path = vm_root.join("parent").join(plan.parent_vhd_name);
    if !parent_vhd_path.exists() {
        return Err(format!(
            "Prepared eXoWin9x install is missing 86Box parent disk {}.",
            parent_vhd_path.display()
        ));
    }

    let child_vhd_path = vm_root.join(plan.child_vhd_name);
    if !child_vhd_path.exists() {
        std::fs::copy(&parent_vhd_path, &child_vhd_path).map_err(|e| {
            format!(
                "Failed to create writable 86Box disk {} from {}: {}",
                child_vhd_path.display(),
                parent_vhd_path.display(),
                e
            )
        })?;
    }

    let source_cfg_path = launch_config_path
        .parent()
        .map(|dir| dir.join(plan.config_name))
        .ok_or_else(|| {
            format!(
                "Prepared eXoWin9x config {} does not have a parent directory.",
                launch_config_path.display()
            )
        })?;
    if !source_cfg_path.exists() {
        return Err(format!(
            "Prepared eXoWin9x install is missing 86Box config {}.",
            source_cfg_path.display()
        ));
    }

    std::fs::copy(&source_cfg_path, vm_config_path).map_err(|e| {
        format!(
            "Failed to copy {} to {}: {}",
            source_cfg_path.display(),
            vm_config_path.display(),
            e
        )
    })?;

    Ok(())
}

fn prepare_pcbox_vm_root(
    launch_config_path: &Path,
    plan: &Win9xPcBoxPlan,
    vm_root: &Path,
    vm_config_path: &Path,
) -> Result<(), String> {
    std::fs::create_dir_all(vm_root)
        .map_err(|e| format!("Failed to create {}: {}", vm_root.display(), e))?;

    let parent_vhd_path = vm_root.join("parent").join(plan.parent_vhd_name);
    if !parent_vhd_path.exists() {
        return Err(format!(
            "Prepared eXoWin9x install is missing PCBox parent disk {}.",
            parent_vhd_path.display()
        ));
    }

    let child_vhd_path = vm_root.join(plan.child_vhd_name);
    if !child_vhd_path.exists() {
        std::fs::copy(&parent_vhd_path, &child_vhd_path).map_err(|e| {
            format!(
                "Failed to create writable PCBox disk {} from {}: {}",
                child_vhd_path.display(),
                parent_vhd_path.display(),
                e
            )
        })?;
    }

    let source_cfg_path = launch_config_path
        .parent()
        .map(|dir| dir.join(plan.config_name))
        .ok_or_else(|| {
            format!(
                "Prepared eXoWin9x config {} does not have a parent directory.",
                launch_config_path.display()
            )
        })?;
    if !source_cfg_path.exists() {
        return Err(format!(
            "Prepared eXoWin9x install is missing PCBox config {}.",
            source_cfg_path.display()
        ));
    }

    std::fs::copy(&source_cfg_path, vm_config_path).map_err(|e| {
        format!(
            "Failed to copy {} to {}: {}",
            source_cfg_path.display(),
            vm_config_path.display(),
            e
        )
    })?;

    Ok(())
}

fn launch_scummvm_install(
    emulator: &EmulatorInfo,
    install_root: &Path,
    plan: &crate::exo::LinuxScummvmExceptionPlan,
    as_retroarch_core: bool,
) -> Result<u32, String> {
    if as_retroarch_core {
        return Err(
            "ScummVM-based eXo installs currently require the standalone ScummVM emulator."
                .to_string(),
        );
    }

    if !emulator.name.to_ascii_lowercase().contains("scummvm") {
        return Err(format!(
            "This eXo title uses ScummVM, not DOSBox. Choose ScummVM instead of {}.",
            emulator.name
        ));
    }

    let config_path = ensure_scummvm_config_file(install_root, &plan.config_path)?;
    let game_path = install_root.join(normalize_prepared_install_relative_path(&plan.game_path));
    if !game_path.exists() {
        return Err(format!(
            "ScummVM game directory {} does not exist in the prepared install.",
            game_path.display()
        ));
    }

    let exe_path = check_standalone_installation(emulator)
        .ok_or_else(|| format!("{} standalone is not installed", emulator.name))?;

    if exe_path.to_string_lossy().starts_with("flatpak::") {
        let app_id = exe_path.to_string_lossy().replace("flatpak::", "");
        let mut cmd = Command::new("flatpak");
        cmd.arg("run");
        add_flatpak_filesystem_args(&mut cmd, vec![(install_root, false)]);
        cmd.arg(&app_id);
        cmd.arg(format!(
            "--config={}",
            map_path_for_flatpak(&config_path.to_string_lossy())
        ));
        cmd.args(&plan.extra_args);
        cmd.arg("-p")
            .arg(map_path_for_flatpak(&game_path.to_string_lossy()));
        cmd.arg(&plan.game_id);
        cmd.current_dir(install_root);
        tracing::info!(
            command = ?cmd,
            install_root = %install_root.display(),
            config = %config_path.display(),
            game_path = %game_path.display(),
            game_id = %plan.game_id,
            "Spawning ScummVM install via flatpak"
        );
        return spawn_and_verify(cmd, &emulator.name, Vec::new());
    }

    let mut cmd = Command::new(&exe_path);
    cmd.arg(format!("--config={}", config_path.display()));
    cmd.args(&plan.extra_args);
    cmd.arg("-p").arg(&game_path);
    cmd.arg(&plan.game_id);
    cmd.current_dir(install_root);
    tracing::info!(
        command = ?cmd,
        install_root = %install_root.display(),
        config = %config_path.display(),
        game_path = %game_path.display(),
        game_id = %plan.game_id,
        "Spawning ScummVM install natively"
    );
    spawn_and_verify(cmd, &emulator.name, Vec::new())
}

fn normalize_prepared_install_relative_path(path: &str) -> PathBuf {
    let trimmed = path.trim().trim_matches('"').trim_matches('\'');
    let trimmed = trimmed.strip_prefix("./").unwrap_or(trimmed);
    PathBuf::from(trimmed)
}

fn ensure_scummvm_config_file(install_root: &Path, relative_path: &str) -> Result<PathBuf, String> {
    let config_path = install_root.join(normalize_prepared_install_relative_path(relative_path));
    if config_path.exists() {
        return Ok(config_path);
    }

    if let Some(parent) = config_path.parent() {
        std::fs::create_dir_all(parent)
            .map_err(|e| format!("Failed to create {}: {}", parent.display(), e))?;
    }

    std::fs::write(&config_path, DEFAULT_SCUMMVM_CONFIG).map_err(|e| {
        format!(
            "Failed to create ScummVM config {}: {}",
            config_path.display(),
            e
        )
    })?;

    Ok(config_path)
}

fn prepare_dosbox_exception_files(
    install_root: &Path,
    plan: &crate::exo::DosboxExceptionPlan,
) -> Result<Vec<PathBuf>, String> {
    let mut cleanup_paths = Vec::new();

    if plan.copy_mt32_roms {
        let mt32_dir = install_root.join("mt32");
        if !mt32_dir.exists() {
            return Err(format!(
                "This eXo exception launcher expects MT-32 ROMs in {}, but that directory is missing.",
                mt32_dir.display()
            ));
        }

        for entry in std::fs::read_dir(&mt32_dir)
            .map_err(|e| format!("Failed to read {}: {}", mt32_dir.display(), e))?
        {
            let entry = entry.map_err(|e| format!("Failed to read mt32 entry: {}", e))?;
            let path = entry.path();
            let is_rom = path
                .extension()
                .and_then(|ext| ext.to_str())
                .map(|ext| ext.eq_ignore_ascii_case("rom"))
                .unwrap_or(false);
            if !is_rom {
                continue;
            }

            let target_path = install_root.join(entry.file_name());
            if target_path.exists() {
                continue;
            }

            std::fs::copy(&path, &target_path).map_err(|e| {
                format!(
                    "Failed to copy {} to {}: {}",
                    path.display(),
                    target_path.display(),
                    e
                )
            })?;
            cleanup_paths.push(target_path);
        }
    }

    Ok(cleanup_paths)
}

/// Find the RetroArch executable path for native launches.
fn find_retroarch_binary() -> Option<PathBuf> {
    match current_os() {
        "Windows" => which::which("retroarch")
            .or_else(|_| which::which("retroarch.exe"))
            .ok(),
        "macOS" => {
            if let Ok(path) = which::which("retroarch") {
                Some(path)
            } else {
                let app_binary =
                    PathBuf::from("/Applications/RetroArch.app/Contents/MacOS/RetroArch");
                if app_binary.exists() {
                    Some(app_binary)
                } else {
                    None
                }
            }
        }
        _ => which::which("retroarch").ok(),
    }
}

/// Convert a host path into the Flatpak-visible path on Linux.
fn map_path_for_flatpak(path: &str) -> String {
    if current_os() != "Linux" {
        return path.to_string();
    }

    // On systems where /var/home does not exist (common non-ostree Linux),
    // keep canonical /home paths as-is.
    if !std::path::Path::new("/var/home").exists() {
        return path.to_string();
    }

    let Some(home) = dirs::home_dir() else {
        return path.to_string();
    };
    let home_path = home.to_string_lossy().to_string();

    // If the host home is already under /var/home, no rewrite needed.
    if home_path.starts_with("/var/home/") {
        return path.to_string();
    }

    let username = home
        .file_name()
        .and_then(|name| name.to_str())
        .unwrap_or("user");

    let home_with_sep = format!("{}/", home_path);
    if let Some(rest) = path.strip_prefix(&home_with_sep) {
        return format!("/var/home/{}/{}", username, rest);
    }
    if path == home_path {
        return format!("/var/home/{}", username);
    }

    path.to_string()
}

fn flatpak_filesystem_mount_point(path: &Path) -> Option<PathBuf> {
    let normalized = std::fs::canonicalize(path).unwrap_or_else(|_| path.to_path_buf());
    if normalized.is_dir() {
        Some(normalized)
    } else {
        normalized.parent().map(Path::to_path_buf)
    }
}

fn flatpak_filesystem_args(paths: Vec<(&Path, bool)>) -> Vec<String> {
    let mut mounts: BTreeMap<String, bool> = BTreeMap::new();

    for (path, read_only) in paths {
        let Some(mount_point) = flatpak_filesystem_mount_point(path) else {
            continue;
        };
        let mapped = map_path_for_flatpak(&mount_point.to_string_lossy());
        mounts
            .entry(mapped)
            .and_modify(|existing_ro| *existing_ro &= read_only)
            .or_insert(read_only);
    }

    mounts
        .into_iter()
        .map(|(path, read_only)| {
            if read_only {
                format!("--filesystem={path}:ro")
            } else {
                format!("--filesystem={path}")
            }
        })
        .collect()
}

fn add_flatpak_filesystem_args(cmd: &mut Command, paths: Vec<(&Path, bool)>) {
    for arg in flatpak_filesystem_args(paths) {
        cmd.arg(arg);
    }
}

/// Launch RetroArch with a specific core (optionally with a ROM)
fn launch_retroarch(
    core_name: &str,
    rom_path: Option<&str>,
    cleanup_paths: Vec<PathBuf>,
    launch_args: &[LaunchArg],
) -> Result<u32, String> {
    let core_path = check_retroarch_core_installed(core_name)
        .ok_or_else(|| format!("RetroArch core '{}' is not installed", core_name))?;
    let core_path_str = core_path.to_string_lossy().to_string();

    tracing::debug!(
        core = %core_name,
        core_path = %core_path_str,
        rom = ?rom_path,
        "Launching RetroArch with explicit core"
    );

    match current_os() {
        "Linux" => {
            let is_flatpak = is_flatpak_installed("org.libretro.RetroArch");
            if is_flatpak {
                let mut cmd = Command::new("flatpak");
                cmd.arg("run");
                let mut filesystem_paths = vec![(core_path.as_path(), true)];
                if let Some(rom) = rom_path {
                    filesystem_paths.push((Path::new(rom), false));
                }
                for arg in launch_args {
                    if let LaunchArg::Path(path) = arg {
                        filesystem_paths.push((Path::new(path), false));
                    }
                }
                add_flatpak_filesystem_args(&mut cmd, filesystem_paths);
                cmd.arg("org.libretro.RetroArch");
                cmd.arg("--verbose");
                cmd.arg("-L").arg(map_path_for_flatpak(&core_path_str));
                append_launch_args_for_flatpak(&mut cmd, launch_args);
                if let Some(rom) = rom_path {
                    cmd.arg(map_path_for_flatpak(rom));
                }
                tracing::info!(
                    command = ?cmd,
                    core = %core_name,
                    rom = ?rom_path,
                    "Spawning RetroArch via flatpak"
                );
                spawn_and_verify(cmd, "RetroArch", cleanup_paths)
            } else {
                let retroarch_path = find_retroarch_binary()
                    .ok_or_else(|| "Could not find RetroArch executable".to_string())?;
                let mut cmd = Command::new(retroarch_path);
                cmd.arg("--verbose");
                cmd.arg("-L").arg(&core_path_str);
                append_launch_args_native(&mut cmd, launch_args);
                if let Some(rom) = rom_path {
                    cmd.arg(rom);
                }
                tracing::info!(
                    command = ?cmd,
                    core = %core_name,
                    rom = ?rom_path,
                    "Spawning RetroArch native"
                );
                spawn_and_verify(cmd, "RetroArch", cleanup_paths)
            }
        }
        "Windows" | "macOS" => {
            let retroarch_path = find_retroarch_binary()
                .ok_or_else(|| "Could not find RetroArch executable".to_string())?;
            let mut cmd = Command::new(retroarch_path);
            cmd.arg("--verbose");
            cmd.arg("-L").arg(&core_path_str);
            append_launch_args_native(&mut cmd, launch_args);
            if let Some(rom) = rom_path {
                cmd.arg(rom);
            }
            spawn_and_verify(cmd, "RetroArch", cleanup_paths)
        }
        _ => Err("Unsupported operating system".to_string()),
    }
}

/// Launch a standalone emulator (optionally with a ROM)
fn launch_standalone(
    emulator: &EmulatorInfo,
    rom_path: Option<&str>,
    cleanup_paths: Vec<PathBuf>,
    launch_args: &[LaunchArg],
) -> Result<u32, String> {
    tracing::debug!(emulator = %emulator.name, rom = ?rom_path, "Launching standalone emulator");

    // Use check_standalone_installation to skip RetroArch core check
    let exe_path = check_standalone_installation(emulator).ok_or_else(|| {
        let err = format!("{} standalone is not installed", emulator.name);
        tracing::error!(error = %err, "Emulator not installed");
        err
    })?;

    tracing::debug!(exe_path = ?exe_path, "Found emulator executable");

    if exe_path.to_string_lossy().starts_with("flatpak::") {
        // Flatpak app
        let app_id = exe_path.to_string_lossy().replace("flatpak::", "");
        let mut cmd = Command::new("flatpak");
        cmd.arg("run");
        let mut filesystem_paths = Vec::new();
        if let Some(rom) = rom_path {
            filesystem_paths.push((Path::new(rom), false));
        }
        for arg in launch_args {
            if let LaunchArg::Path(path) = arg {
                filesystem_paths.push((Path::new(path), false));
            }
        }
        if !filesystem_paths.is_empty() {
            add_flatpak_filesystem_args(&mut cmd, filesystem_paths);
        }
        cmd.arg(&app_id);
        append_standalone_rom_and_args_for_flatpak(&mut cmd, &emulator.name, rom_path, launch_args);
        tracing::info!(command = ?cmd, app_id = %app_id, "Spawning via flatpak");
        spawn_and_verify(cmd, &emulator.name, cleanup_paths)
    } else if current_os() == "macOS" && exe_path.extension().map(|e| e == "app").unwrap_or(false) {
        // macOS .app bundle
        let mut cmd = Command::new("open");
        cmd.arg("-a").arg(exe_path.to_str().unwrap_or_default());
        append_standalone_rom_and_args_native(&mut cmd, &emulator.name, rom_path, launch_args);
        tracing::info!(command = ?cmd, "Spawning macOS app");
        spawn_and_verify(cmd, &emulator.name, cleanup_paths)
    } else {
        // Regular executable
        let mut cmd = Command::new(&exe_path);
        append_standalone_rom_and_args_native(&mut cmd, &emulator.name, rom_path, launch_args);
        tracing::info!(command = ?cmd, "Spawning native executable");
        spawn_and_verify(cmd, &emulator.name, cleanup_paths)
    }
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
        firmware_statuses: Vec::new(),
    }
}

fn append_launch_args_native(cmd: &mut Command, args: &[LaunchArg]) {
    for arg in args {
        match arg {
            LaunchArg::Literal(value) | LaunchArg::Path(value) => {
                cmd.arg(value);
            }
        }
    }
}

fn append_standalone_rom_and_args_native(
    cmd: &mut Command,
    emulator_name: &str,
    rom_path: Option<&str>,
    args: &[LaunchArg],
) {
    if emulator_name == "LoopyMSE" {
        if let Some(rom) = rom_path {
            cmd.arg(rom);
        }
        append_launch_args_native(cmd, args);
    } else {
        append_launch_args_native(cmd, args);
        if let Some(rom) = rom_path {
            cmd.arg(rom);
        }
    }
}

fn append_launch_args_for_flatpak(cmd: &mut Command, args: &[LaunchArg]) {
    for arg in args {
        match arg {
            LaunchArg::Literal(value) => {
                cmd.arg(value);
            }
            LaunchArg::Path(path) => {
                cmd.arg(map_path_for_flatpak(path));
            }
        }
    }
}

fn append_standalone_rom_and_args_for_flatpak(
    cmd: &mut Command,
    emulator_name: &str,
    rom_path: Option<&str>,
    args: &[LaunchArg],
) {
    if emulator_name == "LoopyMSE" {
        if let Some(rom) = rom_path {
            cmd.arg(map_path_for_flatpak(rom));
        }
        append_launch_args_for_flatpak(cmd, args);
    } else {
        append_launch_args_for_flatpak(cmd, args);
        if let Some(rom) = rom_path {
            cmd.arg(map_path_for_flatpak(rom));
        }
    }
}

/// Add status for an emulator as a RetroArch core entry
pub fn add_status_as_retroarch(emulator: EmulatorInfo) -> EmulatorWithStatus {
    let core_name = emulator
        .retroarch_core
        .as_ref()
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
        firmware_statuses: Vec::new(),
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
        firmware_statuses: Vec::new(),
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

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs::File;

    #[test]
    fn extracts_zip_rom_into_same_directory() {
        let temp_dir = tempfile::tempdir().unwrap();
        let archive_path = temp_dir.path().join("game.zip");

        let archive_file = File::create(&archive_path).unwrap();
        let mut writer = zip::ZipWriter::new(archive_file);
        writer
            .start_file::<_, ()>(
                "Super Mario Bros. 3 (USA).nes",
                zip::write::FileOptions::default(),
            )
            .unwrap();
        writer.write_all(b"NES ROM").unwrap();
        writer.finish().unwrap();

        let prepared =
            extract_launchable_content_from_archive(&archive_path, ArchiveKind::Zip).unwrap();
        let extracted_path = PathBuf::from(prepared.rom_path.unwrap());

        assert_eq!(extracted_path.parent(), archive_path.parent());
        assert_eq!(
            extracted_path.file_name().and_then(|name| name.to_str()),
            Some("Super Mario Bros. 3 (USA).nes")
        );
        assert_eq!(std::fs::read(&extracted_path).unwrap(), b"NES ROM");
        assert_eq!(prepared.cleanup_paths, vec![extracted_path]);
    }

    #[test]
    fn avoids_overwriting_existing_rom_when_extracting_zip() {
        let temp_dir = tempfile::tempdir().unwrap();
        let archive_path = temp_dir.path().join("game.zip");
        let existing_path = temp_dir.path().join("Existing Game.nes");
        std::fs::write(&existing_path, b"existing").unwrap();

        let archive_file = File::create(&archive_path).unwrap();
        let mut writer = zip::ZipWriter::new(archive_file);
        writer
            .start_file::<_, ()>("Existing Game.nes", zip::write::FileOptions::default())
            .unwrap();
        writer.write_all(b"new").unwrap();
        writer.finish().unwrap();

        let prepared =
            extract_launchable_content_from_archive(&archive_path, ArchiveKind::Zip).unwrap();
        let extracted_path = PathBuf::from(prepared.rom_path.unwrap());

        assert_ne!(extracted_path, existing_path);
        assert!(extracted_path
            .file_name()
            .and_then(|name| name.to_str())
            .unwrap()
            .starts_with("Existing Game.lunchbox-"));
        assert_eq!(std::fs::read(&existing_path).unwrap(), b"existing");
        assert_eq!(std::fs::read(&extracted_path).unwrap(), b"new");
        assert_eq!(prepared.cleanup_paths, vec![extracted_path]);
    }

    #[test]
    fn extracts_cue_with_referenced_bin_from_zip_without_deleting_sidecars_blindly() {
        let temp_dir = tempfile::tempdir().unwrap();
        let archive_path = temp_dir.path().join("disc.zip");

        let archive_file = File::create(&archive_path).unwrap();
        let mut writer = zip::ZipWriter::new(archive_file);
        writer
            .start_file::<_, ()>("Disc/Game.cue", zip::write::FileOptions::default())
            .unwrap();
        writer
            .write_all(b"FILE \"Game (Track 1).bin\" BINARY\n  TRACK 01 MODE1/2352\n")
            .unwrap();
        writer
            .start_file::<_, ()>(
                "Disc/Game (Track 1).bin",
                zip::write::FileOptions::default(),
            )
            .unwrap();
        writer.write_all(b"TRACK").unwrap();
        writer.finish().unwrap();

        let prepared =
            extract_launchable_content_from_archive(&archive_path, ArchiveKind::Zip).unwrap();
        let rom_path = PathBuf::from(prepared.rom_path.unwrap());
        let cleanup_paths = prepared.cleanup_paths;

        assert_eq!(rom_path, temp_dir.path().join("Disc/Game.cue"));
        assert!(cleanup_paths.contains(&temp_dir.path().join("Disc/Game.cue")));
        assert!(cleanup_paths.contains(&temp_dir.path().join("Disc/Game (Track 1).bin")));
        assert_eq!(
            std::fs::read(temp_dir.path().join("Disc/Game (Track 1).bin")).unwrap(),
            b"TRACK"
        );
    }

    #[test]
    fn flatpak_filesystem_args_mount_rom_directory_writable() {
        let temp_dir = tempfile::tempdir().unwrap();
        let rom_dir = temp_dir.path().join("roms");
        std::fs::create_dir_all(&rom_dir).unwrap();
        let rom_path = rom_dir.join("game.nes");
        std::fs::write(&rom_path, b"rom").unwrap();

        let args = flatpak_filesystem_args(vec![(rom_path.as_path(), false)]);

        assert_eq!(
            args,
            vec![format!("--filesystem={}", rom_dir.to_string_lossy())]
        );
    }

    #[test]
    fn flatpak_filesystem_args_deduplicate_and_prefer_writable_mounts() {
        let temp_dir = tempfile::tempdir().unwrap();
        let shared_dir = temp_dir.path().join("shared");
        std::fs::create_dir_all(&shared_dir).unwrap();
        let rom_path = shared_dir.join("game.nes");
        let helper_path = shared_dir.join("helper.dat");
        std::fs::write(&rom_path, b"rom").unwrap();
        std::fs::write(&helper_path, b"helper").unwrap();

        let args = flatpak_filesystem_args(vec![
            (rom_path.as_path(), false),
            (helper_path.as_path(), true),
        ]);

        assert_eq!(
            args,
            vec![format!("--filesystem={}", shared_dir.to_string_lossy())]
        );
    }

    #[test]
    fn loopymse_places_rom_before_firmware_args() {
        let mut cmd = Command::new("echo");
        append_standalone_rom_and_args_native(
            &mut cmd,
            "LoopyMSE",
            Some("/roms/game.bin"),
            &[
                LaunchArg::Path("/firmware/main.bin".to_string()),
                LaunchArg::Path("/firmware/sound.bin".to_string()),
            ],
        );

        let args = cmd
            .get_args()
            .map(|arg| arg.to_string_lossy().to_string())
            .collect::<Vec<_>>();

        assert_eq!(
            args,
            vec![
                "/roms/game.bin".to_string(),
                "/firmware/main.bin".to_string(),
                "/firmware/sound.bin".to_string(),
            ]
        );
    }

    #[test]
    fn prepare_dosbox_exception_files_copies_mt32_roms_and_tracks_cleanup() {
        let temp_dir = tempfile::tempdir().unwrap();
        let mt32_dir = temp_dir.path().join("mt32");
        std::fs::create_dir_all(&mt32_dir).unwrap();
        std::fs::write(mt32_dir.join("MT32_CONTROL.ROM"), b"control").unwrap();
        std::fs::write(mt32_dir.join("MT32_PCM.ROM"), b"pcm").unwrap();
        std::fs::write(mt32_dir.join("SoundCanvas.sf2"), b"sf2").unwrap();

        let cleanup_paths = prepare_dosbox_exception_files(
            temp_dir.path(),
            &crate::exo::DosboxExceptionPlan {
                copy_mt32_roms: true,
            },
        )
        .unwrap();

        assert_eq!(cleanup_paths.len(), 2);
        assert!(cleanup_paths.contains(&temp_dir.path().join("MT32_CONTROL.ROM")));
        assert!(cleanup_paths.contains(&temp_dir.path().join("MT32_PCM.ROM")));
        assert_eq!(
            std::fs::read(temp_dir.path().join("MT32_CONTROL.ROM")).unwrap(),
            b"control"
        );
        assert!(!cleanup_paths.contains(&temp_dir.path().join("SoundCanvas.sf2")));
    }

    #[test]
    fn detects_win9x_86box_launcher_variants() {
        let temp_dir = tempfile::tempdir().unwrap();
        let metadata_dir = temp_dir.path().join("Daytona USA (1996)");
        std::fs::create_dir_all(&metadata_dir).unwrap();

        std::fs::write(
            metadata_dir.join("Daytona USA (1996).bat"),
            b"@echo off\r\n.\\util\\9xlaunch86Box.bat\r\n",
        )
        .unwrap();

        assert_eq!(
            detect_win9x_launcher_kind(&metadata_dir).unwrap(),
            Win9xLauncherKind::EightySixBox(Win9xEightySixBoxPlan {
                config_name: "Play.cfg",
                parent_vhd_name: "W98-P.vhd",
                child_vhd_name: "W98-C.vhd",
            })
        );
    }

    #[test]
    fn prepares_86box_vm_root_from_parent_disk_and_metadata_cfg() {
        let temp_dir = tempfile::tempdir().unwrap();
        let install_root = temp_dir.path().join("install");
        let metadata_dir = install_root.join("eXoWin9x/!win9x/1996/Daytona USA (1996)");
        let vm_root = install_root.join("emulators/86Box98");
        std::fs::create_dir_all(metadata_dir.clone()).unwrap();
        std::fs::create_dir_all(vm_root.join("parent")).unwrap();

        let launch_config_path = metadata_dir.join("Play.cfg");
        std::fs::write(
            &launch_config_path,
            b"[Hard disks]\nhdd_01_fn = W98-C.vhd\n",
        )
        .unwrap();
        std::fs::write(vm_root.join("parent/W98-P.vhd"), b"parent").unwrap();

        prepare_86box_vm_root(
            &install_root,
            &launch_config_path,
            &Win9xEightySixBoxPlan {
                config_name: "Play.cfg",
                parent_vhd_name: "W98-P.vhd",
                child_vhd_name: "W98-C.vhd",
            },
            &vm_root,
            &vm_root.join("86box.cfg"),
        )
        .unwrap();

        assert_eq!(std::fs::read(vm_root.join("W98-C.vhd")).unwrap(), b"parent");
        assert_eq!(
            std::fs::read(vm_root.join("86box.cfg")).unwrap(),
            b"[Hard disks]\nhdd_01_fn = W98-C.vhd\n"
        );
    }

    #[test]
    fn detects_win9x_pcbox_launcher_variant() {
        let temp_dir = tempfile::tempdir().unwrap();
        let metadata_dir = temp_dir.path().join("Need for Speed II SE (1997)");
        std::fs::create_dir_all(&metadata_dir).unwrap();

        std::fs::write(
            metadata_dir.join("Need for Speed II SE (1997).bat"),
            b"@echo off\r\n.\\util\\9xlaunchPCBox.bat\r\n",
        )
        .unwrap();

        assert_eq!(
            detect_win9x_launcher_kind(&metadata_dir).unwrap(),
            Win9xLauncherKind::PcBox(Win9xPcBoxPlan {
                config_name: "Play.cfg",
                parent_vhd_name: "W98-P.vhd",
                child_vhd_name: "W98-C.vhd",
            })
        );
    }

    #[test]
    fn prepares_pcbox_vm_root_from_parent_disk_and_metadata_cfg() {
        let temp_dir = tempfile::tempdir().unwrap();
        let install_root = temp_dir.path().join("install");
        let metadata_dir = install_root.join("eXoWin9x/!win9x/1997/Need for Speed II SE (1997)");
        let vm_root = install_root.join("emulators/PCBox");
        std::fs::create_dir_all(metadata_dir.clone()).unwrap();
        std::fs::create_dir_all(vm_root.join("parent")).unwrap();

        let launch_config_path = metadata_dir.join("Play.cfg");
        std::fs::write(
            &launch_config_path,
            b"[Hard disks]\nhdd_01_fn = W98-C.vhd\n",
        )
        .unwrap();
        std::fs::write(vm_root.join("parent/W98-P.vhd"), b"pcboxparent").unwrap();

        prepare_pcbox_vm_root(
            &launch_config_path,
            &Win9xPcBoxPlan {
                config_name: "Play.cfg",
                parent_vhd_name: "W98-P.vhd",
                child_vhd_name: "W98-C.vhd",
            },
            &vm_root,
            &vm_root.join("play.cfg"),
        )
        .unwrap();

        assert_eq!(
            std::fs::read(vm_root.join("W98-C.vhd")).unwrap(),
            b"pcboxparent"
        );
        assert_eq!(
            std::fs::read(vm_root.join("play.cfg")).unwrap(),
            b"[Hard disks]\nhdd_01_fn = W98-C.vhd\n"
        );
    }
}
