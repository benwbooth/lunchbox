//! Cross-platform host abstraction for native launch adapters.
//!
//! The per-emulator writers (INI patchers, table renderers, database lines)
//! are pure functions and already portable. What differs per host is the
//! launch session around them:
//!
//! * device identity — Linux pins kernel input identity through
//!   `InputTopology` (sysfs). Windows/macOS have no sysfs, so sessions pin
//!   the SDL device-interface `path` plus index, require it to be unique,
//!   and re-probe it at verify time. Names and GUIDs are never identity:
//!   the probe marks GUIDs as diagnostic model data, and names are fuzzy.
//! * child ownership — Linux checks `/proc/{pid}/exe` plus the SDL library
//!   in `/proc/{pid}/maps`. Other hosts check the child executable through
//!   `sysinfo` (all hosts) and re-probe the device instead of reading maps.
//!   The ownership guarantee is therefore strongest on Linux; the per-host
//!   strength is documented at each call site, never silently equated.
//! * session base paths — Linux/macOS isolate with `HOME`; Windows isolates
//!   with `USERPROFILE`/`APPDATA`/`LOCALAPPDATA` under the session base.
//!   Adapters that stage beside the working directory need no override on
//!   any host.

use anyhow::{Context, Result, ensure};
use std::ffi::OsString;
use std::path::Path;

/// Host the current binary was compiled for.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Host {
    Linux,
    Windows,
    Macos,
}

pub(crate) fn current_host() -> Host {
    if cfg!(target_os = "windows") {
        Host::Windows
    } else if cfg!(target_os = "macos") {
        Host::Macos
    } else {
        Host::Linux
    }
}

/// True when `pid` is alive and its executable is `executable`.
/// Portable across Linux, Windows, and macOS through `sysinfo`.
pub(crate) fn child_exe_matches(pid: u32, executable: &Path) -> Result<bool> {
    let wanted = executable.canonicalize().with_context(|| {
        format!(
            "trusted executable is not resolvable: {}",
            executable.display()
        )
    })?;
    let mut system = sysinfo::System::new();
    system.refresh_processes(
        sysinfo::ProcessesToUpdate::Some(&[sysinfo::Pid::from_u32(pid)]),
        false,
    );
    let Some(process) = system.process(sysinfo::Pid::from_u32(pid)) else {
        return Ok(false);
    };
    let Some(actual) = process.exe().and_then(|path| path.canonicalize().ok()) else {
        return Ok(false);
    };
    Ok(actual == wanted)
}

/// True when `pid` maps the SDL `library`.
/// Linux reads `/proc/{pid}/maps`. Other hosts have no equivalent readable
/// module map through the vendored dependencies, so they return an explicit
/// refusal: callers must fall back to exe matching plus device re-probing,
/// and must say so in their launch description.
pub(crate) fn child_maps_library(pid: u32, library: &Path) -> Result<bool> {
    if cfg!(target_os = "linux") {
        let expected = library.canonicalize()?;
        let maps = std::fs::read_to_string(format!("/proc/{pid}/maps"))?;
        Ok(maps.lines().any(|line| {
            let path = line
                .split_whitespace()
                .skip(5)
                .collect::<Vec<_>>()
                .join(" ")
                .replace("\\040", " ");
            Path::new(&path) == expected
        }))
    } else {
        anyhow::bail!(
            "module-map ownership is Linux-only; use child_exe_matches plus device re-probing"
        )
    }
}

/// True when a bubblewrap sandbox can nest here: Linux, not already inside
/// a Flatpak/OCI container or an AppImage runtime, where nested
/// bubblewrap is fragile or privileged away. Native and Nix binaries get
/// the sandbox; Flatpak/AppImage-contained launches run direct.
pub(crate) fn linux_sandbox_available() -> bool {
    if !cfg!(target_os = "linux") {
        return false;
    }
    if std::path::Path::new("/.flatpak-info").exists()
        || std::path::Path::new("/run/.containerenv").exists()
    {
        return false;
    }
    if std::env::var_os("APPIMAGE").is_some() {
        return false;
    }
    true
}

/// True when the target executable is itself an AppImage: wrapping it in
/// bubblewrap would fight its own runtime mounts, so it always runs direct.
pub(crate) fn is_appimage_target(executable: &Path) -> bool {
    executable
        .extension()
        .is_some_and(|extension| extension.eq_ignore_ascii_case("AppImage"))
}

/// True when this launch should sandbox through bubblewrap: Linux, outside
/// any container runtime, with a plain (non-AppImage) native target.
/// Packaging, not just the OS, decides.
pub(crate) fn use_bubblewrap_sandbox(executable: &Path) -> bool {
    cfg!(target_os = "linux") && linux_sandbox_available() && !is_appimage_target(executable)
}
/// Stable identity for a filesystem object across verify calls: Unix
/// device/inode pairs, Windows creation-time plus size/attribute packs.
/// Used to detect directory replacement; never persisted across reboots.
pub(crate) fn file_identity(path: &Path) -> Result<(u64, u64)> {
    let metadata = std::fs::metadata(path)
        .with_context(|| format!("identity metadata is missing: {}", path.display()))?;
    #[cfg(unix)]
    {
        use std::os::unix::fs::MetadataExt;
        Ok((metadata.dev(), metadata.ino()))
    }
    #[cfg(target_os = "windows")]
    {
        use std::os::windows::fs::MetadataExt;
        use std::time::UNIX_EPOCH;
        // The volume/file-index pair needs an unstable feature, so pin
        // creation time plus size/attributes instead. This detects
        // replacement (a new file gets a new creation time), which is the
        // threat the identity guards against; it is weaker than the Unix
        // device/inode pair across timestamp-preserving copies, and call
        // sites must not equate the strengths.
        let created = metadata
            .creation_time()
            .context("creation identity is unavailable")?
            .duration_since(UNIX_EPOCH)
            .map(|age| age.as_nanos() as u64)
            .unwrap_or_default();
        let packed =
            (metadata.file_size() << 32) | u64::from(metadata.file_attributes());
        Ok((created, packed))
    }
    #[cfg(not(any(unix, target_os = "windows")))]
    {
        anyhow::bail!("file identity is unsupported on this host")
    }
}

/// Linux/macOS override `HOME`. Windows overrides the profile roots SDL and
/// most emulators resolve (`USERPROFILE`, `APPDATA`, `LOCALAPPDATA`).
/// The caller creates the directories; this only renders the pairs.
pub(crate) fn session_user_env(base: &Path) -> Vec<(OsString, OsString)> {
    if cfg!(target_os = "windows") {
        let roaming = base.join("AppData").join("Roaming");
        let local = base.join("AppData").join("Local");
        vec![
            (OsString::from("USERPROFILE"), base.as_os_str().to_owned()),
            (OsString::from("APPDATA"), roaming.as_os_str().to_owned()),
            (OsString::from("LOCALAPPDATA"), local.as_os_str().to_owned()),
        ]
    } else {
        vec![(OsString::from("HOME"), base.as_os_str().to_owned())]
    }
}

/// Create the user-root directories `session_user_env` points at.
/// Idempotent; portable.
pub(crate) fn prepare_user_dirs(base: &Path) -> Result<()> {
    if cfg!(target_os = "windows") {
        for dir in [
            base.to_path_buf(),
            base.join("AppData").join("Roaming"),
            base.join("AppData").join("Local"),
        ] {
            std::fs::create_dir_all(&dir).with_context(|| {
                format!("session user root is not creatable: {}", dir.display())
            })?;
        }
    }
    Ok(())
}

/// SDL gamecontrollerdb platform token for this host: `Linux`,
/// `Windows`, or `Mac OS X`, matching `SDL_GetPlatform()`.
pub(crate) fn sdl_platform_name() -> &'static str {
    if cfg!(target_os = "windows") {
        "Windows"
    } else if cfg!(target_os = "macos") {
        "Mac OS X"
    } else {
        "Linux"
    }
}

/// Portable SDL device pin: the device at `path` must exist exactly once
/// in `devices`, with the expected SDL index. Names and GUIDs are
/// never consulted. Linux callers additionally hold an `InputTopology`;
/// other hosts rely on this plus snapshot routing equality.
pub(crate) fn require_unique_device_path<'a>(
    devices: &'a [lunchbox_controller_probe::sdl2::Device],
    path: &str,
    expected_index: u32,
) -> Result<&'a lunchbox_controller_probe::sdl2::Device> {
    let found = devices
        .iter()
        .filter(|device| device.path.as_deref() == Some(path))
        .collect::<Vec<_>>();
    ensure!(
        found.len() == 1,
        "SDL device path is missing or ambiguous at launch"
    );
    ensure!(
        found[0].device_index == expected_index,
        "SDL device index moved (expected {expected_index}, found {})",
        found[0].device_index
    );
    Ok(found[0])
}

/// Portable SDL3 device pin: the device at `path` must exist exactly once
/// in `devices`. SDL3 devices carry no stable numeric index (instance IDs
/// are runtime-ephemeral), so path presence plus uniqueness is the pin.
/// Names and GUIDs are never consulted.
pub(crate) fn require_unique_sdl3_path<'a>(
    devices: &'a [lunchbox_controller_probe::Device],
    path: &str,
) -> Result<&'a lunchbox_controller_probe::Device> {
    let found = devices
        .iter()
        .filter(|device| device.path.as_deref() == Some(path))
        .collect::<Vec<_>>();
    ensure!(
        found.len() == 1,
        "SDL3 device path is missing or ambiguous at launch"
    );
    Ok(found[0])
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sandbox_and_appimage_targets_classify() {
        assert!(!is_appimage_target(Path::new("/usr/bin/86box")));
        assert!(is_appimage_target(Path::new("/opt/Emu.AppImage")));
        assert!(is_appimage_target(Path::new("/opt/emu.appimage")));
        // Outside containers on Linux this is true; anywhere else false.
        // The assertion only pins the non-Linux side deterministically.
        if !cfg!(target_os = "linux") {
            assert!(!linux_sandbox_available());
            assert!(!use_bubblewrap_sandbox(Path::new("/usr/bin/86box")));
        }
    }

    #[test]
    fn user_env_is_host_appropriate() {
        let base = Path::new("/tmp/session");
        let env = session_user_env(base);
        if cfg!(target_os = "windows") {
            assert!(env.iter().any(|(key, _)| key == "APPDATA"));
            assert!(env.iter().any(|(key, _)| key == "USERPROFILE"));
        } else {
            assert_eq!(env.len(), 1);
            assert_eq!(env[0].0, OsString::from("HOME"));
        }
    }

    #[test]
    fn device_path_pin_rejects_ambiguity() {
        use lunchbox_controller_probe::sdl2::Device;
        let device = |path: &str, index: u32| Device {
            device_index: index,
            instance_id: 0,
            path: Some(path.to_owned()),
            name: Some("Pad".to_owned()),
            is_game_controller: true,
            guid: String::new(),
            mapping: None,
            controls: None,
            linux_classic: None,
            linux_evdev: None,
            sampled_state: None,
        };
        let devices = vec![device("/dev/a", 0), device("/dev/b", 1)];
        assert!(require_unique_device_path(&devices, "/dev/a", 0).is_ok());
        assert!(require_unique_device_path(&devices, "/dev/a", 1).is_err());
        assert!(require_unique_device_path(&devices, "/dev/missing", 0).is_err());
        let dupes = vec![device("/dev/a", 0), device("/dev/a", 0)];
        assert!(require_unique_device_path(&dupes, "/dev/a", 0).is_err());
    }
}
