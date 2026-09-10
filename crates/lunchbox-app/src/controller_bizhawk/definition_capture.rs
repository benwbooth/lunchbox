//! One-shot Lua interoperability for the active BizHawk controller definition.
//! Preparation, bounded capture and child ownership; launch gating is pending.
use anyhow::{Context, Result, ensure};
use serde::Deserialize;
use std::{collections::BTreeSet, path::Path};

/// Own the prepared script, config and application arguments as one lifetime.
/// This is not a running child or an authenticated controller-definition receipt.
#[cfg(target_os = "linux")]
pub(crate) struct PreparedCapture {
    files: CaptureFiles,
    config: super::PreparedConfig,
    arguments: Vec<std::ffi::OsString>,
    working_directory: std::path::PathBuf,
}

#[cfg(target_os = "linux")]
impl PreparedCapture {
    /// Excludes any direct-Mono executable/assembly prefix. The caller's
    /// arguments remain untouched on success and on every failure. Arguments
    /// contain options only; this method appends the sole content argument.
    pub(crate) fn prepare(
        application_args: &[std::ffi::OsString],
        working_directory: &Path,
        exe_directory: &Path,
        content_path: &Path,
    ) -> Result<Self> {
        ensure!(
            working_directory.is_absolute() && exe_directory.is_absolute(),
            "Capture working and executable directories must be absolute"
        );
        let text_args = application_args
            .iter()
            .map(|argument| {
                argument
                    .to_str()
                    .map(str::to_owned)
                    .context("Capture application arguments must be UTF-8")
            })
            .collect::<Result<Vec<_>>>()?;
        validate_capture_options(&text_args)?;
        let content = content_path
            .to_str()
            .context("Capture content path must be UTF-8")?;
        ensure!(
            content_path.is_absolute()
                && !content.contains('|')
                && !content.chars().any(char::is_control),
            "Capture requires one absolute content file, not an archive-member selector"
        );
        let content_artifact = super::RuntimeArtifact::capture(content_path)?;
        let files = CaptureFiles::prepare()?;
        let mut arguments = files
            .prepare_arguments(&text_args)?
            .into_iter()
            .map(std::ffi::OsString::from)
            .collect::<Vec<_>>();
        let mut config = super::prepare_arguments_using(
            &mut arguments,
            working_directory,
            exe_directory,
            prepare_config,
        )?;
        // The shared owner checks the original source by default; capture
        // also requires its generated startup configuration to stay unchanged.
        let generated = super::RuntimeArtifact::capture(&config.path())?;
        config.runtime_artifacts.push(generated);
        config.runtime_artifacts.push(content_artifact);
        // Keep content last as recommended by the pinned ArgParser. Config
        // rewriting cannot accidentally consume it as an option value.
        arguments.push(content_path.as_os_str().to_owned());
        let prepared = Self {
            files,
            config,
            arguments,
            working_directory: working_directory.to_owned(),
        };
        prepared.verify()?;
        Ok(prepared)
    }

    pub(crate) fn arguments(&self) -> &[std::ffi::OsString] {
        &self.arguments
    }

    pub(super) fn retain_artifact(&mut self, artifact: super::RuntimeArtifact) {
        self.config.runtime_artifacts.push(artifact);
    }

    pub(crate) fn configuration(&self) -> Result<String> {
        self.verify()?;
        let text = super::read_config(&self.config.path())?;
        self.verify()?;
        Ok(text)
    }

    /// Starts only the explicitly selected executable, with an optional Mono
    /// assembly argument. Callers still must bind the selected runtime/content.
    pub(crate) fn spawn(
        mut self,
        program: &Path,
        assembly: Option<&Path>,
        environment: &[(std::ffi::OsString, std::ffi::OsString)],
    ) -> Result<CaptureProcess> {
        use std::os::unix::process::CommandExt;
        use std::process::{Command, Stdio};
        self.config
            .runtime_artifacts
            .push(super::RuntimeArtifact::capture(program)?);
        if let Some(assembly) = assembly {
            self.config
                .runtime_artifacts
                .push(super::RuntimeArtifact::capture(assembly)?);
        }
        self.verify()?;
        let mut command = Command::new(program);
        if let Some(assembly) = assembly {
            command.arg(assembly);
        }
        command
            .args(&self.arguments)
            .current_dir(&self.working_directory)
            .envs(environment.iter().cloned())
            .stdin(Stdio::null())
            .stdout(Stdio::null())
            .stderr(Stdio::piped())
            .process_group(0);
        let child = command
            .spawn()
            .context("Starting owned BizHawk definition capture")?;
        let process = CaptureProcess {
            child,
            reaped: false,
            prepared: self,
            started: std::time::Instant::now(),
            stderr_tail: Vec::new(),
            stderr_bytes: 0,
        };
        use std::os::fd::AsRawFd;
        let fd = process
            .child
            .stderr
            .as_ref()
            .context("Missing capture stderr pipe")?
            .as_raw_fd();
        let flags = unsafe { libc::fcntl(fd, libc::F_GETFL) };
        ensure!(
            flags >= 0 && unsafe { libc::fcntl(fd, libc::F_SETFL, flags | libc::O_NONBLOCK) } >= 0,
            "Cannot make capture stderr nonblocking"
        );
        process.prepared.verify()?;
        Ok(process)
    }

    pub(crate) fn verify(&self) -> Result<()> {
        self.config.verify_source()?;
        self.files.verify_script()
    }

    /// Caller must still establish child/core/content ownership. Nonce and
    /// file checks alone do not authenticate a same-user response writer.
    pub(crate) fn read_response(&self) -> Result<Definition> {
        self.verify()?;
        let definition = self.files.read_response()?;
        self.verify()?;
        Ok(definition)
    }
}

/// Narrow capture CLI, not a replacement for the interactive launch parser.
/// Unknown options fail instead of leaving ambiguous positional arguments.
fn validate_capture_options(arguments: &[String]) -> Result<()> {
    let mut index = 0;
    while index < arguments.len() {
        let argument = &arguments[index];
        match argument.as_str() {
            "--config" => {
                index += 1;
                let path = arguments
                    .get(index)
                    .context("Capture --config needs a path")?;
                ensure!(
                    !path.is_empty() && !path.starts_with('-') && !path.starts_with('@'),
                    "Ambiguous capture config path"
                );
            }
            "--gdi" | "--fullscreen" | "--chromeless" => {}
            other if other.starts_with("--config=") || other.starts_with("--config:") => {
                ensure!(
                    other.len() > "--config=".len(),
                    "Capture --config needs a path"
                );
            }
            _ => anyhow::bail!("Unsupported capture option or extra content argument: {argument}"),
        }
        index += 1;
    }
    Ok(())
}

/// Parse application arguments only, excluding any Mono/assembly prefix.
/// Returns the sole selected config path; callers must retain/read that file
/// and compare its contents before launching, not trust the path alone.
pub(super) fn handoff_config_path(
    arguments: &[std::ffi::OsString],
    expected_content: &Path,
) -> Result<std::path::PathBuf> {
    ensure!(
        !arguments.is_empty() && arguments.len() <= 256,
        "Invalid handoff argument count"
    );
    let text = arguments
        .iter()
        .map(|argument| {
            let text = argument
                .to_str()
                .context("Handoff arguments must be UTF-8")?;
            ensure!(
                text.len() <= 65536 && !text.chars().any(char::is_control),
                "Invalid handoff argument text"
            );
            Ok(text.to_owned())
        })
        .collect::<Result<Vec<_>>>()?;
    let content = text.last().context("Missing handoff content")?;
    ensure!(
        Path::new(content).is_absolute()
            && Path::new(content) == expected_content
            && !content.contains('|'),
        "Handoff must end with the original absolute content path"
    );
    let options = &text[..text.len() - 1];
    validate_capture_options(options)?;
    let mut config = None;
    let mut index = 0;
    while index < options.len() {
        let option = &options[index];
        let path = if option == "--config" {
            index += 1;
            Some(options[index].as_str()) // validated above, including its value
        } else {
            option
                .strip_prefix("--config=")
                .or_else(|| option.strip_prefix("--config:"))
        };
        if let Some(path) = path {
            ensure!(
                config.is_none(),
                "Handoff requires exactly one config option"
            );
            ensure!(
                Path::new(path).is_absolute(),
                "Handoff config path must be absolute"
            );
            config = Some(std::path::PathBuf::from(path));
        }
        index += 1;
    }
    config.context("Handoff requires an explicit owned configuration file")
}

/// Private child handle prevents outside callers from reaping before cleanup.
#[cfg(target_os = "linux")]
pub(crate) struct CaptureProcess {
    child: std::process::Child,
    reaped: bool,
    prepared: PreparedCapture,
    started: std::time::Instant,
    stderr_tail: Vec<u8>,
    stderr_bytes: usize,
}

#[cfg(target_os = "linux")]
impl CaptureProcess {
    fn drain_stderr(&mut self) -> Result<()> {
        use std::io::Read;
        let pipe = self
            .child
            .stderr
            .as_mut()
            .context("Missing capture stderr pipe")?;
        let mut buffer = [0u8; 4096];
        // Bound work per poll so diagnostic output cannot starve cancellation.
        for _ in 0..16 {
            match pipe.read(&mut buffer) {
                Ok(0) => break,
                Ok(length) => {
                    self.stderr_bytes += length;
                    self.stderr_tail.extend_from_slice(&buffer[..length]);
                    if self.stderr_tail.len() > 8192 {
                        self.stderr_tail.drain(..self.stderr_tail.len() - 8192);
                    }
                    ensure!(
                        self.stderr_bytes <= 1024 * 1024,
                        "Capture stderr exceeded 1 MiB"
                    );
                }
                Err(error) if error.kind() == std::io::ErrorKind::WouldBlock => break,
                Err(error) if error.kind() == std::io::ErrorKind::Interrupted => continue,
                Err(error) => return Err(error.into()),
            }
        }
        Ok(())
    }

    /// Worker-thread operation; never call on the UI thread. Always cleans up
    /// the owned group before returning a definition or a capture failure.
    pub(crate) fn wait_for_definition(
        mut self,
        timeout: std::time::Duration,
        cancel: &std::sync::atomic::AtomicBool,
    ) -> Result<Definition> {
        use std::sync::atomic::Ordering;
        use std::time::{Duration, Instant};
        let result: Result<Definition> = (|| {
            ensure!(
                !timeout.is_zero() && timeout <= Duration::from_secs(60),
                "Capture timeout must be greater than zero and at most 60 seconds"
            );
            let deadline = self.started + timeout;
            loop {
                ensure!(
                    !cancel.load(Ordering::Relaxed),
                    "Definition capture cancelled"
                );
                ensure!(Instant::now() < deadline, "Definition capture timed out");
                self.drain_stderr()?;
                let exited = match self.exited_without_reaping() {
                    Err(error)
                        if error.downcast_ref::<std::io::Error>().is_some_and(|error| {
                            error.kind() == std::io::ErrorKind::Interrupted
                        }) =>
                    {
                        continue;
                    }
                    result => result?,
                };
                // Publication is atomic. A present but invalid response is a
                // failure, never a reason to wait indefinitely for a rewrite.
                match std::fs::symlink_metadata(&self.prepared.files.response_path) {
                    Ok(_) => {
                        let definition = self.read_response()?;
                        ensure!(
                            !cancel.load(Ordering::Relaxed),
                            "Definition capture cancelled"
                        );
                        ensure!(Instant::now() < deadline, "Definition capture timed out");
                        return Ok(definition);
                    }
                    Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
                    Err(error) => return Err(error.into()),
                }
                ensure!(
                    !exited,
                    "Definition capture exited without publishing a response"
                );
                std::thread::sleep(
                    Duration::from_millis(20)
                        .min(deadline.saturating_duration_since(Instant::now())),
                );
            }
        })();
        let cleanup = self.stop();
        let diagnostic_read = self.drain_stderr();
        let result = match (result, diagnostic_read) {
            (Ok(definition), Ok(())) => Ok(definition),
            (Err(error), Ok(())) => Err(error),
            (Ok(_), Err(error)) => Err(error),
            (Err(error), Err(diagnostic)) => {
                Err(error.context(format!("Reading diagnostics also failed: {diagnostic:#}")))
            }
        };
        let outcome = match (result, cleanup) {
            (Ok(definition), Ok(())) => {
                self.prepared.verify()?;
                ensure!(
                    !cancel.load(Ordering::Relaxed),
                    "Definition capture cancelled during cleanup"
                );
                ensure!(
                    self.started.elapsed() < timeout,
                    "Definition capture timed out during cleanup"
                );
                Ok(definition)
            }
            (Err(error), Ok(())) => Err(error),
            (Ok(_), Err(error)) => Err(error.context("Cleaning up definition capture")),
            (Err(error), Err(cleanup)) => {
                Err(error.context(format!("Capture cleanup also failed: {cleanup:#}")))
            }
        };
        outcome.map_err(|error| {
            if self.stderr_tail.is_empty() {
                error
            } else {
                error.context(format!(
                    "Capture child stderr (escaped, final {} bytes): {}",
                    self.stderr_tail.len(),
                    String::from_utf8_lossy(&self.stderr_tail).escape_debug()
                ))
            }
        })
    }

    pub(crate) fn exited_without_reaping(&mut self) -> Result<bool> {
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
                self.reaped = true;
            }
            return Err(error.into());
        }
        Ok(unsafe { info.si_pid() } != 0)
    }

    pub(crate) fn read_response(&self) -> Result<Definition> {
        ensure!(
            !self.reaped,
            "Capture child ownership is no longer established"
        );
        self.prepared.read_response()
    }

    pub(crate) fn stop(&mut self) -> Result<()> {
        if self.reaped {
            return Ok(());
        }
        // WNOWAIT retains an exited leader's PID while we signal its group.
        // On ECHILD, ownership is gone: do not signal a potentially reused PID.
        loop {
            match self.exited_without_reaping() {
                Err(error)
                    if error
                        .downcast_ref::<std::io::Error>()
                        .is_some_and(|error| error.kind() == std::io::ErrorKind::Interrupted) =>
                {
                    continue;
                }
                result => {
                    result?;
                    break;
                }
            }
        }
        let result = unsafe { libc::kill(-(self.child.id() as libc::pid_t), libc::SIGKILL) };
        if result < 0 {
            let error = std::io::Error::last_os_error();
            if error.raw_os_error() != Some(libc::ESRCH) {
                return Err(error.into());
            }
        }
        match self.child.wait() {
            Ok(_) => {
                self.reaped = true;
                Ok(())
            }
            Err(error) => {
                if error.raw_os_error() == Some(libc::ECHILD) {
                    self.reaped = true;
                }
                Err(error.into())
            }
        }
    }
}

#[cfg(target_os = "linux")]
impl Drop for CaptureProcess {
    fn drop(&mut self) {
        let _ = self.stop();
    }
}

/// Read explicit core selection and sync settings without changing config.
pub(crate) fn selected_sync_config(
    source: &str,
    system: &str,
    name: &str,
    core: &str,
) -> Result<serde_json::Value> {
    ensure!(
        source.len() <= 16 * 1024 * 1024,
        "Capture config exceeds 16 MiB"
    );
    let value: serde_json::Value = serde_json::from_str(source)?;
    ensure!(
        value["PreferredCores"][system].as_str() == Some(name)
            && value["DontTryOtherCores"].as_bool() == Some(true),
        "Capture config must explicitly select {name} for {system} without fallback"
    );
    let sync = &value["CoreSyncSettings"][core];
    ensure!(
        sync.is_object(),
        "Capture config lacks {name} sync settings"
    );
    Ok(sync.clone())
}

/// Prepare an owned capture-only copy; never modify the user's config file.
/// Callers must select this config in the eventual owned child's arguments.
pub(crate) fn prepare_config(source: &Path) -> Result<super::PreparedConfig> {
    super::prepare_config_using(source, encode_capture_config)
}

/// Shared capture projection for preparation and later configuration comparison.
/// This suppresses startup actions; equality after projection is not permission
/// to enable those actions in a normal launch.
pub(super) fn encode_capture_config(source: &str) -> Result<String> {
    ensure!(
        source.len() <= 16 * 1024 * 1024,
        "Capture config exceeds 16 MiB"
    );
    let mut value: serde_json::Value = serde_json::from_str(source)?;
    let root = value
        .as_object_mut()
        .context("Capture config root must be an object")?;
    for name in [
        "RecentLua",
        "RecentLuaSession",
        "RecentRoms",
        "RecentMovies",
        "RecentWatches",
    ] {
        let recent = root.entry(name).or_insert_with(|| serde_json::json!({}));
        recent
            .as_object_mut()
            .with_context(|| format!("Capture config {name} must be an object"))?
            .insert("AutoLoad".to_owned(), false.into());
    }
    let cheats = root
        .entry("Cheats")
        .or_insert_with(|| serde_json::json!({}));
    let recent = cheats
        .as_object_mut()
        .context("Capture Cheats config must be an object")?
        .entry("Recent")
        .or_insert_with(|| serde_json::json!({}));
    recent
        .as_object_mut()
        .context("Capture Cheats.Recent must be an object")?
        .insert("AutoLoad".to_owned(), false.into());
    root.insert("AutoLoadLastSaveSlot".to_owned(), false.into());
    // The definition exists after core load; capturing it needs no gameplay
    // frames. MainForm resumes non-frame-waiting Lua scripts while paused.
    root.insert("StartPaused".to_owned(), true.into());
    root.insert("AutosaveSaveRAM".to_owned(), false.into());
    // Never inherit launch forwarding into an unrelated instance, or consume
    // gameplay/hotkey input while the capture window is in the background.
    root.insert("SingleInstanceMode".to_owned(), false.into());
    root.insert("AcceptBackgroundInput".to_owned(), false.into());
    root.insert(
        "AcceptBackgroundInputControllerOnly".to_owned(),
        false.into(),
    );
    // ToolManager examines both dictionaries, including typed custom values.
    // An empty capture-only tool configuration avoids instantiating those
    // values or inheriting any tool-specific startup/session behavior.
    root.insert("CommonToolSettings".to_owned(), serde_json::json!({}));
    root.insert("CustomToolSettings".to_owned(), serde_json::json!({}));
    Ok(serde_json::to_string_pretty(&value)? + "\n")
}

/// Startup settings that can invalidate a fresh controller-definition capture.
/// Read-only policy for the future handoff, not an argument/runtime validator.
pub(super) fn validate_handoff_startup(source: &str) -> Result<()> {
    ensure!(
        source.len() <= 16 * 1024 * 1024,
        "Handoff config exceeds 16 MiB"
    );
    let value: serde_json::Value = serde_json::from_str(source)?;
    let root = value
        .as_object()
        .context("Handoff config must be an object")?;
    fn disabled(object: &serde_json::Map<String, serde_json::Value>, key: &str) -> Result<()> {
        ensure!(
            object
                .get(key)
                .is_none_or(|value| value.as_bool() == Some(false)),
            "Captured setup requires {key} disabled (or its default false value)"
        );
        Ok(())
    }
    disabled(root, "SingleInstanceMode")?;
    disabled(root, "AutoLoadLastSaveSlot")?;
    for name in [
        "RecentLua",
        "RecentLuaSession",
        "RecentRoms",
        "RecentMovies",
        "RecentWatches",
    ] {
        if let Some(recent) = root.get(name) {
            let recent = recent
                .as_object()
                .with_context(|| format!("Handoff {name} must be an object"))?;
            disabled(recent, "AutoLoad")
                .with_context(|| format!("Unsafe {name} startup behavior"))?;
        }
    }
    if let Some(cheats) = root.get("Cheats") {
        let cheats = cheats
            .as_object()
            .context("Handoff Cheats must be an object")?;
        if let Some(recent) = cheats.get("Recent") {
            disabled(
                recent
                    .as_object()
                    .context("Handoff Cheats.Recent must be an object")?,
                "AutoLoad",
            )?;
        }
    }
    if let Some(tools) = root.get("CommonToolSettings") {
        for (name, settings) in tools
            .as_object()
            .context("CommonToolSettings must be an object")?
        {
            disabled(
                settings
                    .as_object()
                    .context("Common tool settings must be an object")?,
                "AutoLoad",
            )
            .with_context(|| format!("Tool {name} may auto-load during handoff"))?;
        }
    }
    // ToolManager may instantiate typed custom values and discover nested
    // ToolDialogSettings. Their startup semantics are not yet represented here.
    if let Some(custom) = root.get("CustomToolSettings") {
        ensure!(
            custom
                .as_object()
                .is_some_and(|settings| settings.is_empty()),
            "Custom tool startup settings are unresolved for captured handoff; use an explicitly empty private tool configuration"
        );
    }
    Ok(())
}

/// Owns capture files, not an emulator process. Keep alive until the future
/// owning child has stopped using its script and response paths.
#[cfg(target_os = "linux")]
pub(crate) struct CaptureFiles {
    _directory: tempfile::TempDir,
    script_path: std::path::PathBuf,
    response_path: std::path::PathBuf,
    nonce: String,
    script_text: String,
}

#[cfg(target_os = "linux")]
impl CaptureFiles {
    pub(crate) fn prepare() -> Result<Self> {
        use std::io::{Read, Write};
        use std::os::unix::fs::{OpenOptionsExt, PermissionsExt};
        let directory = tempfile::Builder::new()
            .prefix("lunchbox-bizhawk-definition-")
            .tempdir()?;
        std::fs::set_permissions(directory.path(), std::fs::Permissions::from_mode(0o700))?;
        let root = directory.path().canonicalize()?;
        let script_path = root.join("capture.lua");
        let response_path = root.join("response.json");
        let mut random = [0u8; 32];
        std::fs::File::open("/dev/urandom")?.read_exact(&mut random)?;
        let nonce = random
            .iter()
            .map(|byte| format!("{byte:02x}"))
            .collect::<String>();
        let script_text = script(&response_path, &nonce)?;
        let mut file = std::fs::OpenOptions::new()
            .write(true)
            .create_new(true)
            .mode(0o600)
            .open(&script_path)?;
        file.write_all(script_text.as_bytes())?;
        file.sync_all()?;
        Ok(Self {
            _directory: directory,
            script_path,
            response_path,
            nonce,
            script_text,
        })
    }

    pub(crate) fn script_path(&self) -> &Path {
        &self.script_path
    }

    /// Application arguments only: a direct-Mono executable/assembly prefix
    /// must be kept outside this slice. Returns a new vector, leaving the
    /// caller's launch plan untouched on any error. Does not start a process.
    pub(crate) fn prepare_arguments(&self, application_args: &[String]) -> Result<Vec<String>> {
        ensure!(
            application_args.len() <= 256,
            "Too many capture launch arguments"
        );
        for argument in application_args {
            ensure!(
                argument.len() <= 16384 && !argument.chars().any(char::is_control),
                "Capture launch argument is oversized or contains control characters"
            );
            ensure!(
                !argument.starts_with('@') && argument != "--",
                "Capture launch does not accept response files or option terminators"
            );
            let option = argument
                .split(['=', ':'])
                .next()
                .unwrap_or("")
                .to_ascii_lowercase();
            ensure!(
                !matches!(
                    option.as_str(),
                    "--lua"
                        | "--luaconsole"
                        | "--open-ext-tool-dll"
                        | "--movie"
                        | "--load-state"
                        | "--load-slot"
                        | "--version"
                        | "--help"
                        | "-h"
                        | "-?"
                        | "/?"
                        | "--dump-close"
                        | "--dump-frames"
                        | "--dump-length"
                        | "--dump-name"
                        | "--dump-type"
                ),
                "Capture launch conflicts with startup option {option}"
            );
        }
        self.verify_script()?;
        let script_path = self
            .script_path
            .to_str()
            .context("Capture script path is not UTF-8")?;
        let mut prepared = Vec::with_capacity(application_args.len() + 1);
        // ArgParser's --lua takes one path and implies opening LuaConsole.
        // One argv token keeps spaces literal; there is no shell quoting.
        prepared.push(format!("--lua={script_path}"));
        prepared.extend_from_slice(application_args);
        Ok(prepared)
    }

    pub(crate) fn verify_script(&self) -> Result<()> {
        ensure!(
            read_private_file(&self.script_path)? == self.script_text,
            "Definition capture script changed"
        );
        Ok(())
    }

    /// Call after observing publication by the owned child. A missing response
    /// is an I/O error, not an empty successful capture.
    pub(crate) fn read_response(&self) -> Result<Definition> {
        self.verify_script()?;
        parse(&read_private_file(&self.response_path)?, &self.nonce)
    }
}

#[cfg(target_os = "linux")]
fn read_private_file(path: &Path) -> Result<String> {
    use std::io::Read;
    use std::os::unix::fs::{MetadataExt, OpenOptionsExt};
    let mut file = std::fs::OpenOptions::new()
        .read(true)
        .custom_flags(libc::O_NOFOLLOW | libc::O_NONBLOCK)
        .open(path)?;
    let before = file.metadata()?;
    let owner = std::fs::metadata(path.parent().context("Capture file has no parent")?)?.uid();
    ensure!(
        before.is_file()
            && before.len() <= 1024 * 1024
            && before.nlink() == 1
            && before.uid() == owner,
        "Capture response is not a bounded, singly linked owned regular file"
    );
    let mut text = String::new();
    (&mut file)
        .take(1024 * 1024 + 1)
        .read_to_string(&mut text)?;
    let after = file.metadata()?;
    let named = std::fs::symlink_metadata(path)?;
    ensure!(
        text.len() as u64 == before.len()
            && after.len() == before.len()
            && after.modified()? == before.modified()?
            && after.ctime() == before.ctime()
            && after.ctime_nsec() == before.ctime_nsec()
            && named.is_file()
            && named.dev() == before.dev()
            && named.ino() == before.ino(),
        "Capture file changed while reading"
    );
    Ok(text)
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct Definition {
    pub version: u8,
    pub nonce: String,
    pub system: String,
    pub rom_hash: String,
    pub paused_before: bool,
    pub paused_after: bool,
    pub frame_before: i32,
    pub frame_after: i32,
    pub buttons: Vec<String>,
    pub axes: Vec<String>,
}

fn validate_nonce(nonce: &str) -> Result<()> {
    ensure!(
        nonce.len() == 64
            && nonce
                .bytes()
                .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b)),
        "Definition capture requires a 64-character lowercase hex nonce"
    );
    Ok(())
}

/// The caller must allocate output in a fresh private session directory. Merely
/// matching a nonce is not proof that the expected process wrote this response.
pub(crate) fn script(output: &Path, nonce: &str) -> Result<String> {
    validate_nonce(nonce)?;
    let output = output
        .to_str()
        .context("Capture output path must be UTF-8")?;
    ensure!(
        Path::new(output).is_absolute()
            && output.len() <= 4096
            && !output.chars().any(char::is_control),
        "Capture output needs an absolute bounded path without control characters"
    );
    // Fixed-width Lua decimal escapes prevent path text from becoming code.
    let quoted_path = format!(
        "\"{}\"",
        output
            .bytes()
            .map(|byte| format!("\\{byte:03}"))
            .collect::<String>()
    );
    Ok(
        format!("local output = {quoted_path}\nlocal nonce = \"{nonce}\"\n")
            + r#"
local function quote(s)
    assert(type(s) == 'string' and #s <= 4096, 'oversized capture string')
    return '"' .. s:gsub('[%z\1-\31\\"]', function(c)
        return string.format('\\u%04x', string.byte(c))
    end) .. '"'
end
local function capture()
local pausedBefore, frameBefore = client.ispaused(), emu.framecount()
local buttons, axes = {}, {}
local count = 0
for name, value in pairs(joypad.getimmediate()) do
    assert(type(name) == 'string' and #name > 0 and #name <= 512, 'invalid control name')
    count = count + 1
    assert(count <= 1024, 'too many controls')
    if type(value) == 'boolean' then buttons[#buttons + 1] = name
    elseif type(value) == 'number' then axes[#axes + 1] = name
    else error('unknown control value type') end
end
local function array(values)
    table.sort(values)
    for i, value in ipairs(values) do values[i] = quote(value) end
    return '[' .. table.concat(values, ',') .. ']'
end
local system, romHash = emu.getsystemid(), gameinfo.getromhash()
local pausedAfter, frameAfter = client.ispaused(), emu.framecount()
assert(type(pausedBefore) == 'boolean' and type(pausedAfter) == 'boolean', 'invalid pause evidence')
assert(type(frameBefore) == 'number' and type(frameAfter) == 'number', 'invalid frame evidence')
assert(pausedBefore and pausedAfter and frameBefore >= 0 and frameBefore == frameAfter, 'capture was not stationary and paused')
local response = '{"version":2,"nonce":' .. quote(nonce)
    .. ',"system":' .. quote(system)
    .. ',"rom_hash":' .. quote(romHash)
    .. ',"paused_before":' .. tostring(pausedBefore) .. ',"paused_after":' .. tostring(pausedAfter)
    .. ',"frame_before":' .. string.format('%d', frameBefore) .. ',"frame_after":' .. string.format('%d', frameAfter)
    .. ',"buttons":' .. array(buttons) .. ',"axes":' .. array(axes) .. '}\n'
assert(#response <= 1048576, 'oversized capture response')
return response
end
local ok, response = pcall(capture)
if not ok then
    local message = tostring(response)
    if #message == 0 then message = 'unspecified Lua capture failure' end
    if #message > 4096 then message = 'Lua capture failure message exceeds 4096 bytes' end
    response = '{"version":2,"nonce":' .. quote(nonce) .. ',"error":' .. quote(message) .. '}\n'
end
local partial = output .. '.partial'
local file = assert(io.open(partial, 'wb'))
assert(file:write(response))
assert(file:close())
assert(os.rename(partial, output))
"#,
    )
}

pub(crate) fn parse(text: &str, nonce: &str) -> Result<Definition> {
    validate_nonce(nonce)?;
    ensure!(
        text.len() <= 1024 * 1024,
        "Definition capture exceeds 1 MiB"
    );
    let envelope: serde_json::Value =
        serde_json::from_str(text).context("Parsing definition capture envelope")?;
    if envelope.get("error").is_some() {
        #[derive(Deserialize)]
        #[serde(deny_unknown_fields)]
        struct CaptureFailure {
            version: u8,
            nonce: String,
            error: String,
        }
        // Parse the original bytes again so duplicate fields are rejected.
        let failure: CaptureFailure = serde_json::from_str(text)?;
        ensure!(
            failure.version == 2 && failure.nonce == nonce,
            "Stale or unsupported capture failure response"
        );
        ensure!(
            !failure.error.is_empty() && failure.error.len() <= 4096,
            "Invalid capture failure message length"
        );
        anyhow::bail!(
            "Lua definition capture failed (escaped child message): {}",
            failure.error.escape_debug()
        );
    }
    let result: Definition =
        serde_json::from_str(text).context("Parsing loaded controller definition")?;
    ensure!(
        result.version == 2 && result.nonce == nonce,
        "Stale or unsupported definition capture"
    );
    ensure!(
        result.paused_before
            && result.paused_after
            && result.frame_before >= 0
            && result.frame_before == result.frame_after,
        "Definition capture was not stationary and paused"
    );
    ensure!(
        !result.system.is_empty()
            && result.system != "NULL"
            && result.system.len() <= 32
            && result.system.bytes().all(|b| b.is_ascii_alphanumeric()),
        "No valid loaded system in definition capture"
    );
    ensure!(
        !result.rom_hash.is_empty()
            && result.rom_hash.len() <= 256
            && !result.rom_hash.chars().any(char::is_control),
        "No bounded loaded-content hash in definition capture"
    );
    ensure!(
        !result.buttons.is_empty() && result.buttons.len() + result.axes.len() <= 1024,
        "Invalid captured control count"
    );
    let mut names = BTreeSet::new();
    for name in result.buttons.iter().chain(&result.axes) {
        ensure!(
            !name.is_empty()
                && name.len() <= 512
                && !name.chars().any(char::is_control)
                && names.insert(name),
            "Invalid or duplicate captured control name"
        );
    }
    Ok(result)
}
