//! Target-sandbox supervisor for audited Nestopia UE 1.53.2 launches.
//!
//! The caller inventories controllers using this same staged executable and
//! target SDL first. This mode rechecks routing, starts `/app/bin/nestopia`,
//! and writes readiness only after both private config files are selected and
//! every chosen joydev node is held open. Game saves and states deliberately
//! remain under the Flatpak's persistent XDG data directory.

use crate::{file_hash, sdl2::Snapshot};
use anyhow::{Context, Result, ensure};
use serde::{Deserialize, Serialize};
use std::{
    fs::{File, OpenOptions},
    io::{Read, Write},
    os::unix::{
        fs::{MetadataExt, OpenOptionsExt, PermissionsExt},
        process::CommandExt,
    },
    path::{Path, PathBuf},
    process::{Child, Command},
    time::{Duration, Instant},
};

const REQUEST_LIMIT: u64 = 8 * 1024 * 1024;
const ENVIRONMENT_LIMIT: u64 = 1024 * 1024;
const DESCRIPTOR_LIMIT: usize = 4096;
const STARTUP_TIMEOUT: Duration = Duration::from_secs(20);
const APP_ID: &str = "ca._0ldsk00l.Nestopia";

fn ensure_mode(path: &Path, mode: u32, directory: bool, what: &str) -> Result<()> {
    let metadata = std::fs::symlink_metadata(path)?;
    ensure!(
        if directory {
            metadata.is_dir()
        } else {
            metadata.is_file()
        } && metadata.mode() & 0o7777 == mode,
        "{what} type or mode changed"
    );
    Ok(())
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Request {
    pub schema_version: u32,
    pub executable: PathBuf,
    pub executable_sha256: String,
    pub sdl_library: PathBuf,
    pub sdl_library_sha256: String,
    pub private_root: PathBuf,
    pub main_config_sha256: String,
    pub input_config_sha256: String,
    pub data_home: PathBuf,
    pub content: PathBuf,
    pub current_directory: PathBuf,
    pub runtime_paths: Vec<String>,
    pub inventory: Snapshot,
    pub receipt: PathBuf,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Receipt {
    pub schema_version: u32,
    pub executable_sha256: String,
    pub sdl_library_sha256: String,
    pub main_config_sha256: String,
    pub input_config_sha256: String,
    pub input_environment_sha256: String,
    pub data_home: PathBuf,
    pub runtime_paths: Vec<String>,
}

impl Request {
    fn main_config(&self) -> PathBuf {
        self.private_root.join("config/nestopia/nestopia.conf")
    }

    fn input_config(&self) -> PathBuf {
        self.private_root.join("config/nestopia/input.conf")
    }

    pub fn expected_receipt(&self) -> Receipt {
        Receipt {
            schema_version: 1,
            executable_sha256: self.executable_sha256.clone(),
            sdl_library_sha256: self.sdl_library_sha256.clone(),
            main_config_sha256: self.main_config_sha256.clone(),
            input_config_sha256: self.input_config_sha256.clone(),
            input_environment_sha256: self.inventory.input_environment_sha256.clone(),
            data_home: self.data_home.clone(),
            runtime_paths: self.runtime_paths.clone(),
        }
    }

    pub fn validate(&self) -> Result<()> {
        ensure!(
            self.schema_version == 1,
            "Unknown Nestopia supervisor request"
        );
        for path in [
            &self.executable,
            &self.sdl_library,
            &self.private_root,
            &self.data_home,
            &self.content,
            &self.current_directory,
            &self.receipt,
        ] {
            ensure!(
                path.is_absolute(),
                "Nestopia supervisor paths must be absolute"
            );
        }
        ensure!(
            self.executable == Path::new("/app/bin/nestopia"),
            "Nestopia supervisor supports only the audited Flatpak command"
        );
        for hash in [
            &self.executable_sha256,
            &self.sdl_library_sha256,
            &self.main_config_sha256,
            &self.input_config_sha256,
            &self.inventory.input_environment_sha256,
        ] {
            ensure!(
                hash.len() == 64 && hash.bytes().all(|byte| byte.is_ascii_hexdigit()),
                "Nestopia supervisor hash is malformed"
            );
        }
        ensure!(
            self.content.is_file()
                && self.current_directory.is_dir()
                && self.content.parent().is_some_and(|parent| {
                    parent.canonicalize().ok().as_ref()
                        == self.current_directory.canonicalize().ok().as_ref()
                }),
            "Nestopia supervisor content or launch directory is invalid"
        );
        ensure_mode(&self.private_root, 0o700, true, "Nestopia private root")?;
        for relative in ["home", "config", "config/nestopia", "cache", "state"] {
            ensure_mode(
                &self.private_root.join(relative),
                0o700,
                true,
                "Nestopia private directory",
            )?;
        }
        ensure_mode(
            &self.main_config(),
            0o600,
            false,
            "Nestopia private main config",
        )?;
        ensure_mode(
            &self.input_config(),
            0o600,
            false,
            "Nestopia private input config",
        )?;
        ensure!(
            self.data_home.is_dir()
                && self
                    .data_home
                    .ends_with(Path::new(".var/app").join(APP_ID).join("data"))
                && !self.data_home.starts_with(&self.private_root),
            "Nestopia persistent data home is outside the audited Flatpak profile"
        );
        ensure!(
            self.receipt.parent() == Some(self.private_root.as_path())
                && self
                    .receipt
                    .file_name()
                    .is_some_and(|name| name == "ready.json")
                && !self.receipt.try_exists()?,
            "Nestopia readiness receipt layout is invalid or already used"
        );
        ensure!(
            self.runtime_paths.len() == 2
                && self
                    .runtime_paths
                    .iter()
                    .all(|path| Path::new(path).parent() == Some(Path::new("/dev/input"))),
            "Nestopia supervisor needs two /dev/input controller paths"
        );
        let unique: std::collections::BTreeSet<_> = self.runtime_paths.iter().collect();
        ensure!(
            unique.len() == 2,
            "Nestopia supervisor controller paths must be distinct"
        );
        ensure!(
            self.inventory.version[0] == 2
                && self.inventory.library == self.sdl_library
                && self.inventory.library_sha256 == self.sdl_library_sha256
                && self.inventory.devices.len() <= 12
                && self
                    .inventory
                    .devices
                    .iter()
                    .enumerate()
                    .all(|(index, device)| device.device_index as usize == index),
            "Nestopia supervisor inventory is not a complete target SDL2 snapshot"
        );
        for path in &self.runtime_paths {
            ensure!(
                self.inventory.device_at_path(path)?.device_index <= 9,
                "Nestopia selected controller index is not encodable"
            );
        }
        Ok(())
    }
}

fn read_bounded(path: &Path, limit: u64, what: &str) -> Result<Vec<u8>> {
    let file = File::open(path).with_context(|| format!("Opening {what}"))?;
    ensure!(file.metadata()?.is_file(), "{what} must be a regular file");
    let mut bytes = Vec::new();
    file.take(limit + 1).read_to_end(&mut bytes)?;
    ensure!(bytes.len() as u64 <= limit, "{what} exceeds its size limit");
    Ok(bytes)
}

pub fn read_request(path: &Path) -> Result<Request> {
    ensure!(
        path.is_absolute(),
        "Nestopia supervisor request path must be absolute"
    );
    ensure_mode(path, 0o600, false, "Nestopia supervisor request")?;
    let request: Request = serde_json::from_slice(&read_bounded(
        path,
        REQUEST_LIMIT,
        "Nestopia supervisor request",
    )?)
    .context("Invalid Nestopia supervisor request")?;
    request.validate()?;
    Ok(request)
}

fn routing(mut snapshot: Snapshot) -> Snapshot {
    for device in &mut snapshot.devices {
        device.controls = None;
        device.linux_classic = None;
        device.linux_evdev = None;
        device.sampled_state = None;
    }
    snapshot
}

fn environment(pid: u32) -> Result<Vec<Vec<u8>>> {
    let bytes = read_bounded(
        Path::new(&format!("/proc/{pid}/environ")),
        ENVIRONMENT_LIMIT,
        "Nestopia child environment",
    )?;
    Ok(bytes
        .split(|byte| *byte == 0)
        .filter(|entry| !entry.is_empty())
        .map(<[u8]>::to_vec)
        .collect())
}

fn has_exact_environment(entries: &[Vec<u8>], name: &[u8], value: &[u8]) -> bool {
    let mut expected = Vec::with_capacity(name.len() + value.len() + 1);
    expected.extend_from_slice(name);
    expected.push(b'=');
    expected.extend_from_slice(value);
    entries.iter().filter(|entry| *entry == &expected).count() == 1
}

fn has_unique_nonempty_environment(entries: &[Vec<u8>], name: &[u8]) -> bool {
    let mut prefix = name.to_vec();
    prefix.push(b'=');
    let matches: Vec<_> = entries
        .iter()
        .filter_map(|entry| entry.strip_prefix(prefix.as_slice()))
        .collect();
    matches.len() == 1 && !matches[0].is_empty()
}

fn has_no_environment(entries: &[Vec<u8>], name: &[u8]) -> bool {
    let mut prefix = name.to_vec();
    prefix.push(b'=');
    !entries.iter().any(|entry| entry.starts_with(&prefix))
}

fn mapped_file(pid: u32, expected: &Path) -> Result<bool> {
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
}

fn same_mounted_file(pid: u32, path: &Path) -> Result<bool> {
    let mounted = PathBuf::from(format!("/proc/{pid}/root")).join(path.strip_prefix("/")?);
    let actual = std::fs::metadata(mounted)?;
    let expected = std::fs::metadata(path)?;
    Ok(actual.dev() == expected.dev() && actual.ino() == expected.ino())
}

fn ready(request: &Request, child: &Child) -> Result<bool> {
    request.validate()?;
    let pid = child.id();
    if std::fs::canonicalize(format!("/proc/{pid}/exe"))
        .ok()
        .as_deref()
        != Some(request.executable.as_path())
        || !mapped_file(pid, &request.sdl_library)?
    {
        return Ok(false);
    }
    ensure!(
        same_mounted_file(pid, &request.main_config())?
            && same_mounted_file(pid, &request.input_config())?,
        "Nestopia child does not see both private config files"
    );
    let entries = environment(pid)?;
    for (name, path) in [
        (b"HOME".as_slice(), request.private_root.join("home")),
        (
            b"XDG_CONFIG_HOME".as_slice(),
            request.private_root.join("config"),
        ),
        (
            b"XDG_CACHE_HOME".as_slice(),
            request.private_root.join("cache"),
        ),
        (
            b"XDG_STATE_HOME".as_slice(),
            request.private_root.join("state"),
        ),
        (b"XDG_DATA_HOME".as_slice(), request.data_home.clone()),
    ] {
        ensure!(
            has_exact_environment(&entries, name, path.as_os_str().as_encoded_bytes()),
            "Nestopia child does not select the prepared user roots"
        );
    }
    ensure!(
        has_exact_environment(&entries, b"FLTK_BACKEND", b"x11")
            && has_unique_nonempty_environment(&entries, b"DISPLAY")
            && has_no_environment(&entries, b"WAYLAND_DISPLAY")
            && has_exact_environment(&entries, b"SDL_LINUX_JOYSTICK_CLASSIC", b"1")
            && has_exact_environment(&entries, b"SDL_JOYSTICK_LINUX_CLASSIC", b"1"),
        "Nestopia child does not select the audited X11 and SDL backends"
    );
    let mut opened = std::collections::BTreeSet::new();
    for (index, entry) in std::fs::read_dir(format!("/proc/{pid}/fd"))?.enumerate() {
        ensure!(
            index < DESCRIPTOR_LIMIT,
            "Nestopia descriptor inventory exceeds limit"
        );
        match std::fs::metadata(entry?.path()) {
            Ok(metadata) => {
                opened.insert((metadata.dev(), metadata.ino(), metadata.rdev()));
            }
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => continue,
            Err(error) => return Err(error.into()),
        }
    }
    for path in &request.runtime_paths {
        let metadata = std::fs::metadata(path)?;
        if !opened.contains(&(metadata.dev(), metadata.ino(), metadata.rdev())) {
            return Ok(false);
        }
    }
    Ok(true)
}

fn write_receipt(request: &Request) -> Result<()> {
    let bytes = serde_json::to_vec(&request.expected_receipt())?;
    let mut file = OpenOptions::new()
        .write(true)
        .create_new(true)
        .mode(0o600)
        .open(&request.receipt)
        .context("Creating Nestopia readiness receipt")?;
    file.set_permissions(std::fs::Permissions::from_mode(0o600))?;
    file.write_all(&bytes)?;
    file.sync_all()?;
    Ok(())
}

fn run_request(request: Request) -> Result<()> {
    request.validate()?;
    ensure!(
        file_hash(&request.executable)?.eq_ignore_ascii_case(&request.executable_sha256)
            && file_hash(&request.sdl_library)?.eq_ignore_ascii_case(&request.sdl_library_sha256)
            && file_hash(&request.main_config())?.eq_ignore_ascii_case(&request.main_config_sha256)
            && file_hash(&request.input_config())?
                .eq_ignore_ascii_case(&request.input_config_sha256),
        "Nestopia target runtime or private config differs from request"
    );
    let startup_inventory = routing(crate::sdl2::inspect(&request.sdl_library)?);
    request.inventory.ensure_same_routing(&startup_inventory)?;
    let mut command = Command::new(&request.executable);
    command
        .arg(&request.content)
        .current_dir(&request.current_directory)
        .env("HOME", request.private_root.join("home"))
        .env("XDG_CONFIG_HOME", request.private_root.join("config"))
        .env("XDG_CACHE_HOME", request.private_root.join("cache"))
        .env("XDG_STATE_HOME", request.private_root.join("state"))
        .env("XDG_DATA_HOME", &request.data_home)
        .env("FLTK_BACKEND", "x11")
        .env("SDL_LINUX_JOYSTICK_CLASSIC", "1")
        .env("SDL_JOYSTICK_LINUX_CLASSIC", "1");
    let supervisor_pid = std::process::id();
    unsafe {
        command.pre_exec(move || {
            if libc::prctl(libc::PR_SET_PDEATHSIG, libc::SIGTERM) != 0 {
                return Err(std::io::Error::last_os_error());
            }
            if libc::getppid() != supervisor_pid as libc::pid_t {
                return Err(std::io::Error::new(
                    std::io::ErrorKind::BrokenPipe,
                    "Nestopia supervisor exited during child setup",
                ));
            }
            Ok(())
        });
    }
    let mut child = command.spawn().context("Starting supervised Nestopia")?;
    let startup = (|| {
        let deadline = Instant::now() + STARTUP_TIMEOUT;
        loop {
            ensure!(
                child.try_wait()?.is_none(),
                "Nestopia exited before controller handoff"
            );
            if ready(&request, &child)? {
                let handoff_inventory = routing(crate::sdl2::inspect(&request.sdl_library)?);
                request.inventory.ensure_same_routing(&handoff_inventory)?;
                ensure!(
                    file_hash(&request.executable)?
                        .eq_ignore_ascii_case(&request.executable_sha256)
                        && file_hash(&request.sdl_library)?
                            .eq_ignore_ascii_case(&request.sdl_library_sha256)
                        && file_hash(&request.main_config())?
                            .eq_ignore_ascii_case(&request.main_config_sha256)
                        && file_hash(&request.input_config())?
                            .eq_ignore_ascii_case(&request.input_config_sha256),
                    "Nestopia runtime or private config changed during startup"
                );
                write_receipt(&request)?;
                return Ok(());
            }
            ensure!(
                Instant::now() < deadline,
                "Nestopia did not establish controller routing before timeout"
            );
            std::thread::sleep(Duration::from_millis(25));
        }
    })();
    if let Err(error) = startup {
        let _ = child.kill();
        let _ = child.wait();
        return Err(error);
    }
    let status = child.wait()?;
    ensure!(status.success(), "Supervised Nestopia failed: {status}");
    Ok(())
}

pub fn run(path: &Path) -> Result<()> {
    run_request(read_request(path)?)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn exact_environment_matching_rejects_prefixes_and_duplicates() {
        assert!(has_exact_environment(
            &[b"XDG_CONFIG_HOME=/tmp/config".to_vec()],
            b"XDG_CONFIG_HOME",
            b"/tmp/config"
        ));
        assert!(!has_exact_environment(
            &[b"XDG_CONFIG_HOME=/tmp/config-extra".to_vec()],
            b"XDG_CONFIG_HOME",
            b"/tmp/config"
        ));
        assert!(!has_exact_environment(
            &[
                b"XDG_DATA_HOME=/tmp/data".to_vec(),
                b"XDG_DATA_HOME=/tmp/data".to_vec()
            ],
            b"XDG_DATA_HOME",
            b"/tmp/data"
        ));
        assert!(has_unique_nonempty_environment(
            &[b"DISPLAY=:0".to_vec()],
            b"DISPLAY"
        ));
        assert!(!has_unique_nonempty_environment(
            &[b"DISPLAY=".to_vec()],
            b"DISPLAY"
        ));
        assert!(!has_unique_nonempty_environment(
            &[b"DISPLAY=:0".to_vec(), b"DISPLAY=:1".to_vec()],
            b"DISPLAY"
        ));
        assert!(has_no_environment(
            &[b"DISPLAY=:0".to_vec()],
            b"WAYLAND_DISPLAY"
        ));
        assert!(!has_no_environment(
            &[b"WAYLAND_DISPLAY=".to_vec()],
            b"WAYLAND_DISPLAY"
        ));
    }
}
