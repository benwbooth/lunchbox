//! Bounded, subprocess-isolated behavioral oracle for exact installed arcade cores.
//!
//! This intentionally consumes a user-owned ROM by absolute path. It never embeds,
//! stages, or copies game bytes into the repository or evidence directory.

use anyhow::{Context, Result, bail, ensure};
use clap::{Parser, ValueEnum};
use libloading::Library;
use lunchbox_controller_probe::libretro_options::OptionEnvironment;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::collections::{BTreeMap, BTreeSet};
use std::ffi::{CString, c_char, c_void};
use std::fs::{File, OpenOptions};
use std::io::{Read, Write};
use std::os::unix::fs::{OpenOptionsExt, PermissionsExt};
use std::path::{Path, PathBuf};
use std::process::{Command, ExitStatus, Stdio};
use std::sync::{
    Mutex,
    atomic::{AtomicBool, AtomicU16, AtomicU64, Ordering},
};
use std::time::{Duration, Instant};

const SCHEMA_VERSION: u32 = 1;
const RETRO_API_VERSION: u32 = 1;
const RETRO_DEVICE_JOYPAD: u32 = 1;
const RETRO_DEVICE_ANALOG: u32 = 5;
const RETRO_DEVICE_ID_JOYPAD_MASK: u32 = 256;
const RETRO_MEMORY_SYSTEM_RAM: u32 = 2;
const MAX_STATE_BYTES: usize = 128 * 1024 * 1024;
const MAX_RAM_BYTES: usize = 64 * 1024 * 1024;
const MAX_FRAME_BYTES: usize = 64 * 1024 * 1024;
const MAX_REPORT_BYTES: u64 = 16 * 1024 * 1024;
const BOOT_FRAMES: usize = 600;
const ACTIVATION_STEP_FRAMES: usize = 60;
const ACTIVATION_SEARCH_FRAMES: usize = 1_200;
const RESTORE_CONTINUATION_FRAMES: usize = 30;

static ACTIVE: AtomicBool = AtomicBool::new(false);
static BITMASK: AtomicBool = AtomicBool::new(false);
static SHUTDOWN: AtomicBool = AtomicBool::new(false);
static POLLS: AtomicU64 = AtomicU64::new(0);
static MASK_REQUESTS: AtomicU64 = AtomicU64::new(0);
static SINGLE_REQUESTS: AtomicU64 = AtomicU64::new(0);
static PRESSED: [AtomicU16; 8] = [
    AtomicU16::new(0),
    AtomicU16::new(0),
    AtomicU16::new(0),
    AtomicU16::new(0),
    AtomicU16::new(0),
    AtomicU16::new(0),
    AtomicU16::new(0),
    AtomicU16::new(0),
];
static SYSTEM_DIRECTORY: Mutex<Option<CString>> = Mutex::new(None);
static SAVE_DIRECTORY: Mutex<Option<CString>> = Mutex::new(None);
static CONTENT_DIRECTORY: Mutex<Option<CString>> = Mutex::new(None);
static OPTIONS: Mutex<Option<OptionEnvironment>> = Mutex::new(None);
static INPUT_QUERIES: Mutex<BTreeSet<InputAddress>> = Mutex::new(BTreeSet::new());
static DESCRIPTORS: Mutex<Capture<Vec<InputDescriptor>>> = Mutex::new(Capture {
    updates: 0,
    value: Ok(Vec::new()),
});
static CONTROLLERS: Mutex<Capture<Vec<Vec<ControllerChoice>>>> = Mutex::new(Capture {
    updates: 0,
    value: Ok(Vec::new()),
});
static VIDEO: Mutex<VideoCapture> = Mutex::new(VideoCapture::new());

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize, ValueEnum)]
#[serde(rename_all = "kebab-case")]
pub enum ArcadeProfile {
    Fbneo,
    Mame,
}

#[derive(Clone, Copy)]
struct ProfileSpec {
    core_name: &'static str,
    core_version: &'static str,
    core_sha256: &'static str,
    source_commit: &'static str,
    device: u32,
    state_bytes: usize,
}

impl ArcadeProfile {
    fn spec(self) -> ProfileSpec {
        match self {
            Self::Fbneo => ProfileSpec {
                core_name: "FinalBurn Neo",
                core_version: "v1.0.0.03 260417 GITe923538",
                core_sha256: "3555759523d6da5f78012c6846921ac27884b03264604387d07a4877579a177e",
                source_commit: "e9235389cede90638ad2726cfe80660841b23425",
                // FBNeo advertises Classic as RETRO_DEVICE_ANALOG, while its
                // digital gameplay callbacks still use RETRO_DEVICE_JOYPAD.
                device: RETRO_DEVICE_ANALOG,
                state_bytes: 14_256,
            },
            Self::Mame => ProfileSpec {
                core_name: "MAME",
                core_version: "0.287 (a891bc3b)",
                core_sha256: "8bcc096667a3c24baefece40b610f981a5c8de13447af486b0ad9381e28302e7",
                source_commit: "a891bc3b98c5a9f00848c953c8768007c6d339cb",
                device: RETRO_DEVICE_JOYPAD,
                state_bytes: 4_568_390,
            },
        }
    }

    fn cli(self) -> &'static str {
        match self {
            Self::Fbneo => "fbneo",
            Self::Mame => "mame",
        }
    }
}

#[derive(Debug, Parser)]
#[command(
    about = "Prove deterministic arcade input and save-state behavior in an exact installed core"
)]
pub struct Args {
    #[arg(long, value_enum)]
    profile: ArcadeProfile,
    /// Exact installed libretro core shared library.
    #[arg(long)]
    core: PathBuf,
    /// Exact core SHA-256. Omit only for the pinned Linux Flatpak profile default.
    #[arg(long, requires = "expected_core_version")]
    expected_core_sha256: Option<String>,
    /// Exact `retro_get_system_info` version. Omit only for the pinned Linux profile default.
    #[arg(long, requires = "expected_core_sha256")]
    expected_core_version: Option<String>,
    /// Exact serialized-state byte count expected for this build; defaults to the Linux profile.
    #[arg(long)]
    expected_state_bytes: Option<usize>,
    /// Full source commit for provenance when known for a caller-supplied build.
    #[arg(long)]
    expected_source_commit: Option<String>,
    /// User-owned 1943.zip; bytes are read in place and never copied into the repo.
    #[arg(long)]
    content: PathBuf,
    /// New private evidence directory. A retained private temporary directory is used if omitted.
    #[arg(long)]
    output: Option<PathBuf>,
    /// Exercise libretro's joypad bitmask callback path instead of per-button queries.
    #[arg(long)]
    bitmask: bool,
    #[arg(long, default_value_t = 45, value_parser = clap::value_parser!(u64).range(5..=300))]
    timeout_seconds: u64,
    #[arg(long, hide = true, value_enum)]
    worker_phase: Option<WorkerPhase>,
    #[arg(long, hide = true, requires = "worker_phase")]
    worker_report: Option<PathBuf>,
    #[arg(long, hide = true, requires = "worker_phase")]
    evidence: Option<PathBuf>,
}

#[derive(Clone)]
struct ExpectedCore {
    name: &'static str,
    version: String,
    sha256: String,
    source_commit: Option<String>,
    device: u32,
    state_bytes: usize,
}

impl Args {
    fn expected_core(&self) -> Result<ExpectedCore> {
        let profile = self.profile.spec();
        let caller_override = self.expected_core_sha256.is_some();
        let sha256 = self
            .expected_core_sha256
            .as_deref()
            .unwrap_or(profile.core_sha256);
        ensure!(
            sha256.len() == 64 && sha256.bytes().all(|byte| byte.is_ascii_hexdigit()),
            "Expected core SHA-256 must be exactly 64 hexadecimal digits"
        );
        let version = self
            .expected_core_version
            .as_deref()
            .unwrap_or(profile.core_version);
        ensure!(
            !version.is_empty() && version.len() <= 1024 && !version.contains('\0'),
            "Expected core version is invalid"
        );
        let state_bytes = self.expected_state_bytes.unwrap_or(profile.state_bytes);
        ensure!(
            state_bytes > 0 && state_bytes <= MAX_STATE_BYTES,
            "Expected serialized-state byte count is outside bounds"
        );
        let source_commit = if caller_override {
            self.expected_source_commit.clone()
        } else {
            if let Some(commit) = &self.expected_source_commit {
                ensure!(
                    commit == profile.source_commit,
                    "Default Linux profile source commit cannot be overridden independently"
                );
            }
            Some(profile.source_commit.to_owned())
        };
        if let Some(commit) = &source_commit {
            ensure!(
                commit.len() == 40 && commit.bytes().all(|byte| byte.is_ascii_hexdigit()),
                "Expected source commit must be a full 40-digit hexadecimal ID"
            );
        }
        Ok(ExpectedCore {
            name: profile.core_name,
            version: version.to_owned(),
            sha256: sha256.to_ascii_lowercase(),
            source_commit,
            device: profile.device,
            state_bytes,
        })
    }
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize, ValueEnum)]
#[serde(rename_all = "kebab-case")]
enum WorkerPhase {
    Capture,
    FreshRestore,
}

impl WorkerPhase {
    fn cli(self) -> &'static str {
        match self {
            Self::Capture => "capture",
            Self::FreshRestore => "fresh-restore",
        }
    }
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
struct CoreIdentity {
    path: PathBuf,
    sha256: String,
    name: String,
    version: String,
    valid_extensions: String,
    need_fullpath: bool,
    block_extract: bool,
    source_commit: Option<String>,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
struct ContentIdentity {
    path: PathBuf,
    bytes: u64,
    sha256: String,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Ord, PartialOrd, Serialize)]
#[serde(deny_unknown_fields)]
struct InputAddress {
    port: u32,
    device: u32,
    index: u32,
    id: u32,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
struct InputDescriptor {
    port: u32,
    device: u32,
    index: u32,
    id: u32,
    description: String,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
struct ControllerChoice {
    description: String,
    id: u32,
}

struct Capture<T> {
    updates: u64,
    value: Result<T, String>,
}

struct CallbackSnapshots {
    controller_info_updates: u64,
    controller_choices: Vec<Vec<ControllerChoice>>,
    descriptor_updates: u64,
    input_descriptors: Vec<InputDescriptor>,
    effective_options: BTreeMap<String, String>,
    input_queries: Vec<InputAddress>,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
struct Observation {
    system_ram_bytes: usize,
    system_ram_sha256: String,
    state_bytes: usize,
    state_sha256: String,
    video_width: u32,
    video_height: u32,
    video_pitch: usize,
    video_sha256: Option<String>,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
struct ResponseEvidence {
    stage: String,
    player: u32,
    control: String,
    joypad_id: u32,
    press_frames: usize,
    settle_frames: usize,
    neutral: Observation,
    first: Observation,
    repeat: Observation,
    deterministic: bool,
    system_ram_changed_from_neutral: bool,
    state_changed_from_neutral: bool,
    video_changed_from_neutral: bool,
    distinguishing_observable: Option<String>,
    unique_emulated_response_within_stage: bool,
    promoted_as_distinguished: bool,
    collision_with: Vec<String>,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
struct RestoreEvidence {
    baseline_state_bytes: usize,
    baseline_state_sha256: String,
    continuation_frames: usize,
    initial_continuation: Observation,
    same_process_restored_continuation: Observation,
    same_process_exact: bool,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
struct CaptureReport {
    schema_version: u32,
    profile: ArcadeProfile,
    core: CoreIdentity,
    content: ContentIdentity,
    bitmask_enabled: bool,
    boot_frames: usize,
    player_1_activation_frames: usize,
    player_2_join_activation_frames: usize,
    controller_info_updates: u64,
    controller_choices: Vec<Vec<ControllerChoice>>,
    descriptor_updates: u64,
    input_descriptors: Vec<InputDescriptor>,
    effective_options: BTreeMap<String, String>,
    input_queries: Vec<InputAddress>,
    input_polls: u64,
    bitmask_requests: u64,
    individual_requests: u64,
    responses: Vec<ResponseEvidence>,
    same_process_restore: RestoreEvidence,
    shutdown_requested: bool,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
struct FreshRestoreReport {
    schema_version: u32,
    profile: ArcadeProfile,
    core: CoreIdentity,
    content: ContentIdentity,
    bitmask_enabled: bool,
    continuation_frames: usize,
    restored_continuation: Observation,
}

#[derive(Debug, Serialize)]
struct FinalReport {
    schema_version: u32,
    status: &'static str,
    profile: ArcadeProfile,
    core: CoreIdentity,
    content: ContentIdentity,
    bitmask_enabled: bool,
    evidence_directory: PathBuf,
    topology: TopologySummary,
    responses: Vec<ResponseEvidence>,
    state_restore: FinalRestoreReport,
    callback_evidence: CallbackEvidence,
}

#[derive(Debug, Serialize)]
struct TopologySummary {
    selected_device_by_port: BTreeMap<u32, u32>,
    descriptors: Vec<InputDescriptor>,
    controller_choices: Vec<Vec<ControllerChoice>>,
}

#[derive(Debug, Serialize)]
struct FinalRestoreReport {
    baseline_state_bytes: usize,
    baseline_state_sha256: String,
    continuation_frames: usize,
    same_process_exact: bool,
    fresh_process_system_ram_exact: bool,
    fresh_process_video_exact: bool,
    fresh_process_emulated_exact: bool,
    fresh_process_reserialized_state_exact: bool,
    expected_continuation: Observation,
    same_process_continuation: Observation,
    fresh_process_continuation: Observation,
}

#[derive(Debug, Serialize)]
struct CallbackEvidence {
    input_polls: u64,
    bitmask_requests: u64,
    individual_requests: u64,
    addresses: Vec<InputAddress>,
    effective_options: BTreeMap<String, String>,
    descriptor_updates: u64,
    controller_info_updates: u64,
}

#[derive(Serialize, Deserialize)]
#[serde(tag = "status", rename_all = "snake_case", deny_unknown_fields)]
enum WorkerOutcome<T> {
    Success { report: T },
    Failure { error: String },
}

#[repr(C)]
#[derive(Default)]
struct NativeSystemInfo {
    library_name: *const c_char,
    library_version: *const c_char,
    valid_extensions: *const c_char,
    need_fullpath: bool,
    block_extract: bool,
}

#[repr(C)]
struct NativeGameInfo {
    path: *const c_char,
    data: *const c_void,
    size: usize,
    meta: *const c_char,
}

#[repr(C)]
struct NativeInputDescriptor {
    port: u32,
    device: u32,
    index: u32,
    id: u32,
    description: *const c_char,
}

#[repr(C)]
struct NativeControllerChoice {
    description: *const c_char,
    id: u32,
}
#[repr(C)]
struct NativeControllerInfo {
    types: *const NativeControllerChoice,
    count: u32,
}

type EnvironmentFn = unsafe extern "C" fn(u32, *mut c_void) -> bool;
type VideoFn = unsafe extern "C" fn(*const c_void, u32, u32, usize);
type AudioFn = unsafe extern "C" fn(i16, i16);
type AudioBatchFn = unsafe extern "C" fn(*const i16, usize) -> usize;
type PollFn = unsafe extern "C" fn();
type InputFn = unsafe extern "C" fn(u32, u32, u32, u32) -> i16;

struct VideoCapture {
    bytes: Vec<u8>,
    width: u32,
    height: u32,
    pitch: usize,
    failure: Option<String>,
}
impl VideoCapture {
    const fn new() -> Self {
        Self {
            bytes: Vec::new(),
            width: 0,
            height: 0,
            pitch: 0,
            failure: None,
        }
    }
}

unsafe fn copy_c_string(pointer: *const c_char, limit: usize, label: &str) -> Result<String> {
    ensure!(!pointer.is_null(), "Null {label}");
    let mut bytes = Vec::new();
    for offset in 0..=limit {
        let byte = unsafe { *pointer.add(offset) } as u8;
        if byte == 0 {
            return String::from_utf8(bytes).context(label.to_owned());
        }
        ensure!(offset < limit, "{label} exceeds capture limit");
        bytes.push(byte);
    }
    unreachable!()
}

unsafe fn capture_descriptors(data: *const NativeInputDescriptor) -> Result<Vec<InputDescriptor>> {
    ensure!(!data.is_null(), "Null input-descriptor list");
    let mut values = Vec::new();
    for index in 0..=4096 {
        let item = unsafe { &*data.add(index) };
        if item.description.is_null() {
            return Ok(values);
        }
        ensure!(index < 4096, "Input-descriptor list exceeds capture limit");
        values.push(InputDescriptor {
            port: item.port,
            device: item.device,
            index: item.index,
            id: item.id,
            description: unsafe { copy_c_string(item.description, 1024, "input descriptor") }?,
        });
    }
    unreachable!()
}

unsafe fn capture_controllers(
    data: *const NativeControllerInfo,
) -> Result<Vec<Vec<ControllerChoice>>> {
    ensure!(!data.is_null(), "Null controller-info list");
    let mut ports = Vec::new();
    for port in 0..=16 {
        let info = unsafe { &*data.add(port) };
        if info.types.is_null() {
            ensure!(
                info.count == 0,
                "Controller-info terminator has nonzero count"
            );
            return Ok(ports);
        }
        ensure!(
            port < 16 && info.count <= 64,
            "Controller-info exceeds capture limits"
        );
        let mut choices = Vec::new();
        for index in 0..info.count as usize {
            let choice = unsafe { &*info.types.add(index) };
            // MAME 0.287 includes its null-label terminator in `count`. Accept
            // that specific valid C-array shape rather than dereferencing it.
            if choice.description.is_null() {
                break;
            }
            choices.push(ControllerChoice {
                description: unsafe {
                    copy_c_string(choice.description, 1024, "controller label")
                }?,
                id: choice.id,
            });
        }
        ports.push(choices);
    }
    bail!("Controller-info list has no terminator")
}

unsafe extern "C" fn environment(command: u32, data: *mut c_void) -> bool {
    let command = command & !0x10000;
    if let Ok(mut options) = OPTIONS.lock() {
        if let Some(options) = options.as_mut()
            && let Some(result) = unsafe { options.handle(command, data) }
        {
            return result;
        }
    } else {
        return false;
    }
    if command == 51 {
        return BITMASK.load(Ordering::Relaxed);
    }
    match command {
        7 => {
            SHUTDOWN.store(true, Ordering::Relaxed);
            true
        }
        3 => {
            if data.is_null() {
                return false;
            }
            unsafe {
                data.cast::<bool>().write(true);
            }
            true
        }
        8 => true,
        9 | 30 | 31 => {
            if data.is_null() {
                return false;
            }
            let directory = match command {
                9 => &SYSTEM_DIRECTORY,
                30 => &CONTENT_DIRECTORY,
                _ => &SAVE_DIRECTORY,
            };
            let Ok(directory) = directory.lock() else {
                return false;
            };
            let Some(directory) = directory.as_ref() else {
                return false;
            };
            unsafe {
                data.cast::<*const c_char>().write(directory.as_ptr());
            }
            true
        }
        10 => {
            if data.is_null() {
                return false;
            }
            unsafe { *data.cast::<u32>() <= 2 }
        }
        11 => {
            let result = unsafe { capture_descriptors(data.cast()) }.map_err(|e| e.to_string());
            let valid = result.is_ok();
            let Ok(mut capture) = DESCRIPTORS.lock() else {
                return false;
            };
            capture.updates = capture.updates.saturating_add(1);
            capture.value = result;
            valid
        }
        18 => true,
        32 | 34 | 36 | 37 | 42 => true,
        35 => {
            let result = unsafe { capture_controllers(data.cast()) }.map_err(|e| e.to_string());
            let valid = result.is_ok();
            let Ok(mut capture) = CONTROLLERS.lock() else {
                return false;
            };
            capture.updates = capture.updates.saturating_add(1);
            capture.value = result;
            valid
        }
        39 => {
            if data.is_null() {
                return false;
            }
            unsafe {
                data.cast::<u32>().write(0);
            }
            true
        }
        57 => {
            if data.is_null() {
                return false;
            }
            unsafe {
                data.cast::<u32>().write(1);
            }
            true
        }
        59 => {
            if data.is_null() {
                return false;
            }
            unsafe {
                data.cast::<u32>().write(1);
            }
            true
        }
        _ => false,
    }
}

unsafe extern "C" fn video(data: *const c_void, width: u32, height: u32, pitch: usize) {
    let Ok(mut capture) = VIDEO.lock() else {
        return;
    };
    capture.width = width;
    capture.height = height;
    capture.pitch = pitch;
    if data.is_null() {
        return;
    } // libretro duplicate-frame convention
    if data as usize == usize::MAX {
        capture
            .failure
            .get_or_insert_with(|| "Hardware framebuffer cannot be hashed".into());
        return;
    }
    let Some(bytes) = pitch.checked_mul(height as usize) else {
        capture
            .failure
            .get_or_insert_with(|| "Video byte count overflowed".into());
        return;
    };
    if bytes > MAX_FRAME_BYTES {
        capture
            .failure
            .get_or_insert_with(|| "Video frame exceeds capture limit".into());
        return;
    }
    capture.bytes.clear();
    capture
        .bytes
        .extend_from_slice(unsafe { std::slice::from_raw_parts(data.cast::<u8>(), bytes) });
}

unsafe extern "C" fn audio(_left: i16, _right: i16) {}
unsafe extern "C" fn audio_batch(_data: *const i16, frames: usize) -> usize {
    frames
}
unsafe extern "C" fn input_poll() {
    POLLS.fetch_add(1, Ordering::Relaxed);
}
unsafe extern "C" fn input_state(port: u32, device: u32, index: u32, id: u32) -> i16 {
    if let Ok(mut addresses) = INPUT_QUERIES.lock()
        && addresses.len() < 4096
    {
        addresses.insert(InputAddress {
            port,
            device,
            index,
            id,
        });
    }
    let base_device = device & 0xff;
    if base_device != RETRO_DEVICE_JOYPAD {
        return 0;
    }
    let mask = PRESSED
        .get(port as usize)
        .map(|v| v.load(Ordering::Relaxed))
        .unwrap_or(0);
    if id == RETRO_DEVICE_ID_JOYPAD_MASK {
        MASK_REQUESTS.fetch_add(1, Ordering::Relaxed);
        return if BITMASK.load(Ordering::Relaxed) {
            mask as i16
        } else {
            0
        };
    }
    SINGLE_REQUESTS.fetch_add(1, Ordering::Relaxed);
    if id < 16 && mask & (1_u16 << id) != 0 {
        1
    } else {
        0
    }
}

fn reset_callback_state(bitmask: bool, system: &Path, save: &Path, content: &Path) -> Result<()> {
    ensure!(
        !ACTIVE.swap(true, Ordering::SeqCst),
        "Only one native core may run per worker"
    );
    BITMASK.store(bitmask, Ordering::Relaxed);
    SHUTDOWN.store(false, Ordering::Relaxed);
    POLLS.store(0, Ordering::Relaxed);
    MASK_REQUESTS.store(0, Ordering::Relaxed);
    SINGLE_REQUESTS.store(0, Ordering::Relaxed);
    for pressed in &PRESSED {
        pressed.store(0, Ordering::Relaxed);
    }
    *SYSTEM_DIRECTORY
        .lock()
        .map_err(|_| anyhow::anyhow!("System-directory lock poisoned"))? =
        Some(path_cstring(system)?);
    *SAVE_DIRECTORY
        .lock()
        .map_err(|_| anyhow::anyhow!("Save-directory lock poisoned"))? = Some(path_cstring(save)?);
    *CONTENT_DIRECTORY
        .lock()
        .map_err(|_| anyhow::anyhow!("Content-directory lock poisoned"))? =
        Some(path_cstring(content)?);
    *OPTIONS
        .lock()
        .map_err(|_| anyhow::anyhow!("Core-option lock poisoned"))? =
        Some(OptionEnvironment::new(BTreeMap::new())?);
    *INPUT_QUERIES
        .lock()
        .map_err(|_| anyhow::anyhow!("Input-query lock poisoned"))? = BTreeSet::new();
    *DESCRIPTORS
        .lock()
        .map_err(|_| anyhow::anyhow!("Descriptor lock poisoned"))? = Capture {
        updates: 0,
        value: Ok(Vec::new()),
    };
    *CONTROLLERS
        .lock()
        .map_err(|_| anyhow::anyhow!("Controller lock poisoned"))? = Capture {
        updates: 0,
        value: Ok(Vec::new()),
    };
    *VIDEO
        .lock()
        .map_err(|_| anyhow::anyhow!("Video lock poisoned"))? = VideoCapture::new();
    Ok(())
}

fn clear_callback_state() {
    for pressed in &PRESSED {
        pressed.store(0, Ordering::Relaxed);
    }
    if let Ok(mut value) = SYSTEM_DIRECTORY.lock() {
        *value = None;
    }
    if let Ok(mut value) = SAVE_DIRECTORY.lock() {
        *value = None;
    }
    if let Ok(mut value) = CONTENT_DIRECTORY.lock() {
        *value = None;
    }
    if let Ok(mut value) = OPTIONS.lock() {
        *value = None;
    }
    ACTIVE.store(false, Ordering::SeqCst);
}

fn path_cstring(path: &Path) -> Result<CString> {
    CString::new(path.as_os_str().as_encoded_bytes()).context("Path contains NUL")
}

type SetEnvironment = unsafe extern "C" fn(EnvironmentFn);
type SetVideo = unsafe extern "C" fn(VideoFn);
type SetAudio = unsafe extern "C" fn(AudioFn);
type SetAudioBatch = unsafe extern "C" fn(AudioBatchFn);
type SetPoll = unsafe extern "C" fn(PollFn);
type SetInput = unsafe extern "C" fn(InputFn);
type Init = unsafe extern "C" fn();
type Deinit = unsafe extern "C" fn();
type ApiVersion = unsafe extern "C" fn() -> u32;
type GetSystemInfo = unsafe extern "C" fn(*mut NativeSystemInfo);
type LoadGame = unsafe extern "C" fn(*const NativeGameInfo) -> bool;
type UnloadGame = unsafe extern "C" fn();
type Run = unsafe extern "C" fn();
type SetController = unsafe extern "C" fn(u32, u32);
type SerializeSize = unsafe extern "C" fn() -> usize;
type SerializeState = unsafe extern "C" fn(*mut c_void, usize) -> bool;
type UnserializeState = unsafe extern "C" fn(*const c_void, usize) -> bool;
type GetMemoryData = unsafe extern "C" fn(u32) -> *mut c_void;
type GetMemorySize = unsafe extern "C" fn(u32) -> usize;

struct LoadedCore {
    library: Library,
    deinit: Deinit,
    unload_game: UnloadGame,
    run: Run,
    set_controller: SetController,
    serialize_size: SerializeSize,
    serialize_state: SerializeState,
    unserialize_state: UnserializeState,
    get_memory_data: GetMemoryData,
    get_memory_size: GetMemorySize,
    expected_state_bytes: usize,
    initialized: bool,
    loaded: bool,
}

impl LoadedCore {
    fn open(
        core_path: &Path,
        content_path: &Path,
        args: &Args,
        private_root: &Path,
        bitmask: bool,
    ) -> Result<(Self, CoreIdentity)> {
        let system_dir = private_root.join("system");
        let save_dir = private_root.join("save");
        let content_dir = private_root.join("content-root");
        create_private_dir(&system_dir)?;
        create_private_dir(&save_dir)?;
        create_private_dir(&content_dir)?;
        reset_callback_state(bitmask, &system_dir, &save_dir, &content_dir)?;

        let library = unsafe { Library::new(core_path) }
            .with_context(|| format!("Loading trusted core {}", core_path.display()))?;
        unsafe fn symbol<T: Copy>(library: &Library, name: &'static [u8]) -> Result<T> {
            Ok(*unsafe { library.get::<T>(name) }.with_context(|| {
                format!("Missing libretro symbol {}", String::from_utf8_lossy(name))
            })?)
        }
        let set_environment: SetEnvironment =
            unsafe { symbol(&library, b"retro_set_environment\0")? };
        let set_video: SetVideo = unsafe { symbol(&library, b"retro_set_video_refresh\0")? };
        let set_audio: SetAudio = unsafe { symbol(&library, b"retro_set_audio_sample\0")? };
        let set_audio_batch: SetAudioBatch =
            unsafe { symbol(&library, b"retro_set_audio_sample_batch\0")? };
        let set_poll: SetPoll = unsafe { symbol(&library, b"retro_set_input_poll\0")? };
        let set_input: SetInput = unsafe { symbol(&library, b"retro_set_input_state\0")? };
        let init: Init = unsafe { symbol(&library, b"retro_init\0")? };
        let deinit: Deinit = unsafe { symbol(&library, b"retro_deinit\0")? };
        let api_version: ApiVersion = unsafe { symbol(&library, b"retro_api_version\0")? };
        let get_system_info: GetSystemInfo =
            unsafe { symbol(&library, b"retro_get_system_info\0")? };
        let load_game: LoadGame = unsafe { symbol(&library, b"retro_load_game\0")? };
        let unload_game: UnloadGame = unsafe { symbol(&library, b"retro_unload_game\0")? };
        let run: Run = unsafe { symbol(&library, b"retro_run\0")? };
        let set_controller: SetController =
            unsafe { symbol(&library, b"retro_set_controller_port_device\0")? };
        let serialize_size: SerializeSize = unsafe { symbol(&library, b"retro_serialize_size\0")? };
        let serialize_state: SerializeState = unsafe { symbol(&library, b"retro_serialize\0")? };
        let unserialize_state: UnserializeState =
            unsafe { symbol(&library, b"retro_unserialize\0")? };
        let get_memory_data: GetMemoryData =
            unsafe { symbol(&library, b"retro_get_memory_data\0")? };
        let get_memory_size: GetMemorySize =
            unsafe { symbol(&library, b"retro_get_memory_size\0")? };

        unsafe {
            set_environment(environment);
            set_video(video);
            set_audio(audio);
            set_audio_batch(audio_batch);
            set_poll(input_poll);
            set_input(input_state);
        }
        ensure!(
            unsafe { api_version() } == RETRO_API_VERSION,
            "Unsupported libretro API version"
        );
        let mut system_info = NativeSystemInfo::default();
        unsafe {
            get_system_info(&mut system_info);
        }
        let expected = args.expected_core()?;
        let identity = CoreIdentity {
            path: core_path.canonicalize()?,
            sha256: hash_file(core_path)?,
            name: unsafe { copy_c_string(system_info.library_name, 1024, "core name") }?,
            version: unsafe { copy_c_string(system_info.library_version, 1024, "core version") }?,
            valid_extensions: unsafe {
                copy_c_string(system_info.valid_extensions, 4096, "core extensions")
            }?,
            need_fullpath: system_info.need_fullpath,
            block_extract: system_info.block_extract,
            source_commit: expected.source_commit.clone(),
        };
        ensure!(
            identity.sha256.eq_ignore_ascii_case(&expected.sha256),
            "Installed core SHA-256 does not match pinned profile"
        );
        ensure!(
            identity.name == expected.name && identity.version == expected.version,
            "Installed core identity does not match pinned profile: {} {}",
            identity.name,
            identity.version
        );
        ensure!(
            identity.need_fullpath,
            "Pinned arcade core unexpectedly does not require a full path"
        );

        unsafe {
            init();
        }
        let mut core = Self {
            library,
            deinit,
            unload_game,
            run,
            set_controller,
            serialize_size,
            serialize_state,
            unserialize_state,
            get_memory_data,
            get_memory_size,
            expected_state_bytes: expected.state_bytes,
            initialized: true,
            loaded: false,
        };
        unsafe {
            (core.set_controller)(0, expected.device);
            (core.set_controller)(1, expected.device);
        }
        let path = path_cstring(content_path)?;
        let info = NativeGameInfo {
            path: path.as_ptr(),
            data: std::ptr::null(),
            size: 0,
            meta: std::ptr::null(),
        };
        ensure!(
            unsafe { load_game(&info) },
            "Core rejected pinned 1943.zip content"
        );
        core.loaded = true;
        // Repeat selection after load because FBNeo registers game topology there.
        unsafe {
            (core.set_controller)(0, expected.device);
            (core.set_controller)(1, expected.device);
        }
        Ok((core, identity))
    }

    fn run_frames(&mut self, frames: usize) -> Result<()> {
        for _ in 0..frames {
            ensure!(
                !SHUTDOWN.load(Ordering::Relaxed),
                "Core requested frontend shutdown"
            );
            unsafe {
                (self.run)();
            }
        }
        Ok(())
    }

    fn state(&self) -> Result<Vec<u8>> {
        let bytes = unsafe { (self.serialize_size)() };
        ensure!(bytes > 0, "Core reports no serialization support");
        ensure!(
            bytes == self.expected_state_bytes,
            "Core serialized-state size {bytes} does not match expected {} bytes for this exact build",
            self.expected_state_bytes
        );
        ensure!(
            bytes <= MAX_STATE_BYTES,
            "Serialized state exceeds capture limit"
        );
        let mut state = vec![0_u8; bytes];
        ensure!(
            unsafe { (self.serialize_state)(state.as_mut_ptr().cast(), state.len()) },
            "Core failed to serialize state"
        );
        Ok(state)
    }

    fn restore(&mut self, state: &[u8]) -> Result<()> {
        ensure!(
            !state.is_empty() && state.len() <= MAX_STATE_BYTES,
            "State size is outside bounds"
        );
        ensure!(
            unsafe { (self.unserialize_state)(state.as_ptr().cast(), state.len()) },
            "Core failed to restore state"
        );
        for pressed in &PRESSED {
            pressed.store(0, Ordering::Relaxed);
        }
        Ok(())
    }

    fn system_ram(&self) -> Result<Vec<u8>> {
        let bytes = unsafe { (self.get_memory_size)(RETRO_MEMORY_SYSTEM_RAM) };
        ensure!(bytes > 0, "Core exposes no RETRO_MEMORY_SYSTEM_RAM");
        ensure!(bytes <= MAX_RAM_BYTES, "System RAM exceeds capture limit");
        let pointer = unsafe { (self.get_memory_data)(RETRO_MEMORY_SYSTEM_RAM) };
        ensure!(!pointer.is_null(), "Core returned null system-RAM pointer");
        Ok(unsafe { std::slice::from_raw_parts(pointer.cast::<u8>(), bytes) }.to_vec())
    }

    fn observe(&self) -> Result<Observation> {
        let ram = self.system_ram()?;
        let state = self.state()?;
        let video = VIDEO
            .lock()
            .map_err(|_| anyhow::anyhow!("Video lock poisoned"))?;
        if let Some(failure) = &video.failure {
            bail!("Video callback failed: {failure}");
        }
        Ok(Observation {
            system_ram_bytes: ram.len(),
            system_ram_sha256: hash_bytes(&ram),
            state_bytes: state.len(),
            state_sha256: hash_bytes(&state),
            video_width: video.width,
            video_height: video.height,
            video_pitch: video.pitch,
            video_sha256: (!video.bytes.is_empty()).then(|| hash_bytes(&video.bytes)),
        })
    }
}

impl Drop for LoadedCore {
    fn drop(&mut self) {
        for pressed in &PRESSED {
            pressed.store(0, Ordering::Relaxed);
        }
        unsafe {
            if self.loaded {
                (self.unload_game)();
                self.loaded = false;
            }
            if self.initialized {
                (self.deinit)();
                self.initialized = false;
            }
        }
        clear_callback_state();
        let _ = &self.library;
    }
}

fn press(
    core: &mut LoadedCore,
    player: usize,
    id: u32,
    press_frames: usize,
    settle_frames: usize,
) -> Result<()> {
    ensure!(
        player < PRESSED.len() && id < 16,
        "Input address outside standard joypad bounds"
    );
    PRESSED[player].store(1_u16 << id, Ordering::Relaxed);
    core.run_frames(press_frames)?;
    PRESSED[player].store(0, Ordering::Relaxed);
    core.run_frames(settle_frames)
}

fn neutral(core: &mut LoadedCore, frames: usize) -> Result<()> {
    for pressed in &PRESSED {
        pressed.store(0, Ordering::Relaxed);
    }
    core.run_frames(frames)
}

fn observe_from_state(
    core: &mut LoadedCore,
    baseline: &[u8],
    player: usize,
    id: Option<u32>,
    press_frames: usize,
    settle_frames: usize,
) -> Result<Observation> {
    core.restore(baseline)?;
    match id {
        Some(id) => press(core, player, id, press_frames, settle_frames)?,
        None => neutral(core, press_frames + settle_frames)?,
    }
    core.observe()
}

#[allow(clippy::too_many_arguments)]
fn create_response(
    core: &mut LoadedCore,
    baseline: &[u8],
    stage: &str,
    player: usize,
    control: &str,
    id: u32,
    press_frames: usize,
    settle_frames: usize,
) -> Result<ResponseEvidence> {
    let neutral = observe_from_state(core, baseline, player, None, press_frames, settle_frames)?;
    let first = observe_from_state(
        core,
        baseline,
        player,
        Some(id),
        press_frames,
        settle_frames,
    )?;
    let repeat = observe_from_state(
        core,
        baseline,
        player,
        Some(id),
        press_frames,
        settle_frames,
    )?;
    let deterministic = first == repeat;
    Ok(ResponseEvidence {
        stage: stage.to_owned(),
        player: (player + 1) as u32,
        control: control.to_owned(),
        joypad_id: id,
        press_frames,
        settle_frames,
        system_ram_changed_from_neutral: first.system_ram_sha256 != neutral.system_ram_sha256,
        state_changed_from_neutral: first.state_sha256 != neutral.state_sha256,
        video_changed_from_neutral: first.video_sha256 != neutral.video_sha256,
        neutral,
        first,
        repeat,
        deterministic,
        distinguishing_observable: None,
        unique_emulated_response_within_stage: false,
        promoted_as_distinguished: false,
        collision_with: Vec::new(),
    })
}

fn baseline_accepts_gameplay_input(
    core: &mut LoadedCore,
    baseline: &[u8],
    player: usize,
) -> Result<bool> {
    for id in [6_u32, 0_u32] {
        // Left and Fire 1 exercise distinct game paths.
        let neutral = observe_from_state(core, baseline, player, None, 12, 1)?;
        let action = observe_from_state(core, baseline, player, Some(id), 12, 1)?;
        if action.system_ram_sha256 != neutral.system_ram_sha256
            || action.video_sha256 != neutral.video_sha256
        {
            return Ok(true);
        }
    }
    Ok(false)
}

fn find_active_gameplay(
    core: &mut LoadedCore,
    player: usize,
    stage: &str,
) -> Result<(Vec<u8>, usize)> {
    let mut elapsed = 0;
    while elapsed < ACTIVATION_SEARCH_FRAMES {
        neutral(core, ACTIVATION_STEP_FRAMES)?;
        elapsed += ACTIVATION_STEP_FRAMES;
        let candidate = core.state()?;
        if baseline_accepts_gameplay_input(core, &candidate, player)? {
            core.restore(&candidate)?;
            return Ok((candidate, elapsed));
        }
        core.restore(&candidate)?;
    }
    bail!(
        "{stage} never accepted player {} gameplay input within {} frames",
        player + 1,
        ACTIVATION_SEARCH_FRAMES
    )
}

fn classify_responses(responses: &mut [ResponseEvidence]) {
    for index in 0..responses.len() {
        let key = if responses[index].system_ram_changed_from_neutral {
            Some((
                "system_ram",
                responses[index].first.system_ram_sha256.as_str(),
            ))
        } else if responses[index].video_changed_from_neutral {
            responses[index]
                .first
                .video_sha256
                .as_deref()
                .map(|hash| ("video", hash))
        } else {
            None
        };
        let collisions: Vec<String> = responses
            .iter()
            .enumerate()
            .filter(|(other, candidate)| {
                if *other == index || candidate.stage != responses[index].stage {
                    return false;
                }
                let candidate_key = if candidate.system_ram_changed_from_neutral {
                    Some(("system_ram", candidate.first.system_ram_sha256.as_str()))
                } else if candidate.video_changed_from_neutral {
                    candidate
                        .first
                        .video_sha256
                        .as_deref()
                        .map(|hash| ("video", hash))
                } else {
                    None
                };
                candidate_key == key
            })
            .map(|(_, candidate)| format!("P{} {}", candidate.player, candidate.control))
            .collect();
        responses[index].distinguishing_observable = key.map(|(kind, _)| kind.to_owned());
        responses[index].unique_emulated_response_within_stage =
            key.is_some() && collisions.is_empty();
        responses[index].promoted_as_distinguished =
            responses[index].deterministic && key.is_some() && collisions.is_empty();
        responses[index].collision_with = collisions;
    }
}

fn hash_bytes(bytes: &[u8]) -> String {
    format!("{:x}", Sha256::digest(bytes))
}

fn emulated_observation_equal(left: &Observation, right: &Observation) -> bool {
    left.system_ram_bytes == right.system_ram_bytes
        && left.system_ram_sha256 == right.system_ram_sha256
        && left.video_width == right.video_width
        && left.video_height == right.video_height
        && left.video_pitch == right.video_pitch
        && left.video_sha256 == right.video_sha256
}

fn hash_file(path: &Path) -> Result<String> {
    let mut file = File::open(path)?;
    let mut digest = Sha256::new();
    let mut buffer = [0_u8; 128 * 1024];
    loop {
        let count = file.read(&mut buffer)?;
        if count == 0 {
            break;
        }
        digest.update(&buffer[..count]);
    }
    Ok(format!("{:x}", digest.finalize()))
}

fn content_identity(path: &Path) -> Result<ContentIdentity> {
    const EXPECTED_HASH: &str = "e44b89e80bf8bccc4f16476d7254d959d937a1668d27ec564d57103ac94fba6f";
    const EXPECTED_BYTES: u64 = 805_560;
    let canonical = path.canonicalize()?;
    ensure!(
        canonical.file_name().and_then(|v| v.to_str()) == Some("1943.zip"),
        "Pinned arcade oracle requires 1943.zip"
    );
    let metadata = canonical.metadata()?;
    ensure!(
        metadata.is_file() && metadata.len() == EXPECTED_BYTES,
        "1943.zip byte length does not match pinned user set"
    );
    let sha256 = hash_file(&canonical)?;
    ensure!(
        sha256.eq_ignore_ascii_case(EXPECTED_HASH),
        "1943.zip SHA-256 does not match pinned user set"
    );
    Ok(ContentIdentity {
        path: canonical,
        bytes: metadata.len(),
        sha256,
    })
}

fn create_private_dir(path: &Path) -> Result<()> {
    std::fs::create_dir(path)
        .with_context(|| format!("Creating private directory {}", path.display()))?;
    std::fs::set_permissions(path, std::fs::Permissions::from_mode(0o700))?;
    Ok(())
}

fn write_private_new(path: &Path, bytes: &[u8]) -> Result<()> {
    let mut options = OpenOptions::new();
    options.write(true).create_new(true).mode(0o600);
    let mut file = options
        .open(path)
        .with_context(|| format!("Creating {}", path.display()))?;
    file.write_all(bytes)?;
    file.sync_all()?;
    Ok(())
}

fn read_bounded(path: &Path, limit: u64) -> Result<Vec<u8>> {
    let mut bytes = Vec::new();
    File::open(path)?.take(limit + 1).read_to_end(&mut bytes)?;
    ensure!(
        bytes.len() as u64 <= limit,
        "{} exceeds read limit",
        path.display()
    );
    Ok(bytes)
}

fn snapshots() -> Result<CallbackSnapshots> {
    let controllers = CONTROLLERS
        .lock()
        .map_err(|_| anyhow::anyhow!("Controller lock poisoned"))?;
    let descriptors = DESCRIPTORS
        .lock()
        .map_err(|_| anyhow::anyhow!("Descriptor lock poisoned"))?;
    let options = OPTIONS
        .lock()
        .map_err(|_| anyhow::anyhow!("Core-option lock poisoned"))?;
    let queries = INPUT_QUERIES
        .lock()
        .map_err(|_| anyhow::anyhow!("Input-query lock poisoned"))?;
    Ok(CallbackSnapshots {
        controller_info_updates: controllers.updates,
        controller_choices: controllers.value.clone().map_err(anyhow::Error::msg)?,
        descriptor_updates: descriptors.updates,
        input_descriptors: descriptors.value.clone().map_err(anyhow::Error::msg)?,
        effective_options: options
            .as_ref()
            .ok_or_else(|| anyhow::anyhow!("Core-option registry missing"))?
            .effective_values()?,
        input_queries: queries.iter().cloned().collect(),
    })
}

fn validate_topology(
    profile: ArcadeProfile,
    controllers: &[Vec<ControllerChoice>],
    descriptors: &[InputDescriptor],
) -> Result<()> {
    let selected = profile.spec().device;
    let advertised_ports = if profile == ArcadeProfile::Mame { 1 } else { 2 };
    for port in 0..advertised_ports {
        ensure!(
            controllers
                .get(port)
                .is_some_and(|choices| choices.iter().any(|choice| choice.id == selected)),
            "Core did not advertise selected device {selected} on player {}",
            port + 1
        );
    }
    let expected = [
        (0, 2, "Coin 1"),
        (0, 3, "Start 1"),
        (1, 2, "Coin 2"),
        (1, 3, "Start 2"),
        (0, 4, "P1 Up"),
        (0, 5, "P1 Down"),
        (0, 6, "P1 Left"),
        (0, 7, "P1 Right"),
        (0, 0, "P1 Fire 1"),
        (0, 8, "P1 Fire 2"),
        (1, 4, "P2 Up"),
        (1, 5, "P2 Down"),
        (1, 6, "P2 Left"),
        (1, 7, "P2 Right"),
        (1, 0, "P2 Fire 1"),
        (1, 8, "P2 Fire 2"),
    ];
    for (port, id, label) in expected {
        ensure!(
            descriptors.iter().any(|d| d.port == port
                && d.device == RETRO_DEVICE_JOYPAD
                && d.index == 0
                && d.id == id),
            "Input descriptors omit {label} at port {port}, joypad id {id}"
        );
    }
    Ok(())
}

fn run_capture(args: &Args, evidence: &Path) -> Result<CaptureReport> {
    let content = content_identity(&args.content)?;
    let private_root = evidence.join("capture-private");
    create_private_dir(&private_root)?;
    let (mut core, identity) =
        LoadedCore::open(&args.core, &content.path, args, &private_root, args.bitmask)?;
    neutral(&mut core, BOOT_FRAMES)?;
    let attract = core.state()?;

    let mut responses = Vec::new();
    responses.push(create_response(
        &mut core,
        &attract,
        "attract-coin",
        0,
        "Coin",
        2,
        2,
        20,
    )?);
    responses.push(create_response(
        &mut core,
        &attract,
        "attract-coin",
        1,
        "Coin",
        2,
        2,
        20,
    )?);

    core.restore(&attract)?;
    press(&mut core, 0, 2, 2, 30)?;
    let one_credit = core.state()?;
    responses.push(create_response(
        &mut core,
        &one_credit,
        "credited-start-1",
        0,
        "Start",
        3,
        2,
        60,
    )?);

    core.restore(&attract)?;
    press(&mut core, 0, 2, 2, 30)?;
    press(&mut core, 1, 2, 2, 30)?;
    let two_credits = core.state()?;
    responses.push(create_response(
        &mut core,
        &two_credits,
        "credited-start-2",
        1,
        "Start",
        3,
        2,
        60,
    )?);

    // Enter gameplay through P1 first, then use the second coin/start channel
    // to join P2. This avoids treating a distinct title-screen Start-2 path as
    // proof that either player's gameplay controls are active.
    core.restore(&attract)?;
    press(&mut core, 0, 2, 2, 30)?;
    press(&mut core, 0, 3, 2, 30)?;
    let (one_player_gameplay, player_1_activation_frames) =
        find_active_gameplay(&mut core, 0, "P1 start")?;

    let controls = [
        (4, "Up"),
        (5, "Down"),
        (6, "Left"),
        (7, "Right"),
        (0, "Fire 1"),
        (8, "Fire 2"),
    ];
    for (id, name) in controls {
        responses.push(create_response(
            &mut core,
            &one_player_gameplay,
            "one-player-gameplay",
            0,
            name,
            id,
            12,
            1,
        )?);
    }

    core.restore(&one_player_gameplay)?;
    press(&mut core, 1, 2, 2, 30)?;
    press(&mut core, 1, 3, 2, 60)?;
    let (gameplay, player_2_join_activation_frames) =
        find_active_gameplay(&mut core, 1, "P2 join")?;
    for player in 0..2 {
        for (id, name) in controls {
            responses.push(create_response(
                &mut core,
                &gameplay,
                "two-player-gameplay",
                player,
                name,
                id,
                12,
                1,
            )?);
        }
    }
    classify_responses(&mut responses);

    let baseline_path = evidence.join("two-player-baseline.state");
    write_private_new(&baseline_path, &gameplay)?;
    core.restore(&gameplay)?;
    neutral(&mut core, RESTORE_CONTINUATION_FRAMES)?;
    let initial_continuation = core.observe()?;
    core.restore(&gameplay)?;
    neutral(&mut core, RESTORE_CONTINUATION_FRAMES)?;
    let same_process_restored_continuation = core.observe()?;
    let same_process_exact = initial_continuation == same_process_restored_continuation;
    ensure!(
        same_process_exact,
        "Same-process state restoration was not byte-deterministic"
    );

    let snapshots = snapshots()?;
    validate_topology(
        args.profile,
        &snapshots.controller_choices,
        &snapshots.input_descriptors,
    )?;
    ensure!(
        snapshots.descriptor_updates > 0 && snapshots.controller_info_updates > 0,
        "Core published no controller metadata"
    );
    ensure!(POLLS.load(Ordering::Relaxed) > 0, "Core never polled input");
    if args.bitmask {
        ensure!(
            MASK_REQUESTS.load(Ordering::Relaxed) > 0,
            "Core did not exercise negotiated bitmask input"
        );
    } else {
        ensure!(
            SINGLE_REQUESTS.load(Ordering::Relaxed) > 0,
            "Core did not exercise individual-button input"
        );
    }

    Ok(CaptureReport {
        schema_version: SCHEMA_VERSION,
        profile: args.profile,
        core: identity,
        content,
        bitmask_enabled: args.bitmask,
        boot_frames: BOOT_FRAMES,
        player_1_activation_frames,
        player_2_join_activation_frames,
        controller_info_updates: snapshots.controller_info_updates,
        controller_choices: snapshots.controller_choices,
        descriptor_updates: snapshots.descriptor_updates,
        input_descriptors: snapshots.input_descriptors,
        effective_options: snapshots.effective_options,
        input_queries: snapshots.input_queries,
        input_polls: POLLS.load(Ordering::Relaxed),
        bitmask_requests: MASK_REQUESTS.load(Ordering::Relaxed),
        individual_requests: SINGLE_REQUESTS.load(Ordering::Relaxed),
        responses,
        same_process_restore: RestoreEvidence {
            baseline_state_bytes: gameplay.len(),
            baseline_state_sha256: hash_bytes(&gameplay),
            continuation_frames: RESTORE_CONTINUATION_FRAMES,
            initial_continuation,
            same_process_restored_continuation,
            same_process_exact,
        },
        shutdown_requested: SHUTDOWN.load(Ordering::Relaxed),
    })
}

fn run_fresh_restore(args: &Args, evidence: &Path) -> Result<FreshRestoreReport> {
    let content = content_identity(&args.content)?;
    let state = read_bounded(
        &evidence.join("two-player-baseline.state"),
        MAX_STATE_BYTES as u64,
    )?;
    let private_root = evidence.join("fresh-restore-private");
    create_private_dir(&private_root)?;
    let (mut core, identity) =
        LoadedCore::open(&args.core, &content.path, args, &private_root, args.bitmask)?;
    core.restore(&state)?;
    neutral(&mut core, RESTORE_CONTINUATION_FRAMES)?;
    let restored_continuation = core.observe()?;
    Ok(FreshRestoreReport {
        schema_version: SCHEMA_VERSION,
        profile: args.profile,
        core: identity,
        content,
        bitmask_enabled: args.bitmask,
        continuation_frames: RESTORE_CONTINUATION_FRAMES,
        restored_continuation,
    })
}

struct ChildGuard(std::process::Child);
impl Drop for ChildGuard {
    fn drop(&mut self) {
        let _ = self.0.kill();
        let _ = self.0.wait();
    }
}

fn wait_for_child(child: &mut ChildGuard, timeout: Duration) -> Result<ExitStatus> {
    let deadline = Instant::now() + timeout;
    loop {
        if let Some(status) = child.0.try_wait()? {
            return Ok(status);
        }
        if Instant::now() >= deadline {
            let _ = child.0.kill();
            let status = child.0.wait()?;
            bail!("Trusted core worker exceeded wall-time limit; final status {status}");
        }
        std::thread::sleep(Duration::from_millis(20));
    }
}

fn spawn_worker<T>(
    args: &Args,
    evidence: &Path,
    phase: WorkerPhase,
    report_path: &Path,
) -> Result<T>
where
    T: for<'de> Deserialize<'de>,
{
    let mut command = Command::new(std::env::current_exe()?);
    command
        .arg("--profile")
        .arg(args.profile.cli())
        .arg("--core")
        .arg(&args.core)
        .arg("--content")
        .arg(&args.content)
        .arg("--timeout-seconds")
        .arg(args.timeout_seconds.to_string())
        .arg("--worker-phase")
        .arg(phase.cli())
        .arg("--worker-report")
        .arg(report_path)
        .arg("--evidence")
        .arg(evidence)
        .current_dir(evidence)
        .env("TMPDIR", evidence)
        .env("TMP", evidence)
        .env("TEMP", evidence)
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null());
    if args.bitmask {
        command.arg("--bitmask");
    }
    if let Some(value) = &args.expected_core_sha256 {
        command.arg("--expected-core-sha256").arg(value);
    }
    if let Some(value) = &args.expected_core_version {
        command.arg("--expected-core-version").arg(value);
    }
    if let Some(value) = args.expected_state_bytes {
        command.arg("--expected-state-bytes").arg(value.to_string());
    }
    if let Some(value) = &args.expected_source_commit {
        command.arg("--expected-source-commit").arg(value);
    }
    let mut child = ChildGuard(
        command
            .spawn()
            .context("Starting isolated arcade-oracle worker")?,
    );
    let status = wait_for_child(&mut child, Duration::from_secs(args.timeout_seconds))?;
    ensure!(
        status.success(),
        "Arcade-oracle {} worker failed: {status}",
        phase.cli()
    );
    let outcome: WorkerOutcome<T> =
        serde_json::from_slice(&read_bounded(report_path, MAX_REPORT_BYTES)?)?;
    match outcome {
        WorkerOutcome::Success { report } => Ok(report),
        WorkerOutcome::Failure { error } => bail!("Arcade-oracle {} worker: {error}", phase.cli()),
    }
}

fn make_evidence_root(requested: Option<&Path>) -> Result<PathBuf> {
    if let Some(requested) = requested {
        let path = if requested.is_absolute() {
            requested.to_owned()
        } else {
            std::env::current_dir()?.join(requested)
        };
        create_private_dir(&path)?;
        return Ok(path.canonicalize()?);
    }
    let temp = tempfile::Builder::new()
        .prefix("lunchbox-libretro-arcade-")
        .tempdir()?;
    let path = temp.keep();
    std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o700))?;
    Ok(path)
}

fn validate_capture(args: &Args, report: &CaptureReport) -> Result<()> {
    ensure!(
        report.schema_version == SCHEMA_VERSION
            && report.profile == args.profile
            && report.bitmask_enabled == args.bitmask,
        "Capture worker request/report mismatch"
    );
    ensure!(
        !report.shutdown_requested,
        "Core requested shutdown during capture"
    );
    ensure!(
        report.same_process_restore.same_process_exact,
        "Same-process restore did not match"
    );
    ensure!(
        report.responses.len() == 22,
        "Unexpected response-evidence count"
    );
    for response in &report.responses {
        ensure!(
            response.deterministic,
            "{} P{} {} was nondeterministic",
            response.stage,
            response.player,
            response.control
        );
    }
    Ok(())
}

fn supervise(args: &Args) -> Result<FinalReport> {
    let pinned_content = content_identity(&args.content)?;
    let expected = args.expected_core()?;
    ensure!(
        hash_file(&args.core)?.eq_ignore_ascii_case(&expected.sha256),
        "Installed core hash differs from pinned profile"
    );
    let evidence = make_evidence_root(args.output.as_deref())?;
    let capture_path = evidence.join("capture-worker.json");
    let fresh_path = evidence.join("fresh-restore-worker.json");
    let capture: CaptureReport =
        spawn_worker(args, &evidence, WorkerPhase::Capture, &capture_path)?;
    validate_capture(args, &capture)?;
    let state_path = evidence.join("two-player-baseline.state");
    let state = read_bounded(&state_path, MAX_STATE_BYTES as u64)?;
    ensure!(
        state.len() == capture.same_process_restore.baseline_state_bytes
            && hash_bytes(&state) == capture.same_process_restore.baseline_state_sha256,
        "Persisted baseline state differs from capture report"
    );

    let fresh: FreshRestoreReport =
        spawn_worker(args, &evidence, WorkerPhase::FreshRestore, &fresh_path)?;
    ensure!(
        fresh.schema_version == SCHEMA_VERSION
            && fresh.profile == args.profile
            && fresh.bitmask_enabled == args.bitmask
            && fresh.core == capture.core
            && fresh.content == capture.content
            && fresh.content == pinned_content,
        "Fresh-restore worker provenance differs from capture"
    );
    let fresh_process_system_ram_exact = fresh.restored_continuation.system_ram_bytes
        == capture
            .same_process_restore
            .initial_continuation
            .system_ram_bytes
        && fresh.restored_continuation.system_ram_sha256
            == capture
                .same_process_restore
                .initial_continuation
                .system_ram_sha256;
    let fresh_process_video_exact = fresh.restored_continuation.video_width
        == capture
            .same_process_restore
            .initial_continuation
            .video_width
        && fresh.restored_continuation.video_height
            == capture
                .same_process_restore
                .initial_continuation
                .video_height
        && fresh.restored_continuation.video_pitch
            == capture
                .same_process_restore
                .initial_continuation
                .video_pitch
        && fresh.restored_continuation.video_sha256
            == capture
                .same_process_restore
                .initial_continuation
                .video_sha256;
    let fresh_process_emulated_exact = emulated_observation_equal(
        &fresh.restored_continuation,
        &capture.same_process_restore.initial_continuation,
    );
    let fresh_process_reserialized_state_exact = fresh.restored_continuation.state_bytes
        == capture
            .same_process_restore
            .initial_continuation
            .state_bytes
        && fresh.restored_continuation.state_sha256
            == capture
                .same_process_restore
                .initial_continuation
                .state_sha256;

    let selected_device_by_port = [(0, expected.device), (1, expected.device)]
        .into_iter()
        .collect();
    let all_distinguished = capture
        .responses
        .iter()
        .all(|r| r.promoted_as_distinguished);
    let report = FinalReport {
        schema_version: SCHEMA_VERSION,
        status: if all_distinguished && fresh_process_emulated_exact {
            "pass"
        } else {
            "partial"
        },
        profile: args.profile,
        core: capture.core,
        content: capture.content,
        bitmask_enabled: args.bitmask,
        evidence_directory: evidence.clone(),
        topology: TopologySummary {
            selected_device_by_port,
            descriptors: capture.input_descriptors,
            controller_choices: capture.controller_choices,
        },
        responses: capture.responses,
        state_restore: FinalRestoreReport {
            baseline_state_bytes: capture.same_process_restore.baseline_state_bytes,
            baseline_state_sha256: capture.same_process_restore.baseline_state_sha256,
            continuation_frames: capture.same_process_restore.continuation_frames,
            same_process_exact: capture.same_process_restore.same_process_exact,
            fresh_process_system_ram_exact,
            fresh_process_video_exact,
            fresh_process_emulated_exact,
            fresh_process_reserialized_state_exact,
            expected_continuation: capture.same_process_restore.initial_continuation,
            same_process_continuation: capture
                .same_process_restore
                .same_process_restored_continuation,
            fresh_process_continuation: fresh.restored_continuation,
        },
        callback_evidence: CallbackEvidence {
            input_polls: capture.input_polls,
            bitmask_requests: capture.bitmask_requests,
            individual_requests: capture.individual_requests,
            addresses: capture.input_queries,
            effective_options: capture.effective_options,
            descriptor_updates: capture.descriptor_updates,
            controller_info_updates: capture.controller_info_updates,
        },
    };
    let bytes = serde_json::to_vec_pretty(&report)?;
    write_private_new(&evidence.join("report.json"), &bytes)?;
    Ok(report)
}

fn write_outcome<T: Serialize>(path: &Path, result: Result<T>) -> Result<()> {
    let outcome = match result {
        Ok(report) => WorkerOutcome::Success { report },
        Err(error) => WorkerOutcome::<T>::Failure {
            error: format!("{error:#}").chars().take(32_768).collect(),
        },
    };
    let bytes = serde_json::to_vec(&outcome)?;
    ensure!(
        bytes.len() as u64 <= MAX_REPORT_BYTES,
        "Worker report exceeds limit"
    );
    write_private_new(path, &bytes)
}

pub fn main_entry() -> Result<()> {
    let args = Args::parse();
    match args.worker_phase {
        Some(phase) => {
            ensure!(
                args.output.is_none(),
                "Worker cannot choose an output directory"
            );
            let evidence = args
                .evidence
                .as_deref()
                .ok_or_else(|| anyhow::anyhow!("Worker evidence directory missing"))?;
            let report = args
                .worker_report
                .as_deref()
                .ok_or_else(|| anyhow::anyhow!("Worker report path missing"))?;
            match phase {
                WorkerPhase::Capture => write_outcome(report, run_capture(&args, evidence)),
                WorkerPhase::FreshRestore => {
                    write_outcome(report, run_fresh_restore(&args, evidence))
                }
            }
        }
        None => {
            ensure!(
                args.worker_report.is_none() && args.evidence.is_none(),
                "Hidden worker arguments require --worker-phase"
            );
            let report = supervise(&args)?;
            println!("{}", serde_json::to_string_pretty(&report)?);
            Ok(())
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn observation(ram: &str, state: &str, video: &str) -> Observation {
        Observation {
            system_ram_bytes: 2,
            system_ram_sha256: ram.into(),
            state_bytes: 3,
            state_sha256: state.into(),
            video_width: 1,
            video_height: 1,
            video_pitch: 4,
            video_sha256: Some(video.into()),
        }
    }

    fn response(player: u32, control: &str, ram: &str, video: &str) -> ResponseEvidence {
        let neutral = observation("neutral-ram", "neutral-state", "neutral-video");
        let first = observation(ram, &format!("state-{player}-{control}"), video);
        ResponseEvidence {
            stage: "stage".into(),
            player,
            control: control.into(),
            joypad_id: 0,
            press_frames: 1,
            settle_frames: 1,
            neutral,
            repeat: first.clone(),
            first,
            deterministic: true,
            system_ram_changed_from_neutral: ram != "neutral-ram",
            state_changed_from_neutral: true,
            video_changed_from_neutral: video != "neutral-video",
            distinguishing_observable: None,
            unique_emulated_response_within_stage: false,
            promoted_as_distinguished: false,
            collision_with: Vec::new(),
        }
    }

    #[test]
    fn video_is_a_real_fallback_when_exported_ram_is_not_responsive() {
        let mut responses = vec![
            response(1, "Left", "neutral-ram", "left-frame"),
            response(1, "Right", "neutral-ram", "right-frame"),
        ];
        classify_responses(&mut responses);
        assert!(
            responses
                .iter()
                .all(|response| response.promoted_as_distinguished)
        );
        assert!(
            responses
                .iter()
                .all(|response| response.distinguishing_observable.as_deref() == Some("video"))
        );
    }

    #[test]
    fn shared_coin_outcome_is_not_promoted_as_two_distinct_channels() {
        let mut responses = vec![
            response(1, "Coin", "credit-ram", "credit-frame"),
            response(2, "Coin", "credit-ram", "credit-frame"),
        ];
        classify_responses(&mut responses);
        assert!(
            responses
                .iter()
                .all(|response| !response.promoted_as_distinguished)
        );
        assert_eq!(responses[0].collision_with, ["P2 Coin"]);
        assert_eq!(responses[1].collision_with, ["P1 Coin"]);
    }

    #[test]
    fn fresh_restore_comparison_does_not_hide_video_divergence() {
        let expected = observation("same-ram", "state-a", "frame-a");
        let actual = observation("same-ram", "state-b", "frame-b");
        assert!(!emulated_observation_equal(&expected, &actual));
    }

    #[test]
    fn linux_profiles_pin_exact_state_sizes_and_full_commits() {
        let fbneo = ArcadeProfile::Fbneo.spec();
        let mame = ArcadeProfile::Mame.spec();
        assert_eq!(fbneo.state_bytes, 14_256);
        assert_eq!(mame.state_bytes, 4_568_390);
        assert_eq!(fbneo.source_commit.len(), 40);
        assert_eq!(mame.source_commit.len(), 40);
    }
}
