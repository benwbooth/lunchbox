//! Owned native inspection process. Call only from a background launch task.
//! Private staging is not an OS sandbox: the selected runtime must be trusted.
use anyhow::{Context, Result, ensure};
use sha2::{Digest, Sha256};
use std::io::{Read, Write};
use std::path::{Component, Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::{Duration, Instant};

#[derive(Clone, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct InspectionInput {
    pub source: PathBuf,
    /// Exact relative destination, including its category directory.
    pub destination: PathBuf,
}

#[derive(serde::Serialize, serde::Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct InspectionRequest {
    pub retroarch: PathBuf,
    pub core: PathBuf,
    pub machine: String,
    /// Enable the native mouse class for evidence collection, not host capture.
    #[serde(default)]
    pub inspect_mouse: bool,
    /// Explicit complete ROM/BIOS/CHD dependencies and original cfg/ctrlr files.
    /// Dependency discovery belongs to the caller; this does not guess filenames.
    pub inputs: Vec<InspectionInput>,
    /// Opt-in exact roots for metadata queried from this same trusted core.
    /// Empty retains the explicit-manifest path and does not run discovery.
    #[serde(default)]
    pub dependency_roots: Vec<PathBuf>,
    pub core_options: String,
    #[serde(default)]
    pub environment: Vec<(std::ffi::OsString, std::ffi::OsString)>,
}

pub(crate) struct InspectionOutcome {
    pub directory: tempfile::TempDir,
    /// The complete manifest actually copied, including discovered dependencies.
    pub inputs: Vec<InspectionInput>,
    pub snapshot: super::ActiveFieldSnapshot,
    pub original_controller_config: Option<String>,
    pub original_game_config: Option<String>,
    pub source_hashes: Vec<(PathBuf, [u8; 32])>,
}

impl InspectionOutcome {
    /// Inspect the private native configuration tree after inspection, including
    /// global/default files that are not the selected game's explicit override.
    /// This does not attest to files later copied into the launch tree.
    pub(crate) fn validate_mouse_button_configuration(&self, cancel: &AtomicBool) -> Result<()> {
        validate_mouse_button_configuration_tree(self.directory.path(), &self.snapshot, cancel)
    }
}

/// Validate an owned inspection or final launch tree. This rechecks current
/// contents; it does not lock the tree against subsequent concurrent changes.
pub(crate) fn validate_mouse_button_configuration_tree(
    root: &Path,
    snapshot: &super::ActiveFieldSnapshot,
    cancel: &AtomicBool,
) -> Result<()> {
    snapshot.require_game_mouse_mode()?;
    let mut pending = vec![root.join("cfg"), root.join("ctrlr")];
    let mut nodes = 0usize;
    let mut bytes = 0usize;
    while let Some(path) = pending.pop() {
        ensure!(
            !cancel.load(Ordering::Relaxed),
            "MAME UI binding inspection cancelled"
        );
        nodes += 1;
        ensure!(
            nodes <= 4096,
            "MAME UI configuration tree exceeds entry limit"
        );
        let metadata = std::fs::symlink_metadata(&path)
            .with_context(|| format!("Inspect native configuration {}", path.display()))?;
        ensure!(
            !metadata.file_type().is_symlink(),
            "Native configuration symlink is unsupported: {}",
            path.display()
        );
        if metadata.is_dir() {
            for entry in std::fs::read_dir(&path)? {
                ensure!(
                    pending.len() + nodes < 4096,
                    "MAME UI configuration tree exceeds entry limit"
                );
                pending.push(entry?.path());
            }
        } else {
            ensure!(
                metadata.is_file(),
                "Unsupported native configuration entry: {}",
                path.display()
            );
            let relative = path.strip_prefix(root)?;
            let xml = staged_config(root, relative)?.with_context(|| {
                format!("Native configuration disappeared: {}", relative.display())
            })?;
            bytes = bytes
                .checked_add(xml.len())
                .context("Native configuration byte count overflow")?;
            ensure!(
                bytes <= 32 * 1024 * 1024,
                "MAME UI configurations exceed total byte limit"
            );
            super::relative_buttons::validate_saved_ui_mouse_bindings(&xml)
                .with_context(|| format!("Mouse-button isolation in {}", relative.display()))?;
        }
    }
    Ok(())
}

fn staged_config(root: &Path, relative: &Path) -> Result<Option<String>> {
    let path = root.join(relative);
    let mut file = match std::fs::File::open(path) {
        Ok(file) => file,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(None),
        Err(error) => return Err(error.into()),
    };
    let mut bytes = Vec::new();
    (&mut file)
        .take(8 * 1024 * 1024 + 1)
        .read_to_end(&mut bytes)?;
    ensure!(
        bytes.len() <= 8 * 1024 * 1024,
        "MAME configuration exceeds limit"
    );
    Ok(Some(String::from_utf8(bytes)?))
}

pub(super) fn check(cancel: &AtomicBool, deadline: Instant) -> Result<()> {
    ensure!(!cancel.load(Ordering::Relaxed), "MAME inspection cancelled");
    ensure!(Instant::now() < deadline, "MAME inspection timed out");
    Ok(())
}

fn fingerprint(
    path: &Path,
    mut copy: Option<&mut std::fs::File>,
    cancel: &AtomicBool,
    deadline: Instant,
) -> Result<[u8; 32]> {
    ensure!(path.is_absolute(), "Inspection source must be absolute");
    let mut input = std::fs::File::open(path)?;
    ensure!(
        input.metadata()?.is_file(),
        "Inspection source is not a file"
    );
    let mut hash = Sha256::new();
    let mut buffer = [0u8; 65536];
    loop {
        check(cancel, deadline)?;
        let count = input.read(&mut buffer)?;
        if count == 0 {
            break;
        }
        hash.update(&buffer[..count]);
        if let Some(output) = copy.as_mut() {
            output.write_all(&buffer[..count])?;
        }
    }
    Ok(hash.finalize().into())
}

pub(super) fn config_path(path: &Path) -> Result<String> {
    let value = path.to_str().context("Non-UTF-8 inspection path")?;
    ensure!(
        !value
            .chars()
            .any(|c| c.is_control() || c == '"' || c == '\\'),
        "Unsupported inspection configuration path"
    );
    Ok(format!("\"{value}\""))
}

pub(super) struct Worker(pub(super) std::process::Child);

impl Drop for Worker {
    fn drop(&mut self) {
        let _ = self.0.kill();
        let _ = self.0.wait();
    }
}

pub(crate) fn inspect_runtime(
    request: &InspectionRequest,
    timeout: Duration,
    cancel: &AtomicBool,
) -> Result<InspectionOutcome> {
    use std::process::{Command, Stdio};
    ensure!(
        (Duration::from_secs(1)..=Duration::from_secs(300)).contains(&timeout),
        "MAME inspection timeout must be between 1 and 300 seconds"
    );
    ensure!(
        request.inputs.len() <= 4096,
        "Too many MAME inspection inputs"
    );
    ensure!(
        request.core_options.len() <= 1024 * 1024,
        "MAME options exceed limit"
    );
    let deadline = Instant::now() + timeout;
    check(cancel, deadline)?;
    super::validate_inspection_options(&request.core_options, request.inspect_mouse)?;
    let directory = tempfile::Builder::new()
        .prefix("lunchbox-mame-inspection-")
        .tempdir()?;
    let root = directory.path().canonicalize()?;
    for name in [
        "cfg",
        "ctrlr",
        "roms",
        "nvram",
        "diff",
        "states",
        "snaps",
        "input",
        "system",
        "saves",
        "temporary",
        "home",
    ] {
        std::fs::create_dir(root.join(name))?;
    }
    let mut originals = Vec::new();
    for path in [&request.retroarch, &request.core] {
        originals.push((path.clone(), fingerprint(path, None, cancel, deadline)?));
    }
    let inputs = if request.dependency_roots.is_empty() {
        request.inputs.clone()
    } else {
        let discovered =
            super::metadata::discover(request, &request.dependency_roots, deadline, cancel)?;
        let mut inputs = std::collections::BTreeMap::new();
        for input in &request.inputs {
            ensure!(
                inputs
                    .insert(input.destination.clone(), input.clone())
                    .is_none(),
                "Duplicate declared MAME inspection destination"
            );
        }
        // Discovery identifies mandatory files. Keep explicitly supplied
        // optional images and other inputs even when metadata omits them from
        // that mandatory set. A destination never silently changes its source.
        for input in discovered {
            if let Some(declared) = inputs.get(&input.destination) {
                ensure!(
                    declared == &input,
                    "Discovered MAME dependency conflicts with the declared source at {}: {} versus {}",
                    input.destination.display(),
                    declared.source.display(),
                    input.source.display()
                );
            } else {
                inputs.insert(input.destination.clone(), input);
            }
        }
        ensure!(
            inputs.len() <= 4096,
            "Discovered MAME manifest exceeds 4096 files"
        );
        inputs.into_values().collect()
    };
    let mut destinations = std::collections::BTreeSet::new();
    let mut has_controller = false;
    for item in &inputs {
        check(cancel, deadline)?;
        let parts: Vec<_> = item.destination.components().collect();
        ensure!(
            parts.len() >= 2 && parts.iter().all(|p| matches!(p, Component::Normal(_))),
            "Inspection destination must be a contained relative file"
        );
        let category = parts[0].as_os_str().to_str().unwrap_or("");
        ensure!(
            matches!(
                category,
                "roms" | "cfg" | "ctrlr" | "system" | "nvram" | "diff"
            ),
            "Unsupported inspection input category"
        );
        if category == "ctrlr" {
            ensure!(
                item.destination == Path::new("ctrlr/lunchbox-original.cfg"),
                "Unexpected controller profile destination"
            );
            has_controller = true;
        }
        ensure!(
            destinations.insert(item.destination.clone()),
            "Duplicate inspection destination"
        );
        let target = root.join(&item.destination);
        std::fs::create_dir_all(target.parent().context("Missing staging parent")?)?;
        let mut output = std::fs::OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&target)?;
        let digest = fingerprint(&item.source, Some(&mut output), cancel, deadline)?;
        output.sync_all()?;
        originals.push((item.source.clone(), digest));
    }
    ensure!(
        destinations.iter().any(|p| p.starts_with("roms")),
        "MAME inspection requires explicit ROM dependencies"
    );
    let command = super::inspection::inspection_command_with_mouse(
        &root,
        &request.machine,
        has_controller,
        request.inspect_mouse,
    )?;
    // Capture the copied baseline before the emulator can modify it. The mapper
    // must never reopen a user's configuration after inspecting different bytes.
    let original_controller_config =
        staged_config(&root, Path::new("ctrlr/lunchbox-original.cfg"))?;
    let original_game_config = staged_config(
        &root,
        &PathBuf::from(format!("cfg/{}.cfg", request.machine)),
    )?;
    let result_path = root.join("fields.json");
    std::fs::write(
        root.join("inspect.lua"),
        super::active_field_script(&request.machine, &result_path)?,
    )?;
    std::fs::write(root.join("inspect.cmd"), command)?;
    std::fs::write(root.join("core-options.cfg"), &request.core_options)?;
    let mut config = String::from(
        "config_save_on_exit = \"false\"\nvideo_driver = \"null\"\naudio_driver = \"null\"\nmenu_driver = \"null\"\ninput_driver = \"null\"\ninput_joypad_driver = \"null\"\nvideo_throttle_enable = \"false\"\nsavestate_auto_load = \"false\"\nsavestate_auto_save = \"false\"\n",
    );
    for (key, child) in [
        ("system_directory", "system"),
        ("savefile_directory", "saves"),
        ("savestate_directory", "states"),
        ("screenshot_directory", "snaps"),
        ("core_options_path", "core-options.cfg"),
    ] {
        config.push_str(&format!("{key} = {}\n", config_path(&root.join(child))?));
    }
    std::fs::write(root.join("retroarch.cfg"), config)?;
    // Reject inputs changed during staging before invoking any native code.
    for (path, expected) in &originals {
        ensure!(
            fingerprint(path, None, cancel, deadline)? == *expected,
            "MAME inspection input changed: {}",
            path.display()
        );
    }
    let mut worker = Worker(
        Command::new(&request.retroarch)
            .arg("--config")
            .arg(root.join("retroarch.cfg"))
            .arg("--libretro")
            .arg(&request.core)
            .arg(root.join("inspect.cmd"))
            .envs(request.environment.iter().cloned())
            .env("HOME", root.join("home"))
            .env("XDG_CONFIG_HOME", root.join("home"))
            .env("TMPDIR", root.join("temporary"))
            .env("TMP", root.join("temporary"))
            .env("TEMP", root.join("temporary"))
            .current_dir(&root)
            .stdin(Stdio::null())
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .spawn()
            .context("Starting native MAME field inspection")?,
    );
    loop {
        check(cancel, deadline)?;
        if let Some(status) = worker.0.try_wait()? {
            ensure!(status.success(), "MAME inspection failed: {status}");
            break;
        }
        std::thread::sleep(Duration::from_millis(20));
    }
    for (path, expected) in &originals {
        ensure!(
            fingerprint(path, None, cancel, deadline)? == *expected,
            "MAME inspection input changed: {}",
            path.display()
        );
    }
    let mut bytes = Vec::new();
    std::fs::File::open(result_path)
        .context("MAME did not produce a field snapshot")?
        .take(16 * 1024 * 1024 + 1)
        .read_to_end(&mut bytes)?;
    check(cancel, deadline)?;
    let snapshot = super::ActiveFieldSnapshot::parse(&bytes, &request.machine)?;
    if request.inspect_mouse {
        snapshot
            .mouse_routes()
            .context("Requested mouse inspection did not establish enabled native mouse routes")?;
    }
    Ok(InspectionOutcome {
        directory,
        inputs,
        snapshot,
        original_controller_config,
        original_game_config,
        source_hashes: originals,
    })
}
