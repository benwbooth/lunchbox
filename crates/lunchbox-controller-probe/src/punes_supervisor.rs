//! Target-sandbox evdev proof and supervisor for the audited puNES Flatpak.

use crate::file_hash;
use anyhow::{Context, Result, ensure};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::{
    collections::BTreeSet,
    fs::{File, OpenOptions},
    io::{Read, Write},
    os::{
        fd::AsRawFd,
        unix::{
            fs::{FileTypeExt, MetadataExt, OpenOptionsExt, PermissionsExt},
            process::CommandExt,
        },
    },
    path::{Path, PathBuf},
    process::{Child, Command},
    time::{Duration, Instant},
};

const APP_ID: &str = "io.github.punesemu.puNES";
const REQUEST_LIMIT: u64 = 8 * 1024 * 1024;
const ENVIRONMENT_LIMIT: u64 = 1024 * 1024;
const DESCRIPTOR_LIMIT: usize = 4096;
const INVENTORY_LIMIT: usize = 512;
const STARTUP_TIMEOUT: Duration = Duration::from_secs(20);
const BUS_VIRTUAL: u16 = 0x06;
const VENDOR: u16 = 0x1209;
const PRODUCT_BASE: u16 = 0x4c50;
const VERSION: u16 = 0x0001;
const REQUIRED_KEYS: [u16; 8] = [0x130, 0x131, 0x13a, 0x13b, 0x220, 0x221, 0x222, 0x223];

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct InputId {
    pub bustype: u16,
    pub vendor: u16,
    pub product: u16,
    pub version: u16,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Device {
    pub path: PathBuf,
    pub input_id: InputId,
    pub name: String,
    pub guid: String,
    pub key_codes: Vec<u16>,
    pub device: u64,
    pub inode: u64,
    pub rdev: u64,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Inventory {
    pub schema_version: u32,
    pub devices: Vec<Device>,
    pub sha256: String,
}

fn guid(id: InputId) -> String {
    format!(
        "{{{:04X}{:04X}-{:04X}-{:04X}-{:04X}-{:04X}{:04X}{:04X}}}",
        id.bustype.wrapping_sub(500),
        0u16.wrapping_sub(100),
        id.vendor,
        id.vendor.wrapping_sub(200),
        id.product,
        id.product.wrapping_sub(300),
        id.version,
        id.version.wrapping_sub(400)
    )
}

fn event_name(path: &Path) -> Result<&str> {
    ensure!(
        path.parent() == Some(Path::new("/dev/input")),
        "puNES target must be a /dev/input event node"
    );
    let name = path
        .file_name()
        .and_then(|name| name.to_str())
        .context("Invalid puNES event path")?;
    ensure!(
        name.strip_prefix("event").is_some_and(|index| {
            !index.is_empty() && index.bytes().all(|byte| byte.is_ascii_digit())
        }),
        "puNES target must be a /dev/input event node"
    );
    Ok(name)
}

fn ioctl_bytes(file: &File, operation: u32, bytes: &mut [u8], what: &str) -> Result<()> {
    let request = 0x8000_0000u32 | (u32::try_from(bytes.len())? << 16) | (0x45 << 8) | operation;
    ensure!(
        unsafe {
            libc::ioctl(
                file.as_raw_fd(),
                request as libc::c_ulong,
                bytes.as_mut_ptr(),
            )
        } >= 0,
        "Cannot read puNES {what}: {}",
        std::io::Error::last_os_error()
    );
    Ok(())
}

fn open_event(path: &Path) -> Result<File> {
    event_name(path)?;
    let file = OpenOptions::new()
        .read(true)
        .custom_flags(libc::O_NONBLOCK | libc::O_CLOEXEC | libc::O_NOFOLLOW)
        .open(path)?;
    let opened = file.metadata()?;
    let current = std::fs::symlink_metadata(path)?;
    ensure!(
        opened.file_type().is_char_device()
            && current.file_type().is_char_device()
            && opened.dev() == current.dev()
            && opened.ino() == current.ino()
            && opened.rdev() == current.rdev(),
        "puNES event node changed while opening"
    );
    Ok(file)
}

fn inspect_open(path: &Path, file: &File) -> Result<Device> {
    let metadata = file.metadata()?;
    let mut id = [0u8; 8];
    ioctl_bytes(file, 0x02, &mut id, "input identity")?; // EVIOCGID
    let input_id = InputId {
        bustype: u16::from_ne_bytes([id[0], id[1]]),
        vendor: u16::from_ne_bytes([id[2], id[3]]),
        product: u16::from_ne_bytes([id[4], id[5]]),
        version: u16::from_ne_bytes([id[6], id[7]]),
    };
    let mut name = [0u8; 256];
    ioctl_bytes(file, 0x06, &mut name, "device name")?; // EVIOCGNAME
    let end = name
        .iter()
        .position(|byte| *byte == 0)
        .context("Unterminated puNES device name")?;
    let name = std::str::from_utf8(&name[..end])?.to_owned();
    let mut keys = [0u8; 96];
    ioctl_bytes(file, 0x21, &mut keys, "key capabilities")?; // EVIOCGBIT(EV_KEY)
    let key_codes = (0..0x300u16)
        .filter(|code| keys[usize::from(*code) / 8] & (1 << (*code % 8)) != 0)
        .collect();
    Ok(Device {
        path: path.to_path_buf(),
        input_id,
        name,
        guid: guid(input_id),
        key_codes,
        device: metadata.dev(),
        inode: metadata.ino(),
        rdev: metadata.rdev(),
    })
}

fn validate_target(device: &Device, player: u8) -> Result<()> {
    ensure!((1..=2).contains(&player), "puNES target player is invalid");
    event_name(&device.path)?;
    let expected = InputId {
        bustype: BUS_VIRTUAL,
        vendor: VENDOR,
        product: PRODUCT_BASE + u16::from(player),
        version: VERSION,
    };
    ensure!(
        device.input_id == expected
            && device.name == format!("Lunchbox puNES target P{player}")
            && device.guid == guid(expected)
            && device.key_codes == REQUIRED_KEYS,
        "puNES target P{player} identity or capability set changed"
    );
    Ok(())
}

fn inventory_hash(devices: &[Device]) -> Result<String> {
    let bytes = serde_json::to_vec(devices)?;
    Ok(hex_string(&Sha256::digest(bytes)))
}

fn hex_string(bytes: &[u8]) -> String {
    const HEX: &[u8; 16] = b"0123456789abcdef";
    let mut output = String::with_capacity(bytes.len() * 2);
    for byte in bytes {
        output.push(HEX[usize::from(byte >> 4)] as char);
        output.push(HEX[usize::from(byte & 0x0f)] as char);
    }
    output
}

pub fn inspect_targets() -> Result<Inventory> {
    let mut devices = Vec::new();
    for (index, entry) in std::fs::read_dir("/dev/input")?.enumerate() {
        ensure!(
            index < INVENTORY_LIMIT,
            "puNES input inventory exceeds limit"
        );
        let path = entry?.path();
        if event_name(&path).is_err() {
            continue;
        }
        let Ok(file) = open_event(&path) else {
            continue;
        };
        let Ok(device) = inspect_open(&path, &file) else {
            continue;
        };
        if device.input_id.vendor == VENDOR
            && (device.input_id.product == PRODUCT_BASE + 1
                || device.input_id.product == PRODUCT_BASE + 2)
        {
            devices.push(device);
        }
    }
    devices.sort_by(|left, right| left.path.cmp(&right.path));
    let sha256 = inventory_hash(&devices)?;
    Ok(Inventory {
        schema_version: 1,
        devices,
        sha256,
    })
}

impl Inventory {
    pub fn validate(&self, players: usize) -> Result<()> {
        ensure!(
            self.schema_version == 1 && matches!(players, 1 | 2),
            "Unknown puNES target inventory"
        );
        ensure!(
            self.devices.len() == players,
            "puNES target inventory is incomplete or ambiguous"
        );
        ensure!(
            inventory_hash(&self.devices)? == self.sha256,
            "puNES target inventory hash changed"
        );
        let mut paths = BTreeSet::new();
        for player in 1..=players {
            let matches = self
                .devices
                .iter()
                .filter(|device| device.input_id.product == PRODUCT_BASE + player as u16)
                .collect::<Vec<_>>();
            ensure!(matches.len() == 1, "puNES target P{player} is not unique");
            validate_target(matches[0], player as u8)?;
            ensure!(
                paths.insert(&matches[0].path),
                "puNES target paths are duplicated"
            );
        }
        Ok(())
    }

    pub fn device_for_player(&self, player: u8) -> Result<&Device> {
        self.devices
            .iter()
            .find(|device| device.input_id.product == PRODUCT_BASE + u16::from(player))
            .context("puNES target player is absent")
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ConfigFile {
    pub path: PathBuf,
    pub sha256: String,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Request {
    pub schema_version: u32,
    pub executable: PathBuf,
    pub executable_sha256: String,
    pub libudev: PathBuf,
    pub libudev_sha256: String,
    pub private_root: PathBuf,
    pub configs: Vec<ConfigFile>,
    pub data_home: PathBuf,
    pub content: PathBuf,
    pub current_directory: PathBuf,
    pub inventory: Inventory,
    pub receipt: PathBuf,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Receipt {
    pub schema_version: u32,
    pub executable_sha256: String,
    pub libudev_sha256: String,
    pub config_sha256: Vec<String>,
    pub data_home: PathBuf,
    pub input_environment_sha256: String,
}

fn hash(value: &str) -> bool {
    value.len() == 64 && value.bytes().all(|byte| byte.is_ascii_hexdigit())
}

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

impl Request {
    pub fn expected_receipt(&self) -> Receipt {
        Receipt {
            schema_version: 1,
            executable_sha256: self.executable_sha256.clone(),
            libudev_sha256: self.libudev_sha256.clone(),
            config_sha256: self
                .configs
                .iter()
                .map(|file| file.sha256.clone())
                .collect(),
            data_home: self.data_home.clone(),
            input_environment_sha256: self.inventory.sha256.clone(),
        }
    }

    pub fn validate(&self) -> Result<()> {
        ensure!(self.schema_version == 1, "Unknown puNES supervisor request");
        for path in [
            &self.executable,
            &self.libudev,
            &self.private_root,
            &self.data_home,
            &self.content,
            &self.current_directory,
            &self.receipt,
        ] {
            ensure!(
                path.is_absolute(),
                "puNES supervisor paths must be absolute"
            );
        }
        ensure!(
            self.executable == Path::new("/app/bin/punes"),
            "puNES supervisor supports only the audited Flatpak command"
        );
        ensure!(
            self.libudev == Path::new("/usr/lib/x86_64-linux-gnu/libudev.so.1.7.10"),
            "puNES supervisor requires the audited runtime libudev"
        );
        ensure!(
            hash(&self.executable_sha256) && hash(&self.libudev_sha256),
            "puNES supervisor hash is malformed"
        );
        self.inventory.validate(self.inventory.devices.len())?;
        ensure_mode(&self.private_root, 0o700, true, "puNES private root")?;
        for relative in ["home", "config", "cache", "state"] {
            ensure_mode(
                &self.private_root.join(relative),
                0o700,
                true,
                "puNES private directory",
            )?;
        }
        for relative in [
            "config/puNES",
            "config/puNES/cheat",
            "config/puNES/jsc",
            "config/puNES/pgs",
            "config/puNES/shp",
        ] {
            ensure_mode(
                &self.private_root.join(relative),
                0o700,
                true,
                "puNES private config directory",
            )?;
        }
        ensure!(
            self.configs.len() == self.inventory.devices.len() + 2,
            "puNES supervisor needs main, input and one JSC per target"
        );
        let mut config_paths = BTreeSet::new();
        let config_root = self.private_root.join("config/puNES");
        let jsc_root = config_root.join("jsc");
        for config in &self.configs {
            let direct = config.path.parent() == Some(config_root.as_path())
                && config
                    .path
                    .file_name()
                    .is_some_and(|name| name == "puNES.cfg" || name == "input.cfg");
            let jsc = config.path.parent() == Some(jsc_root.as_path())
                && config
                    .path
                    .file_name()
                    .and_then(|name| name.to_str())
                    .and_then(|name| name.strip_suffix(".jsc"))
                    .is_some_and(|stem| {
                        stem.len() == 32 && stem.bytes().all(|byte| byte.is_ascii_hexdigit())
                    });
            ensure!(
                (direct || jsc) && config_paths.insert(&config.path) && hash(&config.sha256),
                "puNES private config layout or hash is invalid"
            );
            ensure_mode(&config.path, 0o600, false, "puNES private config")?;
        }
        ensure!(
            self.configs
                .iter()
                .any(|file| file.path.ends_with("puNES.cfg"))
                && self
                    .configs
                    .iter()
                    .any(|file| file.path.ends_with("input.cfg")),
            "puNES main or input config is absent"
        );
        for device in &self.inventory.devices {
            let jsc = device
                .guid
                .chars()
                .filter(|ch| ch.is_ascii_hexdigit())
                .collect::<String>()
                + ".jsc";
            let relative = Path::new("jsc").join(jsc);
            ensure!(
                self.configs
                    .iter()
                    .any(|file| file.path.ends_with(&relative)),
                "puNES target JSC is absent"
            );
        }
        ensure!(
            self.data_home.is_dir()
                && self
                    .data_home
                    .ends_with(Path::new(".var/app").join(APP_ID).join("data"))
                && !self.data_home.starts_with(&self.private_root),
            "puNES persistent data home is outside the audited Flatpak profile"
        );
        ensure!(
            self.content.is_file()
                && self
                    .content
                    .extension()
                    .and_then(|value| value.to_str())
                    .is_some_and(|extension| {
                        matches!(
                            extension.to_ascii_lowercase().as_str(),
                            "nes" | "unf" | "unif"
                        )
                    })
                && self.content.parent().is_some_and(|parent| {
                    parent.canonicalize().ok().as_ref()
                        == self.current_directory.canonicalize().ok().as_ref()
                }),
            "puNES standard-pad content or launch directory is invalid"
        );
        ensure!(
            self.receipt.parent() == Some(self.private_root.as_path())
                && self
                    .receipt
                    .file_name()
                    .is_some_and(|name| name == "ready.json")
                && !self.receipt.try_exists()?,
            "puNES readiness receipt layout is invalid or already used"
        );
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
        "puNES supervisor request path must be absolute"
    );
    ensure_mode(path, 0o600, false, "puNES supervisor request")?;
    let request: Request = serde_json::from_slice(&read_bounded(
        path,
        REQUEST_LIMIT,
        "puNES supervisor request",
    )?)
    .context("Invalid puNES supervisor request")?;
    request.validate()?;
    Ok(request)
}

fn environment(pid: u32) -> Result<Vec<Vec<u8>>> {
    Ok(read_bounded(
        Path::new(&format!("/proc/{pid}/environ")),
        ENVIRONMENT_LIMIT,
        "puNES child environment",
    )?
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
    let matches = entries
        .iter()
        .filter_map(|entry| entry.strip_prefix(prefix.as_slice()))
        .collect::<Vec<_>>();
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
        || !mapped_file(pid, &request.libudev)?
    {
        return Ok(false);
    }
    for config in &request.configs {
        ensure!(
            same_mounted_file(pid, &config.path)?,
            "puNES child does not see a private config file"
        );
    }
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
            "puNES child does not select the prepared user roots"
        );
    }
    ensure!(
        has_exact_environment(&entries, b"QT_QPA_PLATFORM", b"xcb")
            && has_unique_nonempty_environment(&entries, b"DISPLAY")
            && has_no_environment(&entries, b"WAYLAND_DISPLAY"),
        "puNES child does not use the exact X11 display environment"
    );
    let mut opened = BTreeSet::new();
    for (index, entry) in std::fs::read_dir(format!("/proc/{pid}/fd"))?.enumerate() {
        ensure!(
            index < DESCRIPTOR_LIMIT,
            "puNES descriptor inventory exceeds limit"
        );
        match std::fs::metadata(entry?.path()) {
            Ok(metadata) => {
                opened.insert((metadata.dev(), metadata.ino(), metadata.rdev()));
            }
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => continue,
            Err(error) => return Err(error.into()),
        }
    }
    for device in &request.inventory.devices {
        if !opened.contains(&(device.device, device.inode, device.rdev)) {
            return Ok(false);
        }
    }
    Ok(true)
}

fn verify_inputs(request: &Request) -> Result<()> {
    ensure!(
        file_hash(&request.executable)?.eq_ignore_ascii_case(&request.executable_sha256),
        "puNES executable differs from request"
    );
    ensure!(
        file_hash(&request.libudev)?.eq_ignore_ascii_case(&request.libudev_sha256),
        "puNES libudev differs from request"
    );
    for config in &request.configs {
        ensure!(
            file_hash(&config.path)?.eq_ignore_ascii_case(&config.sha256),
            "puNES private config differs from request: {}",
            config.path.display()
        );
    }
    ensure!(
        inspect_targets()? == request.inventory,
        "puNES target input inventory differs from request"
    );
    Ok(())
}

fn write_receipt(request: &Request) -> Result<()> {
    let bytes = serde_json::to_vec(&request.expected_receipt())?;
    let mut file = OpenOptions::new()
        .write(true)
        .create_new(true)
        .mode(0o600)
        .open(&request.receipt)
        .context("Creating puNES readiness receipt")?;
    file.set_permissions(std::fs::Permissions::from_mode(0o600))?;
    file.write_all(&bytes)?;
    file.sync_all()?;
    Ok(())
}

fn run_request(request: Request) -> Result<()> {
    request.validate()?;
    verify_inputs(&request)?;
    let held = request
        .inventory
        .devices
        .iter()
        .map(|device| {
            let file = open_event(&device.path)?;
            ensure!(
                inspect_open(&device.path, &file)? == *device,
                "puNES target changed while opening"
            );
            Ok(file)
        })
        .collect::<Result<Vec<_>>>()?;
    let mut command = Command::new(&request.executable);
    command
        .arg(&request.content)
        .current_dir(&request.current_directory)
        .env("HOME", request.private_root.join("home"))
        .env("XDG_CONFIG_HOME", request.private_root.join("config"))
        .env("XDG_CACHE_HOME", request.private_root.join("cache"))
        .env("XDG_STATE_HOME", request.private_root.join("state"))
        .env("XDG_DATA_HOME", &request.data_home);
    let supervisor_pid = std::process::id();
    unsafe {
        command.pre_exec(move || {
            if libc::prctl(libc::PR_SET_PDEATHSIG, libc::SIGTERM) != 0 {
                return Err(std::io::Error::last_os_error());
            }
            if libc::getppid() != supervisor_pid as libc::pid_t {
                return Err(std::io::Error::new(
                    std::io::ErrorKind::BrokenPipe,
                    "puNES supervisor exited during child setup",
                ));
            }
            Ok(())
        });
    }
    let mut child = command.spawn().context("Starting supervised puNES")?;
    let startup = (|| {
        let deadline = Instant::now() + STARTUP_TIMEOUT;
        loop {
            ensure!(
                child.try_wait()?.is_none(),
                "puNES exited before controller handoff"
            );
            if ready(&request, &child)? {
                verify_inputs(&request)?;
                write_receipt(&request)?;
                return Ok(());
            }
            ensure!(
                Instant::now() < deadline,
                "puNES did not establish target routing before timeout"
            );
            std::thread::sleep(Duration::from_millis(25));
        }
    })();
    drop(held);
    if let Err(error) = startup {
        let _ = child.kill();
        let _ = child.wait();
        return Err(error);
    }
    let status = child.wait()?;
    ensure!(status.success(), "Supervised puNES failed: {status}");
    Ok(())
}

pub fn run(path: &Path) -> Result<()> {
    run_request(read_request(path)?)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn write_mode(path: &Path, bytes: &[u8], mode: u32) {
        std::fs::write(path, bytes).unwrap();
        std::fs::set_permissions(path, std::fs::Permissions::from_mode(mode)).unwrap();
    }

    fn request_fixture() -> (tempfile::TempDir, Request) {
        let directory = tempfile::tempdir().unwrap();
        let private_root = directory.path().join("private");
        for relative in [
            "home",
            "config",
            "config/puNES",
            "config/puNES/cheat",
            "config/puNES/jsc",
            "config/puNES/pgs",
            "config/puNES/shp",
            "cache",
            "state",
        ] {
            let path = private_root.join(relative);
            std::fs::create_dir_all(&path).unwrap();
            std::fs::set_permissions(path, std::fs::Permissions::from_mode(0o700)).unwrap();
        }
        std::fs::set_permissions(&private_root, std::fs::Permissions::from_mode(0o700)).unwrap();

        let input_id = InputId {
            bustype: BUS_VIRTUAL,
            vendor: VENDOR,
            product: PRODUCT_BASE + 1,
            version: VERSION,
        };
        let device = Device {
            path: "/dev/input/event999999".into(),
            input_id,
            name: "Lunchbox puNES target P1".into(),
            guid: guid(input_id),
            key_codes: REQUIRED_KEYS.to_vec(),
            device: 1,
            inode: 2,
            rdev: 3,
        };
        let inventory = Inventory {
            schema_version: 1,
            sha256: inventory_hash(std::slice::from_ref(&device)).unwrap(),
            devices: vec![device],
        };
        let config_root = private_root.join("config/puNES");
        let paths = [
            config_root.join("puNES.cfg"),
            config_root.join("input.cfg"),
            config_root
                .join("jsc")
                .join("FE12FF9C120911414C514B250001FE71.jsc"),
        ];
        for path in &paths {
            write_mode(path, b"fixture", 0o600);
        }
        let data_home = directory
            .path()
            .join("profile/.var/app")
            .join(APP_ID)
            .join("data");
        std::fs::create_dir_all(&data_home).unwrap();
        let content = directory.path().join("content/oracle.nes");
        std::fs::create_dir_all(content.parent().unwrap()).unwrap();
        std::fs::write(&content, b"NES fixture").unwrap();
        let request = Request {
            schema_version: 1,
            executable: "/app/bin/punes".into(),
            executable_sha256: "a".repeat(64),
            libudev: "/usr/lib/x86_64-linux-gnu/libudev.so.1.7.10".into(),
            libudev_sha256: "b".repeat(64),
            private_root: private_root.clone(),
            configs: paths
                .into_iter()
                .map(|path| ConfigFile {
                    path,
                    sha256: "c".repeat(64),
                })
                .collect(),
            data_home,
            current_directory: content.parent().unwrap().to_path_buf(),
            content,
            inventory,
            receipt: private_root.join("ready.json"),
        };
        (directory, request)
    }

    #[test]
    fn target_guid_and_capabilities_are_exact() {
        let id = InputId {
            bustype: BUS_VIRTUAL,
            vendor: VENDOR,
            product: PRODUCT_BASE + 1,
            version: VERSION,
        };
        assert_eq!(guid(id), "{FE12FF9C-1209-1141-4C51-4B250001FE71}");
        assert_eq!(
            REQUIRED_KEYS,
            [0x130, 0x131, 0x13a, 0x13b, 0x220, 0x221, 0x222, 0x223]
        );
    }

    #[test]
    fn exact_environment_rejects_prefixes_and_duplicates() {
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
            &[b"HOME=/tmp/home".to_vec(), b"HOME=/tmp/home".to_vec()],
            b"HOME",
            b"/tmp/home"
        ));
        assert!(has_unique_nonempty_environment(
            &[b"DISPLAY=:152".to_vec()],
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
            &[b"DISPLAY=:152".to_vec()],
            b"WAYLAND_DISPLAY"
        ));
        assert!(!has_no_environment(
            &[b"WAYLAND_DISPLAY=".to_vec()],
            b"WAYLAND_DISPLAY"
        ));
    }

    #[test]
    fn request_accepts_only_exact_private_config_layout() {
        let (_directory, request) = request_fixture();
        request.validate().unwrap();

        let readonly = request.clone();
        std::fs::set_permissions(
            readonly.private_root.join("config/puNES"),
            std::fs::Permissions::from_mode(0o500),
        )
        .unwrap();
        assert!(readonly.validate().is_err());
        std::fs::set_permissions(
            readonly.private_root.join("config/puNES"),
            std::fs::Permissions::from_mode(0o700),
        )
        .unwrap();

        let mut misplaced = request.clone();
        let wrong = misplaced.private_root.join("config/puNES-extra/input.cfg");
        std::fs::create_dir_all(wrong.parent().unwrap()).unwrap();
        write_mode(&wrong, b"fixture", 0o600);
        misplaced.configs[1].path = wrong;
        assert!(misplaced.validate().is_err());

        let mut wrong_jsc = request;
        wrong_jsc.configs[2].path = wrong_jsc
            .private_root
            .join("config/puNES/jsc/not-a-guid.jsc");
        write_mode(&wrong_jsc.configs[2].path, b"fixture", 0o600);
        assert!(wrong_jsc.validate().is_err());
    }
}
