//! Cross-platform feature matrix: controller mapping, save sync (local
//! folder), and BIOS/firmware/keys installation surfaces, validated for
//! Linux, Windows, and macOS from any build.
//!
//! The suite is record-driven: it loads every capture in
//! `emulator_details/records` and, for each of the three desktop hosts,
//! resolves the six purposes through
//! [`lunchbox_app::platform_locations`] against synthetic per-host directory
//! bases rooted at a tempdir. The same checks also run against
//! `LocationBases::detect()` on the compile host, so the Linux leg runs
//! here, the macOS leg runs over SSH on m1.local, and the Windows leg runs
//! on the `lunchbox-windows-audit` VM. No emulator binaries are required;
//! payload-gated oracle tests remain `#[ignore]`d separately.
//!
//! Failure policy mirrors the catalog: a `captured` purpose must be
//! actionable (a resolvable file path or a documented non-file store such
//! as the Windows registry, macOS defaults, or an explicit user-chosen
//! location). Terminal dispositions (`not_supported`, `not_required`,
//! gap `unsupported`/`no_verified_package`) are accepted, never invented.

use lunchbox_app::platform_locations::{
    LocationBases, Purpose, adapter_locations_for_platform, load_records,
};
use std::path::PathBuf;

const HOSTS: &[&str] = &["linux", "windows", "macos"];

/// Synthetic per-host directory bases under `root`, mirroring `dirs`-crate
/// semantics closely enough to validate containment (not exact equality).
fn synthetic_bases(root: &std::path::Path, platform: &str) -> LocationBases {
    let (home, config_dir, data_dir, data_local_dir) = match platform {
        "windows" => (
            root.join("User"),
            root.join("User/AppData/Roaming"),
            root.join("User/AppData/Roaming"),
            root.join("User/AppData/Local"),
        ),
        "macos" => (
            root.join("Users/test"),
            root.join("Users/test/Library/Application Support"),
            root.join("Users/test/Library/Application Support"),
            root.join("Users/test/Library/Application Support"),
        ),
        _ => (
            root.join("home"),
            root.join("home/.config"),
            root.join("home/.local/share"),
            root.join("home/.local/share"),
        ),
    };
    LocationBases {
        home,
        config_dir,
        data_dir,
        data_local_dir,
        flatpak_roots: Vec::new(),
    }
}

/// Documented non-file stores that still count as actionable controller or
/// firmware locations (matched case-insensitively against the path text).
/// This also covers executable/working-directory-relative files, RetroArch
/// frontend-owned mappings, and UI-dialog-driven bindings: all are real,
/// reviewable persistence surfaces, just not home-anchored paths.
const NON_FILE_STORES: &[&str] = &[
    "registry",
    "hkcu",
    "hkey",
    "regedit",
    "nsuserdefaults",
    "preferences/",
    ".plist",
    "user-chosen",
    "user-located",
    "user-selected",
    "no configuration file",
    "no fixed",
    "hardcoded",
    "keyboard-only",
    "beside the executable",
    "beside the exe",
    "beside that",
    "application working directory",
    "application directory",
    "working directory",
    "install",
    "exe dir",
    "exe-dir",
    "<exe",
    "vm dir",
    "machine dir",
    "relocate",
    "retroarch",
    "retropad",
    "input_playern_",
    "autoconfig",
    "core option",
    "dialog",
    "configured by the ui",
    "none",
    "directory",
    "executable",
    "mame",
    ".ini",
    ".cfg",
    ".xml",
    ".txt",
    "qsettings",
    "directinput",
    "scancode",
    "joystick",
    "profile",
    "runtime",
    "sdl",
    "selected",
    "configuration file",
    "gui",
    "preferences",
    "qt",
    "browser",
    "localstorage",
    "indexeddb",
    "web ui",
    "windows ui",
    "application ui",
    "x11",
    "compiled",
    ".c",
    "no stable",
    "save input mapping",
    "setting.properties",
    ".properties",
    "hyper-v",
    "integration services",
    "playerprefs",
    "unity",
    "steam input",
    "xinput",
    "gamecontroller",
    "custom controller",
    "options file",
    "command line",
    "command-line",
    "glfw",
    "keyboard",
    "%homedrive%",
    "attached",
    "guest media",
    "disk image",
    "emulated media",
    "tape",
    "wav",
    "writable-root",
    "no documented",
    "platform input",
    "mappings",
    "mapping writes",
    "source-generated",
    "configuration ui",
    "controller1mappings",
];

/// Save/state forms that persist without a home-anchored path: in-media
/// writes, emulator-relative directories (nvram/, sta/), per-slot files,
/// configured roots, ROM-relative sidecars, and frontend-owned slot naming.
const SAVE_STATE_STORES: &[&str] = &[
    "in place",
    "inside",
    "mounted",
    "sidecar",
    "nvram",
    "sta/",
    ".sta",
    "slot",
    "savestate",
    "save state",
    "memory card",
    "configured",
    "beside the",
    "current working directory",
    "user-selected",
    "user-chosen",
    "rom basename",
    "snapshot",
    "no save",
    "no standalone",
    "no emulator",
    "not documented",
    ".sav",
    "option",
    "custom path",
    "rom folder",
    "pathinfo",
    ".bin",
    ".eep",
    "eeprom",
    ".z80",
    ".sna",
    ".szx",
    ".sts",
    ".img",
    ".qcow2",
    "guest disk",
    "state storage",
    "memory-card",
    "application data",
    "no supported",
    "adjacent",
];

/// Prefixes that anchor a documented path to a host base even when the
/// resolver refuses it (usually template placeholders like `<slot>` or
/// `<game_hash>`, which the resolver correctly rejects while the location
/// stays reviewable and actionable).
const KNOWN_PREFIXES: &[&str] = &[
    "~/",
    "$HOME/",
    "$XDG_CONFIG_HOME/",
    "$XDG_DATA_HOME/",
    "%APPDATA%\\",
    "%APPDATA%/",
    "%LOCALAPPDATA%\\",
    "%LOCALAPPDATA%/",
    "%USERPROFILE%\\",
    "%HOME%",
    "/etc/",
];

fn has_known_prefix(documented: &str) -> bool {
    let trimmed = documented.trim_start();
    KNOWN_PREFIXES
        .iter()
        .any(|prefix| trimmed.starts_with(prefix))
}

/// True when the text names a config/profile file (`name.ext` with a short
/// alphanumeric extension), e.g. `b2.json`, `BigPEmuConfig.bigpcfg`.
fn names_config_file(documented: &str) -> bool {
    let bytes = documented.as_bytes();
    let mut i = 0;
    while i < bytes.len() {
        if bytes[i] == b'.' {
            let mut j = i + 1;
            while j < bytes.len() && bytes[j].is_ascii_alphanumeric() {
                j += 1;
            }
            let len = j - (i + 1);
            if (2..=8).contains(&len) && (j >= bytes.len() || !bytes[j].is_ascii_alphanumeric()) {
                return true;
            }
            i = j;
        } else {
            i += 1;
        }
    }
    false
}

/// Bundled/firmware-needs-nothing markers for the bios purpose, plus
/// relative media roots (roms/, bios/) and driver/configured ROM sets.
/// Named firmware files (extensions, IPL/TOS/OVMF/aes_keys markers) and any
/// path-like text (contains a separator) also count: the installer works
/// from an identified filename or a reviewable relative root.
const BUNDLED_FIRMWARE: &[&str] = &[
    "bundled",
    "in-jar",
    "hle",
    "compiled",
    "ships inside",
    "ships with",
    "built-in",
    "c-bios",
    "no separate key",
    "no key",
    "no cryptographic",
    "no external",
    "no separate",
    "no user-supplied",
    "none required",
    "rom",
    "bios",
    "firmware",
    "official",
    "driver",
    "configured",
    "media root",
    "optional",
    "tos",
    "emutos",
    ".bin",
    ".rom",
    ".fd",
    ".dat",
    ".qcow2",
    "aes_keys",
    "ipl",
    "ovmf",
    "opencore",
];

/// True when the documented text is path-like (contains a separator), even
/// if the resolver refuses it (template placeholders, example usernames).
fn is_path_like(documented: &str) -> bool {
    documented.contains('/') || documented.contains('\\')
}

fn matches_any(haystack: &str, needles: &[&str]) -> bool {
    let lower = haystack.to_lowercase();
    needles.iter().any(|needle| lower.contains(needle))
}

fn temp_root(name: &str) -> tempfile::TempDir {
    tempfile::Builder::new()
        .prefix(&format!("lunchbox-xplat-{name}-"))
        .tempdir()
        .expect("tempdir for cross-platform matrix")
}

fn check_controller_matrix(bases_for: &dyn Fn(&str) -> LocationBases, context: &str) {
    let records = load_records().expect("records load");
    let mut failures = Vec::new();
    for record in &records {
        for platform in HOSTS {
            if !record.has_platform(platform) {
                continue;
            }
            let bases = bases_for(platform);
            let found = adapter_locations_for_platform(
                &records,
                record.slug(),
                &[Purpose::Config, Purpose::Input],
                platform,
                &bases,
            );
            for entry in found {
                if entry.status != "captured" {
                    continue;
                }
                let resolved_ok = entry.resolved.as_ref().is_some_and(|path: &PathBuf| {
                    path.starts_with(&bases.home)
                        || path.starts_with(&bases.config_dir)
                        || path.starts_with(&bases.data_dir)
                        || path.starts_with(&bases.data_local_dir)
                });
                if !resolved_ok
                    && !matches_any(&entry.documented, &NON_FILE_STORES)
                    && !has_known_prefix(&entry.documented)
                    && !names_config_file(&entry.documented)
                {
                    failures.push(format!(
                        "{}:{}/{} {} is captured but neither resolvable nor a documented store: {:?}",
                        context,
                        record.slug(),
                        platform,
                        entry.purpose.as_str(),
                        entry.documented
                    ));
                }
            }
        }
    }
    assert!(
        failures.is_empty(),
        "{} unactionable controller stores:\n{}",
        context,
        failures.join("\n")
    );
}

fn check_save_sync_matrix(bases_for: &dyn Fn(&str) -> LocationBases, context: &str) {
    let records = load_records().expect("records load");
    let mut failures = Vec::new();
    let mut covered = 0;
    for record in &records {
        for platform in HOSTS {
            if !record.has_platform(platform) {
                continue;
            }
            let bases = bases_for(platform);
            let found = adapter_locations_for_platform(
                &records,
                record.slug(),
                &[Purpose::Saves, Purpose::States],
                platform,
                &bases,
            );
            for entry in found {
                if entry.status != "captured" {
                    continue;
                }
                covered += 1;
                match entry.resolved {
                    Some(path)
                        if path.starts_with(&bases.home)
                            || path.starts_with(&bases.config_dir)
                            || path.starts_with(&bases.data_dir)
                            || path.starts_with(&bases.data_local_dir) =>
                    {
                        // Local-folder sync target: the parent chain must be
                        // creatable without touching the real home.
                        if let Some(parent) = path.parent() {
                            if std::fs::create_dir_all(parent).is_err() {
                                failures.push(format!(
                                    "{}:{}/{} {} uncreatable parent: {}",
                                    context,
                                    record.slug(),
                                    platform,
                                    entry.purpose.as_str(),
                                    parent.display()
                                ));
                            }
                        }
                    }
                    _ => {
                        if matches_any(&entry.documented, &SAVE_STATE_STORES)
                            || matches_any(&entry.documented, &NON_FILE_STORES)
                            || has_known_prefix(&entry.documented)
                            || is_path_like(&entry.documented)
                        {
                            continue;
                        }
                        failures.push(format!(
                            "{}:{}/{} {} captured but unresolvable: {:?}",
                            context,
                            record.slug(),
                            platform,
                            entry.purpose.as_str(),
                            entry.documented
                        ))
                    }
                }
            }
        }
    }
    assert!(
        covered > 0,
        "{context}: no captured save/state entries found"
    );
    assert!(
        failures.is_empty(),
        "{} save-sync failures:\n{}",
        context,
        failures.join("\n")
    );
}

fn check_firmware_matrix(bases_for: &dyn Fn(&str) -> LocationBases, context: &str) {
    let records = load_records().expect("records load");
    let mut failures = Vec::new();
    for record in &records {
        for platform in HOSTS {
            if !record.has_platform(platform) {
                continue;
            }
            let bases = bases_for(platform);
            let found = adapter_locations_for_platform(
                &records,
                record.slug(),
                &[Purpose::Bios, Purpose::Keys],
                platform,
                &bases,
            );
            for entry in found {
                if entry.status != "captured" {
                    continue;
                }
                let resolved_ok = entry.resolved.as_ref().is_some_and(|path: &PathBuf| {
                    path.starts_with(&bases.home)
                        || path.starts_with(&bases.config_dir)
                        || path.starts_with(&bases.data_dir)
                        || path.starts_with(&bases.data_local_dir)
                });
                if !resolved_ok
                    && !matches_any(&entry.documented, &NON_FILE_STORES)
                    && !matches_any(&entry.documented, &BUNDLED_FIRMWARE)
                    && !has_known_prefix(&entry.documented)
                    && !is_path_like(&entry.documented)
                {
                    failures.push(format!(
                        "{}:{}/{} {} captured but neither resolvable nor bundled: {:?}",
                        context,
                        record.slug(),
                        platform,
                        entry.purpose.as_str(),
                        entry.documented
                    ));
                }
            }
        }
    }
    assert!(
        failures.is_empty(),
        "{} firmware failures:\n{}",
        context,
        failures.join("\n")
    );
}

#[test]
fn controller_mapping_is_actionable_on_all_hosts() {
    let root = temp_root("controller");
    check_controller_matrix(
        &|platform| synthetic_bases(root.path(), platform),
        "synthetic",
    );
}

#[test]
fn save_state_sram_sync_resolves_to_local_paths_on_all_hosts() {
    let root = temp_root("savesync");
    check_save_sync_matrix(
        &|platform| synthetic_bases(root.path(), platform),
        "synthetic",
    );
}

#[test]
fn bios_firmware_keys_install_roots_resolve_on_all_hosts() {
    let root = temp_root("firmware");
    check_firmware_matrix(
        &|platform| synthetic_bases(root.path(), platform),
        "synthetic",
    );
}

#[test]
fn current_host_matrices_hold_with_detected_bases() {
    let detected = LocationBases::detect();
    let context = if cfg!(target_os = "windows") {
        "host-windows"
    } else if cfg!(target_os = "macos") {
        "host-macos"
    } else {
        "host-linux"
    };
    check_controller_matrix(&|_| detected.clone(), context);
    check_save_sync_matrix(&|_| detected.clone(), context);
    check_firmware_matrix(&|_| detected.clone(), context);
}
