//! Exact target-runtime bridge for the audited puNES 0.111 Flatpak.

use super::{isolation::PreparedConfig, settings::SavedSetup};
use crate::{
    controller_native_process::{cancelled, capture},
    emulator::{LaunchPlan, RomEmulatorOption},
    platform_process::{host_command, is_flatpak},
};
use anyhow::{Context, Result, ensure};
use lunchbox_controller_probe::{
    file_hash,
    punes_supervisor::{ConfigFile, Inventory, Receipt, Request},
};
use std::{
    ffi::OsString,
    fs::OpenOptions,
    io::{Read, Write},
    os::unix::fs::{MetadataExt, OpenOptionsExt, PermissionsExt},
    path::{Path, PathBuf},
    sync::atomic::AtomicBool,
};

pub(crate) const APP_ID: &str = "io.github.punesemu.puNES";
pub(crate) const APP_COMMIT: &str =
    "335dff06f700b1d21f600fa8b77e34080f577536a40df9d8c2b319be5d4c86e9";
pub(crate) const APP_EXECUTABLE_SHA256: &str =
    "ae027e3b7bc5396407f82db665c6dd9507dc65e2e41c7f4d9830165f14a15c45";
pub(crate) const RUNTIME_REF: &str = "org.kde.Platform/x86_64/5.15-25.08";
pub(crate) const RUNTIME_COMMIT: &str =
    "1e9b0aa4623015cebd20350b36dc515124567b0f43c2235bd4ea8259fc2b18de";
pub(crate) const LIBUDEV_RELATIVE: &str = "lib/x86_64-linux-gnu/libudev.so.1.7.10";
pub(crate) const LIBUDEV_SHA256: &str =
    "ebee10d92fdc8bfb3d7b8df59771dbc8bed79552c3a0c4cf7d30989ac0d01e4d";
const LOADER_RELATIVE: &str = "lib/x86_64-linux-gnu/ld-linux-x86-64.so.2";
const LOADER_SHA256: &str = "d204c9ce43348a62ec1cefad8e7a04eec7d365deb9e9dc3d881a236f2994dba1";
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
    libudev: PathBuf,
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
        "puNES Flatpak deployment field is oversized"
    );
    let text = std::str::from_utf8(&output)?.trim();
    ensure!(
        !text.is_empty() && !text.contains(['\0', '\r', '\n']),
        "puNES Flatpak deployment field is malformed"
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
        "puNES Flatpak deployment commit is malformed"
    );
    let location = PathBuf::from(text_output(command, target, "--show-location", cancel)?);
    ensure!(
        location.is_absolute(),
        "puNES Flatpak deployment location is not absolute"
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
        "puNES Flatpak launch requires the ordinary one-ROM argument plan"
    );
    let grant = original.arguments[1]
        .to_str()
        .and_then(|value| value.strip_prefix("--filesystem="))
        .context("puNES Flatpak launch is missing its ROM-directory grant")?;
    let grant = grant.strip_suffix(":ro").unwrap_or(grant);
    ensure!(
        !grant.contains(':')
            && Path::new(grant).canonicalize()? == original.current_directory.canonicalize()?,
        "puNES Flatpak launch grants a directory other than its exact launch directory"
    );
    let content = Path::new(&original.arguments[3]);
    ensure!(
        content.canonicalize()? == setup.content.canonicalize()?
            && content.parent().is_some_and(|parent| {
                parent.canonicalize().ok().as_ref()
                    == original.current_directory.canonicalize().ok().as_ref()
            }),
        "puNES Flatpak launch does not select the saved ROM exactly once"
    );
    Ok(content)
}

fn controller_cache() -> Result<PathBuf> {
    let home = directories::BaseDirs::new()
        .context("Finding the user home for puNES controller staging")?
        .home_dir()
        .to_path_buf();
    let cache = home.join(".cache/lunchbox/controller-launch");
    std::fs::create_dir_all(&cache)?;
    Ok(cache)
}

fn valid_local_x11_display(display: &str) -> bool {
    let local = display.strip_prefix(':').unwrap_or_default();
    let mut fields = local.split('.');
    let server = fields.next().unwrap_or_default();
    let screen = fields.next();
    display.starts_with(':')
        && !server.is_empty()
        && server.bytes().all(|byte| byte.is_ascii_digit())
        && screen.is_none_or(|screen| {
            !screen.is_empty() && screen.bytes().all(|byte| byte.is_ascii_digit())
        })
        && fields.next().is_none()
        && display.len() <= 32
}

fn local_x11_display() -> Result<String> {
    let display = std::env::var("DISPLAY").context("puNES exact-X11 launch needs DISPLAY")?;
    ensure!(
        valid_local_x11_display(&display),
        "puNES local X11 display is malformed"
    );
    Ok(display)
}

fn supervised_sandbox_arguments(display: &str) -> Vec<OsString> {
    vec![
        "--nofilesystem=host:reset".into(),
        "--nofilesystem=home".into(),
        "--filesystem=/run/udev:ro".into(),
        "--nosocket=wayland".into(),
        "--socket=x11".into(),
        "--env=QT_QPA_PLATFORM=xcb".into(),
        format!("--env=DISPLAY={display}").into(),
        "--unset-env=WAYLAND_DISPLAY".into(),
    ]
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
            "puNES adapter supports only the audited Flathub application"
        );
        ensure!(
            same_program(command, &original.program),
            "puNES Flatpak command differs from selection"
        );
        parse_original(setup, original)?;

        let profile = directories::BaseDirs::new()
            .context("Finding the puNES Flatpak profile")?
            .home_dir()
            .join(".var/app")
            .join(APP_ID);
        let expected_config = profile.join("config/puNES");
        ensure!(
            expected_config.join("puNES.cfg").canonicalize()?
                == setup.source_main_config.canonicalize()?
                && expected_config.join("input.cfg").canonicalize()?
                    == setup.source_input_config.canonicalize()?,
            "puNES saved configs differ from the target Flatpak profile"
        );
        let data_home = logical_host_path(&profile.join("data").canonicalize()?);

        let app = deployment(command, APP_ID, true, cancel)?;
        ensure!(
            app.commit == APP_COMMIT && app.runtime.as_deref() == Some(RUNTIME_REF),
            "puNES Flatpak application is outside the audited deployment"
        );
        let runtime = deployment(command, RUNTIME_REF, false, cancel)?;
        ensure!(
            runtime.commit == RUNTIME_COMMIT,
            "puNES Flatpak runtime is outside the audited deployment"
        );
        let app_files = current_path_for_host(&app.location)
            .canonicalize()?
            .join("files");
        let executable = app_files.join("bin/punes").canonicalize()?;
        ensure!(
            file_hash(&executable)?.eq_ignore_ascii_case(APP_EXECUTABLE_SHA256)
                && setup
                    .executable_sha256
                    .eq_ignore_ascii_case(APP_EXECUTABLE_SHA256),
            "puNES executable differs from the audited runtime"
        );
        let runtime_files = current_path_for_host(&runtime.location)
            .canonicalize()?
            .join("files");
        let libudev = runtime_files.join(LIBUDEV_RELATIVE).canonicalize()?;
        let loader = runtime_files.join(LOADER_RELATIVE).canonicalize()?;
        ensure!(
            file_hash(&libudev)?.eq_ignore_ascii_case(LIBUDEV_SHA256)
                && file_hash(&loader)?.eq_ignore_ascii_case(LOADER_SHA256),
            "puNES runtime loader or libudev differs from the audited runtime"
        );

        let probe_source = setup.probe_program.canonicalize()?;
        ensure!(
            std::fs::metadata(&probe_source)?.is_file(),
            "puNES controller probe is not a regular file"
        );
        let probe_sha256 = file_hash(&probe_source)?;
        let probe_directory = tempfile::Builder::new()
            .prefix("punes-flatpak-probe-")
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
            "Staged puNES probe differs from its source"
        );

        let prepared = Self {
            command: command.to_path_buf(),
            app,
            runtime,
            executable,
            libudev,
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

    pub(crate) fn observe(&self, cancel: &AtomicBool) -> Result<Inventory> {
        self.verify(cancel)?;
        let probe_root = self
            .staged_probe
            .parent()
            .context("Staged puNES probe has no directory")?;
        let mut command = host_command(&self.command);
        command
            .arg("run")
            .arg("--die-with-parent")
            .arg("--nofilesystem=host:reset")
            .arg("--nofilesystem=home")
            .arg(format!("--filesystem={}:ro", probe_root.display()))
            .arg("--command=/usr/lib/x86_64-linux-gnu/ld-linux-x86-64.so.2")
            .arg(APP_ID)
            .arg(&self.staged_probe)
            .arg("--punes-target-inventory");
        let (output, _) = capture(&mut command, cancel)?;
        ensure!(
            output.len() as u64 <= OUTPUT_LIMIT,
            "puNES target inventory is oversized"
        );
        let inventory: Inventory =
            serde_json::from_slice(&output).context("Invalid target puNES evdev inventory")?;
        inventory.validate(inventory.devices.len())?;
        self.verify(cancel)?;
        Ok(inventory)
    }

    pub(crate) fn prepare_launch(
        &mut self,
        setup: &SavedSetup,
        configuration: &PreparedConfig,
        inventory: &Inventory,
        original: &LaunchPlan,
    ) -> Result<Vec<OsString>> {
        ensure!(
            self.supervisor.is_none(),
            "puNES Flatpak launch was already staged"
        );
        let content = parse_original(setup, original)?;
        configuration.verify()?;
        inventory.validate(setup.players.len())?;
        let request_path = configuration.root().join("supervisor.json");
        let receipt_path = configuration.root().join("ready.json");
        ensure!(
            !request_path.try_exists()? && !receipt_path.try_exists()?,
            "puNES supervisor staging already exists"
        );
        let mut config_paths = vec![configuration.main_path(), configuration.input_path()];
        config_paths.extend(configuration.jsc_paths()?);
        let configs = config_paths
            .into_iter()
            .map(|path| {
                Ok(ConfigFile {
                    sha256: file_hash(&path)?,
                    path,
                })
            })
            .collect::<Result<Vec<_>>>()?;
        let request = Request {
            schema_version: 1,
            executable: "/app/bin/punes".into(),
            executable_sha256: APP_EXECUTABLE_SHA256.into(),
            libudev: format!("/usr/{LIBUDEV_RELATIVE}").into(),
            libudev_sha256: LIBUDEV_SHA256.into(),
            private_root: configuration.root().to_path_buf(),
            configs,
            data_home: self.data_home.clone(),
            content: content.to_path_buf(),
            current_directory: content
                .parent()
                .context("puNES ROM has no parent")?
                .to_path_buf(),
            inventory: inventory.clone(),
            receipt: receipt_path.clone(),
        };
        let request_bytes = serde_json::to_vec(&request)?;
        ensure!(
            request_bytes.len() as u64 <= OUTPUT_LIMIT,
            "puNES supervisor request is oversized"
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
            .context("Staged puNES probe has no directory")?;
        let content_root = content.parent().context("puNES ROM has no parent")?;
        let display = local_x11_display()?;
        let mut arguments = vec![original.arguments[0].clone(), "--die-with-parent".into()];
        arguments.extend(supervised_sandbox_arguments(&display));
        arguments.push(format!("--filesystem={}:ro", content_root.display()).into());
        arguments.extend([
            format!("--filesystem={}:ro", probe_root.display()).into(),
            format!("--filesystem={}", configuration.root().display()).into(),
            "--command=/usr/lib/x86_64-linux-gnu/ld-linux-x86-64.so.2".into(),
            APP_ID.into(),
            self.staged_probe.as_os_str().to_owned(),
            "--punes-supervise".into(),
            request_path.into_os_string(),
        ]);
        Ok(arguments)
    }

    pub(crate) fn receipt_ready(&self) -> Result<bool> {
        let supervisor = self
            .supervisor
            .as_ref()
            .context("puNES supervisor was not staged")?;
        if !supervisor.receipt_path.try_exists()? {
            return Ok(false);
        }
        let file = std::fs::File::open(&supervisor.receipt_path)?;
        ensure!(
            file.metadata()?.is_file() && file.metadata()?.mode() & 0o7777 == 0o600,
            "puNES readiness receipt is not a private regular file"
        );
        let mut bytes = Vec::new();
        file.take(OUTPUT_LIMIT + 1).read_to_end(&mut bytes)?;
        ensure!(
            bytes.len() as u64 <= OUTPUT_LIMIT,
            "puNES readiness receipt is oversized"
        );
        let receipt: Receipt =
            serde_json::from_slice(&bytes).context("Invalid puNES readiness receipt")?;
        ensure!(
            receipt == supervisor.expected_receipt,
            "puNES readiness receipt differs from prepared routing"
        );
        Ok(true)
    }

    pub(crate) fn verify(&self, cancel: &AtomicBool) -> Result<()> {
        cancelled(cancel)?;
        ensure!(
            deployment(&self.command, APP_ID, true, cancel)? == self.app
                && self.app.commit == APP_COMMIT
                && self.app.runtime.as_deref() == Some(RUNTIME_REF),
            "puNES Flatpak application deployment changed"
        );
        ensure!(
            deployment(&self.command, RUNTIME_REF, false, cancel)? == self.runtime
                && self.runtime.commit == RUNTIME_COMMIT,
            "puNES Flatpak runtime deployment changed"
        );
        ensure!(
            file_hash(&self.executable)?.eq_ignore_ascii_case(APP_EXECUTABLE_SHA256)
                && file_hash(&self.libudev)?.eq_ignore_ascii_case(LIBUDEV_SHA256)
                && file_hash(&self.loader)?.eq_ignore_ascii_case(LOADER_SHA256)
                && file_hash(&self.probe_source)? == self.probe_sha256
                && file_hash(&self.staged_probe)? == self.probe_sha256,
            "puNES target runtime or staged probe changed"
        );
        let metadata = std::fs::symlink_metadata(self.staged_probe.parent().unwrap())?;
        ensure!(
            metadata.is_dir() && metadata.mode() & 0o7777 == 0o700,
            "puNES probe directory is not private"
        );
        let metadata = std::fs::symlink_metadata(&self.staged_probe)?;
        ensure!(
            metadata.is_file() && metadata.mode() & 0o7777 == 0o500,
            "puNES staged probe mode changed"
        );
        if let Some(supervisor) = &self.supervisor {
            let metadata = std::fs::symlink_metadata(&supervisor.request_path)?;
            ensure!(
                metadata.is_file()
                    && metadata.mode() & 0o7777 == 0o600
                    && std::fs::read(&supervisor.request_path)? == supervisor.request_bytes,
                "puNES supervisor request or mode changed"
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
        assert_eq!(APP_ID, "io.github.punesemu.puNES");
        assert_eq!(APP_COMMIT.len(), 64);
        assert_eq!(RUNTIME_REF, "org.kde.Platform/x86_64/5.15-25.08");
        assert_eq!(RUNTIME_COMMIT.len(), 64);
        assert_eq!(APP_EXECUTABLE_SHA256.len(), 64);
        assert_eq!(LIBUDEV_SHA256.len(), 64);
        assert!(!LIBUDEV_RELATIVE.contains(".."));
    }

    #[test]
    fn local_x11_display_rejects_remote_or_malformed_values() {
        assert!(valid_local_x11_display(":0"));
        assert!(valid_local_x11_display(":152.0"));
        assert!(!valid_local_x11_display("host:0"));
        assert!(!valid_local_x11_display(":"));
        assert!(!valid_local_x11_display(":1."));
        assert!(!valid_local_x11_display(":1.0.0"));
    }

    #[test]
    fn supervised_launch_restores_udev_once_after_filesystem_reset() {
        let arguments = supervised_sandbox_arguments(":152.0");
        assert_eq!(
            arguments,
            [
                "--nofilesystem=host:reset",
                "--nofilesystem=home",
                "--filesystem=/run/udev:ro",
                "--nosocket=wayland",
                "--socket=x11",
                "--env=QT_QPA_PLATFORM=xcb",
                "--env=DISPLAY=:152.0",
                "--unset-env=WAYLAND_DISPLAY",
            ]
            .map(OsString::from)
        );
        assert_eq!(
            arguments
                .iter()
                .filter(|argument| *argument == "--filesystem=/run/udev:ro")
                .count(),
            1
        );
    }
}
