//! Exact target-runtime bridge for the supported Snes9x GTK Flatpak.
//!
//! A host SDL inventory is not interchangeable with a Flatpak inventory. The
//! staged controller helper is therefore run through the target runtime's own
//! dynamic loader. The same helper then supervises the final emulator child
//! from inside that sandbox and writes a bounded readiness receipt.

use super::{isolation::PreparedConfig, settings::SavedSetup};
use crate::{
    controller_native_process::{cancelled, capture},
    emulator::{LaunchPlan, RomEmulatorOption},
    platform_process::{host_command, is_flatpak},
};
use anyhow::{Context, Result, ensure};
use lunchbox_controller_probe::{
    file_hash,
    sdl2::Snapshot,
    snes9x_supervisor::{Receipt, Request},
};
use std::{
    ffi::{OsStr, OsString},
    fs::OpenOptions,
    io::{Read, Write},
    os::unix::fs::{MetadataExt, OpenOptionsExt, PermissionsExt},
    path::{Path, PathBuf},
    sync::atomic::AtomicBool,
};

const APP_ID: &str = "com.snes9x.Snes9x";
const APP_EXECUTABLE: &str = "bin/snes9x-gtk";
const OUTPUT_LIMIT: u64 = 8 * 1024 * 1024;

#[derive(Clone, Debug, Eq, PartialEq)]
struct Deployment {
    commit: String,
    location: PathBuf,
    runtime: Option<String>,
}

struct Supervisor {
    request_path: PathBuf,
    request_bytes: Vec<u8>,
    receipt_path: PathBuf,
    expected_receipt: Receipt,
}

pub(crate) struct PreparedFlatpak {
    command: PathBuf,
    app_id: String,
    app: Deployment,
    runtime: Deployment,
    executable: PathBuf,
    executable_sha256: String,
    sdl_library: PathBuf,
    sdl_library_sha256: String,
    sandbox_sdl_library: PathBuf,
    loader: PathBuf,
    loader_sha256: String,
    sandbox_loader: PathBuf,
    probe_source: PathBuf,
    probe_sha256: String,
    staged_probe: PathBuf,
    _probe_directory: tempfile::TempDir,
    supervisor: Option<Supervisor>,
}

fn text_output(command: &Path, target: &str, flag: &str, cancel: &AtomicBool) -> Result<String> {
    let mut process = host_command(command);
    process.arg("info").arg(flag).arg(target);
    let (output, _) = capture(&mut process, cancel)?;
    ensure!(
        output.len() <= 4096,
        "Flatpak deployment field is oversized"
    );
    let text = std::str::from_utf8(&output)?.trim();
    ensure!(
        !text.is_empty() && !text.contains(['\0', '\r', '\n']),
        "Flatpak deployment field is malformed"
    );
    Ok(text.to_owned())
}

fn deployment(
    command: &Path,
    target: &str,
    include_runtime: bool,
    cancel: &AtomicBool,
) -> Result<Deployment> {
    let commit = text_output(command, target, "--show-commit", cancel)?;
    ensure!(
        commit.len() == 64 && commit.bytes().all(|byte| byte.is_ascii_hexdigit()),
        "Flatpak deployment commit is malformed"
    );
    let location = PathBuf::from(text_output(command, target, "--show-location", cancel)?);
    ensure!(
        location.is_absolute(),
        "Flatpak deployment location is not absolute"
    );
    let runtime = include_runtime
        .then(|| text_output(command, target, "--show-runtime", cancel))
        .transpose()?;
    Ok(Deployment {
        commit,
        location,
        runtime,
    })
}

fn current_path_for_host(path: &Path) -> PathBuf {
    if is_flatpak() {
        let bridged = Path::new("/run/host").join(path.strip_prefix("/").unwrap_or(path));
        if bridged.try_exists().unwrap_or(false) {
            return bridged;
        }
    }
    path.to_path_buf()
}

fn logical_host_path(path: &Path) -> PathBuf {
    if is_flatpak()
        && let Ok(relative) = path.strip_prefix("/run/host")
    {
        return Path::new("/").join(relative);
    }
    path.to_path_buf()
}

fn same_program(left: &Path, right: &Path) -> bool {
    left == right
        || left
            .canonicalize()
            .ok()
            .zip(right.canonicalize().ok())
            .is_some_and(|(left, right)| left == right)
}

fn parse_original<'a>(
    app_id: &str,
    setup: &SavedSetup,
    original: &'a LaunchPlan,
) -> Result<&'a Path> {
    ensure!(
        original.environment.is_empty()
            && original.retroarch_content.is_none()
            && original.arguments.len() == 4
            && original.arguments[0] == "run"
            && original.arguments[2] == app_id,
        "Snes9x Flatpak launch requires the ordinary one-ROM argument plan"
    );
    let grant = original.arguments[1]
        .to_str()
        .and_then(|value| value.strip_prefix("--filesystem="))
        .context("Snes9x Flatpak launch is missing its ROM-directory grant")?;
    ensure!(
        !grant.contains(':')
            && Path::new(grant).canonicalize()? == original.current_directory.canonicalize()?,
        "Snes9x Flatpak launch grants a directory other than its exact launch directory"
    );
    let content = Path::new(&original.arguments[3]);
    ensure!(
        content.canonicalize()? == setup.content.canonicalize()?
            && content.parent().is_some_and(|parent| {
                parent.canonicalize().ok().as_ref()
                    == original.current_directory.canonicalize().ok().as_ref()
            }),
        "Snes9x Flatpak launch does not select the saved ROM exactly once"
    );
    Ok(content)
}

fn runtime_triplet(runtime: &str) -> Result<(&str, &'static str)> {
    let parts: Vec<_> = runtime.split('/').collect();
    ensure!(
        parts.len() == 3 && parts[0] == "org.freedesktop.Platform",
        "Snes9x Flatpak runtime is outside the supported Freedesktop contract"
    );
    let loader = match parts[1] {
        "x86_64" => "ld-linux-x86-64.so.2",
        "aarch64" => "ld-linux-aarch64.so.1",
        _ => anyhow::bail!("Snes9x Flatpak architecture has no supported loader contract"),
    };
    ensure!(
        parts[1] == std::env::consts::ARCH,
        "Snes9x Flatpak architecture differs from Lunchbox"
    );
    Ok((parts[1], loader))
}

fn controller_cache() -> Result<PathBuf> {
    let home = directories::BaseDirs::new()
        .context("Finding the user home for Flatpak controller staging")?
        .home_dir()
        .to_path_buf();
    let cache = home.join(".cache/lunchbox/controller-launch");
    std::fs::create_dir_all(&cache)?;
    Ok(cache)
}

impl PreparedFlatpak {
    pub(crate) fn prepare(
        setup: &SavedSetup,
        option: &RomEmulatorOption,
        original: &LaunchPlan,
        command: &Path,
        app_id: &str,
        cancel: &AtomicBool,
    ) -> Result<Self> {
        cancelled(cancel)?;
        ensure!(
            app_id == APP_ID && setup.emulator_id == option.emulator_id,
            "Snes9x Flatpak adapter only supports the exact Flathub GTK application"
        );
        let expected_config = directories::BaseDirs::new()
            .context("Finding the Snes9x Flatpak profile")?
            .home_dir()
            .join(".var/app")
            .join(app_id)
            .join("config/snes9x/snes9x.conf");
        ensure!(
            expected_config.canonicalize()? == setup.source_config.canonicalize()?,
            "Snes9x saved config differs from the target Flatpak profile"
        );
        ensure!(
            same_program(command, &original.program),
            "Snes9x Flatpak launch command differs from the selected runtime"
        );
        parse_original(app_id, setup, original)?;
        let app = deployment(command, app_id, true, cancel)?;
        let runtime_ref = app
            .runtime
            .as_deref()
            .context("Snes9x Flatpak deployment omitted its runtime")?;
        let (triplet, loader_name) = runtime_triplet(runtime_ref)?;
        let runtime = deployment(command, runtime_ref, false, cancel)?;

        let app_root = current_path_for_host(&app.location).canonicalize()?;
        let executable = app_root.join("files").join(APP_EXECUTABLE).canonicalize()?;
        let executable_sha256 = file_hash(&executable)?;
        ensure!(
            executable_sha256.eq_ignore_ascii_case(&setup.executable_sha256),
            "Snes9x Flatpak executable differs from the saved trusted GTK runtime"
        );

        let runtime_root = current_path_for_host(&runtime.location).canonicalize()?;
        let runtime_files = runtime_root.join("files").canonicalize()?;
        let accessible_sdl = setup.sdl_library.canonicalize()?;
        let logical_sdl = logical_host_path(&accessible_sdl);
        let logical_runtime_files = logical_host_path(&runtime_files);
        let relative_sdl = logical_sdl
            .strip_prefix(&logical_runtime_files)
            .context("Saved Snes9x SDL is not in the active target Flatpak runtime")?;
        ensure!(
            relative_sdl
                .starts_with(Path::new("lib"))
                .then_some(())
                .is_some()
                && relative_sdl
                    .file_name()
                    .and_then(OsStr::to_str)
                    .is_some_and(|name| name.starts_with("libSDL2-2.0.so.0")),
            "Saved Snes9x SDL path is outside the target runtime's SDL2 contract"
        );
        let sandbox_sdl_library = Path::new("/usr").join(relative_sdl);
        let sdl_library_sha256 = file_hash(&accessible_sdl)?;

        let loader = runtime_files
            .join("lib")
            .join(format!("{triplet}-linux-gnu"))
            .join(loader_name)
            .canonicalize()?;
        let sandbox_loader = Path::new("/usr/lib")
            .join(format!("{triplet}-linux-gnu"))
            .join(loader_name);
        let loader_sha256 = file_hash(&loader)?;

        let probe_source = setup.probe_program.canonicalize()?;
        ensure!(
            std::fs::metadata(&probe_source)?.is_file(),
            "Snes9x controller probe is not a regular file"
        );
        let probe_sha256 = file_hash(&probe_source)?;
        let probe_directory = tempfile::Builder::new()
            .prefix("snes9x-flatpak-probe-")
            .tempdir_in(controller_cache()?)?;
        let staged_probe = probe_directory.path().join("lunchbox-controller-probe");
        std::fs::copy(&probe_source, &staged_probe)?;
        let mut permissions = std::fs::metadata(&staged_probe)?.permissions();
        permissions.set_mode(0o500);
        std::fs::set_permissions(&staged_probe, permissions)?;
        ensure!(
            file_hash(&staged_probe)? == probe_sha256,
            "Staged Snes9x controller probe differs from its trusted source"
        );

        let prepared = Self {
            command: command.to_path_buf(),
            app_id: app_id.to_owned(),
            app,
            runtime,
            executable,
            executable_sha256,
            sdl_library: accessible_sdl,
            sdl_library_sha256,
            sandbox_sdl_library,
            loader,
            loader_sha256,
            sandbox_loader,
            probe_source,
            probe_sha256,
            staged_probe,
            _probe_directory: probe_directory,
            supervisor: None,
        };
        prepared.verify(cancel)?;
        Ok(prepared)
    }

    pub(crate) fn executable(&self) -> &Path {
        &self.executable
    }

    pub(crate) fn observe(&self, path: Option<&str>, cancel: &AtomicBool) -> Result<Snapshot> {
        self.verify(cancel)?;
        let probe_root = self
            .staged_probe
            .parent()
            .context("Staged Snes9x probe has no directory")?;
        let mut command = host_command(&self.command);
        command
            .arg("run")
            .arg("--die-with-parent")
            .arg("--nofilesystem=host:reset")
            .arg("--nofilesystem=home")
            .arg(format!("--filesystem={}:ro", probe_root.display()))
            .arg("--env=SDL_LINUX_JOYSTICK_CLASSIC=1")
            .arg(format!("--command={}", self.sandbox_loader.display()))
            .arg(&self.app_id)
            .arg(&self.staged_probe)
            .arg("--sdl2-inventory")
            .arg("--sdl-library")
            .arg(&self.sandbox_sdl_library);
        if let Some(path) = path {
            command.arg("--sdl2-controls-for-path").arg(path);
        }
        let (output, _) = capture(&mut command, cancel)?;
        let snapshot: Snapshot =
            serde_json::from_slice(&output).context("Invalid sandbox Snes9x SDL capture")?;
        ensure!(
            snapshot.library == self.sandbox_sdl_library
                && snapshot.library_sha256 == self.sdl_library_sha256,
            "Snes9x Flatpak helper inspected a different SDL runtime"
        );
        self.verify(cancel)?;
        Ok(snapshot)
    }

    pub(crate) fn prepare_launch(
        &mut self,
        setup: &SavedSetup,
        configuration: &PreparedConfig,
        inventory: &Snapshot,
        runtime_paths: &[String],
        original: &LaunchPlan,
    ) -> Result<Vec<OsString>> {
        ensure!(
            self.supervisor.is_none(),
            "Snes9x Flatpak launch was already staged"
        );
        let content = parse_original(&self.app_id, setup, original)?;
        configuration.verify()?;
        let request_path = configuration.root().join("supervisor.json");
        let receipt_path = configuration.root().join("ready.json");
        ensure!(
            !request_path.try_exists()? && !receipt_path.try_exists()?,
            "Snes9x Flatpak supervisor staging already exists"
        );
        let request = Request {
            schema_version: 1,
            executable: PathBuf::from("/app/bin/snes9x-gtk"),
            executable_sha256: self.executable_sha256.clone(),
            sdl_library: self.sandbox_sdl_library.clone(),
            sdl_library_sha256: self.sdl_library_sha256.clone(),
            config_home: configuration.root().to_path_buf(),
            config_sha256: file_hash(&configuration.config_path())?,
            content: content.to_path_buf(),
            current_directory: content
                .parent()
                .context("Snes9x Flatpak ROM has no parent directory")?
                .to_path_buf(),
            runtime_paths: runtime_paths.to_vec(),
            inventory: inventory.clone(),
            receipt: receipt_path.clone(),
        };
        let request_bytes = serde_json::to_vec(&request)?;
        ensure!(
            request_bytes.len() as u64 <= OUTPUT_LIMIT,
            "Snes9x Flatpak supervisor request is oversized"
        );
        let mut request_file = OpenOptions::new()
            .write(true)
            .create_new(true)
            .mode(0o600)
            .open(&request_path)?;
        request_file.set_permissions(std::fs::Permissions::from_mode(0o600))?;
        request_file.write_all(&request_bytes)?;
        request_file.sync_all()?;
        self.supervisor = Some(Supervisor {
            request_path: request_path.clone(),
            request_bytes,
            receipt_path,
            expected_receipt: request.expected_receipt(),
        });

        let probe_root = self
            .staged_probe
            .parent()
            .context("Staged Snes9x probe has no directory")?;
        let mut arguments = vec![
            original.arguments[0].clone(),
            OsString::from("--die-with-parent"),
            OsString::from("--nofilesystem=host:reset"),
            OsString::from("--nofilesystem=home"),
            original.arguments[1].clone(),
        ];
        arguments.extend([
            OsString::from(format!("--filesystem={}:ro", probe_root.display())),
            OsString::from(format!("--filesystem={}", configuration.root().display())),
            OsString::from(format!("--env=HOME={}", configuration.root().display())),
            OsString::from("--env=SDL_LINUX_JOYSTICK_CLASSIC=1"),
            OsString::from(format!("--command={}", self.sandbox_loader.display())),
            OsString::from(&self.app_id),
            self.staged_probe.as_os_str().to_owned(),
            OsString::from("--snes9x-supervise"),
            request_path.into_os_string(),
        ]);
        Ok(arguments)
    }

    pub(crate) fn receipt_ready(&self) -> Result<bool> {
        let Some(supervisor) = &self.supervisor else {
            anyhow::bail!("Snes9x Flatpak supervisor was not staged");
        };
        if !supervisor.receipt_path.try_exists()? {
            return Ok(false);
        }
        let file = std::fs::File::open(&supervisor.receipt_path)?;
        ensure!(
            file.metadata()?.is_file() && file.metadata()?.mode() & 0o7777 == 0o600,
            "Snes9x readiness receipt is not a private regular file"
        );
        let mut bytes = Vec::new();
        file.take(OUTPUT_LIMIT + 1).read_to_end(&mut bytes)?;
        ensure!(
            bytes.len() as u64 <= OUTPUT_LIMIT,
            "Snes9x readiness receipt is oversized"
        );
        let receipt: Receipt =
            serde_json::from_slice(&bytes).context("Invalid Snes9x readiness receipt")?;
        ensure!(
            receipt == supervisor.expected_receipt,
            "Snes9x readiness receipt differs from the prepared target routing"
        );
        Ok(true)
    }

    pub(crate) fn verify(&self, cancel: &AtomicBool) -> Result<()> {
        cancelled(cancel)?;
        ensure!(
            deployment(&self.command, &self.app_id, true, cancel)? == self.app,
            "Snes9x Flatpak application deployment changed"
        );
        let runtime_ref = self
            .app
            .runtime
            .as_deref()
            .context("Snes9x Flatpak runtime identity disappeared")?;
        ensure!(
            deployment(&self.command, runtime_ref, false, cancel)? == self.runtime,
            "Snes9x Flatpak runtime deployment changed"
        );
        ensure!(
            file_hash(&self.executable)? == self.executable_sha256
                && file_hash(&self.sdl_library)? == self.sdl_library_sha256
                && file_hash(&self.loader)? == self.loader_sha256
                && file_hash(&self.probe_source)? == self.probe_sha256
                && file_hash(&self.staged_probe)? == self.probe_sha256,
            "Snes9x Flatpak launch input or runtime changed"
        );
        if let Some(supervisor) = &self.supervisor {
            let request_metadata = std::fs::symlink_metadata(&supervisor.request_path)?;
            ensure!(
                request_metadata.is_file()
                    && request_metadata.mode() & 0o7777 == 0o600
                    && std::fs::read(&supervisor.request_path)? == supervisor.request_bytes,
                "Snes9x Flatpak supervisor request or mode changed"
            );
            if supervisor.receipt_path.try_exists()? {
                self.receipt_ready()?;
            }
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        controller_catalog::{Calibration, InputBinding, NativeInput, catalog},
        controllers::ControllerDevice,
        emulator::EmulatorExecutable,
    };
    use anyhow::Context;
    use std::{
        collections::{BTreeMap, HashMap},
        io::{BufRead, BufReader},
        os::unix::fs::FileTypeExt,
        process::{Child, Command, Stdio},
        time::{Duration, Instant},
    };

    struct ChildGuard {
        child: Child,
    }

    impl ChildGuard {
        fn new(child: Child) -> Self {
            Self { child }
        }
    }

    impl Drop for ChildGuard {
        fn drop(&mut self) {
            if self.child.try_wait().ok().flatten().is_none() {
                let _ = self.child.kill();
                let _ = self.child.wait();
            }
        }
    }

    fn setup(content: &Path) -> SavedSetup {
        SavedSetup {
            emulator_id: "snes9x".into(),
            content: content.to_path_buf(),
            source_config: "/tmp/snes9x/snes9x.conf".into(),
            probe_program: "/tmp/probe".into(),
            sdl_library: "/tmp/libSDL2.so".into(),
            bubblewrap_program: "/tmp/bwrap".into(),
            executable_sha256: "0".repeat(64),
            players: vec![super::super::settings::Player {
                player: 1,
                controller_id: "pad".into(),
            }],
        }
    }

    #[test]
    fn ordinary_flatpak_plan_requires_one_exact_rom_and_grant() {
        let directory = tempfile::TempDir::new().unwrap();
        let rom = directory.path().join("game.sfc");
        std::fs::write(&rom, b"rom").unwrap();
        let original = LaunchPlan {
            emulator_name: "Snes9x".into(),
            program: "flatpak".into(),
            arguments: vec![
                "run".into(),
                format!("--filesystem={}", directory.path().display()).into(),
                APP_ID.into(),
                rom.as_os_str().to_owned(),
            ],
            current_directory: directory.path().to_path_buf(),
            environment: vec![],
            cleanup_paths: vec![],
            retroarch_content: None,
        };
        assert_eq!(
            parse_original(APP_ID, &setup(&rom), &original).unwrap(),
            rom
        );
        let mut custom = original;
        custom
            .arguments
            .insert(2, "--env=SDL_VIDEODRIVER=dummy".into());
        assert!(parse_original(APP_ID, &setup(&rom), &custom).is_err());
    }

    fn oracle_calibration() -> Calibration {
        let button_for = BTreeMap::from([
            ("up", 0_u32),
            ("down", 1),
            ("left", 2),
            ("right", 3),
            ("start", 4),
            ("select", 5),
            ("a", 6),
            ("b", 7),
            ("x", 8),
            ("y", 9),
            ("l", 10),
            ("r", 11),
        ]);
        let layout = catalog().layout("snes").unwrap();
        Calibration {
            target_mappings: BTreeMap::new(),
            layout: "snes".into(),
            os: "linux".into(),
            backend: "gilrs-0.11".into(),
            bindings: layout
                .controls
                .iter()
                .map(|control| {
                    let button = button_for[control.id.as_str()];
                    (
                        control.id.clone(),
                        InputBinding {
                            code: button,
                            kind: "button".into(),
                            direction: 0,
                            logical: control.label.clone(),
                            native: Some(NativeInput {
                                code: 0x1_0000 | (0x130 + button),
                                direction: 0,
                            }),
                            axis: None,
                        },
                    )
                })
                .collect(),
        }
    }

    fn state_bytes(root: &Path) -> Result<BTreeMap<PathBuf, String>> {
        fn visit(
            root: &Path,
            directory: &Path,
            result: &mut BTreeMap<PathBuf, String>,
        ) -> Result<()> {
            ensure!(result.len() < 4096, "Snes9x state snapshot is oversized");
            for entry in std::fs::read_dir(directory)? {
                let entry = entry?;
                let path = entry.path();
                let relative = path.strip_prefix(root)?.to_path_buf();
                let metadata = std::fs::symlink_metadata(&path)?;
                if metadata.is_dir() {
                    result.insert(relative, "directory".into());
                    visit(root, &path, result)?;
                } else if metadata.is_file() {
                    result.insert(relative, format!("file:{}", file_hash(&path)?));
                } else if metadata.file_type().is_symlink() {
                    result.insert(
                        relative,
                        format!("symlink:{}", std::fs::read_link(&path)?.display()),
                    );
                }
            }
            Ok(())
        }
        let mut result = BTreeMap::new();
        visit(root, root, &mut result)?;
        Ok(result)
    }

    fn process_parent(pid: u32) -> Result<u32> {
        let status = std::fs::read_to_string(format!("/proc/{pid}/status"))?;
        let value = status
            .lines()
            .find_map(|line| line.strip_prefix("PPid:"))
            .context("Process status omitted PPid")?;
        Ok(value.trim().parse()?)
    }

    fn process_ticks(pid: u32) -> Result<u64> {
        let stat = std::fs::read_to_string(format!("/proc/{pid}/stat"))?;
        let fields: Vec<_> = stat
            .rsplit_once(") ")
            .context("Process stat omitted command terminator")?
            .1
            .split_whitespace()
            .collect();
        ensure!(fields.len() > 12, "Process stat is truncated");
        Ok(fields[11].parse::<u64>()? + fields[12].parse::<u64>()?)
    }

    fn exact_flatpak_child(flatpak: &Path, rom: &Path) -> Result<u32> {
        let deadline = Instant::now() + Duration::from_secs(5);
        let child = loop {
            let mut matches = Vec::new();
            for entry in std::fs::read_dir("/proc")? {
                let entry = entry?;
                let Some(pid) = entry
                    .file_name()
                    .to_str()
                    .and_then(|name| name.parse().ok())
                else {
                    continue;
                };
                let Ok(command_line) = std::fs::read(entry.path().join("cmdline")) else {
                    continue;
                };
                let arguments: Vec<_> = command_line
                    .split(|byte| *byte == 0)
                    .filter(|argument| !argument.is_empty())
                    .collect();
                if arguments.len() == 2
                    && arguments[0] == b"/app/bin/snes9x-gtk"
                    && arguments[1] == rom.as_os_str().as_encoded_bytes()
                {
                    matches.push(pid);
                }
            }
            ensure!(
                matches.len() <= 1,
                "Found multiple exact Snes9x oracle children"
            );
            if let Some(child) = matches.first() {
                break *child;
            }
            ensure!(
                Instant::now() < deadline,
                "The exact Snes9x oracle child did not become visible"
            );
            std::thread::sleep(Duration::from_millis(25));
        };

        let mut ancestors = vec![child];
        let mut ancestor = child;
        for _ in 0..32 {
            ancestor = process_parent(ancestor)?;
            if ancestor == 0 {
                break;
            }
            ancestors.push(ancestor);
        }
        ensure!(
            ancestors.len() < 33,
            "Snes9x Flatpak ancestry exceeds its bound"
        );

        loop {
            let output = Command::new(flatpak)
                .args(["ps", "--columns=pid,application"])
                .output()?;
            ensure!(
                output.status.success(),
                "Could not inspect Flatpak processes"
            );
            let roots: Vec<_> = std::str::from_utf8(&output.stdout)?
                .lines()
                .filter_map(|line| {
                    let mut fields = line.split_whitespace();
                    let pid = fields.next()?.parse::<u32>().ok()?;
                    let app = fields.next()?;
                    (app == APP_ID && fields.next().is_none() && ancestors.contains(&pid))
                        .then_some(pid)
                })
                .collect();
            ensure!(
                roots.len() <= 1,
                "Exact Snes9x child belongs to multiple Flatpak roots"
            );
            if roots.len() == 1 {
                return Ok(child);
            }
            ensure!(
                Instant::now() < deadline,
                "Exact Snes9x child is outside every reported Flatpak tree"
            );
            std::thread::sleep(Duration::from_millis(25));
        }
    }

    /// Opt-in installed-runtime proof. This deliberately requires checked-in
    /// oracle artifacts supplied by the caller and refuses a pre-existing
    /// Snes9x process. It exercises the production Flatpak prepare/spawn path,
    /// not a separately assembled launch command.
    #[test]
    #[ignore = "needs installed Snes9x, /dev/uinput, display and oracle artifacts"]
    fn production_flatpak_two_pad_oracle_preserves_user_state() -> Result<()> {
        use std::io::Write;

        let rom = PathBuf::from(
            std::env::var_os("LUNCHBOX_SNES9X_FLATPAK_ORACLE_ROM")
                .context("Missing Snes9x oracle ROM")?,
        )
        .canonicalize()?;
        let probe = PathBuf::from(
            std::env::var_os("LUNCHBOX_SNES9X_FLATPAK_ORACLE_PROBE")
                .context("Missing Snes9x oracle probe")?,
        )
        .canonicalize()?;
        let pad_driver = PathBuf::from(
            std::env::var_os("LUNCHBOX_SNES9X_FLATPAK_ORACLE_PAD_DRIVER")
                .context("Missing Snes9x oracle pad driver")?,
        )
        .canonicalize()?;
        let window_tool = PathBuf::from(
            std::env::var_os("LUNCHBOX_SNES9X_FLATPAK_ORACLE_WINDOW_TOOL")
                .context("Missing compositor window tool for graceful oracle shutdown")?,
        )
        .canonicalize()?;
        let input_tool = PathBuf::from(
            std::env::var_os("LUNCHBOX_SNES9X_FLATPAK_ORACLE_INPUT_TOOL")
                .context("Missing compositor input tool for continuing the oracle")?,
        )
        .canonicalize()?;
        let x11 = window_tool == input_tool
            && window_tool
                .file_name()
                .is_some_and(|name| name == "xdotool");
        let input_socket = if x11 {
            None
        } else {
            let socket = PathBuf::from(
                std::env::var_os("LUNCHBOX_SNES9X_FLATPAK_ORACLE_INPUT_SOCKET")
                    .context("Missing compositor input socket for continuing the oracle")?,
            )
            .canonicalize()?;
            ensure!(
                std::fs::metadata(&socket)?.file_type().is_socket(),
                "Oracle input socket is not a Unix socket"
            );
            Some(socket)
        };
        let flatpak = PathBuf::from("/run/current-system/sw/bin/flatpak").canonicalize()?;
        let cancel = AtomicBool::new(false);
        let running = Command::new(&flatpak).arg("ps").output()?;
        ensure!(
            !String::from_utf8_lossy(&running.stdout).contains(APP_ID),
            "Refusing to mix the oracle with an existing Snes9x process"
        );

        let home = directories::BaseDirs::new().context("Finding user home")?;
        let profile = home.home_dir().join(".var/app").join(APP_ID);
        let before = state_bytes(&profile)?;
        let source_config = profile.join("config/snes9x/snes9x.conf").canonicalize()?;

        let mut driver = ChildGuard::new(
            Command::new(pad_driver)
                .stdin(Stdio::piped())
                .stdout(Stdio::piped())
                .spawn()?,
        );
        let mut lines = BufReader::new(driver.child.stdout.take().unwrap()).lines();
        let mut pads = Vec::new();
        for number in 1..=2 {
            let line = lines.next().context("Pad driver exited early")??;
            let fields: Vec<_> = line.split('\t').collect();
            ensure!(
                fields.len() == 3 && fields[0] == format!("P{number}"),
                "Unexpected pad-driver identity"
            );
            pads.push((fields[1].to_owned(), fields[2].to_owned()));
        }
        ensure!(
            lines.next().context("Pad driver omitted readiness")?? == "READY",
            "Pad driver did not become ready"
        );

        let app = deployment(&flatpak, APP_ID, true, &cancel)?;
        let runtime_ref = app.runtime.as_deref().context("Missing runtime")?;
        let runtime = deployment(&flatpak, runtime_ref, false, &cancel)?;
        let runtime_files = current_path_for_host(&runtime.location)
            .canonicalize()?
            .join("files");
        let sdl_library = runtime_files
            .join("lib/x86_64-linux-gnu/libSDL2-2.0.so.0")
            .canonicalize()?;
        let executable = current_path_for_host(&app.location)
            .canonicalize()?
            .join("files/bin/snes9x-gtk")
            .canonicalize()?;
        let setup = SavedSetup {
            emulator_id: "snes9x".into(),
            content: rom.clone(),
            source_config,
            probe_program: probe,
            sdl_library,
            bubblewrap_program: "/run/current-system/sw/bin/bwrap".into(),
            executable_sha256: file_hash(&executable)?,
            players: (1..=2)
                .map(|player| super::super::settings::Player {
                    player,
                    controller_id: format!("snes9x-flatpak-oracle-p{player}"),
                })
                .collect(),
        };
        let calibration = oracle_calibration();
        let calibrations = HashMap::from([
            (setup.players[0].controller_id.clone(), calibration.clone()),
            (setup.players[1].controller_id.clone(), calibration),
        ]);
        let inventory: Vec<_> = pads
            .iter()
            .zip(&setup.players)
            .map(|((path, name), player)| ControllerDevice {
                stable_id: player.controller_id.clone(),
                name: name.clone(),
                device_path: path.into(),
                event_paths: vec![],
                vendor_id: Some("1209".into()),
                product_id: None,
                version: None,
                bus_type: Some("0006".into()),
                physical_path: None,
                unique_id: None,
                is_virtual: false,
            })
            .collect();
        let option = RomEmulatorOption::standalone(
            "snes9x".into(),
            "Snes9x".into(),
            EmulatorExecutable::Flatpak {
                command: flatpak.clone(),
                app_id: APP_ID.into(),
            },
        );
        let directory = rom
            .parent()
            .context("Oracle ROM has no parent")?
            .to_path_buf();
        let plan = LaunchPlan {
            emulator_name: "Snes9x".into(),
            program: flatpak.clone(),
            arguments: vec![
                "run".into(),
                format!("--filesystem={}", directory.display()).into(),
                APP_ID.into(),
                rom.as_os_str().to_owned(),
            ],
            current_directory: directory,
            environment: vec![],
            cleanup_paths: vec![],
            retroarch_content: None,
        };
        let mut session = super::super::native_command::prepare(
            &setup,
            &calibrations,
            &inventory,
            &option,
            &plan,
            &cancel,
        )?;
        let launch = session.plan.clone();
        let mut emulator = ChildGuard::new(session.spawn(&launch, &cancel)?);
        let pid = exact_flatpak_child(&flatpak, &rom)?;
        let title = "^production-oracle - Snes9x$";
        let deadline = Instant::now() + Duration::from_secs(20);
        let window_id = loop {
            let mut command = Command::new(&window_tool);
            if x11 {
                // A bare Xvfb has no EWMH window manager to associate
                // _NET_WM_PID with GTK's client windows. This ignored oracle
                // runs on a caller-owned isolated display, so bind input to
                // the one exact title rather than accepting a fuzzy window.
                command.args(["search", "--all", "--limit", "2", "--name", title]);
            } else {
                command.args([
                    "search",
                    "--all",
                    "--title",
                    title,
                    "--limit",
                    "2",
                    "getwindowid",
                    "%@",
                ]);
            }
            let window = command.output()?;
            ensure!(
                window.status.success()
                    || x11
                        && window.status.code() == Some(1)
                        && window.stdout.is_empty()
                        && window.stderr.is_empty(),
                "Could not inspect the oracle window"
            );
            let windows: Vec<_> = std::str::from_utf8(&window.stdout)?
                .lines()
                .filter(|line| !line.is_empty())
                .collect();
            ensure!(windows.len() <= 1, "Found multiple exact oracle windows");
            if let Some(window) = windows.first() {
                break (*window).to_owned();
            }
            ensure!(
                Instant::now() < deadline,
                "The exact oracle window did not become ready"
            );
            std::thread::sleep(Duration::from_millis(25));
        };
        let mut activate_command = Command::new(&window_tool);
        activate_command.arg(if x11 { "windowfocus" } else { "windowactivate" });
        if x11 {
            activate_command.arg("--sync");
        }
        let activate = activate_command.arg(&window_id).output()?;
        ensure!(
            activate.status.success(),
            "Could not activate the exact oracle window"
        );
        let exact_window_active = || -> Result<bool> {
            if x11 {
                let active = Command::new(&window_tool).arg("getwindowfocus").output()?;
                let name = Command::new(&window_tool)
                    .args(["getwindowname", &window_id])
                    .output()?;
                ensure!(
                    active.status.success() && name.status.success(),
                    "Could not inspect exact X11 window"
                );
                return Ok(std::str::from_utf8(&active.stdout)?.trim() == window_id
                    && std::str::from_utf8(&name.stdout)?.trim() == "production-oracle - Snes9x");
            }
            let active = Command::new(&window_tool)
                .args([
                    "getactivewindow",
                    "getwindowid",
                    "%@",
                    "getwindowname",
                    "%@",
                ])
                .output()?;
            ensure!(active.status.success(), "Could not inspect active window");
            let fields: Vec<_> = std::str::from_utf8(&active.stdout)?
                .lines()
                .filter(|line| !line.is_empty())
                .collect();
            Ok(fields == [window_id.as_str(), "production-oracle - Snes9x"])
        };
        let deadline = Instant::now() + Duration::from_secs(5);
        loop {
            if exact_window_active()? {
                break;
            }
            ensure!(
                Instant::now() < deadline,
                "The exact oracle window did not become active"
            );
            std::thread::sleep(Duration::from_millis(25));
        }

        let before_ticks = process_ticks(pid)?;
        ensure!(
            exact_window_active()?,
            "Exact oracle window lost focus before Emulation menu input"
        );
        let mut menu_command = Command::new(&input_tool);
        if x11 {
            menu_command.args(["key", "--window", &window_id, "--clearmodifiers", "alt+e"]);
        } else {
            menu_command
                .env("YDOTOOL_SOCKET", input_socket.as_ref().unwrap())
                .args(["key", "-d", "100", "56:1", "18:1", "18:0", "56:0"]);
        }
        let menu = menu_command.status()?;
        ensure!(menu.success(), "Could not open Snes9x Emulation menu");
        std::thread::sleep(Duration::from_millis(250));
        ensure!(
            exact_window_active()?,
            "Exact oracle window lost focus before Continue input"
        );
        let mut run_command = Command::new(&input_tool);
        if x11 {
            run_command.args(["key", "--window", &window_id, "--clearmodifiers", "c"]);
        } else {
            run_command
                .env("YDOTOOL_SOCKET", input_socket.as_ref().unwrap())
                .args(["key", "-d", "100", "46:1", "46:0"]);
        }
        let run = run_command.status()?;
        ensure!(run.success(), "Could not request Snes9x Continue");
        let deadline = Instant::now() + Duration::from_secs(5);
        loop {
            if process_ticks(pid)? >= before_ticks + 10 {
                break;
            }
            ensure!(
                Instant::now() < deadline,
                "Snes9x did not enter its emulation loop after Continue"
            );
            std::thread::sleep(Duration::from_millis(25));
        }
        driver
            .child
            .stdin
            .as_mut()
            .unwrap()
            .write_all(b"pulse-all\n")?;
        loop {
            if lines.next().context("Pad driver exited during pulse")?? == "PULSE-ALL-DONE" {
                break;
            }
        }
        std::thread::sleep(Duration::from_secs(2));
        // The supported GTK frontend installs SIGTERM as a graceful S9xExit
        // request, which saves SRAM before returning success.
        let signal_result = unsafe { libc::kill(pid as i32, libc::SIGTERM) };
        ensure!(
            signal_result == 0,
            "Could not request graceful oracle shutdown: {}",
            std::io::Error::last_os_error()
        );
        let deadline = Instant::now() + Duration::from_secs(20);
        let status = loop {
            if let Some(status) = emulator.child.try_wait()? {
                break status;
            }
            ensure!(Instant::now() < deadline, "Snes9x oracle did not exit");
            std::thread::sleep(Duration::from_millis(25));
        };
        ensure!(status.success(), "Snes9x oracle launch failed: {status}");
        driver.child.stdin.as_mut().unwrap().write_all(b"quit\n")?;
        ensure!(driver.child.wait()?.success(), "Pad driver failed");

        let sram = rom.with_extension("srm");
        let bytes = std::fs::read(&sram)?;
        ensure!(bytes.len() >= 0x78, "Snes9x oracle SRAM is truncated");
        ensure!(
            &bytes[..4] == b"LB\x01\x5a"
                && u16::from_le_bytes([bytes[4], bytes[5]]) > 100
                && u16::from_le_bytes([bytes[8], bytes[9]]) == 0
                && u16::from_le_bytes([bytes[10], bytes[11]]) == 0
                && u16::from_le_bytes([bytes[16], bytes[17]]) == 0xfff0
                && u16::from_le_bytes([bytes[18], bytes[19]]) == 0xfff0
                && u16::from_le_bytes([bytes[32], bytes[33]]) == 0xfff0
                && u16::from_le_bytes([bytes[34], bytes[35]]) == 0xfff0
                && u16::from_le_bytes([bytes[36], bytes[37]]) == 0
                && u16::from_le_bytes([bytes[38], bytes[39]]) == 0
                && u16::from_le_bytes([bytes[48], bytes[49]]) == 12
                && u16::from_le_bytes([bytes[50], bytes[51]]) == 12,
            "Snes9x hardware counters differ"
        );
        let expected = [
            0x0800_u16, 0x0400, 0x0200, 0x0100, 0x1000, 0x2000, 0x0080, 0x8000, 0x0040, 0x4000,
            0x0020, 0x0010,
        ];
        for (offset, expected) in [(0x40, expected), (0x60, expected)] {
            let observed: Vec<_> = bytes[offset..offset + 24]
                .chunks_exact(2)
                .map(|word| u16::from_le_bytes([word[0], word[1]]))
                .collect();
            ensure!(observed == expected, "Snes9x hardware input log differs");
        }
        ensure!(
            state_bytes(&profile)? == before,
            "Snes9x user state changed"
        );
        Ok(())
    }
}
