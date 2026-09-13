//! Narrow, user-supplied Famicom Disk System firmware installation for the
//! Nestopia UE Flatpak. This code never discovers or downloads firmware.

use std::ffi::{CStr, CString};
use std::fs::{self, File, OpenOptions};
use std::io::{ErrorKind, Read, Write};
#[cfg(target_os = "linux")]
use std::os::fd::{AsRawFd, FromRawFd};
#[cfg(target_os = "linux")]
use std::os::unix::fs::{MetadataExt, OpenOptionsExt};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};

use anyhow::{Context, Result, bail, ensure};
use sha2::{Digest, Sha256};

pub const NESTOPIA_UE_FLATPAK_ID: &str = "ca._0ldsk00l.Nestopia";
pub const NESTOPIA_UE_EMULATOR_ID: &str = "73ad4eb8-0f5f-56ca-b839-8398db3e8d77";
pub const NESTOPIA_UE_EMULATOR_NAME: &str = "Nestopia UE";
pub const FDS_PLATFORM_ID: &str = "d01f03eb-cbf9-5847-92a6-f5fb9ba80b15";
pub const FDS_PLATFORM_NAME: &str = "Nintendo Famicom Disk System";
pub const FDS_BIOS_FILENAME: &str = "disksys.rom";
pub const FDS_BIOS_BYTES: usize = 8_192;
// Primary-source allowlist: SourMesen/Mesen2
// UI/Interop/FirmwareTypeExtensions.cs @ b9fa69ddc6d0a331fb103fdb5eef6904305703c2.
pub const RECOGNIZED_FDS_BIOS_SHA256: [&str; 2] = [
    "99c18490ed9002d9c6d999b9d8d15be5c051bdfa7cc7e73318053c9a994b0178",
    "a0a9d57cbace21bf9c85c2b85e86656317f0768d7772acc90c7411ab1dbff2bf",
];
/// Diagnostic values only; CRC32 is never an acceptance criterion.
pub const RECOGNIZED_FDS_BIOS_CRC32: [u32; 2] = [0x5e60_7dcf, 0x4df2_4a6c];
pub const RUNTIME_PROOF_BLOCKER: &str = "Runtime proof requires a lawfully obtained FDS BIOS and FDS game image; Lunchbox does not discover or download either file.";

static TEMP_SEQUENCE: AtomicU64 = AtomicU64::new(0);

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct FirmwareFingerprint {
    pub crc32: u32,
    pub sha256: String,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ValidatedFdsBios {
    bytes: [u8; FDS_BIOS_BYTES],
    pub fingerprint: FirmwareFingerprint,
}

impl ValidatedFdsBios {
    pub fn bytes(&self) -> &[u8; FDS_BIOS_BYTES] {
        &self.bytes
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum InstallDisposition {
    Installed,
    AlreadyPresent,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct InstallReceipt {
    pub disposition: InstallDisposition,
    pub target: PathBuf,
    pub fingerprint: FirmwareFingerprint,
}

#[derive(Clone, Copy, Debug)]
pub struct FdsRuntimeScope<'a> {
    pub flatpak_app_id: &'a str,
    pub platform_id: &'a str,
    pub platform_name: &'a str,
    pub content_path: &'a Path,
    pub flatpak_data_home: &'a Path,
}

#[derive(Clone, Copy, Debug)]
pub struct InstallRequest<'a> {
    pub flatpak_app_id: &'a str,
    pub platform_id: &'a str,
    pub platform_name: &'a str,
    pub content_path: &'a Path,
    pub flatpak_data_home: &'a Path,
    pub firmware_source: &'a Path,
}

impl<'a> InstallRequest<'a> {
    fn runtime_scope(&self) -> FdsRuntimeScope<'a> {
        FdsRuntimeScope {
            flatpak_app_id: self.flatpak_app_id,
            platform_id: self.platform_id,
            platform_name: self.platform_name,
            content_path: self.content_path,
            flatpak_data_home: self.flatpak_data_home,
        }
    }
}

/// Validate an arbitrary user-selected source. This does not establish runtime
/// readiness at Nestopia's installed target.
pub fn inspect_fds_bios(path: &Path) -> Result<ValidatedFdsBios> {
    #[cfg(not(target_os = "linux"))]
    {
        let _ = path;
        bail!("Nestopia UE Flatpak firmware inspection is supported only on Linux");
    }
    #[cfg(target_os = "linux")]
    {
        Ok(inspect_source(path, &RECOGNIZED_FDS_BIOS_SHA256)?.image)
    }
}

pub fn install_fds_firmware(request: InstallRequest<'_>) -> Result<InstallReceipt> {
    #[cfg(not(target_os = "linux"))]
    {
        let _ = request;
        bail!("Nestopia UE Flatpak firmware installation is supported only on Linux");
    }
    #[cfg(target_os = "linux")]
    {
        install_with_policy(
            request,
            &current_home()?,
            &RECOGNIZED_FDS_BIOS_SHA256,
            euid(),
            |_| {},
        )
    }
}

/// Freshly re-open and validate the installed file immediately before launch.
pub fn verify_fds_firmware_for_launch(scope: FdsRuntimeScope<'_>) -> Result<InstallReceipt> {
    #[cfg(not(target_os = "linux"))]
    {
        let _ = scope;
        bail!("Nestopia UE Flatpak firmware verification is supported only on Linux");
    }
    #[cfg(target_os = "linux")]
    {
        verify_with_policy(scope, &current_home()?, &RECOGNIZED_FDS_BIOS_SHA256, euid())
    }
}

#[cfg(target_os = "linux")]
fn current_home() -> Result<PathBuf> {
    let base = directories::BaseDirs::new().context("finding the current user's home")?;
    fs::canonicalize(base.home_dir())
        .with_context(|| format!("resolving current user home {}", base.home_dir().display()))
}

#[cfg(target_os = "linux")]
fn euid() -> u32 {
    // SAFETY: geteuid has no preconditions.
    unsafe { libc::geteuid() }
}

#[cfg(target_os = "linux")]
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum InstallPhase {
    BeforeLink,
    AfterLink,
}

#[cfg(all(test, target_os = "linux"))]
pub(crate) fn inspect_fds_bios_for_test(
    path: &Path,
    accepted_sha256: &[String],
) -> Result<ValidatedFdsBios> {
    let allowed: Vec<&str> = accepted_sha256.iter().map(String::as_str).collect();
    Ok(inspect_source(path, &allowed)?.image)
}

#[cfg(all(test, target_os = "linux"))]
pub(crate) fn install_fds_firmware_for_test<F>(
    request: InstallRequest<'_>,
    home: &Path,
    accepted_sha256: &[String],
    expected_uid: u32,
    hook: F,
) -> Result<InstallReceipt>
where
    F: FnMut(InstallPhase),
{
    let allowed: Vec<&str> = accepted_sha256.iter().map(String::as_str).collect();
    install_with_policy(request, home, &allowed, expected_uid, hook)
}

#[cfg(all(test, target_os = "linux"))]
pub(crate) fn verify_fds_firmware_for_test(
    scope: FdsRuntimeScope<'_>,
    home: &Path,
    accepted_sha256: &[String],
    expected_uid: u32,
) -> Result<InstallReceipt> {
    let allowed: Vec<&str> = accepted_sha256.iter().map(String::as_str).collect();
    verify_with_policy(scope, home, &allowed, expected_uid)
}

#[cfg(target_os = "linux")]
fn install_with_policy<F>(
    request: InstallRequest<'_>,
    home: &Path,
    allowed: &[&str],
    expected_uid: u32,
    mut hook: F,
) -> Result<InstallReceipt>
where
    F: FnMut(InstallPhase),
{
    let scope = request.runtime_scope();
    validate_scope(scope)?;
    let data = open_data_home(scope.flatpak_data_home, home, expected_uid)?;
    let source = inspect_source(request.firmware_source, allowed)
        .with_context(|| format!("validating {}", request.firmware_source.display()))?;
    let target_dir = open_or_create_target_dir(&data, expected_uid)?;
    let target_path = target_dir.path.join(FDS_BIOS_FILENAME);
    let target_name = cstr(FDS_BIOS_FILENAME)?;

    match stat_at(&target_dir.file, &target_name) {
        Ok(_) => {
            return existing_receipt(
                &target_dir,
                &target_name,
                &target_path,
                &source.image,
                allowed,
                expected_uid,
            );
        }
        Err(error) if error.kind() == ErrorKind::NotFound => {}
        Err(error) => return Err(error).context("inspecting existing Nestopia BIOS target"),
    }

    let (temp_name, mut temp_file) = create_temp_at(&target_dir.file)?;
    let mut temp = RelativeTemp::new(&target_dir.file, temp_name);
    temp_file
        .write_all(source.image.bytes())
        .context("writing temporary FDS BIOS")?;
    temp_file.sync_all().context("syncing temporary FDS BIOS")?;
    drop(temp_file);
    let staged = inspect_at(&target_dir.file, temp.name(), allowed)
        .context("rechecking temporary FDS BIOS")?;
    ensure!(
        staged.image.fingerprint == source.image.fingerprint,
        "temporary FDS BIOS changed before installation"
    );
    ensure_installed_constraints(&staged, expected_uid, true, "temporary FDS BIOS")?;
    ensure_named_directory(&target_dir, expected_uid)?;
    ensure_named_directory(&data, expected_uid)?;

    hook(InstallPhase::BeforeLink);
    match link_at(
        &target_dir.file,
        temp.name(),
        &target_dir.file,
        &target_name,
    ) {
        Ok(()) => {}
        Err(error) if error.kind() == ErrorKind::AlreadyExists => {
            let result = existing_receipt(
                &target_dir,
                &target_name,
                &target_path,
                &source.image,
                allowed,
                expected_uid,
            );
            temp.remove().context("removing temporary FDS BIOS")?;
            target_dir
                .file
                .sync_all()
                .context("syncing Nestopia data directory")?;
            return result;
        }
        Err(error) => return Err(error).context("atomically installing Nestopia FDS BIOS"),
    }

    // Establish which inode this successful link operation actually published.
    // If a same-UID process exchanged the temporary name before `linkat`, use
    // the linked inode (not the formerly inspected one) for safe rollback.
    let linked_identity =
        match linked_identity_after_link(&target_dir.file, temp.name(), &target_name) {
            Ok(identity) => identity,
            Err(error) => {
                rollback_if_identity(&target_dir.file, &target_name, staged.identity);
                return Err(error);
            }
        };
    hook(InstallPhase::AfterLink);
    let result = (|| -> Result<InstallReceipt> {
        let linked = inspect_at(&target_dir.file, &target_name, allowed)
            .context("rechecking installed FDS BIOS")?;
        ensure!(
            linked.identity == linked_identity
                && linked.image.fingerprint == source.image.fingerprint,
            "installed FDS BIOS identity changed during atomic installation"
        );
        // The temporary name is still the second link at this point.
        ensure_installed_constraints(&linked, expected_uid, false, "installed FDS BIOS")?;
        ensure_named_directory(&target_dir, expected_uid)?;
        ensure_named_directory(&data, expected_uid)?;
        temp.remove().context("removing temporary FDS BIOS")?;
        target_dir
            .file
            .sync_all()
            .context("syncing Nestopia data directory")?;

        let installed = inspect_at(&target_dir.file, &target_name, allowed)
            .context("finalizing installed FDS BIOS")?;
        ensure!(
            installed.identity == linked_identity
                && installed.image.fingerprint == source.image.fingerprint,
            "installed FDS BIOS identity changed while installation was finalized"
        );
        ensure_installed_constraints(&installed, expected_uid, true, "installed FDS BIOS")?;
        ensure_named_directory(&target_dir, expected_uid)?;
        ensure_named_directory(&data, expected_uid)?;
        Ok(InstallReceipt {
            disposition: InstallDisposition::Installed,
            target: target_path,
            fingerprint: installed.image.fingerprint,
        })
    })();
    if result.is_err() {
        rollback_if_identity(&target_dir.file, &target_name, linked_identity);
    }
    result
}

#[cfg(target_os = "linux")]
fn verify_with_policy(
    scope: FdsRuntimeScope<'_>,
    home: &Path,
    allowed: &[&str],
    expected_uid: u32,
) -> Result<InstallReceipt> {
    validate_scope(scope)?;
    let data = open_data_home(scope.flatpak_data_home, home, expected_uid)?;
    let target_dir = open_target_dir(&data, expected_uid)?;
    let target_path = target_dir.path.join(FDS_BIOS_FILENAME);
    let target_name = cstr(FDS_BIOS_FILENAME)?;
    let installed = inspect_at(&target_dir.file, &target_name, allowed).with_context(|| {
        format!(
            "validating installed Nestopia BIOS {}",
            target_path.display()
        )
    })?;
    ensure_installed_constraints(&installed, expected_uid, true, "installed Nestopia BIOS")?;
    ensure_named_directory(&target_dir, expected_uid)?;
    ensure_named_directory(&data, expected_uid)?;
    Ok(InstallReceipt {
        disposition: InstallDisposition::AlreadyPresent,
        target: target_path,
        fingerprint: installed.image.fingerprint,
    })
}

#[cfg(target_os = "linux")]
fn validate_scope(scope: FdsRuntimeScope<'_>) -> Result<()> {
    ensure!(
        scope.flatpak_app_id == NESTOPIA_UE_FLATPAK_ID,
        "FDS firmware is confined to the exact {NESTOPIA_UE_FLATPAK_ID} Flatpak"
    );
    ensure!(
        scope.platform_id == FDS_PLATFORM_ID && scope.platform_name == FDS_PLATFORM_NAME,
        "FDS firmware is confined to platform {FDS_PLATFORM_ID} ({FDS_PLATFORM_NAME})"
    );
    validate_content(scope.content_path)
}

#[cfg(target_os = "linux")]
fn validate_content(path: &Path) -> Result<()> {
    ensure!(path.is_absolute(), "FDS content path must be absolute");
    ensure!(
        matches!(
            path.extension().and_then(|v| v.to_str()),
            Some("fds" | "FDS")
        ),
        "FDS content must use the exact .fds or .FDS extension"
    );
    let named = fs::symlink_metadata(path)
        .with_context(|| format!("inspecting FDS content {}", path.display()))?;
    ensure!(
        named.is_file() && !named.file_type().is_symlink(),
        "FDS content must be an existing regular, non-symlink file"
    );
    ensure_canonical(path, "FDS content")?;
    let file = open_path_file(path)?;
    let opened = file.metadata()?;
    ensure!(
        opened.is_file() && FileIdentity::metadata(&opened) == FileIdentity::metadata(&named),
        "FDS content identity changed while it was being validated"
    );
    let final_named = fs::symlink_metadata(path)?;
    ensure!(
        final_named.is_file()
            && !final_named.file_type().is_symlink()
            && FileIdentity::metadata(&final_named) == FileIdentity::metadata(&opened),
        "FDS content identity changed while it was being validated"
    );
    Ok(())
}

#[cfg(target_os = "linux")]
#[derive(Debug)]
struct OpenDir {
    file: File,
    path: PathBuf,
    identity: FileIdentity,
}

#[cfg(target_os = "linux")]
fn open_data_home(path: &Path, home: &Path, expected_uid: u32) -> Result<OpenDir> {
    ensure!(home.is_absolute(), "current user home must be absolute");
    let home_meta = fs::symlink_metadata(home)
        .with_context(|| format!("inspecting current user home {}", home.display()))?;
    ensure!(
        home_meta.is_dir() && !home_meta.file_type().is_symlink(),
        "current user home must be an existing non-symlink directory"
    );
    ensure!(
        home_meta.uid() == expected_uid,
        "current user home must be owned by effective UID {expected_uid}"
    );
    ensure_canonical(home, "current user home")?;
    let expected = home
        .join(".var/app")
        .join(NESTOPIA_UE_FLATPAK_ID)
        .join("data");
    ensure!(
        path == expected,
        "Flatpak data home must be the current user's exact ~/.var/app/{NESTOPIA_UE_FLATPAK_ID}/data directory"
    );
    let file = open_path_dir(path)
        .with_context(|| format!("opening Flatpak data home {}", path.display()))?;
    let directory = OpenDir {
        identity: FileIdentity::metadata(&file.metadata()?),
        file,
        path: path.to_path_buf(),
    };
    ensure_named_directory(&directory, expected_uid)?;
    Ok(directory)
}

#[cfg(target_os = "linux")]
fn open_or_create_target_dir(data: &OpenDir, expected_uid: u32) -> Result<OpenDir> {
    let name = cstr("nestopia")?;
    let created = match mkdir_at(&data.file, &name, 0o700) {
        Ok(()) => true,
        Err(error) if error.kind() == ErrorKind::AlreadyExists => false,
        Err(error) => return Err(error).context("creating Nestopia data directory"),
    };
    if created {
        data.file
            .sync_all()
            .context("syncing Flatpak data home after creating Nestopia directory")?;
    }
    open_target_dir(data, expected_uid)
}

#[cfg(target_os = "linux")]
fn open_target_dir(data: &OpenDir, expected_uid: u32) -> Result<OpenDir> {
    let name = cstr("nestopia")?;
    let file = open_at(
        &data.file,
        &name,
        libc::O_RDONLY | libc::O_CLOEXEC | libc::O_NOFOLLOW | libc::O_DIRECTORY,
        0,
    )
    .context("opening Nestopia data directory without following symlinks")?;
    let directory = OpenDir {
        identity: FileIdentity::metadata(&file.metadata()?),
        file,
        path: data.path.join("nestopia"),
    };
    ensure_named_directory(&directory, expected_uid)?;
    ensure_named_directory(data, expected_uid)?;
    Ok(directory)
}

#[cfg(target_os = "linux")]
fn ensure_named_directory(directory: &OpenDir, expected_uid: u32) -> Result<()> {
    let opened = directory.file.metadata()?;
    ensure!(
        opened.is_dir()
            && opened.uid() == expected_uid
            && FileIdentity::metadata(&opened) == directory.identity,
        "opened directory is not owned by effective UID {expected_uid}: {}",
        directory.path.display()
    );
    let named = fs::symlink_metadata(&directory.path)
        .with_context(|| format!("rechecking directory {}", directory.path.display()))?;
    ensure!(
        named.is_dir()
            && !named.file_type().is_symlink()
            && named.uid() == expected_uid
            && FileIdentity::metadata(&named) == directory.identity,
        "directory identity or effective-UID ownership changed: {}",
        directory.path.display()
    );
    ensure_canonical(&directory.path, "directory")
}

#[cfg(target_os = "linux")]
fn ensure_canonical(path: &Path, description: &str) -> Result<()> {
    ensure!(
        fs::canonicalize(path)
            .with_context(|| format!("resolving {description} {}", path.display()))?
            == path,
        "{description} must be supplied as its canonical path: {}",
        path.display()
    );
    Ok(())
}

#[cfg(target_os = "linux")]
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
struct FileIdentity {
    device: u64,
    inode: u64,
}

#[cfg(target_os = "linux")]
impl FileIdentity {
    fn metadata(value: &fs::Metadata) -> Self {
        Self {
            device: value.dev(),
            inode: value.ino(),
        }
    }

    fn stat(value: &libc::stat) -> Self {
        Self {
            device: value.st_dev,
            inode: value.st_ino,
        }
    }
}

#[cfg(target_os = "linux")]
#[derive(Debug)]
struct Inspected {
    image: ValidatedFdsBios,
    identity: FileIdentity,
    mode: u32,
    uid: u32,
    links: u64,
}

#[cfg(target_os = "linux")]
fn inspect_source(path: &Path, allowed: &[&str]) -> Result<Inspected> {
    ensure!(path.is_absolute(), "FDS BIOS source path must be absolute");
    let named = fs::symlink_metadata(path)
        .with_context(|| format!("inspecting BIOS candidate {}", path.display()))?;
    ensure!(
        named.is_file() && !named.file_type().is_symlink(),
        "FDS BIOS must be a regular, non-symlink file"
    );
    ensure_canonical(path, "FDS BIOS source")?;
    let file = open_path_file(path)?;
    ensure!(
        FileIdentity::metadata(&file.metadata()?) == FileIdentity::metadata(&named),
        "FDS BIOS identity changed while it was being opened"
    );
    let inspected = inspect_open(file, allowed)?;
    let final_named = fs::symlink_metadata(path)?;
    ensure!(
        final_named.is_file()
            && !final_named.file_type().is_symlink()
            && FileIdentity::metadata(&final_named) == inspected.identity,
        "FDS BIOS identity changed while it was being inspected"
    );
    ensure_canonical(path, "FDS BIOS source")?;
    Ok(inspected)
}

#[cfg(target_os = "linux")]
fn inspect_at(directory: &File, name: &CStr, allowed: &[&str]) -> Result<Inspected> {
    let named = stat_at(directory, name)?;
    ensure!(
        named.st_mode & libc::S_IFMT == libc::S_IFREG,
        "FDS BIOS must be a regular, non-symlink file"
    );
    let file = open_at(
        directory,
        name,
        libc::O_RDONLY | libc::O_CLOEXEC | libc::O_NOFOLLOW,
        0,
    )?;
    ensure!(
        FileIdentity::metadata(&file.metadata()?) == FileIdentity::stat(&named),
        "FDS BIOS identity changed while it was being opened"
    );
    let inspected = inspect_open(file, allowed)?;
    let final_named = stat_at(directory, name)?;
    ensure!(
        final_named.st_mode & libc::S_IFMT == libc::S_IFREG
            && FileIdentity::stat(&final_named) == inspected.identity,
        "FDS BIOS identity changed while it was being inspected"
    );
    Ok(inspected)
}

#[cfg(target_os = "linux")]
fn inspect_open(mut file: File, allowed: &[&str]) -> Result<Inspected> {
    let initial = file.metadata()?;
    ensure!(
        initial.is_file() && initial.len() == FDS_BIOS_BYTES as u64,
        "FDS BIOS must be a raw {FDS_BIOS_BYTES}-byte file"
    );
    let identity = FileIdentity::metadata(&initial);
    let mut bytes = [0; FDS_BIOS_BYTES];
    file.read_exact(&mut bytes).context("reading FDS BIOS")?;
    let mut trailing = [0];
    ensure!(
        file.read(&mut trailing)? == 0,
        "FDS BIOS must contain exactly {FDS_BIOS_BYTES} bytes"
    );
    let final_meta = file.metadata()?;
    ensure!(
        final_meta.len() == FDS_BIOS_BYTES as u64
            && FileIdentity::metadata(&final_meta) == identity,
        "FDS BIOS changed while it was being inspected"
    );
    let crc32 = crc32fast::hash(&bytes);
    let sha256 = hex::encode(Sha256::digest(bytes));
    ensure!(
        allowed.iter().any(|candidate| *candidate == sha256),
        "unrecognized FDS BIOS SHA-256 {sha256} (diagnostic CRC32 {crc32:08x})"
    );
    Ok(Inspected {
        image: ValidatedFdsBios {
            bytes,
            fingerprint: FirmwareFingerprint { crc32, sha256 },
        },
        identity,
        mode: final_meta.mode(),
        uid: final_meta.uid(),
        links: final_meta.nlink(),
    })
}

#[cfg(target_os = "linux")]
fn existing_receipt(
    directory: &OpenDir,
    name: &CStr,
    target: &Path,
    requested: &ValidatedFdsBios,
    allowed: &[&str],
    expected_uid: u32,
) -> Result<InstallReceipt> {
    let existing = inspect_at(&directory.file, name, allowed).with_context(|| {
        format!(
            "refusing to replace existing Nestopia BIOS target {}",
            target.display()
        )
    })?;
    ensure_installed_constraints(&existing, expected_uid, true, "existing Nestopia BIOS")?;
    ensure!(
        existing.image.fingerprint.sha256 == requested.fingerprint.sha256,
        "refusing to replace different Nestopia BIOS at {} (existing SHA-256 {}, requested SHA-256 {})",
        target.display(),
        existing.image.fingerprint.sha256,
        requested.fingerprint.sha256
    );
    ensure_named_directory(directory, expected_uid)?;
    Ok(InstallReceipt {
        disposition: InstallDisposition::AlreadyPresent,
        target: target.to_path_buf(),
        fingerprint: existing.image.fingerprint,
    })
}

#[cfg(target_os = "linux")]
fn ensure_installed_constraints(
    file: &Inspected,
    expected_uid: u32,
    require_single_link: bool,
    description: &str,
) -> Result<()> {
    ensure!(
        file.uid == expected_uid,
        "{description} must be owned by effective UID {expected_uid}"
    );
    ensure!(
        file.mode & 0o7777 == 0o600,
        "{description} must have exact mode 0600"
    );
    if require_single_link {
        ensure!(
            file.links == 1,
            "{description} must have exactly one hard link"
        );
    }
    Ok(())
}

#[cfg(target_os = "linux")]
fn open_path_dir(path: &Path) -> std::io::Result<File> {
    let mut options = OpenOptions::new();
    options
        .read(true)
        .custom_flags(libc::O_CLOEXEC | libc::O_NOFOLLOW | libc::O_DIRECTORY);
    options.open(path)
}

#[cfg(target_os = "linux")]
fn open_path_file(path: &Path) -> std::io::Result<File> {
    let mut options = OpenOptions::new();
    options
        .read(true)
        .custom_flags(libc::O_CLOEXEC | libc::O_NOFOLLOW);
    options.open(path)
}

#[cfg(target_os = "linux")]
fn open_at(directory: &File, name: &CStr, flags: i32, mode: libc::mode_t) -> std::io::Result<File> {
    // SAFETY: valid directory FD and NUL-terminated name; successful FD is owned.
    let fd = unsafe { libc::openat(directory.as_raw_fd(), name.as_ptr(), flags, mode) };
    if fd < 0 {
        Err(std::io::Error::last_os_error())
    } else {
        // SAFETY: openat returned a new descriptor.
        Ok(unsafe { File::from_raw_fd(fd) })
    }
}

#[cfg(target_os = "linux")]
fn mkdir_at(directory: &File, name: &CStr, mode: libc::mode_t) -> std::io::Result<()> {
    // SAFETY: valid directory FD and NUL-terminated name.
    let rc = unsafe { libc::mkdirat(directory.as_raw_fd(), name.as_ptr(), mode) };
    if rc == 0 {
        Ok(())
    } else {
        Err(std::io::Error::last_os_error())
    }
}

#[cfg(target_os = "linux")]
fn stat_at(directory: &File, name: &CStr) -> std::io::Result<libc::stat> {
    // SAFETY: zeroed stat is initialized by successful fstatat.
    let mut value: libc::stat = unsafe { std::mem::zeroed() };
    // SAFETY: valid directory FD, name, and output pointer.
    let rc = unsafe {
        libc::fstatat(
            directory.as_raw_fd(),
            name.as_ptr(),
            &mut value,
            libc::AT_SYMLINK_NOFOLLOW,
        )
    };
    if rc == 0 {
        Ok(value)
    } else {
        Err(std::io::Error::last_os_error())
    }
}

#[cfg(target_os = "linux")]
fn link_at(
    source_dir: &File,
    source: &CStr,
    target_dir: &File,
    target: &CStr,
) -> std::io::Result<()> {
    // SAFETY: valid directory FDs and names; flags 0 never replaces target.
    let rc = unsafe {
        libc::linkat(
            source_dir.as_raw_fd(),
            source.as_ptr(),
            target_dir.as_raw_fd(),
            target.as_ptr(),
            0,
        )
    };
    if rc == 0 {
        Ok(())
    } else {
        Err(std::io::Error::last_os_error())
    }
}

#[cfg(target_os = "linux")]
fn unlink_at(directory: &File, name: &CStr) -> std::io::Result<()> {
    // SAFETY: valid directory FD and name; flags 0 removes non-directories only.
    let rc = unsafe { libc::unlinkat(directory.as_raw_fd(), name.as_ptr(), 0) };
    if rc == 0 {
        Ok(())
    } else {
        Err(std::io::Error::last_os_error())
    }
}

#[cfg(target_os = "linux")]
fn create_temp_at(directory: &File) -> Result<(CString, File)> {
    for _ in 0..128 {
        let sequence = TEMP_SEQUENCE.fetch_add(1, Ordering::Relaxed);
        let name = cstr(&format!(
            ".{FDS_BIOS_FILENAME}.lunchbox-{}-{sequence}.tmp",
            std::process::id()
        ))?;
        match open_at(
            directory,
            &name,
            libc::O_WRONLY | libc::O_CREAT | libc::O_EXCL | libc::O_CLOEXEC | libc::O_NOFOLLOW,
            0o600,
        ) {
            Ok(file) => return Ok((name, file)),
            Err(error) if error.kind() == ErrorKind::AlreadyExists => continue,
            Err(error) => return Err(error).context("creating temporary FDS BIOS"),
        }
    }
    bail!("could not allocate a unique temporary FDS BIOS name")
}

#[cfg(target_os = "linux")]
fn rollback_if_identity(directory: &File, name: &CStr, expected: FileIdentity) {
    let matches = stat_at(directory, name)
        .map(|meta| FileIdentity::stat(&meta) == expected)
        .unwrap_or(false);
    if matches && unlink_at(directory, name).is_ok() {
        let _ = directory.sync_all();
    }
}

#[cfg(target_os = "linux")]
fn linked_identity_after_link(
    directory: &File,
    source: &CStr,
    target: &CStr,
) -> Result<FileIdentity> {
    let source = FileIdentity::stat(
        &stat_at(directory, source)
            .context("temporary FDS BIOS name changed immediately after atomic installation")?,
    );
    let target = FileIdentity::stat(
        &stat_at(directory, target)
            .context("installed FDS BIOS name changed immediately after atomic installation")?,
    );
    ensure!(
        source == target,
        "temporary or installed FDS BIOS identity changed immediately after atomic installation"
    );
    Ok(target)
}

#[cfg(target_os = "linux")]
struct RelativeTemp<'a> {
    directory: &'a File,
    name: Option<CString>,
}

#[cfg(target_os = "linux")]
impl<'a> RelativeTemp<'a> {
    fn new(directory: &'a File, name: CString) -> Self {
        Self {
            directory,
            name: Some(name),
        }
    }

    fn name(&self) -> &CStr {
        self.name.as_deref().expect("temporary guard is armed")
    }

    fn remove(&mut self) -> std::io::Result<()> {
        unlink_at(self.directory, self.name())?;
        self.name = None;
        Ok(())
    }
}

#[cfg(target_os = "linux")]
impl Drop for RelativeTemp<'_> {
    fn drop(&mut self) {
        if let Some(name) = self.name.take() {
            let _ = unlink_at(self.directory, &name);
            let _ = self.directory.sync_all();
        }
    }
}

fn cstr(value: &str) -> Result<CString> {
    CString::new(value).context("relative file name contains a NUL byte")
}
