//! Stella's detected-device -> frontend-port routing, separate from detection.
//! Source: libretro/stella ba52c43b9eda950eb0c0eec69cda9b17dee8c39b,
//! src/os/libretro/libretro.cxx update_input (lines 255-436).
//! Callers must establish the final post-property/post-swap controller types.
//! A requested profile, filename, or joystick default is not detection evidence.
use anyhow::{Result, ensure};

/// Private ROM/property inputs for a future detection invocation. Keep this
/// owner alive through report collection and compare both source and snapshot
/// again before accepting the report. The original basename is significant:
/// Stella can use it as a cartridge-name fallback and to find the .pro file.
pub struct ContentSnapshot {
    _directory: tempfile::TempDir,
    content: std::path::PathBuf,
    original: std::path::PathBuf,
    cartridge_md5: String,
    content_sha256: String,
    property_sha256: Option<String>,
}

impl ContentSnapshot {
    pub fn create(content: &std::path::Path) -> Result<Self> {
        Self::create_in(content, None)
    }

    fn create_in(content: &std::path::Path, parent: Option<&std::path::Path>) -> Result<Self> {
        use md5::Md5;
        use sha2::Digest;
        ensure!(
            content.is_absolute(),
            "Stella detection requires an absolute cartridge path"
        );
        let name = content
            .file_name()
            .and_then(|name| name.to_str())
            .ok_or_else(|| anyhow::anyhow!("Stella cartridge filename is not UTF-8"))?;
        ensure!(
            !name.chars().any(char::is_control),
            "Stella cartridge filename contains control characters"
        );
        let extension = content
            .extension()
            .and_then(|extension| extension.to_str())
            .unwrap_or("");
        ensure!(
            extension.eq_ignore_ascii_case("a26") || extension.eq_ignore_ascii_case("bin"),
            "Stella detection requires an unpacked A26/BIN cartridge"
        );
        // Pinned Stella Cartridge::maxSize() is 512 KiB. Do not silently
        // truncate larger inputs or calculate an identity for different bytes.
        let bytes = read_bounded_input(content, 512 * 1024)?;
        ensure!(!bytes.is_empty(), "Stella cartridge is empty");
        let properties = read_optional_properties(&content.with_extension("pro"))?;
        let directory = private_directory("lunchbox-stella-input-", parent)?;
        let snapshot_path = directory.path().join(name);
        write_readonly_input(&snapshot_path, &bytes)?;
        if let Some(properties) = &properties {
            write_readonly_input(&snapshot_path.with_extension("pro"), properties)?;
        }
        let snapshot = Self {
            _directory: directory,
            content: snapshot_path,
            original: content.to_owned(),
            cartridge_md5: format!("{:x}", Md5::digest(&bytes)),
            content_sha256: input_sha256(&bytes),
            property_sha256: properties.as_deref().map(input_sha256),
        };
        snapshot.verify_unchanged()?;
        Ok(snapshot)
    }

    pub fn content_path(&self) -> &std::path::Path {
        &self.content
    }

    pub fn cartridge_md5(&self) -> &str {
        &self.cartridge_md5
    }

    pub fn verify_unchanged(&self) -> Result<()> {
        for path in [&self.original, &self.content] {
            ensure!(
                input_sha256(&read_bounded_input(path, 512 * 1024)?) == self.content_sha256,
                "Stella cartridge changed during detection preparation"
            );
            let properties = read_optional_properties(&path.with_extension("pro"))?;
            ensure!(
                properties.as_deref().map(input_sha256) == self.property_sha256,
                "Stella companion properties changed during detection preparation"
            );
        }
        Ok(())
    }
}

fn input_sha256(bytes: &[u8]) -> String {
    use sha2::{Digest, Sha256};
    format!("{:x}", Sha256::digest(bytes))
}

fn read_bounded_input(path: &std::path::Path, maximum: u64) -> Result<Vec<u8>> {
    use std::io::Read;
    let file = std::fs::File::open(path)?;
    ensure!(
        file.metadata()?.is_file(),
        "Stella input is not a regular file"
    );
    let mut bytes = Vec::new();
    file.take(maximum + 1).read_to_end(&mut bytes)?;
    ensure!(
        bytes.len() as u64 <= maximum,
        "Stella input exceeds its size limit"
    );
    Ok(bytes)
}

fn read_optional_properties(path: &std::path::Path) -> Result<Option<Vec<u8>>> {
    // Only true absence means no properties. Permission errors, directories,
    // broken links and oversized files must never become a default profile.
    match std::fs::symlink_metadata(path) {
        Ok(_) => Ok(Some(read_bounded_input(path, 1024 * 1024)?)),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(None),
        Err(error) => Err(error.into()),
    }
}

fn write_readonly_input(path: &std::path::Path, bytes: &[u8]) -> Result<()> {
    use std::io::Write;
    let mut file = std::fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(path)?;
    file.write_all(bytes)?;
    let mut permissions = file.metadata()?.permissions();
    permissions.set_readonly(true);
    file.set_permissions(permissions)?;
    Ok(())
}

fn private_directory(prefix: &str, parent: Option<&std::path::Path>) -> Result<tempfile::TempDir> {
    let mut builder = tempfile::Builder::new();
    builder.prefix(prefix);
    if let Some(parent) = parent {
        ensure!(
            parent.is_absolute() && parent.is_dir(),
            "Detection storage must be an existing absolute directory"
        );
        Ok(builder.tempdir_in(parent)?)
    } else {
        Ok(builder.tempdir()?)
    }
}

/// Native RetroArch detection context. Callers must identify the executable as
/// native RetroArch and the selected core as Stella before constructing this.
/// Flatpak/container path translation is deliberately a separate adapter.
#[cfg(target_os = "linux")]
pub struct NativeDetection {
    inputs: ContentSnapshot,
    _runtime: tempfile::TempDir,
    executable: std::path::PathBuf,
    core: std::path::PathBuf,
    core_sha256: String,
    config: std::path::PathBuf,
}

#[cfg(target_os = "linux")]
impl NativeDetection {
    pub fn prepare(
        executable: &std::path::Path,
        core: &std::path::Path,
        content: &std::path::Path,
        options: &std::collections::BTreeMap<String, String>,
    ) -> Result<Self> {
        Self::prepare_in(executable, core, content, options, None)
    }

    /// Shared context construction for a runtime whose private storage must
    /// live outside the host's default temporary namespace. Selecting the
    /// sandbox-visible parent and constructing its command remain adapter work.
    fn prepare_in(
        executable: &std::path::Path,
        core: &std::path::Path,
        content: &std::path::Path,
        options: &std::collections::BTreeMap<String, String>,
        parent: Option<&std::path::Path>,
    ) -> Result<Self> {
        ensure!(
            executable.is_absolute() && core.is_absolute(),
            "Native Stella detection requires absolute executable/core paths"
        );
        ensure!(
            executable.is_file(),
            "Native RetroArch executable is unavailable"
        );
        let inputs = ContentSnapshot::create_in(content, parent)?;
        let core_sha256 = input_sha256(&read_bounded_input(core, 256 * 1024 * 1024)?);
        let runtime = private_directory("lunchbox-stella-runtime-", parent)?;
        let root = runtime.path();
        let mut config = String::from(
            "video_driver = \"null\"\naudio_driver = \"null\"\ninput_driver = \"null\"\naudio_enable = \"false\"\nvideo_vsync = \"false\"\nconfig_save_on_exit = \"false\"\nsavestate_auto_load = \"false\"\nsavestate_auto_save = \"false\"\nautosave_interval = \"0\"\nrewind_enable = \"false\"\nrun_ahead_enabled = \"false\"\npreemptive_frames_enable = \"false\"\nnetplay_enable = \"false\"\ncheevos_enable = \"false\"\nauto_overrides_enable = \"false\"\nauto_remaps_enable = \"false\"\ninput_autodetect_enable = \"false\"\ngame_specific_options = \"false\"\nglobal_core_options = \"false\"\n",
        );
        config.push_str("history_list_enable = \"false\"\ncontent_runtime_log = \"false\"\ncontent_runtime_log_aggregate = \"false\"\nlog_to_file = \"false\"\n");
        for (key, directory) in [
            ("savefile_directory", "saves"),
            ("savestate_directory", "states"),
            ("system_directory", "system"),
            ("rgui_config_directory", "config"),
            ("screenshot_directory", "screenshots"),
            ("cache_directory", "cache"),
        ] {
            let path = root.join(directory);
            std::fs::create_dir(&path)?;
            config.push_str(&format!("{key} = \"{}\"\n", config_path(&path)?));
        }
        let mut option_text = String::new();
        for (key, value) in options {
            ensure!(
                !key.is_empty()
                    && key
                        .bytes()
                        .all(|byte| byte.is_ascii_alphanumeric() || byte == b'_'),
                "Invalid Stella core option name"
            );
            ensure!(
                !value
                    .chars()
                    .any(|character| character.is_control() || matches!(character, '"' | '\\')),
                "Stella core option cannot be represented safely"
            );
            option_text.push_str(&format!("{key} = \"{value}\"\n"));
        }
        let option_path = root.join("core-options.cfg");
        write_readonly_input(&option_path, option_text.as_bytes())?;
        config.push_str(&format!(
            "core_options_path = \"{}\"\n",
            config_path(&option_path)?
        ));
        let config_path = root.join("retroarch.cfg");
        write_readonly_input(&config_path, config.as_bytes())?;
        Ok(Self {
            inputs,
            _runtime: runtime,
            executable: executable.to_owned(),
            core: core.to_owned(),
            core_sha256,
            config: config_path,
        })
    }

    pub fn detect(&self, cancel: &std::sync::atomic::AtomicBool) -> Result<ConsoleReport> {
        self.verify_inputs()?;
        let mut command = std::process::Command::new(&self.executable);
        command
            .current_dir(self._runtime.path())
            .arg("--config")
            .arg(&self.config)
            .arg("--max-frames=1")
            .arg("--verbose")
            .arg("-L")
            .arg(&self.core)
            .arg("--")
            .arg(self.inputs.content_path());
        let report = collect_report(command, self.inputs.cartridge_md5(), cancel)?;
        self.verify_inputs()?;
        Ok(report)
    }

    pub(crate) fn verify_inputs(&self) -> Result<()> {
        self.inputs.verify_unchanged()?;
        ensure!(
            input_sha256(&read_bounded_input(&self.core, 256 * 1024 * 1024)?) == self.core_sha256,
            "Selected Stella core changed during controller detection"
        );
        Ok(())
    }
}

#[cfg(target_os = "linux")]
fn config_path(path: &std::path::Path) -> Result<&str> {
    let text = path
        .to_str()
        .ok_or_else(|| anyhow::anyhow!("Stella runtime path is not UTF-8"))?;
    ensure!(
        !text
            .chars()
            .any(|character| character.is_control() || matches!(character, '"' | '\\')),
        "Stella runtime path cannot be represented safely"
    );
    Ok(text)
}

/// Prepared inputs and command for the selected RetroArch Flatpak. This is not
/// a detection runner: Flatpak instance ownership must be attached before the
/// command can be passed to a collector. In particular, the native process-group
/// collector alone is insufficient for a sandbox which changes process groups.
#[cfg(target_os = "linux")]
pub(crate) struct FlatpakDetection {
    inputs: NativeDetection,
    app_id: String,
}

#[cfg(target_os = "linux")]
impl FlatpakDetection {
    pub(crate) fn prepare(
        launcher: &std::path::Path,
        app_id: &str,
        launch_arguments: &[std::ffi::OsString],
        core: &std::path::Path,
        content: &std::path::Path,
        options: &std::collections::BTreeMap<String, String>,
    ) -> Result<Self> {
        ensure!(
            app_id == "org.libretro.RetroArch",
            "Unreviewed Stella Flatpak application identity"
        );
        // Match the prefix constructed by emulator::command_prefix_with_access_roots.
        // Do not silently detect in a different installation, branch, runtime,
        // architecture or overridden command from the user's selected launch.
        ensure!(
            launch_arguments.first().is_some_and(|arg| arg == "run"),
            "Stella Flatpak launch must use the resolved run command"
        );
        let boundary = launch_arguments
            .iter()
            .position(|arg| arg.to_str() == Some(app_id))
            .ok_or_else(|| {
                anyhow::anyhow!("Missing selected Stella Flatpak application boundary")
            })?;
        for argument in &launch_arguments[1..boundary] {
            let path = argument.to_str().and_then(|arg| arg.strip_prefix("--filesystem="))
                .ok_or_else(|| anyhow::anyhow!("Custom Flatpak runtime selection needs matching Stella detection resolution"))?;
            ensure!(
                std::path::Path::new(path).is_absolute()
                    && !path.contains(':')
                    && !path.chars().any(char::is_control),
                "Unresolved selected Flatpak filesystem argument"
            );
        }
        let home = directories::BaseDirs::new()
            .ok_or_else(|| anyhow::anyhow!("Finding Flatpak controller detection storage"))?
            .home_dir()
            .canonicalize()?;
        let app_root = home.join(".var/app").join(app_id);
        // Existing installation data establishes this location; do not create
        // a fake application tree when the selected installation is absent.
        ensure!(
            app_root.is_dir(),
            "Selected RetroArch Flatpak data directory is unavailable"
        );
        let parent = app_root.join("cache");
        std::fs::create_dir_all(&parent)?;
        let parent = parent.canonicalize()?;
        ensure!(
            parent.starts_with(app_root.canonicalize()?),
            "Flatpak detection cache resolves outside the selected application directory"
        );
        let core = core.canonicalize()?;
        // Both config contents and CLI arguments use these same absolute paths.
        // Flatpak owns /tmp, /app and /usr, so a host path there cannot be
        // assumed to identify the same file inside the sandbox.
        flatpak_detection_path(&core)?;
        flatpak_detection_path(&parent)?;
        let inputs = NativeDetection::prepare_in(launcher, &core, content, options, Some(&parent))?;
        Ok(Self {
            inputs,
            app_id: app_id.to_owned(),
        })
    }

    pub(crate) fn verify_inputs(&self) -> Result<()> {
        self.inputs.verify_inputs()
    }

    /// Construct the command and its one-to-one identity channel together.
    /// No process is started here. The eventual collector must own both values
    /// until spawn, then immediately drop the command's parent-side writer.
    pub(crate) fn invocation(&self) -> Result<(std::process::Command, FlatpakInstanceReport)> {
        let (report, writer) = FlatpakInstanceReport::channel(self.inputs._runtime.path())?;
        Ok((self.command(writer)?, report))
    }

    /// The command owns the dedicated descriptor for Flatpak's instance ID.
    /// The collector must drop the command after spawning to close the parent's
    /// writer, drain identity separately, and clean up only that instance.
    fn command(&self, instance_id_fd: std::os::fd::OwnedFd) -> Result<std::process::Command> {
        use std::os::fd::AsRawFd;
        use std::os::unix::process::CommandExt;
        self.verify_inputs()?;
        let descriptor = instance_id_fd.as_raw_fd();
        ensure!(
            descriptor > 2,
            "Flatpak instance identity needs a dedicated descriptor"
        );
        let flags = unsafe { libc::fcntl(descriptor, libc::F_GETFD) };
        ensure!(
            flags >= 0 && flags & libc::FD_CLOEXEC != 0,
            "Flatpak instance identity descriptor must be close-on-exec in the parent"
        );
        let runtime = self.inputs._runtime.path();
        let input_root = self.inputs.inputs._directory.path();
        let mut command = std::process::Command::new(&self.inputs.executable);
        command
            .current_dir(runtime)
            .args([
                "run",
                "--sandbox",
                "--die-with-parent",
                "--nofilesystem=host:reset",
                "--command=retroarch",
            ])
            .arg(format!("--instance-id-fd={descriptor}"))
            .arg(format!("--cwd={}", flatpak_detection_path(runtime)?));
        for (path, access) in [
            (input_root, "ro"),
            (self.inputs.core.as_path(), "ro"),
            (runtime, "rw"),
        ] {
            command.arg(format!(
                "--filesystem={}:{access}",
                flatpak_detection_path(path)?
            ));
        }
        command
            .arg(&self.app_id)
            .arg("--config")
            .arg(&self.inputs.config)
            .args(["--max-frames=1", "--verbose", "-L"])
            .arg(&self.inputs.core)
            .arg("--")
            .arg(self.inputs.inputs.content_path());
        // Only the forked child inherits this pipe across exec. Clearing the
        // flag in the multithreaded parent could leak it to unrelated launches.
        // Own the fd in the closure so it cannot be closed/reused before spawn.
        unsafe {
            command.pre_exec(move || {
                if libc::fcntl(
                    instance_id_fd.as_raw_fd(),
                    libc::F_SETFD,
                    flags & !libc::FD_CLOEXEC,
                ) < 0
                {
                    return Err(std::io::Error::last_os_error());
                }
                Ok(())
            });
        }
        Ok(command)
    }
}

/// The numeric identifier written by Flatpak itself on a dedicated pipe. It is
/// never inferred from logs, an application ID, or a pre-existing instance list.
/// Source: flatpak/common/flatpak-run.c flatpak_run_add_app_info_args writes the
/// complete ID and closes the descriptor, without a newline. Allocation in
/// flatpak-instance.c formats one unsigned 32-bit integer in decimal.
#[cfg(target_os = "linux")]
pub(crate) struct FlatpakInstanceReport {
    reader: std::fs::File,
    instances: std::path::PathBuf,
    expected_mount: String,
    bytes: Vec<u8>,
    complete: Option<String>,
    failed: bool,
    leased: bool,
}

#[cfg(target_os = "linux")]
impl FlatpakInstanceReport {
    fn channel(private_runtime: &std::path::Path) -> Result<(Self, std::os::fd::OwnedFd)> {
        use std::os::fd::{AsRawFd, FromRawFd, OwnedFd};
        // Flatpak serializes read-write filesystem flags without the :rw
        // suffix in [Instance] extra-args. This unique directory binds the
        // report to our invocation even if an old numeric ID was recycled.
        let expected_mount = format!("--filesystem={}", flatpak_detection_path(private_runtime)?);
        let directories = directories::BaseDirs::new()
            .ok_or_else(|| anyhow::anyhow!("Finding Flatpak instance runtime directory"))?;
        let runtime = directories
            .runtime_dir()
            .ok_or_else(|| anyhow::anyhow!("Flatpak detection needs XDG_RUNTIME_DIR"))?
            .canonicalize()?;
        ensure!(
            runtime.is_absolute(),
            "Flatpak runtime directory must be absolute"
        );
        let mut descriptors = [-1; 2];
        ensure!(
            unsafe { libc::pipe2(descriptors.as_mut_ptr(), libc::O_CLOEXEC) } == 0,
            "Cannot create Flatpak instance report pipe: {}",
            std::io::Error::last_os_error()
        );
        let reader = unsafe { OwnedFd::from_raw_fd(descriptors[0]) };
        let writer = unsafe { OwnedFd::from_raw_fd(descriptors[1]) };
        // Never rely on stdin/stdout/stderr already being open. A descriptor
        // at 0..2 would be replaced by the collector's stdio configuration.
        let duplicate = |fd: &OwnedFd| -> Result<OwnedFd> {
            let duplicated = unsafe { libc::fcntl(fd.as_raw_fd(), libc::F_DUPFD_CLOEXEC, 3) };
            ensure!(
                duplicated >= 0,
                "Cannot reserve Flatpak report descriptor: {}",
                std::io::Error::last_os_error()
            );
            Ok(unsafe { OwnedFd::from_raw_fd(duplicated) })
        };
        let reader = std::fs::File::from(duplicate(&reader)?);
        let writer = duplicate(&writer)?;
        // Only reads are nonblocking. Flatpak's tiny write must not fail with
        // EAGAIN, and each endpoint retains close-on-exec in this process.
        set_nonblocking(&reader)?;
        Ok((
            Self {
                reader,
                instances: runtime.join(".flatpak"),
                expected_mount,
                bytes: Vec::with_capacity(10),
                complete: None,
                failed: false,
                leased: false,
            },
            writer,
        ))
    }

    /// Acquire only the fresh identity received on this invocation's pipe.
    /// Retain the returned lease through cleanup; do not reacquire by ID later.
    pub(crate) fn lease(&mut self) -> Result<Option<FlatpakInstanceLease>> {
        ensure!(!self.leased, "Flatpak instance report already has an owner");
        let Some(id) = self.poll()?.map(str::to_owned) else {
            return Ok(None);
        };
        let lease =
            FlatpakInstanceLease::acquire(&self.instances, id, self.expected_mount.clone())?;
        self.leased = true;
        Ok(Some(lease))
    }

    /// Poll once without blocking or waiting for a delimiter Flatpak never
    /// sends. A partial number is not an accepted ID: only EOF commits it.
    /// An error permanently invalidates the report, including later polls.
    pub(crate) fn poll(&mut self) -> Result<Option<&str>> {
        ensure!(
            !self.failed,
            "Flatpak instance report was previously invalidated"
        );
        if self.complete.is_none() {
            if let Err(error) = self.read_available() {
                self.failed = true;
                self.bytes.clear();
                return Err(error);
            }
        }
        Ok(self.complete.as_deref())
    }

    fn read_available(&mut self) -> Result<()> {
        use std::io::Read;
        let mut buffer = [0u8; 11];
        // Bound even EINTR retries so the owner's cancel/deadline checks run.
        for _ in 0..4 {
            match self.reader.read(&mut buffer) {
                Ok(0) => {
                    ensure!(
                        !self.bytes.is_empty(),
                        "Flatpak closed its instance report without an identity"
                    );
                    let text = std::str::from_utf8(&self.bytes)?;
                    let value: u32 = text.parse().map_err(|_| {
                        anyhow::anyhow!("Flatpak instance identity is outside its numeric range")
                    })?;
                    ensure!(
                        value.to_string() == text,
                        "Flatpak instance identity is not canonical decimal"
                    );
                    self.complete = Some(text.to_owned());
                    return Ok(());
                }
                Ok(length) => {
                    ensure!(
                        self.bytes.len() + length <= 10,
                        "Flatpak instance report exceeds its numeric identity limit"
                    );
                    ensure!(
                        buffer[..length].iter().all(u8::is_ascii_digit),
                        "Flatpak instance report contains nonnumeric data"
                    );
                    self.bytes.extend_from_slice(&buffer[..length]);
                }
                Err(error) if error.kind() == std::io::ErrorKind::WouldBlock => return Ok(()),
                Err(error) if error.kind() == std::io::ErrorKind::Interrupted => continue,
                Err(error) => return Err(error.into()),
            }
        }
        Ok(())
    }
}

/// Holds Flatpak's instance reference lock through cleanup. Flatpak's garbage
/// collector requires an exclusive lock on .ref before removing this directory;
/// an OFD read lock participates in that protocol without process-wide POSIX
/// lock release when another thread closes a descriptor for the same inode.
#[cfg(target_os = "linux")]
pub(crate) struct FlatpakInstanceLease {
    id: String,
    expected_mount: String,
    path: std::path::PathBuf,
    directory: std::fs::File,
    reference: std::fs::File,
}

#[cfg(target_os = "linux")]
impl FlatpakInstanceLease {
    fn acquire(instances: &std::path::Path, id: String, expected_mount: String) -> Result<Self> {
        use std::os::fd::{AsRawFd, FromRawFd};
        use std::os::unix::fs::OpenOptionsExt;
        let path = instances.join(&id);
        let directory = std::fs::OpenOptions::new()
            .read(true)
            .custom_flags(libc::O_DIRECTORY | libc::O_NOFOLLOW | libc::O_CLOEXEC)
            .open(&path)?;
        let raw = unsafe {
            libc::openat(
                directory.as_raw_fd(),
                b".ref\0".as_ptr().cast(),
                libc::O_RDONLY | libc::O_NOFOLLOW | libc::O_CLOEXEC | libc::O_NONBLOCK,
            )
        };
        ensure!(
            raw >= 0,
            "Cannot retain Flatpak instance reference: {}",
            std::io::Error::last_os_error()
        );
        let reference = unsafe { std::fs::File::from_raw_fd(raw) };
        let lease = Self {
            id,
            expected_mount,
            path,
            directory,
            reference,
        };
        lease.verify_identity()?;
        let mut lock: libc::flock = unsafe { std::mem::zeroed() };
        lock.l_type = libc::F_RDLCK as _;
        lock.l_whence = libc::SEEK_SET as _;
        // Nonblocking: GC already holding the write lock means this instance
        // cannot safely be retained. Never wait and accept a replacement.
        ensure!(
            unsafe { libc::fcntl(lease.reference.as_raw_fd(), libc::F_OFD_SETLK, &lock) } == 0,
            "Cannot lock the reported Flatpak instance: {}",
            std::io::Error::last_os_error()
        );
        lease.verify_identity()?;
        lease.verify_metadata()?;
        Ok(lease)
    }

    fn verify_identity(&self) -> Result<()> {
        use std::os::unix::fs::MetadataExt;
        let directory = self.directory.metadata()?;
        let reference = self.reference.metadata()?;
        let path_directory = std::fs::symlink_metadata(&self.path)?;
        let path_reference = std::fs::symlink_metadata(self.path.join(".ref"))?;
        let uid = unsafe { libc::geteuid() };
        ensure!(
            directory.is_dir()
                && reference.is_file()
                && directory.uid() == uid
                && reference.uid() == uid
                && directory.nlink() > 0
                && reference.nlink() == 1,
            "Flatpak instance reference has an unexpected type or owner"
        );
        ensure!(
            path_directory.is_dir()
                && path_reference.is_file()
                && (directory.dev(), directory.ino())
                    == (path_directory.dev(), path_directory.ino())
                && (reference.dev(), reference.ino())
                    == (path_reference.dev(), path_reference.ino()),
            "Reported Flatpak instance was removed or replaced"
        );
        Ok(())
    }

    fn verify_metadata(&self) -> Result<()> {
        use std::io::Read;
        use std::os::fd::{AsRawFd, FromRawFd};
        use std::os::unix::fs::MetadataExt;
        let raw = unsafe {
            libc::openat(
                self.directory.as_raw_fd(),
                b"info\0".as_ptr().cast(),
                libc::O_RDONLY | libc::O_NOFOLLOW | libc::O_CLOEXEC | libc::O_NONBLOCK,
            )
        };
        ensure!(
            raw >= 0,
            "Cannot read Flatpak instance metadata: {}",
            std::io::Error::last_os_error()
        );
        let file = unsafe { std::fs::File::from_raw_fd(raw) };
        let metadata = file.metadata()?;
        ensure!(
            metadata.is_file()
                && metadata.uid() == unsafe { libc::geteuid() }
                && metadata.len() <= 65536,
            "Unexpected Flatpak instance metadata file"
        );
        let mut text = String::new();
        file.take(65537).read_to_string(&mut text)?;
        ensure!(
            text.len() <= 65536,
            "Flatpak instance metadata exceeds its size limit"
        );
        ensure!(
            flatpak_info_field(&text, "Application", "name")? == "org.libretro.RetroArch"
                && flatpak_info_field(&text, "Instance", "instance-id")? == self.id
                && flatpak_info_field(&text, "Instance", "sandbox")? == "true",
            "Flatpak instance metadata does not identify this detection sandbox"
        );
        let arguments = flatpak_info_list(flatpak_info_field(&text, "Instance", "extra-args")?)?;
        ensure!(
            arguments
                .iter()
                .filter(|argument| *argument == &self.expected_mount)
                .count()
                == 1,
            "Flatpak instance does not belong to this private detection invocation"
        );
        self.verify_identity()
    }

    fn cleanup_command(&self, launcher: &std::path::Path) -> Result<std::process::Command> {
        self.verify_identity()?;
        self.verify_metadata()?;
        ensure!(
            launcher.is_absolute() && launcher.is_file(),
            "Selected Flatpak launcher is unavailable"
        );
        let mut command = std::process::Command::new(launcher);
        command.arg("kill").arg(&self.id);
        Ok(command)
    }

    /// Query locks other than this lease's own OFD lock. Flatpak holds a read
    /// lock during setup and the sandbox lifetime. Our own read lock is ignored
    /// by F_OFD_GETLK on this same open file description; other holders conflict
    /// with the hypothetical write lock. Do not actually upgrade to a write lock.
    fn has_runtime_holder(&self) -> Result<bool> {
        use std::os::fd::AsRawFd;
        self.verify_identity()?;
        let mut lock: libc::flock = unsafe { std::mem::zeroed() };
        lock.l_type = libc::F_WRLCK as _;
        lock.l_whence = libc::SEEK_SET as _;
        ensure!(
            unsafe { libc::fcntl(self.reference.as_raw_fd(), libc::F_OFD_GETLK, &mut lock) } == 0,
            "Cannot inspect Flatpak instance lifetime: {}",
            std::io::Error::last_os_error()
        );
        Ok(i32::from(lock.l_type) != libc::F_UNLCK)
    }

    /// Stop only this retained instance. A failed or timed-out cleanup remains
    /// an error; callers must not relabel it as a successful detection result.
    /// The reference lock stays held for the entire command and its cleanup.
    pub(crate) fn terminate(&self, launcher: &std::path::Path) -> Result<()> {
        use std::os::unix::process::CommandExt;
        use std::process::Stdio;
        use std::time::{Duration, Instant};
        if !self.has_runtime_holder()? {
            return Ok(());
        }
        let mut command = self.cleanup_command(launcher)?;
        command
            .stdin(Stdio::null())
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .process_group(0);
        let mut process = DetectionProcess {
            child: command.spawn()?,
            reaped: false,
        };
        drop(command);
        let deadline = Instant::now() + Duration::from_secs(1);
        loop {
            if process.exited_without_reaping()? {
                let status = process.finish()?;
                // An instance may exit between the lifetime check and `kill`.
                // Conversely, flatpak kill can return zero with startup still
                // pending. Neither exit code by itself proves cleanup.
                while Instant::now() < deadline {
                    if !self.has_runtime_holder()? {
                        return Ok(());
                    }
                    std::thread::sleep(Duration::from_millis(5));
                }
                anyhow::bail!("Flatpak instance remains active after cleanup ({status})");
            }
            ensure!(
                Instant::now() < deadline,
                "Flatpak instance cleanup exceeded one second"
            );
            std::thread::sleep(Duration::from_millis(5));
        }
    }
}

#[cfg(target_os = "linux")]
fn flatpak_info_field<'a>(text: &'a str, section: &str, key: &str) -> Result<&'a str> {
    let mut current = "";
    let mut found = None;
    for line in text.lines() {
        let line = line.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        if let Some(group) = line
            .strip_prefix('[')
            .and_then(|line| line.strip_suffix(']'))
        {
            current = group;
        } else if current == section {
            if let Some((name, value)) = line.split_once('=') {
                if name.trim() == key {
                    ensure!(
                        found.is_none(),
                        "Duplicate Flatpak instance metadata field {section}/{key}"
                    );
                    found = Some(value.trim());
                }
            }
        }
    }
    found.ok_or_else(|| anyhow::anyhow!("Missing Flatpak instance metadata field {section}/{key}"))
}

/// Decode the GLib key-file string-list escaping used for extra-args. Match
/// whole decoded arguments, never substrings of paths or escaped separators.
#[cfg(target_os = "linux")]
fn flatpak_info_list(text: &str) -> Result<Vec<String>> {
    let mut values = Vec::new();
    let mut value = String::new();
    let mut characters = text.chars();
    while let Some(character) = characters.next() {
        match character {
            ';' => values.push(std::mem::take(&mut value)),
            '\\' => value.push(match characters.next() {
                Some('s') => ' ',
                Some('n') => '\n',
                Some('t') => '\t',
                Some('r') => '\r',
                Some('\\') => '\\',
                Some(';') => ';',
                _ => anyhow::bail!("Invalid Flatpak instance argument escape"),
            }),
            character => value.push(character),
        }
        ensure!(
            values.len() <= 256,
            "Flatpak instance has too many extra arguments"
        );
    }
    if !value.is_empty() {
        values.push(value);
    }
    ensure!(
        values.len() <= 256,
        "Flatpak instance has too many extra arguments"
    );
    Ok(values)
}

#[cfg(target_os = "linux")]
fn flatpak_detection_path(path: &std::path::Path) -> Result<&str> {
    let text = config_path(path)?;
    ensure!(
        path.is_absolute() && !text.contains(':'),
        "Flatpak detection needs an absolute path without filesystem-option separators"
    );
    ensure!(
        !path
            .components()
            .any(|component| matches!(component, std::path::Component::ParentDir)),
        "Flatpak detection path contains an unresolved parent component"
    );
    ensure!(
        ![
            "/tmp", "/var/tmp", "/app", "/usr", "/etc", "/proc", "/sys", "/dev", "/run"
        ]
        .iter()
        .any(|root| path.starts_with(root)),
        "Selected Stella detection path is in a Flatpak-owned namespace"
    );
    Ok(text)
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ControllerKind {
    Joystick,
    Genesis,
    BoosterGrip,
    Joy2BPlus,
    Driving,
    Paddles,
    Lightgun,
    AmigaMouse,
    AtariMouse,
    TrakBall,
    Unsupported,
}

impl ControllerKind {
    fn from_runtime_name(name: &str) -> Result<Self> {
        Ok(match name {
            "Joystick" => Self::Joystick,
            "Sega Genesis" => Self::Genesis,
            "Booster Grip" => Self::BoosterGrip,
            "Joy 2B+" => Self::Joy2BPlus,
            "Driving" => Self::Driving,
            "Paddles" => Self::Paddles,
            "Light Gun" => Self::Lightgun,
            "Amiga mouse" => Self::AmigaMouse,
            "Atari mouse" => Self::AtariMouse,
            "Trak-Ball" => Self::TrakBall,
            _ => anyhow::bail!("Unreviewed Stella runtime controller description: {name}"),
        })
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ConsoleReport {
    pub cartridge_md5: String,
    pub left: ControllerKind,
    pub right: ControllerKind,
}

impl ConsoleReport {
    pub fn routing(&self) -> Result<Vec<RoutedController>> {
        route_detected(self.left, self.right)
    }
}

/// Expected emulated devices for the three currently documented digital
/// profiles. This is an acceptance condition, never a way to force detection.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum DigitalContract {
    Joysticks,
    GenesisPads,
    BoosterGripOrJoy2BPlus,
}

impl DigitalContract {
    fn accepts(self, kind: ControllerKind) -> bool {
        match self {
            Self::Joysticks => kind == ControllerKind::Joystick,
            Self::GenesisPads => kind == ControllerKind::Genesis,
            Self::BoosterGripOrJoy2BPlus => matches!(
                kind,
                ControllerKind::BoosterGrip | ControllerKind::Joy2BPlus
            ),
        }
    }

    pub fn accept_report(self, report: &ConsoleReport) -> Result<Vec<RoutedController>> {
        ensure!(
            self.accepts(report.left) && self.accepts(report.right),
            "Detected Stella controllers {:?}/{:?} do not match the selected {:?} profile; choose their actual controller contract",
            report.left,
            report.right,
            self
        );
        let routes = report.routing()?;
        ensure!(
            routes.len() == 2
                && routes[0].frontend_port == 1
                && routes[1].frontend_port == 2
                && routes[0].jack == Jack::Left
                && routes[1].jack == Jack::Right
                && routes[0].owns_console_panel
                && !routes[1].owns_console_panel
                && routes.iter().all(|route| route.paddle.is_none()),
            "Detected Stella routing does not match the selected two-controller topology"
        );
        Ok(routes)
    }
}

#[cfg(target_os = "linux")]
impl NativeDetection {
    /// Run detection and accept only this contract's actual emulated types.
    /// Returned per-port outputs include console controls on player one only.
    /// A mixed BoosterGrip/Joy2BPlus pair is allowed because both use the same
    /// three independent frontend fire events; the core retains electrical
    /// differences. Other mixed pairs cannot silently become two joysticks.
    pub fn detect_digital_contract(
        &self,
        expected: DigitalContract,
        cancel: &std::sync::atomic::AtomicBool,
    ) -> Result<Vec<RoutedController>> {
        let report = self.detect(cancel)?;
        expected.accept_report(&report)
    }
}

/// Parse the core-owned OSystem::getROMInfo report, emitted by createConsole
/// AFTER constructing and swapping its controllers. This is deliberately not
/// a parser for heuristic "detected for left port" messages, which can precede
/// property overrides or port swapping. The collector must supply only fresh
/// output from the exact selected core/content launch, not a saved user log.
///
/// The MD5 is an identity match to Stella's report, not a cryptographic trust
/// claim. The collector must bind the core/content files separately as well.
pub fn parse_console_report(log: &str, expected_md5: &str) -> Result<ConsoleReport> {
    ensure!(
        log.len() <= 1024 * 1024,
        "Stella detection output exceeds its limit"
    );
    ensure!(
        expected_md5.len() == 32 && expected_md5.bytes().all(|byte| byte.is_ascii_hexdigit()),
        "Invalid expected Stella cartridge MD5"
    );
    let lines: Vec<_> = log.lines().collect();
    let mut selected: Option<ConsoleReport> = None;
    for (start, line) in lines.iter().enumerate() {
        if *line != "Game console created:" {
            continue;
        }
        let mut cursor = start + 1;
        let field = |cursor: &mut usize, prefix: &str| -> Result<&str> {
            let line = lines
                .get(*cursor)
                .ok_or_else(|| anyhow::anyhow!("Incomplete Stella console report"))?;
            *cursor += 1;
            ensure!(
                line.len() <= 4096,
                "Stella console report line exceeds its limit"
            );
            line.strip_prefix(prefix).ok_or_else(|| {
                anyhow::anyhow!("Unexpected Stella console report field; expected {prefix}")
            })
        };
        ensure!(
            !field(&mut cursor, "  ROM file: ")?.is_empty(),
            "Stella report lacks a ROM path"
        );
        if lines
            .get(cursor)
            .is_some_and(|line| line.starts_with("  PRO file: "))
        {
            ensure!(
                !field(&mut cursor, "  PRO file: ")?.is_empty(),
                "Invalid Stella property-file report"
            );
        }
        ensure!(
            lines.get(cursor) == Some(&""),
            "Malformed Stella console report separator"
        );
        cursor += 1;
        let _name = field(&mut cursor, "  Cart Name:       ")?;
        let md5 = field(&mut cursor, "  Cart MD5:        ")?;
        ensure!(
            md5.eq_ignore_ascii_case(expected_md5),
            "Stella console report belongs to different cartridge content"
        );
        let left = field(&mut cursor, "  Controller 0:    ")?
            .strip_suffix(" in left port")
            .ok_or_else(|| {
                anyhow::anyhow!("Stella left controller has unexpected post-swap routing")
            })?;
        let right = field(&mut cursor, "  Controller 1:    ")?
            .strip_suffix(" in right port")
            .ok_or_else(|| {
                anyhow::anyhow!("Stella right controller has unexpected post-swap routing")
            })?;
        ensure!(
            !field(&mut cursor, "  Display Format:  ")?.is_empty(),
            "Incomplete Stella display report"
        );
        ensure!(
            !field(&mut cursor, "  Bankswitch Type: ")?.is_empty(),
            "Incomplete Stella cartridge report"
        );
        let report = ConsoleReport {
            cartridge_md5: md5.to_ascii_lowercase(),
            left: ControllerKind::from_runtime_name(left)?,
            right: ControllerKind::from_runtime_name(right)?,
        };
        if let Some(previous) = &selected {
            // The core can send the same report to both its log callback and
            // stdout. Accept exact duplicate detection, not changing topology.
            ensure!(
                previous == &report,
                "Conflicting Stella controller reports in one detection session"
            );
        } else {
            selected = Some(report);
        }
    }
    selected
        .ok_or_else(|| anyhow::anyhow!("Stella did not emit a complete final controller report"))
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Jack {
    Left,
    Right,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Paddle {
    A,
    B,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RoutedController {
    /// One-based frontend player slot, not the Atari console jack number.
    pub frontend_port: usize,
    pub jack: Jack,
    pub kind: ControllerKind,
    pub paddle: Option<Paddle>,
    /// Console switches always come from frontend port one, even with paddles.
    pub owns_console_panel: bool,
}

pub const CONSOLE_OUTPUTS: &[(&str, &str)] = &[
    ("select", "Select"),
    ("reset", "Start"),
    ("left_diff_a", "LeftBumper"),
    ("left_diff_b", "LeftTrigger"),
    ("right_diff_a", "RightBumper"),
    ("right_diff_b", "RightTrigger"),
    ("color", "LeftStick"),
    ("black_white", "RightStick"),
];

impl RoutedController {
    /// Digital actions only. Driving/paddle analog transport remains distinct;
    /// this list must not be treated as a complete proportional-input contract.
    pub fn digital_outputs(&self) -> Result<Vec<(&'static str, &'static str)>> {
        let mut outputs = match self.kind {
            ControllerKind::Joystick
            | ControllerKind::Genesis
            | ControllerKind::BoosterGrip
            | ControllerKind::Joy2BPlus => vec![
                ("up", "DPadUp"),
                ("down", "DPadDown"),
                ("left", "DPadLeft"),
                ("right", "DPadRight"),
                ("fire", "South"),
            ],
            ControllerKind::Driving => vec![
                ("counterclockwise", "DPadLeft"),
                ("clockwise", "DPadRight"),
                ("fire", "South"),
            ],
            ControllerKind::Paddles => vec![
                ("increase", "DPadLeft"),
                ("decrease", "DPadRight"),
                ("fire", "South"),
            ],
            _ => anyhow::bail!("This Stella controller has no reviewed digital-output contract"),
        };
        if matches!(
            self.kind,
            ControllerKind::Genesis | ControllerKind::BoosterGrip | ControllerKind::Joy2BPlus
        ) {
            outputs.push(("trigger", "East"));
        }
        if matches!(
            self.kind,
            ControllerKind::BoosterGrip | ControllerKind::Joy2BPlus
        ) {
            outputs.push(("booster", "West"));
        }
        if self.owns_console_panel {
            outputs.extend_from_slice(CONSOLE_OUTPUTS);
        }
        Ok(outputs)
    }
}

/// Calculate routing from already-established final controller types. This
/// function does NOT infer types from ROM content and does not authorize launch.
/// In the core's input loop only a Paddles case increments the pad index inside
/// a jack; the unconditional increment between jacks determines the right base.
pub fn route_detected(
    left: ControllerKind,
    right: ControllerKind,
) -> Result<Vec<RoutedController>> {
    let mut routes = Vec::new();
    for (jack, kind) in [(Jack::Left, left), (Jack::Right, right)] {
        ensure!(
            matches!(
                kind,
                ControllerKind::Joystick
                    | ControllerKind::Genesis
                    | ControllerKind::BoosterGrip
                    | ControllerKind::Joy2BPlus
                    | ControllerKind::Driving
                    | ControllerKind::Paddles
            ),
            "Detected Stella controller requires a separate pointer or peripheral transport"
        );
        let paddles: &[Option<Paddle>] = if kind == ControllerKind::Paddles {
            &[Some(Paddle::A), Some(Paddle::B)]
        } else {
            &[None]
        };
        for paddle in paddles {
            let frontend_port = routes.len() + 1;
            routes.push(RoutedController {
                frontend_port,
                jack,
                kind,
                paddle: *paddle,
                owns_console_panel: frontend_port == 1,
            });
        }
    }
    Ok(routes)
}

/// Collect fresh stdout from a prepared, isolated native detection command.
/// This process mechanism does not itself choose a core, content, properties,
/// config or sandbox. The launch adapter must prepare that exact context first.
/// Nothing is executed until this function is called; it has no current launch
/// caller while the detection-context preparation remains unfinished.
#[cfg(target_os = "linux")]
pub fn collect_report(
    mut command: std::process::Command,
    expected_md5: &str,
    cancel: &std::sync::atomic::AtomicBool,
) -> Result<ConsoleReport> {
    use std::os::unix::process::CommandExt;
    use std::process::Stdio;
    use std::sync::atomic::Ordering;
    use std::time::{Duration, Instant};
    ensure!(
        expected_md5.len() == 32 && expected_md5.bytes().all(|byte| byte.is_ascii_hexdigit()),
        "Invalid expected Stella cartridge MD5"
    );
    ensure!(
        !cancel.load(Ordering::Relaxed),
        "Stella controller detection cancelled"
    );
    command
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .process_group(0);
    let mut process = DetectionProcess {
        child: command.spawn()?,
        reaped: false,
    };
    let mut stdout = process
        .child
        .stdout
        .take()
        .ok_or_else(|| anyhow::anyhow!("Missing Stella detection stdout"))?;
    let mut stderr = process
        .child
        .stderr
        .take()
        .ok_or_else(|| anyhow::anyhow!("Missing Stella detection stderr"))?;
    set_nonblocking(&stdout)?;
    set_nonblocking(&stderr)?;
    let mut output = Vec::new();
    let mut diagnostics = Vec::new();
    let mut stdout_done = false;
    let mut stderr_done = false;
    let deadline = Instant::now() + Duration::from_secs(10);
    loop {
        ensure!(
            !cancel.load(Ordering::Relaxed),
            "Stella controller detection cancelled"
        );
        ensure!(
            Instant::now() < deadline,
            "Stella controller detection exceeded ten seconds"
        );
        if !stdout_done {
            stdout_done = drain_detection_pipe(&mut stdout, &mut output)?;
        }
        if !stderr_done {
            stderr_done = drain_detection_pipe(&mut stderr, &mut diagnostics)?;
        }
        if process.exited_without_reaping()? && stdout_done && stderr_done {
            break;
        }
        std::thread::sleep(Duration::from_millis(5));
    }
    let status = process.finish()?;
    ensure!(
        status.success(),
        "Stella controller detection exited with {status}"
    );
    // Raw stdout contains the final OSystem report. Do not join stderr into it:
    // frontend log prefixes and interleaving could fabricate a field sequence.
    let output = std::str::from_utf8(&output)
        .map_err(|_| anyhow::anyhow!("Stella detection output is not UTF-8"))?;
    parse_console_report(output, expected_md5)
}

#[cfg(target_os = "linux")]
fn set_nonblocking(file: &impl std::os::fd::AsRawFd) -> Result<()> {
    let descriptor = file.as_raw_fd();
    let flags = unsafe { libc::fcntl(descriptor, libc::F_GETFL) };
    ensure!(
        flags >= 0
            && unsafe { libc::fcntl(descriptor, libc::F_SETFL, flags | libc::O_NONBLOCK) } >= 0,
        "Cannot make Stella detection output nonblocking: {}",
        std::io::Error::last_os_error()
    );
    Ok(())
}

#[cfg(target_os = "linux")]
fn drain_detection_pipe(reader: &mut impl std::io::Read, output: &mut Vec<u8>) -> Result<bool> {
    let mut chunk = [0u8; 4096];
    // Bound each pipe's work so a noisy stderr cannot starve timeout/cancel.
    for _ in 0..16 {
        match reader.read(&mut chunk) {
            Ok(0) => return Ok(true),
            Ok(length) => {
                ensure!(
                    output.len() + length <= 1024 * 1024,
                    "Stella detection output exceeds its one-megabyte limit"
                );
                output.extend_from_slice(&chunk[..length]);
            }
            Err(error) if error.kind() == std::io::ErrorKind::WouldBlock => return Ok(false),
            Err(error) if error.kind() == std::io::ErrorKind::Interrupted => continue,
            Err(error) => return Err(error.into()),
        }
    }
    Ok(false)
}

#[cfg(target_os = "linux")]
struct DetectionProcess {
    child: std::process::Child,
    reaped: bool,
}

#[cfg(target_os = "linux")]
impl DetectionProcess {
    fn exited_without_reaping(&mut self) -> Result<bool> {
        // Keep the leader's PID reserved until group cleanup. Reaping first
        // could let a later killpg target a reused, unrelated process-group ID.
        let mut info: libc::siginfo_t = unsafe { std::mem::zeroed() };
        let result = unsafe {
            libc::waitid(
                libc::P_PID,
                self.child.id(),
                &mut info,
                libc::WEXITED | libc::WNOHANG | libc::WNOWAIT,
            )
        };
        if result < 0 {
            let error = std::io::Error::last_os_error();
            if error.raw_os_error() == Some(libc::ECHILD) {
                // Another process reaper or host SIGCHLD policy removed our
                // ownership evidence. Never signal a potentially reused PID.
                self.reaped = true;
            }
            if error.kind() == std::io::ErrorKind::Interrupted {
                return Ok(false);
            }
            return Err(error.into());
        }
        Ok(unsafe { info.si_pid() } != 0)
    }

    fn finish(&mut self) -> Result<std::process::ExitStatus> {
        self.stop_owned_group();
        let status = match self.child.wait() {
            Ok(status) => status,
            Err(error) => {
                if error.raw_os_error() == Some(libc::ECHILD) {
                    self.reaped = true;
                }
                return Err(error.into());
            }
        };
        self.reaped = true;
        Ok(status)
    }

    fn stop_owned_group(&self) {
        if !self.reaped {
            unsafe {
                libc::kill(-(self.child.id() as libc::pid_t), libc::SIGKILL);
            }
        }
    }
}

#[cfg(target_os = "linux")]
impl Drop for DetectionProcess {
    fn drop(&mut self) {
        if !self.reaped {
            self.stop_owned_group();
            let _ = self.child.wait();
            self.reaped = true;
        }
    }
}
