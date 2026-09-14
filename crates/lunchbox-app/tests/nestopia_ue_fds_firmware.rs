#![cfg(target_os = "linux")]

//! Exercise the isolated Nestopia UE FDS helper before central registration.

#[path = "../src/nestopia_ue_fds_firmware.rs"]
mod firmware;

use std::fs::{self, OpenOptions};
use std::io::Write;
use std::os::unix::fs::{MetadataExt, OpenOptionsExt, PermissionsExt, symlink};
use std::path::{Path, PathBuf};
use std::process::{Child, Command};
use std::time::{Duration, Instant};

use anyhow::{Context, Result, ensure};
use firmware::{
    FDS_BIOS_BYTES, FDS_BIOS_FILENAME, FDS_MINERVA_PACKAGE_NAME, FDS_PLATFORM_ID,
    FDS_PLATFORM_NAME, FdsRuntimeScope, InstallDisposition, InstallPhase, InstallRequest,
    NESTOPIA_UE_FLATPAK_ID, RECOGNIZED_FDS_BIOS_CRC32, RECOGNIZED_FDS_BIOS_SHA256,
    RUNTIME_PROOF_BLOCKER, inspect_fds_bios, inspect_fds_bios_for_test, install_fds_firmware,
    install_fds_firmware_for_test, verify_fds_firmware_for_launch, verify_fds_firmware_for_test,
};
use sha2::{Digest, Sha256};
use tempfile::TempDir;

fn test_bios(seed: u8) -> [u8; FDS_BIOS_BYTES] {
    let mut bytes = [seed; FDS_BIOS_BYTES];
    for (index, byte) in bytes.iter_mut().enumerate() {
        *byte = byte.wrapping_add((index % 251) as u8);
    }
    bytes
}

fn sha256(bytes: &[u8]) -> String {
    hex::encode(Sha256::digest(bytes))
}

fn allowlist(bytes: &[u8]) -> Vec<String> {
    vec![sha256(bytes)]
}

// Construct a same-CRC/different-SHA payload to prove CRC is diagnostic only.
fn bytes_with_crc(target: u32) -> [u8; FDS_BIOS_BYTES] {
    let baseline = [0_u8; FDS_BIOS_BYTES];
    let baseline_crc = crc32fast::hash(&baseline);
    let mut basis = [None; 32];
    for input_bit in 0..32 {
        let mut candidate = baseline;
        candidate[FDS_BIOS_BYTES - 4 + input_bit / 8] ^= 1 << (input_bit % 8);
        let mut effect = crc32fast::hash(&candidate) ^ baseline_crc;
        let mut inputs = 1_u32 << input_bit;
        for output_bit in (0..32).rev() {
            if effect & (1_u32 << output_bit) == 0 {
                continue;
            }
            if let Some((basis_effect, basis_inputs)) = basis[output_bit] {
                effect ^= basis_effect;
                inputs ^= basis_inputs;
            } else {
                basis[output_bit] = Some((effect, inputs));
                effect = 0;
                break;
            }
        }
        assert_eq!(effect, 0);
    }
    let mut remaining = target ^ baseline_crc;
    let mut inputs = 0;
    for output_bit in (0..32).rev() {
        if remaining & (1_u32 << output_bit) != 0 {
            let (effect, effect_inputs) = basis[output_bit].unwrap();
            remaining ^= effect;
            inputs ^= effect_inputs;
        }
    }
    assert_eq!(remaining, 0);
    let mut bytes = baseline;
    for input_bit in 0..32 {
        if inputs & (1_u32 << input_bit) != 0 {
            bytes[FDS_BIOS_BYTES - 4 + input_bit / 8] ^= 1 << (input_bit % 8);
        }
    }
    assert_eq!(crc32fast::hash(&bytes), target);
    bytes
}

fn flatpak_paths(temp: &TempDir) -> (PathBuf, PathBuf) {
    let home = temp.path().join("home");
    let data = home
        .join(".var/app")
        .join(NESTOPIA_UE_FLATPAK_ID)
        .join("data");
    fs::create_dir_all(&data).unwrap();
    (
        fs::canonicalize(home).unwrap(),
        fs::canonicalize(data).unwrap(),
    )
}

fn write_file(temp: &TempDir, name: &str, bytes: &[u8]) -> PathBuf {
    let path = temp.path().join(name);
    fs::write(&path, bytes).unwrap();
    fs::canonicalize(path).unwrap()
}

fn write_zip(temp: &TempDir, name: &str, entries: &[(&str, &[u8])]) -> PathBuf {
    let path = temp.path().join(name);
    let file = fs::File::create(&path).unwrap();
    let mut archive = zip::ZipWriter::new(file);
    for (entry_name, bytes) in entries {
        archive
            .start_file(*entry_name, zip::write::SimpleFileOptions::default())
            .unwrap();
        archive.write_all(bytes).unwrap();
    }
    archive.finish().unwrap();
    fs::canonicalize(path).unwrap()
}

fn content(temp: &TempDir, name: &str) -> PathBuf {
    write_file(temp, name, b"lawfully owned test content")
}

fn request<'a>(data: &'a Path, source: &'a Path, content: &'a Path) -> InstallRequest<'a> {
    InstallRequest {
        flatpak_app_id: NESTOPIA_UE_FLATPAK_ID,
        platform_id: FDS_PLATFORM_ID,
        platform_name: FDS_PLATFORM_NAME,
        content_path: content,
        flatpak_data_home: data,
        firmware_source: source,
    }
}

fn scope<'a>(data: &'a Path, content: &'a Path) -> FdsRuntimeScope<'a> {
    FdsRuntimeScope {
        flatpak_app_id: NESTOPIA_UE_FLATPAK_ID,
        platform_id: FDS_PLATFORM_ID,
        platform_name: FDS_PLATFORM_NAME,
        content_path: content,
        flatpak_data_home: data,
    }
}

fn uid() -> u32 {
    // SAFETY: geteuid has no preconditions.
    unsafe { libc::geteuid() }
}

fn install(
    home: &Path,
    data: &Path,
    source: &Path,
    game: &Path,
    allowed: &[String],
) -> Result<firmware::InstallReceipt> {
    install_fds_firmware_for_test(request(data, source, game), home, allowed, uid(), |_| {})
}

struct RuntimeCleanup {
    outputs: Vec<PathBuf>,
    firmware_target: PathBuf,
    remove_firmware: bool,
    remove_firmware_directory: bool,
}

struct RuntimeChild {
    child: Child,
}

impl RuntimeChild {
    fn new(child: Child) -> Self {
        Self { child }
    }
}

impl Drop for RuntimeChild {
    fn drop(&mut self) {
        if self.child.try_wait().ok().flatten().is_none() {
            let _ = self.child.kill();
            let _ = self.child.wait();
        }
    }
}

impl RuntimeCleanup {
    fn finish(&mut self) -> Result<()> {
        for output in &self.outputs {
            if output.symlink_metadata().is_ok() {
                fs::remove_file(output)
                    .with_context(|| format!("removing FDS oracle output {}", output.display()))?;
            }
        }
        if self.remove_firmware && self.firmware_target.symlink_metadata().is_ok() {
            fs::remove_file(&self.firmware_target).context("removing installed FDS oracle BIOS")?;
        }
        self.remove_firmware = false;
        if self.remove_firmware_directory {
            fs::remove_dir(
                self.firmware_target
                    .parent()
                    .context("FDS firmware target has no parent")?,
            )
            .context("removing FDS oracle firmware directory")?;
        }
        self.remove_firmware_directory = false;
        Ok(())
    }
}

impl Drop for RuntimeCleanup {
    fn drop(&mut self) {
        for output in &self.outputs {
            if output.symlink_metadata().is_ok() {
                let _ = fs::remove_file(output);
            }
        }
        if self.remove_firmware && self.firmware_target.symlink_metadata().is_ok() {
            let _ = fs::remove_file(&self.firmware_target);
        }
        if self.remove_firmware_directory
            && let Some(parent) = self.firmware_target.parent()
        {
            let _ = fs::remove_dir(parent);
        }
    }
}

fn command_stdout(command: &mut Command, description: &str) -> Result<String> {
    let output = command.output()?;
    ensure!(
        output.status.success(),
        "{description} failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    Ok(std::str::from_utf8(&output.stdout)?.trim().to_owned())
}

fn wait_for_exact_x11_window(xdotool: &Path, title: &str) -> Result<String> {
    let pattern = format!("^{title}$");
    let deadline = Instant::now() + Duration::from_secs(20);
    loop {
        let output = Command::new(xdotool)
            .args(["search", "--all", "--limit", "2", "--name", &pattern])
            .output()?;
        ensure!(
            output.status.success()
                || output.status.code() == Some(1)
                    && output.stdout.is_empty()
                    && output.stderr.is_empty(),
            "could not inspect the exact FDS oracle window: {}",
            String::from_utf8_lossy(&output.stderr)
        );
        let windows = std::str::from_utf8(&output.stdout)?
            .lines()
            .filter(|line| !line.is_empty())
            .collect::<Vec<_>>();
        ensure!(
            windows.len() <= 1,
            "found multiple exact FDS oracle windows"
        );
        if let Some(window) = windows.first() {
            let observed = command_stdout(
                Command::new(xdotool).args(["getwindowname", window]),
                "reading FDS oracle window title",
            )?;
            if observed == title {
                return Ok((*window).to_owned());
            }
        }
        ensure!(
            Instant::now() < deadline,
            "the exact FDS oracle window did not become ready"
        );
        std::thread::sleep(Duration::from_millis(25));
    }
}

fn focus_and_key(xdotool: &Path, window: &str, title: &str, key: &str) -> Result<()> {
    ensure!(
        Command::new(xdotool)
            .args(["windowfocus", "--sync", window])
            .status()?
            .success(),
        "could not focus the exact FDS oracle window"
    );
    let focused = command_stdout(
        Command::new(xdotool).arg("getwindowfocus"),
        "reading focused FDS oracle window",
    )?;
    let observed = command_stdout(
        Command::new(xdotool).args(["getwindowname", window]),
        "reading focused FDS oracle title",
    )?;
    ensure!(
        focused == window && observed == title,
        "FDS oracle focus changed"
    );
    ensure!(
        Command::new(xdotool)
            .args(["key", "--clearmodifiers", key])
            .status()?
            .success(),
        "could not send {key} to the FDS oracle"
    );
    Ok(())
}

/// Opt-in proof using user-supplied lawful inputs. The source BIOS and source
/// disk are read-only inputs; all installed/runtime artifacts are restored to
/// their pre-test state before the report is retained.
#[test]
#[ignore = "needs installed pinned Nestopia UE Flatpak, Xvfb/xdotool, and user-supplied lawful FDS BIOS/game"]
fn production_fds_install_verify_and_state_runtime_oracle() -> Result<()> {
    const APP_COMMIT: &str = "0f3d30d71419cee53f903b77944821d919fcfbe98aa3648bf8d12765bddd6169";
    const APP_EXECUTABLE_SHA256: &str =
        "1b63638f9e19e007900ac7c81451dbaa734661f79e3033d7013dbec2509543ea";
    const RUNTIME_REF: &str = "org.freedesktop.Platform/x86_64/25.08";
    const RUNTIME_COMMIT: &str = "bd44a6230581917d04f89812a4c21090c304d390edb73995af1c2f9fd8abf4e8";

    let bios = fs::canonicalize(
        std::env::var_os("LUNCHBOX_NESTOPIA_FDS_BIOS").context("missing FDS BIOS path")?,
    )?;
    let content = fs::canonicalize(
        std::env::var_os("LUNCHBOX_NESTOPIA_FDS_CONTENT")
            .context("missing extracted FDS content path")?,
    )?;
    let xdotool = fs::canonicalize(
        std::env::var_os("LUNCHBOX_NESTOPIA_FDS_XDOTOOL").context("missing xdotool path")?,
    )?;
    let report_path = PathBuf::from(
        std::env::var_os("LUNCHBOX_NESTOPIA_FDS_REPORT").context("missing FDS report path")?,
    );
    ensure!(
        report_path.is_absolute() && !report_path.try_exists()?,
        "FDS report must be a new absolute path"
    );
    ensure!(
        report_path.parent().is_some_and(Path::is_dir),
        "FDS report parent does not exist"
    );
    let source_zip = std::env::var_os("LUNCHBOX_NESTOPIA_FDS_SOURCE_ZIP")
        .map(fs::canonicalize)
        .transpose()?;
    let title = content
        .file_stem()
        .and_then(|value| value.to_str())
        .context("FDS content has no UTF-8 stem")?;
    ensure!(
        !title.is_empty()
            && title
                .bytes()
                .all(|byte| byte.is_ascii_alphanumeric() || byte == b'-'),
        "FDS oracle content needs a unique ASCII word stem"
    );
    ensure!(
        matches!(
            content.extension().and_then(|value| value.to_str()),
            Some("fds" | "FDS")
        ),
        "FDS oracle content extension differs"
    );

    let flatpak = fs::canonicalize("/run/current-system/sw/bin/flatpak")?;
    let running = command_stdout(
        Command::new(&flatpak).arg("ps"),
        "listing Flatpak processes",
    )?;
    ensure!(
        !running.contains(NESTOPIA_UE_FLATPAK_ID),
        "refusing to mix the FDS oracle with an existing Nestopia process"
    );
    let app_commit = command_stdout(
        Command::new(&flatpak).args(["info", "--show-commit", NESTOPIA_UE_FLATPAK_ID]),
        "reading Nestopia Flatpak commit",
    )?;
    ensure!(app_commit == APP_COMMIT, "Nestopia Flatpak commit differs");
    let runtime_ref = command_stdout(
        Command::new(&flatpak).args(["info", "--show-runtime", NESTOPIA_UE_FLATPAK_ID]),
        "reading Nestopia runtime ref",
    )?;
    ensure!(runtime_ref == RUNTIME_REF, "Nestopia runtime ref differs");
    let runtime_commit = command_stdout(
        Command::new(&flatpak).args(["info", "--show-commit", RUNTIME_REF]),
        "reading Nestopia runtime commit",
    )?;
    ensure!(
        runtime_commit == RUNTIME_COMMIT,
        "Nestopia runtime commit differs"
    );
    let app_location = PathBuf::from(command_stdout(
        Command::new(&flatpak).args(["info", "--show-location", NESTOPIA_UE_FLATPAK_ID]),
        "locating Nestopia Flatpak",
    )?);
    let executable = fs::canonicalize(app_location.join("files/bin/nestopia"))?;
    let executable_sha256 = sha256(&fs::read(&executable)?);
    ensure!(
        executable_sha256 == APP_EXECUTABLE_SHA256,
        "Nestopia executable identity differs"
    );

    let bios_before = fs::read(&bios)?;
    let bios_metadata_before = bios.metadata()?;
    let inspected = inspect_fds_bios(&bios)?;
    let home = fs::canonicalize(
        directories::BaseDirs::new()
            .context("finding user home")?
            .home_dir(),
    )?;
    let data_home = fs::canonicalize(
        home.join(".var/app")
            .join(NESTOPIA_UE_FLATPAK_ID)
            .join("data"),
    )?;
    let firmware_directory = data_home.join("nestopia");
    let firmware_directory_existed = firmware_directory.is_dir();
    let firmware_target = firmware_directory.join(FDS_BIOS_FILENAME);
    let firmware_existed = firmware_target.is_file();
    let firmware_before = firmware_existed
        .then(|| fs::read(&firmware_target))
        .transpose()?;
    let save_root = firmware_directory.join("save");
    let state_root = firmware_directory.join("state");
    let outputs = vec![
        save_root.join(format!("{title}.sav")),
        save_root.join(format!("{title}.ups")),
        state_root.join(format!("{title}_0.nst")),
        content.with_extension("ups"),
    ];
    ensure!(
        outputs
            .iter()
            .all(|path| !path.try_exists().unwrap_or(false)),
        "FDS oracle needs a unique content stem without existing outputs"
    );
    let mut cleanup = RuntimeCleanup {
        outputs,
        firmware_target: firmware_target.clone(),
        remove_firmware: !firmware_existed,
        remove_firmware_directory: !firmware_directory_existed,
    };

    let receipt = install_fds_firmware(InstallRequest {
        flatpak_app_id: NESTOPIA_UE_FLATPAK_ID,
        platform_id: FDS_PLATFORM_ID,
        platform_name: FDS_PLATFORM_NAME,
        content_path: &content,
        flatpak_data_home: &data_home,
        firmware_source: &bios,
    })?;
    ensure!(receipt.target == firmware_target, "FDS BIOS target differs");
    let verified = verify_fds_firmware_for_launch(scope(&data_home, &content))?;
    ensure!(
        verified.target == firmware_target && verified.fingerprint == inspected.fingerprint,
        "fresh FDS pre-launch verification differs"
    );

    let private = TempDir::new()?;
    let private_config = private.path().join("config");
    fs::create_dir(&private_config)?;
    let mut child = RuntimeChild::new(
        Command::new(&flatpak)
            .args([
                "run",
                "--die-with-parent",
                "--nofilesystem=host:reset",
                "--nofilesystem=home",
                "--nosocket=wayland",
                "--socket=x11",
            ])
            .arg(format!(
                "--filesystem={}",
                content.parent().unwrap().display()
            ))
            .arg(format!("--filesystem={}", private.path().display()))
            .arg("--env=FLTK_BACKEND=x11")
            .arg(format!(
                "--env=XDG_CONFIG_HOME={}",
                private_config.display()
            ))
            .arg(NESTOPIA_UE_FLATPAK_ID)
            .arg(&content)
            .spawn()?,
    );
    let window = wait_for_exact_x11_window(&xdotool, title)?;
    std::thread::sleep(Duration::from_secs(2));
    focus_and_key(&xdotool, &window, title, "F5")?;
    let state = state_root.join(format!("{title}_0.nst"));
    let deadline = Instant::now() + Duration::from_secs(10);
    while !state.is_file() {
        ensure!(
            child.child.try_wait()?.is_none(),
            "Nestopia exited before creating the FDS state"
        );
        ensure!(Instant::now() < deadline, "FDS state did not appear");
        std::thread::sleep(Duration::from_millis(25));
    }
    let state_bytes = fs::read(&state)?;
    ensure!(!state_bytes.is_empty(), "FDS state is empty");
    let state_sha256 = sha256(&state_bytes);
    focus_and_key(&xdotool, &window, title, "F7")?;
    std::thread::sleep(Duration::from_millis(500));
    ensure!(
        child.child.try_wait()?.is_none(),
        "Nestopia exited after loading the FDS state"
    );
    ensure!(
        Command::new(&xdotool)
            .args(["windowclose", &window])
            .status()?
            .success(),
        "could not close the exact FDS oracle window"
    );
    let deadline = Instant::now() + Duration::from_secs(20);
    let status = loop {
        if let Some(status) = child.child.try_wait()? {
            break status;
        }
        ensure!(
            Instant::now() < deadline,
            "Nestopia FDS oracle did not exit"
        );
        std::thread::sleep(Duration::from_millis(25));
    };
    ensure!(status.success(), "Nestopia FDS oracle failed: {status}");
    let post_launch = verify_fds_firmware_for_launch(scope(&data_home, &content))?;
    ensure!(
        post_launch.fingerprint == inspected.fingerprint,
        "installed FDS BIOS changed during runtime"
    );
    cleanup.finish()?;

    ensure!(fs::read(&bios)? == bios_before, "source FDS BIOS changed");
    let bios_metadata_after = bios.metadata()?;
    ensure!(
        bios_metadata_before.len() == bios_metadata_after.len()
            && bios_metadata_before.modified()? == bios_metadata_after.modified()?,
        "source FDS BIOS metadata changed"
    );
    if let Some(before) = firmware_before {
        ensure!(
            fs::read(&firmware_target)? == before,
            "pre-existing FDS BIOS changed"
        );
    } else {
        ensure!(
            !firmware_target.try_exists()?,
            "temporary installed FDS BIOS remained"
        );
    }
    ensure!(
        cleanup
            .outputs
            .iter()
            .all(|path| !path.try_exists().unwrap_or(false)),
        "unique FDS runtime outputs remained"
    );
    // The reaped child's sandbox can linger briefly in `flatpak ps`
    // after a clean exit; poll for teardown instead of asserting once.
    let deadline = Instant::now() + Duration::from_secs(20);
    loop {
        let running = command_stdout(
            Command::new(&flatpak).arg("ps"),
            "listing final Flatpak processes",
        )?
        .contains(NESTOPIA_UE_FLATPAK_ID);
        if !running {
            break;
        }
        ensure!(
            Instant::now() < deadline,
            "Nestopia remained running after the FDS oracle"
        );
        std::thread::sleep(Duration::from_millis(250));
    }

    let source_zip_sha256 = source_zip
        .as_ref()
        .map(|path| fs::read(path).map(|bytes| sha256(&bytes)))
        .transpose()?;
    let report = serde_json::json!({
        "schema": 1,
        "oracle": "nestopia-ue-linux-flatpak-fds-runtime",
        "deployment": {
            "flatpak_app_id": NESTOPIA_UE_FLATPAK_ID,
            "flatpak_app_commit": app_commit,
            "flatpak_executable_sha256": executable_sha256,
            "flatpak_runtime": runtime_ref,
            "flatpak_runtime_commit": runtime_commit,
        },
        "inputs": {
            "bios": bios,
            "bios_sha256": inspected.fingerprint.sha256,
            "bios_crc32": format!("{:08x}", inspected.fingerprint.crc32),
            "content": content,
            "content_sha256": sha256(&fs::read(&content)?),
            "source_zip": source_zip,
            "source_zip_sha256": source_zip_sha256,
        },
        "firmware": {
            "install_disposition": format!("{:?}", receipt.disposition),
            "target": receipt.target,
            "immediate_prelaunch_reverified": true,
            "post_runtime_reverified": true,
        },
        "runtime": {
            "display_backend": "x11",
            "exact_window_title": title,
            "state_bytes": state_bytes.len(),
            "state_sha256": state_sha256,
            "f5_created_state": true,
            "f7_process_remained_live": true,
            "clean_exit": true,
        },
        "safety": {
            "bios_source_unchanged": true,
            "firmware_baseline_restored": true,
            "unique_outputs_removed": true,
            "nestopia_process_reaped": true,
            "private_config_removed": true,
        },
        "minerva_follow_up": "Retroarch-System/Nintendo - NES - Famicom (Nestopia UE).zip",
    });
    let mut report_file = OpenOptions::new()
        .write(true)
        .create_new(true)
        .mode(0o600)
        .open(&report_path)?;
    serde_json::to_writer_pretty(&mut report_file, &report)?;
    report_file.write_all(b"\n")?;
    report_file.sync_all()?;
    Ok(())
}

#[test]
fn production_allowlist_matches_pinned_mesen_primary_source() {
    // SourMesen/Mesen2 UI/Interop/FirmwareTypeExtensions.cs
    // @ b9fa69ddc6d0a331fb103fdb5eef6904305703c2.
    assert_eq!(
        RECOGNIZED_FDS_BIOS_SHA256,
        [
            "99c18490ed9002d9c6d999b9d8d15be5c051bdfa7cc7e73318053c9a994b0178",
            "a0a9d57cbace21bf9c85c2b85e86656317f0768d7772acc90c7411ab1dbff2bf",
        ]
    );
}

#[test]
fn production_acceptance_is_sha256_not_crc32() -> Result<()> {
    let temp = TempDir::new()?;
    for crc32 in RECOGNIZED_FDS_BIOS_CRC32 {
        let path = write_file(
            &temp,
            &format!("same-crc-{crc32:08x}.rom"),
            &bytes_with_crc(crc32),
        );
        let error = inspect_fds_bios(&path).unwrap_err().to_string();
        assert!(error.contains("unrecognized FDS BIOS SHA-256"));
        assert!(error.contains(&format!("CRC32 {crc32:08x}")));
    }
    Ok(())
}

#[test]
fn test_hashes_accept_only_exact_raw_or_reviewed_zip_sources() -> Result<()> {
    let temp = TempDir::new()?;
    let bytes = test_bios(17);
    let allowed = allowlist(&bytes);
    let source = write_file(&temp, "selected.rom", &bytes);
    let inspected = inspect_fds_bios_for_test(&source, &allowed)?;
    assert_eq!(inspected.bytes(), &bytes);
    assert_eq!(inspected.fingerprint.sha256, allowed[0]);

    let archive = write_zip(
        &temp,
        FDS_MINERVA_PACKAGE_NAME,
        &[("system/disksys.rom", &bytes), ("README.txt", b"metadata")],
    );
    let inspected = inspect_fds_bios_for_test(&archive, &allowed)?;
    assert_eq!(inspected.bytes(), &bytes);
    assert_eq!(inspected.fingerprint.sha256, allowed[0]);

    let unsafe_archive = write_zip(&temp, "unsafe.zip", &[("../disksys.rom", &bytes)]);
    assert!(
        inspect_fds_bios_for_test(&unsafe_archive, &allowed)
            .unwrap_err()
            .to_string()
            .contains("unsafe path")
    );
    let duplicate_archive = write_zip(
        &temp,
        "duplicate.zip",
        &[("a/disksys.rom", &bytes), ("b/DISKSYS.ROM", &bytes)],
    );
    assert!(
        inspect_fds_bios_for_test(&duplicate_archive, &allowed)
            .unwrap_err()
            .to_string()
            .contains("multiple disksys.rom")
    );
    let missing_archive = write_zip(&temp, "missing.zip", &[("readme.txt", b"metadata")]);
    assert!(
        inspect_fds_bios_for_test(&missing_archive, &allowed)
            .unwrap_err()
            .to_string()
            .contains("no recognized disksys.rom")
    );

    assert!(
        inspect_fds_bios_for_test(Path::new("relative.rom"), &allowed)
            .unwrap_err()
            .to_string()
            .contains("must be absolute")
    );
    let short = write_file(&temp, "short.rom", &bytes[..FDS_BIOS_BYTES - 1]);
    assert!(
        inspect_fds_bios_for_test(&short, &allowed)
            .unwrap_err()
            .to_string()
            .contains("8192-byte")
    );
    let linked = temp.path().join("linked.rom");
    symlink(&source, &linked)?;
    assert!(
        inspect_fds_bios_for_test(&linked, &allowed)
            .unwrap_err()
            .to_string()
            .contains("non-symlink")
    );
    let actual = temp.path().join("actual");
    fs::create_dir(&actual)?;
    fs::write(actual.join("bios.rom"), bytes)?;
    let linked_dir = temp.path().join("linked-dir");
    symlink(&actual, &linked_dir)?;
    assert!(
        inspect_fds_bios_for_test(&linked_dir.join("bios.rom"), &allowed)
            .unwrap_err()
            .to_string()
            .contains("canonical path")
    );
    Ok(())
}

#[test]
fn atomic_install_is_private_preserves_state_and_reverifies() -> Result<()> {
    let temp = TempDir::new()?;
    let (home, data) = flatpak_paths(&temp);
    let bytes = test_bios(33);
    let allowed = allowlist(&bytes);
    let source = write_file(&temp, "source.rom", &bytes);
    let game = content(&temp, "game.FDS");
    let sentinel = data.join("unrelated-state");
    fs::write(&sentinel, b"keep")?;
    let installed = install(&home, &data, &source, &game, &allowed)?;
    assert_eq!(installed.disposition, InstallDisposition::Installed);
    assert_eq!(installed.target, data.join("nestopia/disksys.rom"));
    assert_eq!(fs::read(&installed.target)?, bytes);
    let metadata = fs::metadata(&installed.target)?;
    assert_eq!(metadata.permissions().mode() & 0o7777, 0o600);
    assert_eq!(metadata.nlink(), 1);
    assert_eq!(fs::read(&sentinel)?, b"keep");
    assert_eq!(fs::read_dir(data.join("nestopia"))?.count(), 1);

    let verified = verify_fds_firmware_for_test(scope(&data, &game), &home, &allowed, uid())?;
    assert_eq!(verified.disposition, InstallDisposition::AlreadyPresent);
    assert_eq!(verified.fingerprint, installed.fingerprint);
    assert_eq!(
        install(&home, &data, &source, &game, &allowed)?.disposition,
        InstallDisposition::AlreadyPresent
    );
    Ok(())
}

#[test]
fn prelaunch_reverification_detects_mode_link_content_and_symlink_drift() -> Result<()> {
    let temp = TempDir::new()?;
    let (home, data) = flatpak_paths(&temp);
    let bytes = test_bios(44);
    let allowed = allowlist(&bytes);
    let source = write_file(&temp, "source.rom", &bytes);
    let game = content(&temp, "game.fds");
    let installed = install(&home, &data, &source, &game, &allowed)?;

    fs::set_permissions(&installed.target, fs::Permissions::from_mode(0o644))?;
    assert!(
        verify_fds_firmware_for_test(scope(&data, &game), &home, &allowed, uid())
            .unwrap_err()
            .to_string()
            .contains("exact mode 0600")
    );
    fs::set_permissions(&installed.target, fs::Permissions::from_mode(0o600))?;
    let extra = temp.path().join("extra-link");
    fs::hard_link(&installed.target, &extra)?;
    assert!(
        verify_fds_firmware_for_test(scope(&data, &game), &home, &allowed, uid())
            .unwrap_err()
            .to_string()
            .contains("exactly one hard link")
    );
    fs::remove_file(extra)?;
    fs::write(&installed.target, test_bios(45))?;
    let error =
        verify_fds_firmware_for_test(scope(&data, &game), &home, &allowed, uid()).unwrap_err();
    assert!(format!("{error:#}").contains("unrecognized FDS BIOS SHA-256"));
    fs::remove_file(&installed.target)?;
    symlink(&source, &installed.target)?;
    let error =
        verify_fds_firmware_for_test(scope(&data, &game), &home, &allowed, uid()).unwrap_err();
    assert!(format!("{error:#}").contains("regular, non-symlink"));
    Ok(())
}

#[test]
fn effective_uid_is_required_for_owned_tree_and_target() -> Result<()> {
    let temp = TempDir::new()?;
    let (home, data) = flatpak_paths(&temp);
    let bytes = test_bios(55);
    let allowed = allowlist(&bytes);
    let source = write_file(&temp, "source.rom", &bytes);
    let game = content(&temp, "game.fds");
    let wrong_uid = uid().checked_add(1).unwrap_or_else(|| uid() - 1);
    assert!(
        install_fds_firmware_for_test(
            request(&data, &source, &game),
            &home,
            &allowed,
            wrong_uid,
            |_| {},
        )
        .unwrap_err()
        .to_string()
        .contains("effective UID")
    );
    let installed = install(&home, &data, &source, &game, &allowed)?;
    assert!(
        verify_fds_firmware_for_test(scope(&data, &game), &home, &allowed, wrong_uid)
            .unwrap_err()
            .to_string()
            .contains("effective UID")
    );
    assert_eq!(fs::read(installed.target)?, bytes);
    Ok(())
}

#[test]
fn existing_different_unrecognized_or_symlink_targets_are_never_replaced() -> Result<()> {
    let temp = TempDir::new()?;
    let (home, data) = flatpak_paths(&temp);
    let dir = data.join("nestopia");
    fs::create_dir(&dir)?;
    let target = dir.join(FDS_BIOS_FILENAME);
    let first = test_bios(61);
    let second = test_bios(62);
    let allowed = vec![sha256(&first), sha256(&second)];
    fs::write(&target, first)?;
    fs::set_permissions(&target, fs::Permissions::from_mode(0o600))?;
    let source = write_file(&temp, "source.rom", &second);
    let game = content(&temp, "game.fds");
    assert!(
        install(&home, &data, &source, &game, &allowed)
            .unwrap_err()
            .to_string()
            .contains("refusing to replace different")
    );
    assert_eq!(fs::read(&target)?, first);

    fs::remove_file(&target)?;
    fs::write(&target, b"unrecognized user file")?;
    assert!(
        install(&home, &data, &source, &game, &allowed)
            .unwrap_err()
            .to_string()
            .contains("refusing to replace existing")
    );
    assert_eq!(fs::read(&target)?, b"unrecognized user file");

    fs::remove_file(&target)?;
    let outside = temp.path().join("outside");
    fs::write(&outside, first)?;
    symlink(&outside, &target)?;
    assert!(
        install(&home, &data, &source, &game, &allowed)
            .unwrap_err()
            .to_string()
            .contains("refusing to replace existing")
    );
    assert_eq!(fs::read_link(&target)?, outside);
    assert_eq!(fs::read(&outside)?, first);

    fs::remove_file(&target)?;
    fs::write(&target, second)?;
    fs::set_permissions(&target, fs::Permissions::from_mode(0o644))?;
    assert!(
        install(&home, &data, &source, &game, &allowed)
            .unwrap_err()
            .to_string()
            .contains("exact mode 0600")
    );
    assert_eq!(fs::metadata(&target)?.permissions().mode() & 0o7777, 0o644);
    Ok(())
}

#[test]
fn concurrent_target_creation_is_atomic_and_never_replaced() -> Result<()> {
    let temp = TempDir::new()?;
    let (home, data) = flatpak_paths(&temp);
    let requested = test_bios(71);
    let concurrent = test_bios(72);
    let allowed = vec![sha256(&requested), sha256(&concurrent)];
    let source = write_file(&temp, "source.rom", &requested);
    let game = content(&temp, "game.fds");
    let target = data.join("nestopia/disksys.rom");
    let error = install_fds_firmware_for_test(
        request(&data, &source, &game),
        &home,
        &allowed,
        uid(),
        |phase| {
            if phase == InstallPhase::BeforeLink {
                fs::write(&target, concurrent).unwrap();
                fs::set_permissions(&target, fs::Permissions::from_mode(0o600)).unwrap();
            }
        },
    )
    .unwrap_err();
    assert!(error.to_string().contains("refusing to replace different"));
    assert_eq!(fs::read(&target)?, concurrent);
    assert_eq!(fs::read_dir(data.join("nestopia"))?.count(), 1);
    Ok(())
}

#[test]
fn concurrent_temporary_name_exchange_is_detected_and_rolled_back() -> Result<()> {
    let temp = TempDir::new()?;
    let (home, data) = flatpak_paths(&temp);
    let bytes = test_bios(76);
    let allowed = allowlist(&bytes);
    let source = write_file(&temp, "source.rom", &bytes);
    let game = content(&temp, "game.fds");
    let outside = temp.path().join("outside");
    fs::write(&outside, b"outside remains unchanged")?;
    let target_dir = data.join("nestopia");
    let error = install_fds_firmware_for_test(
        request(&data, &source, &game),
        &home,
        &allowed,
        uid(),
        |phase| {
            if phase == InstallPhase::BeforeLink {
                let temporary = fs::read_dir(&target_dir)
                    .unwrap()
                    .map(|entry| entry.unwrap().path())
                    .find(|path| {
                        path.file_name()
                            .unwrap()
                            .to_string_lossy()
                            .starts_with(".disksys.rom.lunchbox-")
                    })
                    .unwrap();
                fs::remove_file(&temporary).unwrap();
                symlink(&outside, temporary).unwrap();
            }
        },
    )
    .unwrap_err();
    assert!(format!("{error:#}").contains("regular, non-symlink"));
    assert!(!target_dir.join(FDS_BIOS_FILENAME).exists());
    assert_eq!(fs::read(&outside)?, b"outside remains unchanged");
    assert_eq!(fs::read_dir(&target_dir)?.count(), 0);
    Ok(())
}

#[test]
fn directory_swap_fails_closed_and_rolls_back_via_original_dirfd() -> Result<()> {
    let temp = TempDir::new()?;
    let (home, data) = flatpak_paths(&temp);
    let bytes = test_bios(81);
    let allowed = allowlist(&bytes);
    let source = write_file(&temp, "source.rom", &bytes);
    let game = content(&temp, "game.fds");
    let original = data.join("nestopia");
    let moved = data.join("nestopia-moved");
    let replacement = temp.path().join("replacement-dir");
    fs::create_dir(&replacement)?;
    let error = install_fds_firmware_for_test(
        request(&data, &source, &game),
        &home,
        &allowed,
        uid(),
        |phase| {
            if phase == InstallPhase::BeforeLink {
                fs::rename(&original, &moved).unwrap();
                symlink(&replacement, &original).unwrap();
            }
        },
    )
    .unwrap_err();
    assert!(error.to_string().contains("directory identity"));
    assert!(!moved.join(FDS_BIOS_FILENAME).exists());
    assert!(!replacement.join(FDS_BIOS_FILENAME).exists());
    assert_eq!(fs::read_dir(&moved)?.count(), 0);
    Ok(())
}

#[test]
fn rollback_preserves_attackers_post_link_replacement() -> Result<()> {
    let temp = TempDir::new()?;
    let (home, data) = flatpak_paths(&temp);
    let bytes = test_bios(91);
    let replacement = b"attacker replacement";
    let allowed = allowlist(&bytes);
    let source = write_file(&temp, "source.rom", &bytes);
    let game = content(&temp, "game.fds");
    let target = data.join("nestopia/disksys.rom");
    let error = install_fds_firmware_for_test(
        request(&data, &source, &game),
        &home,
        &allowed,
        uid(),
        |phase| {
            if phase == InstallPhase::AfterLink {
                fs::remove_file(&target).unwrap();
                fs::write(&target, replacement).unwrap();
            }
        },
    )
    .unwrap_err();
    assert!(error.to_string().contains("rechecking installed FDS BIOS"));
    assert_eq!(fs::read(&target)?, replacement);
    assert_eq!(fs::read_dir(data.join("nestopia"))?.count(), 1);
    Ok(())
}

#[test]
fn exact_app_platform_content_and_data_home_scope_is_mandatory() -> Result<()> {
    let temp = TempDir::new()?;
    let (home, data) = flatpak_paths(&temp);
    let bytes = test_bios(101);
    let allowed = allowlist(&bytes);
    let source = write_file(&temp, "source.rom", &bytes);
    let game = content(&temp, "game.fds");

    let mut wrong = request(&data, &source, &game);
    wrong.flatpak_app_id = "org.libretro.RetroArch";
    assert!(
        install_fds_firmware_for_test(wrong, &home, &allowed, uid(), |_| {})
            .unwrap_err()
            .to_string()
            .contains("exact ca._0ldsk00l.Nestopia")
    );
    let mut wrong = request(&data, &source, &game);
    wrong.platform_id = "famicom-disk-system";
    assert!(
        install_fds_firmware_for_test(wrong, &home, &allowed, uid(), |_| {})
            .unwrap_err()
            .to_string()
            .contains(FDS_PLATFORM_ID)
    );
    let mut wrong = request(&data, &source, &game);
    wrong.platform_name = "Famicom Disk System";
    assert!(
        install_fds_firmware_for_test(wrong, &home, &allowed, uid(), |_| {})
            .unwrap_err()
            .to_string()
            .contains(FDS_PLATFORM_NAME)
    );
    let mixed = content(&temp, "game.FdS");
    assert!(
        install(&home, &data, &source, &mixed, &allowed)
            .unwrap_err()
            .to_string()
            .contains("exact .fds or .FDS")
    );
    let lookalike = temp
        .path()
        .join("other/.var/app")
        .join(NESTOPIA_UE_FLATPAK_ID)
        .join("data");
    fs::create_dir_all(&lookalike)?;
    assert!(
        install(&home, &lookalike, &source, &game, &allowed)
            .unwrap_err()
            .to_string()
            .contains("current user's exact")
    );
    assert!(!data.join("nestopia").exists());
    assert!(RUNTIME_PROOF_BLOCKER.contains("lawfully obtained"));
    Ok(())
}

#[test]
fn rejects_unsafe_content_paths_and_symlinked_target_directory() -> Result<()> {
    let temp = TempDir::new()?;
    let (home, data) = flatpak_paths(&temp);
    let bytes = test_bios(111);
    let allowed = allowlist(&bytes);
    let source = write_file(&temp, "source.rom", &bytes);
    assert!(
        install(&home, &data, &source, Path::new("relative.fds"), &allowed)
            .unwrap_err()
            .to_string()
            .contains("must be absolute")
    );
    let real_content = content(&temp, "real.fds");
    let linked_content = temp.path().join("linked.fds");
    symlink(&real_content, &linked_content)?;
    assert!(
        install(&home, &data, &source, &linked_content, &allowed)
            .unwrap_err()
            .to_string()
            .contains("regular, non-symlink")
    );

    let outside = temp.path().join("outside-nestopia");
    fs::create_dir(&outside)?;
    symlink(&outside, data.join("nestopia"))?;
    assert!(
        install(&home, &data, &source, &real_content, &allowed)
            .unwrap_err()
            .to_string()
            .contains("without following symlinks")
    );
    assert!(!outside.join(FDS_BIOS_FILENAME).exists());

    let linked_temp = TempDir::new()?;
    let linked_home = linked_temp.path().join("home");
    let linked_data = linked_home
        .join(".var/app")
        .join(NESTOPIA_UE_FLATPAK_ID)
        .join("data");
    fs::create_dir_all(linked_data.parent().unwrap())?;
    let actual_data = linked_temp.path().join("actual-data");
    fs::create_dir(&actual_data)?;
    symlink(&actual_data, &linked_data)?;
    let linked_home = fs::canonicalize(linked_home)?;
    let linked_game = content(&linked_temp, "game.fds");
    assert!(
        install(&linked_home, &linked_data, &source, &linked_game, &allowed)
            .unwrap_err()
            .to_string()
            .contains("opening Flatpak data home")
    );
    assert!(!actual_data.join("nestopia").exists());
    Ok(())
}
