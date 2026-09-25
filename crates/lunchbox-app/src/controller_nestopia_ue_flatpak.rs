//! Production controller launch path for the exact audited Nestopia UE Flatpak.
//!
//! Source contract: `0ldsk00l/nestopia` 1.53.2 at
//! `4470a2e99199d8010322eef4bf680fb3760f6eda`. The target application,
//! Freedesktop runtime, SDL2 library and loader are pinned separately in
//! `flatpak.rs`; package metadata alone is not accepted as runtime identity.

pub(crate) mod configuration;
mod flatpak;
pub(crate) mod guided;
mod isolation;
mod physical;
mod session;
pub(crate) mod settings;

use crate::{
    controller_catalog::Calibration,
    controller_native_process::cancelled,
    controllers::ControllerDevice,
    emulator::{EmulatorExecutable, LaunchPlan, RomEmulatorOption},
};
use anyhow::{Result, ensure};
use lunchbox_controller_probe::file_hash;
use std::{
    collections::{BTreeMap, HashMap},
    path::PathBuf,
    sync::atomic::AtomicBool,
    time::{Duration, Instant},
};

pub(crate) const SOURCE_COMMIT: &str = "4470a2e99199d8010322eef4bf680fb3760f6eda";

pub(crate) struct PreparedLaunch {
    pub(crate) inputs: session::PreparedSession,
    pub(crate) executable: PathBuf,
    pub(crate) setup: settings::SavedSetup,
    pub(crate) plan: LaunchPlan,
    files: BTreeMap<PathBuf, String>,
}

impl PreparedLaunch {
    pub(crate) fn check_health(&self) -> Result<()> {
        self.inputs.check_health()
    }

    pub(crate) fn verify(&self, cancel: &AtomicBool) -> Result<()> {
        cancelled(cancel)?;
        for (path, expected) in &self.files {
            ensure!(
                file_hash(path)? == *expected,
                "Nestopia launch input changed"
            );
        }
        self.inputs.verify(cancel)
    }

    pub(crate) fn spawn(
        &mut self,
        plan: &LaunchPlan,
        cancel: &AtomicBool,
    ) -> Result<std::process::Child> {
        ensure!(
            plan == &self.plan,
            "Nestopia launch plan changed after preparation"
        );
        self.verify(cancel)?;
        let mut child = crate::emulator::spawn_launch_plan(plan)?;
        let result = (|| {
            let deadline = Instant::now() + Duration::from_secs(20);
            loop {
                cancelled(cancel)?;
                ensure!(
                    child.try_wait()?.is_none(),
                    "Nestopia Flatpak supervisor exited before controller handoff"
                );
                if self.inputs.receipt_ready()? {
                    self.verify(cancel)?;
                    return Ok(());
                }
                ensure!(
                    Instant::now() < deadline,
                    "Nestopia Flatpak supervisor did not prove controller routing before timeout"
                );
                std::thread::sleep(Duration::from_millis(25));
            }
        })();
        if let Err(error) = result {
            let _ = child.kill();
            let _ = child.wait();
            return Err(error);
        }
        Ok(child)
    }
}

pub(crate) fn prepare(
    setup: &settings::SavedSetup,
    calibrations: &HashMap<String, Calibration>,
    inventory: &[ControllerDevice],
    option: &RomEmulatorOption,
    original: &LaunchPlan,
    cancel: &AtomicBool,
) -> Result<PreparedLaunch> {
    cancelled(cancel)?;
    setup.validate()?;
    ensure!(
        setup.emulator_id == option.emulator_id && original.environment.is_empty(),
        "Nestopia identity differs or custom environment needs resolution"
    );
    let EmulatorExecutable::Flatpak { command, app_id } = &option.executable else {
        anyhow::bail!("Nestopia production adapter supports only its audited Flatpak");
    };
    let runtime =
        flatpak::PreparedFlatpak::prepare(setup, option, original, command, app_id, cancel)?;
    let executable = runtime.executable().to_path_buf();
    let mut files = BTreeMap::new();
    for path in [
        &setup.content,
        &setup.probe_program,
        &setup.sdl_library,
        &original.program,
        &executable,
    ] {
        files.insert(path.clone(), file_hash(path)?);
    }
    let mut inputs =
        session::PreparedSession::prepare(setup, calibrations, inventory, runtime, cancel)?;
    let mut plan = original.clone();
    plan.arguments = inputs.stage_launch(original)?;
    let prepared = PreparedLaunch {
        inputs,
        executable,
        setup: setup.clone(),
        plan,
        files,
    };
    prepared.verify(cancel)?;
    Ok(prepared)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        controller_catalog::{Calibration, InputBinding, NativeInput, catalog},
        controllers::ControllerDevice,
        emulator::EmulatorExecutable,
        platform_locations::{LocationBases, load_records, save_route_roots_for_platform},
        save_cloud::{CloudProfile, CloudStore},
        save_sync::{ArtifactKey, SavePurpose, SaveRoute, SyncAction, SyncActionKind, SyncScope},
        save_sync_service::{AppliedSync, prepare_sync},
    };
    use anyhow::Context;
    use serde::Serialize;
    use std::{
        collections::BTreeMap,
        fs::{FileTimes, OpenOptions},
        io::{BufRead, BufReader, Write},
        os::unix::fs::{FileTypeExt, OpenOptionsExt},
        path::Path,
        process::{Child, Command, Stdio},
        time::SystemTime,
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

    struct OracleOutputs {
        save: PathBuf,
        state: PathBuf,
        created_directories: Vec<PathBuf>,
    }

    impl Drop for OracleOutputs {
        fn drop(&mut self) {
            for path in [&self.save, &self.state] {
                if path.symlink_metadata().is_ok() {
                    let _ = std::fs::remove_file(path);
                }
            }
            for path in self.created_directories.iter().rev() {
                let _ = std::fs::remove_dir(path);
            }
        }
    }

    #[derive(Clone, Debug, Eq, PartialEq, Serialize)]
    struct TreeSnapshot {
        existed: bool,
        entries: BTreeMap<String, TreeEntry>,
    }

    #[derive(Clone, Debug, Eq, PartialEq, Serialize)]
    struct TreeEntry {
        kind: &'static str,
        size: Option<u64>,
        sha256: Option<String>,
        symlink_target: Option<String>,
    }

    fn snapshot_tree(root: &Path) -> Result<TreeSnapshot> {
        let existed = root.try_exists()?;
        let mut entries = BTreeMap::new();
        if !existed {
            return Ok(TreeSnapshot { existed, entries });
        }
        ensure!(
            root.symlink_metadata()?.file_type().is_dir(),
            "Nestopia sync root is not a directory"
        );
        let mut pending = vec![root.to_path_buf()];
        while let Some(directory) = pending.pop() {
            let mut children =
                std::fs::read_dir(&directory)?.collect::<std::io::Result<Vec<_>>>()?;
            children.sort_by_key(|entry| entry.file_name());
            for child in children {
                let path = child.path();
                let relative = path
                    .strip_prefix(root)?
                    .to_str()
                    .context("Nestopia sync path is not UTF-8")?
                    .replace(std::path::MAIN_SEPARATOR, "/");
                let metadata = path.symlink_metadata()?;
                let file_type = metadata.file_type();
                let entry = if file_type.is_dir() {
                    pending.push(path);
                    TreeEntry {
                        kind: "directory",
                        size: None,
                        sha256: None,
                        symlink_target: None,
                    }
                } else if file_type.is_file() {
                    TreeEntry {
                        kind: "file",
                        size: Some(metadata.len()),
                        sha256: Some(file_hash(&path)?),
                        symlink_target: None,
                    }
                } else if file_type.is_symlink() {
                    TreeEntry {
                        kind: "symlink",
                        size: None,
                        sha256: None,
                        symlink_target: Some(
                            std::fs::read_link(&path)?
                                .to_str()
                                .context("Nestopia sync symlink target is not UTF-8")?
                                .to_owned(),
                        ),
                    }
                } else {
                    TreeEntry {
                        kind: "other",
                        size: None,
                        sha256: None,
                        symlink_target: None,
                    }
                };
                entries.insert(relative, entry);
            }
        }
        Ok(TreeSnapshot { existed, entries })
    }

    fn snapshot_roots(roots: &[crate::save_sync::RouteRoot]) -> Result<Vec<TreeSnapshot>> {
        roots.iter().map(|root| snapshot_tree(&root.path)).collect()
    }

    fn ensure_stopped(child: &mut ChildGuard) -> Result<()> {
        ensure!(
            child.child.try_wait()?.is_some(),
            "Refusing to synchronize while Nestopia is running"
        );
        Ok(())
    }

    fn unique_actions<'a>(actions: &'a [SyncAction], title: &str) -> Vec<&'a SyncAction> {
        let save = format!("saves/0/{title}.sav");
        let state = format!("states/0/{title}_0.nst");
        actions
            .iter()
            .filter(|action| {
                let key = action.key.as_str();
                key == save || key == state
            })
            .collect()
    }

    fn restore_file(path: &Path, bytes: &[u8], modified: SystemTime) -> Result<()> {
        std::fs::write(path, bytes)?;
        OpenOptions::new()
            .write(true)
            .open(path)?
            .set_times(FileTimes::new().set_modified(modified))?;
        Ok(())
    }

    fn applied_evidence(applied: &AppliedSync) -> serde_json::Value {
        serde_json::json!({
            "manifest_id": applied.manifest_id,
            "actions": applied.actions,
            "recovery_directory_created": applied.recovery_directory.is_some(),
        })
    }

    #[test]
    fn source_identity_is_a_full_commit() {
        assert_eq!(SOURCE_COMMIT.len(), 40);
        assert!(SOURCE_COMMIT.bytes().all(|byte| byte.is_ascii_hexdigit()));
    }

    fn oracle_calibration() -> Calibration {
        let button_for = BTreeMap::from([
            ("b", 0_u32),
            ("a", 1),
            ("select", 2),
            ("start", 3),
            ("up", 4),
            ("down", 5),
            ("left", 6),
            ("right", 7),
        ]);
        let layout = catalog().layout("nes").unwrap();
        Calibration {
            target_mappings: BTreeMap::new(),
            layout: "nes".into(),
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

    fn flatpak_location(flatpak: &Path, target: &str) -> Result<PathBuf> {
        let output = Command::new(flatpak)
            .args(["info", "--show-location", target])
            .output()?;
        ensure!(output.status.success(), "Could not locate {target}");
        let location = PathBuf::from(std::str::from_utf8(&output.stdout)?.trim());
        ensure!(location.is_absolute(), "Flatpak location is not absolute");
        Ok(location)
    }

    fn exact_window(
        window_tool: &Path,
        title: &str,
        x11: bool,
        deadline: Instant,
    ) -> Result<String> {
        loop {
            let output = if x11 {
                Command::new(window_tool)
                    .args(["search", "--all", "--limit", "2", "--name", title])
                    .output()?
            } else {
                Command::new(window_tool)
                    .args([
                        "search",
                        "--all",
                        "--case-sensitive",
                        "--title",
                        title,
                        "--limit",
                        "2",
                        "getwindowid",
                        "%@",
                        "getwindowname",
                        "%@",
                    ])
                    .output()?
            };
            ensure!(
                output.status.success()
                    || x11
                        && output.status.code() == Some(1)
                        && output.stdout.is_empty()
                        && output.stderr.is_empty(),
                "Could not inspect the exact Nestopia oracle window: {}",
                String::from_utf8_lossy(&output.stderr)
            );
            let fields: Vec<_> = std::str::from_utf8(&output.stdout)?
                .lines()
                .filter(|line| !line.is_empty())
                .collect();
            ensure!(
                fields.len() <= if x11 { 1 } else { 2 },
                "Found multiple exact Nestopia oracle windows"
            );
            if let Some(window) = fields.first()
                && (x11 || fields.get(1) == Some(&title.trim_matches('^').trim_matches('$')))
            {
                return Ok((*window).to_owned());
            }
            ensure!(
                Instant::now() < deadline,
                "The exact Nestopia oracle window did not become ready"
            );
            std::thread::sleep(Duration::from_millis(25));
        }
    }

    fn activate_window(window_tool: &Path, window: &str, title: &str, x11: bool) -> Result<()> {
        let mut command = Command::new(window_tool);
        command.arg(if x11 { "windowfocus" } else { "windowactivate" });
        if x11 {
            command.arg("--sync");
        }
        ensure!(
            command.arg(window).status()?.success(),
            "Could not activate the exact Nestopia oracle window"
        );
        let deadline = Instant::now() + Duration::from_secs(5);
        loop {
            let output = if x11 {
                let focus = Command::new(window_tool).arg("getwindowfocus").output()?;
                let name = Command::new(window_tool)
                    .args(["getwindowname", window])
                    .output()?;
                ensure!(
                    focus.status.success() && name.status.success(),
                    "Could not inspect exact X11 window"
                );
                format!(
                    "{}\n{}",
                    std::str::from_utf8(&focus.stdout)?.trim(),
                    std::str::from_utf8(&name.stdout)?.trim()
                )
            } else {
                let active = Command::new(window_tool)
                    .args([
                        "getactivewindow",
                        "getwindowid",
                        "%@",
                        "getwindowname",
                        "%@",
                    ])
                    .output()?;
                ensure!(active.status.success(), "Could not inspect active window");
                std::str::from_utf8(&active.stdout)?.trim().to_owned()
            };
            if output.lines().eq([window, title]) {
                return Ok(());
            }
            ensure!(
                Instant::now() < deadline,
                "The exact Nestopia oracle window did not become active"
            );
            std::thread::sleep(Duration::from_millis(25));
        }
    }

    fn send_function_key(
        window_tool: &Path,
        input_tool: &Path,
        input_socket: Option<&Path>,
        window: &str,
        title: &str,
        x11: bool,
        key: &str,
        evdev_code: u16,
    ) -> Result<()> {
        activate_window(window_tool, window, title, x11)?;
        let mut command = Command::new(input_tool);
        if x11 {
            // Nestopia checks the global X11 key state on FL_KEYUP. XSendEvent
            // (`xdotool --window`) does not update that state, so target the
            // exact, verified focus through XTEST instead.
            command.args(["key", "--clearmodifiers", key]);
        } else {
            command
                .env(
                    "YDOTOOL_SOCKET",
                    input_socket.context("Missing ydotool socket")?,
                )
                .args([
                    "key",
                    "-d",
                    "100",
                    &format!("{evdev_code}:1"),
                    &format!("{evdev_code}:0"),
                ]);
        }
        ensure!(command.status()?.success(), "Could not send Nestopia {key}");
        Ok(())
    }

    fn pulse(
        driver: &mut ChildGuard,
        lines: &mut impl Iterator<Item = std::io::Result<String>>,
        player: usize,
        button: usize,
    ) -> Result<()> {
        writeln!(
            driver
                .child
                .stdin
                .as_mut()
                .context("Pad stdin disappeared")?,
            "pulse {player} {button}"
        )?;
        let response = lines.next().context("Pad driver exited during pulse")??;
        ensure!(
            response == format!("PULSED {player} {button}"),
            "Unexpected pad-driver pulse response"
        );
        Ok(())
    }

    fn close_and_wait(
        window_tool: &Path,
        window: &str,
        x11: bool,
        emulator: &mut ChildGuard,
    ) -> Result<()> {
        let status = Command::new(window_tool)
            .args(["windowclose", window])
            .status()?;
        ensure!(status.success(), "Could not close exact Nestopia window");
        let deadline = Instant::now() + Duration::from_secs(20);
        loop {
            if let Some(status) = emulator.child.try_wait()? {
                ensure!(status.success(), "Nestopia oracle failed: {status}");
                return Ok(());
            }
            ensure!(
                Instant::now() < deadline,
                "Nestopia oracle did not exit cleanly"
            );
            std::thread::sleep(Duration::from_millis(if x11 { 25 } else { 50 }));
        }
    }

    fn assert_sram(bytes: &[u8], generation: u8, expected: &[(u8, u8)]) -> Result<()> {
        ensure!(bytes.len() == 8192, "Nestopia oracle SRAM size differs");
        ensure!(
            &bytes[..4] == b"LBNP"
                && bytes[4] == generation
                && usize::from(bytes[5]) == expected.len(),
            "Nestopia oracle SRAM header differs"
        );
        let observed: Vec<_> = bytes[0x10..0x10 + expected.len() * 2]
            .chunks_exact(2)
            .map(|pair| (pair[0], pair[1]))
            .collect();
        ensure!(observed == expected, "Nestopia oracle input log differs");
        Ok(())
    }

    /// Opt-in installed-runtime proof. It exercises the ordinary production
    /// prepare/spawn path across four fresh processes, uses the exact new
    /// compositor window for save/load/close actions, and removes only its
    /// unique save/state files.
    #[test]
    #[ignore = "needs installed Nestopia UE Flatpak, /dev/uinput, compositor tools and generated oracle artifacts"]
    fn production_two_pad_sram_and_state_oracle() -> Result<()> {
        let rom = PathBuf::from(
            std::env::var_os("LUNCHBOX_NESTOPIA_FLATPAK_ORACLE_ROM")
                .context("Missing Nestopia oracle ROM")?,
        )
        .canonicalize()?;
        let probe = PathBuf::from(
            std::env::var_os("LUNCHBOX_NESTOPIA_FLATPAK_ORACLE_PROBE")
                .context("Missing Nestopia oracle probe")?,
        )
        .canonicalize()?;
        let pad_driver = PathBuf::from(
            std::env::var_os("LUNCHBOX_NESTOPIA_FLATPAK_ORACLE_PAD_DRIVER")
                .context("Missing Nestopia oracle pad driver")?,
        )
        .canonicalize()?;
        let window_tool = PathBuf::from(
            std::env::var_os("LUNCHBOX_NESTOPIA_FLATPAK_ORACLE_WINDOW_TOOL")
                .context("Missing exact-window tool")?,
        )
        .canonicalize()?;
        let input_tool = PathBuf::from(
            std::env::var_os("LUNCHBOX_NESTOPIA_FLATPAK_ORACLE_INPUT_TOOL")
                .context("Missing exact-window input tool")?,
        )
        .canonicalize()?;
        let report_path = PathBuf::from(
            std::env::var_os("LUNCHBOX_NESTOPIA_FLATPAK_SYNC_REPORT")
                .context("Missing retained Nestopia sync report path")?,
        );
        ensure!(
            report_path.is_absolute() && !report_path.try_exists()?,
            "Nestopia sync report must be a new absolute path"
        );
        ensure!(
            report_path.parent().is_some_and(|parent| parent.is_dir()),
            "Nestopia sync report parent does not exist"
        );
        let x11 = window_tool == input_tool
            && window_tool
                .file_name()
                .is_some_and(|name| name == "xdotool");
        let input_socket = if x11 {
            None
        } else {
            let socket = PathBuf::from(
                std::env::var_os("LUNCHBOX_NESTOPIA_FLATPAK_ORACLE_INPUT_SOCKET")
                    .context("Missing ydotool socket")?,
            )
            .canonicalize()?;
            ensure!(
                std::fs::metadata(&socket)?.file_type().is_socket(),
                "Oracle input socket is not a Unix socket"
            );
            Some(socket)
        };
        let title = rom
            .file_stem()
            .and_then(|value| value.to_str())
            .context("Nestopia oracle ROM has no UTF-8 stem")?;
        ensure!(
            !title.is_empty()
                && title
                    .bytes()
                    .all(|byte| byte.is_ascii_alphanumeric() || byte == b'-'),
            "Nestopia oracle stem must be a nonempty ASCII word"
        );
        let title_pattern = format!("^{title}$");

        let flatpak = PathBuf::from("/run/current-system/sw/bin/flatpak").canonicalize()?;
        let cancel = AtomicBool::new(false);
        let running = Command::new(&flatpak).arg("ps").output()?;
        ensure!(
            running.status.success()
                && !String::from_utf8_lossy(&running.stdout).contains(flatpak::APP_ID),
            "Refusing to mix the oracle with an existing Nestopia process"
        );
        let home = directories::BaseDirs::new().context("Finding user home")?;
        let profile = home.home_dir().join(".var/app").join(flatpak::APP_ID);
        let source_main_config = profile.join("config/nestopia/nestopia.conf");
        let source_input_config = profile.join("config/nestopia/input.conf");
        let source_hashes = [
            file_hash(&source_main_config)?,
            file_hash(&source_input_config)?,
        ];
        let roots = save_route_roots_for_platform(
            &load_records()?,
            "nestopia-ue",
            "linux-flatpak",
            &LocationBases::detect(),
        )?;
        ensure!(
            roots.len() == 2,
            "Nestopia must expose exactly two sync routes"
        );
        let save_root = profile.join("data/nestopia/save");
        let state_root = profile.join("data/nestopia/state");
        ensure!(
            roots[0].route
                == (SaveRoute {
                    purpose: SavePurpose::Saves,
                    root_index: 0,
                })
                && roots[0].path == save_root
                && roots[1].route
                    == (SaveRoute {
                        purpose: SavePurpose::States,
                        root_index: 0,
                    })
                && roots[1].path == state_root
                && !save_root.starts_with(&state_root)
                && !state_root.starts_with(&save_root),
            "Nestopia sync routes differ from the exact disjoint sandbox contract"
        );
        let baseline_before = snapshot_roots(&roots)?;
        let created_directories = roots
            .iter()
            .zip(&baseline_before)
            .filter(|(_, snapshot)| !snapshot.existed)
            .map(|(root, _)| root.path.clone())
            .collect();
        let mut outputs = OracleOutputs {
            save: save_root.join(format!("{title}.sav")),
            state: state_root.join(format!("{title}_0.nst")),
            created_directories,
        };
        ensure!(
            !outputs.save.try_exists()? && !outputs.state.try_exists()?,
            "Nestopia oracle needs a unique ROM stem without existing outputs"
        );

        let mut driver = ChildGuard::new(
            Command::new(pad_driver)
                .stdin(Stdio::piped())
                .stdout(Stdio::piped())
                .spawn()?,
        );
        let mut lines = BufReader::new(
            driver
                .child
                .stdout
                .take()
                .context("Pad stdout disappeared")?,
        )
        .lines();
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

        let runtime_files = flatpak_location(&flatpak, flatpak::RUNTIME_REF)?
            .canonicalize()?
            .join("files");
        let sdl_library = runtime_files.join(flatpak::SDL_RELATIVE).canonicalize()?;
        let setup = settings::SavedSetup {
            emulator_id: "nestopia".into(),
            content: rom.clone(),
            source_main_config,
            source_input_config,
            probe_program: probe,
            sdl_library,
            executable_sha256: flatpak::APP_EXECUTABLE_SHA256.into(),
            players: vec![
                settings::Player {
                    player: 1,
                    controller_id: "nestopia-flatpak-oracle-p1".into(),
                },
                settings::Player {
                    player: 2,
                    controller_id: "nestopia-flatpak-oracle-p2".into(),
                },
            ],
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
            "nestopia".into(),
            "Nestopia UE".into(),
            EmulatorExecutable::Flatpak {
                command: flatpak.clone(),
                app_id: flatpak::APP_ID.into(),
            },
        );
        let directory = rom
            .parent()
            .context("Oracle ROM has no parent")?
            .to_path_buf();
        let plan = LaunchPlan {
            emulator_name: "Nestopia UE".into(),
            program: flatpak.clone(),
            arguments: vec![
                "run".into(),
                format!("--filesystem={}", directory.display()).into(),
                flatpak::APP_ID.into(),
                rom.as_os_str().to_owned(),
            ],
            current_directory: directory,
            environment: vec![],
            cleanup_paths: vec![],
            retroarch_content: None,
        };

        let scope = SyncScope::new("nestopia-ue", "linux-flatpak")?;
        let provider_root = tempfile::tempdir()?;
        let provider_profile =
            CloudProfile::new_local_folder(provider_root.path(), "device-a", true)?;
        let store = CloudStore::connect(
            provider_profile.provider,
            &provider_profile.root,
            &provider_profile.auth,
        )?;
        store.probe()?;
        let recovery = tempfile::tempdir()?;
        let save_key = ArtifactKey::new(
            SaveRoute {
                purpose: SavePurpose::Saves,
                root_index: 0,
            },
            &format!("{title}.sav"),
        )?;
        let state_key = ArtifactKey::new(
            SaveRoute {
                purpose: SavePurpose::States,
                root_index: 0,
            },
            &format!("{title}_0.nst"),
        )?;

        let mut first = prepare(&setup, &calibrations, &inventory, &option, &plan, &cancel)?;
        let deployed_executable_hash = file_hash(&first.executable)?;
        let launch = first.plan.clone();
        let mut first_emulator = ChildGuard::new(first.spawn(&launch, &cancel)?);
        let first_window = exact_window(
            &window_tool,
            &title_pattern,
            x11,
            Instant::now() + Duration::from_secs(20),
        )?;
        // The compositor surface can precede the frontend's loaded-game state;
        // its UI hotkeys are deliberately ignored until that transition.
        std::thread::sleep(Duration::from_secs(2));
        pulse(&mut driver, &mut lines, 1, 0)?;
        send_function_key(
            &window_tool,
            &input_tool,
            input_socket.as_deref(),
            &first_window,
            title,
            x11,
            "F5",
            63,
        )?;
        let deadline = Instant::now() + Duration::from_secs(10);
        while !outputs.state.is_file() {
            ensure!(
                Instant::now() < deadline,
                "Nestopia quick-save state did not appear"
            );
            std::thread::sleep(Duration::from_millis(25));
        }
        let state_hash = file_hash(&outputs.state)?;
        pulse(&mut driver, &mut lines, 2, 0)?;
        send_function_key(
            &window_tool,
            &input_tool,
            input_socket.as_deref(),
            &first_window,
            title,
            x11,
            "F7",
            65,
        )?;
        std::thread::sleep(Duration::from_millis(500));
        pulse(&mut driver, &mut lines, 1, 1)?;
        close_and_wait(&window_tool, &first_window, x11, &mut first_emulator)?;
        ensure_stopped(&mut first_emulator)?;
        drop(first);
        let first_sram = std::fs::read(&outputs.save)?;
        assert_sram(&first_sram, 1, &[(0x02, 0), (0x01, 0)])?;
        let first_sram_hash = file_hash(&outputs.save)?;
        let first_sram_modified = outputs.save.metadata()?.modified()?;
        let first_state = std::fs::read(&outputs.state)?;
        let first_state_modified = outputs.state.metadata()?.modified()?;

        // Device A publishes only after its exact runtime has stopped. Other
        // pre-existing saves may be present in this ephemeral memory store,
        // but the two oracle artifacts must map to the two distinct routes.
        let prepared_a = prepare_sync(&store, scope.clone(), "device-a", roots.clone(), None)?;
        ensure!(
            !prepared_a.plan.requires_user_choice()
                && unique_actions(&prepared_a.plan.actions, title).len() == 2
                && unique_actions(&prepared_a.plan.actions, title)
                    .iter()
                    .all(|action| action.kind == SyncActionKind::Upload),
            "Device A did not plan both unique Nestopia uploads"
        );
        let applied_a = prepared_a.apply(&store, &BTreeMap::new(), recovery.path(), 1_000)?;
        let manifest_a = store.get_manifest(&scope, &applied_a.manifest_id)?;
        ensure!(
            manifest_a.files.contains_key(&save_key) && manifest_a.files.contains_key(&state_key),
            "Device A manifest omitted a Nestopia route"
        );

        // Simulate a fresh device B data root without touching any unrelated
        // user entry. Pulling must restore exactly the absent unique save and
        // state from A's immutable manifest.
        std::fs::remove_file(&outputs.save)?;
        std::fs::remove_file(&outputs.state)?;
        ensure!(
            !outputs.save.try_exists()? && !outputs.state.try_exists()?,
            "Simulated device B did not start without the oracle outputs"
        );
        let prepared_b_pull = prepare_sync(&store, scope.clone(), "device-b", roots.clone(), None)?;
        ensure!(
            !prepared_b_pull.plan.requires_user_choice()
                && prepared_b_pull.plan.actions.len() == 2
                && unique_actions(&prepared_b_pull.plan.actions, title).len() == 2
                && prepared_b_pull
                    .plan
                    .actions
                    .iter()
                    .all(|action| action.kind == SyncActionKind::Download),
            "Device B did not plan exactly the two unique Nestopia downloads"
        );
        let applied_b_pull =
            prepared_b_pull.apply(&store, &BTreeMap::new(), recovery.path(), 2_000)?;
        ensure!(
            std::fs::read(&outputs.save)? == first_sram
                && std::fs::read(&outputs.state)? == first_state,
            "Device B did not restore Device A's exact Nestopia artifacts"
        );

        let mut second = prepare(&setup, &calibrations, &inventory, &option, &plan, &cancel)?;
        let launch = second.plan.clone();
        let mut second_emulator = ChildGuard::new(second.spawn(&launch, &cancel)?);
        let second_window = exact_window(
            &window_tool,
            &title_pattern,
            x11,
            Instant::now() + Duration::from_secs(20),
        )?;
        std::thread::sleep(Duration::from_secs(2));
        driver
            .child
            .stdin
            .as_mut()
            .context("Pad stdin disappeared")?
            .write_all(b"pulse-all\n")?;
        loop {
            if lines
                .next()
                .context("Pad driver exited during pulse-all")??
                == "PULSE-ALL-DONE"
            {
                break;
            }
        }
        close_and_wait(&window_tool, &second_window, x11, &mut second_emulator)?;
        ensure_stopped(&mut second_emulator)?;
        drop(second);

        let expected_buttons = [0x02_u8, 0x01, 0x04, 0x08, 0x10, 0x20, 0x40, 0x80];
        let mut expected = vec![(0x02, 0), (0x01, 0)];
        expected.extend(expected_buttons.map(|button| (button, 0)));
        expected.extend(expected_buttons.map(|button| (0, button)));
        let second_sram = std::fs::read(&outputs.save)?;
        assert_sram(&second_sram, 2, &expected)?;
        let second_sram_hash = file_hash(&outputs.save)?;
        let second_sram_modified = outputs.save.metadata()?.modified()?;
        ensure!(
            file_hash(&outputs.state)? == state_hash,
            "Nestopia state changed without another save-state action"
        );

        // A third fresh process behaviorally loads the state pulled by B. The
        // state was captured after P1 B, so loading it then pressing P2 A must
        // discard every later SRAM event and restore generation 1.
        let mut third = prepare(&setup, &calibrations, &inventory, &option, &plan, &cancel)?;
        let launch = third.plan.clone();
        let mut third_emulator = ChildGuard::new(third.spawn(&launch, &cancel)?);
        let third_window = exact_window(
            &window_tool,
            &title_pattern,
            x11,
            Instant::now() + Duration::from_secs(20),
        )?;
        std::thread::sleep(Duration::from_secs(2));
        send_function_key(
            &window_tool,
            &input_tool,
            input_socket.as_deref(),
            &third_window,
            title,
            x11,
            "F7",
            65,
        )?;
        std::thread::sleep(Duration::from_millis(500));
        pulse(&mut driver, &mut lines, 2, 1)?;
        close_and_wait(&window_tool, &third_window, x11, &mut third_emulator)?;
        ensure_stopped(&mut third_emulator)?;
        drop(third);
        let state_loaded_sram = std::fs::read(&outputs.save)?;
        assert_sram(&state_loaded_sram, 1, &[(0x02, 0), (0, 0x01)])?;
        let state_loaded_sram_hash = file_hash(&outputs.save)?;
        ensure!(
            file_hash(&outputs.state)? == state_hash,
            "Behavioral state load unexpectedly rewrote the pulled state"
        );

        // Restore B's generation-2 save byte-for-byte and with its captured
        // mtime so the coordinator sees only B's real stopped-runtime change.
        restore_file(&outputs.save, &second_sram, second_sram_modified)?;
        let prepared_b_upload =
            prepare_sync(&store, scope.clone(), "device-b", roots.clone(), None)?;
        ensure!(
            !prepared_b_upload.plan.requires_user_choice()
                && prepared_b_upload.plan.actions.len() == 1
                && prepared_b_upload.plan.actions[0].key == save_key
                && prepared_b_upload.plan.actions[0].kind == SyncActionKind::Upload,
            "Device B did not plan exactly its newer stopped-runtime save upload"
        );
        let applied_b_upload =
            prepared_b_upload.apply(&store, &BTreeMap::new(), recovery.path(), 3_000)?;

        // Return the unique physical files to A's earlier snapshot, select B's
        // descendant head, and prove the coordinator downloads only B's save.
        restore_file(&outputs.save, &first_sram, first_sram_modified)?;
        restore_file(&outputs.state, &first_state, first_state_modified)?;
        let prepared_a_pull = prepare_sync(
            &store,
            scope.clone(),
            "device-a",
            roots.clone(),
            Some("device-b"),
        )?;
        ensure!(
            !prepared_a_pull.plan.requires_user_choice()
                && prepared_a_pull.plan.actions.len() == 1
                && prepared_a_pull.plan.actions[0].key == save_key
                && prepared_a_pull.plan.actions[0].kind == SyncActionKind::Download,
            "Device A did not plan exactly B's newer save download"
        );
        let applied_a_pull =
            prepared_a_pull.apply(&store, &BTreeMap::new(), recovery.path(), 4_000)?;
        ensure!(
            std::fs::read(&outputs.save)? == second_sram
                && std::fs::read(&outputs.state)? == first_state,
            "Device A did not receive B's newer save while retaining the state"
        );

        // The fourth fresh installed-runtime process must load B's pulled
        // generation 2 and advance it to generation 3.
        let mut fourth = prepare(&setup, &calibrations, &inventory, &option, &plan, &cancel)?;
        let launch = fourth.plan.clone();
        let mut fourth_emulator = ChildGuard::new(fourth.spawn(&launch, &cancel)?);
        let fourth_window = exact_window(
            &window_tool,
            &title_pattern,
            x11,
            Instant::now() + Duration::from_secs(20),
        )?;
        std::thread::sleep(Duration::from_secs(2));
        pulse(&mut driver, &mut lines, 1, 2)?;
        close_and_wait(&window_tool, &fourth_window, x11, &mut fourth_emulator)?;
        ensure_stopped(&mut fourth_emulator)?;
        drop(fourth);
        let mut final_expected = expected.clone();
        final_expected.push((0x04, 0));
        let final_sram = std::fs::read(&outputs.save)?;
        assert_sram(&final_sram, 3, &final_expected)?;
        let final_sram_hash = file_hash(&outputs.save)?;
        ensure!(
            file_hash(&outputs.state)? == state_hash,
            "Final fresh launch unexpectedly rewrote the pulled state"
        );

        driver
            .child
            .stdin
            .as_mut()
            .context("Pad stdin disappeared")?
            .write_all(b"quit\n")?;
        ensure!(driver.child.wait()?.success(), "Pad driver failed");
        let running_after = Command::new(&flatpak).arg("ps").output()?;
        ensure!(
            running_after.status.success()
                && !String::from_utf8_lossy(&running_after.stdout).contains(flatpak::APP_ID),
            "Nestopia remained running after the sync oracle"
        );
        ensure!(
            [
                file_hash(&setup.source_main_config)?,
                file_hash(&setup.source_input_config)?,
            ] == source_hashes,
            "Nestopia source configuration changed during oracle"
        );

        // Restore the user's physical directory trees before writing the
        // success report. Only this unique generated fixture is removed.
        std::fs::remove_file(&outputs.save)?;
        std::fs::remove_file(&outputs.state)?;
        for directory in outputs.created_directories.iter().rev() {
            std::fs::remove_dir(directory)?;
        }
        outputs.created_directories.clear();
        let baseline_after = snapshot_roots(&roots)?;
        ensure!(
            baseline_after == baseline_before,
            "Nestopia user save/state baseline changed during the oracle"
        );

        let heads = store.device_heads(&scope)?;
        ensure!(
            heads.len() == 2
                && heads.iter().any(|head| {
                    head.device_id == "device-a" && head.manifest_id == applied_a_pull.manifest_id
                })
                && heads.iter().any(|head| {
                    head.device_id == "device-b" && head.manifest_id == applied_b_upload.manifest_id
                }),
            "Simulated device heads do not retain the expected merged history"
        );
        let provider_snapshot = snapshot_tree(provider_root.path())?;
        let provider_prefix = "lunchbox/saves/v1/nestopia-ue/linux-flatpak";
        for expected in [
            format!("{provider_prefix}/devices/device-a.json"),
            format!("{provider_prefix}/devices/device-b.json"),
            format!(
                "{provider_prefix}/manifests/{}.json",
                applied_a_pull.manifest_id
            ),
            format!(
                "{provider_prefix}/manifests/{}.json",
                applied_b_upload.manifest_id
            ),
        ] {
            ensure!(
                provider_snapshot
                    .entries
                    .get(&expected)
                    .is_some_and(|entry| entry.kind == "file"),
                "Local-folder provider omitted {expected}"
            );
        }
        ensure!(
            provider_snapshot
                .entries
                .values()
                .all(|entry| matches!(entry.kind, "directory" | "file")),
            "Local-folder provider tree contains a non-file entry"
        );
        let recovery_directories = std::fs::read_dir(recovery.path())?.count();
        let report = serde_json::json!({
            "schema": 1,
            "oracle": "nestopia-ue-linux-flatpak-save-state-sync",
            "provider": "local_folder",
            "provider_claim": {
                "local_folder": true,
                "network_cloud": false,
                "write_read_delete_probe": true,
                "ephemeral_root": provider_root.path(),
                "snapshot": provider_snapshot,
            },
            "scope": scope,
            "deployment": {
                "source_commit": SOURCE_COMMIT,
                "flatpak_app_id": flatpak::APP_ID,
                "flatpak_app_commit": flatpak::APP_COMMIT,
                "flatpak_executable_sha256": deployed_executable_hash,
                "flatpak_runtime": flatpak::RUNTIME_REF,
                "flatpak_runtime_commit": flatpak::RUNTIME_COMMIT,
                "sdl_sha256": file_hash(&setup.sdl_library)?,
                "probe_sha256": file_hash(&setup.probe_program)?,
            },
            "fixture": {
                "rom": rom,
                "rom_sha256": file_hash(&setup.content)?,
                "rom_bytes": setup.content.metadata()?.len(),
                "unique_stem": title,
            },
            "routes": roots.iter().map(|root| serde_json::json!({
                "purpose": root.route.purpose.as_str(),
                "root_index": root.route.root_index,
                "path": root.path,
                "create_if_missing": root.create_if_missing,
            })).collect::<Vec<_>>(),
            "device_a_initial": {
                "save_sha256": first_sram_hash,
                "state_sha256": state_hash,
                "save_generation": 1,
                "manifest_save_version": manifest_a.files.get(&save_key),
                "manifest_state_version": manifest_a.files.get(&state_key),
                "sync": applied_evidence(&applied_a),
            },
            "device_b_restore": {
                "removed_unique_outputs_before_pull": true,
                "restored_save_sha256": first_sram_hash,
                "restored_state_sha256": state_hash,
                "sync": applied_evidence(&applied_b_pull),
            },
            "device_b_fresh_reload": {
                "save_sha256": second_sram_hash,
                "save_generation": 2,
                "events": expected.len(),
            },
            "device_b_pulled_state_behavior": {
                "save_sha256_after_load": state_loaded_sram_hash,
                "restored_generation": 1,
                "restored_events": 2,
            },
            "device_b_newer_upload": applied_evidence(&applied_b_upload),
            "device_a_newer_pull": applied_evidence(&applied_a_pull),
            "device_a_final_fresh_reload": {
                "save_sha256": final_sram_hash,
                "save_generation": 3,
                "events": final_expected.len(),
            },
            "device_heads": heads,
            "safety": {
                "syncs_started_only_after_observed_process_exit": true,
                "source_config_hashes_before": source_hashes,
                "source_config_hashes_after": [
                    file_hash(&setup.source_main_config)?,
                    file_hash(&setup.source_input_config)?,
                ],
                "baseline_before": baseline_before,
                "baseline_after": baseline_after,
                "baseline_unchanged": true,
                "unique_outputs_removed": true,
                "nestopia_process_reaped": true,
                "pad_driver_reaped": true,
                "temporary_recovery_directories": recovery_directories,
            },
        });
        let mut report_file = OpenOptions::new()
            .write(true)
            .create_new(true)
            .mode(0o600)
            .open(&report_path)?;
        serde_json::to_writer_pretty(&mut report_file, &report)?;
        report_file.write_all(b"\n")?;
        report_file.sync_all()?;
        eprintln!(
            "Nestopia sync oracle passed: final_save_sha256={} state_sha256={} generations=1,2,3 events={} report={}",
            final_sram_hash,
            state_hash,
            final_expected.len(),
            report_path.display()
        );
        Ok(())
    }
}
