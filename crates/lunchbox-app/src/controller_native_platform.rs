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

/// Environment isolating user roots under `base`.
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

/// Portable SDL device pin: the device at `path` must exist exactly once in
/// `devices` by path, with the expected SDL index. Names and GUIDs are
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

#[cfg(test)]
mod tests {
    use super::*;

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
