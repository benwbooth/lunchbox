//! Exact target-runtime bridge for the audited Nestopia UE 1.53.2 Flatpak.

use super::{isolation::PreparedConfig, settings::SavedSetup};
use crate::{
    controller_native_process::{cancelled, capture},
    emulator::{LaunchPlan, RomEmulatorOption},
    platform_process::{host_command, is_flatpak},
};
use anyhow::{Context, Result, ensure};
use lunchbox_controller_probe::{
    file_hash,
    nestopia_supervisor::{Receipt, Request},
    sdl2::Snapshot,
};
use std::{
    ffi::OsString,
    fs::OpenOptions,
    io::{Read, Write},
    os::unix::fs::{MetadataExt, OpenOptionsExt, PermissionsExt},
    path::{Path, PathBuf},
    sync::atomic::AtomicBool,
};

pub(crate) const APP_ID: &str = "ca._0ldsk00l.Nestopia";
pub(crate) const APP_COMMIT: &str =
    "0f3d30d71419cee53f903b77944821d919fcfbe98aa3648bf8d12765bddd6169";
pub(crate) const APP_EXECUTABLE_SHA256: &str =
    "1b63638f9e19e007900ac7c81451dbaa734661f79e3033d7013dbec2509543ea";
pub(crate) const RUNTIME_REF: &str = "org.freedesktop.Platform/x86_64/25.08";
pub(crate) const RUNTIME_COMMIT: &str =
    "bd44a6230581917d04f89812a4c21090c304d390edb73995af1c2f9fd8abf4e8";
pub(crate) const SDL_RELATIVE: &str = "lib/x86_64-linux-gnu/libSDL2-2.0.so.0.3200.70";
pub(crate) const SDL_SHA256: &str =
    "6a2edbdfb43cea4c8e8d44033f05a626336123e426793f34a43d8d9576f9068d";
const LOADER_RELATIVE: &str = "lib/x86_64-linux-gnu/ld-linux-x86-64.so.2";
const LOADER_SHA256: &str = "a3b79fc634bbfdc51b5f1c6dc0013b14a7ceb98068efbd1f0055d9ca96d27cf6";
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
    app: Deployment,
    runtime: Deployment,
    executable: PathBuf,
    sdl_library: PathBuf,
    loader: PathBuf,
    probe_source: PathBuf,
    probe_sha256: String,
    staged_probe: PathBuf,
    data_home: PathBuf,
    _probe_directory: tempfile::TempDir,
    supervisor: Option<Supervisor>,
}

fn text_output(command: &Path, target: &str, flag: &str, cancel: &AtomicBool) -> Result<String> {
    let mut process = host_command(command);
    process.arg("info").arg(flag).arg(target);
    let (output, _) = capture(&mut process, cancel)?;
    ensure!(
        output.len() <= 4096,
        "Nestopia Flatpak deployment field is oversized"
    );
    let text = std::str::from_utf8(&output)?.trim();
    ensure!(
        !text.is_empty() && !text.contains(['\0', '\r', '\n']),
        "Nestopia Flatpak deployment field is malformed"
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
        "Nestopia Flatpak deployment commit is malformed"
    );
    let location = PathBuf::from(text_output(command, target, "--show-location", cancel)?);
    ensure!(
        location.is_absolute(),
        "Nestopia Flatpak deployment location is not absolute"
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

fn parse_original<'a>(setup: &SavedSetup, original: &'a LaunchPlan) -> Result<&'a Path> {
    ensure!(
        original.environment.is_empty()
            && original.retroarch_content.is_none()
            && original.arguments.len() == 4
            && original.arguments[0] == "run"
            && original.arguments[2] == APP_ID,
        "Nestopia Flatpak launch requires the ordinary one-ROM argument plan"
    );
    let grant = original.arguments[1]
        .to_str()
        .and_then(|value| value.strip_prefix("--filesystem="))
        .context("Nestopia Flatpak launch is missing its ROM-directory grant")?;
    ensure!(
        !grant.contains(':')
            && Path::new(grant).canonicalize()? == original.current_directory.canonicalize()?,
        "Nestopia Flatpak launch grants a directory other than its exact launch directory"
    );
    let content = Path::new(&original.arguments[3]);
    ensure!(
        content.canonicalize()? == setup.content.canonicalize()?
            && content.parent().is_some_and(|parent| {
                parent.canonicalize().ok().as_ref()
                    == original.current_directory.canonicalize().ok().as_ref()
            }),
        "Nestopia Flatpak launch does not select the saved ROM exactly once"
    );
    Ok(content)
}

fn controller_cache() -> Result<PathBuf> {
    let home = directories::BaseDirs::new()
        .context("Finding the user home for Nestopia controller staging")?
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
        setup.validate()?;
        ensure!(
            app_id == APP_ID && setup.emulator_id == option.emulator_id,
            "Nestopia adapter supports only the audited Flathub application"
        );
        ensure!(
            same_program(command, &original.program),
            "Nestopia Flatpak command differs from selection"
        );
        parse_original(setup, original)?;

        let profile = directories::BaseDirs::new()
            .context("Finding the Nestopia Flatpak profile")?
            .home_dir()
            .join(".var/app")
            .join(APP_ID);
        let expected_config = profile.join("config/nestopia");
        ensure!(
            expected_config.join("nestopia.conf").canonicalize()?
                == setup.source_main_config.canonicalize()?
                && expected_config.join("input.conf").canonicalize()?
                    == setup.source_input_config.canonicalize()?,
            "Nestopia saved configs differ from the target Flatpak profile"
        );
        let data_home = logical_host_path(&profile.join("data").canonicalize()?);

        let app = deployment(command, APP_ID, true, cancel)?;
        ensure!(
            app.commit == APP_COMMIT && app.runtime.as_deref() == Some(RUNTIME_REF),
            "Nestopia Flatpak application is outside the audited deployment"
        );
        let runtime = deployment(command, RUNTIME_REF, false, cancel)?;
        ensure!(
            runtime.commit == RUNTIME_COMMIT,
            "Nestopia Flatpak runtime is outside the audited deployment"
        );

        let app_files = current_path_for_host(&app.location)
            .canonicalize()?
            .join("files");
        let executable = app_files.join("bin/nestopia").canonicalize()?;
        ensure!(
            file_hash(&executable)?.eq_ignore_ascii_case(APP_EXECUTABLE_SHA256)
                && setup
                    .executable_sha256
                    .eq_ignore_ascii_case(APP_EXECUTABLE_SHA256),
            "Nestopia executable differs from the audited runtime"
        );
        let runtime_files = current_path_for_host(&runtime.location)
            .canonicalize()?
            .join("files");
        let sdl_library = runtime_files.join(SDL_RELATIVE).canonicalize()?;
        let loader = runtime_files.join(LOADER_RELATIVE).canonicalize()?;
        ensure!(
            setup.sdl_library.canonicalize()? == sdl_library
                && file_hash(&sdl_library)?.eq_ignore_ascii_case(SDL_SHA256)
                && file_hash(&loader)?.eq_ignore_ascii_case(LOADER_SHA256),
            "Nestopia SDL2 or loader differs from the audited target runtime"
        );

        let probe_source = setup.probe_program.canonicalize()?;
        ensure!(
            std::fs::metadata(&probe_source)?.is_file(),
            "Nestopia controller probe is not a regular file"
        );
        let probe_sha256 = file_hash(&probe_source)?;
        let probe_directory = tempfile::Builder::new()
            .prefix("nestopia-flatpak-probe-")
            .tempdir_in(controller_cache()?)?;
        std::fs::set_permissions(
            probe_directory.path(),
            std::fs::Permissions::from_mode(0o700),
        )?;
        let staged_probe = probe_directory.path().join("lunchbox-controller-probe");
        std::fs::copy(&probe_source, &staged_probe)?;
        std::fs::set_permissions(&staged_probe, std::fs::Permissions::from_mode(0o500))?;
        ensure!(
            file_hash(&staged_probe)? == probe_sha256,
            "Staged Nestopia probe differs from its source"
        );

        let prepared = Self {
            command: command.to_path_buf(),
            app,
            runtime,
            executable,
            sdl_library,
            loader,
            probe_source,
            probe_sha256,
            staged_probe,
            data_home,
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
            .context("Staged Nestopia probe has no directory")?;
        let mut command = host_command(&self.command);
        command
            .arg("run")
            .arg("--die-with-parent")
            .arg("--nofilesystem=host:reset")
            .arg("--nofilesystem=home")
            .arg("--nosocket=wayland")
            .arg("--socket=x11")
            .arg(format!("--filesystem={}:ro", probe_root.display()))
            .arg("--env=FLTK_BACKEND=x11")
            .arg("--env=SDL_LINUX_JOYSTICK_CLASSIC=1")
            .arg("--env=SDL_JOYSTICK_LINUX_CLASSIC=1")
            .arg("--command=/usr/lib/x86_64-linux-gnu/ld-linux-x86-64.so.2")
            .arg(APP_ID)
            .arg(&self.staged_probe)
            .arg("--sdl2-inventory")
            .arg("--sdl-library")
            .arg(format!("/usr/{SDL_RELATIVE}"));
        if let Some(path) = path {
            command.arg("--sdl2-controls-for-path").arg(path);
        }
        let (output, _) = capture(&mut command, cancel)?;
        let snapshot: Snapshot =
            serde_json::from_slice(&output).context("Invalid target Nestopia SDL capture")?;
        ensure!(
            snapshot.version[0] == 2
                && snapshot.library == Path::new(&format!("/usr/{SDL_RELATIVE}"))
                && snapshot.library_sha256.eq_ignore_ascii_case(SDL_SHA256)
                && snapshot.devices.len() <= 12
                && snapshot
                    .devices
                    .iter()
                    .enumerate()
                    .all(|(index, device)| device.device_index as usize == index),
            "Nestopia helper did not capture a complete audited SDL2 routing"
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
            "Nestopia Flatpak launch was already staged"
        );
        let content = parse_original(setup, original)?;
        configuration.verify()?;
        let request_path = configuration.root().join("supervisor.json");
        let receipt_path = configuration.root().join("ready.json");
        ensure!(
            !request_path.try_exists()? && !receipt_path.try_exists()?,
            "Nestopia supervisor staging already exists"
        );
        let request = Request {
            schema_version: 1,
            executable: "/app/bin/nestopia".into(),
            executable_sha256: APP_EXECUTABLE_SHA256.into(),
            sdl_library: format!("/usr/{SDL_RELATIVE}").into(),
            sdl_library_sha256: SDL_SHA256.into(),
            private_root: configuration.root().to_path_buf(),
            main_config_sha256: file_hash(&configuration.main_path())?,
            input_config_sha256: file_hash(&configuration.input_path())?,
            data_home: self.data_home.clone(),
            content: content.to_path_buf(),
            current_directory: content
                .parent()
                .context("Nestopia ROM has no parent")?
                .to_path_buf(),
            runtime_paths: runtime_paths.to_vec(),
            inventory: inventory.clone(),
            receipt: receipt_path.clone(),
        };
        let request_bytes = serde_json::to_vec(&request)?;
        ensure!(
            request_bytes.len() as u64 <= OUTPUT_LIMIT,
            "Nestopia supervisor request is oversized"
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
            .context("Staged Nestopia probe has no directory")?;
        let mut arguments = vec![
            original.arguments[0].clone(),
            "--die-with-parent".into(),
            "--nofilesystem=host:reset".into(),
            "--nofilesystem=home".into(),
            "--nosocket=wayland".into(),
            "--socket=x11".into(),
            original.arguments[1].clone(),
        ];
        arguments.extend([
            format!("--filesystem={}:ro", probe_root.display()).into(),
            format!("--filesystem={}", configuration.root().display()).into(),
            "--env=FLTK_BACKEND=x11".into(),
            "--env=SDL_LINUX_JOYSTICK_CLASSIC=1".into(),
            "--env=SDL_JOYSTICK_LINUX_CLASSIC=1".into(),
            "--command=/usr/lib/x86_64-linux-gnu/ld-linux-x86-64.so.2".into(),
            APP_ID.into(),
            self.staged_probe.as_os_str().to_owned(),
            "--nestopia-supervise".into(),
            request_path.into_os_string(),
        ]);
        Ok(arguments)
    }

    pub(crate) fn receipt_ready(&self) -> Result<bool> {
        let supervisor = self
            .supervisor
            .as_ref()
            .context("Nestopia supervisor was not staged")?;
        if !supervisor.receipt_path.try_exists()? {
            return Ok(false);
        }
        let file = std::fs::File::open(&supervisor.receipt_path)?;
        ensure!(
            file.metadata()?.is_file() && file.metadata()?.mode() & 0o7777 == 0o600,
            "Nestopia readiness receipt is not a private regular file"
        );
        let mut bytes = Vec::new();
        file.take(OUTPUT_LIMIT + 1).read_to_end(&mut bytes)?;
        ensure!(
            bytes.len() as u64 <= OUTPUT_LIMIT,
            "Nestopia readiness receipt is oversized"
        );
        let receipt: Receipt =
            serde_json::from_slice(&bytes).context("Invalid Nestopia readiness receipt")?;
        ensure!(
            receipt == supervisor.expected_receipt,
            "Nestopia readiness receipt differs from prepared routing"
        );
        Ok(true)
    }

    pub(crate) fn verify(&self, cancel: &AtomicBool) -> Result<()> {
        cancelled(cancel)?;
        ensure!(
            deployment(&self.command, APP_ID, true, cancel)? == self.app
                && self.app.commit == APP_COMMIT
                && self.app.runtime.as_deref() == Some(RUNTIME_REF),
            "Nestopia Flatpak application deployment changed"
        );
        ensure!(
            deployment(&self.command, RUNTIME_REF, false, cancel)? == self.runtime
                && self.runtime.commit == RUNTIME_COMMIT,
            "Nestopia Flatpak runtime deployment changed"
        );
        ensure!(
            file_hash(&self.executable)?.eq_ignore_ascii_case(APP_EXECUTABLE_SHA256)
                && file_hash(&self.sdl_library)?.eq_ignore_ascii_case(SDL_SHA256)
                && file_hash(&self.loader)?.eq_ignore_ascii_case(LOADER_SHA256)
                && file_hash(&self.probe_source)? == self.probe_sha256
                && file_hash(&self.staged_probe)? == self.probe_sha256,
            "Nestopia target runtime or staged probe changed"
        );
        let metadata = std::fs::symlink_metadata(self.staged_probe.parent().unwrap())?;
        ensure!(
            metadata.is_dir() && metadata.mode() & 0o7777 == 0o700,
            "Nestopia probe directory is not private"
        );
        let metadata = std::fs::symlink_metadata(&self.staged_probe)?;
        ensure!(
            metadata.is_file() && metadata.mode() & 0o7777 == 0o500,
            "Nestopia staged probe mode changed"
        );
        if let Some(supervisor) = &self.supervisor {
            let metadata = std::fs::symlink_metadata(&supervisor.request_path)?;
            ensure!(
                metadata.is_file()
                    && metadata.mode() & 0o7777 == 0o600
                    && std::fs::read(&supervisor.request_path)? == supervisor.request_bytes,
                "Nestopia supervisor request or mode changed"
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

    #[test]
    fn audited_runtime_identity_is_exact() {
        assert_eq!(APP_ID, "ca._0ldsk00l.Nestopia");
        assert_eq!(APP_COMMIT.len(), 64);
        assert_eq!(RUNTIME_REF, "org.freedesktop.Platform/x86_64/25.08");
        assert_eq!(RUNTIME_COMMIT.len(), 64);
        assert_eq!(SDL_SHA256.len(), 64);
        assert_eq!(APP_EXECUTABLE_SHA256.len(), 64);
        assert!(!SDL_RELATIVE.contains(".."));
    }
}
