//! Emulator detection, installation, and launching
//!
//! This module handles:
//! - Detecting if emulators are installed on the system
//! - Installing emulators via package managers (flatpak/nix/winget/homebrew)
//! - Launching games with the appropriate emulator

use crate::db::schema::EmulatorInfo;
use crate::firmware::FirmwareStatus;
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet, HashSet};
use std::io::{Read, Write};
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
#[cfg(target_os = "linux")]
use std::{fs::Permissions, os::unix::fs::PermissionsExt};

const FLATPAK_INSTALL_PREFIX: &str = "flatpak::";
const APPIMAGE_INSTALL_PREFIX: &str = "appimage::";
const WINE_INSTALL_PREFIX: &str = "wine::";

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct WineInstallInfo {
    slug: &'static str,
    download_page_url: &'static str,
    executable_candidates: &'static [&'static str],
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct AppImageInstallInfo {
    slug: &'static str,
    github_repo: &'static str,
    preferred_asset_terms: &'static [&'static str],
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct AppImageUpdaterInfo {
    slug: &'static str,
    github_repo: &'static str,
    preferred_asset_terms: &'static [&'static str],
}

#[cfg(target_os = "linux")]
#[derive(Debug, Clone, Deserialize)]
struct GitHubRelease {
    tag_name: String,
    assets: Vec<GitHubReleaseAsset>,
}

#[cfg(target_os = "linux")]
#[derive(Debug, Clone, Deserialize)]
struct GitHubReleaseAsset {
    name: String,
    browser_download_url: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct AppImageInstallManifest {
    slug: String,
    version: String,
    github_repo: String,
    asset_name: String,
    download_url: String,
    installed_at: String,
    update_transport: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
enum FlatpakScope {
    User,
    System,
}

impl FlatpakScope {
    fn flag(self) -> &'static str {
        match self {
            Self::User => "--user",
            Self::System => "--system",
        }
    }

    fn key_prefix(self) -> &'static str {
        match self {
            Self::User => "user",
            Self::System => "system",
        }
    }

    fn label(self) -> &'static str {
        match self {
            Self::User => "Flatpak (user)",
            Self::System => "Flatpak (system)",
        }
    }
}

#[derive(Debug, Default)]
struct ManagedUpdateTarget {
    display_names: BTreeSet<String>,
}

#[derive(Debug, Deserialize)]
struct NixProfileList {
    elements: BTreeMap<String, NixProfileListElement>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct NixProfileListElement {
    active: bool,
    #[serde(default)]
    attr_path: Option<String>,
    #[serde(default)]
    store_paths: Vec<String>,
}

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
    /// The uninstall method available for the current installed instance, if Lunchbox owns it
    pub uninstall_method: Option<String>,
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

/// An explicit update available for a Lunchbox-managed emulator install.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct EmulatorUpdate {
    /// Stable key used to perform the update.
    pub key: String,
    /// User-facing emulator name for the update pane.
    pub display_name: String,
    /// Backend handling the update (flatpak, nix).
    pub install_method: String,
    /// User-facing backend/scope label.
    pub source_label: String,
    /// Currently installed version, when available.
    pub current_version: Option<String>,
    /// Available version, when available.
    pub available_version: Option<String>,
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
            if emulator.flatpak_id.is_some() && is_flatpak_available() {
                Some("flatpak".to_string())
            } else if nix_package_for_emulator(emulator).is_some() && is_nix_available() {
                Some("nix".to_string())
            } else if appimage_install_for_emulator(emulator).is_some() {
                Some("appimage".to_string())
            } else if wine_install_for_emulator(emulator).is_some() && is_wine_available() {
                Some("wine".to_string())
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

fn get_retroarch_install_method() -> Option<String> {
    match current_os() {
        "Linux" => {
            if is_flatpak_available() {
                Some("flatpak".to_string())
            } else if is_nix_available() {
                Some("nix".to_string())
            } else {
                None
            }
        }
        "Windows" => Some("winget".to_string()),
        "macOS" => Some("homebrew".to_string()),
        _ => None,
    }
}

fn get_uninstall_method_for_path(path: &Path) -> Option<String> {
    let path_text = path.to_string_lossy();
    if path_text.starts_with(FLATPAK_INSTALL_PREFIX) {
        Some("flatpak".to_string())
    } else if path_text.starts_with(APPIMAGE_INSTALL_PREFIX)
        || path.starts_with(lunchbox_appimage_root())
    {
        Some("appimage".to_string())
    } else if path_text.starts_with(WINE_INSTALL_PREFIX) {
        Some("wine".to_string())
    } else if path.starts_with(lunchbox_nix_profile_bin_dir()) {
        Some("nix".to_string())
    } else {
        None
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
            return Some(PathBuf::from(format!(
                "{FLATPAK_INSTALL_PREFIX}{flatpak_id}"
            )));
        }
    }

    if let Some(info) = wine_install_for_emulator(emulator) {
        if is_wine_available() {
            if let Some(path) = find_wine_install_executable(&info) {
                return Some(PathBuf::from(format!(
                    "{WINE_INSTALL_PREFIX}{}",
                    path.to_string_lossy()
                )));
            }
        }
    }

    if let Some(info) = appimage_install_for_emulator(emulator) {
        if let Some(path) = find_appimage_install_executable(&info) {
            return Some(path);
        }
    }

    if let Some(path) = find_in_lunchbox_nix_profile(&get_executable_names(&emulator.name)) {
        return Some(path);
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
            if find_lunchbox_nix_profile_executable("retroarch").is_some() {
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

fn is_flatpak_available() -> bool {
    #[cfg(target_os = "linux")]
    {
        which::which("flatpak").is_ok()
    }
    #[cfg(not(target_os = "linux"))]
    {
        false
    }
}

fn is_nix_available() -> bool {
    #[cfg(target_os = "linux")]
    {
        which::which("nix").is_ok()
    }
    #[cfg(not(target_os = "linux"))]
    {
        false
    }
}

fn find_wine_command() -> Option<PathBuf> {
    #[cfg(target_os = "linux")]
    {
        which::which("wine")
            .or_else(|_| which::which("wine64"))
            .ok()
    }
    #[cfg(not(target_os = "linux"))]
    {
        None
    }
}

fn find_winepath_command() -> Option<PathBuf> {
    #[cfg(target_os = "linux")]
    {
        which::which("winepath").ok()
    }
    #[cfg(not(target_os = "linux"))]
    {
        None
    }
}

fn is_wine_available() -> bool {
    find_wine_command().is_some() && find_winepath_command().is_some()
}

/// Check if a flatpak app is installed
fn is_flatpak_installed(app_id: &str) -> bool {
    #[cfg(target_os = "linux")]
    {
        if !is_flatpak_available() {
            return false;
        }
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

fn lunchbox_nix_profile_path() -> PathBuf {
    let state_root = dirs::state_dir()
        .or_else(|| dirs::home_dir().map(|home| home.join(".local/state")))
        .unwrap_or_else(|| PathBuf::from("."));
    state_root.join("nix/profiles/lunchbox")
}

fn lunchbox_nix_profile_bin_dir() -> PathBuf {
    lunchbox_nix_profile_path().join("bin")
}

fn find_lunchbox_nix_profile_executable(name: &str) -> Option<PathBuf> {
    let path = lunchbox_nix_profile_bin_dir().join(name);
    path.exists().then_some(path)
}

fn find_in_lunchbox_nix_profile(names: &[String]) -> Option<PathBuf> {
    names
        .iter()
        .find_map(|name| find_lunchbox_nix_profile_executable(name))
}

fn lunchbox_programs_dir() -> PathBuf {
    directories::ProjectDirs::from("", "", crate::db::APP_DATA_DIR)
        .map(|dirs| dirs.data_dir().join("programs"))
        .unwrap_or_else(|| PathBuf::from(".").join("programs"))
}

fn lunchbox_appimage_root() -> PathBuf {
    lunchbox_programs_dir().join("appimages")
}

fn lunchbox_appimage_tool_root() -> PathBuf {
    lunchbox_programs_dir().join("appimage-tools")
}

fn lunchbox_wine_root() -> PathBuf {
    lunchbox_programs_dir().join("wine")
}

fn appimage_install_for_emulator(emulator: &EmulatorInfo) -> Option<AppImageInstallInfo> {
    match emulator.name.to_ascii_lowercase().as_str() {
        "hypseus singe" => Some(AppImageInstallInfo {
            slug: "hypseus-singe",
            github_repo: "DirtBagXon/hypseus-singe",
            preferred_asset_terms: &["appimage", "steamdeck", "x86_64"],
        }),
        _ => None,
    }
}

fn appimage_install_for_slug(slug: &str) -> Option<AppImageInstallInfo> {
    match slug {
        "hypseus-singe" => Some(AppImageInstallInfo {
            slug: "hypseus-singe",
            github_repo: "DirtBagXon/hypseus-singe",
            preferred_asset_terms: &["appimage", "steamdeck", "x86_64"],
        }),
        _ => None,
    }
}

fn appimage_updater_info() -> AppImageUpdaterInfo {
    AppImageUpdaterInfo {
        slug: "appimageupdate",
        github_repo: "AppImageCommunity/AppImageUpdate",
        preferred_asset_terms: &["appimageupdatetool", "x86_64"],
    }
}

fn wine_install_for_emulator(emulator: &EmulatorInfo) -> Option<WineInstallInfo> {
    match emulator.name.to_ascii_lowercase().as_str() {
        "altirra" => Some(WineInstallInfo {
            slug: "altirra",
            download_page_url: "https://www.virtualdub.org/altirra.html",
            executable_candidates: &["Altirra64.exe", "Altirra.exe"],
        }),
        _ => None,
    }
}

pub fn is_emulator_visible_on_current_os(emulator: &EmulatorInfo) -> bool {
    let os = current_os();
    let supported = emulator
        .supported_os
        .as_deref()
        .map(|supported_os| supported_os.split(';').any(|entry| entry.trim() == os))
        .unwrap_or(true);

    if supported {
        return true;
    }

    matches!(os, "Linux") && wine_install_for_emulator(emulator).is_some()
}

fn wine_install_root(info: &WineInstallInfo) -> PathBuf {
    lunchbox_wine_root().join(info.slug)
}

fn appimage_install_root(info: &AppImageInstallInfo) -> PathBuf {
    lunchbox_appimage_root().join(info.slug)
}

fn appimage_updater_root(info: &AppImageUpdaterInfo) -> PathBuf {
    lunchbox_appimage_tool_root().join(info.slug)
}

fn appimage_version_dir(info: &AppImageInstallInfo, version: &str) -> PathBuf {
    appimage_install_root(info).join(version)
}

fn appimage_updater_version_dir(info: &AppImageUpdaterInfo, version: &str) -> PathBuf {
    appimage_updater_root(info).join(version)
}

fn appimage_current_link(info: &AppImageInstallInfo) -> PathBuf {
    appimage_install_root(info).join("current")
}

fn appimage_updater_current_link(info: &AppImageUpdaterInfo) -> PathBuf {
    appimage_updater_root(info).join("current")
}

fn appimage_manifest_path(info: &AppImageInstallInfo) -> PathBuf {
    appimage_install_root(info).join("install.json")
}

fn wine_app_dir(info: &WineInstallInfo) -> PathBuf {
    wine_install_root(info).join("app")
}

fn wine_prefix_dir(info: &WineInstallInfo) -> PathBuf {
    wine_install_root(info).join("prefix")
}

fn find_wine_install_executable(info: &WineInstallInfo) -> Option<PathBuf> {
    find_file_by_name_recursive(&wine_app_dir(info), info.executable_candidates)
}

fn read_appimage_manifest(info: &AppImageInstallInfo) -> Option<AppImageInstallManifest> {
    let path = appimage_manifest_path(info);
    let bytes = std::fs::read(path).ok()?;
    serde_json::from_slice(&bytes).ok()
}

fn write_appimage_manifest(
    info: &AppImageInstallInfo,
    manifest: &AppImageInstallManifest,
) -> Result<(), String> {
    let root = appimage_install_root(info);
    std::fs::create_dir_all(&root)
        .map_err(|e| format!("Failed to create AppImage install directory: {}", e))?;
    let bytes = serde_json::to_vec_pretty(manifest)
        .map_err(|e| format!("Failed to serialize AppImage manifest: {}", e))?;
    std::fs::write(appimage_manifest_path(info), bytes)
        .map_err(|e| format!("Failed to write AppImage manifest: {}", e))
}

fn find_appimage_install_executable(info: &AppImageInstallInfo) -> Option<PathBuf> {
    if let Some(manifest) = read_appimage_manifest(info) {
        let candidate = appimage_version_dir(info, &manifest.version).join(&manifest.asset_name);
        if candidate.exists() {
            return Some(candidate);
        }
    }

    let current = appimage_current_link(info);
    let current_dir = std::fs::read_link(&current).ok().map(|path| {
        if path.is_absolute() {
            path
        } else {
            current.parent().unwrap_or_else(|| Path::new("")).join(path)
        }
    })?;

    let mut entries = std::fs::read_dir(current_dir).ok()?;
    entries.find_map(|entry| {
        let path = entry.ok()?.path();
        let is_appimage = path
            .extension()
            .and_then(|ext| ext.to_str())
            .map(|ext| ext.eq_ignore_ascii_case("appimage"))
            .unwrap_or(false);
        is_appimage.then_some(path)
    })
}

fn find_file_by_name_recursive(root: &Path, candidates: &[&str]) -> Option<PathBuf> {
    if !root.exists() {
        return None;
    }

    let wanted = candidates
        .iter()
        .map(|name| name.to_ascii_lowercase())
        .collect::<HashSet<_>>();
    let mut stack = vec![root.to_path_buf()];

    while let Some(dir) = stack.pop() {
        let entries = match std::fs::read_dir(&dir) {
            Ok(entries) => entries,
            Err(_) => continue,
        };

        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_dir() {
                stack.push(path);
                continue;
            }

            let Some(file_name) = path.file_name().and_then(|name| name.to_str()) else {
                continue;
            };
            if wanted.contains(&file_name.to_ascii_lowercase()) {
                return Some(path);
            }
        }
    }

    None
}

fn extract_altirra_download_href(html: &str) -> Option<String> {
    let start = html.find("downloads/Altirra-")?;
    let suffix = &html[start..];
    let end = suffix.find(".zip")?;
    Some(suffix[..end + 4].to_string())
}

async fn resolve_altirra_download_url(page_url: &str) -> Result<String, String> {
    let client = reqwest::Client::new();
    let response = client
        .get(page_url)
        .send()
        .await
        .map_err(|e| format!("Failed to fetch Altirra download page: {}", e))?;

    if !response.status().is_success() {
        return Err(format!(
            "Failed to fetch Altirra download page: HTTP {}",
            response.status()
        ));
    }

    let body = response
        .text()
        .await
        .map_err(|e| format!("Failed to read Altirra download page: {}", e))?;
    let href = extract_altirra_download_href(&body).ok_or_else(|| {
        "Could not find the latest Altirra x86/x64 ZIP on the official site".to_string()
    })?;

    let page = reqwest::Url::parse(page_url)
        .map_err(|e| format!("Invalid Altirra download page URL: {}", e))?;
    page.join(&href)
        .map(|url| url.to_string())
        .map_err(|e| format!("Failed to resolve Altirra download URL: {}", e))
}

#[cfg(target_os = "linux")]
async fn fetch_github_latest_release(repo: &str) -> Result<GitHubRelease, String> {
    let url = format!("https://api.github.com/repos/{repo}/releases/latest");
    reqwest::Client::new()
        .get(url)
        .header(reqwest::header::USER_AGENT, "lunchbox-appimage")
        .send()
        .await
        .map_err(|e| format!("Failed to fetch GitHub release metadata: {}", e))?
        .error_for_status()
        .map_err(|e| format!("GitHub release metadata request failed: {}", e))?
        .json::<GitHubRelease>()
        .await
        .map_err(|e| format!("Failed to parse GitHub release metadata: {}", e))
}

#[cfg(target_os = "linux")]
fn normalize_release_version(tag_name: &str) -> String {
    tag_name
        .strip_prefix('v')
        .unwrap_or(tag_name)
        .trim()
        .to_string()
}

#[cfg(target_os = "linux")]
fn select_github_appimage_asset_with_terms<'a>(
    release: &'a GitHubRelease,
    preferred_asset_terms: &[&str],
) -> Option<&'a GitHubReleaseAsset> {
    release
        .assets
        .iter()
        .filter(|asset| {
            let name = asset.name.to_ascii_lowercase();
            name.ends_with(".appimage")
                || (archive_kind_for_path(Path::new(&asset.name)).is_some()
                    && name.contains("appimage"))
        })
        .max_by_key(|asset| {
            let name = asset.name.to_ascii_lowercase();
            let mut score = 100i32;
            if name.ends_with(".appimage") {
                score += 100;
            }
            for term in preferred_asset_terms {
                if name.contains(&term.to_ascii_lowercase()) {
                    score += 25;
                }
            }
            if name.contains("x86_64") || name.contains("amd64") || name.contains("steamdeck") {
                score += 50;
            }
            score
        })
}

#[cfg(target_os = "linux")]
fn select_github_appimage_asset<'a>(
    release: &'a GitHubRelease,
    info: &AppImageInstallInfo,
) -> Option<&'a GitHubReleaseAsset> {
    select_github_appimage_asset_with_terms(release, info.preferred_asset_terms)
}

#[cfg(target_os = "linux")]
fn select_github_updater_asset<'a>(
    release: &'a GitHubRelease,
    info: &AppImageUpdaterInfo,
) -> Option<&'a GitHubReleaseAsset> {
    select_github_appimage_asset_with_terms(release, info.preferred_asset_terms)
}

fn nix_package_for_emulator(emulator: &EmulatorInfo) -> Option<&'static str> {
    match emulator.name.to_lowercase().as_str() {
        "ares" => Some("ares"),
        "atari++" => Some("ataripp"),
        "atari800" => Some("atari800"),
        "desmume" => Some("desmume"),
        "dolphin" => Some("dolphin-emu"),
        "dosbox staging" => Some("dosbox-staging"),
        "dosbox-x" => Some("dosbox-x"),
        "duckstation" => Some("duckstation"),
        "flycast" => Some("flycast"),
        "fs-uae" => Some("fsuae"),
        "hatari" => Some("hatari"),
        "mame" => Some("mame"),
        "mednafen" => Some("mednafen"),
        "mgba" => Some("mgba"),
        "melonds" => Some("melonds"),
        "openmsx" => Some("openmsx"),
        "pcsx2" => Some("pcsx2"),
        "ppsspp" => Some("ppsspp"),
        "scummvm" => Some("scummvm"),
        "snes9x" => Some("snes9x"),
        "stella" => Some("stella"),
        "vice" => Some("vice"),
        "vice (xpet)" => Some("vice"),
        "vice (xvic)" => Some("vice"),
        "xemu" => Some("xemu"),
        _ => None,
    }
}

/// Get possible executable names for an emulator
fn get_executable_names(name: &str) -> Vec<String> {
    let lower = name.to_lowercase();
    let mut names = vec![lower.clone()];

    // Add common variations
    match lower.as_str() {
        "atari++" => {
            names.push("ataripp".to_string());
        }
        "desmume" => {
            names.extend(["DeSmuME", "desmume-gtk"].iter().map(|s| s.to_string()));
        }
        "dolphin" => {
            names.extend(
                ["dolphin-emu", "dolphin-emu-qt"]
                    .iter()
                    .map(|s| s.to_string()),
            );
        }
        "fs-uae" => {
            names.push("fs-uae-launcher".to_string());
        }
        "vice" => {
            names.extend(
                ["x64sc", "x64", "x128", "xplus4"]
                    .iter()
                    .map(|s| s.to_string()),
            );
        }
        "vice (xpet)" => {
            names.push("xpet".to_string());
        }
        "vice (xvic)" => {
            names.push("xvic".to_string());
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
        "hypseus singe" => {
            names.extend(["hypseus", "singe"].iter().map(|s| s.to_string()));
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
            if let Some(flatpak_id) = emulator
                .flatpak_id
                .as_deref()
                .filter(|_| is_flatpak_available())
            {
                tracing::info!(flatpak_id = flatpak_id, "Installing via flatpak");
                install_flatpak(flatpak_id).await?;
                Ok(PathBuf::from(format!(
                    "{FLATPAK_INSTALL_PREFIX}{flatpak_id}"
                )))
            } else if let Some(package) =
                nix_package_for_emulator(emulator).filter(|_| is_nix_available())
            {
                tracing::info!(package = package, "Installing via nix profile");
                install_nix_package(package).await?;
                check_linux_installation(emulator).ok_or_else(|| {
                    format!("Installed {} but could not find executable", emulator.name)
                })
            } else if let Some(info) = appimage_install_for_emulator(emulator) {
                tracing::info!(emulator = %emulator.name, "Installing via AppImage backend");
                install_appimage_emulator(info).await?;
                check_linux_installation(emulator).ok_or_else(|| {
                    format!("Installed {} but could not find executable", emulator.name)
                })
            } else if let Some(info) =
                wine_install_for_emulator(emulator).filter(|_| is_wine_available())
            {
                tracing::info!(emulator = %emulator.name, "Installing via Wine backend");
                install_wine_emulator(info).await?;
                check_linux_installation(emulator).ok_or_else(|| {
                    format!("Installed {} but could not find executable", emulator.name)
                })
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

pub async fn uninstall_emulator(
    emulator: &EmulatorInfo,
    as_retroarch_core: bool,
) -> Result<(), String> {
    tracing::info!(
        emulator = %emulator.name,
        as_retroarch_core = as_retroarch_core,
        os = current_os(),
        "Uninstalling emulator"
    );

    if as_retroarch_core {
        return Err(format!(
            "RetroArch core uninstall is not supported yet for {} because core ownership is not tracked",
            emulator.name
        ));
    }

    match current_os() {
        "Linux" => {
            let installed_path = check_standalone_installation(emulator)
                .ok_or_else(|| format!("{} is not installed", emulator.name))?;

            match get_uninstall_method_for_path(&installed_path).as_deref() {
                Some("flatpak") => {
                    let app_id = emulator.flatpak_id.as_deref().ok_or_else(|| {
                        format!("{} does not define a Flatpak app id", emulator.name)
                    })?;
                    uninstall_flatpak(app_id).await
                }
                Some("nix") => {
                    let package = nix_package_for_emulator(emulator).ok_or_else(|| {
                        format!("{} does not define a Nix package mapping", emulator.name)
                    })?;
                    uninstall_nix_package(package).await
                }
                Some("appimage") => {
                    let info = appimage_install_for_emulator(emulator).ok_or_else(|| {
                        format!(
                            "{} does not define an AppImage install mapping",
                            emulator.name
                        )
                    })?;
                    uninstall_appimage_emulator(info).await
                }
                Some("wine") => {
                    let info = wine_install_for_emulator(emulator).ok_or_else(|| {
                        format!("{} does not define a Wine install mapping", emulator.name)
                    })?;
                    uninstall_wine_emulator(info).await
                }
                _ => Err(format!(
                    "{} is installed outside Lunchbox-managed Linux backends",
                    emulator.name
                )),
            }
        }
        _ => Err("Emulator uninstall is currently implemented for Linux backends only".to_string()),
    }
}

pub async fn get_available_updates(
    emulators: &[EmulatorInfo],
) -> Result<Vec<EmulatorUpdate>, String> {
    let mut updates = Vec::new();

    #[cfg(target_os = "linux")]
    {
        match get_flatpak_updates(emulators).await {
            Ok(flatpak_updates) => updates.extend(flatpak_updates),
            Err(err) => tracing::warn!(error = %err, "Flatpak emulator update check failed"),
        }
        match get_nix_updates(emulators).await {
            Ok(nix_updates) => updates.extend(nix_updates),
            Err(err) => tracing::warn!(error = %err, "Nix emulator update check failed"),
        }
        match get_appimage_updates(emulators).await {
            Ok(appimage_updates) => updates.extend(appimage_updates),
            Err(err) => tracing::warn!(error = %err, "AppImage emulator update check failed"),
        }
        updates.sort_by(|a, b| {
            a.display_name
                .cmp(&b.display_name)
                .then(a.source_label.cmp(&b.source_label))
        });
    }

    #[cfg(not(target_os = "linux"))]
    {
        let _ = emulators;
    }

    Ok(updates)
}

pub async fn apply_update(update_key: &str) -> Result<(), String> {
    #[cfg(target_os = "linux")]
    {
        if let Some(rest) = update_key.strip_prefix("flatpak:") {
            let (scope, app_id) = parse_flatpak_update_key(rest)?;
            return update_flatpak(app_id, scope).await;
        }

        if let Some(profile_name) = update_key.strip_prefix("nix:") {
            return update_nix_package(profile_name).await;
        }

        if let Some(slug) = update_key.strip_prefix("appimage:") {
            return update_appimage_emulator(slug).await;
        }

        Err(format!("Unsupported emulator update key '{}'", update_key))
    }
    #[cfg(not(target_os = "linux"))]
    {
        let _ = update_key;
        Err("Emulator updates are currently implemented for Linux backends only".to_string())
    }
}

#[cfg(target_os = "linux")]
async fn get_flatpak_updates(emulators: &[EmulatorInfo]) -> Result<Vec<EmulatorUpdate>, String> {
    if !is_flatpak_available() {
        return Ok(Vec::new());
    }

    let mut targets_by_app_id: BTreeMap<String, ManagedUpdateTarget> = BTreeMap::new();
    for emulator in emulators {
        if emulator.flatpak_id.as_deref() == Some("org.libretro.RetroArch")
            && emulator.name != "RetroArch"
        {
            continue;
        }
        let Some(app_id) = emulator.flatpak_id.clone() else {
            continue;
        };
        targets_by_app_id
            .entry(app_id)
            .or_default()
            .display_names
            .insert(emulator.name.clone());
    }

    let installed = flatpak_installed_versions().await?;
    let available = flatpak_available_updates().await?;
    let mut updates = Vec::new();

    for ((scope, app_id), current_version) in installed {
        let Some(target) = targets_by_app_id.get(&app_id) else {
            continue;
        };
        let Some(available_version) = available.get(&(scope, app_id.clone())).cloned() else {
            continue;
        };

        updates.push(EmulatorUpdate {
            key: format!("flatpak:{}:{}", scope.key_prefix(), app_id),
            display_name: summarize_update_display_names(&target.display_names),
            install_method: "flatpak".to_string(),
            source_label: scope.label().to_string(),
            current_version,
            available_version,
        });
    }

    Ok(updates)
}

#[cfg(target_os = "linux")]
async fn get_nix_updates(emulators: &[EmulatorInfo]) -> Result<Vec<EmulatorUpdate>, String> {
    if !is_nix_available() {
        return Ok(Vec::new());
    }

    let profile_entries = nix_profile_list().await?;
    if profile_entries.elements.is_empty() {
        return Ok(Vec::new());
    }

    let mut targets_by_package: BTreeMap<String, ManagedUpdateTarget> = BTreeMap::new();
    for emulator in emulators {
        let Some(package) = nix_package_for_emulator(emulator) else {
            continue;
        };
        targets_by_package
            .entry(package.to_string())
            .or_default()
            .display_names
            .insert(emulator.name.clone());
    }

    let mut updates = Vec::new();
    for (package, target) in targets_by_package {
        let Some((profile_name, entry)) = find_nix_profile_entry(&profile_entries, &package) else {
            continue;
        };
        if !entry.active {
            continue;
        }

        let current_version = entry
            .store_paths
            .iter()
            .find_map(|path| parse_nix_store_version(path, &package));
        let available_version = match nix_available_version(&package).await {
            Ok(version) => version,
            Err(err) => {
                tracing::warn!(package = %package, error = %err, "Skipping nix emulator update because version lookup failed");
                continue;
            }
        };
        let Some(available_version) = available_version else {
            continue;
        };

        if current_version.as_deref() == Some(available_version.as_str()) {
            continue;
        }

        updates.push(EmulatorUpdate {
            key: format!("nix:{}", profile_name),
            display_name: summarize_update_display_names(&target.display_names),
            install_method: "nix".to_string(),
            source_label: "Nix profile".to_string(),
            current_version,
            available_version: Some(available_version),
        });
    }

    Ok(updates)
}

#[cfg(target_os = "linux")]
async fn get_appimage_updates(emulators: &[EmulatorInfo]) -> Result<Vec<EmulatorUpdate>, String> {
    let mut targets_by_slug: BTreeMap<&'static str, (AppImageInstallInfo, ManagedUpdateTarget)> =
        BTreeMap::new();
    for emulator in emulators {
        let Some(info) = appimage_install_for_emulator(emulator) else {
            continue;
        };
        targets_by_slug
            .entry(info.slug)
            .and_modify(|(_, target)| {
                target.display_names.insert(emulator.name.clone());
            })
            .or_insert_with(|| {
                let mut target = ManagedUpdateTarget::default();
                target.display_names.insert(emulator.name.clone());
                (info, target)
            });
    }

    let mut updates = Vec::new();
    for (slug, (info, target)) in targets_by_slug {
        let Some(manifest) = read_appimage_manifest(&info) else {
            continue;
        };
        let release = fetch_github_latest_release(info.github_repo).await?;
        let available_version = normalize_release_version(&release.tag_name);
        if manifest.version == available_version {
            continue;
        }

        let source_label = match manifest.update_transport.as_deref() {
            Some("zsync") => "AppImageUpdate (zsync)".to_string(),
            Some(transport) => format!("AppImageUpdate ({transport})"),
            None => "AppImage (GitHub)".to_string(),
        };

        updates.push(EmulatorUpdate {
            key: format!("appimage:{slug}"),
            display_name: summarize_update_display_names(&target.display_names),
            install_method: "appimage".to_string(),
            source_label,
            current_version: Some(manifest.version),
            available_version: Some(available_version),
        });
    }

    Ok(updates)
}

#[cfg(target_os = "linux")]
fn summarize_update_display_names(names: &BTreeSet<String>) -> String {
    if names.is_empty() {
        return "Unknown Emulator".to_string();
    }

    if let Some(primary) = names.iter().find(|name| !name.contains('(')) {
        if names.len() == 1 {
            return primary.clone();
        }
        return format!("{} (+{} variants)", primary, names.len() - 1);
    }

    names.iter().cloned().collect::<Vec<_>>().join(", ")
}

#[cfg(target_os = "linux")]
fn parse_flatpak_update_key(rest: &str) -> Result<(FlatpakScope, &str), String> {
    let (scope_text, app_id) = rest
        .split_once(':')
        .ok_or_else(|| format!("Invalid flatpak update key '{}'", rest))?;
    let scope = match scope_text {
        "user" => FlatpakScope::User,
        "system" => FlatpakScope::System,
        _ => return Err(format!("Unknown flatpak update scope '{}'", scope_text)),
    };
    Ok((scope, app_id))
}

#[cfg(target_os = "linux")]
async fn flatpak_installed_versions(
) -> Result<BTreeMap<(FlatpakScope, String), Option<String>>, String> {
    let mut installed = BTreeMap::new();
    for scope in [FlatpakScope::System, FlatpakScope::User] {
        let output = tokio::process::Command::new("flatpak")
            .args([
                "list",
                scope.flag(),
                "--app",
                "--columns=application,version",
            ])
            .output()
            .await
            .map_err(|e| format!("Failed to run flatpak list: {}", e))?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            let stderr_trimmed = stderr.trim();
            if scope == FlatpakScope::User && stderr_trimmed.contains("No installations") {
                continue;
            }
            return Err(format!("Flatpak list failed: {}", stderr_trimmed));
        }

        for line in String::from_utf8_lossy(&output.stdout).lines() {
            let trimmed = line.trim();
            if trimmed.is_empty() {
                continue;
            }
            let mut fields = trimmed.split('\t');
            let Some(app_id) = fields
                .next()
                .map(str::trim)
                .filter(|value| !value.is_empty())
            else {
                continue;
            };
            let version = fields
                .next()
                .map(str::trim)
                .filter(|value| !value.is_empty())
                .map(ToOwned::to_owned);
            installed.insert((scope, app_id.to_string()), version);
        }
    }
    Ok(installed)
}

#[cfg(target_os = "linux")]
async fn flatpak_available_updates(
) -> Result<BTreeMap<(FlatpakScope, String), Option<String>>, String> {
    let mut updates = BTreeMap::new();
    for scope in [FlatpakScope::System, FlatpakScope::User] {
        let output = tokio::process::Command::new("flatpak")
            .args([
                "remote-ls",
                scope.flag(),
                "--updates",
                "--app",
                "--columns=application,version",
            ])
            .output()
            .await
            .map_err(|e| format!("Failed to run flatpak remote-ls: {}", e))?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            let stderr_trimmed = stderr.trim();
            if scope == FlatpakScope::User && stderr_trimmed.contains("No installations") {
                continue;
            }
            return Err(format!("Flatpak update check failed: {}", stderr_trimmed));
        }

        for line in String::from_utf8_lossy(&output.stdout).lines() {
            let trimmed = line.trim();
            if trimmed.is_empty() {
                continue;
            }
            let mut fields = trimmed.split('\t');
            let Some(app_id) = fields
                .next()
                .map(str::trim)
                .filter(|value| !value.is_empty())
            else {
                continue;
            };
            let version = fields
                .next()
                .map(str::trim)
                .filter(|value| !value.is_empty())
                .map(ToOwned::to_owned);
            updates.insert((scope, app_id.to_string()), version);
        }
    }
    Ok(updates)
}

#[cfg(target_os = "linux")]
async fn nix_profile_list() -> Result<NixProfileList, String> {
    let profile_path = lunchbox_nix_profile_path();
    let output = tokio::process::Command::new("nix")
        .args([
            "profile",
            "list",
            "--profile",
            profile_path.to_string_lossy().as_ref(),
            "--json",
            "--no-pretty",
        ])
        .output()
        .await
        .map_err(|e| format!("Failed to run nix profile list: {}", e))?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        let stderr_trimmed = stderr.trim();
        if stderr_trimmed.contains("does not exist") || stderr_trimmed.contains("No such file") {
            return Ok(NixProfileList {
                elements: BTreeMap::new(),
            });
        }
        return Err(format!("Nix profile list failed: {}", stderr_trimmed));
    }

    serde_json::from_slice(&output.stdout)
        .map_err(|e| format!("Failed to parse nix profile list output: {}", e))
}

#[cfg(target_os = "linux")]
fn find_nix_profile_entry<'a>(
    profile: &'a NixProfileList,
    package: &str,
) -> Option<(String, &'a NixProfileListElement)> {
    profile.elements.iter().find_map(|(name, entry)| {
        let attr_matches = entry
            .attr_path
            .as_deref()
            .map(|attr| attr.ends_with(&format!(".{}", package)) || attr == package)
            .unwrap_or(false);
        if name == package || attr_matches {
            Some((name.clone(), entry))
        } else {
            None
        }
    })
}

#[cfg(target_os = "linux")]
fn parse_nix_store_version(store_path: &str, package: &str) -> Option<String> {
    let file_name = Path::new(store_path).file_name()?.to_str()?;
    let after_hash = file_name.split_once('-')?.1;
    if let Some(version) = after_hash.strip_prefix(&format!("{}-", package)) {
        return Some(version.to_string());
    }
    after_hash
        .rsplit_once('-')
        .map(|(_, version)| version.to_string())
}

#[cfg(target_os = "linux")]
async fn nix_available_version(package: &str) -> Result<Option<String>, String> {
    let attr = format!("nixpkgs#{}.version", package);
    let output = tokio::process::Command::new("nix")
        .args(["eval", "--raw", &attr])
        .output()
        .await
        .map_err(|e| format!("Failed to run nix eval for {}: {}", package, e))?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        let stderr_trimmed = stderr.trim();
        if stderr_trimmed.is_empty() {
            return Ok(None);
        }
        return Err(format!(
            "Nix update check failed for {}: {}",
            package, stderr_trimmed
        ));
    }

    let version = String::from_utf8_lossy(&output.stdout).trim().to_string();
    if version.is_empty() {
        Ok(None)
    } else {
        Ok(Some(version))
    }
}

#[cfg(target_os = "linux")]
async fn download_appimage_release(
    info: &AppImageInstallInfo,
) -> Result<(String, GitHubReleaseAsset), String> {
    let release = fetch_github_latest_release(info.github_repo).await?;
    let version = normalize_release_version(&release.tag_name);
    let asset = select_github_appimage_asset(&release, info)
        .cloned()
        .ok_or_else(|| {
            format!(
                "No AppImage asset was found in the latest release for {}",
                info.github_repo
            )
        })?;
    Ok((version, asset))
}

#[cfg(target_os = "linux")]
async fn download_github_asset_to_path(url: &str, destination: &Path) -> Result<(), String> {
    let bytes = reqwest::Client::new()
        .get(url)
        .header(reqwest::header::USER_AGENT, "lunchbox-appimage")
        .send()
        .await
        .map_err(|e| format!("Failed to download AppImage asset: {}", e))?
        .error_for_status()
        .map_err(|e| format!("AppImage asset download failed: {}", e))?
        .bytes()
        .await
        .map_err(|e| format!("Failed to read AppImage asset download: {}", e))?;

    let temp_path = destination.with_extension("part");
    tokio::fs::write(&temp_path, &bytes)
        .await
        .map_err(|e| format!("Failed to write AppImage asset download: {}", e))?;
    tokio::fs::rename(&temp_path, destination)
        .await
        .map_err(|e| format!("Failed to finalize AppImage asset download: {}", e))
}

#[cfg(target_os = "linux")]
async fn ensure_appimage_updater_tool() -> Result<PathBuf, String> {
    let info = appimage_updater_info();
    let release = fetch_github_latest_release(info.github_repo).await?;
    let version = normalize_release_version(&release.tag_name);
    let asset = select_github_updater_asset(&release, &info)
        .cloned()
        .ok_or_else(|| "No x86_64 appimageupdatetool asset was found".to_string())?;

    let version_dir = appimage_updater_version_dir(&info, &version);
    tokio::fs::create_dir_all(&version_dir)
        .await
        .map_err(|e| format!("Failed to create AppImage updater directory: {}", e))?;

    let asset_path = version_dir.join(&asset.name);
    if !asset_path.exists() {
        download_github_asset_to_path(&asset.browser_download_url, &asset_path).await?;
    }

    set_executable_permissions(&asset_path)?;
    replace_symlink(&version_dir, &appimage_updater_current_link(&info))?;
    Ok(asset_path)
}

#[cfg(target_os = "linux")]
async fn read_appimage_update_information(path: &Path) -> Result<Option<String>, String> {
    for flag in ["--appimage-updateinformation", "--appimage-updateinfo"] {
        let output = tokio::process::Command::new(path)
            .arg(flag)
            .env("APPIMAGE_EXTRACT_AND_RUN", "1")
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .output()
            .await
            .map_err(|e| format!("Failed to inspect AppImage update information: {}", e))?;

        if output.status.success() {
            let update_info = String::from_utf8_lossy(&output.stdout).trim().to_string();
            if !update_info.is_empty() {
                return Ok(Some(update_info));
            }
        }

        let stderr = String::from_utf8_lossy(&output.stderr).to_ascii_lowercase();
        if stderr.contains("unrecognized option")
            || stderr.contains("unknown option")
            || stderr.contains("unknown argument")
        {
            continue;
        }
    }

    Ok(None)
}

#[cfg(target_os = "linux")]
fn normalize_appimage_update_transport(update_info: &str) -> Option<String> {
    let transport = update_info
        .split('|')
        .next()
        .map(str::trim)
        .filter(|value| !value.is_empty())?
        .to_ascii_lowercase();

    if transport.contains("zsync") {
        Some("zsync".to_string())
    } else {
        Some(transport)
    }
}

#[cfg(target_os = "linux")]
fn select_updated_appimage(work_dir: &Path, staged_input: &Path) -> Result<PathBuf, String> {
    let mut appimages = std::fs::read_dir(work_dir)
        .map_err(|e| format!("Failed to inspect AppImage update output: {}", e))?
        .flatten()
        .map(|entry| entry.path())
        .filter(|path| {
            path.extension()
                .and_then(|ext| ext.to_str())
                .map(|ext| ext.eq_ignore_ascii_case("appimage"))
                .unwrap_or(false)
        })
        .collect::<Vec<_>>();

    appimages.sort();

    if let Some(updated) = appimages.iter().find(|path| *path != staged_input) {
        return Ok(updated.clone());
    }

    if staged_input.exists() {
        return Ok(staged_input.to_path_buf());
    }

    Err("AppImageUpdate completed but no updated AppImage was produced".to_string())
}

#[cfg(target_os = "linux")]
fn select_appimage_payload_entry(entries: &[ArchiveEntry]) -> Option<PathBuf> {
    entries
        .iter()
        .filter(|entry| !entry.is_dir)
        .filter_map(|entry| {
            let name = entry
                .path
                .file_name()
                .and_then(|value| value.to_str())
                .map(|value| value.to_ascii_lowercase())?;
            if !name.ends_with(".appimage") {
                return None;
            }

            let mut score = 100i32;
            if name.contains("steamdeck") || name.contains("x86_64") || name.contains("amd64") {
                score += 50;
            }
            Some((score, entry.path.clone()))
        })
        .max_by(|(score_a, path_a), (score_b, path_b)| {
            score_a.cmp(score_b).then_with(|| path_a.cmp(path_b))
        })
        .map(|(_, path)| path)
}

#[cfg(target_os = "linux")]
fn extract_appimage_payload_from_archive(
    archive_path: &Path,
    output_dir: &Path,
) -> Result<PathBuf, String> {
    let kind = archive_kind_for_path(archive_path).ok_or_else(|| {
        format!(
            "Asset {} is not a supported AppImage archive",
            archive_path.display()
        )
    })?;
    let entries = list_archive_entries(archive_path, kind)?;
    let appimage_entry = select_appimage_payload_entry(&entries).ok_or_else(|| {
        format!(
            "Archive {} does not contain an AppImage payload",
            archive_path.display()
        )
    })?;
    let extracted = extract_archive_entries(archive_path, kind, output_dir, &[appimage_entry])?;
    extracted.into_iter().next().ok_or_else(|| {
        format!(
            "Archive {} did not produce an extracted AppImage",
            archive_path.display()
        )
    })
}

#[cfg(target_os = "linux")]
async fn install_appimage_release_asset(
    info: &AppImageInstallInfo,
    version: &str,
    asset: &GitHubReleaseAsset,
) -> Result<(), String> {
    let root = appimage_install_root(info);
    tokio::fs::create_dir_all(&root)
        .await
        .map_err(|e| format!("Failed to create AppImage install directory: {}", e))?;

    let version_dir = appimage_version_dir(info, version);
    tokio::fs::create_dir_all(&version_dir)
        .await
        .map_err(|e| format!("Failed to create AppImage version directory: {}", e))?;

    let asset_path = version_dir.join(&asset.name);
    if !asset_path.exists() {
        download_github_asset_to_path(&asset.browser_download_url, &asset_path).await?;
    }

    let installed_path = if asset.name.to_ascii_lowercase().ends_with(".appimage") {
        asset_path.clone()
    } else {
        let extracted_path = extract_appimage_payload_from_archive(&asset_path, &version_dir)?;
        let _ = tokio::fs::remove_file(&asset_path).await;
        extracted_path
    };

    set_executable_permissions(&installed_path)?;
    let update_transport = read_appimage_update_information(&installed_path)
        .await?
        .as_deref()
        .and_then(normalize_appimage_update_transport);

    replace_symlink(&version_dir, &appimage_current_link(info))?;

    let manifest = AppImageInstallManifest {
        slug: info.slug.to_string(),
        version: version.to_string(),
        github_repo: info.github_repo.to_string(),
        asset_name: installed_path
            .file_name()
            .and_then(|name| name.to_str())
            .unwrap_or(&asset.name)
            .to_string(),
        download_url: asset.browser_download_url.clone(),
        installed_at: chrono::Utc::now().to_rfc3339(),
        update_transport,
    };
    write_appimage_manifest(info, &manifest)?;
    Ok(())
}

#[cfg(target_os = "linux")]
async fn try_delta_update_appimage(
    info: &AppImageInstallInfo,
    version: &str,
    asset: &GitHubReleaseAsset,
) -> Result<(), String> {
    let installed_path = find_appimage_install_executable(info)
        .ok_or_else(|| format!("No installed AppImage was found for {}", info.slug))?;
    let update_info = read_appimage_update_information(&installed_path)
        .await?
        .ok_or_else(|| "Installed AppImage does not advertise update information".to_string())?;
    let update_transport = normalize_appimage_update_transport(&update_info)
        .ok_or_else(|| "Installed AppImage uses an unknown update transport".to_string())?;

    let updater_path = ensure_appimage_updater_tool().await?;
    let scratch_root = appimage_install_root(info).join(".update-work");
    tokio::fs::create_dir_all(&scratch_root)
        .await
        .map_err(|e| format!("Failed to create AppImage update work directory: {}", e))?;
    let work_dir = scratch_root.join(format!(
        "delta-{}-{}",
        std::process::id(),
        chrono::Utc::now().timestamp_millis()
    ));
    tokio::fs::create_dir_all(&work_dir)
        .await
        .map_err(|e| format!("Failed to create AppImage update workspace: {}", e))?;

    let staged_name = installed_path
        .file_name()
        .ok_or_else(|| "Installed AppImage path has no filename".to_string())?;
    let staged_input = work_dir.join(staged_name);
    tokio::fs::copy(installed_path.as_path(), staged_input.as_path())
        .await
        .map_err(|e| format!("Failed to stage AppImage for delta update: {}", e))?;
    set_executable_permissions(&staged_input)?;

    let output = tokio::process::Command::new(&updater_path)
        .arg(&staged_input)
        .env("APPIMAGE_EXTRACT_AND_RUN", "1")
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .output()
        .await
        .map_err(|e| format!("Failed to run appimageupdatetool: {}", e))?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr).trim().to_string();
        let stdout = String::from_utf8_lossy(&output.stdout).trim().to_string();
        let detail = if !stderr.is_empty() { stderr } else { stdout };
        return Err(format!("appimageupdatetool failed: {}", detail));
    }

    let updated_path = select_updated_appimage(&work_dir, &staged_input)?;
    set_executable_permissions(&updated_path)?;

    let version_dir = appimage_version_dir(info, version);
    tokio::fs::create_dir_all(&version_dir)
        .await
        .map_err(|e| format!("Failed to create AppImage version directory: {}", e))?;

    let final_path = version_dir.join(
        updated_path
            .file_name()
            .ok_or_else(|| "Updated AppImage path has no filename".to_string())?,
    );
    if final_path.exists() {
        tokio::fs::remove_file(&final_path)
            .await
            .map_err(|e| format!("Failed to replace updated AppImage: {}", e))?;
    }
    tokio::fs::rename(&updated_path, &final_path)
        .await
        .map_err(|e| format!("Failed to finalize delta-updated AppImage: {}", e))?;

    replace_symlink(&version_dir, &appimage_current_link(info))?;

    let manifest = AppImageInstallManifest {
        slug: info.slug.to_string(),
        version: version.to_string(),
        github_repo: info.github_repo.to_string(),
        asset_name: final_path
            .file_name()
            .and_then(|name| name.to_str())
            .unwrap_or(&asset.name)
            .to_string(),
        download_url: asset.browser_download_url.clone(),
        installed_at: chrono::Utc::now().to_rfc3339(),
        update_transport: Some(update_transport),
    };
    write_appimage_manifest(info, &manifest)?;
    Ok(())
}

#[cfg(target_os = "linux")]
fn set_executable_permissions(path: &Path) -> Result<(), String> {
    std::fs::set_permissions(path, Permissions::from_mode(0o755))
        .map_err(|e| format!("Failed to mark {} executable: {}", path.display(), e))
}

#[cfg(target_os = "linux")]
fn replace_symlink(target: &Path, link_path: &Path) -> Result<(), String> {
    if let Some(parent) = link_path.parent() {
        std::fs::create_dir_all(parent)
            .map_err(|e| format!("Failed to create {}: {}", parent.display(), e))?;
    }

    match std::fs::symlink_metadata(link_path) {
        Ok(metadata) => {
            if metadata.file_type().is_dir() && !metadata.file_type().is_symlink() {
                std::fs::remove_dir_all(link_path)
                    .map_err(|e| format!("Failed to replace {}: {}", link_path.display(), e))?;
            } else {
                std::fs::remove_file(link_path)
                    .map_err(|e| format!("Failed to replace {}: {}", link_path.display(), e))?;
            }
        }
        Err(err) if err.kind() == std::io::ErrorKind::NotFound => {}
        Err(err) => {
            return Err(format!(
                "Failed to inspect {} before replacing symlink: {}",
                link_path.display(),
                err
            ));
        }
    }

    std::os::unix::fs::symlink(target, link_path)
        .map_err(|e| format!("Failed to create {}: {}", link_path.display(), e))
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

async fn uninstall_flatpak(app_id: &str) -> Result<(), String> {
    #[cfg(target_os = "linux")]
    {
        let output = tokio::process::Command::new("flatpak")
            .args(["uninstall", "-y", app_id])
            .output()
            .await
            .map_err(|e| format!("Failed to run flatpak: {}", e))?;

        if output.status.success() {
            Ok(())
        } else {
            let stderr = String::from_utf8_lossy(&output.stderr);
            Err(format!("Flatpak uninstall failed: {}", stderr))
        }
    }
    #[cfg(not(target_os = "linux"))]
    {
        let _ = app_id;
        Err("Flatpak is only available on Linux".to_string())
    }
}

async fn update_flatpak(app_id: &str, scope: FlatpakScope) -> Result<(), String> {
    #[cfg(target_os = "linux")]
    {
        let output = tokio::process::Command::new("flatpak")
            .args(["update", "-y", scope.flag(), app_id])
            .output()
            .await
            .map_err(|e| format!("Failed to run flatpak update: {}", e))?;

        if output.status.success() {
            Ok(())
        } else {
            let stderr = String::from_utf8_lossy(&output.stderr);
            Err(format!("Flatpak update failed: {}", stderr.trim()))
        }
    }
    #[cfg(not(target_os = "linux"))]
    {
        let _ = (app_id, scope);
        Err("Flatpak is only available on Linux".to_string())
    }
}

async fn install_nix_package(package: &str) -> Result<(), String> {
    #[cfg(target_os = "linux")]
    {
        let profile_path = lunchbox_nix_profile_path();
        if let Some(parent) = profile_path.parent() {
            tokio::fs::create_dir_all(parent)
                .await
                .map_err(|e| format!("Failed to create Lunchbox nix profile directory: {}", e))?;
        }

        let package_ref = format!("nixpkgs#{}", package);
        let output = tokio::process::Command::new("nix")
            .args([
                "profile",
                "install",
                "--profile",
                profile_path.to_string_lossy().as_ref(),
                &package_ref,
            ])
            .output()
            .await
            .map_err(|e| format!("Failed to run nix profile install: {}", e))?;

        if output.status.success() {
            Ok(())
        } else {
            let stderr = String::from_utf8_lossy(&output.stderr);
            Err(format!("Nix profile install failed: {}", stderr))
        }
    }
    #[cfg(not(target_os = "linux"))]
    {
        let _ = package;
        Err("Nix profile installs are only available on Linux".to_string())
    }
}

async fn install_appimage_emulator(info: AppImageInstallInfo) -> Result<(), String> {
    #[cfg(target_os = "linux")]
    {
        let (version, asset) = download_appimage_release(&info).await?;
        install_appimage_release_asset(&info, &version, &asset).await
    }
    #[cfg(not(target_os = "linux"))]
    {
        let _ = info;
        Err("AppImage installs are only available on Linux".to_string())
    }
}

async fn uninstall_nix_package(package: &str) -> Result<(), String> {
    #[cfg(target_os = "linux")]
    {
        let profile_path = lunchbox_nix_profile_path();
        let package_ref = format!("nixpkgs#{}", package);
        let output = tokio::process::Command::new("nix")
            .args([
                "profile",
                "remove",
                "--profile",
                profile_path.to_string_lossy().as_ref(),
                &package_ref,
            ])
            .output()
            .await
            .map_err(|e| format!("Failed to run nix profile remove: {}", e))?;

        if output.status.success() {
            Ok(())
        } else {
            let stderr = String::from_utf8_lossy(&output.stderr);
            Err(format!("Nix profile remove failed: {}", stderr))
        }
    }
    #[cfg(not(target_os = "linux"))]
    {
        let _ = package;
        Err("Nix profile uninstalls are only available on Linux".to_string())
    }
}

async fn uninstall_appimage_emulator(info: AppImageInstallInfo) -> Result<(), String> {
    #[cfg(target_os = "linux")]
    {
        remove_path_if_exists(&appimage_install_root(&info)).await
    }
    #[cfg(not(target_os = "linux"))]
    {
        let _ = info;
        Err("AppImage uninstall is only available on Linux".to_string())
    }
}

async fn update_nix_package(profile_name: &str) -> Result<(), String> {
    #[cfg(target_os = "linux")]
    {
        let profile_path = lunchbox_nix_profile_path();
        let output = tokio::process::Command::new("nix")
            .args([
                "profile",
                "upgrade",
                "--profile",
                profile_path.to_string_lossy().as_ref(),
                profile_name,
            ])
            .output()
            .await
            .map_err(|e| format!("Failed to run nix profile upgrade: {}", e))?;

        if output.status.success() {
            Ok(())
        } else {
            let stderr = String::from_utf8_lossy(&output.stderr);
            Err(format!("Nix profile upgrade failed: {}", stderr.trim()))
        }
    }
    #[cfg(not(target_os = "linux"))]
    {
        let _ = profile_name;
        Err("Nix profile updates are only available on Linux".to_string())
    }
}

async fn update_appimage_emulator(slug: &str) -> Result<(), String> {
    #[cfg(target_os = "linux")]
    {
        let info = appimage_install_for_slug(slug)
            .ok_or_else(|| format!("Unsupported AppImage update slug '{}'", slug))?;
        let release = fetch_github_latest_release(info.github_repo).await?;
        let version = normalize_release_version(&release.tag_name);
        let asset = select_github_appimage_asset(&release, &info)
            .cloned()
            .ok_or_else(|| {
                format!(
                    "No AppImage asset was found in the latest release for {}",
                    info.github_repo
                )
            })?;

        if find_appimage_install_executable(&info).is_some()
            && read_appimage_update_information(
                &find_appimage_install_executable(&info)
                    .ok_or_else(|| format!("No installed AppImage was found for {}", info.slug))?,
            )
            .await?
            .is_some()
        {
            if try_delta_update_appimage(&info, &version, &asset)
                .await
                .is_ok()
            {
                return Ok(());
            }
        }

        install_appimage_release_asset(&info, &version, &asset).await
    }
    #[cfg(not(target_os = "linux"))]
    {
        let _ = slug;
        Err("AppImage updates are only available on Linux".to_string())
    }
}

async fn install_wine_emulator(info: WineInstallInfo) -> Result<(), String> {
    #[cfg(target_os = "linux")]
    {
        let install_root = wine_install_root(&info);
        let app_dir = wine_app_dir(&info);
        tokio::fs::create_dir_all(&install_root)
            .await
            .map_err(|e| format!("Failed to create Wine install directory: {}", e))?;

        let download_url = resolve_altirra_download_url(info.download_page_url).await?;
        let archive_path = install_root.join("download.zip");
        let response = reqwest::Client::new()
            .get(&download_url)
            .send()
            .await
            .map_err(|e| format!("Failed to download Altirra: {}", e))?;

        if !response.status().is_success() {
            return Err(format!(
                "Failed to download Altirra: HTTP {}",
                response.status()
            ));
        }

        let bytes = response
            .bytes()
            .await
            .map_err(|e| format!("Failed to read Altirra download: {}", e))?;
        tokio::fs::write(&archive_path, &bytes)
            .await
            .map_err(|e| format!("Failed to write Altirra archive: {}", e))?;

        if app_dir.exists() {
            tokio::fs::remove_dir_all(&app_dir)
                .await
                .map_err(|e| format!("Failed to replace existing Altirra install: {}", e))?;
        }
        tokio::fs::create_dir_all(&app_dir)
            .await
            .map_err(|e| format!("Failed to create Altirra app directory: {}", e))?;

        let archive_path_clone = archive_path.clone();
        let app_dir_clone = app_dir.clone();
        tokio::task::spawn_blocking(move || {
            let file = std::fs::File::open(&archive_path_clone)
                .map_err(|e| format!("Failed to open Altirra archive: {}", e))?;
            let mut archive = zip::ZipArchive::new(file)
                .map_err(|e| format!("Failed to read Altirra archive: {}", e))?;
            archive
                .extract(&app_dir_clone)
                .map_err(|e| format!("Failed to extract Altirra archive: {}", e))?;
            Ok::<_, String>(())
        })
        .await
        .map_err(|e| format!("Altirra extraction task failed: {}", e))??;

        let _ = tokio::fs::remove_file(&archive_path).await;

        if find_wine_install_executable(&info).is_some() {
            Ok(())
        } else {
            Err("Altirra was downloaded, but Lunchbox could not find Altirra64.exe or Altirra.exe after extraction".to_string())
        }
    }
    #[cfg(not(target_os = "linux"))]
    {
        let _ = info;
        Err("Wine installs are only available on Linux".to_string())
    }
}

async fn uninstall_wine_emulator(info: WineInstallInfo) -> Result<(), String> {
    #[cfg(target_os = "linux")]
    {
        let install_root = wine_install_root(&info);
        remove_path_if_exists(&install_root).await
    }
    #[cfg(not(target_os = "linux"))]
    {
        let _ = info;
        Err("Wine emulator uninstall is only available on Linux".to_string())
    }
}

async fn remove_path_if_exists(path: &Path) -> Result<(), String> {
    match tokio::fs::symlink_metadata(path).await {
        Ok(metadata) => {
            let file_type = metadata.file_type();
            if file_type.is_dir() && !file_type.is_symlink() {
                tokio::fs::remove_dir_all(path)
                    .await
                    .map_err(|e| format!("Failed to remove {}: {}", path.display(), e))
            } else {
                tokio::fs::remove_file(path)
                    .await
                    .map_err(|e| format!("Failed to remove {}: {}", path.display(), e))
            }
        }
        Err(err) if err.kind() == std::io::ErrorKind::NotFound => Ok(()),
        Err(err) => Err(format!("Failed to inspect {}: {}", path.display(), err)),
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
        "Linux" => {
            if is_flatpak_available() {
                install_flatpak("org.libretro.RetroArch").await
            } else if is_nix_available() {
                install_nix_package("retroarch").await
            } else {
                Err("No Linux installation method available for RetroArch".to_string())
            }
        }
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

fn prepare_rom_for_launch_for_runtime(
    emulator: &EmulatorInfo,
    rom_path: Option<&str>,
    platform_name: Option<&str>,
    as_retroarch_core: bool,
) -> Result<PreparedRomLaunch, String> {
    if let Some(rom_path) = rom_path {
        if should_preserve_arcade_mame_romset_archive(
            emulator,
            platform_name,
            as_retroarch_core,
            rom_path,
        ) {
            return Ok(PreparedRomLaunch {
                rom_path: Some(rom_path.to_string()),
                cleanup_paths: Vec::new(),
            });
        }
    }

    prepare_rom_for_launch(rom_path)
}

fn should_preserve_arcade_mame_romset_archive(
    emulator: &EmulatorInfo,
    platform_name: Option<&str>,
    as_retroarch_core: bool,
    rom_path: &str,
) -> bool {
    if platform_name != Some("Arcade") {
        return false;
    }

    let is_mame = if as_retroarch_core {
        emulator.retroarch_core.as_deref() == Some("mame")
    } else {
        emulator.name == "MAME"
    };
    if !is_mame {
        return false;
    }

    matches!(
        Path::new(rom_path)
            .extension()
            .and_then(|ext| ext.to_str())
            .map(|ext| ext.to_ascii_lowercase())
            .as_deref(),
        Some("zip" | "7z")
    )
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
            list_tar_entries(archive_path, kind)
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

fn tar_list_flag(kind: ArchiveKind) -> &'static str {
    match kind {
        ArchiveKind::Tar => "-tf",
        ArchiveKind::TarGz => "-tzf",
        ArchiveKind::TarBz2 => "-tjf",
        ArchiveKind::TarXz => "-tJf",
        _ => unreachable!(),
    }
}

fn tar_extract_flag(kind: ArchiveKind) -> &'static str {
    match kind {
        ArchiveKind::Tar => "-xf",
        ArchiveKind::TarGz => "-xzf",
        ArchiveKind::TarBz2 => "-xjf",
        ArchiveKind::TarXz => "-xJf",
        _ => unreachable!(),
    }
}

fn list_tar_entries(archive_path: &Path, kind: ArchiveKind) -> Result<Vec<ArchiveEntry>, String> {
    let output = Command::new("tar")
        .arg(tar_list_flag(kind))
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
            extract_tar_entries(archive_path, kind, parent_dir, entry_paths)?
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
    kind: ArchiveKind,
    parent_dir: &Path,
    entry_paths: &[PathBuf],
) -> Result<(), String> {
    let mut cmd = Command::new("tar");
    cmd.arg(tar_extract_flag(kind))
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
    platform_name: Option<&str>,
    as_retroarch_core: bool,
    launch_args: &[LaunchArg],
) -> Result<u32, String> {
    tracing::info!(
        emulator = %emulator.name,
        rom = ?rom_path,
        as_retroarch_core = as_retroarch_core,
        "Launching emulator"
    );

    let prepared_rom =
        prepare_rom_for_launch_for_runtime(emulator, rom_path, platform_name, as_retroarch_core)?;

    let result = if as_retroarch_core {
        if let Some(ref core_name) = emulator.retroarch_core {
            launch_retroarch(
                core_name,
                prepared_rom.rom_path.as_deref(),
                platform_name,
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
            platform_name,
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
    launch_emulator(emulator, Some(rom_path), None, as_retroarch_core, &[])
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
    _platform_name: Option<&str>,
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
    platform_name: Option<&str>,
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

    if exe_path
        .to_string_lossy()
        .starts_with(FLATPAK_INSTALL_PREFIX)
    {
        // Flatpak app
        let app_id = exe_path
            .to_string_lossy()
            .replace(FLATPAK_INSTALL_PREFIX, "");
        let mut cmd = Command::new("flatpak");
        cmd.arg("run");
        let extra_runtime_roms_dir =
            if is_arcade_mame_standalone_launch(&emulator.name, platform_name, rom_path) {
                mame_runtime_roms_dir(true)
            } else {
                None
            };
        let mut filesystem_paths = Vec::new();
        if let Some(rom) = rom_path {
            filesystem_paths.push((Path::new(rom), false));
        }
        if let Some(runtime_roms_dir) = extra_runtime_roms_dir.as_ref() {
            filesystem_paths.push((runtime_roms_dir.as_path(), true));
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
        append_standalone_rom_and_args_for_flatpak(
            &mut cmd,
            &emulator.name,
            platform_name,
            rom_path,
            launch_args,
        );
        tracing::info!(command = ?cmd, app_id = %app_id, "Spawning via flatpak");
        spawn_and_verify(cmd, &emulator.name, cleanup_paths)
    } else if let Some(wine_executable) = exe_path
        .to_string_lossy()
        .strip_prefix(WINE_INSTALL_PREFIX)
        .map(|path| path.to_string())
    {
        let info = wine_install_for_emulator(emulator)
            .ok_or_else(|| format!("{} does not have a Wine launch profile", emulator.name))?;
        let prefix_dir = wine_prefix_dir(&info);
        let prefix_exists = prefix_dir.exists();
        if let Some(parent) = prefix_dir.parent() {
            std::fs::create_dir_all(parent)
                .map_err(|e| format!("Failed to create Wine prefix parent directory: {}", e))?;
        }

        let wine_command = find_wine_command()
            .ok_or_else(|| "Wine is not installed or not available on PATH".to_string())?;
        let mut cmd = Command::new(wine_command);
        cmd.env("WINEPREFIX", &prefix_dir);
        if !prefix_exists {
            cmd.env("WINEARCH", "win64");
        }
        cmd.arg(&wine_executable);
        append_standalone_rom_and_args_for_wine(
            &mut cmd,
            &emulator.name,
            &prefix_dir,
            rom_path,
            launch_args,
        )?;
        tracing::info!(
            command = ?cmd,
            prefix = %prefix_dir.display(),
            executable = %wine_executable,
            "Spawning via wine"
        );
        spawn_and_verify(cmd, &emulator.name, cleanup_paths)
    } else if current_os() == "macOS" && exe_path.extension().map(|e| e == "app").unwrap_or(false) {
        // macOS .app bundle
        let mut cmd = Command::new("open");
        cmd.arg("-a").arg(exe_path.to_str().unwrap_or_default());
        append_standalone_rom_and_args_native(
            &mut cmd,
            &emulator.name,
            platform_name,
            rom_path,
            launch_args,
        );
        tracing::info!(command = ?cmd, "Spawning macOS app");
        spawn_and_verify(cmd, &emulator.name, cleanup_paths)
    } else {
        // Regular executable
        let mut cmd = Command::new(&exe_path);
        append_standalone_rom_and_args_native(
            &mut cmd,
            &emulator.name,
            platform_name,
            rom_path,
            launch_args,
        );
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
    let uninstall_method = if is_retroarch_core {
        None
    } else {
        install_path
            .as_deref()
            .and_then(get_uninstall_method_for_path)
    };

    let display_name = if let Some(ref core) = emulator.retroarch_core {
        format!("RetroArch: {}", core)
    } else {
        emulator.name.clone()
    };

    EmulatorWithStatus {
        info: emulator,
        is_installed,
        install_method,
        uninstall_method,
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
    platform_name: Option<&str>,
    rom_path: Option<&str>,
    args: &[LaunchArg],
) {
    if emulator_name == "LoopyMSE" {
        if let Some(rom) = rom_path {
            cmd.arg(rom);
        }
        append_launch_args_native(cmd, args);
    } else if is_arcade_mame_standalone_launch(emulator_name, platform_name, rom_path) {
        append_launch_args_native(cmd, args);
        append_mame_arcade_launch_args_native(cmd, rom_path);
    } else if emulator_name == "Altirra" {
        append_launch_args_native(cmd, args);
        if let Some(rom) = rom_path {
            cmd.arg(altirra_media_switch(rom));
            cmd.arg(rom);
        }
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
    platform_name: Option<&str>,
    rom_path: Option<&str>,
    args: &[LaunchArg],
) {
    if emulator_name == "LoopyMSE" {
        if let Some(rom) = rom_path {
            cmd.arg(map_path_for_flatpak(rom));
        }
        append_launch_args_for_flatpak(cmd, args);
    } else if is_arcade_mame_standalone_launch(emulator_name, platform_name, rom_path) {
        append_launch_args_for_flatpak(cmd, args);
        append_mame_arcade_launch_args_flatpak(cmd, rom_path);
    } else if emulator_name == "Altirra" {
        append_launch_args_for_flatpak(cmd, args);
        if let Some(rom) = rom_path {
            cmd.arg(altirra_media_switch(rom));
            cmd.arg(map_path_for_flatpak(rom));
        }
    } else {
        append_launch_args_for_flatpak(cmd, args);
        if let Some(rom) = rom_path {
            cmd.arg(map_path_for_flatpak(rom));
        }
    }
}

fn is_arcade_mame_standalone_launch(
    emulator_name: &str,
    platform_name: Option<&str>,
    rom_path: Option<&str>,
) -> bool {
    emulator_name == "MAME"
        && platform_name == Some("Arcade")
        && matches!(
            rom_path.and_then(|path| {
                Path::new(path)
                    .extension()
                    .and_then(|ext| ext.to_str())
                    .map(|ext| ext.to_ascii_lowercase())
            }),
            Some(ext) if ext == "zip" || ext == "7z"
        )
}

fn append_mame_arcade_launch_args_native(cmd: &mut Command, rom_path: Option<&str>) {
    let Some(rom_path) = rom_path else {
        return;
    };
    let Some(romset_name) = mame_arcade_romset_name(rom_path) else {
        cmd.arg(rom_path);
        return;
    };

    cmd.arg("-rompath")
        .arg(mame_arcade_rompath_value(rom_path, false))
        .arg(romset_name);
}

fn append_mame_arcade_launch_args_flatpak(cmd: &mut Command, rom_path: Option<&str>) {
    let Some(rom_path) = rom_path else {
        return;
    };
    let Some(romset_name) = mame_arcade_romset_name(rom_path) else {
        cmd.arg(map_path_for_flatpak(rom_path));
        return;
    };

    cmd.arg("-rompath")
        .arg(mame_arcade_rompath_value(rom_path, true))
        .arg(romset_name);
}

fn mame_arcade_romset_name(rom_path: &str) -> Option<String> {
    Path::new(rom_path)
        .file_stem()
        .and_then(|stem| stem.to_str())
        .filter(|stem| !stem.is_empty())
        .map(|stem| stem.to_string())
}

fn mame_arcade_rompath_value(rom_path: &str, flatpak: bool) -> String {
    let mut parts = Vec::new();

    if let Some(parent) = Path::new(rom_path).parent() {
        let value = if flatpak {
            map_path_for_flatpak(&parent.to_string_lossy())
        } else {
            parent.to_string_lossy().to_string()
        };
        parts.push(value);
    }

    if let Some(runtime_dir) = mame_runtime_roms_dir(flatpak) {
        let runtime_text = runtime_dir.to_string_lossy().to_string();
        let runtime_value = if flatpak {
            map_path_for_flatpak(&runtime_text)
        } else {
            runtime_text
        };
        if !parts.iter().any(|existing| existing == &runtime_value) {
            parts.push(runtime_value);
        }
    }

    parts.join(";")
}

fn mame_runtime_roms_dir(flatpak: bool) -> Option<PathBuf> {
    match current_os() {
        "Linux" => dirs::home_dir().map(|home| {
            if flatpak {
                home.join(".var/app/org.mamedev.MAME/data/mame/roms")
            } else {
                home.join(".mame/roms")
            }
        }),
        "Windows" => dirs::data_local_dir().map(|dir| dir.join("mame").join("roms")),
        "macOS" => dirs::home_dir().map(|home| home.join(".mame/roms")),
        _ => None,
    }
}

fn altirra_media_switch(path: &str) -> &'static str {
    let extension = Path::new(path)
        .extension()
        .and_then(|ext| ext.to_str())
        .map(|ext| ext.to_ascii_lowercase());

    match extension.as_deref() {
        Some("atr" | "atx" | "xfd" | "dcm" | "pro" | "atz") => "/disk",
        Some("cas" | "wav") => "/tape",
        Some("car" | "rom" | "bin" | "a52") => "/cart",
        _ => "/run",
    }
}

fn map_path_for_wine(prefix_dir: &Path, path: &str) -> Result<String, String> {
    #[cfg(target_os = "linux")]
    {
        let winepath = find_winepath_command()
            .ok_or_else(|| "winepath is not installed or not available on PATH".to_string())?;
        let output = Command::new(winepath)
            .env("WINEPREFIX", prefix_dir)
            .args(["-w", path])
            .output()
            .map_err(|e| format!("Failed to run winepath: {}", e))?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            return Err(format!("winepath failed: {}", stderr.trim()));
        }

        Ok(String::from_utf8_lossy(&output.stdout).trim().to_string())
    }
    #[cfg(not(target_os = "linux"))]
    {
        let _ = prefix_dir;
        Ok(path.to_string())
    }
}

fn append_launch_args_for_wine(
    cmd: &mut Command,
    prefix_dir: &Path,
    args: &[LaunchArg],
) -> Result<(), String> {
    for arg in args {
        match arg {
            LaunchArg::Literal(value) => {
                cmd.arg(value);
            }
            LaunchArg::Path(path) => {
                cmd.arg(map_path_for_wine(prefix_dir, path)?);
            }
        }
    }

    Ok(())
}

fn append_standalone_rom_and_args_for_wine(
    cmd: &mut Command,
    emulator_name: &str,
    prefix_dir: &Path,
    rom_path: Option<&str>,
    args: &[LaunchArg],
) -> Result<(), String> {
    if emulator_name == "LoopyMSE" {
        if let Some(rom) = rom_path {
            cmd.arg(map_path_for_wine(prefix_dir, rom)?);
        }
        append_launch_args_for_wine(cmd, prefix_dir, args)?;
    } else if emulator_name == "Altirra" {
        append_launch_args_for_wine(cmd, prefix_dir, args)?;
        if let Some(rom) = rom_path {
            cmd.arg(altirra_media_switch(rom));
            cmd.arg(map_path_for_wine(prefix_dir, rom)?);
        }
    } else {
        append_launch_args_for_wine(cmd, prefix_dir, args)?;
        if let Some(rom) = rom_path {
            cmd.arg(map_path_for_wine(prefix_dir, rom)?);
        }
    }

    Ok(())
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
        install_method: get_retroarch_install_method(),
        uninstall_method: None,
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
    let uninstall_method = install_path
        .as_deref()
        .and_then(get_uninstall_method_for_path);
    let display_name = emulator.name.clone();

    EmulatorWithStatus {
        info: emulator,
        is_installed,
        install_method,
        uninstall_method,
        is_retroarch_core: false,
        display_name,
        executable_path: install_path.map(|p| p.to_string_lossy().to_string()),
        firmware_statuses: Vec::new(),
    }
}

/// Find the RetroArch executable
fn find_retroarch_executable() -> Option<PathBuf> {
    // Check flatpak first
    if is_flatpak_installed("org.libretro.RetroArch") {
        return Some(PathBuf::from("flatpak run org.libretro.RetroArch"));
    }

    if let Some(path) = find_lunchbox_nix_profile_executable("retroarch") {
        return Some(path);
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
                "Linux" => {
                    (e.flatpak_id.is_some() && is_flatpak_available())
                        || (nix_package_for_emulator(e).is_some() && is_nix_available())
                        || (wine_install_for_emulator(e).is_some() && is_wine_available())
                }
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
            None,
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
    fn altirra_uses_disk_switch_for_disk_images() {
        let mut cmd = Command::new("echo");
        append_standalone_rom_and_args_native(
            &mut cmd,
            "Altirra",
            None,
            Some("/roms/game.atr"),
            &[LaunchArg::Literal("/portable".to_string())],
        );

        let args = cmd
            .get_args()
            .map(|arg| arg.to_string_lossy().to_string())
            .collect::<Vec<_>>();

        assert_eq!(
            args,
            vec![
                "/portable".to_string(),
                "/disk".to_string(),
                "/roms/game.atr".to_string(),
            ]
        );
    }

    #[test]
    fn altirra_uses_run_switch_for_executables() {
        let mut cmd = Command::new("echo");
        append_standalone_rom_and_args_native(
            &mut cmd,
            "Altirra",
            None,
            Some("/roms/game.xex"),
            &[LaunchArg::Literal("/portable".to_string())],
        );

        let args = cmd
            .get_args()
            .map(|arg| arg.to_string_lossy().to_string())
            .collect::<Vec<_>>();

        assert_eq!(
            args,
            vec![
                "/portable".to_string(),
                "/run".to_string(),
                "/roms/game.xex".to_string(),
            ]
        );
    }

    #[test]
    fn arcade_mame_uses_romset_name_and_rompath() {
        let mut cmd = Command::new("echo");
        append_standalone_rom_and_args_native(
            &mut cmd,
            "MAME",
            Some("Arcade"),
            Some("/roms/arcade/ddsomu.zip"),
            &[],
        );

        let args = cmd
            .get_args()
            .map(|arg| arg.to_string_lossy().to_string())
            .collect::<Vec<_>>();

        assert_eq!(args.first().map(String::as_str), Some("-rompath"));
        assert_eq!(args.last().map(String::as_str), Some("ddsomu"));
        assert!(args
            .get(1)
            .is_some_and(|value| value.contains("/roms/arcade")));
    }

    #[test]
    fn arcade_mame_romsets_stay_zipped_for_launch() {
        let emulator = EmulatorInfo {
            id: 1,
            name: "MAME".to_string(),
            homepage: None,
            supported_os: None,
            winget_id: None,
            homebrew_formula: None,
            flatpak_id: None,
            retroarch_core: Some("mame".to_string()),
            save_directory: None,
            save_extensions: None,
            notes: None,
        };

        let prepared = prepare_rom_for_launch_for_runtime(
            &emulator,
            Some("/roms/arcade/ddsomu.zip"),
            Some("Arcade"),
            true,
        )
        .unwrap();

        assert_eq!(
            prepared.rom_path.as_deref(),
            Some("/roms/arcade/ddsomu.zip")
        );
        assert!(prepared.cleanup_paths.is_empty());
    }

    #[test]
    fn extract_altirra_download_href_prefers_latest_x86_x64_zip() {
        let html = r#"
            <a href="downloads/Altirra-4.40.zip">4.40 (x86/x64)</a>
            <a href="downloads/AltirraARM64-4.40.zip">4.40 (ARM64)</a>
            <a href="downloads/Altirra-4.40-src.7z">source</a>
        "#;

        assert_eq!(
            extract_altirra_download_href(html),
            Some("downloads/Altirra-4.40.zip".to_string())
        );
    }

    #[test]
    fn wine_install_mapping_covers_altirra() {
        let base = EmulatorInfo {
            id: 1,
            name: "Altirra".to_string(),
            homepage: None,
            supported_os: None,
            winget_id: None,
            homebrew_formula: None,
            flatpak_id: None,
            retroarch_core: None,
            save_directory: None,
            save_extensions: None,
            notes: None,
        };

        let info = wine_install_for_emulator(&base).expect("Altirra should have Wine metadata");
        assert_eq!(info.slug, "altirra");
        assert_eq!(
            info.download_page_url,
            "https://www.virtualdub.org/altirra.html"
        );
        assert_eq!(
            info.executable_candidates,
            &["Altirra64.exe", "Altirra.exe"]
        );
    }

    #[test]
    fn linux_visibility_includes_wine_backed_windows_emulator() {
        let altirra = EmulatorInfo {
            id: 1,
            name: "Altirra".to_string(),
            homepage: None,
            supported_os: Some("Windows".to_string()),
            winget_id: None,
            homebrew_formula: None,
            flatpak_id: None,
            retroarch_core: None,
            save_directory: None,
            save_extensions: None,
            notes: None,
        };

        if current_os() == "Linux" {
            assert!(is_emulator_visible_on_current_os(&altirra));
        }
    }

    #[test]
    fn appimage_install_mapping_covers_hypseus_singe() {
        let emulator = EmulatorInfo {
            id: 1,
            name: "Hypseus Singe".to_string(),
            homepage: None,
            supported_os: Some("Linux".to_string()),
            winget_id: None,
            homebrew_formula: None,
            flatpak_id: None,
            retroarch_core: None,
            save_directory: None,
            save_extensions: None,
            notes: None,
        };

        let info = appimage_install_for_emulator(&emulator)
            .expect("Hypseus Singe should have AppImage metadata");
        assert_eq!(info.slug, "hypseus-singe");
        assert_eq!(info.github_repo, "DirtBagXon/hypseus-singe");
    }

    #[test]
    fn selects_best_github_appimage_asset() {
        let release = GitHubRelease {
            tag_name: "v2.11.7".to_string(),
            assets: vec![
                GitHubReleaseAsset {
                    name: "hypseus-singe-v2.11.7-src.tar.gz".to_string(),
                    browser_download_url: "https://example.invalid/src.tar.gz".to_string(),
                },
                GitHubReleaseAsset {
                    name: "hypseus-singe-v2.11.7-x86_64.AppImage".to_string(),
                    browser_download_url: "https://example.invalid/plain.appimage".to_string(),
                },
                GitHubReleaseAsset {
                    name: "hypseus-singe-v2.11.7-SteamDeck-x86_64.AppImage".to_string(),
                    browser_download_url: "https://example.invalid/steamdeck.appimage".to_string(),
                },
            ],
        };
        let info = appimage_install_for_slug("hypseus-singe").unwrap();

        let asset = select_github_appimage_asset(&release, &info).expect("asset should match");
        assert_eq!(
            asset.name,
            "hypseus-singe-v2.11.7-SteamDeck-x86_64.AppImage"
        );
    }

    #[test]
    fn selects_wrapped_appimage_release_asset() {
        let release = GitHubRelease {
            tag_name: "v2.11.7".to_string(),
            assets: vec![
                GitHubReleaseAsset {
                    name: "hypseus-singe-v2.11.7-src.tar.gz".to_string(),
                    browser_download_url: "https://example.invalid/src.tar.gz".to_string(),
                },
                GitHubReleaseAsset {
                    name: "Hypseus.Singe-v2.11.7-win64.zip".to_string(),
                    browser_download_url: "https://example.invalid/win64.zip".to_string(),
                },
                GitHubReleaseAsset {
                    name: "hypseus-singe_v2.11.7_AppImage.tar.gz".to_string(),
                    browser_download_url: "https://example.invalid/appimage.tar.gz".to_string(),
                },
            ],
        };
        let info = appimage_install_for_slug("hypseus-singe").unwrap();

        let asset = select_github_appimage_asset(&release, &info).expect("asset should match");
        assert_eq!(asset.name, "hypseus-singe_v2.11.7_AppImage.tar.gz");
    }

    #[test]
    fn parses_zsync_update_transport() {
        assert_eq!(
            normalize_appimage_update_transport("zsync|https://example.invalid/app.zsync"),
            Some("zsync".to_string())
        );
        assert_eq!(
            normalize_appimage_update_transport(
                "gh-releases-zsync|owner|repo|latest|Example*x86_64.AppImage.zsync"
            ),
            Some("zsync".to_string())
        );
    }

    #[test]
    fn selects_best_appimage_updater_asset() {
        let release = GitHubRelease {
            tag_name: "2.0.0-alpha".to_string(),
            assets: vec![
                GitHubReleaseAsset {
                    name: "AppImageUpdate-x86_64.AppImage".to_string(),
                    browser_download_url: "https://example.invalid/gui.appimage".to_string(),
                },
                GitHubReleaseAsset {
                    name: "appimageupdatetool-x86_64.AppImage".to_string(),
                    browser_download_url: "https://example.invalid/cli.appimage".to_string(),
                },
            ],
        };

        let asset = select_github_updater_asset(&release, &appimage_updater_info())
            .expect("updater asset should match");
        assert_eq!(asset.name, "appimageupdatetool-x86_64.AppImage");
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

    #[test]
    fn lunchbox_nix_profile_bin_dir_uses_dedicated_profile() {
        let bin_dir = lunchbox_nix_profile_bin_dir();
        assert!(bin_dir.ends_with(Path::new("nix/profiles/lunchbox/bin")));
    }

    #[test]
    fn nix_package_mapping_covers_known_linux_emulators() {
        let base = EmulatorInfo {
            id: 1,
            name: "Atari800".to_string(),
            homepage: None,
            supported_os: None,
            winget_id: None,
            homebrew_formula: None,
            flatpak_id: None,
            retroarch_core: None,
            save_directory: None,
            save_extensions: None,
            notes: None,
        };
        let dolphin = EmulatorInfo {
            name: "Dolphin".to_string(),
            ..base.clone()
        };
        let atari_pp = EmulatorInfo {
            name: "Atari++".to_string(),
            ..base.clone()
        };
        let mednafen = EmulatorInfo {
            name: "Mednafen".to_string(),
            ..base.clone()
        };
        let fs_uae = EmulatorInfo {
            name: "FS-UAE".to_string(),
            ..base.clone()
        };
        let vice = EmulatorInfo {
            name: "VICE".to_string(),
            ..base.clone()
        };
        let vice_xpet = EmulatorInfo {
            name: "VICE (xpet)".to_string(),
            ..base.clone()
        };

        assert_eq!(nix_package_for_emulator(&base), Some("atari800"));
        assert_eq!(nix_package_for_emulator(&dolphin), Some("dolphin-emu"));
        assert_eq!(nix_package_for_emulator(&atari_pp), Some("ataripp"));
        assert_eq!(nix_package_for_emulator(&mednafen), Some("mednafen"));
        assert_eq!(nix_package_for_emulator(&fs_uae), Some("fsuae"));
        assert_eq!(nix_package_for_emulator(&vice), Some("vice"));
        assert_eq!(nix_package_for_emulator(&vice_xpet), Some("vice"));
    }

    #[test]
    fn executable_aliases_cover_nix_installed_variants() {
        assert!(get_executable_names("Atari++").contains(&"ataripp".to_string()));
        assert!(get_executable_names("FS-UAE").contains(&"fs-uae-launcher".to_string()));
        assert!(get_executable_names("DeSmuME").contains(&"desmume-gtk".to_string()));
        assert!(get_executable_names("Hypseus Singe").contains(&"hypseus".to_string()));
        assert!(get_executable_names("Hypseus Singe").contains(&"singe".to_string()));
        assert!(get_executable_names("VICE").contains(&"x64sc".to_string()));
        assert!(get_executable_names("VICE (xpet)").contains(&"xpet".to_string()));
        assert!(get_executable_names("VICE (xvic)").contains(&"xvic".to_string()));
    }
}
