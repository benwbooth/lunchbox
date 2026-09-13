//! Opt-in, noninteractive native macOS RetroArch frontend oracle.
//!
//! This runs the exact production `frontend_autoconfig` helper against the
//! audited RetroArch/Nestopia identities. It clones the supplied app into a
//! private portable installation, records and rewrites a native BSV2 replay,
//! verifies that the frontend applies the private core-options snapshot, and
//! exercises fresh-process SRAM and save-state restoration. It never treats
//! simulated replay input as physical-controller evidence.

#![cfg(target_os = "macos")]

use std::collections::{BTreeMap, BTreeSet};
use std::ffi::{OsStr, OsString};
use std::fs::{self, File};
use std::io::{ErrorKind, Read, Seek, SeekFrom, Write};
use std::net::{SocketAddrV4, UdpSocket};
use std::os::unix::fs::{PermissionsExt, symlink};
use std::path::{Path, PathBuf};
use std::process::{Child, Command, ExitStatus, Output, Stdio};
use std::thread;
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

use anyhow::{Context, Result, bail, ensure};
use lunchbox_controller_probe::retroarch_frontend_autoconfig::{
    FrontendAutoconfigProfileSpec, FrontendAutoconfigSession, MACOS_NESTOPIA_SHA256,
    MACOS_RETROARCH_1_22_2_SHA256, MacOsInstallDisposition, NESTOPIA_NES_FOUR_PLAYER_PROFILE,
    NESTOPIA_NES_TWO_PLAYER_PROFILE, NativeRetroArchPathRequest, ensure_no_physical_mapping_keys,
    prepare_pinned_frontend_autoconfig_session,
};
use lunchbox_controller_probe::{libretro_input, libretro_persistence};
use serde::Serialize;
use sha2::{Digest, Sha256};

const INPUT_ROM_SHA256: &str = "0aa0053c7932bf5ba3db562229310f01e9e0b4697db8389e515a37d599e95f19";
const PERSISTENCE_ROM_SHA256: &str =
    "0ae807dfe80d7178f082e0d6297e94b145d2880ff5a974fa5c1b26f6bf1beaaa";
const BSV2_MAGIC: u32 = 0x4253_5632;
const BSV2_VERSION: u32 = 2;
const BSV_HEADER_BYTES: usize = 40;
const BSV_REGULAR_FRAME: u8 = b'f';
const LIBRETRO_JOYPAD_DEVICE: u8 = 1;
const LIBRETRO_JOYPAD_MASK_ID: u16 = 256;
const LIBRETRO_A_MASK: i16 = 1 << 8;
const LIBRETRO_B_MASK: i16 = 1;
const LIBRETRO_START_MASK: i16 = 1 << 3;
const LIBRETRO_RIGHT_MASK: i16 = 1 << 7;
const TWO_PLAYER_SYNTHETIC_FRAMES: usize = 1200;
const P1_A_FIRST_FRAME: usize = 240;
const P1_A_LAST_FRAME_EXCLUSIVE: usize = 480;
const P2_B_FIRST_FRAME: usize = 480;
const P2_B_LAST_FRAME_EXCLUSIVE: usize = 720;
const FOUR_PLAYER_SYNTHETIC_FRAMES: usize = 2400;
const FOUR_P1_FIRST_FRAME: usize = 240;
const FOUR_P1_LAST_FRAME_EXCLUSIVE: usize = 480;
const FOUR_P2_FIRST_FRAME: usize = 480;
const FOUR_P2_LAST_FRAME_EXCLUSIVE: usize = 720;
const FOUR_P3_FIRST_FRAME: usize = 720;
const FOUR_P3_LAST_FRAME_EXCLUSIVE: usize = 960;
const FOUR_P4_FIRST_FRAME: usize = 960;
const FOUR_P4_LAST_FRAME_EXCLUSIVE: usize = 1800;

#[derive(Serialize)]
struct FileEvidence {
    path: String,
    bytes: u64,
    sha256: String,
}

#[derive(Clone, Serialize)]
struct LaunchEvidence {
    purpose: String,
    program: String,
    argv: Vec<String>,
    effective_main_config: String,
    effective_source_core_options: String,
    generated_append_config: String,
    generated_core_options: String,
    exit_code: Option<i32>,
    forced_termination: bool,
}

#[derive(Serialize)]
struct InputEvidence {
    virtual_input_driver: &'static str,
    bsv_fallback_reason: &'static str,
    bsv_template: FileEvidence,
    bsv_simulated: FileEvidence,
    source_frames: usize,
    output_frames: usize,
    p1_a_frames: [usize; 2],
    p2_b_frames: [usize; 2],
    observed_pairs: Vec<[u8; 2]>,
    marker: String,
    semantic_assertion: String,
}

#[derive(Serialize)]
struct FourPlayerInputEvidence {
    virtual_input_driver: &'static str,
    adapter_option: &'static str,
    button_shift_option: &'static str,
    bsv_template: FileEvidence,
    bsv_simulated: FileEvidence,
    source_frames: usize,
    output_frames: usize,
    p1_a_frames: [usize; 2],
    p2_b_frames: [usize; 2],
    p3_start_frames: [usize; 2],
    p4_right_frames: [usize; 2],
    observed_channels: Vec<[u8; 4]>,
    marker: String,
    semantic_assertion: String,
}

#[derive(Serialize)]
struct StateEvidence {
    file: FileEvidence,
    saved_bytes: String,
    mutated_bytes: String,
    same_process_restored_bytes: String,
    fresh_process_before_load: String,
    fresh_process_restored_bytes: String,
}

#[derive(Serialize)]
struct SramEvidence {
    file: FileEvidence,
    generation_one_sha256: String,
    generation_two_sha256: String,
    generation_one_prefix: String,
    generation_two_prefix: String,
}

#[derive(Serialize)]
struct OracleReport {
    schema_version: u32,
    result: &'static str,
    unix_time_seconds: u64,
    host_os: &'static str,
    frontend: FileEvidence,
    core: FileEvidence,
    input_rom: FileEvidence,
    persistence_rom: FileEvidence,
    profile_id: String,
    source_main_config_sha256_before: String,
    source_main_config_sha256_after: String,
    source_core_options_sha256_before: String,
    source_core_options_sha256_after: String,
    source_core_options_seed: String,
    four_player_profile_id: String,
    four_player_source_core_options_sha256_before: String,
    four_player_source_core_options_sha256_after: String,
    four_player_source_core_options_seed: String,
    macos_bundle_seal_verified: bool,
    sealed_resource_mutation_rejected: bool,
    late_launch_environment_drift_rejected: bool,
    launches: Vec<LaunchEvidence>,
    input: InputEvidence,
    four_player_input: FourPlayerInputEvidence,
    state: StateEvidence,
    sram: SramEvidence,
    physical_controller: PhysicalControllerDisposition,
    user_path_snapshots_unchanged: bool,
    isolated_run_root_removed: bool,
    orphan_relaunch_invariant_verified: bool,
    owned_processes_remaining: usize,
}

#[derive(Serialize)]
struct PhysicalControllerDisposition {
    status: &'static str,
    reason: &'static str,
}

#[derive(Clone, Debug, Eq, PartialEq)]
struct ExternalPathSnapshot {
    path: PathBuf,
    kind: &'static str,
    len: u64,
    modified: Option<SystemTime>,
    mode: u32,
    sha256: Option<String>,
}

struct CommandClient {
    socket: UdpSocket,
    target: SocketAddrV4,
    bundle: PathBuf,
}

struct RunningFrontend {
    child: Option<Child>,
    session: FrontendAutoconfigSession,
    client: CommandClient,
    evidence: LaunchEvidence,
    program: PathBuf,
    core: PathBuf,
    content: PathBuf,
    source_main: Vec<u8>,
    source_options: Vec<u8>,
}

#[derive(Clone)]
struct OracleInputs {
    app: PathBuf,
    core: PathBuf,
    report: PathBuf,
    home: PathBuf,
}

struct IsolatedBundleGuard {
    executable_aliases: BTreeSet<String>,
}

impl IsolatedBundleGuard {
    fn new(executable: &Path) -> Result<Self> {
        let mut executable_aliases = BTreeSet::from([unicode(executable)?.to_owned()]);
        executable_aliases.insert(unicode(&fs::canonicalize(executable)?)?.to_owned());
        Ok(Self { executable_aliases })
    }

    fn process_ids(&self) -> Result<Vec<u32>> {
        let mut command = Command::new("/bin/ps");
        command.args(["-axo", "pid=,command="]);
        let output = command_output_bounded(
            &mut command,
            Duration::from_secs(3),
            "listing processes for the isolated RetroArch bundle",
        )?;
        ensure!(
            output.status.success(),
            "ps failed while checking owned children"
        );
        let text = std::str::from_utf8(&output.stdout)?;
        let mut result = Vec::new();
        for line in text.lines() {
            let mut fields = line.split_ascii_whitespace();
            let Some(pid) = fields.next() else { continue };
            let Some(program) = fields.next() else {
                continue;
            };
            if self.executable_aliases.contains(program) {
                result.push(pid.parse().context("invalid PID from ps")?);
            }
        }
        Ok(result)
    }

    fn terminate_all(&self) {
        let terminate = |signal: &str, pids: Vec<u32>| {
            for pid in pids {
                let mut command = Command::new("/bin/kill");
                command.args([signal, &pid.to_string()]);
                let _ = command_output_bounded(
                    &mut command,
                    Duration::from_secs(1),
                    "terminating an exact copied-bundle RetroArch process",
                );
            }
        };
        if let Ok(pids) = self.process_ids() {
            terminate("-TERM", pids);
            thread::sleep(Duration::from_millis(250));
        }
        if let Ok(pids) = self.process_ids() {
            terminate("-KILL", pids);
        }
    }
}

impl Drop for IsolatedBundleGuard {
    fn drop(&mut self) {
        self.terminate_all();
    }
}

#[test]
#[ignore = "requires macOS plus exact LUNCHBOX_MACOS_RETROARCH_APP, LUNCHBOX_MACOS_NESTOPIA_CORE, and LUNCHBOX_RETROARCH_ORACLE_REPORT paths"]
fn production_frontend_autoconfig_macos_oracle() {
    let report_path = std::env::var_os("LUNCHBOX_RETROARCH_ORACLE_REPORT").map(PathBuf::from);
    let outcome = load_inputs().and_then(run_oracle);
    match outcome {
        Ok(report) => {
            let report_path = report_path.expect("validated report path disappeared");
            write_json_report(&report_path, &report).expect("writing passing oracle report");
        }
        Err(error) => {
            if let Some(report_path) = report_path {
                let failure = serde_json::json!({
                    "schema_version": 1,
                    "result": "fail",
                    "host_os": "macos",
                    "error": format!("{error:#}"),
                    "physical_controller": {
                        "status": "not_tested",
                        "reason": "this oracle uses only deterministic BSV replay input"
                    }
                });
                let _ = write_json_report(&report_path, &failure);
            }
            panic!("native macOS RetroArch frontend oracle failed: {error:#}");
        }
    }
}

fn load_inputs() -> Result<OracleInputs> {
    let required = |name: &str| -> Result<PathBuf> {
        let path =
            PathBuf::from(std::env::var_os(name).with_context(|| format!("{name} is required"))?);
        ensure!(path.is_absolute(), "{name} must be absolute");
        Ok(path)
    };
    let inputs = OracleInputs {
        app: required("LUNCHBOX_MACOS_RETROARCH_APP")?,
        core: required("LUNCHBOX_MACOS_NESTOPIA_CORE")?,
        report: required("LUNCHBOX_RETROARCH_ORACLE_REPORT")?,
        home: PathBuf::from(
            std::env::var_os("HOME").context("the exact inherited HOME is required")?,
        ),
    };
    ensure!(inputs.home.is_absolute(), "HOME must be absolute");
    ensure!(inputs.app.extension() == Some(OsStr::new("app")) && inputs.app.is_dir());
    ensure!(
        inputs.core.is_file(),
        "required artifact is not a file: {}",
        inputs.core.display()
    );
    ensure!(
        inputs.report.parent().is_some_and(|parent| parent.is_dir()),
        "report parent must already exist"
    );
    Ok(inputs)
}

fn run_oracle(inputs: OracleInputs) -> Result<OracleReport> {
    let source_executable = inputs.app.join("Contents/MacOS/RetroArch");
    assert_sha256(&source_executable, MACOS_RETROARCH_1_22_2_SHA256)?;
    assert_sha256(&inputs.core, MACOS_NESTOPIA_SHA256)?;
    ensure!(
        !file_contains(&source_executable, b"test_input_file_joypad")?,
        "audited frontend unexpectedly exposes the test joypad driver; replace the BSV fallback with the native test driver"
    );

    let external_paths = external_retroarch_paths(&inputs.home);
    let external_before = external_paths
        .iter()
        .map(|path| ExternalPathSnapshot::capture(path))
        .collect::<Result<Vec<_>>>()?;

    let temporary = tempfile::Builder::new()
        .prefix("lunchbox-retroarch-macos-oracle-")
        .tempdir()
        .context("creating isolated macOS oracle root")?;
    let root = temporary.path().to_path_buf();
    ensure!(
        !inputs.report.starts_with(&root),
        "report must be outside the disposable oracle root"
    );
    let bundle = root.join("RetroArch.app");
    copy_tree(&inputs.app, &bundle)?;
    let executable = bundle.join("Contents/MacOS/RetroArch");
    assert_sha256(&executable, MACOS_RETROARCH_1_22_2_SHA256)?;
    let bundle_guard = IsolatedBundleGuard::new(&executable)?;

    let payload = root.join("payload");
    let logs = root.join("logs");
    let replays = root.join("replays");
    let saves = root.join("saves");
    let states = root.join("states");
    let cache = root.join("cache/controller-launch");
    for directory in [&payload, &logs, &replays, &saves, &states, &cache] {
        fs::create_dir_all(directory)?;
    }
    let core = payload.join("nestopia_libretro.dylib");
    let input_rom = payload.join("controller-diagnostic.nes");
    let persistence_rom = payload.join("persistence-diagnostic.nes");
    fs::copy(&inputs.core, &core)?;
    fs::write(&input_rom, libretro_input::nes_diagnostic_rom())?;
    fs::write(
        &persistence_rom,
        libretro_persistence::nes_persistence_rom(),
    )?;
    assert_sha256(&core, MACOS_NESTOPIA_SHA256)?;
    assert_sha256(&input_rom, INPUT_ROM_SHA256)?;
    assert_sha256(&persistence_rom, PERSISTENCE_ROM_SHA256)?;

    fs::write(
        root.join("portable.txt"),
        b"Lunchbox isolated native oracle\n",
    )?;
    let config_directory = root.join("config");
    fs::create_dir_all(&config_directory)?;
    let source_options = config_directory.join("source-core-options.cfg");
    let source_options_bytes =
        b"oracle_preserved_option = \"sentinel\"\nnestopia_button_shift = \"enabled\"\n";
    fs::write(&source_options, source_options_bytes)?;
    let four_player_source_options = config_directory.join("four-player-source-core-options.cfg");
    let four_player_source_options_bytes = b"oracle_preserved_option = \"four-player-sentinel\"\n\
nestopia_button_shift = \"enabled\"\n\
nestopia_select_adapter = \"ntsc\"\n";
    fs::write(
        &four_player_source_options,
        four_player_source_options_bytes,
    )?;
    let port = reserve_udp_port()?;
    let main_config = config_directory.join("retroarch.cfg");
    let main_config_text = isolated_main_config(&root, &source_options, port)?;
    fs::write(&main_config, &main_config_text)?;
    let main_before = sha256_bytes(main_config_text.as_bytes());
    let options_before = sha256_bytes(source_options_bytes);
    let four_player_options_before = sha256_bytes(four_player_source_options_bytes);

    let profile = bundled_nestopia_profile(NESTOPIA_NES_TWO_PLAYER_PROFILE)?;
    ensure!(profile.id == NESTOPIA_NES_TWO_PLAYER_PROFILE);
    let four_player_profile = bundled_nestopia_profile(NESTOPIA_NES_FOUR_PLAYER_PROFILE)?;
    ensure!(four_player_profile.id == NESTOPIA_NES_FOUR_PLAYER_PROFILE);
    let request = NativeRetroArchPathRequest::MacOs {
        executable: executable.clone(),
        application_bundle: bundle.clone(),
        home: inputs.home.clone(),
        application_support: inputs.home.join("Library/Application Support"),
        install: MacOsInstallDisposition::Ambiguous,
    };

    let (sealed_resource_mutation_rejected, late_launch_environment_drift_rejected) =
        verify_sealed_resource_mutation_rejected(
            &request, &profile, &core, &input_rom, &cache, &bundle,
        )?;

    let mut launches = Vec::new();
    let template_path = replays.join("native-baseline.bsv");
    let mut recording = start_frontend(
        "record-native-bsv-template",
        &request,
        &profile,
        &core,
        &input_rom,
        &[format!("--record-replay={}", unicode(&template_path)?)],
        &cache,
        &logs.join("record-native-bsv.log"),
        port,
        &root,
    )?;
    recording
        .client
        .wait_core_ram(&mut recording.child, 0, 7, |bytes| {
            bytes.len() == 7 && &bytes[1..4] == b"LBN"
        })?;
    pump_bundle_events(&bundle, Duration::from_millis(1600))?;
    recording
        .client
        .action(&mut recording.child, "HALT_REPLAY")?;
    wait_for_file_stable(&template_path, Duration::from_secs(5))?;
    launches.push(recording.finish()?);

    let simulated_path = replays.join("simulated-p1-a.bsv");
    let synthesis = synthesize_replay(
        &template_path,
        &simulated_path,
        2,
        TWO_PLAYER_SYNTHETIC_FRAMES,
        &[
            (
                P1_A_FIRST_FRAME,
                P1_A_LAST_FRAME_EXCLUSIVE,
                0,
                LIBRETRO_A_MASK,
            ),
            (
                P2_B_FIRST_FRAME,
                P2_B_LAST_FRAME_EXCLUSIVE,
                1,
                LIBRETRO_B_MASK,
            ),
        ],
    )?;
    let mut playback = start_frontend(
        "play-simulated-p1-a",
        &request,
        &profile,
        &core,
        &input_rom,
        &[format!("--play-replay={}", unicode(&simulated_path)?)],
        &cache,
        &logs.join("play-simulated-p1-a.log"),
        port,
        &root,
    )?;
    let input_observation = observe_simulated_input(&mut playback)?;
    launches.push(playback.finish()?);

    let four_player_main_config_text =
        isolated_main_config(&root, &four_player_source_options, port)?;
    fs::write(&main_config, &four_player_main_config_text)?;
    let four_player_template_path = replays.join("native-four-score-baseline.bsv");
    let mut four_player_recording = start_frontend(
        "record-native-four-score-bsv-template",
        &request,
        &four_player_profile,
        &core,
        &input_rom,
        &[format!(
            "--record-replay={}",
            unicode(&four_player_template_path)?
        )],
        &cache,
        &logs.join("record-native-four-score-bsv.log"),
        port,
        &root,
    )?;
    let four_player_baseline = four_player_recording.client.wait_core_ram(
        &mut four_player_recording.child,
        0,
        7,
        |bytes| bytes.len() == 7 && &bytes[1..4] == b"LBN",
    )?;
    ensure!(
        [
            four_player_baseline[0],
            four_player_baseline[4],
            four_player_baseline[5],
            four_player_baseline[6],
        ] == [0xFF; 4],
        "released Four Score baseline was not FF on all four encoded channels: {four_player_baseline:02X?}"
    );
    pump_bundle_events(&bundle, Duration::from_millis(1600))?;
    four_player_recording
        .client
        .action(&mut four_player_recording.child, "HALT_REPLAY")?;
    wait_for_file_stable(&four_player_template_path, Duration::from_secs(5))?;
    launches.push(four_player_recording.finish()?);

    let four_player_simulated_path = replays.join("simulated-four-score-four-player.bsv");
    let four_player_synthesis = synthesize_replay(
        &four_player_template_path,
        &four_player_simulated_path,
        4,
        FOUR_PLAYER_SYNTHETIC_FRAMES,
        &[
            (
                FOUR_P1_FIRST_FRAME,
                FOUR_P1_LAST_FRAME_EXCLUSIVE,
                0,
                LIBRETRO_A_MASK,
            ),
            (
                FOUR_P2_FIRST_FRAME,
                FOUR_P2_LAST_FRAME_EXCLUSIVE,
                1,
                LIBRETRO_B_MASK,
            ),
            (
                FOUR_P3_FIRST_FRAME,
                FOUR_P3_LAST_FRAME_EXCLUSIVE,
                2,
                LIBRETRO_START_MASK,
            ),
            (
                FOUR_P4_FIRST_FRAME,
                FOUR_P4_LAST_FRAME_EXCLUSIVE,
                3,
                LIBRETRO_RIGHT_MASK,
            ),
        ],
    )?;
    let mut four_player_playback = start_frontend(
        "play-simulated-four-score-four-player-input",
        &request,
        &four_player_profile,
        &core,
        &input_rom,
        &[format!(
            "--play-replay={}",
            unicode(&four_player_simulated_path)?
        )],
        &cache,
        &logs.join("play-simulated-four-score-four-player.log"),
        port,
        &root,
    )?;
    let four_player_observation = observe_four_player_input(&mut four_player_playback)?;
    launches.push(four_player_playback.finish()?);
    fs::write(&main_config, &main_config_text)?;

    let state_file = states.join("controller-diagnostic.state");
    let saved = b"LBSTATE1";
    let mutated = b"MUTATED!";
    let mut state_same = start_frontend(
        "state-same-process",
        &request,
        &profile,
        &core,
        &input_rom,
        &[],
        &cache,
        &logs.join("state-same-process.log"),
        port,
        &root,
    )?;
    state_same
        .client
        .wait_core_ram(&mut state_same.child, 0, 7, |bytes| &bytes[1..4] == b"LBN")?;
    state_same
        .client
        .write_core_ram(&mut state_same.child, 0x100, saved)?;
    let saved_readback =
        state_same
            .client
            .wait_core_ram(&mut state_same.child, 0x100, saved.len(), |bytes| {
                bytes == saved
            })?;
    state_same
        .client
        .action(&mut state_same.child, "SAVE_STATE")?;
    wait_for_file_stable(&state_file, Duration::from_secs(8))?;
    state_same
        .client
        .write_core_ram(&mut state_same.child, 0x100, mutated)?;
    let mutated_readback =
        state_same
            .client
            .wait_core_ram(&mut state_same.child, 0x100, mutated.len(), |bytes| {
                bytes == mutated
            })?;
    state_same
        .client
        .action(&mut state_same.child, "LOAD_STATE")?;
    let same_process_restored =
        state_same
            .client
            .wait_core_ram(&mut state_same.child, 0x100, saved.len(), |bytes| {
                bytes == saved
            })?;
    launches.push(state_same.finish()?);

    let mut state_fresh = start_frontend(
        "state-fresh-process",
        &request,
        &profile,
        &core,
        &input_rom,
        &[],
        &cache,
        &logs.join("state-fresh-process.log"),
        port,
        &root,
    )?;
    state_fresh
        .client
        .wait_core_ram(&mut state_fresh.child, 0, 7, |bytes| &bytes[1..4] == b"LBN")?;
    let fresh_before =
        state_fresh
            .client
            .wait_core_ram(&mut state_fresh.child, 0x100, saved.len(), |bytes| {
                bytes.iter().all(|byte| *byte == 0)
            })?;
    state_fresh
        .client
        .action(&mut state_fresh.child, "LOAD_STATE")?;
    let fresh_restored =
        state_fresh
            .client
            .wait_core_ram(&mut state_fresh.child, 0x100, saved.len(), |bytes| {
                bytes == saved
            })?;
    launches.push(state_fresh.finish()?);

    let save_file = saves.join("persistence-diagnostic.srm");
    let mut save_one = start_frontend(
        "sram-generation-one",
        &request,
        &profile,
        &core,
        &persistence_rom,
        &[],
        &cache,
        &logs.join("sram-generation-one.log"),
        port,
        &root,
    )?;
    save_one
        .client
        .wait_core_ram(&mut save_one.child, 0, 5, |bytes| bytes == b"LBSR\x01")?;
    save_one.client.action(&mut save_one.child, "SAVE_FILES")?;
    let generation_one =
        wait_for_file_prefix(&save_file, b"LBSR\x01", 8192, Duration::from_secs(8))?;
    let generation_one_sha256 = sha256_bytes(&generation_one);
    launches.push(save_one.finish()?);

    let mut save_two = start_frontend(
        "sram-fresh-process-generation-two",
        &request,
        &profile,
        &core,
        &persistence_rom,
        &[],
        &cache,
        &logs.join("sram-generation-two.log"),
        port,
        &root,
    )?;
    save_two
        .client
        .wait_core_ram(&mut save_two.child, 0, 5, |bytes| bytes == b"LBSR\x02")?;
    save_two.client.action(&mut save_two.child, "SAVE_FILES")?;
    let generation_two =
        wait_for_file_prefix(&save_file, b"LBSR\x02", 8192, Duration::from_secs(8))?;
    let generation_two_sha256 = sha256_bytes(&generation_two);
    ensure!(
        generation_one_sha256 != generation_two_sha256,
        "SRAM generations unexpectedly have identical bytes"
    );
    launches.push(save_two.finish()?);

    let main_after_bytes = fs::read(&main_config)?;
    let options_after_bytes = fs::read(&source_options)?;
    let four_player_options_after_bytes = fs::read(&four_player_source_options)?;
    ensure!(main_after_bytes == main_config_text.as_bytes());
    ensure!(options_after_bytes == source_options_bytes);
    ensure!(four_player_options_after_bytes == four_player_source_options_bytes);
    let external_after = external_paths
        .iter()
        .map(|path| ExternalPathSnapshot::capture(path))
        .collect::<Result<Vec<_>>>()?;
    ensure!(
        external_before == external_after,
        "a standard RetroArch user path changed during the portable oracle"
    );

    let frontend_evidence = file_evidence(&executable)?;
    let core_evidence = file_evidence(&core)?;
    let input_rom_evidence = file_evidence(&input_rom)?;
    let persistence_rom_evidence = file_evidence(&persistence_rom)?;
    let input = InputEvidence {
        virtual_input_driver: "RetroArch BSV2 replay",
        bsv_fallback_reason: "the audited official macOS 1.22.2 Metal executable was built without RetroArch HAVE_TEST_DRIVERS/test_input_file_joypad support",
        bsv_template: file_evidence(&template_path)?,
        bsv_simulated: file_evidence(&simulated_path)?,
        source_frames: synthesis.source_frames,
        output_frames: TWO_PLAYER_SYNTHETIC_FRAMES,
        p1_a_frames: [P1_A_FIRST_FRAME, P1_A_LAST_FRAME_EXCLUSIVE - 1],
        p2_b_frames: [P2_B_FIRST_FRAME, P2_B_LAST_FRAME_EXCLUSIVE - 1],
        observed_pairs: input_observation.pairs,
        marker: "LBN".to_owned(),
        semantic_assertion: "P1 libretro A produced active-low NES A (FE) and P2 libretro B produced active-low NES B (FD), proving both configured RetroPad ports and the private nestopia_button_shift=disabled override were effective".to_owned(),
    };
    let four_player_input = FourPlayerInputEvidence {
        virtual_input_driver: "RetroArch BSV2 replay",
        adapter_option: "nestopia_select_adapter = \"ntsc\"",
        button_shift_option: "nestopia_button_shift = \"disabled\"",
        bsv_template: file_evidence(&four_player_template_path)?,
        bsv_simulated: file_evidence(&four_player_simulated_path)?,
        source_frames: four_player_synthesis.source_frames,
        output_frames: FOUR_PLAYER_SYNTHETIC_FRAMES,
        p1_a_frames: [FOUR_P1_FIRST_FRAME, FOUR_P1_LAST_FRAME_EXCLUSIVE - 1],
        p2_b_frames: [FOUR_P2_FIRST_FRAME, FOUR_P2_LAST_FRAME_EXCLUSIVE - 1],
        p3_start_frames: [FOUR_P3_FIRST_FRAME, FOUR_P3_LAST_FRAME_EXCLUSIVE - 1],
        p4_right_frames: [FOUR_P4_FIRST_FRAME, FOUR_P4_LAST_FRAME_EXCLUSIVE - 1],
        observed_channels: four_player_observation.channels,
        marker: "LBN".to_owned(),
        semantic_assertion: "With the exact four-player profile and preserved NTSC Four Score adapter option, P1 A, P2 B, P3 Start, and P4 Right independently produced active-low NES channels FE/FD/F7/7F across the first and second controller-port serial bytes".to_owned(),
    };
    let state = StateEvidence {
        file: file_evidence(&state_file)?,
        saved_bytes: hex::encode_upper(saved_readback),
        mutated_bytes: hex::encode_upper(mutated_readback),
        same_process_restored_bytes: hex::encode_upper(same_process_restored),
        fresh_process_before_load: hex::encode_upper(fresh_before),
        fresh_process_restored_bytes: hex::encode_upper(fresh_restored),
    };
    let sram = SramEvidence {
        file: file_evidence(&save_file)?,
        generation_one_sha256,
        generation_two_sha256,
        generation_one_prefix: hex::encode(&generation_one[..16]),
        generation_two_prefix: hex::encode(&generation_two[..16]),
    };
    let owned_processes_remaining = bundle_guard.process_ids()?.len();
    ensure!(
        owned_processes_remaining == 0,
        "an exact copied-bundle RetroArch process survived all owned child teardowns"
    );
    let mut report = OracleReport {
        schema_version: 1,
        result: "pass",
        unix_time_seconds: SystemTime::now().duration_since(UNIX_EPOCH)?.as_secs(),
        host_os: "macos",
        frontend: frontend_evidence,
        core: core_evidence,
        input_rom: input_rom_evidence,
        persistence_rom: persistence_rom_evidence,
        profile_id: profile.id,
        source_main_config_sha256_before: main_before,
        source_main_config_sha256_after: sha256_bytes(&main_after_bytes),
        source_core_options_sha256_before: options_before,
        source_core_options_sha256_after: sha256_bytes(&options_after_bytes),
        source_core_options_seed: String::from_utf8(source_options_bytes.to_vec())?,
        four_player_profile_id: four_player_profile.id,
        four_player_source_core_options_sha256_before: four_player_options_before,
        four_player_source_core_options_sha256_after: sha256_bytes(
            &four_player_options_after_bytes,
        ),
        four_player_source_core_options_seed: String::from_utf8(
            four_player_source_options_bytes.to_vec(),
        )?,
        macos_bundle_seal_verified: true,
        sealed_resource_mutation_rejected,
        late_launch_environment_drift_rejected,
        launches,
        input,
        four_player_input,
        state,
        sram,
        physical_controller: PhysicalControllerDisposition {
            status: "not_tested",
            reason: "the oracle uses a deterministic frontend BSV replay and makes no physical-controller compatibility or mapping claim",
        },
        user_path_snapshots_unchanged: true,
        isolated_run_root_removed: false,
        orphan_relaunch_invariant_verified: true,
        owned_processes_remaining,
    };
    temporary
        .close()
        .context("removing the isolated macOS oracle root")?;
    ensure!(!root.exists(), "isolated oracle root remains after cleanup");
    report.isolated_run_root_removed = true;
    Ok(report)
}

fn bundled_nestopia_profile(profile_id: &str) -> Result<FrontendAutoconfigProfileSpec> {
    let catalog: serde_json::Value = serde_json::from_str(include_str!(
        "../../lunchbox-app/data/controllers/catalog.json"
    ))?;
    let profile = catalog["emulator_profiles"]
        .as_array()
        .context("controller catalog has no emulator_profiles array")?
        .iter()
        .find(|profile| profile["id"] == profile_id)
        .with_context(|| format!("controller catalog has no Nestopia profile {profile_id}"))?;
    let launch = &profile["retroarch_launch"];
    let strings = |field: &str| -> Result<BTreeSet<String>> {
        profile[field]
            .as_array()
            .with_context(|| format!("profile has no {field} array"))?
            .iter()
            .map(|value| {
                value
                    .as_str()
                    .map(str::to_owned)
                    .with_context(|| format!("{field} contains a non-string"))
            })
            .collect::<Result<_>>()
    };
    let core_options = profile["core_options"]
        .as_object()
        .context("profile has no core_options object")?
        .iter()
        .map(|(key, value)| {
            Ok((
                key.clone(),
                value
                    .as_str()
                    .context("core option is not a string")?
                    .to_owned(),
            ))
        })
        .collect::<Result<BTreeMap<_, _>>>()?;
    Ok(FrontendAutoconfigProfileSpec {
        id: profile["id"].as_str().context("profile id")?.to_owned(),
        core: profile["core"].as_str().context("profile core")?.to_owned(),
        target_layout: profile["target_layout"]
            .as_str()
            .context("target layout")?
            .to_owned(),
        transport: profile["transport"]
            .as_str()
            .context("transport")?
            .to_owned(),
        retroarch_library: profile["retroarch_library"].as_str().map(str::to_owned),
        explicit_selection: profile["explicit_selection"].as_bool().unwrap_or(false),
        platforms: launch["platforms"]
            .as_array()
            .context("launch platforms")?
            .iter()
            .map(|value| {
                value
                    .as_str()
                    .map(str::to_owned)
                    .context("non-string platform")
            })
            .collect::<Result<_>>()?,
        content_extensions: strings("content_extensions")?,
        frontend_ports: profile["frontend_ports"]
            .as_u64()
            .context("frontend_ports")? as usize,
        max_players: launch["max_players"].as_u64().context("max_players")? as usize,
        default_device: launch["device"].as_u64().context("device")? as u32,
        port_devices: BTreeMap::new(),
        core_options,
        content_guard: profile["content_guard"].as_str().map(str::to_owned),
        requires_fresh_start: profile["requires_fresh_start"].as_bool().unwrap_or(false),
        has_player_topology: launch.get("player_topology").is_some(),
        dynamic_profile: false,
        special_preparation: false,
    })
}

fn verify_sealed_resource_mutation_rejected(
    request: &NativeRetroArchPathRequest,
    profile: &FrontendAutoconfigProfileSpec,
    core: &Path,
    content: &Path,
    cache: &Path,
    bundle: &Path,
) -> Result<(bool, bool)> {
    let arguments = vec![
        OsString::from("--verbose"),
        OsString::from("-L"),
        core.as_os_str().to_owned(),
        content.as_os_str().to_owned(),
    ];
    let (session, prepared) = prepare_pinned_frontend_autoconfig_session(
        request,
        core,
        MACOS_RETROARCH_1_22_2_SHA256,
        MACOS_NESTOPIA_SHA256,
        profile,
        content,
        &arguments,
        &[],
        cache,
    )?;
    let executable = match request {
        NativeRetroArchPathRequest::MacOs { executable, .. } => executable,
        NativeRetroArchPathRequest::Windows { .. } => bail!("macOS seal test got Windows request"),
    };
    session.verify_launch_identity(executable, core, content, &prepared, &[])?;
    let late_environment = [(
        OsString::from("LUNCHBOX_ORACLE_LATE_ENV"),
        OsString::from("must-be-rejected"),
    )];
    let late_environment_drift_rejected = session
        .verify_launch_identity(executable, core, content, &prepared, &late_environment)
        .is_err();
    ensure!(
        late_environment_drift_rejected,
        "production session accepted launch-environment drift after preparation"
    );
    let resource = bundle.join("Contents/Resources/en.lproj/InfoPlist.strings");
    let original = fs::read(&resource)
        .with_context(|| format!("reading sealed test resource {}", resource.display()))?;
    ensure!(!original.is_empty(), "sealed test resource is empty");
    let mut mutated = original.clone();
    mutated[0] ^= 1;
    fs::write(&resource, &mutated)?;
    let rejected = session.verify().is_err();
    fs::write(&resource, &original)?;
    session
        .verify()
        .context("restored macOS app bundle did not recover its valid seal")?;
    ensure!(
        rejected,
        "production session accepted a mutated sealed macOS app resource"
    );
    Ok((true, late_environment_drift_rejected))
}

#[allow(clippy::too_many_arguments)]
fn start_frontend(
    purpose: &str,
    request: &NativeRetroArchPathRequest,
    profile: &FrontendAutoconfigProfileSpec,
    core: &Path,
    content: &Path,
    extra_arguments: &[String],
    cache: &Path,
    log_path: &Path,
    port: u16,
    current_directory: &Path,
) -> Result<RunningFrontend> {
    let executable = match request {
        NativeRetroArchPathRequest::MacOs { executable, .. } => executable,
        NativeRetroArchPathRequest::Windows { .. } => bail!("macOS oracle got Windows request"),
    };
    let bundle = match request {
        NativeRetroArchPathRequest::MacOs {
            application_bundle, ..
        } => application_bundle,
        NativeRetroArchPathRequest::Windows { .. } => unreachable!(),
    };
    let mut base_arguments = vec![OsString::from("--verbose")];
    base_arguments.extend(extra_arguments.iter().map(OsString::from));
    base_arguments.extend([
        OsString::from("-L"),
        core.as_os_str().to_owned(),
        content.as_os_str().to_owned(),
    ]);
    let (session, arguments) = prepare_pinned_frontend_autoconfig_session(
        request,
        core,
        MACOS_RETROARCH_1_22_2_SHA256,
        MACOS_NESTOPIA_SHA256,
        profile,
        content,
        &base_arguments,
        &[],
        cache,
    )?;
    ensure!(session.paths.main_config.ends_with("config/retroarch.cfg"));
    ensure_no_physical_mapping_keys(&session.artifacts.append_config)?;
    ensure!(
        session
            .artifacts
            .core_options
            .contains("nestopia_button_shift = \"disabled\"")
    );
    ensure!(
        !session
            .artifacts
            .core_options
            .contains("nestopia_button_shift = \"enabled\"")
    );
    ensure!(
        session
            .artifacts
            .core_options
            .contains("oracle_preserved_option = ")
    );
    match profile.id.as_str() {
        NESTOPIA_NES_TWO_PLAYER_PROFILE => {
            ensure!(
                session
                    .effective_core_options_path
                    .ends_with("config/source-core-options.cfg"),
                "two-player helper selected an unexpected source options file"
            );
            for expected in [
                "input_libretro_device_p1 = \"257\"",
                "input_libretro_device_p2 = \"257\"",
                "input_libretro_device_p3 = \"0\"",
                "input_libretro_device_p4 = \"0\"",
                "input_max_users = \"2\"",
            ] {
                ensure!(session.artifacts.append_config.contains(expected));
            }
            ensure!(
                !session
                    .artifacts
                    .core_options
                    .contains("nestopia_select_adapter =")
            );
        }
        NESTOPIA_NES_FOUR_PLAYER_PROFILE => {
            ensure!(
                session
                    .effective_core_options_path
                    .ends_with("config/four-player-source-core-options.cfg"),
                "four-player helper selected an unexpected source options file"
            );
            for expected in [
                "input_libretro_device_p1 = \"257\"",
                "input_libretro_device_p2 = \"257\"",
                "input_libretro_device_p3 = \"257\"",
                "input_libretro_device_p4 = \"257\"",
                "input_max_users = \"4\"",
            ] {
                ensure!(session.artifacts.append_config.contains(expected));
            }
            ensure!(
                session
                    .artifacts
                    .core_options
                    .contains("nestopia_select_adapter = \"ntsc\"")
            );
        }
        _ => bail!("macOS oracle got an unexpected profile"),
    }
    session.verify_launch_identity(executable, core, content, &arguments, &[])?;
    let log = File::create(log_path)?;
    let child = Command::new(executable)
        .args(&arguments)
        .current_dir(current_directory)
        .stdout(Stdio::from(log.try_clone()?))
        .stderr(Stdio::from(log))
        .spawn()
        .with_context(|| format!("starting {}", executable.display()))?;
    let evidence = LaunchEvidence {
        purpose: purpose.to_owned(),
        program: unicode(executable)?.to_owned(),
        argv: arguments
            .iter()
            .map(|argument| {
                argument
                    .to_str()
                    .context("non-Unicode prepared argument")
                    .map(str::to_owned)
            })
            .collect::<Result<_>>()?,
        effective_main_config: unicode(&session.paths.main_config)?.to_owned(),
        effective_source_core_options: unicode(&session.effective_core_options_path)?.to_owned(),
        generated_append_config: session.artifacts.append_config.clone(),
        generated_core_options: session.artifacts.core_options.clone(),
        exit_code: None,
        forced_termination: false,
    };
    let source_main = fs::read(&session.paths.main_config)?;
    let source_options = fs::read(&session.effective_core_options_path)?;
    Ok(RunningFrontend {
        child: Some(child),
        session,
        client: CommandClient::new(port, bundle.clone())?,
        evidence,
        program: executable.clone(),
        core: core.to_path_buf(),
        content: content.to_path_buf(),
        source_main,
        source_options,
    })
}

impl RunningFrontend {
    fn finish(mut self) -> Result<LaunchEvidence> {
        if let Some(status) = self
            .child
            .as_mut()
            .context("missing owned RetroArch child")?
            .try_wait()?
        {
            self.child = None;
            self.verify_after_runtime()?;
            self.evidence.exit_code = status.code();
            return Ok(self.evidence.clone());
        }
        let mut forced = false;
        self.client.ensure_child_running(&mut self.child)?;
        self.client.wake()?;
        self.client.ensure_child_running(&mut self.child)?;
        self.client.send("QUIT")?;
        let status = wait_for_child(&mut self.child, Duration::from_secs(5))?;
        let status = match status {
            Some(status) => status,
            None => {
                forced = true;
                let child = self
                    .child
                    .as_mut()
                    .context("missing owned RetroArch child")?;
                child.kill().context("terminating owned RetroArch child")?;
                child.wait().context("reaping owned RetroArch child")?
            }
        };
        self.child = None;
        self.verify_after_runtime()?;
        self.evidence.exit_code = status.code();
        self.evidence.forced_termination = forced;
        Ok(self.evidence.clone())
    }

    fn verify_after_runtime(&self) -> Result<()> {
        verify_macos_bundle_seal_direct(&self.client.bundle)?;
        assert_sha256(&self.program, MACOS_RETROARCH_1_22_2_SHA256)?;
        assert_sha256(&self.core, MACOS_NESTOPIA_SHA256)?;
        ensure!(
            fs::read(&self.session.paths.main_config)? == self.source_main,
            "RetroArch changed the selected source main configuration"
        );
        ensure!(
            fs::read(&self.session.effective_core_options_path)? == self.source_options,
            "RetroArch changed the selected source core-options file"
        );
        ensure!(
            fs::read_to_string(&self.session.artifacts.append_config_path)?
                == self.session.artifacts.append_config,
            "RetroArch changed the private append configuration"
        );
        ensure!(
            fs::read_to_string(&self.session.artifacts.core_options_path)?
                == self.session.artifacts.core_options,
            "RetroArch changed the private core-options snapshot"
        );
        let content_hash = sha256_file(&self.content)?;
        ensure!(
            matches!(
                content_hash.as_str(),
                INPUT_ROM_SHA256 | PERSISTENCE_ROM_SHA256
            ),
            "generated oracle content changed during the frontend run"
        );
        verify_private_runtime_writes(&self.session)
    }
}

fn verify_macos_bundle_seal_direct(bundle: &Path) -> Result<()> {
    let mut command = Command::new("/usr/bin/codesign");
    command.args(["--verify", "--deep", "--strict"]).arg(bundle);
    let output = command_output_bounded(
        &mut command,
        Duration::from_secs(15),
        "executing the macOS code-signature verifier after a frontend run",
    )?;
    ensure!(
        output.status.success(),
        "isolated RetroArch app bundle failed strict deep code-signature verification: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    Ok(())
}

fn verify_private_runtime_writes(session: &FrontendAutoconfigSession) -> Result<()> {
    let allowed = BTreeSet::from([
        session.artifacts.append_config_path.clone(),
        session.artifacts.core_options_path.clone(),
        session.root().join("config/Nestopia/Nestopia.opt"),
    ]);
    let files = regular_files(session.root())?;
    ensure!(
        files.iter().all(|path| allowed.contains(path)),
        "RetroArch wrote an unexpected file inside the private frontend_autoconfig root: {:?}",
        files
            .iter()
            .filter(|path| !allowed.contains(*path))
            .collect::<Vec<_>>()
    );
    if let Some(options) = files
        .iter()
        .find(|path| path.ends_with("config/Nestopia/Nestopia.opt"))
    {
        let text = fs::read_to_string(options)?;
        ensure!(
            text.contains("nestopia_button_shift = \"disabled\"")
                && !text.contains("nestopia_button_shift = \"enabled\""),
            "RetroArch did not preserve the private disabled button-shift option"
        );
        if session
            .artifacts
            .core_options
            .contains("nestopia_select_adapter = \"ntsc\"")
        {
            ensure!(
                text.contains("nestopia_select_adapter = \"ntsc\""),
                "RetroArch did not preserve the private NTSC Four Score adapter option"
            );
        }
    }
    Ok(())
}

fn regular_files(root: &Path) -> Result<Vec<PathBuf>> {
    let mut files = Vec::new();
    let mut pending = vec![root.to_path_buf()];
    while let Some(directory) = pending.pop() {
        for entry in
            fs::read_dir(&directory).with_context(|| format!("reading {}", directory.display()))?
        {
            let entry = entry?;
            let kind = entry.file_type()?;
            ensure!(
                !kind.is_symlink(),
                "private frontend_autoconfig root contains a symlink: {}",
                entry.path().display()
            );
            if kind.is_dir() {
                pending.push(entry.path());
            } else if kind.is_file() {
                files.push(entry.path());
            } else {
                bail!(
                    "unsupported artifact inside private frontend_autoconfig root: {}",
                    entry.path().display()
                );
            }
        }
    }
    files.sort();
    Ok(files)
}

impl Drop for RunningFrontend {
    fn drop(&mut self) {
        if let Some(child) = self.child.as_mut() {
            let _ = child.kill();
            let _ = child.wait();
        }
    }
}

impl CommandClient {
    fn new(port: u16, bundle: PathBuf) -> Result<Self> {
        let socket = UdpSocket::bind((std::net::Ipv4Addr::LOCALHOST, 0))?;
        socket.set_read_timeout(Some(Duration::from_millis(300)))?;
        Ok(Self {
            socket,
            target: SocketAddrV4::new(std::net::Ipv4Addr::LOCALHOST, port),
            bundle,
        })
    }

    fn action(&self, child: &mut Option<Child>, command: &str) -> Result<()> {
        self.ensure_child_running(child)?;
        self.send(command)?;
        self.wake()?;
        thread::sleep(Duration::from_millis(120));
        self.ensure_child_running(child)
    }

    fn query(&self, child: &mut Option<Child>, command: &str) -> Result<String> {
        let deadline = Instant::now() + Duration::from_secs(15);
        let mut buffer = [0_u8; 65_535];
        loop {
            self.ensure_child_running(child)?;
            self.send(command)?;
            let _ = self.wake();
            match self.socket.recv_from(&mut buffer) {
                Ok((count, source)) => {
                    ensure!(
                        source == self.target.into(),
                        "unexpected command responder {source}"
                    );
                    let response = std::str::from_utf8(&buffer[..count])?
                        .trim_end_matches(['\0', '\r', '\n'])
                        .to_owned();
                    ensure!(!response.is_empty(), "empty RetroArch command response");
                    if response_matches_query(command, &response) {
                        return Ok(response);
                    }
                }
                Err(error)
                    if matches!(error.kind(), ErrorKind::WouldBlock | ErrorKind::TimedOut)
                        && Instant::now() < deadline => {}
                Err(error) => return Err(error.into()),
            }
        }
    }

    fn wait_core_ram(
        &self,
        child: &mut Option<Child>,
        address: usize,
        count: usize,
        predicate: impl Fn(&[u8]) -> bool,
    ) -> Result<Vec<u8>> {
        let deadline = Instant::now() + Duration::from_secs(15);
        let mut last_bytes = None;
        loop {
            let response = self.query(child, &format!("READ_CORE_RAM {address:X} {count}"))?;
            if let Ok(bytes) = parse_ram_response(&response, count) {
                if predicate(&bytes) {
                    return Ok(bytes);
                }
                last_bytes = Some(bytes);
            }
            ensure!(
                Instant::now() < deadline,
                "timed out waiting for core RAM predicate; last bytes={last_bytes:02X?}"
            );
            thread::sleep(Duration::from_millis(40));
        }
    }

    fn write_core_ram(
        &self,
        child: &mut Option<Child>,
        address: usize,
        bytes: &[u8],
    ) -> Result<()> {
        let mut command = format!("WRITE_CORE_RAM {address:X}");
        for byte in bytes {
            command.push_str(&format!(" {byte:02X}"));
        }
        self.action(child, &command)
    }

    fn send(&self, command: &str) -> Result<()> {
        ensure!(
            !command.is_empty()
                && command.is_ascii()
                && !command.as_bytes().contains(&0)
                && command.len() < 4096,
            "invalid RetroArch network command"
        );
        let mut packet = command.as_bytes().to_vec();
        packet.push(0);
        self.socket.send_to(&packet, self.target)?;
        Ok(())
    }

    fn wake(&self) -> Result<()> {
        let mut command = Command::new("/usr/bin/open");
        command.arg("-g").arg("-a").arg(&self.bundle);
        let output = command_output_bounded(
            &mut command,
            Duration::from_secs(3),
            "waking the RetroArch AppKit run loop",
        )?;
        ensure!(
            output.status.success(),
            "open wake failed: {}",
            String::from_utf8_lossy(&output.stderr)
        );
        Ok(())
    }

    fn ensure_child_running(&self, child: &mut Option<Child>) -> Result<()> {
        let child = child.as_mut().context("owned RetroArch child is absent")?;
        ensure!(
            child.try_wait()?.is_none(),
            "owned RetroArch child exited before the oracle operation"
        );
        Ok(())
    }
}

fn response_matches_query(command: &str, response: &str) -> bool {
    let requested = command.split_ascii_whitespace().collect::<Vec<_>>();
    let returned = response.split_ascii_whitespace().collect::<Vec<_>>();
    match requested.as_slice() {
        ["READ_CORE_RAM", address, _count] => {
            returned.len() >= 3
                && returned[0] == "READ_CORE_RAM"
                && returned[1].eq_ignore_ascii_case(address)
        }
        _ => response.starts_with(command),
    }
}

struct InputObservation {
    pairs: Vec<[u8; 2]>,
}

fn observe_simulated_input(frontend: &mut RunningFrontend) -> Result<InputObservation> {
    let initial = frontend
        .client
        .wait_core_ram(&mut frontend.child, 0, 7, |bytes| {
            bytes.len() == 7 && &bytes[1..4] == b"LBN" && bytes[0] == 0xFF && bytes[4] == 0xFF
        })?;
    let deadline = Instant::now() + Duration::from_secs(30);
    let mut pairs = vec![[initial[0], initial[4]]];
    let mut stage = 1;
    while Instant::now() < deadline {
        if frontend
            .child
            .as_mut()
            .is_some_and(|child| child.try_wait().ok().flatten().is_some())
        {
            break;
        }
        let response = frontend
            .client
            .query(&mut frontend.child, "READ_CORE_RAM 0 7")?;
        let bytes = parse_ram_response(&response, 7)?;
        ensure!(
            &bytes[1..4] == b"LBN",
            "diagnostic ROM marker changed at stage {stage}; bytes={bytes:02X?}, observed={pairs:02X?}"
        );
        ensure!(bytes[5] == 0 && bytes[6] == 0, "unexpected P3/P4 activity");
        ensure!(
            matches!(
                (bytes[0], bytes[4]),
                (0xFF, 0xFF) | (0xFE, 0xFF) | (0xFF, 0xFD)
            ),
            "unexpected P1/P2 input bytes"
        );
        let pair = [bytes[0], bytes[4]];
        if pairs.last() != Some(&pair) {
            pairs.push(pair);
        }
        stage = match (stage, pair) {
            (1, [0xFE, 0xFF]) => 2,
            (2, [0xFF, 0xFD]) => 3,
            (current, _) => current,
        };
        if stage == 3 {
            break;
        }
    }
    ensure!(
        stage == 3,
        "did not observe initial release -> P1 A -> P2 B through BSV replay; got {pairs:02X?}"
    );
    Ok(InputObservation { pairs })
}

struct FourPlayerInputObservation {
    channels: Vec<[u8; 4]>,
}

fn observe_four_player_input(frontend: &mut RunningFrontend) -> Result<FourPlayerInputObservation> {
    let initial = frontend
        .client
        .wait_core_ram(&mut frontend.child, 0, 7, |bytes| {
            bytes.len() == 7
                && &bytes[1..4] == b"LBN"
                && [bytes[0], bytes[4], bytes[5], bytes[6]] == [0xFF; 4]
        })?;
    let deadline = Instant::now() + Duration::from_secs(60);
    let mut channels = vec![[initial[0], initial[4], initial[5], initial[6]]];
    let mut stage = 1;
    while Instant::now() < deadline {
        if frontend
            .child
            .as_mut()
            .is_some_and(|child| child.try_wait().ok().flatten().is_some())
        {
            break;
        }
        let response = frontend
            .client
            .query(&mut frontend.child, "READ_CORE_RAM 0 7")?;
        let bytes = parse_ram_response(&response, 7)?;
        ensure!(
            &bytes[1..4] == b"LBN",
            "four-player diagnostic marker changed at stage {stage}; bytes={bytes:02X?}, observed={channels:02X?}"
        );
        let sample = [bytes[0], bytes[4], bytes[5], bytes[6]];
        ensure!(
            matches!(
                sample,
                [0xFF, 0xFF, 0xFF, 0xFF]
                    | [0xFE, 0xFF, 0xFF, 0xFF]
                    | [0xFF, 0xFD, 0xFF, 0xFF]
                    | [0xFF, 0xFF, 0xF7, 0xFF]
                    | [0xFF, 0xFF, 0xFF, 0x7F]
            ),
            "unexpected Four Score input channels: {sample:02X?}"
        );
        if channels.last() != Some(&sample) {
            channels.push(sample);
        }
        stage = match (stage, sample) {
            (1, [0xFE, 0xFF, 0xFF, 0xFF]) => 2,
            (2, [0xFF, 0xFD, 0xFF, 0xFF]) => 3,
            (3, [0xFF, 0xFF, 0xF7, 0xFF]) => 4,
            (4, [0xFF, 0xFF, 0xFF, 0x7F]) => 5,
            (current, _) => current,
        };
        if stage == 5 {
            break;
        }
    }
    ensure!(
        stage == 5,
        "did not observe initial release -> P1 A -> P2 B -> P3 Start -> P4 Right through Four Score BSV replay; got {channels:02X?}"
    );
    Ok(FourPlayerInputObservation { channels })
}

struct ReplaySynthesis {
    source_frames: usize,
}

fn synthesize_replay(
    source: &Path,
    output: &Path,
    player_count: usize,
    output_frames: usize,
    phases: &[(usize, usize, usize, i16)],
) -> Result<ReplaySynthesis> {
    ensure!(
        matches!(player_count, 2 | 4),
        "unsupported replay player count"
    );
    ensure!(output_frames > 0, "replay output must contain frames");
    for &(first, last, player, mask) in phases {
        ensure!(
            first < last && last <= output_frames && player < player_count && mask > 0,
            "invalid simulated replay phase"
        );
    }
    let source_bytes = fs::read(source)?;
    ensure!(
        source_bytes.len() >= BSV_HEADER_BYTES,
        "BSV2 header is truncated"
    );
    ensure!(
        read_u32(&source_bytes, 0)? == BSV2_MAGIC,
        "wrong BSV2 magic"
    );
    ensure!(
        read_u32(&source_bytes, 4)? == BSV2_VERSION,
        "wrong BSV2 version"
    );
    let initial_state_bytes = read_u32(&source_bytes, 12)? as usize;
    let first_frame = BSV_HEADER_BYTES
        .checked_add(initial_state_bytes)
        .context("BSV2 initial state offset overflow")?;
    ensure!(
        first_frame > BSV_HEADER_BYTES && first_frame < source_bytes.len(),
        "invalid BSV2 initial state size"
    );
    let reported_frames = read_u32(&source_bytes, 24)? as usize;
    let mut frames = Vec::new();
    let mut mask_offsets = Vec::new();
    let mut offset = first_frame;
    while offset < source_bytes.len() {
        let start = offset;
        offset = offset.checked_add(4).context("BSV2 frame overflow")?;
        ensure!(offset < source_bytes.len(), "BSV2 key count is truncated");
        let key_count = source_bytes[offset] as usize;
        offset = offset
            .checked_add(1 + key_count * 12)
            .context("BSV2 key event overflow")?;
        ensure!(
            offset + 2 <= source_bytes.len(),
            "BSV2 input count is truncated"
        );
        let input_count = read_u16(&source_bytes, offset)? as usize;
        offset += 2;
        let events_start = offset;
        offset = offset
            .checked_add(input_count * 8)
            .context("BSV2 input event overflow")?;
        ensure!(offset < source_bytes.len(), "BSV2 frame token is truncated");
        ensure!(
            source_bytes[offset] == BSV_REGULAR_FRAME,
            "non-regular BSV2 frame"
        );
        offset += 1;
        let frame = source_bytes[start..offset].to_vec();
        let mut player_matches = vec![Vec::new(); player_count];
        for event in 0..input_count {
            let event_offset = events_start + event * 8;
            let local = event_offset - start;
            ensure!(frame[local + 3] == 0, "BSV2 event padding is nonzero");
            if usize::from(frame[local]) < player_count
                && frame[local + 1] == LIBRETRO_JOYPAD_DEVICE
                && frame[local + 2] == 0
                && read_u16(&frame, local + 4)? == LIBRETRO_JOYPAD_MASK_ID
            {
                ensure!(
                    read_i16(&frame, local + 6)? == 0,
                    "template P1 mask is not released"
                );
                player_matches[frame[local] as usize].push(local + 6);
            }
        }
        for (player, matches) in player_matches.iter().enumerate() {
            ensure!(
                matches.len() == 1,
                "frame does not have exactly one P{} joypad-mask event",
                player + 1
            );
        }
        frames.push(frame);
        mask_offsets.push(
            player_matches
                .into_iter()
                .map(|matches| matches[0])
                .collect::<Vec<_>>(),
        );
    }
    ensure!(!frames.is_empty(), "BSV2 template has no frames");
    ensure!(
        frames.len() == reported_frames,
        "BSV2 parsed {} frames but header reports {reported_frames}",
        frames.len()
    );
    let mut generated = source_bytes[..first_frame].to_vec();
    write_u32(&mut generated, 24, output_frames as u32)?;
    for frame_index in 0..output_frames {
        let template_index = frame_index % frames.len();
        let mut frame = frames[template_index].clone();
        let frame_len = frame.len() as u32;
        write_u32(&mut frame, 0, if frame_index == 0 { 0 } else { frame_len })?;
        for &(first, last, player, mask) in phases {
            if (first..last).contains(&frame_index) {
                write_i16(&mut frame, mask_offsets[template_index][player], mask)?;
            }
        }
        generated.extend_from_slice(&frame);
    }
    fs::write(output, generated)?;
    Ok(ReplaySynthesis {
        source_frames: frames.len(),
    })
}

fn isolated_main_config(root: &Path, source_options: &Path, port: u16) -> Result<String> {
    let path = |suffix: &str| -> Result<String> { unicode(&root.join(suffix)).map(str::to_owned) };
    Ok(format!(
        "config_save_on_exit = \"false\"\n\
history_list_enable = \"false\"\n\
content_history_size = \"0\"\n\
network_cmd_enable = \"true\"\n\
network_cmd_port = \"{port}\"\n\
input_driver = \"cocoa\"\n\
input_joypad_driver = \"mfi\"\n\
input_autodetect_enable = \"true\"\n\
video_driver = \"metal\"\n\
video_fullscreen = \"false\"\n\
video_windowed_fullscreen = \"false\"\n\
video_vsync = \"true\"\n\
audio_driver = \"null\"\n\
audio_enable = \"false\"\n\
menu_driver = \"null\"\n\
menu_pause_libretro = \"false\"\n\
pause_nonactive = \"false\"\n\
pause_on_disconnect = \"false\"\n\
bundle_assets_extract_enable = \"false\"\n\
autosave_interval = \"0\"\n\
block_sram_overwrite = \"false\"\n\
save_file_compression = \"false\"\n\
savestate_file_compression = \"false\"\n\
savestate_auto_index = \"false\"\n\
savestate_auto_load = \"false\"\n\
savestate_auto_save = \"false\"\n\
sort_savefiles_enable = \"false\"\n\
sort_savestates_enable = \"false\"\n\
state_slot = \"0\"\n\
game_specific_options = \"false\"\n\
global_core_options = \"true\"\n\
core_options_path = \"{}\"\n\
savefile_directory = \"{}\"\n\
savestate_directory = \"{}\"\n\
system_directory = \"{}\"\n\
cache_directory = \"{}\"\n\
assets_directory = \"{}\"\n\
core_assets_directory = \"{}\"\n\
playlist_directory = \"{}\"\n\
screenshot_directory = \"{}\"\n\
recording_output_directory = \"{}\"\n\
recording_config_directory = \"{}\"\n\
cheat_database_path = \"{}\"\n\
content_database_path = \"{}\"\n\
thumbnails_directory = \"{}\"\n\
joypad_autoconfig_dir = \"{}\"\n\
overlay_directory = \"{}\"\n\
osk_overlay_directory = \"{}\"\n\
libretro_info_path = \"{}\"\n\
core_info_cache_enable = \"false\"\n\
cheevos_enable = \"false\"\n",
        unicode(source_options)?,
        path("saves")?,
        path("states")?,
        path("system")?,
        path("cache")?,
        path("assets")?,
        path("downloads")?,
        path("playlists")?,
        path("screenshots")?,
        path("recordings")?,
        path("recording-config")?,
        path("cheats")?,
        path("database")?,
        path("thumbnails")?,
        path("autoconfig")?,
        path("overlays")?,
        path("overlays/keyboards")?,
        path("info")?,
    ))
}

impl ExternalPathSnapshot {
    fn capture(path: &Path) -> Result<Self> {
        match fs::symlink_metadata(path) {
            Ok(metadata) => Ok(Self {
                path: path.to_path_buf(),
                kind: if metadata.file_type().is_symlink() {
                    "symlink"
                } else if metadata.is_file() {
                    "file"
                } else if metadata.is_dir() {
                    "directory"
                } else {
                    "other"
                },
                len: metadata.len(),
                modified: metadata.modified().ok(),
                mode: metadata.permissions().mode(),
                sha256: metadata.is_file().then(|| sha256_file(path)).transpose()?,
            }),
            Err(error) if error.kind() == ErrorKind::NotFound => Ok(Self {
                path: path.to_path_buf(),
                kind: "absent",
                len: 0,
                modified: None,
                mode: 0,
                sha256: None,
            }),
            Err(error) => Err(error).with_context(|| format!("snapshotting {}", path.display())),
        }
    }
}

fn external_retroarch_paths(home: &Path) -> Vec<PathBuf> {
    vec![
        home.join("Library/Application Support/RetroArch"),
        home.join(".config/retroarch"),
        home.join(".retroarch.cfg"),
        home.join("Library/Preferences/org.libretro.RetroArch.plist"),
        home.join("Library/Preferences/com.libretro.RetroArch.plist"),
        home.join("Library/Caches/org.libretro.RetroArch"),
        home.join("Library/Saved Application State/org.libretro.RetroArch.savedState"),
    ]
}

fn copy_tree(source: &Path, destination: &Path) -> Result<()> {
    let metadata = fs::symlink_metadata(source)?;
    if metadata.file_type().is_symlink() {
        symlink(fs::read_link(source)?, destination)?;
    } else if metadata.is_dir() {
        fs::create_dir(destination)?;
        for entry in fs::read_dir(source)? {
            let entry = entry?;
            copy_tree(&entry.path(), &destination.join(entry.file_name()))?;
        }
        fs::set_permissions(destination, metadata.permissions())?;
    } else if metadata.is_file() {
        fs::copy(source, destination)?;
        fs::set_permissions(destination, metadata.permissions())?;
    } else {
        bail!("unsupported app-bundle entry: {}", source.display());
    }
    Ok(())
}

fn reserve_udp_port() -> Result<u16> {
    let socket = UdpSocket::bind((std::net::Ipv4Addr::LOCALHOST, 0))?;
    Ok(socket.local_addr()?.port())
}

fn pump_bundle_events(bundle: &Path, duration: Duration) -> Result<()> {
    let deadline = Instant::now() + duration;
    while Instant::now() < deadline {
        let output = Command::new("/usr/bin/open")
            .arg("-a")
            .arg(bundle)
            .output()?;
        ensure!(output.status.success(), "failed to pump AppKit event loop");
        thread::sleep(Duration::from_millis(80));
    }
    Ok(())
}

fn wait_for_child(child: &mut Option<Child>, timeout: Duration) -> Result<Option<ExitStatus>> {
    let deadline = Instant::now() + timeout;
    loop {
        let child = child.as_mut().context("owned RetroArch child is absent")?;
        if let Some(status) = child.try_wait()? {
            return Ok(Some(status));
        }
        if Instant::now() >= deadline {
            return Ok(None);
        }
        thread::sleep(Duration::from_millis(50));
    }
}

fn command_output_bounded(
    command: &mut Command,
    timeout: Duration,
    description: &str,
) -> Result<Output> {
    let mut stdout = tempfile::tempfile()?;
    let mut stderr = tempfile::tempfile()?;
    let mut child = command
        .stdin(Stdio::null())
        .stdout(Stdio::from(stdout.try_clone()?))
        .stderr(Stdio::from(stderr.try_clone()?))
        .spawn()
        .with_context(|| description.to_owned())?;
    let deadline = Instant::now() + timeout;
    let status = loop {
        if let Some(status) = child.try_wait()? {
            break status;
        }
        if Instant::now() >= deadline {
            let _ = child.kill();
            let _ = child.wait();
            bail!("timed out {description}");
        }
        thread::sleep(Duration::from_millis(10));
    };
    stdout.seek(SeekFrom::Start(0))?;
    stderr.seek(SeekFrom::Start(0))?;
    let mut stdout_bytes = Vec::new();
    let mut stderr_bytes = Vec::new();
    stdout.read_to_end(&mut stdout_bytes)?;
    stderr.read_to_end(&mut stderr_bytes)?;
    Ok(Output {
        status,
        stdout: stdout_bytes,
        stderr: stderr_bytes,
    })
}

fn wait_for_file_stable(path: &Path, timeout: Duration) -> Result<Vec<u8>> {
    let deadline = Instant::now() + timeout;
    let mut last = None;
    let mut stable = 0;
    loop {
        if let Ok(bytes) = fs::read(path) {
            ensure!(!bytes.is_empty(), "{} is empty", path.display());
            let current = (bytes.len(), sha256_bytes(&bytes));
            if last.as_ref() == Some(&current) {
                stable += 1;
                if stable >= 3 {
                    return Ok(bytes);
                }
            } else {
                stable = 0;
                last = Some(current);
            }
        }
        ensure!(
            Instant::now() < deadline,
            "timed out waiting for {}",
            path.display()
        );
        thread::sleep(Duration::from_millis(100));
    }
}

fn wait_for_file_prefix(
    path: &Path,
    prefix: &[u8],
    expected_len: usize,
    timeout: Duration,
) -> Result<Vec<u8>> {
    let deadline = Instant::now() + timeout;
    loop {
        if let Ok(bytes) = fs::read(path)
            && bytes.len() == expected_len
            && bytes.starts_with(prefix)
        {
            return Ok(bytes);
        }
        ensure!(
            Instant::now() < deadline,
            "timed out waiting for {}",
            path.display()
        );
        thread::sleep(Duration::from_millis(100));
    }
}

fn parse_ram_response(response: &str, expected: usize) -> Result<Vec<u8>> {
    let fields = response.split_ascii_whitespace().collect::<Vec<_>>();
    ensure!(
        fields.len() >= expected + 2,
        "short core RAM response: {response}"
    );
    let bytes = fields[fields.len() - expected..]
        .iter()
        .map(|field| u8::from_str_radix(field, 16).context("invalid core RAM byte"))
        .collect::<Result<Vec<_>>>()?;
    ensure!(bytes.len() == expected);
    Ok(bytes)
}

fn file_evidence(path: &Path) -> Result<FileEvidence> {
    Ok(FileEvidence {
        path: unicode(path)?.to_owned(),
        bytes: fs::metadata(path)?.len(),
        sha256: sha256_file(path)?,
    })
}

fn assert_sha256(path: &Path, expected: &str) -> Result<()> {
    let actual = sha256_file(path)?;
    ensure!(
        actual == expected,
        "wrong SHA-256 for {}: {actual}",
        path.display()
    );
    Ok(())
}

fn sha256_file(path: &Path) -> Result<String> {
    let mut file = File::open(path)?;
    let mut digest = Sha256::new();
    let mut buffer = [0_u8; 64 * 1024];
    loop {
        let count = file.read(&mut buffer)?;
        if count == 0 {
            break;
        }
        digest.update(&buffer[..count]);
    }
    Ok(hex::encode(digest.finalize()))
}

fn file_contains(path: &Path, needle: &[u8]) -> Result<bool> {
    ensure!(!needle.is_empty());
    let bytes = fs::read(path)?;
    Ok(bytes.windows(needle.len()).any(|window| window == needle))
}

fn sha256_bytes(bytes: &[u8]) -> String {
    hex::encode(Sha256::digest(bytes))
}

fn write_json_report(path: &Path, value: &impl Serialize) -> Result<()> {
    let bytes = serde_json::to_vec_pretty(value)?;
    let parent = path.parent().context("report path has no parent")?;
    let mut temporary = tempfile::NamedTempFile::new_in(parent)?;
    temporary.write_all(&bytes)?;
    temporary.write_all(b"\n")?;
    temporary.as_file().sync_all()?;
    temporary
        .persist(path)
        .map_err(|error| error.error)
        .with_context(|| format!("persisting {}", path.display()))?;
    Ok(())
}

fn unicode(path: &Path) -> Result<&str> {
    path.to_str()
        .with_context(|| format!("path is not Unicode: {}", path.display()))
}

fn read_u16(bytes: &[u8], offset: usize) -> Result<u16> {
    let value: [u8; 2] = bytes
        .get(offset..offset + 2)
        .context("truncated little-endian u16")?
        .try_into()?;
    Ok(u16::from_le_bytes(value))
}

fn read_i16(bytes: &[u8], offset: usize) -> Result<i16> {
    Ok(read_u16(bytes, offset)? as i16)
}

fn read_u32(bytes: &[u8], offset: usize) -> Result<u32> {
    let value: [u8; 4] = bytes
        .get(offset..offset + 4)
        .context("truncated little-endian u32")?
        .try_into()?;
    Ok(u32::from_le_bytes(value))
}

fn write_i16(bytes: &mut [u8], offset: usize, value: i16) -> Result<()> {
    bytes
        .get_mut(offset..offset + 2)
        .context("truncated i16 write")?
        .copy_from_slice(&value.to_le_bytes());
    Ok(())
}

fn write_u32(bytes: &mut [u8], offset: usize, value: u32) -> Result<()> {
    bytes
        .get_mut(offset..offset + 4)
        .context("truncated u32 write")?
        .copy_from_slice(&value.to_le_bytes());
    Ok(())
}
