use std::collections::{HashMap, HashSet};
use std::fs::{self, File};
use std::io::{Read, Write};
use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};

use anyhow::{Context, Result, bail};
use directories::{BaseDirs, ProjectDirs, UserDirs};
use lava_torrent::torrent::v1::Torrent;
use rusqlite::{Connection, params};
use serde::Deserialize;
use sha2::{Digest, Sha256};
use uuid::Uuid;
use walkdir::WalkDir;

use crate::emulator::{EmulatorExecutable, EmulatorRuntimeKind, RomEmulatorOption};
use crate::platform_process::host_command;
use crate::settings::{
    FirmwareDownloadReceipt, FirmwareFileReceipt, FirmwareInstallReceipt, FirmwarePackageReceipt,
    SettingsStore,
};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum FirmwareHost {
    Linux,
    Windows,
    MacOs,
}

impl FirmwareHost {
    fn current() -> Result<Self> {
        if cfg!(target_os = "linux") {
            Ok(Self::Linux)
        } else if cfg!(target_os = "windows") {
            Ok(Self::Windows)
        } else if cfg!(target_os = "macos") {
            Ok(Self::MacOs)
        } else {
            bail!("firmware management does not support this host operating system")
        }
    }
}

const MAX_FIRMWARE_ARCHIVE_BYTES: u64 = 4 * 1024 * 1024 * 1024;
const MAX_FIRMWARE_EXTRACTED_BYTES: u64 = 16 * 1024 * 1024 * 1024;
const MAX_FIRMWARE_FILES: usize = 250_000;
const MAX_GITHUB_RELEASE_BYTES: u64 = 4 * 1024 * 1024;
const FIRMWARE_DOWNLOAD_TIMEOUT: Duration = Duration::from_secs(60 * 60);
const SWITCH_KEYS_MAX_BYTES: u64 = 8 * 1024 * 1024;
const SWITCH_KEY_VALUE_MAX_HEX_CHARS: usize = 4096;
const SWITCH_KEYS_PACKAGE: &str = "switch-keys.zip";
const SWITCH_FIRMWARE_PACKAGE: &str = "switch-firmware.zip";

#[derive(Debug, Deserialize)]
struct GitHubRelease {
    zipball_url: String,
    assets: Vec<GitHubAsset>,
}

#[derive(Debug, Deserialize)]
struct GitHubAsset {
    name: String,
    browser_download_url: String,
    #[serde(default)]
    size: u64,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct FirmwareStatus {
    pub rule_key: String,
    pub source_id: String,
    pub source_transport: String,
    pub source_url: String,
    pub torrent_file: String,
    pub path_prefix: String,
    pub package_name: String,
    pub runtime_kind: String,
    pub install_mode: String,
    pub required: bool,
    pub supports_hle_fallback: bool,
    pub target_strategy: String,
    pub imported: bool,
    pub synced: bool,
    pub runtime_path: String,
    pub target_path: String,
    pub notes: String,
}

impl FirmwareStatus {
    pub fn needs_action(&self) -> bool {
        self.required
            && !self.supports_hle_fallback
            && (!self.imported
                || (!self.synced
                    && !matches!(
                        self.target_strategy.as_str(),
                        "launch_scoped" | "manual_import"
                    )))
    }

    pub fn source_label(&self) -> &'static str {
        if self.target_strategy == "managed_import" {
            return "User-owned import";
        }
        match self.source_transport.as_str() {
            "minerva" => "Minerva",
            "github_release" => "GitHub release",
            "direct" => "Official download",
            "manual" => "Manual import",
            _ => "Firmware source",
        }
    }

    pub fn can_sync(&self) -> bool {
        self.target_strategy != "manual_import"
            && self.imported
            && !self.synced
            && !self.target_path.is_empty()
    }
}

#[derive(Clone, Debug)]
struct FirmwareRuleRow {
    rule_key: String,
    runtime_kind: String,
    runtime_name: String,
    source_id: String,
    source_transport: String,
    source_url: String,
    torrent_file: String,
    path_prefix: String,
    package_name: String,
    target_subdir: String,
    install_mode: String,
    target_strategy: String,
    required: bool,
    supports_hle_fallback: bool,
    notes: String,
}

pub fn statuses_for_options(
    catalog_database: &Path,
    platform: &str,
    rom_path: &Path,
    options: &[RomEmulatorOption],
) -> Result<Vec<Vec<FirmwareStatus>>> {
    let connection = crate::catalog::open_read_only(catalog_database, "firmware catalog")?;
    let platform_id = resolve_platform_id(&connection, platform)?;
    let store = SettingsStore::open_default()?;
    let packages = store
        .firmware_packages()?
        .into_iter()
        .filter(package_receipt_files_exist)
        .map(|package| {
            (
                (package.source_id.clone(), package.package_name.clone()),
                package,
            )
        })
        .collect::<HashMap<_, _>>();
    let installs = store
        .firmware_installs()?
        .into_iter()
        .map(|install| {
            (
                (install.rule_key.clone(), install.runtime_path.clone()),
                install,
            )
        })
        .collect::<HashMap<_, _>>();

    options
        .iter()
        .map(|option| {
            let rules = load_rules(&connection, &platform_id, option)?;
            rules
                .into_iter()
                .map(|rule| {
                    status_for_rule(&rule, option, platform, rom_path, &packages, &installs)
                })
                .collect()
        })
        .collect()
}

pub fn summarize(statuses: &[FirmwareStatus]) -> String {
    if statuses.is_empty() {
        return "No external firmware is required for this runtime and platform.".to_owned();
    }
    let manual = statuses
        .iter()
        .filter(|status| status.target_strategy == "manual_import" && !status.imported)
        .count();
    let hle = statuses
        .iter()
        .filter(|status| status.supports_hle_fallback && !status.imported)
        .count();
    let missing = statuses
        .iter()
        .filter(|status| status.needs_action())
        .count();
    if manual > 0 {
        format!(
            "{manual} firmware item{} require{} a manual dump and emulator configuration.",
            if manual == 1 { "" } else { "s" },
            if manual == 1 { "s" } else { "" }
        )
    } else if missing > 0 {
        format!(
            "{missing} required firmware package{} need{} importing or application to the selected emulator.",
            if missing == 1 { "" } else { "s" },
            if missing == 1 { "s" } else { "" }
        )
    } else if hle > 0 {
        format!(
            "Firmware is optional for this runtime; {hle} package{} can improve compatibility.",
            if hle == 1 { "" } else { "s" }
        )
    } else {
        format!(
            "All {} firmware requirement{} are ready.",
            statuses.len(),
            if statuses.len() == 1 { "" } else { "s" }
        )
    }
}

pub fn open_firmware_directory(statuses: &[FirmwareStatus]) -> Result<PathBuf> {
    let first = statuses
        .first()
        .context("the selected emulator has no firmware directory")?;
    let directory = PathBuf::from(&first.runtime_path);
    if directory.as_os_str().is_empty() {
        bail!("this firmware is resolved only while constructing the launch command");
    }
    fs::create_dir_all(&directory)
        .with_context(|| format!("creating firmware directory {}", directory.display()))?;
    if statuses
        .iter()
        .any(|status| status.target_strategy == "manual_import")
    {
        write_manual_readme(&directory, statuses)?;
    }
    open_path(&directory)?;
    Ok(directory)
}

fn load_rules(
    connection: &Connection,
    platform_id: &str,
    option: &RomEmulatorOption,
) -> Result<Vec<FirmwareRuleRow>> {
    let (condition, values): (&str, Vec<String>) = match option.runtime_kind {
        EmulatorRuntimeKind::RetroArch => (
            "r.runtime_kind='retroarch' AND r.runtime_name=?2",
            vec![platform_id.to_owned(), option.core_name.clone()],
        ),
        EmulatorRuntimeKind::Standalone => (
            "r.runtime_kind<>'retroarch' AND r.emulator_id=?2",
            vec![platform_id.to_owned(), option.emulator_id.clone()],
        ),
    };
    let sql = format!(
        "SELECT r.rule_key, r.runtime_kind, r.runtime_name, r.source_id,
                s.transport, coalesce(s.locator_url, ''),
                coalesce(s.torrent_file, ''), coalesce(s.path_prefix, ''),
                r.source_package_name, r.target_subdir,
                r.install_mode, r.target_strategy, r.required,
                r.supports_hle_fallback, r.notes
         FROM firmware_rules r
         JOIN firmware_sources s ON s.id=r.source_id
         WHERE r.platform_id=?1 AND {condition}
         ORDER BY r.required DESC, r.source_package_name COLLATE NOCASE, r.rule_key"
    );
    let mut statement = connection.prepare(&sql)?;
    let rows = statement.query_map(params![values[0], values[1]], |row| {
        Ok(FirmwareRuleRow {
            rule_key: row.get(0)?,
            runtime_kind: row.get(1)?,
            runtime_name: row.get(2)?,
            source_id: row.get(3)?,
            source_transport: row.get(4)?,
            source_url: row.get(5)?,
            torrent_file: row.get(6)?,
            path_prefix: row.get(7)?,
            package_name: row.get(8)?,
            target_subdir: row.get(9)?,
            install_mode: row.get(10)?,
            target_strategy: row.get(11)?,
            required: row.get(12)?,
            supports_hle_fallback: row.get(13)?,
            notes: row.get(14)?,
        })
    })?;
    Ok(rows.collect::<rusqlite::Result<Vec<_>>>()?)
}

fn status_for_rule(
    rule: &FirmwareRuleRow,
    option: &RomEmulatorOption,
    platform: &str,
    rom_path: &Path,
    packages: &HashMap<(String, String), FirmwarePackageReceipt>,
    installs: &HashMap<(String, String), FirmwareInstallReceipt>,
) -> Result<FirmwareStatus> {
    let runtime_root = runtime_root(rule, option, platform, rom_path)?;
    let runtime_path = runtime_root
        .as_deref()
        .map(Path::to_path_buf)
        .unwrap_or_default();
    let target_root = runtime_root
        .as_ref()
        .and_then(|root| target_root_for_rule(root, rule));
    let target_path = target_root.as_ref().map(|target| {
        if rule.install_mode == "copy_archive" {
            target.join(&rule.package_name)
        } else {
            target.clone()
        }
    });
    let package_key = (rule.source_id.clone(), rule.package_name.clone());
    if rule.runtime_kind == "ryubing"
        && rule.source_id == "manual:nintendo-switch-firmware"
        && let Some(target) = target_path.as_deref()
    {
        let _ = migrate_legacy_ryubing_firmware_layout(target);
    }
    let runtime_target_ready = target_path
        .as_deref()
        .is_some_and(|target| managed_runtime_target_ready(rule, target));
    let imported = if rule.target_strategy == "manual_import" {
        directory_contains_files(&runtime_path)
    } else {
        packages.contains_key(&package_key) || runtime_target_ready
    };
    let runtime_text = runtime_path.to_string_lossy().into_owned();
    let synced = if rule.target_strategy == "managed_import" {
        runtime_target_ready
    } else {
        target_path.as_ref().is_some_and(|target| {
            installs
                .get(&(rule.rule_key.clone(), runtime_text.clone()))
                .is_some_and(|receipt| {
                    receipt.source_id == rule.source_id
                        && receipt.package_name == rule.package_name
                        && Path::new(&receipt.target_path) == target
                        && target.exists()
                })
        })
    };

    Ok(FirmwareStatus {
        rule_key: rule.rule_key.clone(),
        source_id: rule.source_id.clone(),
        source_transport: rule.source_transport.clone(),
        source_url: rule.source_url.clone(),
        torrent_file: rule.torrent_file.clone(),
        path_prefix: rule.path_prefix.clone(),
        package_name: rule.package_name.clone(),
        runtime_kind: rule.runtime_kind.clone(),
        install_mode: rule.install_mode.clone(),
        required: rule.required,
        supports_hle_fallback: rule.supports_hle_fallback,
        target_strategy: rule.target_strategy.clone(),
        imported,
        synced,
        runtime_path: runtime_text,
        target_path: target_path
            .as_deref()
            .map(Path::to_string_lossy)
            .map(|path| path.into_owned())
            .unwrap_or_default(),
        notes: rule.notes.clone(),
    })
}

fn managed_runtime_target_ready(rule: &FirmwareRuleRow, target: &Path) -> bool {
    if rule.target_strategy != "managed_import" {
        return false;
    }
    match (rule.source_id.as_str(), rule.package_name.as_str()) {
        ("manual:nintendo-switch-keys", SWITCH_KEYS_PACKAGE) => {
            validate_switch_prod_keys(&target.join("prod.keys")).is_ok()
        }
        ("manual:nintendo-switch-firmware", SWITCH_FIRMWARE_PACKAGE) => {
            validate_switch_firmware_directory(target, &rule.runtime_kind).is_ok()
        }
        _ => false,
    }
}

fn target_root_for_rule(runtime_root: &Path, rule: &FirmwareRuleRow) -> Option<PathBuf> {
    if rule.target_subdir == "@hash" {
        return (rule.runtime_kind == "mame")
            .then(|| runtime_root.parent().map(|parent| parent.join("hash")))?;
    }
    if rule.target_subdir.is_empty() {
        Some(runtime_root.to_path_buf())
    } else {
        Some(runtime_root.join(&rule.target_subdir))
    }
}

fn runtime_root(
    rule: &FirmwareRuleRow,
    option: &RomEmulatorOption,
    platform: &str,
    rom_path: &Path,
) -> Result<Option<PathBuf>> {
    if rule.target_strategy == "manual_import" {
        return Ok(Some(manual_directory(
            &rule.runtime_kind,
            &rule.runtime_name,
            platform,
        )?));
    }
    if rule.target_strategy == "launch_scoped" {
        return if rule.runtime_kind == "retroarch" && rule.runtime_name == "mame" {
            Ok(rom_path.parent().map(Path::to_path_buf))
        } else {
            Ok(None)
        };
    }

    let base = BaseDirs::new().context("could not determine firmware runtime directories")?;
    let home = base.home_dir();
    let flatpak = matches!(option.executable, EmulatorExecutable::Flatpak { .. });
    if matches!(rule.runtime_kind.as_str(), "eden" | "ryubing" | "torzu") {
        let host = FirmwareHost::current()?;
        let switch_data_dir = if host == FirmwareHost::MacOs {
            std::env::var_os("XDG_DATA_HOME")
                .filter(|value| !value.is_empty())
                .map(PathBuf::from)
                .unwrap_or_else(|| home.join(".local/share"))
        } else {
            base.data_local_dir().to_path_buf()
        };
        return switch_runtime_root(
            host,
            &rule.runtime_kind,
            &option.executable,
            rom_path,
            home,
            base.config_dir(),
            &switch_data_dir,
        )
        .map(Some);
    }
    let root = match rule.runtime_kind.as_str() {
        "retroarch" if cfg!(target_os = "linux") && flatpak => {
            home.join(".var/app/org.libretro.RetroArch/config/retroarch/system")
        }
        "retroarch" if cfg!(target_os = "linux") => base.config_dir().join("retroarch/system"),
        "retroarch" if cfg!(target_os = "windows") => {
            base.data_local_dir().join("RetroArch/system")
        }
        "retroarch" => home.join("Library/Application Support/RetroArch/system"),
        "duckstation" if cfg!(target_os = "linux") && flatpak => {
            home.join(".var/app/org.duckstation.DuckStation/data/duckstation/bios")
        }
        "duckstation" if cfg!(target_os = "linux") => {
            base.data_local_dir().join("duckstation/bios")
        }
        "duckstation" if cfg!(target_os = "windows") => {
            base.data_local_dir().join("duckstation/bios")
        }
        "duckstation" => home.join("Library/Application Support/DuckStation/bios"),
        "pcsx2" if cfg!(target_os = "linux") && flatpak => {
            home.join(".var/app/net.pcsx2.PCSX2/config/PCSX2/bios")
        }
        "pcsx2" if cfg!(target_os = "linux") => base.config_dir().join("PCSX2/bios"),
        "pcsx2" if cfg!(target_os = "windows") => base.config_dir().join("PCSX2/bios"),
        "pcsx2" => home.join("Library/Application Support/PCSX2/bios"),
        "melonds" if cfg!(target_os = "linux") && flatpak => {
            home.join(".var/app/net.kuribo64.melonDS/config/melonDS")
        }
        "melonds" if cfg!(target_os = "linux") => base.config_dir().join("melonDS"),
        "melonds" if cfg!(target_os = "windows") => base.config_dir().join("melonDS"),
        "melonds" => home.join("Library/Application Support/melonDS"),
        "dolphin" if cfg!(target_os = "linux") && flatpak => {
            home.join(".var/app/org.DolphinEmu.dolphin-emu/data/dolphin-emu")
        }
        "dolphin" if cfg!(target_os = "linux") => {
            let xdg = base.data_local_dir().join("dolphin-emu");
            if xdg.exists() {
                xdg
            } else {
                home.join(".dolphin-emu")
            }
        }
        "dolphin" if cfg!(target_os = "windows") => UserDirs::new()
            .and_then(|dirs| dirs.document_dir().map(Path::to_path_buf))
            .unwrap_or_else(|| base.data_local_dir().to_path_buf())
            .join("Dolphin Emulator"),
        "dolphin" => home.join("Library/Application Support/Dolphin"),
        "flycast" if cfg!(target_os = "linux") && flatpak => {
            home.join(".var/app/org.flycast.Flycast/data/flycast")
        }
        "flycast" if cfg!(target_os = "linux") => base.data_local_dir().join("flycast"),
        "flycast" if cfg!(target_os = "windows") => base.data_local_dir().join("Flycast"),
        "flycast" => home.join("Library/Application Support/flycast"),
        "openmsx" if cfg!(target_os = "linux") => home.join(".openMSX/share/systemroms"),
        "openmsx" if cfg!(target_os = "windows") => UserDirs::new()
            .and_then(|dirs| dirs.document_dir().map(Path::to_path_buf))
            .unwrap_or_else(|| home.to_path_buf())
            .join("openMSX/share/systemroms"),
        "openmsx" => home.join("Library/openMSX/share/systemroms"),
        "snes9x" if cfg!(target_os = "linux") && flatpak => {
            home.join(".var/app/com.snes9x.Snes9x/config/snes9x/Bios")
        }
        "snes9x" => home.join(".snes9x/Bios"),
        "o2em" => home.join(".o2em"),
        "mame" if cfg!(target_os = "linux") && flatpak => {
            home.join(".var/app/org.mamedev.MAME/data/mame/roms")
        }
        "mame" if cfg!(target_os = "windows") => base.data_local_dir().join("mame/roms"),
        "mame" => home.join(".mame/roms"),
        "86box" if cfg!(target_os = "linux") && flatpak => {
            home.join(".var/app/net._86box._86Box/data/86Box/roms")
        }
        "86box" if cfg!(target_os = "macos") => home.join("Library/Application Support/86Box/roms"),
        "86box" => base.data_local_dir().join("86Box/roms"),
        "pcem" => home.join(".pcem/roms"),
        kind => bail!("firmware runtime {kind} has no reviewed target adapter"),
    };
    Ok(Some(root))
}

fn switch_runtime_root(
    host: FirmwareHost,
    runtime_kind: &str,
    executable: &EmulatorExecutable,
    rom_path: &Path,
    home: &Path,
    config_dir: &Path,
    data_local_dir: &Path,
) -> Result<PathBuf> {
    let flatpak_app_id = match executable {
        EmulatorExecutable::Flatpak { app_id, .. } if host == FirmwareHost::Linux => Some(app_id),
        _ => None,
    };
    if let Some(app_id) = flatpak_app_id {
        let sandbox = home.join(".var/app").join(app_id);
        return match runtime_kind {
            "eden" => Ok(sandbox.join("data/eden")),
            "ryubing" => Ok(sandbox.join("config/Ryujinx")),
            "torzu" => Ok(sandbox.join("data/yuzu")),
            _ => bail!("unsupported Switch firmware runtime {runtime_kind}"),
        };
    }

    let executable_parent = match executable {
        EmulatorExecutable::Native(executable) => executable.parent(),
        EmulatorExecutable::Wine { executable, .. } => executable.parent(),
        EmulatorExecutable::Flatpak { .. } => None,
    };
    let portable = match runtime_kind {
        "ryubing" => executable_parent.map(|parent| parent.join("portable")),
        // Eden and yuzu-family builds use the executable directory on Windows,
        // but the process working directory on Unix. Lunchbox launches ROMs with
        // their containing directory as the working directory.
        "eden" | "torzu" if host == FirmwareHost::Windows => {
            executable_parent.map(|parent| parent.join("user"))
        }
        "eden" | "torzu" => rom_path.parent().map(|parent| parent.join("user")),
        _ => bail!("unsupported Switch firmware runtime {runtime_kind}"),
    };
    if let Some(portable) = portable
        && portable.is_dir()
    {
        return Ok(portable);
    }

    match (runtime_kind, host) {
        ("ryubing", FirmwareHost::MacOs) => Ok(home.join("Library/Application Support/Ryujinx")),
        ("ryubing", _) => Ok(config_dir.join("Ryujinx")),
        ("eden", FirmwareHost::Windows) => Ok(config_dir.join("eden")),
        ("torzu", FirmwareHost::Windows) => Ok(config_dir.join("yuzu")),
        // These C++ runtimes intentionally use XDG_DATA_HOME on every Unix,
        // including macOS, rather than Apple's Application Support directory.
        ("eden", FirmwareHost::Linux | FirmwareHost::MacOs) => Ok(data_local_dir.join("eden")),
        ("torzu", FirmwareHost::Linux | FirmwareHost::MacOs) => Ok(data_local_dir.join("yuzu")),
        _ => bail!("unsupported Switch firmware runtime {runtime_kind}"),
    }
}

fn resolve_platform_id(connection: &Connection, platform: &str) -> Result<String> {
    let mut statement = connection.prepare(
        "SELECT id FROM platforms WHERE canonical_name=?1 COLLATE NOCASE
         UNION
         SELECT platform_id FROM platform_aliases WHERE alias=?1 COLLATE NOCASE",
    )?;
    let rows = statement.query_map([platform.trim()], |row| row.get::<_, String>(0))?;
    let ids = rows.collect::<rusqlite::Result<HashSet<_>>>()?;
    match ids.len() {
        1 => Ok(ids
            .into_iter()
            .next()
            .expect("single exact platform identity")),
        0 => bail!("no exact canonical platform identity exists for {platform}"),
        _ => bail!("platform alias {platform} resolves to multiple canonical identities"),
    }
}

fn manual_directory(runtime_kind: &str, runtime_name: &str, platform: &str) -> Result<PathBuf> {
    let root = ProjectDirs::from("com", "Lunchbox", "Lunchbox")
        .map(|dirs| dirs.data_local_dir().join("firmware/manual"))
        .context("could not determine the Lunchbox firmware directory")?;
    Ok(root
        .join(safe_component(runtime_kind))
        .join(safe_component(runtime_name))
        .join(safe_component(platform)))
}

fn safe_component(value: &str) -> String {
    value
        .chars()
        .map(|character| {
            if character.is_ascii_alphanumeric() || matches!(character, '.' | '_' | '-') {
                character
            } else {
                '_'
            }
        })
        .collect()
}

fn directory_contains_files(path: &Path) -> bool {
    let Ok(entries) = fs::read_dir(path) else {
        return false;
    };
    entries.flatten().any(|entry| {
        entry
            .file_name()
            .to_str()
            .is_some_and(|name| name != "README.txt")
    })
}

fn write_manual_readme(directory: &Path, statuses: &[FirmwareStatus]) -> Result<()> {
    let packages = statuses
        .iter()
        .map(|status| format!("- {}\n  {}", status.package_name, status.notes))
        .collect::<Vec<_>>()
        .join("\n");
    let body = format!(
        "Lunchbox manual firmware folder\n\nPlace legally acquired firmware for this exact runtime and platform here.\n\nExpected files or packages:\n{packages}\n\nLunchbox does not download manual-only dumps. Configure the emulator to use these files when its runtime adapter requires it.\n"
    );
    let path = directory.join("README.txt");
    if fs::read_to_string(&path).ok().as_deref() != Some(body.as_str()) {
        fs::write(&path, body)?;
    }
    Ok(())
}

pub fn import_and_sync(statuses: &[FirmwareStatus], selected_path: &Path) -> Result<String> {
    import_and_sync_with_progress(statuses, selected_path, |_, _| {})
}

pub fn import_and_sync_with_progress(
    statuses: &[FirmwareStatus],
    selected_path: &Path,
    mut progress: impl FnMut(String, u8),
) -> Result<String> {
    let mut last_progress = None;
    let mut report = |message: String, percent: u8| {
        if last_progress != Some(percent) {
            last_progress = Some(percent);
            progress(message, percent);
        }
    };
    report("Inspecting the selected firmware package…".into(), 0);
    if !selected_path.is_file() {
        bail!("the selected firmware package is not a readable local file");
    }
    let selected_name = selected_path
        .file_name()
        .and_then(|name| name.to_str())
        .context("the selected firmware package name is not valid Unicode")?;
    let selected_is_switch_keys = selected_name.eq_ignore_ascii_case("prod.keys")
        || (selected_name.to_ascii_lowercase().ends_with(".zip")
            && switch_key_zip_contains_prod_keys(selected_path).unwrap_or(false));
    let candidates = statuses
        .iter()
        .filter(|status| status.target_strategy != "manual_import" && !status.imported)
        .filter(|status| firmware_selection_matches(status, selected_name, selected_is_switch_keys))
        .collect::<Vec<_>>();
    if candidates.is_empty() {
        let mut expected = statuses
            .iter()
            .filter(|status| status.target_strategy != "manual_import" && !status.imported)
            .map(|status| status.package_name.as_str())
            .collect::<Vec<_>>();
        expected.sort_unstable();
        expected.dedup();
        if expected.is_empty() {
            bail!("all selected-runtime firmware packages are already imported");
        }
        bail!(
            "selected {selected_name}; this runtime expects an exact package named {}",
            expected.join(" or ")
        );
    }
    let identities = candidates
        .iter()
        .map(|status| (status.source_id.as_str(), status.package_name.as_str()))
        .collect::<HashSet<_>>();
    if identities.len() != 1 {
        bail!(
            "the selected filename maps to multiple firmware sources; choose a source-specific action"
        );
    }
    let selected = candidates[0];
    validate_managed_firmware_selection_with_progress(selected, selected_path, |done, total| {
        let percent = progress_between(2, 20, done, total);
        report(
            format!("Validating firmware package · {done}/{total} entries"),
            percent,
        );
    })?;
    report("Preparing the verified firmware package…".into(), 22);
    let normalized_keys = (selected.package_name == SWITCH_KEYS_PACKAGE)
        .then(|| normalize_switch_key_package(selected_path))
        .transpose()?;
    let package_path = normalized_keys
        .as_ref()
        .map(|(_, package)| package.as_path())
        .unwrap_or(selected_path);
    let imported = import_package_with_progress(
        &selected.source_id,
        &selected.package_name,
        package_path,
        |message, percent| report(message, progress_between(22, 68, usize::from(percent), 100)),
    );
    if let Some((staging, _)) = normalized_keys.as_ref() {
        let _ = fs::remove_dir_all(staging);
    }
    let receipt = imported?;
    report(
        "Verifying and applying files to the emulator profile…".into(),
        70,
    );
    let synced = sync_matching_statuses_with_progress(
        statuses,
        std::slice::from_ref(&receipt),
        |message, percent| report(message, progress_between(70, 99, usize::from(percent), 100)),
    )?;
    report("Firmware setup complete.".into(), 100);
    Ok(format!(
        "Imported {} ({} file{}){}.",
        receipt.package_name,
        receipt.file_count,
        if receipt.file_count == 1 { "" } else { "s" },
        if synced == 0 {
            " and recorded it for launch-time staging".to_owned()
        } else {
            format!(
                " and applied {synced} runtime requirement{}",
                if synced == 1 { "" } else { "s" }
            )
        }
    ))
}

fn progress_between(start: u8, end: u8, done: usize, total: usize) -> u8 {
    if total == 0 || start >= end {
        return start;
    }
    let span = usize::from(end - start);
    let offset = done.min(total).saturating_mul(span) / total;
    start.saturating_add(u8::try_from(offset).unwrap_or(end - start))
}

fn firmware_selection_matches(
    status: &FirmwareStatus,
    selected_name: &str,
    selected_is_switch_keys: bool,
) -> bool {
    if status.source_id == "manual:nintendo-switch-keys"
        && status.package_name == SWITCH_KEYS_PACKAGE
    {
        return selected_is_switch_keys;
    }
    if status.source_id == "manual:nintendo-switch-firmware"
        && status.package_name == SWITCH_FIRMWARE_PACKAGE
    {
        return !selected_is_switch_keys && selected_name.to_ascii_lowercase().ends_with(".zip");
    }
    status.package_name.eq_ignore_ascii_case(selected_name)
}

fn validate_managed_firmware_selection_with_progress(
    status: &FirmwareStatus,
    selected_path: &Path,
    mut progress: impl FnMut(usize, usize),
) -> Result<()> {
    if status.target_strategy != "managed_import" {
        return Ok(());
    }
    match (status.source_id.as_str(), status.package_name.as_str()) {
        ("manual:nintendo-switch-keys", SWITCH_KEYS_PACKAGE) => {
            read_switch_key_package(selected_path).map(|_| ())
        }
        ("manual:nintendo-switch-firmware", SWITCH_FIRMWARE_PACKAGE) => {
            validate_switch_firmware_zip_with_progress(selected_path, &mut progress)
        }
        (source, package) => {
            bail!("managed firmware source {source} package {package} has no validator")
        }
    }
}

struct SwitchKeyPackage {
    prod_keys: Vec<u8>,
    title_keys: Option<Vec<u8>>,
}

fn switch_key_zip_contains_prod_keys(path: &Path) -> Result<bool> {
    let file = File::open(path)?;
    let mut archive = zip::ZipArchive::new(file).context("opening Switch keys ZIP")?;
    if archive.len() > 128 {
        bail!("Switch keys ZIP contains too many entries");
    }
    for index in 0..archive.len() {
        let entry = archive.by_index(index)?;
        let relative = entry
            .enclosed_name()
            .context("Switch keys ZIP contains an unsafe path")?;
        if relative
            .file_name()
            .and_then(|name| name.to_str())
            .is_some_and(|name| name.eq_ignore_ascii_case("prod.keys"))
        {
            return Ok(true);
        }
    }
    Ok(false)
}

fn read_switch_key_package(path: &Path) -> Result<SwitchKeyPackage> {
    let name = path
        .file_name()
        .and_then(|name| name.to_str())
        .context("Switch key package name is not valid Unicode")?;
    let package = if name.eq_ignore_ascii_case("prod.keys") {
        let metadata = fs::metadata(path)?;
        if metadata.len() == 0 || metadata.len() > SWITCH_KEYS_MAX_BYTES {
            bail!("prod.keys must be a non-empty text file smaller than 8 MiB");
        }
        SwitchKeyPackage {
            prod_keys: fs::read(path)?,
            title_keys: None,
        }
    } else if name.to_ascii_lowercase().ends_with(".zip") {
        let file = File::open(path)?;
        let mut archive = zip::ZipArchive::new(file).context("opening Switch keys ZIP")?;
        if archive.len() > 128 {
            bail!("Switch keys ZIP contains too many entries");
        }
        let mut prod_keys = None;
        let mut title_keys = None;
        for index in 0..archive.len() {
            let mut entry = archive.by_index(index)?;
            let relative = entry
                .enclosed_name()
                .context("Switch keys ZIP contains an unsafe path")?;
            if entry.is_dir() {
                continue;
            }
            let Some(file_name) = relative.file_name().and_then(|name| name.to_str()) else {
                continue;
            };
            let destination = if file_name.eq_ignore_ascii_case("prod.keys") {
                &mut prod_keys
            } else if file_name.eq_ignore_ascii_case("title.keys") {
                &mut title_keys
            } else {
                continue;
            };
            if destination.is_some() {
                bail!("Switch keys ZIP contains duplicate {file_name} entries");
            }
            if entry.size() == 0 || entry.size() > SWITCH_KEYS_MAX_BYTES {
                bail!("Switch keys ZIP contains an invalid {file_name} entry");
            }
            let mut bytes = Vec::with_capacity(entry.size() as usize);
            entry
                .by_ref()
                .take(SWITCH_KEYS_MAX_BYTES + 1)
                .read_to_end(&mut bytes)?;
            if bytes.len() as u64 > SWITCH_KEYS_MAX_BYTES {
                bail!("Switch keys ZIP contains an oversized {file_name} entry");
            }
            *destination = Some(bytes);
        }
        SwitchKeyPackage {
            prod_keys: prod_keys.context("Switch keys ZIP does not contain prod.keys")?,
            title_keys,
        }
    } else {
        bail!("choose a prod.keys file or a ZIP containing prod.keys");
    };

    validate_switch_prod_keys_bytes(&package.prod_keys)?;
    if let Some(title_keys) = package.title_keys.as_deref() {
        validate_switch_title_keys_bytes(title_keys)?;
    }
    Ok(package)
}

fn normalize_switch_key_package(selected_path: &Path) -> Result<(PathBuf, PathBuf)> {
    let package = read_switch_key_package(selected_path)?;
    let store = firmware_package_store()?;
    fs::create_dir_all(&store)?;
    let staging = store.join(format!(".switch-keys-input-{}", Uuid::new_v4()));
    fs::create_dir(&staging)?;
    let archive_path = staging.join(SWITCH_KEYS_PACKAGE);
    let normalized = (|| -> Result<()> {
        let file = File::create(&archive_path)?;
        let mut archive = zip::ZipWriter::new(file);
        let options = zip::write::SimpleFileOptions::default()
            .compression_method(zip::CompressionMethod::Deflated);
        archive.start_file("prod.keys", options)?;
        archive.write_all(&package.prod_keys)?;
        if let Some(title_keys) = package.title_keys.as_deref() {
            archive.start_file("title.keys", options)?;
            archive.write_all(title_keys)?;
        }
        archive.finish()?;
        Ok(())
    })();
    if let Err(error) = normalized {
        let _ = fs::remove_dir_all(&staging);
        return Err(error);
    }
    Ok((staging, archive_path))
}

fn validate_switch_prod_keys(path: &Path) -> Result<()> {
    let metadata = fs::metadata(path)?;
    if metadata.len() == 0 || metadata.len() > SWITCH_KEYS_MAX_BYTES {
        bail!("prod.keys must be a non-empty text file smaller than 8 MiB");
    }
    validate_switch_prod_keys_bytes(&fs::read(path)?)
}

fn validate_switch_prod_keys_bytes(bytes: &[u8]) -> Result<()> {
    let contents = std::str::from_utf8(bytes).context("prod.keys is not valid UTF-8 text")?;
    let mut valid_entries = 0usize;
    let mut has_header_key = false;
    let mut has_master_key = false;
    for line in contents.lines() {
        let line = line.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        let Some((name, value)) = line.split_once('=') else {
            bail!("prod.keys contains a malformed non-comment line");
        };
        let name = name.trim();
        let value = value.trim();
        if name.is_empty()
            || !name
                .bytes()
                .all(|byte| byte.is_ascii_lowercase() || byte.is_ascii_digit() || byte == b'_')
            || !(16..=SWITCH_KEY_VALUE_MAX_HEX_CHARS).contains(&value.len())
            || value.len() % 2 != 0
            || !value.bytes().all(|byte| byte.is_ascii_hexdigit())
        {
            bail!("prod.keys contains an invalid key name or hexadecimal value");
        }
        valid_entries += 1;
        has_header_key |= name == "header_key";
        has_master_key |= name.starts_with("master_key_");
    }
    if valid_entries < 8 || !has_header_key || !has_master_key {
        bail!("prod.keys does not contain the expected Switch production-key structure");
    }
    Ok(())
}

fn validate_switch_title_keys(path: &Path) -> Result<()> {
    let metadata = fs::metadata(path)?;
    if metadata.len() == 0 || metadata.len() > SWITCH_KEYS_MAX_BYTES {
        bail!("title.keys must be a non-empty text file smaller than 8 MiB");
    }
    validate_switch_title_keys_bytes(&fs::read(path)?)
}

fn validate_switch_title_keys_bytes(bytes: &[u8]) -> Result<()> {
    let contents = std::str::from_utf8(bytes).context("title.keys is not valid UTF-8 text")?;
    let mut valid_entries = 0usize;
    for line in contents.lines() {
        let line = line.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        let Some((rights_id, title_key)) = line.split_once('=') else {
            bail!("title.keys contains a malformed non-comment line");
        };
        let rights_id = rights_id.trim();
        let title_key = title_key.trim();
        if rights_id.len() != 32
            || title_key.len() != 32
            || !rights_id.bytes().all(|byte| byte.is_ascii_hexdigit())
            || !title_key.bytes().all(|byte| byte.is_ascii_hexdigit())
        {
            bail!("title.keys contains an invalid rights ID or title key");
        }
        valid_entries += 1;
    }
    if valid_entries == 0 {
        bail!("title.keys does not contain any title-key entries");
    }
    Ok(())
}

fn validate_switch_firmware_directory(path: &Path, runtime_kind: &str) -> Result<()> {
    if !path.is_dir() {
        bail!("Switch firmware directory does not exist");
    }
    let mut nca_count = 0usize;
    let mut total_nca_bytes = 0u64;
    for entry in fs::read_dir(path)? {
        let entry = entry?;
        let file_type = entry.file_type()?;
        let entry_path = entry.path();
        let Some(file_name) = entry_path.file_name().and_then(|name| name.to_str()) else {
            continue;
        };
        let lowercase_file_name = file_name.to_ascii_lowercase();
        let nca_path = if runtime_kind == "ryubing" {
            if !file_type.is_dir() || switch_nca_content_id(&lowercase_file_name).is_none() {
                continue;
            }
            entry_path.join("00")
        } else {
            if !file_type.is_file() || switch_nca_content_id(&lowercase_file_name).is_none() {
                continue;
            }
            entry_path
        };
        if !nca_path.is_file() {
            continue;
        }
        let size = nca_path.metadata()?.len();
        if size == 0 {
            continue;
        }
        nca_count += 1;
        total_nca_bytes = total_nca_bytes.saturating_add(size);
        if nca_count >= 10 && total_nca_bytes >= 1024 * 1024 {
            return Ok(());
        }
        if nca_count >= MAX_FIRMWARE_FILES {
            break;
        }
    }
    bail!("Switch firmware directory does not contain a recognizable installed NCA set")
}

fn migrate_legacy_ryubing_firmware_layout(target: &Path) -> Result<bool> {
    if validate_switch_firmware_directory(target, "ryubing").is_ok() || !target.is_dir() {
        return Ok(false);
    }
    let mut legacy_files = Vec::new();
    let mut content_ids = HashSet::new();
    let mut total_bytes = 0u64;
    for entry in fs::read_dir(target)? {
        let entry = entry?;
        if !entry.file_type()?.is_file() {
            return Ok(false);
        }
        let name = entry.file_name();
        let Some(name) = name.to_str() else {
            return Ok(false);
        };
        let lowercase_name = name.to_ascii_lowercase();
        let Some(content_id) = switch_nca_content_id(&lowercase_name) else {
            return Ok(false);
        };
        if !content_ids.insert(content_id.to_owned()) {
            return Ok(false);
        }
        let size = entry.metadata()?.len();
        if size == 0 {
            return Ok(false);
        }
        total_bytes = total_bytes.saturating_add(size);
        legacy_files.push((entry.path(), content_id.to_owned()));
    }
    if legacy_files.len() < 10 || total_bytes < 1024 * 1024 {
        return Ok(false);
    }

    let parent = target
        .parent()
        .context("Ryubing firmware directory has no parent")?;
    let directory_name = target
        .file_name()
        .and_then(|name| name.to_str())
        .context("Ryubing firmware directory name is not valid Unicode")?;
    let staging = parent.join(format!(
        ".{directory_name}.lunchbox-layout-{}",
        Uuid::new_v4()
    ));
    fs::create_dir(&staging)?;
    let staged = (|| -> Result<()> {
        for (source, content_id) in &legacy_files {
            let destination = staging.join(format!("{content_id}.nca/00"));
            fs::create_dir_all(
                destination
                    .parent()
                    .context("Ryubing firmware fragment has no parent")?,
            )?;
            fs::hard_link(source, &destination).with_context(|| {
                format!("preparing Ryubing firmware layout for {}", source.display())
            })?;
        }
        validate_switch_firmware_directory(&staging, "ryubing")
    })();
    if let Err(error) = staged {
        let _ = fs::remove_dir_all(&staging);
        return Err(error);
    }

    let backup = parent.join(format!(
        ".{directory_name}.lunchbox-flat-backup-{}",
        Uuid::new_v4()
    ));
    fs::rename(target, &backup).context("preserving legacy Ryubing firmware layout")?;
    if let Err(error) = fs::rename(&staging, target) {
        let _ = fs::remove_dir_all(&staging);
        let _ = fs::rename(&backup, target);
        return Err(error).context("activating Ryubing firmware layout");
    }
    fs::remove_dir_all(&backup).context("removing migrated Ryubing firmware layout")?;
    Ok(true)
}

fn switch_nca_content_id(file_name: &str) -> Option<&str> {
    let content_id = file_name
        .strip_suffix(".cnmt.nca")
        .or_else(|| file_name.strip_suffix(".nca"))?;
    (content_id.len() == 32 && content_id.bytes().all(|byte| byte.is_ascii_hexdigit()))
        .then_some(content_id)
}

#[cfg(test)]
fn validate_switch_firmware_zip(path: &Path) -> Result<()> {
    validate_switch_firmware_zip_with_progress(path, &mut |_, _| {})
}

fn validate_switch_firmware_zip_with_progress(
    path: &Path,
    progress: &mut impl FnMut(usize, usize),
) -> Result<()> {
    let file = File::open(path)?;
    let mut archive = zip::ZipArchive::new(file)
        .with_context(|| format!("reading Switch firmware ZIP {}", path.display()))?;
    if archive.len() > MAX_FIRMWARE_FILES {
        bail!("Switch firmware ZIP exceeds the file-count safety limit");
    }
    let mut nca_names = HashSet::new();
    let mut nca_count = 0usize;
    let mut total_nca_bytes = 0u64;
    let archive_len = archive.len();
    progress(0, archive_len.max(1));
    for index in 0..archive_len {
        let entry = archive.by_index(index)?;
        progress(index + 1, archive_len);
        let Some(enclosed) = entry.enclosed_name() else {
            bail!("Switch firmware ZIP contains an unsafe path");
        };
        if entry.is_dir()
            || !enclosed
                .extension()
                .and_then(|extension| extension.to_str())
                .is_some_and(|extension| extension.eq_ignore_ascii_case("nca"))
        {
            continue;
        }
        let file_name = enclosed
            .file_name()
            .and_then(|name| name.to_str())
            .context("Switch firmware ZIP contains a non-Unicode NCA filename")?
            .to_ascii_lowercase();
        if switch_nca_content_id(&file_name).is_none() || entry.size() == 0 {
            bail!("Switch firmware ZIP contains an invalid NCA entry");
        }
        if !nca_names.insert(file_name) {
            bail!("Switch firmware ZIP contains duplicate NCA filenames");
        }
        nca_count += 1;
        total_nca_bytes = total_nca_bytes
            .checked_add(entry.size())
            .context("Switch firmware ZIP declares an invalid total size")?;
        if total_nca_bytes > MAX_FIRMWARE_EXTRACTED_BYTES {
            bail!("Switch firmware ZIP exceeds the extracted-size safety limit");
        }
    }
    if nca_count < 10 || total_nca_bytes < 1024 * 1024 {
        bail!("selected ZIP is not a recognizable Nintendo Switch firmware dump");
    }
    Ok(())
}

pub fn sync_imported_with_progress(
    statuses: &[FirmwareStatus],
    mut progress: impl FnMut(String, u8),
) -> Result<String> {
    let packages = SettingsStore::open_default()?.firmware_packages()?;
    let mut last_progress = None;
    let synced = sync_matching_statuses_with_progress(statuses, &packages, |message, percent| {
        if last_progress != Some(percent) {
            last_progress = Some(percent);
            progress(message, percent);
        }
    })?;
    if synced == 0 {
        bail!("no imported firmware package is available to sync for this runtime");
    }
    Ok(format!(
        "Synced {synced} firmware requirement{} to the selected runtime.",
        if synced == 1 { "" } else { "s" }
    ))
}

pub fn acquire_and_sync(
    statuses: &[FirmwareStatus],
    mut progress: impl FnMut(String),
) -> Result<String> {
    let selected = statuses
        .iter()
        .filter(|status| status.target_strategy != "manual_import" && !status.imported)
        .min_by_key(|status| {
            (
                if status.required && !status.supports_hle_fallback {
                    0
                } else {
                    1
                },
                status.package_name.to_ascii_lowercase(),
                status.source_id.clone(),
            )
        })
        .context("this runtime has no downloadable firmware package to acquire")?;
    let download = match selected.source_transport.as_str() {
        "minerva" => acquire_minerva_package(selected, &mut progress)?,
        "direct" => acquire_direct_package(selected, &mut progress)?,
        "github_release" => acquire_github_package(selected, &mut progress)?,
        "manual" => bail!("this firmware requires a legally acquired manual dump"),
        transport => bail!("unsupported firmware source transport {transport}"),
    };
    import_and_sync(statuses, &download)
}

fn acquire_minerva_package(
    status: &FirmwareStatus,
    progress: &mut impl FnMut(String),
) -> Result<PathBuf> {
    if status.torrent_file.is_empty() || status.path_prefix.is_empty() {
        bail!("Minerva firmware source metadata is incomplete");
    }
    let minerva_database = crate::catalog::requested_minerva_database_path().context(
        "The Minerva database is unavailable. Add minerva.db before downloading firmware.",
    )?;
    let connection = crate::catalog::open_read_only(&minerva_database, "Minerva catalog")?;
    let torrent_url = connection
        .query_row(
            "SELECT torrent_url FROM minerva_torrents WHERE torrent_file=?1",
            [&status.torrent_file],
            |row| row.get::<_, String>(0),
        )
        .with_context(|| {
            format!(
                "Minerva does not contain the reviewed firmware torrent {}",
                status.torrent_file
            )
        })?;
    if !torrent_url.starts_with("https://") {
        bail!("Minerva firmware torrent URL is not HTTPS");
    }

    progress(format!(
        "Fetching torrent metadata for {}…",
        status.package_name
    ));
    let torrent_bytes = crate::game_details::torrent_bytes_for_url(&torrent_url)?;
    let torrent = Torrent::read_from_bytes(&torrent_bytes)
        .map_err(|error| anyhow::anyhow!("could not parse Minerva firmware torrent: {error}"))?;
    let info_hash = torrent.info_hash();
    let expected =
        normalized_torrent_path(&format!("{}{}", status.path_prefix, status.package_name));
    let matching = torrent_file_listing(&torrent)?
        .into_iter()
        .filter(|(_, path)| {
            let normalized = normalized_torrent_path(path);
            normalized == expected || normalized.ends_with(&format!("/{expected}"))
        })
        .collect::<Vec<_>>();
    let [(file_index, reviewed_path)] = matching.as_slice() else {
        bail!(
            "Minerva torrent must contain exactly one reviewed path ending in {}; found {}",
            expected,
            matching.len()
        );
    };

    let store = SettingsStore::open_default()?;
    let settings = store.load()?;
    if settings.torrent_library_directory.as_os_str().is_empty() {
        bail!("choose a native torrent library directory in Settings");
    }
    if settings
        .qbittorrent_container_torrent_library_directory
        .trim()
        .is_empty()
    {
        bail!("enter the qBittorrent torrent-library path in Settings");
    }
    let password = crate::settings::load_password()?
        .context("save the qBittorrent password in Settings before downloading firmware")?;
    let native_root = settings.torrent_library_directory.join("lunchbox-firmware");
    fs::create_dir_all(&native_root)?;
    let client_root = crate::qbittorrent::client_path_join(
        &settings.qbittorrent_container_torrent_library_directory,
        "lunchbox-firmware",
    );

    let requested = vec![(*file_index, reviewed_path.clone())];
    let game_jobs = store.jobs_for_info_hash(&info_hash)?;
    let mut selection = crate::qbittorrent::active_selection(&game_jobs, false, &requested)?;
    if let crate::qbittorrent::DownloadSelection::Exact(files) = &mut selection {
        files.extend(
            store
                .firmware_downloads_for_info_hash(&info_hash)?
                .into_iter()
                .filter(|receipt| {
                    matches!(
                        receipt.state.as_str(),
                        "queued" | "downloading" | "paused" | "complete"
                    )
                })
                .map(|receipt| (receipt.file_index, receipt.file_path)),
        );
        files.sort_by(|left, right| left.0.cmp(&right.0).then_with(|| left.1.cmp(&right.1)));
        files.dedup();
    }

    progress(format!("Starting {} in qBittorrent…", status.package_name));
    let client = crate::qbittorrent::QbittorrentClient::authenticated(&settings, &password)?;
    let actual_files = client.add(
        &torrent_bytes,
        &info_hash,
        &client_root,
        &selection,
        &requested,
    )?;
    let actual_path = actual_files
        .iter()
        .find(|(index, _)| index == file_index)
        .map(|(_, path)| path.clone())
        .context("qBittorrent did not return the reviewed firmware file")?;
    let local_path = native_root.join(crate::qbittorrent::safe_torrent_relative_path(
        &actual_path,
    )?);
    let mut receipt = FirmwareDownloadReceipt {
        source_id: status.source_id.clone(),
        package_name: status.package_name.clone(),
        torrent_url,
        info_hash: info_hash.clone(),
        file_index: *file_index,
        file_path: actual_path.clone(),
        local_path: local_path.to_string_lossy().into_owned(),
        state: "queued".into(),
        progress: 0.0,
        message: "Queued in qBittorrent".into(),
        updated_at: crate::settings::unix_timestamp(),
    };
    store.record_firmware_download(&receipt)?;

    let started = Instant::now();
    loop {
        if started.elapsed() > FIRMWARE_DOWNLOAD_TIMEOUT {
            receipt.state = "failed".into();
            receipt.message = "Firmware download timed out after one hour".into();
            receipt.updated_at = crate::settings::unix_timestamp();
            store.record_firmware_download(&receipt)?;
            bail!("firmware download timed out after one hour");
        }
        let selected_file = (*file_index, actual_path.clone());
        let snapshot = client.snapshot(&info_hash, Some(std::slice::from_ref(&selected_file)))?;
        receipt.progress = snapshot.progress;
        receipt.message = snapshot.message.clone();
        receipt.state = match snapshot.state.as_str() {
            "complete" => "complete",
            "paused" => "paused",
            "error" => "failed",
            _ => "downloading",
        }
        .into();
        receipt.updated_at = crate::settings::unix_timestamp();
        store.record_firmware_download(&receipt)?;
        progress(format!(
            "{} · {:.0}% · {}",
            status.package_name,
            snapshot.progress * 100.0,
            snapshot.message
        ));
        if snapshot.state == "error" {
            bail!("qBittorrent reported an error for {}", status.package_name);
        }
        if snapshot.state == "complete" {
            if !local_path.is_file() {
                bail!(
                    "qBittorrent completed the package but {} is not visible at the configured native path",
                    local_path.display()
                );
            }
            receipt.message = "Downloaded; importing into the firmware store".into();
            receipt.updated_at = crate::settings::unix_timestamp();
            store.record_firmware_download(&receipt)?;
            return Ok(local_path);
        }
        std::thread::sleep(Duration::from_secs(2));
    }
}

fn acquire_direct_package(
    status: &FirmwareStatus,
    progress: &mut impl FnMut(String),
) -> Result<PathBuf> {
    if !status.source_url.starts_with("https://") {
        bail!("official firmware download URL is missing or is not HTTPS");
    }
    let target = firmware_download_path(status)?;
    progress(format!(
        "Downloading {} from its official source…",
        status.package_name
    ));
    download_https(&status.source_url, &target, progress)?;
    Ok(target)
}

fn acquire_github_package(
    status: &FirmwareStatus,
    progress: &mut impl FnMut(String),
) -> Result<PathBuf> {
    if !status.source_url.starts_with("https://api.github.com/") {
        bail!("GitHub firmware release API URL is missing or invalid");
    }
    progress(format!(
        "Checking the latest release for {}…",
        status.package_name
    ));
    let release = fetch_github_release(&status.source_url)?;
    let mut matching = release
        .assets
        .into_iter()
        .filter(|asset| {
            let name = asset.name.to_ascii_lowercase();
            match status.source_id.as_str() {
                "github:86box-romset" => name.ends_with(".zip"),
                "github:pcem-romset" => name.ends_with(".zip") && name.contains("rom"),
                _ => false,
            }
        })
        .collect::<Vec<_>>();
    matching.sort_by(|left, right| left.name.cmp(&right.name));
    if matching.len() > 1 {
        bail!(
            "GitHub release exposes multiple possible ROM-set ZIP assets; refusing an ambiguous selection"
        );
    }
    let (url, label, size) = if let Some(asset) = matching.first() {
        (
            asset.browser_download_url.as_str(),
            asset.name.as_str(),
            asset.size,
        )
    } else {
        (
            release.zipball_url.as_str(),
            "reviewed release source archive",
            0,
        )
    };
    if !url.starts_with("https://") || size > MAX_FIRMWARE_ARCHIVE_BYTES {
        bail!("GitHub firmware asset exceeds the download safety limit");
    }
    let target = firmware_download_path(status)?;
    progress(format!("Downloading {} ({})…", status.package_name, label));
    download_https(url, &target, progress)?;
    Ok(target)
}

fn fetch_github_release(url: &str) -> Result<GitHubRelease> {
    let agent: ureq::Agent = ureq::Agent::config_builder()
        .timeout_connect(Some(Duration::from_secs(10)))
        .timeout_global(Some(Duration::from_secs(30)))
        .http_status_as_error(false)
        .user_agent("Lunchbox/0.1 firmware manager")
        .build()
        .into();
    let mut response = agent
        .get(url)
        .header("Accept", "application/vnd.github+json")
        .call()
        .with_context(|| format!("querying GitHub firmware release {url}"))?;
    if !response.status().is_success() {
        bail!(
            "GitHub firmware release request returned HTTP {}",
            response.status()
        );
    }
    let mut bytes = Vec::new();
    response
        .body_mut()
        .as_reader()
        .take(MAX_GITHUB_RELEASE_BYTES + 1)
        .read_to_end(&mut bytes)?;
    if bytes.len() as u64 > MAX_GITHUB_RELEASE_BYTES {
        bail!("GitHub firmware release metadata exceeds the safety limit");
    }
    serde_json::from_slice(&bytes).context("decoding GitHub firmware release metadata")
}

fn firmware_download_path(status: &FirmwareStatus) -> Result<PathBuf> {
    validate_package_filename(&status.package_name)?;
    let root = ProjectDirs::from("com", "Lunchbox", "Lunchbox")
        .map(|dirs| dirs.data_local_dir().join("firmware/downloads"))
        .context("could not determine the Lunchbox firmware download directory")?;
    let directory = root.join(store_component(&status.source_id));
    fs::create_dir_all(&directory)?;
    Ok(directory.join(&status.package_name))
}

fn download_https(url: &str, target: &Path, progress: &mut impl FnMut(String)) -> Result<()> {
    if !url.starts_with("https://") {
        bail!("firmware downloads require HTTPS");
    }
    let agent: ureq::Agent = ureq::Agent::config_builder()
        .timeout_connect(Some(Duration::from_secs(15)))
        .timeout_global(Some(FIRMWARE_DOWNLOAD_TIMEOUT))
        .http_status_as_error(false)
        .user_agent("Lunchbox/0.1 firmware manager")
        .build()
        .into();
    let mut response = agent
        .get(url)
        .call()
        .with_context(|| format!("downloading firmware from {url}"))?;
    if !response.status().is_success() {
        bail!("firmware download returned HTTP {}", response.status());
    }
    let expected = response
        .headers()
        .get(ureq::http::header::CONTENT_LENGTH)
        .and_then(|value| value.to_str().ok())
        .and_then(|value| value.parse::<u64>().ok());
    if expected.is_some_and(|size| size > MAX_FIRMWARE_ARCHIVE_BYTES) {
        bail!("firmware download exceeds the size safety limit");
    }
    let parent = target
        .parent()
        .context("firmware download target has no parent")?;
    fs::create_dir_all(parent)?;
    let temporary = parent.join(format!(
        ".{}.download-{}",
        target
            .file_name()
            .and_then(|name| name.to_str())
            .unwrap_or("firmware"),
        Uuid::new_v4()
    ));
    let result = (|| -> Result<()> {
        let mut output = File::create(&temporary)?;
        let mut reader = response.body_mut().as_reader();
        let mut buffer = [0u8; 128 * 1024];
        let mut downloaded = 0u64;
        let mut last_progress = Instant::now() - Duration::from_secs(1);
        loop {
            let read = reader.read(&mut buffer)?;
            if read == 0 {
                break;
            }
            downloaded = downloaded
                .checked_add(u64::try_from(read).unwrap_or(u64::MAX))
                .context("firmware download size overflow")?;
            if downloaded > MAX_FIRMWARE_ARCHIVE_BYTES {
                bail!("firmware download exceeds the size safety limit");
            }
            output.write_all(&buffer[..read])?;
            if last_progress.elapsed() >= Duration::from_millis(500) {
                let detail = expected
                    .filter(|size| *size > 0)
                    .map(|size| format!("{:.0}%", downloaded as f64 / size as f64 * 100.0))
                    .unwrap_or_else(|| crate::game_details::format_bytes(downloaded));
                progress(format!("Downloading firmware · {detail}"));
                last_progress = Instant::now();
            }
        }
        output.flush()?;
        output.sync_all()?;
        if let Some(expected) = expected
            && downloaded != expected
        {
            bail!("firmware download ended at {downloaded} bytes; expected {expected}");
        }
        atomic_activate_file(&temporary, target)
    })();
    if result.is_err() {
        let _ = fs::remove_file(&temporary);
    }
    result
}

fn torrent_file_listing(torrent: &Torrent) -> Result<Vec<(u32, String)>> {
    if let Some(files) = torrent.files.as_ref() {
        files
            .iter()
            .enumerate()
            .map(|(index, file)| {
                Ok((
                    u32::try_from(index).context("torrent file index is too large")?,
                    file.path
                        .components()
                        .map(|component| component.as_os_str().to_string_lossy())
                        .collect::<Vec<_>>()
                        .join("/"),
                ))
            })
            .collect()
    } else {
        Ok(vec![(0, torrent.name.clone())])
    }
}

fn normalized_torrent_path(value: &str) -> String {
    value
        .trim_start_matches("./")
        .replace('\\', "/")
        .to_ascii_lowercase()
}

fn import_package_with_progress(
    source_id: &str,
    package_name: &str,
    selected_path: &Path,
    mut progress: impl FnMut(String, u8),
) -> Result<FirmwarePackageReceipt> {
    validate_package_filename(package_name)?;
    let store = SettingsStore::open_default()?;
    let metadata = fs::metadata(selected_path)
        .with_context(|| format!("reading firmware package {}", selected_path.display()))?;
    if metadata.len() > MAX_FIRMWARE_ARCHIVE_BYTES {
        bail!(
            "firmware package is larger than the {} GiB safety limit",
            MAX_FIRMWARE_ARCHIVE_BYTES / 1024 / 1024 / 1024
        );
    }

    let store_root = firmware_package_store()?;
    fs::create_dir_all(&store_root)
        .with_context(|| format!("creating firmware store {}", store_root.display()))?;
    let package_root = store_root
        .join(store_component(source_id))
        .join(store_component(package_name));
    let package_parent = package_root
        .parent()
        .context("firmware package store path has no parent")?;
    fs::create_dir_all(package_parent)?;
    let staging = package_parent.join(format!(
        ".{}.staging-{}",
        store_component(package_name),
        Uuid::new_v4()
    ));
    let staged_archive = staging.join(package_name);
    let staged_extracted = staging.join("extracted");

    let staged = (|| -> Result<(String, Vec<FirmwareFileReceipt>)> {
        progress("Copying firmware into the managed store…".into(), 0);
        fs::create_dir_all(&staged_extracted)?;
        fs::copy(selected_path, &staged_archive).with_context(|| {
            format!(
                "copying firmware package {} to {}",
                selected_path.display(),
                staged_archive.display()
            )
        })?;
        if package_name.to_ascii_lowercase().ends_with(".zip") {
            extract_zip_archive_with_progress(
                &staged_archive,
                &staged_extracted,
                &mut |done, total| {
                    progress(
                        format!("Extracting firmware · {done}/{total} files"),
                        progress_between(5, 55, done, total),
                    );
                },
            )?;
        } else {
            fs::copy(&staged_archive, staged_extracted.join(package_name))?;
        }
        progress("Hashing the firmware archive…".into(), 58);
        let archive_sha256 = sha256_file(&staged_archive)?;
        let files = build_file_manifest_with_progress(
            source_id,
            package_name,
            &staged_extracted,
            &package_root.join("extracted"),
            &mut |done, total| {
                progress(
                    format!("Verifying extracted firmware · {done}/{total} files"),
                    progress_between(60, 96, done, total),
                );
            },
        )?;
        if files.is_empty() {
            bail!("firmware package contains no files");
        }
        Ok((archive_sha256, files))
    })();
    let (archive_sha256, files) = match staged {
        Ok(value) => value,
        Err(error) => {
            let _ = fs::remove_dir_all(&staging);
            return Err(error);
        }
    };

    let receipt = FirmwarePackageReceipt {
        source_id: source_id.to_owned(),
        package_name: package_name.to_owned(),
        archive_path: package_root
            .join(package_name)
            .to_string_lossy()
            .into_owned(),
        extracted_root: package_root
            .join("extracted")
            .to_string_lossy()
            .into_owned(),
        sha256: archive_sha256,
        file_count: i64::try_from(files.len()).expect("firmware safety limit fits in i64"),
        imported_at: crate::settings::unix_timestamp(),
    };
    let backup = activate_package_directory(&store_root, &staging, &package_root)?;
    if let Err(error) = store.record_firmware_package(&receipt, &files) {
        let _ = fs::remove_dir_all(&package_root);
        if let Some(backup) = backup.as_ref() {
            let _ = fs::rename(backup, &package_root);
        }
        return Err(error).context("recording firmware package provenance");
    }
    if let Some(backup) = backup {
        let _ = fs::remove_dir_all(backup);
    }
    progress("Firmware package stored and verified.".into(), 100);
    Ok(receipt)
}

fn sync_matching_statuses_with_progress(
    statuses: &[FirmwareStatus],
    packages: &[FirmwarePackageReceipt],
    mut progress: impl FnMut(String, u8),
) -> Result<usize> {
    let package_map = packages
        .iter()
        .map(|package| {
            (
                (package.source_id.as_str(), package.package_name.as_str()),
                package,
            )
        })
        .collect::<HashMap<_, _>>();
    let store = SettingsStore::open_default()?;
    let mut synced = 0usize;
    let matching_statuses = statuses
        .iter()
        .filter(|status| status.target_strategy != "manual_import")
        .filter(|status| {
            package_map.contains_key(&(status.source_id.as_str(), status.package_name.as_str()))
        })
        .collect::<Vec<_>>();
    let status_count = matching_statuses.len();
    for (index, status) in matching_statuses.into_iter().enumerate() {
        let Some(package) =
            package_map.get(&(status.source_id.as_str(), status.package_name.as_str()))
        else {
            continue;
        };
        let phase_start = progress_between(0, 100, index, status_count.max(1));
        let phase_end = progress_between(0, 100, index + 1, status_count.max(1));
        if sync_status_with_progress(status, package, &store, |message, percent| {
            progress(
                message,
                progress_between(phase_start, phase_end, usize::from(percent), 100),
            );
        })? {
            synced += 1;
        }
    }
    progress(
        "Firmware has been applied to the emulator profile.".into(),
        100,
    );
    Ok(synced)
}

fn sync_status_with_progress(
    status: &FirmwareStatus,
    package: &FirmwarePackageReceipt,
    store: &SettingsStore,
    mut progress: impl FnMut(String, u8),
) -> Result<bool> {
    if status.runtime_path.is_empty() || status.target_path.is_empty() {
        return Ok(false);
    }
    progress(format!("Verifying {}…", status.package_name), 0);
    verify_package_receipt_with_progress(package, store, &mut |done, total| {
        progress(
            format!("Verifying {} · {done}/{total} files", status.package_name),
            progress_between(0, 58, done, total),
        );
    })?;
    let target = PathBuf::from(&status.target_path);
    progress(
        format!("Applying {} to the emulator profile…", status.package_name),
        60,
    );
    match status.install_mode.as_str() {
        "merge_tree" if status.source_id == "manual:nintendo-switch-keys" => {
            copy_switch_key_package(Path::new(&package.extracted_root), &target)?;
        }
        "copy_archive" => atomic_copy(Path::new(&package.archive_path), &target)?,
        "merge_tree" if status.source_id == "manual:nintendo-switch-firmware" => {
            copy_switch_firmware_tree_with_progress(
                Path::new(&package.extracted_root),
                &target,
                &status.runtime_kind,
                &mut |done, total| {
                    progress(
                        format!("Installing Switch firmware · {done}/{total} archives"),
                        progress_between(60, 98, done, total),
                    );
                },
            )?;
        }
        "merge_tree" => {
            let source = package_sync_root(Path::new(&package.extracted_root), &status.source_id)?;
            if status.runtime_kind == "openmsx" {
                copy_tree(&source, &target, true)?;
            } else {
                copy_tree(&source, &target, false)?;
            }
        }
        mode => bail!("unsupported firmware install mode {mode}"),
    }
    store.record_firmware_install(&FirmwareInstallReceipt {
        rule_key: status.rule_key.clone(),
        runtime_path: status.runtime_path.clone(),
        target_path: status.target_path.clone(),
        source_id: status.source_id.clone(),
        package_name: status.package_name.clone(),
        synced_at: crate::settings::unix_timestamp(),
    })?;
    progress(format!("Applied {}.", status.package_name), 100);
    Ok(true)
}

fn copy_switch_key_package(source: &Path, target: &Path) -> Result<()> {
    let prod_keys = source.join("prod.keys");
    validate_switch_prod_keys(&prod_keys)
        .context("imported Switch keys package no longer contains valid prod.keys")?;
    fs::create_dir_all(target)?;
    atomic_copy(&prod_keys, &target.join("prod.keys"))?;

    let title_keys = source.join("title.keys");
    if title_keys.is_file() {
        validate_switch_title_keys(&title_keys)
            .context("imported Switch keys package contains invalid title.keys")?;
        atomic_copy(&title_keys, &target.join("title.keys"))?;
    }
    Ok(())
}

#[cfg(test)]
fn copy_switch_firmware_tree(source: &Path, target: &Path, runtime_kind: &str) -> Result<()> {
    copy_switch_firmware_tree_with_progress(source, target, runtime_kind, &mut |_, _| {})
}

fn copy_switch_firmware_tree_with_progress(
    source: &Path,
    target: &Path,
    runtime_kind: &str,
    progress: &mut impl FnMut(usize, usize),
) -> Result<()> {
    let parent = target
        .parent()
        .context("Switch firmware target has no parent directory")?;
    let directory_name = target
        .file_name()
        .and_then(|name| name.to_str())
        .context("Switch firmware target is not valid Unicode")?;
    fs::create_dir_all(parent)?;
    if fs::symlink_metadata(target).is_ok_and(|metadata| metadata.file_type().is_symlink()) {
        bail!("refusing to replace a symbolic-link Switch firmware directory");
    }
    let staging = parent.join(format!(
        ".{directory_name}.lunchbox-staging-{}",
        Uuid::new_v4()
    ));
    fs::create_dir(&staging)?;
    let nca_files = WalkDir::new(source)
        .follow_links(false)
        .into_iter()
        .filter_map(|entry| match entry {
            Ok(entry)
                if entry.file_type().is_file()
                    && entry
                        .path()
                        .extension()
                        .and_then(|extension| extension.to_str())
                        .is_some_and(|extension| extension.eq_ignore_ascii_case("nca")) =>
            {
                Some(Ok(entry.into_path()))
            }
            Ok(_) => None,
            Err(error) => Some(Err(error)),
        })
        .collect::<std::result::Result<Vec<_>, _>>()?;
    let nca_count = nca_files.len();
    progress(0, nca_count.max(1));
    let mut copied_names = HashSet::new();
    let staged = (|| -> Result<()> {
        for (index, entry) in nca_files.iter().enumerate() {
            let name = entry
                .file_name()
                .and_then(|name| name.to_str())
                .context("Switch firmware contains a non-Unicode NCA filename")?;
            if !copied_names.insert(name.to_ascii_lowercase()) {
                bail!("Switch firmware contains duplicate NCA filenames");
            }
            let destination = if runtime_kind == "ryubing" {
                let lowercase_name = name.to_ascii_lowercase();
                let content_id = switch_nca_content_id(&lowercase_name)
                    .context("Switch firmware contains an invalid NCA content ID")?;
                staging.join(format!("{content_id}.nca/00"))
            } else {
                staging.join(name)
            };
            atomic_copy(entry, &destination)?;
            progress(index + 1, nca_count);
        }
        if nca_count < 10 {
            bail!("Switch firmware package no longer contains the reviewed NCA set");
        }
        Ok(())
    })();
    if let Err(error) = staged {
        let _ = fs::remove_dir_all(&staging);
        return Err(error);
    }

    let backup = parent.join(format!(".{directory_name}.previous-{}", Uuid::new_v4()));
    let had_previous = target.exists();
    if had_previous && let Err(error) = fs::rename(target, &backup) {
        let _ = fs::remove_dir_all(&staging);
        return Err(error).context("preserving the previous Switch firmware directory");
    }
    if let Err(error) = fs::rename(&staging, target) {
        let _ = fs::remove_dir_all(&staging);
        if had_previous {
            let _ = fs::rename(&backup, target);
        }
        return Err(error).context("activating the imported Switch firmware directory");
    }
    if had_previous {
        fs::remove_dir_all(&backup).context("removing the replaced Switch firmware directory")?;
    }
    Ok(())
}

fn firmware_package_store() -> Result<PathBuf> {
    ProjectDirs::from("com", "Lunchbox", "Lunchbox")
        .map(|dirs| dirs.data_local_dir().join("firmware/packages"))
        .context("could not determine the Lunchbox firmware package store")
}

fn validate_package_filename(package_name: &str) -> Result<()> {
    let mut components = Path::new(package_name).components();
    if package_name.trim().is_empty()
        || !matches!(components.next(), Some(std::path::Component::Normal(_)))
        || components.next().is_some()
    {
        bail!("firmware package identity is not a safe filename");
    }
    Ok(())
}

fn store_component(value: &str) -> String {
    let safe = safe_component(value);
    let digest = hex::encode(Sha256::digest(value.as_bytes()));
    format!("{}-{}", safe.trim_matches('_'), &digest[..12])
}

fn activate_package_directory(
    store_root: &Path,
    staging: &Path,
    target: &Path,
) -> Result<Option<PathBuf>> {
    if !staging.starts_with(store_root)
        || !target.starts_with(store_root)
        || staging == store_root
        || target == store_root
    {
        bail!("refusing to replace a firmware directory outside the package store");
    }
    let backup = target.with_file_name(format!(
        ".{}.previous-{}",
        target
            .file_name()
            .and_then(|name| name.to_str())
            .unwrap_or("firmware"),
        Uuid::new_v4()
    ));
    let had_previous = target.exists();
    if had_previous {
        fs::rename(target, &backup).with_context(|| {
            format!("preserving previous firmware package {}", target.display())
        })?;
    }
    if let Err(error) = fs::rename(staging, target) {
        if backup.exists() {
            let _ = fs::rename(&backup, target);
        }
        return Err(error)
            .with_context(|| format!("activating firmware package {}", target.display()));
    }
    Ok(had_previous.then_some(backup))
}

#[cfg(test)]
fn extract_zip_archive(archive_path: &Path, destination: &Path) -> Result<()> {
    extract_zip_archive_with_progress(archive_path, destination, &mut |_, _| {})
}

fn extract_zip_archive_with_progress(
    archive_path: &Path,
    destination: &Path,
    progress: &mut impl FnMut(usize, usize),
) -> Result<()> {
    let file = File::open(archive_path)?;
    let mut archive = zip::ZipArchive::new(file)
        .with_context(|| format!("reading firmware ZIP {}", archive_path.display()))?;
    if archive.len() > MAX_FIRMWARE_FILES {
        bail!("firmware ZIP exceeds the {MAX_FIRMWARE_FILES}-file safety limit");
    }
    let mut extracted_bytes = 0u64;
    let mut paths = HashSet::new();
    let archive_len = archive.len();
    progress(0, archive_len.max(1));
    for index in 0..archive_len {
        let mut entry = archive.by_index(index)?;
        progress(index + 1, archive_len);
        let enclosed = entry
            .enclosed_name()
            .context("firmware ZIP contains an unsafe path")?
            .to_path_buf();
        if enclosed.as_os_str().is_empty() || !paths.insert(enclosed.clone()) {
            bail!("firmware ZIP contains an empty or duplicate path");
        }
        if entry
            .unix_mode()
            .is_some_and(|mode| mode & 0o170000 == 0o120000)
        {
            bail!("firmware ZIP contains a symbolic link");
        }
        let output = destination.join(enclosed);
        if entry.is_dir() {
            fs::create_dir_all(&output)?;
            continue;
        }
        extracted_bytes = extracted_bytes
            .checked_add(entry.size())
            .context("firmware ZIP extracted size overflow")?;
        if extracted_bytes > MAX_FIRMWARE_EXTRACTED_BYTES {
            bail!("firmware ZIP exceeds the extracted-size safety limit");
        }
        if let Some(parent) = output.parent() {
            fs::create_dir_all(parent)?;
        }
        let mut destination_file = File::create(&output)?;
        let remaining = MAX_FIRMWARE_EXTRACTED_BYTES - (extracted_bytes - entry.size());
        let copied = std::io::copy(
            &mut entry.by_ref().take(remaining + 1),
            &mut destination_file,
        )?;
        if copied > remaining {
            bail!("firmware ZIP exceeded the extracted-size safety limit while reading");
        }
        destination_file.flush()?;
    }
    Ok(())
}

#[cfg(test)]
fn build_file_manifest(
    source_id: &str,
    package_name: &str,
    staged_root: &Path,
    final_root: &Path,
) -> Result<Vec<FirmwareFileReceipt>> {
    build_file_manifest_with_progress(
        source_id,
        package_name,
        staged_root,
        final_root,
        &mut |_, _| {},
    )
}

fn build_file_manifest_with_progress(
    source_id: &str,
    package_name: &str,
    staged_root: &Path,
    final_root: &Path,
    progress: &mut impl FnMut(usize, usize),
) -> Result<Vec<FirmwareFileReceipt>> {
    let mut files = Vec::new();
    let mut bytes = 0u64;
    let entries = WalkDir::new(staged_root)
        .follow_links(false)
        .into_iter()
        .filter_map(|entry| match entry {
            Ok(entry) if entry.file_type().is_file() => Some(Ok(entry)),
            Ok(_) => None,
            Err(error) => Some(Err(error)),
        })
        .collect::<std::result::Result<Vec<_>, _>>()?;
    let entry_count = entries.len();
    progress(0, entry_count.max(1));
    for (index, entry) in entries.into_iter().enumerate() {
        if files.len() >= MAX_FIRMWARE_FILES {
            bail!("firmware package exceeds the {MAX_FIRMWARE_FILES}-file safety limit");
        }
        let relative = entry.path().strip_prefix(staged_root)?;
        let relative_text = relative
            .components()
            .map(|component| component.as_os_str().to_string_lossy())
            .collect::<Vec<_>>()
            .join("/");
        let size = entry.metadata()?.len();
        bytes = bytes
            .checked_add(size)
            .context("firmware package size overflow")?;
        if bytes > MAX_FIRMWARE_EXTRACTED_BYTES {
            bail!("firmware package exceeds the extracted-size safety limit");
        }
        files.push(FirmwareFileReceipt {
            source_id: source_id.to_owned(),
            package_name: package_name.to_owned(),
            relative_path: relative_text,
            store_path: final_root.join(relative).to_string_lossy().into_owned(),
            sha256: sha256_file(entry.path())?,
            file_size: i64::try_from(size).unwrap_or(i64::MAX),
        });
        progress(index + 1, entry_count);
    }
    files.sort_by(|left, right| left.relative_path.cmp(&right.relative_path));
    Ok(files)
}

fn package_receipt_files_exist(package: &FirmwarePackageReceipt) -> bool {
    Path::new(&package.archive_path).is_file() && Path::new(&package.extracted_root).is_dir()
}

#[cfg(test)]
fn verify_package_receipt(package: &FirmwarePackageReceipt, store: &SettingsStore) -> Result<()> {
    verify_package_receipt_with_progress(package, store, &mut |_, _| {})
}

fn verify_package_receipt_with_progress(
    package: &FirmwarePackageReceipt,
    store: &SettingsStore,
    progress: &mut impl FnMut(usize, usize),
) -> Result<()> {
    let archive = Path::new(&package.archive_path);
    let extracted_root = Path::new(&package.extracted_root);
    if !archive.is_file() || !extracted_root.is_dir() {
        bail!("imported firmware package files are missing; import the package again");
    }
    if sha256_file(archive)? != package.sha256 {
        bail!("imported firmware package failed its SHA-256 integrity check");
    }
    let files = store.firmware_files(&package.source_id, &package.package_name)?;
    if usize::try_from(package.file_count).ok() != Some(files.len()) {
        bail!("imported firmware package manifest is incomplete; import the package again");
    }
    let file_count = files.len();
    progress(0, file_count.max(1));
    for (index, file) in files.into_iter().enumerate() {
        let expected_path = extracted_root.join(Path::new(&file.relative_path));
        let actual_path = Path::new(&file.store_path);
        if actual_path != expected_path || !actual_path.is_file() {
            bail!(
                "imported firmware file {} is missing or outside its package store",
                file.relative_path
            );
        }
        let expected_size = u64::try_from(file.file_size)
            .context("firmware manifest contains a negative file size")?;
        if actual_path.metadata()?.len() != expected_size
            || sha256_file(actual_path)? != file.sha256
        {
            bail!(
                "imported firmware file {} failed its integrity check",
                file.relative_path
            );
        }
        progress(index + 1, file_count);
    }
    Ok(())
}

fn sha256_file(path: &Path) -> Result<String> {
    let mut file =
        File::open(path).with_context(|| format!("opening firmware file {}", path.display()))?;
    let mut digest = Sha256::new();
    let mut buffer = [0u8; 128 * 1024];
    loop {
        let read = file.read(&mut buffer)?;
        if read == 0 {
            break;
        }
        digest.update(&buffer[..read]);
    }
    Ok(hex::encode(digest.finalize()))
}

fn package_sync_root(extracted_root: &Path, source_id: &str) -> Result<PathBuf> {
    if !matches!(source_id, "github:pcem-romset" | "github:86box-romset") {
        return Ok(extracted_root.to_path_buf());
    }
    let children = fs::read_dir(extracted_root)?.collect::<std::io::Result<Vec<_>>>()?;
    if children.len() == 1 && children[0].path().is_dir() {
        Ok(children[0].path())
    } else {
        Ok(extracted_root.to_path_buf())
    }
}

fn copy_tree(source: &Path, target: &Path, openmsx_filter: bool) -> Result<()> {
    for entry in WalkDir::new(source).follow_links(false) {
        let entry = entry?;
        if !entry.file_type().is_file() {
            continue;
        }
        if openmsx_filter
            && entry
                .path()
                .extension()
                .and_then(|extension| extension.to_str())
                .is_some_and(|extension| {
                    matches!(
                        extension.to_ascii_lowercase().as_str(),
                        "ini" | "xml" | "txt"
                    )
                })
        {
            continue;
        }
        let relative = entry.path().strip_prefix(source)?;
        atomic_copy(entry.path(), &target.join(relative))?;
    }
    Ok(())
}

fn atomic_copy(source: &Path, target: &Path) -> Result<()> {
    let parent = target
        .parent()
        .context("firmware target file has no parent directory")?;
    fs::create_dir_all(parent)?;
    let file_name = target
        .file_name()
        .and_then(|name| name.to_str())
        .context("firmware target filename is not valid Unicode")?;
    let temporary = parent.join(format!(".{file_name}.lunchbox-{}", Uuid::new_v4()));
    if let Err(error) = fs::copy(source, &temporary) {
        let _ = fs::remove_file(&temporary);
        return Err(error).with_context(|| {
            format!(
                "copying firmware {} to {}",
                source.display(),
                target.display()
            )
        });
    }
    atomic_activate_file(&temporary, target)
}

fn atomic_activate_file(temporary: &Path, target: &Path) -> Result<()> {
    let parent = target
        .parent()
        .context("firmware target file has no parent directory")?;
    let file_name = target
        .file_name()
        .and_then(|name| name.to_str())
        .context("firmware target filename is not valid Unicode")?;
    let backup = parent.join(format!(".{file_name}.previous-{}", Uuid::new_v4()));
    let had_previous = target.exists();
    if had_previous && let Err(error) = fs::rename(target, &backup) {
        let _ = fs::remove_file(temporary);
        return Err(error)
            .with_context(|| format!("preserving existing firmware file {}", target.display()));
    }
    if let Err(error) = fs::rename(temporary, target) {
        let _ = fs::remove_file(temporary);
        if had_previous {
            let _ = fs::rename(&backup, target);
        }
        return Err(error).with_context(|| format!("activating firmware {}", target.display()));
    }
    if had_previous {
        let _ = fs::remove_file(backup);
    }
    Ok(())
}

fn open_path(path: &Path) -> Result<()> {
    let (program, argument) = if cfg!(target_os = "windows") {
        ("explorer", path.as_os_str())
    } else if cfg!(target_os = "macos") {
        ("open", path.as_os_str())
    } else {
        ("xdg-open", path.as_os_str())
    };
    host_command(program)
        .arg(argument)
        .spawn()
        .with_context(|| format!("opening firmware directory {}", path.display()))?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn write_zip(path: &Path, entries: &[(&str, &[u8])]) {
        let file = File::create(path).unwrap();
        let mut archive = zip::ZipWriter::new(file);
        for (name, contents) in entries {
            archive
                .start_file(*name, zip::write::SimpleFileOptions::default())
                .unwrap();
            archive.write_all(contents).unwrap();
        }
        archive.finish().unwrap();
    }

    #[test]
    fn firmware_summary_distinguishes_missing_optional_and_ready() {
        let status = |required, hle, imported, synced| FirmwareStatus {
            rule_key: "rule".into(),
            source_id: "source".into(),
            source_transport: "minerva".into(),
            source_url: String::new(),
            torrent_file: "firmware.torrent".into(),
            path_prefix: "firmware/".into(),
            package_name: "bios.zip".into(),
            runtime_kind: "retroarch".into(),
            install_mode: "merge_tree".into(),
            required,
            supports_hle_fallback: hle,
            target_strategy: "runtime_dir".into(),
            imported,
            synced,
            runtime_path: "/tmp/firmware".into(),
            target_path: "/tmp/firmware".into(),
            notes: "notes".into(),
        };
        assert!(summarize(&[status(true, false, false, false)]).contains("required"));
        assert!(summarize(&[status(false, true, false, false)]).contains("optional"));
        assert!(summarize(&[status(true, false, true, true)]).contains("ready"));
        assert!(status(true, false, false, false).needs_action());
        assert!(!status(false, true, false, false).needs_action());
        assert!(!status(true, false, true, true).needs_action());
        assert!(status(true, false, true, false).can_sync());
        assert!(!status(true, false, true, true).can_sync());
    }

    #[test]
    fn manual_paths_are_cross_platform_components() {
        assert_eq!(safe_component("Sony PlayStation 2+"), "Sony_PlayStation_2_");
        assert_ne!(store_component("source:a"), store_component("source/a"));
        assert!(validate_package_filename("../bios.zip").is_err());
        assert!(validate_package_filename("bios.zip").is_ok());
    }

    #[test]
    fn switch_default_profiles_match_each_runtime_on_all_desktop_hosts() {
        let native = EmulatorExecutable::Native(PathBuf::from("/opt/emulators/runtime"));
        let rom = Path::new("/games/Switch/game.xci");
        let home = Path::new("/users/player");
        let roaming = Path::new("/users/player/AppData/Roaming");
        let config = Path::new("/users/player/.config");
        let xdg_data = Path::new("/users/player/.local/share");

        assert_eq!(
            switch_runtime_root(
                FirmwareHost::Windows,
                "ryubing",
                &native,
                rom,
                home,
                roaming,
                xdg_data,
            )
            .unwrap(),
            roaming.join("Ryujinx")
        );
        assert_eq!(
            switch_runtime_root(
                FirmwareHost::MacOs,
                "ryubing",
                &native,
                rom,
                home,
                config,
                xdg_data,
            )
            .unwrap(),
            home.join("Library/Application Support/Ryujinx")
        );
        assert_eq!(
            switch_runtime_root(
                FirmwareHost::Linux,
                "ryubing",
                &native,
                rom,
                home,
                config,
                xdg_data,
            )
            .unwrap(),
            config.join("Ryujinx")
        );
        for host in [FirmwareHost::Linux, FirmwareHost::MacOs] {
            assert_eq!(
                switch_runtime_root(host, "eden", &native, rom, home, config, xdg_data,).unwrap(),
                xdg_data.join("eden")
            );
            assert_eq!(
                switch_runtime_root(host, "torzu", &native, rom, home, config, xdg_data,).unwrap(),
                xdg_data.join("yuzu")
            );
        }
        assert_eq!(
            switch_runtime_root(
                FirmwareHost::Windows,
                "eden",
                &native,
                rom,
                home,
                roaming,
                xdg_data,
            )
            .unwrap(),
            roaming.join("eden")
        );
        assert_eq!(
            switch_runtime_root(
                FirmwareHost::Windows,
                "torzu",
                &native,
                rom,
                home,
                roaming,
                xdg_data,
            )
            .unwrap(),
            roaming.join("yuzu")
        );
    }

    #[test]
    fn switch_profiles_honor_flatpak_and_runtime_portable_directories() {
        let temporary = tempfile::tempdir().unwrap();
        let home = temporary.path().join("home");
        let executable_directory = temporary.path().join("emulator");
        let executable = executable_directory.join("Ryujinx");
        let rom = temporary.path().join("games/game.xci");
        fs::create_dir_all(executable_directory.join("portable")).unwrap();
        fs::create_dir_all(rom.parent().unwrap().join("user")).unwrap();

        assert_eq!(
            switch_runtime_root(
                FirmwareHost::Linux,
                "ryubing",
                &EmulatorExecutable::Native(executable),
                &rom,
                &home,
                &home.join(".config"),
                &home.join(".local/share"),
            )
            .unwrap(),
            executable_directory.join("portable")
        );
        assert_eq!(
            switch_runtime_root(
                FirmwareHost::MacOs,
                "eden",
                &EmulatorExecutable::Native(executable_directory.join("Eden")),
                &rom,
                &home,
                &home.join(".config"),
                &home.join(".local/share"),
            )
            .unwrap(),
            rom.parent().unwrap().join("user")
        );
        assert_eq!(
            switch_runtime_root(
                FirmwareHost::Linux,
                "ryubing",
                &EmulatorExecutable::Flatpak {
                    command: PathBuf::from("flatpak"),
                    app_id: "io.github.ryubing.Ryujinx".into(),
                },
                &rom,
                &home,
                &home.join(".config"),
                &home.join(".local/share"),
            )
            .unwrap(),
            home.join(".var/app/io.github.ryubing.Ryujinx/config/Ryujinx")
        );
    }

    #[test]
    fn firmware_zip_extraction_is_bounded_and_manifested() {
        let temporary = tempfile::tempdir().unwrap();
        let archive = temporary.path().join("bios.zip");
        let extracted = temporary.path().join("extracted");
        fs::create_dir(&extracted).unwrap();
        write_zip(
            &archive,
            &[("system/bios.bin", b"bios"), ("font.rom", b"font")],
        );

        extract_zip_archive(&archive, &extracted).unwrap();
        let manifest = build_file_manifest(
            "source",
            "bios.zip",
            &extracted,
            Path::new("/final/extracted"),
        )
        .unwrap();
        assert_eq!(manifest.len(), 2);
        assert_eq!(manifest[1].relative_path, "system/bios.bin");
        assert_eq!(manifest[1].sha256.len(), 64);
    }

    #[test]
    fn firmware_zip_rejects_path_traversal() {
        let temporary = tempfile::tempdir().unwrap();
        let archive = temporary.path().join("unsafe.zip");
        let extracted = temporary.path().join("extracted");
        fs::create_dir(&extracted).unwrap();
        write_zip(&archive, &[("../escape.bin", b"escape")]);

        assert!(extract_zip_archive(&archive, &extracted).is_err());
        assert!(!temporary.path().join("escape.bin").exists());
    }

    #[test]
    fn switch_user_material_requires_structured_keys_and_recognizable_ncas() {
        let temporary = tempfile::tempdir().unwrap();
        let keys = temporary.path().join("prod.keys");
        let mut key_lines = vec![format!("header_key = {}", "a1".repeat(16))];
        for index in 0..7 {
            key_lines.push(format!("master_key_{index:02x} = {}", "b2".repeat(16)));
        }
        key_lines.push(format!("eticket_rsa_keypair = {}", "c3".repeat(528)));
        key_lines.push(format!("ssl_rsa_key = {}", "d4".repeat(256)));
        fs::write(&keys, key_lines.join("\n")).unwrap();
        validate_switch_prod_keys(&keys).unwrap();
        fs::write(&keys, "not a production key file").unwrap();
        assert!(validate_switch_prod_keys(&keys).is_err());

        let title_keys = temporary.path().join("title.keys");
        fs::write(
            &title_keys,
            format!("{} = {}", "12".repeat(16), "ab".repeat(16)),
        )
        .unwrap();
        validate_switch_title_keys(&title_keys).unwrap();
        fs::write(&title_keys, "not a title key file").unwrap();
        assert!(validate_switch_title_keys(&title_keys).is_err());

        let firmware = temporary.path().join("my-console-firmware.zip");
        let file = File::create(&firmware).unwrap();
        let mut archive = zip::ZipWriter::new(file);
        for index in 0..10 {
            let suffix = if index % 2 == 0 { ".nca" } else { ".cnmt.nca" };
            archive
                .start_file(
                    format!("dump/{index:032x}{suffix}"),
                    zip::write::SimpleFileOptions::default(),
                )
                .unwrap();
            archive.write_all(&vec![index as u8; 128 * 1024]).unwrap();
        }
        archive.finish().unwrap();
        validate_switch_firmware_zip(&firmware).unwrap();

        let malformed = temporary.path().join("malformed-firmware.zip");
        let file = File::create(&malformed).unwrap();
        let mut archive = zip::ZipWriter::new(file);
        for index in 0..10 {
            archive
                .start_file(
                    format!("dump/{index:032x}.nca"),
                    zip::write::SimpleFileOptions::default(),
                )
                .unwrap();
            archive.write_all(&vec![index as u8; 128 * 1024]).unwrap();
        }
        archive
            .start_file(
                "dump/not-a-content-id.cnmt.nca",
                zip::write::SimpleFileOptions::default(),
            )
            .unwrap();
        archive.write_all(b"invalid").unwrap();
        archive.finish().unwrap();
        assert!(validate_switch_firmware_zip(&malformed).is_err());

        let unrelated = temporary.path().join("unrelated.zip");
        write_zip(&unrelated, &[("readme.txt", b"not firmware")]);
        assert!(validate_switch_firmware_zip(&unrelated).is_err());
    }

    #[test]
    fn switch_key_zip_combines_prod_and_optional_title_keys() {
        let temporary = tempfile::tempdir().unwrap();
        let mut prod_lines = vec![format!("header_key = {}", "a1".repeat(16))];
        for index in 0..7 {
            prod_lines.push(format!("master_key_{index:02x} = {}", "b2".repeat(16)));
        }
        let prod = prod_lines.join("\n");
        let title = format!("{} = {}", "12".repeat(16), "ab".repeat(16));
        let keys_zip = temporary.path().join("console-keys.zip");
        write_zip(
            &keys_zip,
            &[
                ("dump/prod.keys", prod.as_bytes()),
                ("title.keys", title.as_bytes()),
            ],
        );

        assert!(switch_key_zip_contains_prod_keys(&keys_zip).unwrap());
        let package = read_switch_key_package(&keys_zip).unwrap();
        assert_eq!(package.prod_keys, prod.as_bytes());
        assert_eq!(package.title_keys.as_deref(), Some(title.as_bytes()));

        let invalid = temporary.path().join("title-only.zip");
        write_zip(&invalid, &[("title.keys", title.as_bytes())]);
        assert!(!switch_key_zip_contains_prod_keys(&invalid).unwrap());
        assert!(read_switch_key_package(&invalid).is_err());
    }

    #[test]
    fn switch_runtime_readiness_detects_valid_files_without_lunchbox_receipts() {
        let temporary = tempfile::tempdir().unwrap();
        let prod_keys = temporary.path().join("prod.keys");
        let mut key_lines = vec![format!("header_key = {}", "a1".repeat(16))];
        for index in 0..7 {
            key_lines.push(format!("master_key_{index:02x} = {}", "b2".repeat(16)));
        }
        fs::write(&prod_keys, key_lines.join("\n")).unwrap();

        let firmware_directory = temporary.path().join("registered");
        fs::create_dir(&firmware_directory).unwrap();
        for index in 0..10 {
            let nca_directory = firmware_directory.join(format!("{index:032x}.nca"));
            fs::create_dir(&nca_directory).unwrap();
            fs::write(nca_directory.join("00"), vec![index as u8; 128 * 1024]).unwrap();
        }

        let rule = |source: &str, package: &str, install_mode: &str| FirmwareRuleRow {
            rule_key: package.to_owned(),
            runtime_kind: "ryubing".to_owned(),
            runtime_name: "Ryubing (Ryujinx fork)".to_owned(),
            source_id: source.to_owned(),
            source_transport: "manual".to_owned(),
            source_url: String::new(),
            torrent_file: String::new(),
            path_prefix: String::new(),
            package_name: package.to_owned(),
            target_subdir: String::new(),
            install_mode: install_mode.to_owned(),
            target_strategy: "managed_import".to_owned(),
            required: true,
            supports_hle_fallback: false,
            notes: String::new(),
        };
        assert!(managed_runtime_target_ready(
            &rule(
                "manual:nintendo-switch-keys",
                SWITCH_KEYS_PACKAGE,
                "merge_tree"
            ),
            temporary.path()
        ));
        assert!(managed_runtime_target_ready(
            &rule(
                "manual:nintendo-switch-firmware",
                SWITCH_FIRMWARE_PACKAGE,
                "merge_tree"
            ),
            &firmware_directory
        ));
    }

    #[test]
    fn legacy_flat_ryubing_firmware_is_migrated_idempotently() {
        let temporary = tempfile::tempdir().unwrap();
        let registered = temporary.path().join("bis/system/Contents/registered");
        fs::create_dir_all(&registered).unwrap();
        for index in 0..10 {
            fs::write(
                registered.join(format!("{index:032x}.nca")),
                vec![index as u8; 128 * 1024],
            )
            .unwrap();
        }

        assert!(migrate_legacy_ryubing_firmware_layout(&registered).unwrap());
        assert!(validate_switch_firmware_directory(&registered, "ryubing").is_ok());
        assert!(
            registered
                .join("00000000000000000000000000000000.nca/00")
                .is_file()
        );
        assert!(!migrate_legacy_ryubing_firmware_layout(&registered).unwrap());
    }

    #[test]
    fn switch_firmware_sync_atomically_replaces_the_registered_nca_set() {
        let temporary = tempfile::tempdir().unwrap();
        let source = temporary.path().join("source/nested");
        let target = temporary.path().join("nand/system/Contents/registered");
        fs::create_dir_all(&source).unwrap();
        fs::create_dir_all(&target).unwrap();
        fs::write(target.join("obsolete.nca"), b"old firmware").unwrap();
        fs::write(
            target.join("unrelated.txt"),
            b"must not survive replacement",
        )
        .unwrap();
        for index in 0..10 {
            fs::write(
                source.join(format!("{index:032x}.nca")),
                vec![index as u8; 16],
            )
            .unwrap();
        }

        copy_switch_firmware_tree(
            temporary.path().join("source").as_path(),
            &target,
            "ryubing",
        )
        .unwrap();

        let mut installed = fs::read_dir(&target)
            .unwrap()
            .map(|entry| entry.unwrap().file_name().to_string_lossy().into_owned())
            .collect::<Vec<_>>();
        installed.sort();
        assert_eq!(installed.len(), 10);
        assert_eq!(installed[0], "00000000000000000000000000000000.nca");
        assert_eq!(installed[9], "00000000000000000000000000000009.nca");
        assert!(target.join(&installed[0]).join("00").is_file());
        assert!(target.join(&installed[9]).join("00").is_file());
        assert!(!target.join("obsolete.nca").exists());
        assert!(!target.join("unrelated.txt").exists());
    }

    #[test]
    fn firmware_repair_rejects_a_tampered_extracted_manifest() {
        let temporary = tempfile::tempdir().unwrap();
        let archive = temporary.path().join("bios.zip");
        let extracted = temporary.path().join("extracted");
        fs::create_dir(&extracted).unwrap();
        write_zip(&archive, &[("bios.bin", b"reviewed firmware")]);
        extract_zip_archive(&archive, &extracted).unwrap();
        let files = build_file_manifest("source", "bios.zip", &extracted, &extracted).unwrap();
        let receipt = FirmwarePackageReceipt {
            source_id: "source".into(),
            package_name: "bios.zip".into(),
            archive_path: archive.to_string_lossy().into_owned(),
            extracted_root: extracted.to_string_lossy().into_owned(),
            sha256: sha256_file(&archive).unwrap(),
            file_count: 1,
            imported_at: 1,
        };
        let store = SettingsStore::at(temporary.path().join("state.db")).unwrap();
        store.record_firmware_package(&receipt, &files).unwrap();

        verify_package_receipt(&receipt, &store).unwrap();
        fs::write(extracted.join("bios.bin"), b"tampered firmware").unwrap();
        assert!(verify_package_receipt(&receipt, &store).is_err());
    }

    #[test]
    fn firmware_copy_replaces_only_the_exact_target() {
        let temporary = tempfile::tempdir().unwrap();
        let source = temporary.path().join("source.bin");
        let target = temporary.path().join("runtime/bios.bin");
        fs::write(&source, b"new").unwrap();
        fs::create_dir_all(target.parent().unwrap()).unwrap();
        fs::write(&target, b"old").unwrap();

        atomic_copy(&source, &target).unwrap();
        assert_eq!(fs::read(&target).unwrap(), b"new");
        assert_eq!(fs::read(&source).unwrap(), b"new");
    }

    #[test]
    #[ignore = "downloads the current Minerva torrent metadata"]
    fn live_minerva_firmware_packages_have_one_exact_torrent_member() {
        let repository = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
        let catalog = crate::catalog::open_read_only(
            &repository.join("build/lunchbox.db"),
            "firmware catalog",
        )
        .unwrap();
        let minerva =
            crate::catalog::open_read_only(&repository.join("db/minerva.db"), "Minerva catalog")
                .unwrap();
        let mut statement = catalog
            .prepare(
                "SELECT DISTINCT s.torrent_file, s.path_prefix, r.source_package_name
                 FROM firmware_sources s
                 JOIN firmware_rules r ON r.source_id=s.id
                 WHERE s.transport='minerva'
                 ORDER BY s.torrent_file, r.source_package_name",
            )
            .unwrap();
        let expected = statement
            .query_map([], |row| {
                Ok((
                    row.get::<_, String>(0)?,
                    row.get::<_, String>(1)?,
                    row.get::<_, String>(2)?,
                ))
            })
            .unwrap()
            .collect::<rusqlite::Result<Vec<_>>>()
            .unwrap();
        let mut listings = HashMap::<String, Vec<(u32, String)>>::new();
        for (torrent_file, prefix, package) in expected {
            if !listings.contains_key(&torrent_file) {
                let url = minerva
                    .query_row(
                        "SELECT torrent_url FROM minerva_torrents WHERE torrent_file=?1",
                        [&torrent_file],
                        |row| row.get::<_, String>(0),
                    )
                    .unwrap();
                let bytes = crate::game_details::torrent_bytes_for_url(&url).unwrap();
                let torrent = Torrent::read_from_bytes(&bytes).unwrap();
                listings.insert(
                    torrent_file.clone(),
                    torrent_file_listing(&torrent).unwrap(),
                );
            }
            let listing = listings.get(&torrent_file).unwrap();
            let exact = normalized_torrent_path(&format!("{prefix}{package}"));
            let matches = listing
                .iter()
                .filter(|(_, path)| {
                    let path = normalized_torrent_path(path);
                    path == exact || path.ends_with(&format!("/{exact}"))
                })
                .count();
            assert_eq!(matches, 1, "{torrent_file}: {exact}");
        }
    }
}
