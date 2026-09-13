//! Snes9x GTK launch supervisor for a target Flatpak sandbox.
//!
//! The ordinary controller inventory is run by the caller in this same target
//! runtime before it renders the private configuration. This mode rechecks that
//! exact routing, starts the pinned GTK executable, and emits a receipt only
//! after the child has loaded the expected SDL, selected the private config and
//! opened every selected kernel controller node.

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
    pub config_home: PathBuf,
    pub config_sha256: String,
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
    pub config_sha256: String,
    pub input_environment_sha256: String,
    pub runtime_paths: Vec<String>,
}

impl Request {
    pub fn expected_receipt(&self) -> Receipt {
        Receipt {
            schema_version: 1,
            executable_sha256: self.executable_sha256.clone(),
            sdl_library_sha256: self.sdl_library_sha256.clone(),
            config_sha256: self.config_sha256.clone(),
            input_environment_sha256: self.inventory.input_environment_sha256.clone(),
            runtime_paths: self.runtime_paths.clone(),
        }
    }

    pub fn validate(&self) -> Result<()> {
        ensure!(
            self.schema_version == 1,
            "Unknown Snes9x supervisor request"
        );
        for path in [
            &self.executable,
            &self.sdl_library,
            &self.config_home,
            &self.content,
            &self.current_directory,
            &self.receipt,
        ] {
            ensure!(
                path.is_absolute(),
                "Snes9x supervisor paths must be absolute"
            );
        }
        ensure!(
            self.executable == Path::new("/app/bin/snes9x-gtk"),
            "Snes9x supervisor only supports the pinned GTK Flatpak command"
        );
        ensure!(
            self.executable_sha256.len() == 64
                && self.sdl_library_sha256.len() == 64
                && self.config_sha256.len() == 64
                && self.inventory.input_environment_sha256.len() == 64,
            "Snes9x supervisor hashes are malformed"
        );
        ensure!(
            self.content.is_file() && self.current_directory.is_dir(),
            "Snes9x supervisor content or launch directory is absent"
        );
        let config = self.config_home.join("snes9x/snes9x.conf");
        ensure_mode(&self.config_home, 0o700, true, "Snes9x private root")?;
        for name in ["snes9x", "cache", "data", "state"] {
            ensure_mode(
                &self.config_home.join(name),
                0o700,
                true,
                "Snes9x private directory",
            )?;
        }
        ensure_mode(&config, 0o600, false, "Snes9x private config")?;
        ensure!(
            config.is_file()
                && self.receipt.parent() == Some(self.config_home.as_path())
                && self
                    .receipt
                    .file_name()
                    .is_some_and(|name| name == "ready.json")
                && !self.receipt.try_exists()?,
            "Snes9x supervisor staging layout is invalid or already used"
        );
        ensure!(
            !self.runtime_paths.is_empty()
                && self.runtime_paths.len() <= 5
                && self
                    .runtime_paths
                    .iter()
                    .all(|path| Path::new(path).parent() == Some(Path::new("/dev/input"))),
            "Snes9x supervisor needs one to five /dev/input controller paths"
        );
        let unique: std::collections::BTreeSet<_> = self.runtime_paths.iter().collect();
        ensure!(
            unique.len() == self.runtime_paths.len(),
            "Snes9x supervisor controller paths must be distinct"
        );
        ensure!(
            self.inventory.version[0] == 2
                && self.inventory.library == self.sdl_library
                && self.inventory.library_sha256 == self.sdl_library_sha256
                && self.inventory.devices.len() <= 10
                && self
                    .inventory
                    .devices
                    .iter()
                    .enumerate()
                    .all(|(index, device)| device.device_index as usize == index),
            "Snes9x supervisor inventory is not a complete target SDL2 routing snapshot"
        );
        for path in &self.runtime_paths {
            self.inventory.device_at_path(path)?;
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
        "Snes9x supervisor request path must be absolute"
    );
    ensure_mode(path, 0o600, false, "Snes9x supervisor request")?;
    let request: Request = serde_json::from_slice(&read_bounded(
        path,
        REQUEST_LIMIT,
        "Snes9x supervisor request",
    )?)
    .context("Invalid Snes9x supervisor request")?;
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
        "Snes9x child environment",
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

fn ready(request: &Request, child: &Child) -> Result<bool> {
    request.validate()?;
    let pid = child.id();
    if std::fs::canonicalize(format!("/proc/{pid}/exe"))
        .ok()
        .as_deref()
        != Some(request.executable.as_path())
    {
        return Ok(false);
    }
    if !mapped_file(pid, &request.sdl_library)? {
        return Ok(false);
    }
    let config = request.config_home.join("snes9x/snes9x.conf");
    let mounted = PathBuf::from(format!("/proc/{pid}/root")).join(config.strip_prefix("/")?);
    let actual = std::fs::metadata(mounted)?;
    let expected = std::fs::metadata(config)?;
    ensure!(
        actual.dev() == expected.dev() && actual.ino() == expected.ino(),
        "Snes9x child does not see the private snes9x.conf"
    );
    let entries = environment(pid)?;
    let config_bytes = request.config_home.as_os_str().as_encoded_bytes();
    ensure!(
        has_exact_environment(&entries, b"XDG_CONFIG_HOME", config_bytes),
        "Snes9x child environment does not select the private config"
    );
    for (name, suffix) in [
        (b"HOME".as_slice(), None),
        (b"XDG_CACHE_HOME".as_slice(), Some("cache")),
        (b"XDG_DATA_HOME".as_slice(), Some("data")),
        (b"XDG_STATE_HOME".as_slice(), Some("state")),
    ] {
        let path = suffix.map_or_else(
            || request.config_home.clone(),
            |suffix| request.config_home.join(suffix),
        );
        ensure!(
            has_exact_environment(&entries, name, path.as_os_str().as_encoded_bytes()),
            "Snes9x child environment does not isolate every writable user root"
        );
    }
    ensure!(
        has_exact_environment(&entries, b"SDL_LINUX_JOYSTICK_CLASSIC", b"1"),
        "Snes9x child environment does not select the probed SDL backend"
    );
    let mut opened = std::collections::BTreeSet::new();
    for (index, entry) in std::fs::read_dir(format!("/proc/{pid}/fd"))?.enumerate() {
        ensure!(
            index < DESCRIPTOR_LIMIT,
            "Snes9x descriptor inventory exceeds limit"
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
        .context("Creating Snes9x readiness receipt")?;
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
            && file_hash(&request.config_home.join("snes9x/snes9x.conf"))?
                .eq_ignore_ascii_case(&request.config_sha256),
        "Snes9x target executable, SDL or private config differs from the launch request"
    );
    let fresh = routing(crate::sdl2::inspect(&request.sdl_library)?);
    request.inventory.ensure_same_routing(&fresh)?;
    let mut command = Command::new(&request.executable);
    command
        .arg(&request.content)
        .current_dir(&request.current_directory)
        // Flatpak owns the sandbox XDG variables and rewrites an attempted
        // --env=XDG_CONFIG_HOME at the app boundary. Set the private roots in
        // this already-sandboxed supervisor immediately before exec instead.
        .env("HOME", &request.config_home)
        .env("XDG_CONFIG_HOME", &request.config_home)
        .env("XDG_CACHE_HOME", request.config_home.join("cache"))
        .env("XDG_DATA_HOME", request.config_home.join("data"))
        .env("XDG_STATE_HOME", request.config_home.join("state"))
        .env("SDL_LINUX_JOYSTICK_CLASSIC", "1");
    // If the Flatpak launcher or this supervisor dies, do not leave an
    // unowned emulator using a temporary config that Lunchbox can no longer
    // retain or verify.
    let supervisor_pid = std::process::id();
    unsafe {
        command.pre_exec(move || {
            if libc::prctl(libc::PR_SET_PDEATHSIG, libc::SIGTERM) != 0 {
                return Err(std::io::Error::last_os_error());
            }
            if libc::getppid() != supervisor_pid as libc::pid_t {
                return Err(std::io::Error::new(
                    std::io::ErrorKind::BrokenPipe,
                    "Snes9x supervisor exited during child setup",
                ));
            }
            Ok(())
        });
    }
    let mut child = command.spawn().context("Starting supervised Snes9x GTK")?;
    let result = (|| {
        let deadline = Instant::now() + STARTUP_TIMEOUT;
        loop {
            ensure!(
                child.try_wait()?.is_none(),
                "Snes9x exited before controller handoff"
            );
            if ready(&request, &child)? {
                let fresh = routing(crate::sdl2::inspect(&request.sdl_library)?);
                request.inventory.ensure_same_routing(&fresh)?;
                ensure!(
                    file_hash(&request.executable)?
                        .eq_ignore_ascii_case(&request.executable_sha256)
                        && file_hash(&request.sdl_library)?
                            .eq_ignore_ascii_case(&request.sdl_library_sha256)
                        && file_hash(&request.config_home.join("snes9x/snes9x.conf"))?
                            .eq_ignore_ascii_case(&request.config_sha256),
                    "Snes9x target runtime or private config changed during startup"
                );
                write_receipt(&request)?;
                return Ok(());
            }
            ensure!(
                Instant::now() < deadline,
                "Snes9x did not establish sandbox controller routing before timeout"
            );
            std::thread::sleep(Duration::from_millis(25));
        }
    })();
    if let Err(error) = result {
        let _ = child.kill();
        let _ = child.wait();
        return Err(error);
    }
    let status = child.wait()?;
    ensure!(status.success(), "Supervised Snes9x failed: {status}");
    Ok(())
}

pub fn run(path: &Path) -> Result<()> {
    run_request(read_request(path)?)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn exact_environment_matching_does_not_accept_prefixes_or_duplicates() {
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
                b"SDL_LINUX_JOYSTICK_CLASSIC=1".to_vec(),
                b"SDL_LINUX_JOYSTICK_CLASSIC=1".to_vec(),
            ],
            b"SDL_LINUX_JOYSTICK_CLASSIC",
            b"1"
        ));
    }
}
