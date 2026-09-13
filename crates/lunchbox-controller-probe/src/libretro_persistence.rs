//! Isolated, fail-closed persistence oracle for explicitly trusted libretro cores.
//!
//! The public driver always delegates each lifecycle to a fresh subprocess. The
//! worker runs an original diagnostic ROM, never user content, and exercises
//! the core's save-memory and serialization ABIs directly.

use anyhow::{Context, Result, bail, ensure};
use clap::{Parser, ValueEnum};
use libloading::Library;
use lunchbox_controller_probe::libretro_memory_map::{ExactMapping, MemoryMapSnapshot};
use lunchbox_controller_probe::{file_hash, libretro_options::OptionEnvironment};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::ffi::{CStr, CString, OsString, c_char, c_void};
use std::io::Read;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::sync::{
    Mutex,
    atomic::{AtomicBool, Ordering},
};
use std::time::{Duration, Instant};

const REPORT_SCHEMA: u32 = 6;
const RETRO_MEMORY_SAVE_RAM: u32 = 0;
const RETRO_MEMORY_SYSTEM_RAM: u32 = 2;
const MARKER_OFFSET: usize = 0x100;
const STATE_MARKER: &[u8; 8] = b"LBSTATE1";
const MUTATED_MARKER: &[u8; 8] = b"MUTATED!";
const FIRST_SAVE_OBSERVATION: &[u8; 6] = b"LBSG01";
const SECOND_SAVE_OBSERVATION: &[u8; 6] = b"LBSG02";
const FIRST_NES_SAVE_OBSERVATION: &[u8; 5] = b"LBSR\x01";
const SECOND_NES_SAVE_OBSERVATION: &[u8; 5] = b"LBSR\x02";
const PSX_MEMORY_CARD_MARKER_OFFSET: usize = 0x1f000;
const CAPTURE_LIMIT: usize = 1024 * 1024;

static SYSTEM_DIRECTORY: Mutex<Option<CString>> = Mutex::new(None);
static SAVE_DIRECTORY: Mutex<Option<CString>> = Mutex::new(None);
static METADATA_VFS_ENABLED: AtomicBool = AtomicBool::new(false);
static OPTIONS: Mutex<Option<OptionEnvironment>> = Mutex::new(None);
static MEMORY_MAP: Mutex<Result<Option<MemoryMapSnapshot>, String>> = Mutex::new(Ok(None));

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize, ValueEnum)]
#[serde(rename_all = "kebab-case")]
pub enum PersistenceSystem {
    Gba,
    GbaSkyemu,
    GbaVbam,
    GameboyGambatte,
    GameboyMgba,
    GameboySameboy,
    GameboySkyemu,
    GameboyVbam,
    Atari2600Stella,
    GameGear,
    NesFceumm,
    NesMesen,
    Snes9x,
    Bsnes,
    MesenS,
    PsxBeetle,
    PsxBeetleHw,
}

impl PersistenceSystem {
    fn spec(self) -> CoreSpec {
        match self {
            Self::Gba => CoreSpec {
                name: "mGBA",
                version: "0.11-219-e31759b",
                extension: "gba",
                need_fullpath: false,
                pre_save_bytes: 128 * 1024,
                post_save_bytes: 32 * 1024,
                system_ram_bytes: 32 * 1024,
                system_ram_offset: 0,
                memory_map: None,
                frame_limit: 12,
                first_observation: FIRST_SAVE_OBSERVATION,
                second_observation: SECOND_SAVE_OBSERVATION,
                // mGBA's state contracts to the detected 32 KiB SRAM size.
                state_bytes: Some(430_144),
            },
            Self::GbaSkyemu => CoreSpec {
                name: "SkyEmu",
                version: "adacd0788964ed89f5c43dcbc1f3cc26deec996c",
                extension: "gba",
                need_fullpath: false,
                pre_save_bytes: 128 * 1024,
                post_save_bytes: 128 * 1024,
                system_ram_bytes: 256 * 1024,
                system_ram_offset: 0,
                memory_map: Some(ExactMapping {
                    address: 0x0200_0000,
                    bytes: 256 * 1024,
                    select: 0xff00_0000,
                }),
                frame_limit: 12,
                first_observation: FIRST_SAVE_OBSERVATION,
                second_observation: SECOND_SAVE_OBSERVATION,
                state_bytes: Some(581_832),
            },
            Self::GbaVbam => CoreSpec {
                name: "VBA-M",
                version: "2.1.3 115defb",
                extension: "gba",
                need_fullpath: false,
                pre_save_bytes: 32 * 1024,
                post_save_bytes: 32 * 1024,
                system_ram_bytes: 256 * 1024,
                system_ram_offset: 0,
                memory_map: None,
                frame_limit: 12,
                first_observation: FIRST_SAVE_OBSERVATION,
                second_observation: SECOND_SAVE_OBSERVATION,
                state_bytes: Some(723_452),
            },
            Self::GameboyGambatte => CoreSpec {
                name: "Gambatte",
                version: "v0.5.0-netlink d9d6cd0",
                extension: "gb",
                need_fullpath: false,
                pre_save_bytes: 32 * 1024,
                post_save_bytes: 32 * 1024,
                system_ram_bytes: 8 * 1024,
                system_ram_offset: 0,
                memory_map: None,
                frame_limit: 240,
                first_observation: FIRST_SAVE_OBSERVATION,
                second_observation: SECOND_SAVE_OBSERVATION,
                state_bytes: Some(59_650),
            },
            Self::GameboyMgba => CoreSpec {
                name: "mGBA",
                version: "0.11-219-e31759b",
                extension: "gb",
                need_fullpath: false,
                pre_save_bytes: 32 * 1024,
                post_save_bytes: 32 * 1024,
                system_ram_bytes: 32 * 1024,
                system_ram_offset: 0,
                memory_map: None,
                frame_limit: 240,
                first_observation: FIRST_SAVE_OBSERVATION,
                second_observation: SECOND_SAVE_OBSERVATION,
                state_bytes: Some(202_816),
            },
            Self::GameboySameboy => CoreSpec {
                name: "SameBoy",
                version: "1.0.3 8230189",
                extension: "gb",
                need_fullpath: false,
                pre_save_bytes: 32 * 1024,
                post_save_bytes: 32 * 1024,
                system_ram_bytes: 8 * 1024,
                system_ram_offset: 0,
                memory_map: None,
                frame_limit: 240,
                first_observation: FIRST_SAVE_OBSERVATION,
                second_observation: SECOND_SAVE_OBSERVATION,
                state_bytes: Some(252_666),
            },
            Self::GameboySkyemu => CoreSpec {
                name: "SkyEmu",
                version: "adacd0788964ed89f5c43dcbc1f3cc26deec996c",
                extension: "gb",
                need_fullpath: false,
                pre_save_bytes: 128 * 1024,
                post_save_bytes: 128 * 1024,
                system_ram_bytes: 96 * 1024,
                system_ram_offset: 0xc000,
                memory_map: None,
                frame_limit: 240,
                first_observation: FIRST_SAVE_OBSERVATION,
                second_observation: SECOND_SAVE_OBSERVATION,
                state_bytes: Some(246_416),
            },
            Self::GameboyVbam => CoreSpec {
                name: "VBA-M",
                version: "2.1.3 115defb",
                extension: "gb",
                need_fullpath: false,
                pre_save_bytes: 32 * 1024,
                post_save_bytes: 32 * 1024,
                system_ram_bytes: 32 * 1024,
                system_ram_offset: 0,
                memory_map: None,
                frame_limit: 240,
                first_observation: FIRST_SAVE_OBSERVATION,
                second_observation: SECOND_SAVE_OBSERVATION,
                state_bytes: Some(115_948),
            },
            Self::Atari2600Stella => CoreSpec {
                name: "Stella",
                version: "8.0_pre c65c845",
                extension: "bin",
                need_fullpath: false,
                pre_save_bytes: 0,
                post_save_bytes: 0,
                system_ram_bytes: 128,
                system_ram_offset: 0,
                memory_map: None,
                frame_limit: 120,
                first_observation: b"LB26",
                second_observation: b"",
                state_bytes: Some(1_041),
            },
            Self::GameGear => CoreSpec {
                name: "Genesis Plus GX",
                version: "v1.7.4 c2838c7",
                extension: "gg",
                need_fullpath: true,
                pre_save_bytes: 64 * 1024,
                post_save_bytes: FIRST_SAVE_OBSERVATION.len(),
                system_ram_bytes: 8 * 1024,
                system_ram_offset: 0,
                memory_map: None,
                frame_limit: 12,
                first_observation: FIRST_SAVE_OBSERVATION,
                second_observation: SECOND_SAVE_OBSERVATION,
                state_bytes: Some(1_036_288),
            },
            Self::NesFceumm => CoreSpec {
                name: "FCEUmm",
                version: "(SVN) 5cd4a43",
                extension: "nes",
                need_fullpath: true,
                pre_save_bytes: 8 * 1024,
                post_save_bytes: 8 * 1024,
                system_ram_bytes: 2 * 1024,
                system_ram_offset: 0,
                memory_map: None,
                frame_limit: 12,
                first_observation: FIRST_NES_SAVE_OBSERVATION,
                second_observation: SECOND_NES_SAVE_OBSERVATION,
                state_bytes: Some(13_726),
            },
            Self::NesMesen => CoreSpec {
                name: "Mesen",
                version: "0.9.9",
                extension: "nes",
                need_fullpath: true,
                pre_save_bytes: 8 * 1024,
                post_save_bytes: 8 * 1024,
                system_ram_bytes: 2 * 1024,
                system_ram_offset: 0,
                memory_map: None,
                frame_limit: 12,
                first_observation: FIRST_NES_SAVE_OBSERVATION,
                second_observation: SECOND_NES_SAVE_OBSERVATION,
                state_bytes: Some(35_840),
            },
            Self::Snes9x => CoreSpec {
                name: "Snes9x",
                version: "1.63 185488c",
                extension: "sfc",
                need_fullpath: false,
                pre_save_bytes: 8 * 1024,
                post_save_bytes: 8 * 1024,
                system_ram_bytes: 128 * 1024,
                system_ram_offset: 0,
                memory_map: None,
                frame_limit: 12,
                first_observation: FIRST_SAVE_OBSERVATION,
                second_observation: SECOND_SAVE_OBSERVATION,
                state_bytes: Some(823_407),
            },
            Self::Bsnes => CoreSpec {
                name: "bsnes",
                version: "115",
                extension: "sfc",
                need_fullpath: true,
                pre_save_bytes: 8 * 1024,
                post_save_bytes: 8 * 1024,
                system_ram_bytes: 128 * 1024,
                system_ram_offset: 0,
                memory_map: None,
                frame_limit: 12,
                first_observation: FIRST_SAVE_OBSERVATION,
                second_observation: SECOND_SAVE_OBSERVATION,
                state_bytes: None,
            },
            Self::MesenS => CoreSpec {
                name: "Mesen-S",
                version: "0.4.0",
                extension: "sfc",
                need_fullpath: false,
                pre_save_bytes: 8 * 1024,
                post_save_bytes: 8 * 1024,
                system_ram_bytes: 128 * 1024,
                system_ram_offset: 0,
                memory_map: None,
                frame_limit: 12,
                first_observation: FIRST_SAVE_OBSERVATION,
                second_observation: SECOND_SAVE_OBSERVATION,
                state_bytes: Some(550_912),
            },
            Self::PsxBeetle => CoreSpec {
                name: "Beetle PSX",
                version: "0.9.44.1 82d8e05",
                extension: "exe",
                need_fullpath: true,
                pre_save_bytes: 128 * 1024,
                post_save_bytes: 128 * 1024,
                system_ram_bytes: 2 * 1024 * 1024,
                system_ram_offset: 0,
                memory_map: None,
                frame_limit: 12,
                first_observation: FIRST_SAVE_OBSERVATION,
                second_observation: SECOND_SAVE_OBSERVATION,
                state_bytes: Some(16_777_216),
            },
            Self::PsxBeetleHw => CoreSpec {
                name: "Beetle PSX HW",
                version: "0.9.44.1 82d8e05",
                extension: "exe",
                need_fullpath: true,
                pre_save_bytes: 128 * 1024,
                post_save_bytes: 128 * 1024,
                system_ram_bytes: 2 * 1024 * 1024,
                system_ram_offset: 0,
                memory_map: None,
                frame_limit: 12,
                first_observation: FIRST_SAVE_OBSERVATION,
                second_observation: SECOND_SAVE_OBSERVATION,
                state_bytes: Some(16_777_216),
            },
        }
    }

    fn slug(self) -> &'static str {
        match self {
            Self::Gba => "gba",
            Self::GbaSkyemu => "gba-skyemu",
            Self::GbaVbam => "gba-vbam",
            Self::GameboyGambatte => "gameboy-gambatte",
            Self::GameboyMgba => "gameboy-mgba",
            Self::GameboySameboy => "gameboy-sameboy",
            Self::GameboySkyemu => "gameboy-skyemu",
            Self::GameboyVbam => "gameboy-vbam",
            Self::Atari2600Stella => "atari2600-stella",
            Self::GameGear => "game-gear",
            Self::NesFceumm => "nes-fceumm",
            Self::NesMesen => "nes-mesen",
            Self::Snes9x => "snes9x",
            Self::Bsnes => "bsnes",
            Self::MesenS => "mesen-s",
            Self::PsxBeetle => "psx-beetle",
            Self::PsxBeetleHw => "psx-beetle-hw",
        }
    }

    fn diagnostic_rom(self) -> Vec<u8> {
        match self {
            Self::Gba | Self::GbaSkyemu | Self::GbaVbam => gba_persistence_rom(),
            Self::GameboyGambatte
            | Self::GameboyMgba
            | Self::GameboySameboy
            | Self::GameboySkyemu
            | Self::GameboyVbam => gameboy_persistence_rom(),
            Self::Atari2600Stella => {
                lunchbox_controller_probe::libretro_input::atari2600_diagnostic_rom()
            }
            Self::GameGear => game_gear_persistence_rom(),
            Self::NesFceumm | Self::NesMesen => nes_persistence_rom(),
            Self::Snes9x | Self::Bsnes | Self::MesenS => snes_persistence_rom(),
            Self::PsxBeetle | Self::PsxBeetleHw => {
                lunchbox_controller_probe::libretro_input::psx_diagnostic_exe()
            }
        }
    }

    fn is_psx(self) -> bool {
        matches!(self, Self::PsxBeetle | Self::PsxBeetleHw)
    }

    fn save_supported(self) -> bool {
        self != Self::Atari2600Stella
    }

    fn state_marker_offset(self) -> usize {
        if self == Self::Atari2600Stella {
            0x20
        } else {
            self.spec().system_ram_offset + MARKER_OFFSET
        }
    }

    fn state_ready_spec(self) -> CoreSpec {
        if self == Self::Atari2600Stella {
            CoreSpec {
                system_ram_offset: 4,
                ..self.spec()
            }
        } else {
            self.spec()
        }
    }

    fn save_marker_offset(self) -> usize {
        if self.is_psx() {
            PSX_MEMORY_CARD_MARKER_OFFSET
        } else {
            0
        }
    }

    fn save_observation_source(self) -> &'static str {
        if self.is_psx() {
            "frontend-libretro-memory-card-buffer"
        } else {
            "executed-diagnostic-program"
        }
    }

    fn core_option_overrides(self) -> BTreeMap<String, String> {
        if self == Self::PsxBeetleHw {
            BTreeMap::from([("beetle_psx_hw_renderer".into(), "software".into())])
        } else {
            BTreeMap::new()
        }
    }
}

#[derive(Clone, Copy)]
struct CoreSpec {
    name: &'static str,
    version: &'static str,
    extension: &'static str,
    need_fullpath: bool,
    pre_save_bytes: usize,
    post_save_bytes: usize,
    system_ram_bytes: usize,
    system_ram_offset: usize,
    memory_map: Option<ExactMapping>,
    frame_limit: usize,
    first_observation: &'static [u8],
    second_observation: &'static [u8],
    state_bytes: Option<usize>,
}

#[derive(Debug, Parser)]
#[command(about = "Verify save RAM and save states in an exact trusted libretro core")]
pub struct SupervisorArgs {
    /// Exact libretro core shared library to execute.
    #[arg(long)]
    core: PathBuf,
    /// Expected SHA-256 of the core file.
    #[arg(long)]
    sha256: String,
    #[arg(long, value_enum)]
    system: PersistenceSystem,
    /// Exact core version string. Defaults to the pinned Linux identity for the selected system.
    #[arg(long)]
    expected_version: Option<String>,
    /// Exact serialized-state size. Defaults to the pinned size for the selected system.
    #[arg(long)]
    expected_state_bytes: Option<usize>,
    /// New evidence directory. A private retained temporary directory is used when omitted.
    #[arg(long)]
    output: Option<PathBuf>,
    /// Wall-time limit applied independently to each of four worker processes.
    #[arg(long, default_value_t = 15, value_parser = clap::value_parser!(u64).range(1..=120))]
    timeout_seconds: u64,
    /// Trusted runtime dependency to load before the core, repeatable in dependency order.
    #[arg(long)]
    runtime_library: Vec<PathBuf>,
    /// Directory containing exact scph5500.bin, scph5501.bin, and scph5502.bin dumps.
    #[arg(long)]
    bios_dir: Option<PathBuf>,
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize, ValueEnum)]
#[serde(rename_all = "kebab-case")]
enum WorkerPhase {
    SaveCreate,
    SaveReload,
    StateCreate,
    StateReload,
}

impl WorkerPhase {
    fn as_cli_value(self) -> &'static str {
        match self {
            Self::SaveCreate => "save-create",
            Self::SaveReload => "save-reload",
            Self::StateCreate => "state-create",
            Self::StateReload => "state-reload",
        }
    }
}

#[derive(Debug, Parser)]
struct WorkerArgs {
    #[arg(long, value_enum)]
    phase: WorkerPhase,
    #[arg(long)]
    core: PathBuf,
    #[arg(long)]
    sha256: String,
    #[arg(long, value_enum)]
    system: PersistenceSystem,
    #[arg(long)]
    expected_version: String,
    #[arg(long)]
    expected_state_bytes: Option<usize>,
    #[arg(long)]
    evidence: PathBuf,
    #[arg(long)]
    runtime_library: Vec<PathBuf>,
    #[arg(long)]
    bios_dir: Option<PathBuf>,
    #[arg(long)]
    report: PathBuf,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
struct CoreIdentity {
    name: String,
    version: String,
    sha256: String,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
struct RuntimeLibrary {
    path: PathBuf,
    sha256: String,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
struct FirmwareIdentity {
    filename: String,
    bytes: usize,
    sha256: String,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
struct WorkerReport {
    schema_version: u32,
    phase: WorkerPhase,
    system: PersistenceSystem,
    core: CoreIdentity,
    expected_state_bytes: Option<usize>,
    runtime_libraries: Vec<RuntimeLibrary>,
    firmware: Vec<FirmwareIdentity>,
    core_option_overrides: BTreeMap<String, String>,
    diagnostic_rom_sha256: String,
    save_status: String,
    pre_save_bytes: Option<usize>,
    post_save_bytes: Option<usize>,
    reported_system_ram_bytes: usize,
    system_ram_source: String,
    system_ram_emulated_address: Option<usize>,
    system_ram_bytes: usize,
    system_ram_offset: usize,
    save_observation_source: String,
    save_marker_offset: usize,
    observation_hex: Option<String>,
    save_sha256: Option<String>,
    state_bytes: Option<usize>,
    state_sha256: Option<String>,
    before_marker_hex: Option<String>,
    restored_marker_hex: Option<String>,
}

#[derive(Debug, Serialize)]
struct PersistenceReport {
    schema_version: u32,
    status: &'static str,
    system: PersistenceSystem,
    core_path: PathBuf,
    core: CoreIdentity,
    expected_state_bytes: usize,
    runtime_libraries: Vec<RuntimeLibrary>,
    firmware: Vec<FirmwareIdentity>,
    core_option_overrides: BTreeMap<String, String>,
    evidence_directory: PathBuf,
    diagnostic_rom: Artifact,
    reported_system_ram_bytes: usize,
    system_ram_source: String,
    system_ram_emulated_address: Option<usize>,
    system_ram_bytes: usize,
    system_ram_offset: usize,
    save_ram: SaveRamReport,
    save_state: SaveStateReport,
}

#[derive(Debug, Serialize)]
struct Artifact {
    path: PathBuf,
    bytes: usize,
    sha256: String,
}

#[derive(Debug, Serialize)]
struct SaveRamReport {
    status: &'static str,
    #[serde(skip_serializing_if = "Option::is_none")]
    observation_source: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    marker_offset: Option<usize>,
    #[serde(skip_serializing_if = "Option::is_none")]
    initial_observation_hex: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    fresh_process_observation_hex: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    initial: Option<Artifact>,
    #[serde(skip_serializing_if = "Option::is_none")]
    after_reload: Option<Artifact>,
}

#[derive(Debug, Serialize)]
struct SaveStateReport {
    status: &'static str,
    expected_bytes: usize,
    same_process_restored_marker_hex: String,
    fresh_process_before_marker_hex: String,
    fresh_process_restored_marker_hex: String,
    artifact: Artifact,
}

#[repr(C)]
#[derive(Default)]
struct SystemInfo {
    name: *const c_char,
    version: *const c_char,
    extensions: *const c_char,
    need_fullpath: bool,
    block_extract: bool,
}

#[repr(C)]
struct GameInfo {
    path: *const c_char,
    data: *const c_void,
    size: usize,
    meta: *const c_char,
}

#[repr(C)]
struct Variable {
    key: *const c_char,
    value: *const c_char,
}

type Environment = unsafe extern "C" fn(u32, *mut c_void) -> bool;
type Input = unsafe extern "C" fn(u32, u32, u32, u32) -> i16;
type Video = unsafe extern "C" fn(*const c_void, u32, u32, usize);
type Audio = unsafe extern "C" fn(i16, i16);
type AudioBatch = unsafe extern "C" fn(*const i16, usize) -> usize;
type Poll = unsafe extern "C" fn();

unsafe extern "C" fn environment(command: u32, data: *mut c_void) -> bool {
    let command = command & !0x10000; // Strip the libretro experimental flag.
    if let Ok(mut options) = OPTIONS.lock()
        && let Some(options) = options.as_mut()
        && let Some(result) = unsafe { options.handle(command, data) }
        && (result || command != 15)
    {
        return result;
    }
    if command == 51 {
        return false; // No content-less core support.
    }
    if data.is_null() {
        return false;
    }
    match command {
        3 => {
            unsafe { data.cast::<bool>().write(true) };
            true
        }
        9 | 31 => {
            let slot = if command == 9 {
                &SYSTEM_DIRECTORY
            } else {
                &SAVE_DIRECTORY
            };
            let Ok(slot) = slot.lock() else {
                return false;
            };
            let Some(directory) = slot.as_ref() else {
                return false;
            };
            unsafe { data.cast::<*const c_char>().write(directory.as_ptr()) };
            true
        }
        10 => unsafe { *data.cast::<u32>() <= 2 },
        36 => {
            let captured = unsafe { MemoryMapSnapshot::capture(data.cast_const()) }
                .map(Some)
                .map_err(|error| error.to_string());
            let valid = captured.is_ok();
            let Ok(mut slot) = MEMORY_MAP.lock() else {
                return false;
            };
            *slot = captured;
            valid
        }
        11 | 35 | 37 => true,
        15 => {
            let variable = unsafe { &mut *data.cast::<Variable>() };
            if variable.key.is_null() {
                return false;
            }
            let key = unsafe { CStr::from_ptr(variable.key) }.to_bytes();
            variable.value = match key {
                b"mgba_use_bios" => c"OFF".as_ptr(),
                b"mgba_skip_bios" => c"ON".as_ptr(),
                b"sameboy_model" => c"Game Boy".as_ptr(),
                b"system_core_override" => c"Automatic".as_ptr(),
                b"system_gb_bios_enable"
                | b"system_gba_bios_enable"
                | b"system_nds_bios_enable" => c"OFF".as_ptr(),
                _ => std::ptr::null(),
            };
            !variable.value.is_null()
        }
        24 => {
            unsafe { data.cast::<u64>().write(1 << 1) };
            true
        }
        27 => unsafe { lunchbox_controller_probe::libretro_log::install(data) },
        39 => {
            unsafe { data.cast::<u32>().write(0) };
            true
        }
        47 => {
            unsafe { data.cast::<i32>().write(3) };
            true
        }
        _ if METADATA_VFS_ENABLED.load(Ordering::Relaxed) => {
            unsafe { lunchbox_controller_probe::libretro_vfs::handle_environment(command, data) }
                .unwrap_or(false)
        }
        _ => false,
    }
}

unsafe extern "C" fn video(_: *const c_void, _: u32, _: u32, _: usize) {}
unsafe extern "C" fn audio(_: i16, _: i16) {}
unsafe extern "C" fn audio_batch(_: *const i16, frames: usize) -> usize {
    frames
}
unsafe extern "C" fn poll() {}
unsafe extern "C" fn input(_: u32, _: u32, _: u32, _: u32) -> i16 {
    0
}

struct Core {
    library: Library,
    _runtime_dependencies: Vec<Library>,
    deinit: unsafe extern "C" fn(),
    unload: unsafe extern "C" fn(),
    run: unsafe extern "C" fn(),
    memory: unsafe extern "C" fn(u32) -> *mut c_void,
    memory_size: unsafe extern "C" fn(u32) -> usize,
    serialize_size: unsafe extern "C" fn() -> usize,
    serialize: unsafe extern "C" fn(*mut c_void, usize) -> bool,
    unserialize: unsafe extern "C" fn(*const c_void, usize) -> bool,
    initialized: bool,
    loaded: bool,
    identity: CoreIdentity,
}

impl Drop for Core {
    fn drop(&mut self) {
        unsafe {
            if self.loaded {
                (self.unload)();
            }
            if self.initialized {
                (self.deinit)();
            }
        }
        if let Ok(mut value) = SYSTEM_DIRECTORY.lock() {
            *value = None;
        }
        if let Ok(mut value) = SAVE_DIRECTORY.lock() {
            *value = None;
        }
        if let Ok(mut value) = OPTIONS.lock() {
            *value = None;
        }
        if let Ok(mut value) = MEMORY_MAP.lock() {
            *value = Ok(None);
        }
        METADATA_VFS_ENABLED.store(false, Ordering::Relaxed);
    }
}

impl Core {
    #[allow(clippy::too_many_arguments)]
    unsafe fn load(
        core_path: &Path,
        expected_hash: &str,
        expected_version: &str,
        system: PersistenceSystem,
        content_path: &Path,
        content: &[u8],
        root: &Path,
        system_directory: Option<&Path>,
        runtime_library_paths: &[PathBuf],
    ) -> Result<Self> {
        let spec = system.spec();
        ensure!(
            file_hash(core_path)?.eq_ignore_ascii_case(expected_hash),
            "Core hash does not match --sha256"
        );
        for directory in [
            "system",
            "saves",
            "states",
            "xdg-config",
            "xdg-cache",
            "xdg-data",
        ] {
            std::fs::create_dir_all(root.join(directory))?;
        }
        let system_directory = system_directory
            .map(Path::to_path_buf)
            .unwrap_or_else(|| root.join("system"))
            .canonicalize()?;
        ensure!(
            system_directory.is_dir(),
            "System directory is not a directory"
        );
        *SYSTEM_DIRECTORY
            .lock()
            .map_err(|_| anyhow::anyhow!("System-directory callback lock poisoned"))? =
            Some(path_c_string(&system_directory)?);
        *SAVE_DIRECTORY
            .lock()
            .map_err(|_| anyhow::anyhow!("Save-directory callback lock poisoned"))? =
            Some(path_c_string(&root.join("saves"))?);
        *OPTIONS
            .lock()
            .map_err(|_| anyhow::anyhow!("Core-option callback lock poisoned"))? =
            Some(OptionEnvironment::new(system.core_option_overrides())?);
        *MEMORY_MAP
            .lock()
            .map_err(|_| anyhow::anyhow!("Memory-map capture lock poisoned"))? = Ok(None);
        METADATA_VFS_ENABLED.store(
            system == PersistenceSystem::Atari2600Stella,
            Ordering::Relaxed,
        );

        let mut runtime_dependencies = Vec::new();
        for path in runtime_library_paths {
            runtime_dependencies.push(load_runtime_dependency(path)?);
        }
        let library = unsafe { Library::new(core_path) }
            .with_context(|| format!("Loading trusted core {}", core_path.display()))?;
        let api_version =
            unsafe { *library.get::<unsafe extern "C" fn() -> u32>(b"retro_api_version\0")? };
        ensure!(
            unsafe { api_version() } == 1,
            "Unsupported libretro API version"
        );
        let get_system_info = unsafe {
            *library.get::<unsafe extern "C" fn(*mut SystemInfo)>(b"retro_get_system_info\0")?
        };
        let mut info = SystemInfo::default();
        unsafe { get_system_info(&mut info) };
        ensure!(
            !info.name.is_null() && !info.version.is_null() && !info.extensions.is_null(),
            "Core returned incomplete identity"
        );
        let name = unsafe { CStr::from_ptr(info.name) }.to_str()?.to_owned();
        let version = unsafe { CStr::from_ptr(info.version) }.to_str()?.to_owned();
        let extensions = unsafe { CStr::from_ptr(info.extensions) }.to_str()?;
        ensure!(name == spec.name, "Expected core {}, got {name}", spec.name);
        ensure!(
            version == expected_version,
            "Expected core version {expected_version}, got {version}"
        );
        ensure!(
            extensions.split('|').any(|value| value == spec.extension),
            "Core does not advertise .{} content",
            spec.extension
        );
        ensure!(
            info.need_fullpath == spec.need_fullpath,
            "Core full-path contract changed"
        );

        let init = unsafe { *library.get::<unsafe extern "C" fn()>(b"retro_init\0")? };
        let load_game = unsafe {
            *library.get::<unsafe extern "C" fn(*const GameInfo) -> bool>(b"retro_load_game\0")?
        };
        let set_environment = unsafe {
            *library.get::<unsafe extern "C" fn(Environment)>(b"retro_set_environment\0")?
        };
        let mut core = Self {
            _runtime_dependencies: runtime_dependencies,
            deinit: unsafe { *library.get(b"retro_deinit\0")? },
            unload: unsafe { *library.get(b"retro_unload_game\0")? },
            run: unsafe { *library.get(b"retro_run\0")? },
            memory: unsafe { *library.get(b"retro_get_memory_data\0")? },
            memory_size: unsafe { *library.get(b"retro_get_memory_size\0")? },
            serialize_size: unsafe { *library.get(b"retro_serialize_size\0")? },
            serialize: unsafe { *library.get(b"retro_serialize\0")? },
            unserialize: unsafe { *library.get(b"retro_unserialize\0")? },
            library,
            initialized: false,
            loaded: false,
            identity: CoreIdentity {
                name,
                version,
                sha256: expected_hash.to_ascii_lowercase(),
            },
        };
        unsafe {
            set_environment(environment);
            init();
        }
        core.initialized = true;
        unsafe {
            core.library
                .get::<unsafe extern "C" fn(Video)>(b"retro_set_video_refresh\0")?(
                video
            );
            core.library
                .get::<unsafe extern "C" fn(Audio)>(b"retro_set_audio_sample\0")?(audio);
            core.library
                .get::<unsafe extern "C" fn(AudioBatch)>(b"retro_set_audio_sample_batch\0")?(
                audio_batch,
            );
            core.library
                .get::<unsafe extern "C" fn(Poll)>(b"retro_set_input_poll\0")?(poll);
            core.library
                .get::<unsafe extern "C" fn(Input)>(b"retro_set_input_state\0")?(input);
        }
        let path = path_c_string(content_path)?;
        let game = GameInfo {
            path: path.as_ptr(),
            data: if info.need_fullpath {
                std::ptr::null()
            } else {
                content.as_ptr().cast()
            },
            size: if info.need_fullpath { 0 } else { content.len() },
            meta: std::ptr::null(),
        };
        ensure!(
            unsafe { load_game(&game) },
            "Core rejected diagnostic content"
        );
        core.loaded = true;
        OPTIONS
            .lock()
            .map_err(|_| anyhow::anyhow!("Core-option callback lock poisoned"))?
            .as_ref()
            .context("Core-option callback state disappeared")?
            .effective_values()?;
        Ok(core)
    }

    fn memory_size(&self, id: u32) -> usize {
        unsafe { (self.memory_size)(id) }
    }

    unsafe fn memory_slice(&self, id: u32) -> Result<&[u8]> {
        let size = self.memory_size(id);
        let data = unsafe { (self.memory)(id) };
        ensure!(
            size > 0 && !data.is_null(),
            "Core memory {id} is unavailable"
        );
        Ok(unsafe { std::slice::from_raw_parts(data.cast(), size) })
    }

    unsafe fn memory_slice_mut(&mut self, id: u32) -> Result<&mut [u8]> {
        let size = self.memory_size(id);
        let data = unsafe { (self.memory)(id) };
        ensure!(
            size > 0 && !data.is_null(),
            "Core memory {id} is unavailable"
        );
        Ok(unsafe { std::slice::from_raw_parts_mut(data.cast(), size) })
    }

    unsafe fn observation_memory_slice(&self, spec: CoreSpec) -> Result<&[u8]> {
        if let Some(expected) = spec.memory_map {
            ensure!(
                self.memory_size(RETRO_MEMORY_SYSTEM_RAM) == 0
                    && unsafe { (self.memory)(RETRO_MEMORY_SYSTEM_RAM) }.is_null(),
                "Mapped-memory core unexpectedly exposed RETRO_MEMORY_SYSTEM_RAM; re-audit its exact contract"
            );
            let snapshot = MEMORY_MAP
                .lock()
                .map_err(|_| anyhow::anyhow!("Memory-map capture lock poisoned"))?
                .clone()
                .map_err(anyhow::Error::msg)?
                .context("Core did not publish a memory map")?;
            let region = snapshot.exact_writable_region(expected)?;
            Ok(unsafe { std::slice::from_raw_parts(region.pointer(), region.bytes()) })
        } else {
            unsafe { self.memory_slice(RETRO_MEMORY_SYSTEM_RAM) }
        }
    }

    unsafe fn observation_memory_slice_mut(&mut self, spec: CoreSpec) -> Result<&mut [u8]> {
        if let Some(expected) = spec.memory_map {
            ensure!(
                self.memory_size(RETRO_MEMORY_SYSTEM_RAM) == 0
                    && unsafe { (self.memory)(RETRO_MEMORY_SYSTEM_RAM) }.is_null(),
                "Mapped-memory core unexpectedly exposed RETRO_MEMORY_SYSTEM_RAM; re-audit its exact contract"
            );
            let snapshot = MEMORY_MAP
                .lock()
                .map_err(|_| anyhow::anyhow!("Memory-map capture lock poisoned"))?
                .clone()
                .map_err(anyhow::Error::msg)?
                .context("Core did not publish a memory map")?;
            let region = snapshot.exact_writable_region(expected)?;
            Ok(unsafe { std::slice::from_raw_parts_mut(region.pointer(), region.bytes()) })
        } else {
            unsafe { self.memory_slice_mut(RETRO_MEMORY_SYSTEM_RAM) }
        }
    }

    fn run_until_observation(
        &self,
        expected: &[u8],
        spec: CoreSpec,
        frame_limit: usize,
    ) -> Result<Vec<u8>> {
        let mut observed = Vec::new();
        for _ in 0..frame_limit {
            unsafe { (self.run)() };
            let memory = unsafe { self.observation_memory_slice(spec)? };
            ensure!(
                memory.len() >= spec.system_ram_offset + expected.len(),
                "Observation RAM is too small for offset {}",
                spec.system_ram_offset
            );
            observed =
                memory[spec.system_ram_offset..spec.system_ram_offset + expected.len()].to_vec();
            if observed == expected {
                return Ok(observed);
            }
        }
        bail!(
            "Diagnostic program did not publish {} at observation-RAM offset {} in {frame_limit} frames (observed {})",
            bytes_hex(expected),
            spec.system_ram_offset,
            bytes_hex(&observed)
        )
    }
}

fn load_runtime_dependency(path: &Path) -> Result<Library> {
    #[cfg(unix)]
    {
        let library = unsafe {
            libloading::os::unix::Library::open(Some(path), libc::RTLD_NOW | libc::RTLD_GLOBAL)
        }
        .with_context(|| format!("Loading trusted runtime dependency {}", path.display()))?;
        Ok(library.into())
    }
    #[cfg(not(unix))]
    {
        unsafe { Library::new(path) }
            .with_context(|| format!("Loading trusted runtime dependency {}", path.display()))
    }
}

fn path_c_string(path: &Path) -> Result<CString> {
    CString::new(
        path.to_str()
            .with_context(|| format!("Non-UTF-8 path: {}", path.display()))?,
    )
    .context("Path contains a NUL byte")
}

fn bytes_hex(bytes: &[u8]) -> String {
    bytes.iter().map(|value| format!("{value:02x}")).collect()
}

fn validate_hash(hash: &str) -> Result<String> {
    ensure!(
        hash.len() == 64 && hash.bytes().all(|byte| byte.is_ascii_hexdigit()),
        "--sha256 must contain exactly 64 hexadecimal digits"
    );
    Ok(hash.to_ascii_lowercase())
}

fn resolve_expected_version(
    system: PersistenceSystem,
    override_value: Option<&str>,
) -> Result<String> {
    let version = override_value.unwrap_or(system.spec().version);
    ensure!(
        !version.is_empty() && version.len() <= 256,
        "--expected-version must contain between 1 and 256 bytes"
    );
    Ok(version.to_owned())
}

fn resolve_expected_state_bytes(
    system: PersistenceSystem,
    override_value: Option<usize>,
) -> Result<Option<usize>> {
    let bytes = override_value.or(system.spec().state_bytes);
    if let Some(bytes) = bytes {
        ensure!(
            (1..=32 * 1024 * 1024).contains(&bytes),
            "--expected-state-bytes must be between 1 and 33554432"
        );
    }
    Ok(bytes)
}

fn runtime_inventory(paths: &[PathBuf]) -> Result<(Vec<PathBuf>, Vec<RuntimeLibrary>)> {
    let mut canonical = Vec::new();
    let mut inventory = Vec::new();
    for path in paths {
        ensure!(path.is_absolute(), "Runtime library path must be absolute");
        let path = path
            .canonicalize()
            .with_context(|| format!("Canonicalizing runtime library {}", path.display()))?;
        ensure!(
            !canonical.contains(&path),
            "Duplicate runtime library {}",
            path.display()
        );
        inventory.push(RuntimeLibrary {
            sha256: file_hash(&path)?,
            path: path.clone(),
        });
        canonical.push(path);
    }
    Ok((canonical, inventory))
}

fn psx_firmware_inventory(directory: &Path) -> Result<(PathBuf, Vec<FirmwareIdentity>)> {
    let directory = directory
        .canonicalize()
        .context("Canonicalizing PSX BIOS directory")?;
    ensure!(directory.is_dir(), "PSX BIOS path is not a directory");
    let expected = [
        (
            "scph5500.bin",
            "9c0421858e217805f4abe18698afea8d5aa36ff0727eb8484944e00eb5e7eadb",
        ),
        (
            "scph5501.bin",
            "11052b6499e466bbf0a709b1f9cb6834a9418e66680387912451e971cf8a1fef",
        ),
        (
            "scph5502.bin",
            "1faaa18fa820a0225e488d9f086296b8e6c46df739666093987ff7d8fd352c09",
        ),
    ];
    let mut firmware = Vec::new();
    for (filename, expected_hash) in expected {
        let path = directory.join(filename);
        let metadata = std::fs::metadata(&path)
            .with_context(|| format!("Reading required BIOS {}", path.display()))?;
        ensure!(metadata.is_file(), "{filename} is not a regular file");
        let bytes = usize::try_from(metadata.len()).context("PSX BIOS file is too large")?;
        ensure!(bytes == 512 * 1024, "{filename} is not exactly 512 KiB");
        let sha256 = file_hash(&path)
            .with_context(|| format!("Hashing required BIOS {}", path.display()))?;
        ensure!(
            sha256 == expected_hash,
            "{filename} SHA-256 is not the pinned dump"
        );
        firmware.push(FirmwareIdentity {
            filename: filename.into(),
            bytes,
            sha256,
        });
    }
    Ok((directory, firmware))
}

fn diagnostic_path(root: &Path, system: PersistenceSystem) -> PathBuf {
    root.join(format!(
        "persistence-diagnostic.{}",
        system.spec().extension
    ))
}

fn save_path(root: &Path, system: PersistenceSystem, after_reload: bool) -> PathBuf {
    let suffix = if after_reload { "-after-reload" } else { "" };
    root.join("saves")
        .join(format!("{}-persistence{suffix}.srm", system.slug()))
}

fn state_path(root: &Path, system: PersistenceSystem) -> PathBuf {
    root.join("states")
        .join(format!("{}-persistence.state", system.slug()))
}

fn pre_run_observation_bytes(
    spec: CoreSpec,
    reported_system_ram_bytes: usize,
    mapped_bytes: impl FnOnce() -> Result<usize>,
) -> Result<usize> {
    if spec.memory_map.is_some() {
        mapped_bytes()
    } else {
        // Some cores publish the standard memory size during load but leave its
        // pointer null until the first retro_run. The phase-specific paths run
        // a frame before dereferencing observation RAM, matching that ABI
        // lifecycle while still pinning the advertised size here.
        Ok(reported_system_ram_bytes)
    }
}

fn applied_core_option_overrides(system: PersistenceSystem) -> Result<BTreeMap<String, String>> {
    let requested = system.core_option_overrides();
    let options = OPTIONS
        .lock()
        .map_err(|_| anyhow::anyhow!("Core-option callback lock poisoned"))?;
    let effective = options
        .as_ref()
        .context("Core-option environment is unavailable")?
        .effective_values()?;
    requested
        .into_iter()
        .map(|(key, expected)| {
            let observed = effective
                .get(&key)
                .with_context(|| format!("Requested core option was not registered: {key}"))?;
            ensure!(
                observed == &expected,
                "Core option {key} resolved to {observed}, expected {expected}"
            );
            Ok((key, expected))
        })
        .collect()
}

fn worker(args: WorkerArgs) -> Result<WorkerReport> {
    let expected_hash = validate_hash(&args.sha256)?;
    let expected_version = resolve_expected_version(args.system, Some(&args.expected_version))?;
    let expected_state_bytes =
        resolve_expected_state_bytes(args.system, args.expected_state_bytes)?;
    ensure!(
        args.evidence.is_absolute(),
        "Worker evidence path is not absolute"
    );
    let (runtime_paths, runtime_libraries) = runtime_inventory(&args.runtime_library)?;
    let spec = args.system.spec();
    let (system_directory, firmware) = if args.system.is_psx() {
        let directory = args
            .bios_dir
            .as_deref()
            .context("PSX persistence requires --bios-dir")?;
        let (directory, firmware) = psx_firmware_inventory(directory)?;
        (Some(directory), firmware)
    } else {
        ensure!(
            args.bios_dir.is_none(),
            "--bios-dir is only valid for PSX persistence"
        );
        (None, Vec::new())
    };
    let rom = args.system.diagnostic_rom();
    let rom_path = diagnostic_path(&args.evidence, args.system);
    std::fs::create_dir_all(&args.evidence)?;
    std::fs::write(&rom_path, &rom)?;
    let rom_hash = file_hash(&rom_path)?;
    let mut core = unsafe {
        Core::load(
            &args.core,
            &expected_hash,
            &expected_version,
            args.system,
            &rom_path,
            &rom,
            &args.evidence,
            system_directory.as_deref(),
            &runtime_paths,
        )?
    };
    let core_option_overrides = applied_core_option_overrides(args.system)?;
    if args.system == PersistenceSystem::Bsnes {
        let save_pointer = unsafe { (core.memory)(RETRO_MEMORY_SAVE_RAM) };
        let system_pointer = unsafe { (core.memory)(RETRO_MEMORY_SYSTEM_RAM) };
        ensure!(
            core.memory_size(RETRO_MEMORY_SAVE_RAM) == 0
                && save_pointer.is_null()
                && core.memory_size(RETRO_MEMORY_SYSTEM_RAM) == 0
                && system_pointer.is_null(),
            "bsnes memory-interface behavior changed; re-audit before enabling this oracle"
        );
        bail!(
            "bsnes 115 deliberately exposes neither save RAM nor system RAM through the libretro memory interface (null pointers and zero sizes); this direct-memory oracle cannot observe save reload or state restoration"
        );
    }
    let reported_system_ram_bytes = core.memory_size(RETRO_MEMORY_SYSTEM_RAM);
    let system_ram_bytes = pre_run_observation_bytes(spec, reported_system_ram_bytes, || {
        Ok(unsafe { core.observation_memory_slice(spec)? }.len())
    })?;
    ensure!(
        system_ram_bytes == spec.system_ram_bytes,
        "Expected {} bytes of observation RAM, got {system_ram_bytes}",
        spec.system_ram_bytes
    );
    let marker_offset = args.system.state_marker_offset();
    ensure!(
        system_ram_bytes >= marker_offset + STATE_MARKER.len(),
        "System RAM is too small for the configured diagnostic window"
    );
    let mut report = WorkerReport {
        schema_version: REPORT_SCHEMA,
        phase: args.phase,
        system: args.system,
        core: core.identity.clone(),
        expected_state_bytes,
        runtime_libraries,
        firmware,
        core_option_overrides,
        diagnostic_rom_sha256: rom_hash,
        save_status: if args.system.save_supported() {
            "pass".into()
        } else {
            "not_applicable".into()
        },
        pre_save_bytes: None,
        post_save_bytes: None,
        reported_system_ram_bytes,
        system_ram_source: if spec.memory_map.is_some() {
            "retro-environment-memory-map".into()
        } else {
            "retro-memory-system-ram".into()
        },
        system_ram_emulated_address: spec.memory_map.map(|mapping| mapping.address),
        system_ram_bytes,
        system_ram_offset: spec.system_ram_offset,
        save_observation_source: args.system.save_observation_source().into(),
        save_marker_offset: args.system.save_marker_offset(),
        observation_hex: None,
        save_sha256: None,
        state_bytes: None,
        state_sha256: None,
        before_marker_hex: None,
        restored_marker_hex: None,
    };

    match args.phase {
        WorkerPhase::SaveCreate => {
            if !args.system.save_supported() {
                ensure!(
                    core.memory_size(RETRO_MEMORY_SAVE_RAM) == 0
                        && unsafe { (core.memory)(RETRO_MEMORY_SAVE_RAM) }.is_null(),
                    "State-only core unexpectedly exposed save RAM"
                );
                report.pre_save_bytes = Some(0);
                report.post_save_bytes = Some(0);
                return Ok(report);
            }
            let pre_size = core.memory_size(RETRO_MEMORY_SAVE_RAM);
            ensure!(
                pre_size == spec.pre_save_bytes,
                "Expected {} pre-run save bytes, got {pre_size}",
                spec.pre_save_bytes
            );
            let observed = if args.system.is_psx() {
                unsafe { (core.run)() };
                let offset = args.system.save_marker_offset();
                let save = unsafe { core.memory_slice_mut(RETRO_MEMORY_SAVE_RAM)? };
                ensure!(
                    offset + spec.first_observation.len() <= save.len(),
                    "PSX memory-card marker is outside save memory"
                );
                save[offset..offset + spec.first_observation.len()]
                    .copy_from_slice(spec.first_observation);
                save[offset..offset + spec.first_observation.len()].to_vec()
            } else {
                core.run_until_observation(spec.first_observation, spec, spec.frame_limit)?
            };
            let post_size = core.memory_size(RETRO_MEMORY_SAVE_RAM);
            ensure!(
                post_size == spec.post_save_bytes,
                "Expected {} post-run save bytes, got {post_size}",
                spec.post_save_bytes
            );
            let save = unsafe { core.memory_slice(RETRO_MEMORY_SAVE_RAM)? }.to_vec();
            let offset = args.system.save_marker_offset();
            ensure!(
                save.get(offset..offset + spec.first_observation.len())
                    == Some(spec.first_observation),
                "Save buffer does not contain the expected observation"
            );
            let path = save_path(&args.evidence, args.system, false);
            std::fs::write(&path, save)?;
            report.pre_save_bytes = Some(pre_size);
            report.post_save_bytes = Some(post_size);
            report.observation_hex = Some(bytes_hex(&observed));
            report.save_sha256 = Some(file_hash(&path)?);
        }
        WorkerPhase::SaveReload => {
            if !args.system.save_supported() {
                ensure!(
                    core.memory_size(RETRO_MEMORY_SAVE_RAM) == 0
                        && unsafe { (core.memory)(RETRO_MEMORY_SAVE_RAM) }.is_null(),
                    "State-only core unexpectedly exposed save RAM"
                );
                report.pre_save_bytes = Some(0);
                report.post_save_bytes = Some(0);
                return Ok(report);
            }
            let path = save_path(&args.evidence, args.system, false);
            let save = std::fs::read(&path)
                .with_context(|| format!("Reading prior save {}", path.display()))?;
            ensure!(
                save.len() == spec.post_save_bytes,
                "Persisted save has the wrong length"
            );
            let pre_size = core.memory_size(RETRO_MEMORY_SAVE_RAM);
            ensure!(
                pre_size == spec.pre_save_bytes,
                "Expected {} pre-run save bytes, got {pre_size}",
                spec.pre_save_bytes
            );
            let target = unsafe { core.memory_slice_mut(RETRO_MEMORY_SAVE_RAM)? };
            target.fill(0xff);
            target[..save.len()].copy_from_slice(&save);
            let observed = if args.system.is_psx() {
                unsafe { (core.run)() };
                let offset = args.system.save_marker_offset();
                let target = unsafe { core.memory_slice_mut(RETRO_MEMORY_SAVE_RAM)? };
                ensure!(
                    target.get(offset..offset + spec.first_observation.len())
                        == Some(spec.first_observation),
                    "Fresh PSX process did not receive the persisted memory-card marker"
                );
                target[offset..offset + spec.second_observation.len()]
                    .copy_from_slice(spec.second_observation);
                unsafe { (core.run)() };
                let target = unsafe { core.memory_slice(RETRO_MEMORY_SAVE_RAM)? };
                target[offset..offset + spec.second_observation.len()].to_vec()
            } else {
                core.run_until_observation(spec.second_observation, spec, spec.frame_limit)?
            };
            let post_size = core.memory_size(RETRO_MEMORY_SAVE_RAM);
            ensure!(
                post_size == spec.post_save_bytes,
                "Expected {} post-reload save bytes, got {post_size}",
                spec.post_save_bytes
            );
            let after = unsafe { core.memory_slice(RETRO_MEMORY_SAVE_RAM)? }.to_vec();
            let offset = args.system.save_marker_offset();
            ensure!(
                after.get(offset..offset + spec.second_observation.len())
                    == Some(spec.second_observation),
                "Reloaded save buffer does not contain the second observation"
            );
            let after_path = save_path(&args.evidence, args.system, true);
            std::fs::write(&after_path, after)?;
            report.pre_save_bytes = Some(pre_size);
            report.post_save_bytes = Some(post_size);
            report.observation_hex = Some(bytes_hex(&observed));
            report.save_sha256 = Some(file_hash(&after_path)?);
        }
        WorkerPhase::StateCreate => {
            let expected_state_bytes = expected_state_bytes
                .context("No pinned state size; supply --expected-state-bytes for this system")?;
            if args.system.is_psx() {
                unsafe { (core.run)() };
            } else {
                let ready_spec = args.system.state_ready_spec();
                core.run_until_observation(
                    ready_spec.first_observation,
                    ready_spec,
                    ready_spec.frame_limit,
                )?;
            }
            let ram = unsafe { core.observation_memory_slice_mut(spec)? };
            ensure!(
                ram.len() >= marker_offset + STATE_MARKER.len(),
                "System RAM is too small"
            );
            ram[marker_offset..marker_offset + STATE_MARKER.len()].copy_from_slice(STATE_MARKER);
            // `RETRO_MEMORY_SYSTEM_RAM` may be a frontend-facing mirror rather
            // than the core's internal allocation. Run a frame and require the
            // marker to survive before serializing so the mutation is known to
            // have entered emulated state.
            unsafe { (core.run)() };
            ensure!(
                unsafe { core.observation_memory_slice(spec)? }
                    [marker_offset..marker_offset + STATE_MARKER.len()]
                    == *STATE_MARKER,
                "Core did not retain the state marker through an emulated frame"
            );
            let state_size = unsafe { (core.serialize_size)() };
            ensure!(
                state_size > 0 && state_size <= 32 * 1024 * 1024,
                "Invalid state size"
            );
            ensure!(
                state_size == expected_state_bytes,
                "Expected {expected_state_bytes} serialized bytes, got {state_size}"
            );
            let mut state = vec![0; state_size];
            ensure!(
                unsafe { (core.serialize)(state.as_mut_ptr().cast(), state.len()) },
                "Core refused state serialization"
            );
            let ram = unsafe { core.observation_memory_slice_mut(spec)? };
            ram[marker_offset..marker_offset + MUTATED_MARKER.len()]
                .copy_from_slice(MUTATED_MARKER);
            unsafe { (core.run)() };
            ensure!(
                unsafe { core.observation_memory_slice(spec)? }
                    [marker_offset..marker_offset + MUTATED_MARKER.len()]
                    == *MUTATED_MARKER,
                "Core did not retain the mutated marker through an emulated frame"
            );
            ensure!(
                unsafe { (core.unserialize)(state.as_ptr().cast(), state.len()) },
                "Core refused same-process state restoration"
            );
            let restored = unsafe { core.observation_memory_slice(spec)? }
                [marker_offset..marker_offset + STATE_MARKER.len()]
                .to_vec();
            ensure!(
                restored == STATE_MARKER,
                "Same-process state did not restore RAM"
            );
            let path = state_path(&args.evidence, args.system);
            std::fs::write(&path, state)?;
            report.state_bytes = Some(state_size);
            report.state_sha256 = Some(file_hash(&path)?);
            report.restored_marker_hex = Some(bytes_hex(&restored));
        }
        WorkerPhase::StateReload => {
            let expected_state_bytes = expected_state_bytes
                .context("No pinned state size; supply --expected-state-bytes for this system")?;
            if args.system.is_psx() {
                unsafe { (core.run)() };
            } else {
                let ready_spec = args.system.state_ready_spec();
                core.run_until_observation(
                    ready_spec.first_observation,
                    ready_spec,
                    ready_spec.frame_limit,
                )?;
            }
            let before = unsafe { core.observation_memory_slice(spec)? }
                [marker_offset..marker_offset + STATE_MARKER.len()]
                .to_vec();
            ensure!(
                before != STATE_MARKER,
                "Fresh process unexpectedly contains state marker"
            );
            let path = state_path(&args.evidence, args.system);
            let state = std::fs::read(&path)
                .with_context(|| format!("Reading prior state {}", path.display()))?;
            ensure!(
                state.len() == expected_state_bytes,
                "Expected {expected_state_bytes} persisted state bytes, got {}",
                state.len()
            );
            ensure!(
                unsafe { (core.serialize_size)() } == state.len(),
                "Fresh core reports a different serialization size"
            );
            ensure!(
                unsafe { (core.unserialize)(state.as_ptr().cast(), state.len()) },
                "Fresh core refused persisted state"
            );
            let restored = unsafe { core.observation_memory_slice(spec)? }
                [marker_offset..marker_offset + STATE_MARKER.len()]
                .to_vec();
            ensure!(
                restored == STATE_MARKER,
                "Fresh-process state did not restore RAM"
            );
            report.state_bytes = Some(state.len());
            report.state_sha256 = Some(file_hash(&path)?);
            report.before_marker_hex = Some(bytes_hex(&before));
            report.restored_marker_hex = Some(bytes_hex(&restored));
        }
    }
    Ok(report)
}

struct Capture {
    bytes: Vec<u8>,
    truncated: bool,
}

fn drain_bounded(
    mut reader: impl Read + Send + 'static,
) -> std::thread::JoinHandle<std::io::Result<Capture>> {
    std::thread::spawn(move || {
        let mut capture = Capture {
            bytes: Vec::new(),
            truncated: false,
        };
        let mut chunk = [0u8; 8192];
        loop {
            let count = reader.read(&mut chunk)?;
            if count == 0 {
                return Ok(capture);
            }
            let remaining = CAPTURE_LIMIT.saturating_sub(capture.bytes.len());
            capture
                .bytes
                .extend_from_slice(&chunk[..count.min(remaining)]);
            capture.truncated |= count > remaining;
        }
    })
}

#[allow(clippy::too_many_arguments)]
fn run_worker(
    executable: &Path,
    args: &SupervisorArgs,
    core: &Path,
    hash: &str,
    expected_version: &str,
    expected_state_bytes: Option<usize>,
    evidence: &Path,
    phase: WorkerPhase,
) -> Result<WorkerReport> {
    let report_path = evidence
        .join("workers")
        .join(format!("{}.json", phase.as_cli_value()));
    let mut command = Command::new(executable);
    command
        .arg("__worker")
        .arg("--phase")
        .arg(phase.as_cli_value())
        .arg("--core")
        .arg(core)
        .arg("--sha256")
        .arg(hash)
        .arg("--system")
        .arg(match args.system {
            PersistenceSystem::Gba => "gba",
            PersistenceSystem::GbaSkyemu => "gba-skyemu",
            PersistenceSystem::GbaVbam => "gba-vbam",
            PersistenceSystem::GameboyGambatte => "gameboy-gambatte",
            PersistenceSystem::GameboyMgba => "gameboy-mgba",
            PersistenceSystem::GameboySameboy => "gameboy-sameboy",
            PersistenceSystem::GameboySkyemu => "gameboy-skyemu",
            PersistenceSystem::GameboyVbam => "gameboy-vbam",
            PersistenceSystem::Atari2600Stella => "atari2600-stella",
            PersistenceSystem::GameGear => "game-gear",
            PersistenceSystem::NesFceumm => "nes-fceumm",
            PersistenceSystem::NesMesen => "nes-mesen",
            PersistenceSystem::Snes9x => "snes9x",
            PersistenceSystem::Bsnes => "bsnes",
            PersistenceSystem::MesenS => "mesen-s",
            PersistenceSystem::PsxBeetle => "psx-beetle",
            PersistenceSystem::PsxBeetleHw => "psx-beetle-hw",
        })
        .arg("--expected-version")
        .arg(expected_version)
        .arg("--evidence")
        .arg(evidence)
        .arg("--report")
        .arg(&report_path);
    if let Some(bytes) = expected_state_bytes {
        command.arg("--expected-state-bytes").arg(bytes.to_string());
    }
    for path in &args.runtime_library {
        command.arg("--runtime-library").arg(path);
    }
    if let Some(path) = &args.bios_dir {
        command.arg("--bios-dir").arg(path);
    }
    let mut child = command
        .env("XDG_CONFIG_HOME", evidence.join("xdg-config"))
        .env("XDG_CACHE_HOME", evidence.join("xdg-cache"))
        .env("XDG_DATA_HOME", evidence.join("xdg-data"))
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .context("Starting persistence worker")?;
    let stdout = drain_bounded(child.stdout.take().context("Worker stdout unavailable")?);
    let stderr = drain_bounded(child.stderr.take().context("Worker stderr unavailable")?);
    let deadline = Instant::now() + Duration::from_secs(args.timeout_seconds);
    let status = loop {
        if let Some(status) = child.try_wait()? {
            break status;
        }
        if Instant::now() >= deadline {
            let _ = child.kill();
            let _ = child.wait();
            let stderr = stderr
                .join()
                .map_err(|_| anyhow::anyhow!("Worker stderr reader panicked"))??;
            bail!(
                "Persistence worker {phase:?} exceeded {} seconds; stderr: {}",
                args.timeout_seconds,
                String::from_utf8_lossy(&stderr.bytes)
            );
        }
        std::thread::sleep(Duration::from_millis(10));
    };
    let stdout = stdout
        .join()
        .map_err(|_| anyhow::anyhow!("Worker stdout reader panicked"))??;
    let stderr = stderr
        .join()
        .map_err(|_| anyhow::anyhow!("Worker stderr reader panicked"))??;
    ensure!(
        !stdout.truncated && !stderr.truncated,
        "Worker output exceeded capture limit"
    );
    ensure!(
        status.success(),
        "Persistence worker {phase:?} failed with {status}; stderr: {}",
        String::from_utf8_lossy(&stderr.bytes)
    );
    let report_bytes = std::fs::read(&report_path)
        .with_context(|| format!("Reading worker report {}", report_path.display()))?;
    ensure!(
        report_bytes.len() <= CAPTURE_LIMIT,
        "Worker report exceeded size limit"
    );
    let report: WorkerReport = serde_json::from_slice(&report_bytes)
        .with_context(|| format!("Parsing worker {phase:?} JSON"))?;
    ensure!(
        report.schema_version == REPORT_SCHEMA,
        "Worker schema changed"
    );
    ensure!(report.phase == phase, "Worker returned the wrong phase");
    ensure!(
        report.system == args.system,
        "Worker returned the wrong system"
    );
    ensure!(
        report.core.sha256 == hash,
        "Worker returned the wrong core hash"
    );
    Ok(report)
}

fn create_evidence_directory(requested: Option<&Path>) -> Result<PathBuf> {
    let path = if let Some(path) = requested {
        ensure!(
            !path.exists(),
            "Evidence directory already exists: {}",
            path.display()
        );
        std::fs::create_dir(path)
            .with_context(|| format!("Creating evidence directory {}", path.display()))?;
        path.to_path_buf()
    } else {
        tempfile::Builder::new()
            .prefix("lunchbox-libretro-persistence-")
            .tempdir()?
            .keep()
    };
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o700))?;
    }
    path.canonicalize()
        .context("Canonicalizing evidence directory")
}

fn artifact(path: PathBuf, expected_hash: &str) -> Result<Artifact> {
    let bytes =
        usize::try_from(std::fs::metadata(&path)?.len()).context("Artifact is too large")?;
    let hash = file_hash(&path)?;
    ensure!(
        hash == expected_hash,
        "Artifact changed after worker verification"
    );
    Ok(Artifact {
        path,
        bytes,
        sha256: hash,
    })
}

fn validate_reports(
    system: PersistenceSystem,
    hash: &str,
    expected_version: &str,
    expected_state_bytes: Option<usize>,
    reports: &[WorkerReport; 4],
) -> Result<()> {
    let spec = system.spec();
    let expected_identity = CoreIdentity {
        name: spec.name.into(),
        version: expected_version.into(),
        sha256: hash.into(),
    };
    let diagnostic_hash = &reports[0].diagnostic_rom_sha256;
    let runtime_libraries = &reports[0].runtime_libraries;
    let firmware = &reports[0].firmware;
    let core_option_overrides = system.core_option_overrides();
    let expected_reported_system_ram_bytes = if spec.memory_map.is_some() {
        0
    } else {
        spec.system_ram_bytes
    };
    let expected_system_ram_source = if spec.memory_map.is_some() {
        "retro-environment-memory-map"
    } else {
        "retro-memory-system-ram"
    };
    for report in reports {
        ensure!(
            report.core == expected_identity,
            "Core identity changed between workers"
        );
        ensure!(
            report.expected_state_bytes == expected_state_bytes,
            "Expected state size changed between workers"
        );
        ensure!(
            report.diagnostic_rom_sha256 == *diagnostic_hash,
            "Diagnostic ROM changed between workers"
        );
        ensure!(
            report.save_status
                == if system.save_supported() {
                    "pass"
                } else {
                    "not_applicable"
                },
            "Save applicability changed between workers"
        );
        ensure!(
            report.runtime_libraries == *runtime_libraries,
            "Runtime dependencies changed between workers"
        );
        ensure!(
            report.firmware == *firmware,
            "Firmware identities changed between workers"
        );
        ensure!(
            report.core_option_overrides == core_option_overrides,
            "Core-option overrides changed between workers"
        );
        ensure!(
            report.system_ram_bytes == spec.system_ram_bytes,
            "System RAM size changed between workers"
        );
        ensure!(
            report.reported_system_ram_bytes == expected_reported_system_ram_bytes
                && report.system_ram_source == expected_system_ram_source
                && report.system_ram_emulated_address
                    == spec.memory_map.map(|mapping| mapping.address),
            "System RAM source contract changed between workers"
        );
        ensure!(
            report.system_ram_offset == spec.system_ram_offset,
            "System RAM offset changed between workers"
        );
        ensure!(
            report.save_observation_source == system.save_observation_source()
                && report.save_marker_offset == system.save_marker_offset(),
            "Save observation contract changed between workers"
        );
    }
    if system.save_supported() {
        ensure!(
            reports[0].pre_save_bytes == Some(spec.pre_save_bytes)
                && reports[0].post_save_bytes == Some(spec.post_save_bytes)
                && reports[0].observation_hex.as_deref()
                    == Some(&bytes_hex(spec.first_observation)),
            "Save-create report violated its contract"
        );
        ensure!(
            reports[1].pre_save_bytes == Some(spec.pre_save_bytes)
                && reports[1].post_save_bytes == Some(spec.post_save_bytes)
                && reports[1].observation_hex.as_deref()
                    == Some(&bytes_hex(spec.second_observation)),
            "Save-reload report violated its contract"
        );
    } else {
        for report in &reports[..2] {
            ensure!(
                report.pre_save_bytes == Some(0)
                    && report.post_save_bytes == Some(0)
                    && report.observation_hex.is_none()
                    && report.save_sha256.is_none(),
                "State-only save disposition changed"
            );
        }
    }
    ensure!(
        expected_state_bytes.is_some()
            && reports[2].state_bytes == expected_state_bytes
            && reports[2].restored_marker_hex.as_deref() == Some(&bytes_hex(STATE_MARKER)),
        "State-create report violated its contract"
    );
    ensure!(
        reports[3].state_bytes == reports[2].state_bytes
            && reports[3].restored_marker_hex.as_deref() == Some(&bytes_hex(STATE_MARKER))
            && reports[3].before_marker_hex.as_deref() != Some(&bytes_hex(STATE_MARKER)),
        "State-reload report violated its contract"
    );
    ensure!(
        reports[2].state_sha256 == reports[3].state_sha256,
        "Persisted state hash changed between workers"
    );
    Ok(())
}

fn supervisor(mut args: SupervisorArgs) -> Result<PersistenceReport> {
    let hash = validate_hash(&args.sha256)?;
    let expected_version = resolve_expected_version(args.system, args.expected_version.as_deref())?;
    let expected_state_bytes =
        resolve_expected_state_bytes(args.system, args.expected_state_bytes)?;
    let (runtime_paths, runtime_libraries) = runtime_inventory(&args.runtime_library)?;
    args.runtime_library = runtime_paths;
    let firmware = if args.system.is_psx() {
        let bios = args
            .bios_dir
            .as_deref()
            .context("PSX persistence requires --bios-dir")?;
        let (bios, firmware) = psx_firmware_inventory(bios)?;
        args.bios_dir = Some(bios);
        firmware
    } else {
        ensure!(
            args.bios_dir.is_none(),
            "--bios-dir is only valid for PSX persistence"
        );
        Vec::new()
    };
    let core = args
        .core
        .canonicalize()
        .with_context(|| format!("Canonicalizing core {}", args.core.display()))?;
    ensure!(core.is_file(), "Core path is not a file");
    ensure!(
        file_hash(&core)? == hash,
        "Core hash does not match --sha256"
    );
    let evidence = create_evidence_directory(args.output.as_deref())?;
    let executable = std::env::current_exe()?.canonicalize()?;
    let phases = [
        WorkerPhase::SaveCreate,
        WorkerPhase::SaveReload,
        WorkerPhase::StateCreate,
        WorkerPhase::StateReload,
    ];
    let reports_vec = phases
        .into_iter()
        .map(|phase| {
            run_worker(
                &executable,
                &args,
                &core,
                &hash,
                &expected_version,
                expected_state_bytes,
                &evidence,
                phase,
            )
        })
        .collect::<Result<Vec<_>>>()?;
    let reports: [WorkerReport; 4] = reports_vec
        .try_into()
        .map_err(|_| anyhow::anyhow!("Internal worker count changed"))?;
    validate_reports(
        args.system,
        &hash,
        &expected_version,
        expected_state_bytes,
        &reports,
    )?;
    ensure!(
        reports[0].runtime_libraries == runtime_libraries,
        "Worker runtime dependency inventory differs from the supervisor"
    );
    ensure!(
        reports[0].firmware == firmware,
        "Worker firmware inventory differs from the supervisor"
    );

    let initial_save_hash = reports[0].save_sha256.as_deref();
    let reloaded_save_hash = reports[1].save_sha256.as_deref();
    if args.system.save_supported() {
        ensure!(
            initial_save_hash.is_some() && reloaded_save_hash.is_some(),
            "Missing save artifact hash"
        );
    } else {
        ensure!(
            initial_save_hash.is_none() && reloaded_save_hash.is_none(),
            "State-only core produced an unexpected save artifact hash"
        );
    }
    let state_hash = reports[2]
        .state_sha256
        .as_deref()
        .context("Missing state hash")?;
    let expected_state_bytes = expected_state_bytes
        .context("No pinned state size; supply --expected-state-bytes for this system")?;
    let report = PersistenceReport {
        schema_version: REPORT_SCHEMA,
        status: "pass",
        system: args.system,
        core_path: core,
        core: reports[0].core.clone(),
        expected_state_bytes,
        runtime_libraries,
        firmware,
        core_option_overrides: args.system.core_option_overrides(),
        evidence_directory: evidence.clone(),
        diagnostic_rom: artifact(
            diagnostic_path(&evidence, args.system),
            &reports[0].diagnostic_rom_sha256,
        )?,
        reported_system_ram_bytes: reports[0].reported_system_ram_bytes,
        system_ram_source: reports[0].system_ram_source.clone(),
        system_ram_emulated_address: reports[0].system_ram_emulated_address,
        system_ram_bytes: args.system.spec().system_ram_bytes,
        system_ram_offset: args.system.spec().system_ram_offset,
        save_ram: SaveRamReport {
            status: if args.system.save_supported() {
                "pass"
            } else {
                "not_applicable"
            },
            observation_source: args
                .system
                .save_supported()
                .then(|| args.system.save_observation_source().into()),
            marker_offset: args
                .system
                .save_supported()
                .then(|| args.system.save_marker_offset()),
            initial_observation_hex: args
                .system
                .save_supported()
                .then(|| bytes_hex(args.system.spec().first_observation)),
            fresh_process_observation_hex: args
                .system
                .save_supported()
                .then(|| bytes_hex(args.system.spec().second_observation)),
            initial: initial_save_hash
                .map(|hash| artifact(save_path(&evidence, args.system, false), hash))
                .transpose()?,
            after_reload: reloaded_save_hash
                .map(|hash| artifact(save_path(&evidence, args.system, true), hash))
                .transpose()?,
        },
        save_state: SaveStateReport {
            status: "pass",
            expected_bytes: expected_state_bytes,
            same_process_restored_marker_hex: bytes_hex(STATE_MARKER),
            fresh_process_before_marker_hex: reports[3]
                .before_marker_hex
                .clone()
                .context("Missing fresh-process marker")?,
            fresh_process_restored_marker_hex: bytes_hex(STATE_MARKER),
            artifact: artifact(state_path(&evidence, args.system), state_hash)?,
        },
    };
    let result_path = evidence.join("results.json");
    std::fs::write(&result_path, serde_json::to_vec_pretty(&report)?)?;
    Ok(report)
}

/// Parse the public CLI, or the private subprocess protocol used by the driver.
pub fn run_cli() -> Result<()> {
    let raw = std::env::args_os().collect::<Vec<_>>();
    if raw.get(1).is_some_and(|value| value == "__worker") {
        let worker_args = std::iter::once(raw[0].clone())
            .chain(raw.into_iter().skip(2))
            .collect::<Vec<OsString>>();
        let args = WorkerArgs::parse_from(worker_args);
        ensure!(
            args.report.is_absolute(),
            "Worker report path is not absolute"
        );
        let canonical_evidence = args
            .evidence
            .canonicalize()
            .context("Canonicalizing worker evidence directory")?;
        ensure!(
            args.evidence == canonical_evidence,
            "Worker evidence path is not canonical"
        );
        let expected_report = args
            .evidence
            .join("workers")
            .join(format!("{}.json", args.phase.as_cli_value()));
        ensure!(
            args.report == expected_report,
            "Worker report path does not match its private phase path"
        );
        ensure!(!args.report.exists(), "Worker report already exists");
        let report_path = args.report.clone();
        let report = worker(args)?;
        std::fs::create_dir_all(
            report_path
                .parent()
                .context("Worker report has no parent")?,
        )?;
        std::fs::write(report_path, serde_json::to_vec(&report)?)?;
    } else {
        let report = supervisor(SupervisorArgs::parse_from(raw))?;
        println!("{}", serde_json::to_string_pretty(&report)?);
    }
    Ok(())
}

fn arm_branch(words: &mut [u32], instruction: usize, target: usize, condition: u32) {
    let displacement = isize::try_from(target).unwrap() - isize::try_from(instruction + 2).unwrap();
    assert!((-(1 << 23)..(1 << 23)).contains(&displacement));
    words[instruction] =
        (condition << 28) | 0x0a00_0000 | ((displacement as i32 as u32) & 0x00ff_ffff);
}

/// Original ARM program that accesses the GBA SRAM bus byte-by-byte. It writes
/// `LBSG01` on a blank cartridge, increments it to `LBSG02` after a frontend
/// reload, and publishes the six bytes to EWRAM for independent observation.
pub fn gba_persistence_rom() -> Vec<u8> {
    let mut words = Vec::new();
    let load_save = words.len();
    words.push(0); // ldr r0, [pc, #save_literal]
    let load_output = words.len();
    words.push(0); // ldr r1, [pc, #output_literal]
    let mut mismatch_branches = Vec::new();
    for (offset, expected) in b"LBSG".iter().copied().enumerate() {
        words.push(0xe5d0_2000 | u32::try_from(offset).unwrap()); // ldrb r2, [r0, #offset]
        words.push(0xe352_0000 | u32::from(expected)); // cmp r2, #expected
        mismatch_branches.push(words.len());
        words.push(0); // bne initialize
    }
    words.push(0xe5d0_2005); // ldrb r2, [r0, #5]
    words.push(0xe282_2001); // add r2, r2, #1
    words.push(0xe5c0_2005); // strb r2, [r0, #5]
    let publish_branch = words.len();
    words.push(0); // b publish
    let initialize = words.len();
    for (offset, value) in FIRST_SAVE_OBSERVATION.iter().copied().enumerate() {
        words.push(0xe3a0_2000 | u32::from(value)); // mov r2, #value
        words.push(0xe5c0_2000 | u32::try_from(offset).unwrap()); // strb r2, [r0, #offset]
    }
    let publish = words.len();
    for offset in 0..FIRST_SAVE_OBSERVATION.len() {
        words.push(0xe5d0_2000 | u32::try_from(offset).unwrap()); // ldrb r2, [r0, #offset]
        words.push(0xe5c1_2000 | u32::try_from(offset).unwrap()); // strb r2, [r1, #offset]
    }
    let halt = words.len();
    words.push(0); // b halt
    let save_literal = words.len();
    words.push(0x0e00_0000);
    let output_literal = words.len();
    words.push(0x0200_0000);
    for branch in mismatch_branches {
        arm_branch(&mut words, branch, initialize, 1); // NE
    }
    arm_branch(&mut words, publish_branch, publish, 14); // AL
    arm_branch(&mut words, halt, halt, 14); // AL
    for (instruction, target, register) in [
        (load_save, save_literal, 0u32),
        (load_output, output_literal, 1u32),
    ] {
        let byte_offset = (target - instruction - 2) * 4;
        assert!(byte_offset <= 0xfff);
        words[instruction] = 0xe59f_0000 | (register << 12) | u32::try_from(byte_offset).unwrap();
    }

    let mut rom = vec![0u8; 0x1000];
    rom[..4].copy_from_slice(&0xea00_002eu32.to_le_bytes()); // b 0x080000c0
    rom[0xa0..0xac].copy_from_slice(b"LBSAVETEST00");
    rom[0xb2] = 0x96;
    rom[0xbd] = rom[0xa0..0xbd]
        .iter()
        .fold(0u8, |sum, byte| sum.wrapping_sub(*byte))
        .wrapping_sub(0x19);
    for (index, word) in words.into_iter().enumerate() {
        let offset = 0xc0 + index * 4;
        rom[offset..offset + 4].copy_from_slice(&word.to_le_bytes());
    }
    rom[0x300..0x309].copy_from_slice(b"SRAM_V113");
    rom
}

/// Original LR35902 program in a 32 KiB MBC1+RAM+battery cartridge. It enables
/// external RAM, selects MBC1 RAM-banking mode and bank zero, performs the same
/// `LBSG01`/`LBSG02` transaction at `$A000`, and copies the result to `$C000`.
pub fn gameboy_persistence_rom() -> Vec<u8> {
    const CARTRIDGE_LOGO: [u8; 48] = [
        0xce, 0xed, 0x66, 0x66, 0xcc, 0x0d, 0x00, 0x0b, 0x03, 0x73, 0x00, 0x83, 0x00, 0x0c, 0x00,
        0x0d, 0x00, 0x08, 0x11, 0x1f, 0x88, 0x89, 0x00, 0x0e, 0xdc, 0xcc, 0x6e, 0xe6, 0xdd, 0xdd,
        0xd9, 0x99, 0xbb, 0xbb, 0x67, 0x63, 0x6e, 0x0e, 0xec, 0xcc, 0xdd, 0xdc, 0x99, 0x9f, 0xbb,
        0xb9, 0x33, 0x3e,
    ];

    let mut program = vec![
        0xf3, // di
        0x31, 0xf0, 0xdf, // ld sp, $dff0
        0x3e, 0x0a, // ld a, $0a
        0xea, 0x00, 0x00, // ld ($0000), a: enable external RAM
        0x3e, 0x01, // ld a, $01
        0xea, 0x00, 0x60, // ld ($6000), a: MBC1 RAM-banking mode
        0xaf, // xor a
        0xea, 0x00, 0x40, // ld ($4000), a: RAM bank zero
        0x21, 0x00, 0xa0, // ld hl, $a000
    ];
    let mut mismatch_operands = Vec::new();
    for expected in b"LBSG" {
        program.extend([0x7e, 0xfe, *expected, 0x20, 0]); // ld a,(hl); cp n; jr nz,init
        mismatch_operands.push(program.len() - 1);
        program.push(0x23); // inc hl
    }
    program.extend([0x23, 0x7e, 0x3c, 0x77, 0x18, 0]); // skip '0'; increment byte 5; jr publish
    let publish_operand = program.len() - 1;
    let initialize = program.len();
    program.extend([0x21, 0x00, 0xa0]); // ld hl, $a000
    for value in FIRST_SAVE_OBSERVATION {
        program.extend([0x36, *value, 0x23]); // ld (hl),n; inc hl
    }
    let publish = program.len();
    program.extend([
        0x21, 0x00, 0xa0, // ld hl, $a000
        0x11, 0x00, 0xc0, // ld de, $c000
        0x06, 0x06, // ld b, 6
        0x2a, // copy: ld a,(hl+)
        0x12, // ld (de),a
        0x13, // inc de
        0x05, // dec b
        0x20, 0xfa, // jr nz,copy
        0x76, // halt
        0x18, 0xfd, // jr halt
    ]);
    for operand in mismatch_operands {
        let delta = isize::try_from(initialize).unwrap() - isize::try_from(operand + 1).unwrap();
        assert!((-128..=127).contains(&delta));
        program[operand] = delta as i8 as u8;
    }
    let delta = isize::try_from(publish).unwrap() - isize::try_from(publish_operand + 1).unwrap();
    assert!((-128..=127).contains(&delta));
    program[publish_operand] = delta as i8 as u8;

    let mut rom = vec![0u8; 32 * 1024];
    rom[0x100..0x104].copy_from_slice(&[0x00, 0xc3, 0x50, 0x01]); // nop; jp $0150
    rom[0x104..0x134].copy_from_slice(&CARTRIDGE_LOGO);
    rom[0x134..0x140].copy_from_slice(b"LUNCHBOXSAVE");
    rom[0x147] = 0x03; // MBC1 + RAM + battery
    rom[0x148] = 0x00; // 32 KiB ROM
    rom[0x149] = 0x03; // 32 KiB external RAM
    rom[0x14a] = 0x01; // non-Japanese destination
    rom[0x14b] = 0x33; // extended licensee marker
    rom[0x150..0x150 + program.len()].copy_from_slice(&program);
    rom[0x14d] = rom[0x134..0x14d]
        .iter()
        .fold(0u8, |sum, byte| sum.wrapping_sub(*byte).wrapping_sub(1));
    let global_checksum = rom
        .iter()
        .enumerate()
        .filter(|(index, _)| !matches!(index, 0x14e | 0x14f))
        .fold(0u16, |sum, (_, byte)| sum.wrapping_add(u16::from(*byte)));
    rom[0x14e..0x150].copy_from_slice(&global_checksum.to_be_bytes());
    rom
}

/// Original Z80 program for a 64 KiB Game Gear cartridge. The Sega mapper is
/// selected by ROM size, then `$FFFC = $08` maps cartridge SRAM at `$8000`.
/// The same `LBSG01`/`LBSG02` transaction is copied to work RAM at `$C000`.
pub fn game_gear_persistence_rom() -> Vec<u8> {
    let mut program = vec![
        0xf3, // di
        0x31, 0xf0, 0xdf, // ld sp, $dff0
        0x3e, 0x08, // ld a, $08
        0x32, 0xfc, 0xff, // ld ($fffc), a: map lower SRAM bank at $8000
        0x21, 0x00, 0x80, // ld hl, $8000
    ];
    let mut mismatch_operands = Vec::new();
    for expected in b"LBSG" {
        program.extend([0x7e, 0xfe, *expected, 0x20, 0]); // ld a,(hl); cp n; jr nz,init
        mismatch_operands.push(program.len() - 1);
        program.push(0x23); // inc hl
    }
    program.extend([0x23, 0x7e, 0x3c, 0x77, 0x18, 0]); // skip '0'; inc byte 5; jr publish
    let publish_operand = program.len() - 1;
    let initialize = program.len();
    program.extend([0x21, 0x00, 0x80]); // ld hl, $8000
    for value in FIRST_SAVE_OBSERVATION {
        program.extend([0x36, *value, 0x23]); // ld (hl),n; inc hl
    }
    let publish = program.len();
    program.extend([
        0x21, 0x00, 0x80, // ld hl, $8000
        0x11, 0x00, 0xc0, // ld de, $c000
        0x01, 0x06, 0x00, // ld bc, 6
        0xed, 0xb0, // ldir
        0x76, // halt
        0x18, 0xfd, // jr halt
    ]);
    for operand in mismatch_operands {
        let delta = isize::try_from(initialize).unwrap() - isize::try_from(operand + 1).unwrap();
        assert!((-128..=127).contains(&delta));
        program[operand] = delta as i8 as u8;
    }
    let delta = isize::try_from(publish).unwrap() - isize::try_from(publish_operand + 1).unwrap();
    assert!((-128..=127).contains(&delta));
    program[publish_operand] = delta as i8 as u8;

    let mut rom = vec![0u8; 64 * 1024];
    rom[..program.len()].copy_from_slice(&program);
    rom[0x7ff0..0x7ff8].copy_from_slice(b"TMR SEGA");
    rom[0x7fff] = 0x6e; // Export Game Gear, 64 KiB.
    let checksum = rom[..0x7ff0]
        .iter()
        .fold(0u16, |sum, byte| sum.wrapping_add(u16::from(*byte)));
    rom[0x7ffa..0x7ffc].copy_from_slice(&checksum.to_le_bytes());
    rom
}

/// Original 6502/NROM-128 program with the iNES battery flag. It initializes
/// PRG RAM at `$6000` to `LBSR\x01`, increments the counter after reload, and
/// publishes the five bytes to zero-page RAM.
pub fn nes_persistence_rom() -> Vec<u8> {
    const HEADER_SIZE: usize = 16;
    const PRG_SIZE: usize = 16 * 1024;
    const CHR_SIZE: usize = 8 * 1024;
    let mut rom = vec![0u8; HEADER_SIZE + PRG_SIZE + CHR_SIZE];
    rom[..16].copy_from_slice(&[
        b'N', b'E', b'S', 0x1a, 1, 1, 0x02, 0, 1, 0, 0, 0, 0, 0, 0, 0,
    ]);
    let mut program = vec![
        0x78, // sei
        0xd8, // cld
        0xa2, 0xff, // ldx #$ff
        0x9a, // txs
    ];
    let mut mismatch_operands = Vec::new();
    for (address, expected) in [
        (0x6000u16, b'L'),
        (0x6001, b'B'),
        (0x6002, b'S'),
        (0x6003, b'R'),
    ] {
        program.extend([
            0xad,
            address as u8,
            (address >> 8) as u8,
            0xc9,
            expected,
            0xd0,
            0,
        ]); // lda address; cmp #expected; bne initialize
        mismatch_operands.push(program.len() - 1);
    }
    program.extend([0xee, 0x04, 0x60, 0x4c, 0, 0]); // inc $6004; jmp publish
    let publish_operand = program.len() - 2;
    let initialize = program.len();
    for (address, value) in [
        (0x6000u16, b'L'),
        (0x6001, b'B'),
        (0x6002, b'S'),
        (0x6003, b'R'),
        (0x6004, 1),
    ] {
        program.extend([0xa9, value, 0x8d, address as u8, (address >> 8) as u8]); // lda #value; sta address
    }
    let publish = program.len();
    for offset in 0..FIRST_NES_SAVE_OBSERVATION.len() {
        let address = 0x6000u16 + u16::try_from(offset).unwrap();
        program.extend([
            0xad,
            address as u8,
            (address >> 8) as u8,
            0x85,
            u8::try_from(offset).unwrap(),
        ]); // lda address; sta zero-page result
    }
    let halt_address = 0x8000u16 + u16::try_from(program.len()).unwrap();
    program.extend([0x4c, halt_address as u8, (halt_address >> 8) as u8]);
    for operand in mismatch_operands {
        let displacement =
            isize::try_from(initialize).unwrap() - isize::try_from(operand + 1).unwrap();
        assert!((-128..=127).contains(&displacement));
        program[operand] = displacement as i8 as u8;
    }
    let publish_address = 0x8000u16 + u16::try_from(publish).unwrap();
    program[publish_operand..publish_operand + 2].copy_from_slice(&publish_address.to_le_bytes());
    rom[HEADER_SIZE..HEADER_SIZE + program.len()].copy_from_slice(&program);
    for vector in [0x3ffa, 0x3ffc, 0x3ffe] {
        rom[HEADER_SIZE + vector..HEADER_SIZE + vector + 2]
            .copy_from_slice(&0x8000u16.to_le_bytes());
    }
    rom
}

/// Original 65C816 LoROM program with an 8 KiB battery-backed SRAM header. It
/// accesses SRAM through bank `$70`, publishes to WRAM bank `$7E`, and uses the
/// same visible `LBSG01`/`LBSG02` fresh-process transaction as GBA/Game Gear.
pub fn snes_persistence_rom() -> Vec<u8> {
    let mut program = vec![
        0x78, // sei
        0xd8, // cld
        0xa2, 0xff, // ldx #$ff
        0x9a, // txs
    ];
    let mut mismatch_operands = Vec::new();
    for (offset, expected) in b"LBSG".iter().copied().enumerate() {
        program.extend([
            0xaf,
            u8::try_from(offset).unwrap(),
            0x00,
            0x70, // lda $70:0000+offset
            0xc9,
            expected, // cmp #expected
            0xd0,
            0, // bne initialize
        ]);
        mismatch_operands.push(program.len() - 1);
    }
    program.extend([
        0xaf, 0x05, 0x00, 0x70, // lda $70:0005
        0x18, // clc
        0x69, 0x01, // adc #1
        0x8f, 0x05, 0x00, 0x70, // sta $70:0005
        0x80, 0, // bra publish
    ]);
    let publish_operand = program.len() - 1;
    let initialize = program.len();
    for (offset, value) in FIRST_SAVE_OBSERVATION.iter().copied().enumerate() {
        program.extend([0xa9, value, 0x8f, u8::try_from(offset).unwrap(), 0x00, 0x70]); // lda #value; sta $70:0000+offset
    }
    let publish = program.len();
    for offset in 0..FIRST_SAVE_OBSERVATION.len() {
        program.extend([
            0xaf,
            u8::try_from(offset).unwrap(),
            0x00,
            0x70, // lda SRAM
            0x8f,
            u8::try_from(offset).unwrap(),
            0x00,
            0x7e, // sta WRAM
        ]);
    }
    let halt_address = 0x8000u16 + u16::try_from(program.len()).unwrap();
    program.extend([0x4c, halt_address as u8, (halt_address >> 8) as u8]);
    for operand in mismatch_operands {
        let displacement =
            isize::try_from(initialize).unwrap() - isize::try_from(operand + 1).unwrap();
        assert!((-128..=127).contains(&displacement));
        program[operand] = displacement as i8 as u8;
    }
    let displacement =
        isize::try_from(publish).unwrap() - isize::try_from(publish_operand + 1).unwrap();
    assert!((-128..=127).contains(&displacement));
    program[publish_operand] = displacement as i8 as u8;

    let mut rom = vec![0u8; 32 * 1024];
    rom[..program.len()].copy_from_slice(&program);
    rom[0x7fc0..0x7fd5].copy_from_slice(b"LUNCHBOX PERSISTENCE ");
    rom[0x7fd5..0x7fdc].copy_from_slice(&[
        0x20, // LoROM, slow ROM
        0x02, // ROM + RAM + battery
        0x05, // 32 KiB ROM
        0x03, // 8 KiB SRAM
        0x01, // NTSC region
        0x33, // extended publisher marker
        0x00, // version
    ]);
    for vector in (0x7fe4..0x8000).step_by(2) {
        rom[vector..vector + 2].copy_from_slice(&0x8000u16.to_le_bytes());
    }
    let checksum = rom
        .iter()
        .fold(0u16, |sum, byte| sum.wrapping_add(u16::from(*byte)))
        .wrapping_add(0x01fe);
    rom[0x7fdc..0x7fde].copy_from_slice(&(!checksum).to_le_bytes());
    rom[0x7fde..0x7fe0].copy_from_slice(&checksum.to_le_bytes());
    rom
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn missing_psx_bios_reports_the_required_filename() {
        let directory = tempfile::tempdir().unwrap();
        let error = psx_firmware_inventory(directory.path()).unwrap_err();
        let message = format!("{error:#}");
        assert!(message.contains("Reading required BIOS"));
        assert!(message.contains("scph5500.bin"));
    }

    #[test]
    fn gba_rom_is_reproducible_and_declares_sram() {
        let rom = gba_persistence_rom();
        assert_eq!(rom, gba_persistence_rom());
        assert_eq!(rom.len(), 0x1000);
        assert_eq!(
            u32::from_le_bytes(rom[..4].try_into().unwrap()),
            0xea00_002e
        );
        assert_eq!(&rom[0x300..0x309], b"SRAM_V113");
        assert_eq!(
            rom[0xbd],
            rom[0xa0..0xbd]
                .iter()
                .fold(0u8, |sum, byte| sum.wrapping_sub(*byte))
                .wrapping_sub(0x19)
        );
        assert!(
            rom.windows(4)
                .any(|bytes| bytes == 0x0e00_0000u32.to_le_bytes())
        );
        assert!(
            rom.windows(4)
                .any(|bytes| bytes == 0x0200_0000u32.to_le_bytes())
        );
    }

    #[test]
    fn game_gear_rom_is_reproducible_and_enables_sega_sram_mapper() {
        let rom = game_gear_persistence_rom();
        assert_eq!(rom, game_gear_persistence_rom());
        assert_eq!(rom.len(), 64 * 1024);
        assert_eq!(
            &rom[..9],
            &[0xf3, 0x31, 0xf0, 0xdf, 0x3e, 0x08, 0x32, 0xfc, 0xff]
        );
        assert_eq!(&rom[0x7ff0..0x7ff8], b"TMR SEGA");
        assert_eq!(rom[0x7fff], 0x6e);
        assert_eq!(
            u16::from_le_bytes(rom[0x7ffa..0x7ffc].try_into().unwrap()),
            rom[..0x7ff0]
                .iter()
                .fold(0u16, |sum, byte| sum.wrapping_add(u16::from(*byte)))
        );
    }

    #[test]
    fn gameboy_rom_is_reproducible_and_declares_mbc1_battery_ram() {
        let rom = gameboy_persistence_rom();
        assert_eq!(rom, gameboy_persistence_rom());
        assert_eq!(rom.len(), 32 * 1024);
        assert_eq!(&rom[0x100..0x104], &[0x00, 0xc3, 0x50, 0x01]);
        assert_eq!(&rom[0x134..0x140], b"LUNCHBOXSAVE");
        assert_eq!(&rom[0x147..0x14a], &[0x03, 0x00, 0x03]);
        assert_eq!(
            rom[0x14d],
            rom[0x134..0x14d]
                .iter()
                .fold(0u8, |sum, byte| sum.wrapping_sub(*byte).wrapping_sub(1))
        );
        assert_eq!(
            u16::from_be_bytes(rom[0x14e..0x150].try_into().unwrap()),
            rom.iter()
                .enumerate()
                .filter(|(index, _)| !matches!(index, 0x14e | 0x14f))
                .fold(0u16, |sum, (_, byte)| sum.wrapping_add(u16::from(*byte)))
        );
        assert!(rom.windows(3).any(|bytes| bytes == [0xea, 0x00, 0x00]));
        assert!(rom.windows(3).any(|bytes| bytes == [0xea, 0x00, 0x60]));
        assert!(rom.windows(3).any(|bytes| bytes == [0xea, 0x00, 0x40]));
    }

    #[test]
    fn nes_rom_is_reproducible_and_declares_battery_prg_ram() {
        let rom = nes_persistence_rom();
        assert_eq!(rom, nes_persistence_rom());
        assert_eq!(rom.len(), 16 + 16 * 1024 + 8 * 1024);
        assert_eq!(&rom[..9], &[b'N', b'E', b'S', 0x1a, 1, 1, 0x02, 0, 1]);
        for vector in [0x3ffa, 0x3ffc, 0x3ffe] {
            assert_eq!(
                u16::from_le_bytes(rom[16 + vector..16 + vector + 2].try_into().unwrap()),
                0x8000
            );
        }
    }

    #[test]
    fn snes_rom_is_reproducible_and_declares_battery_sram() {
        let rom = snes_persistence_rom();
        assert_eq!(rom, snes_persistence_rom());
        assert_eq!(rom.len(), 32 * 1024);
        assert_eq!(&rom[0x7fc0..0x7fd5], b"LUNCHBOX PERSISTENCE ");
        assert_eq!(
            &rom[0x7fd5..0x7fdc],
            &[0x20, 0x02, 0x05, 0x03, 0x01, 0x33, 0x00]
        );
        let complement = u16::from_le_bytes(rom[0x7fdc..0x7fde].try_into().unwrap());
        let checksum = u16::from_le_bytes(rom[0x7fde..0x7fe0].try_into().unwrap());
        assert_eq!(complement, !checksum);
        assert_eq!(
            rom.iter()
                .fold(0u16, |sum, byte| sum.wrapping_add(u16::from(*byte))),
            checksum
        );
        assert!(
            rom.windows(4)
                .any(|bytes| bytes == [0xaf, 0x00, 0x00, 0x70])
        );
        assert!(
            rom.windows(4)
                .any(|bytes| bytes == [0x8f, 0x00, 0x00, 0x7e])
        );
    }

    #[test]
    fn standard_memory_is_not_dereferenced_before_the_first_frame() {
        let mgba_gameboy = PersistenceSystem::GameboyMgba.spec();
        assert_eq!(
            pre_run_observation_bytes(mgba_gameboy, 32_768, || {
                panic!("standard-memory pointer was inspected before retro_run")
            })
            .unwrap(),
            32_768
        );

        let skyemu_gba = PersistenceSystem::GbaSkyemu.spec();
        assert_eq!(
            pre_run_observation_bytes(skyemu_gba, 0, || Ok(262_144)).unwrap(),
            262_144
        );
    }

    #[test]
    fn exact_core_contracts_are_system_specific() {
        let gba = PersistenceSystem::Gba.spec();
        assert_eq!(
            (gba.name, gba.pre_save_bytes, gba.post_save_bytes),
            ("mGBA", 131_072, 32_768)
        );
        let gba_skyemu = PersistenceSystem::GbaSkyemu.spec();
        assert_eq!(
            (
                gba_skyemu.name,
                gba_skyemu.pre_save_bytes,
                gba_skyemu.post_save_bytes,
                gba_skyemu.system_ram_bytes,
                gba_skyemu.state_bytes,
                gba_skyemu.memory_map,
            ),
            (
                "SkyEmu",
                131_072,
                131_072,
                262_144,
                Some(581_832),
                Some(ExactMapping {
                    address: 0x0200_0000,
                    bytes: 262_144,
                    select: 0xff00_0000,
                }),
            )
        );
        let gba_vbam = PersistenceSystem::GbaVbam.spec();
        assert_eq!(
            (
                gba_vbam.name,
                gba_vbam.version,
                gba_vbam.pre_save_bytes,
                gba_vbam.post_save_bytes,
                gba_vbam.system_ram_bytes,
                gba_vbam.system_ram_offset,
                gba_vbam.state_bytes,
            ),
            (
                "VBA-M",
                "2.1.3 115defb",
                32_768,
                32_768,
                262_144,
                0,
                Some(723_452),
            )
        );
        let gg = PersistenceSystem::GameGear.spec();
        assert_eq!(
            (gg.name, gg.pre_save_bytes, gg.post_save_bytes),
            ("Genesis Plus GX", 65_536, 6)
        );
        let snes = PersistenceSystem::Snes9x.spec();
        assert_eq!(
            (snes.name, snes.pre_save_bytes, snes.post_save_bytes),
            ("Snes9x", 8_192, 8_192)
        );
        let stella = PersistenceSystem::Atari2600Stella.spec();
        assert_eq!(
            (
                stella.name,
                stella.version,
                stella.pre_save_bytes,
                stella.post_save_bytes,
                stella.system_ram_bytes,
                stella.state_bytes,
            ),
            ("Stella", "8.0_pre c65c845", 0, 0, 128, Some(1_041))
        );
        assert!(!PersistenceSystem::Atari2600Stella.save_supported());
        assert_eq!(
            PersistenceSystem::Atari2600Stella.state_marker_offset(),
            0x20
        );
        assert_eq!(
            PersistenceSystem::Atari2600Stella
                .state_ready_spec()
                .system_ram_offset,
            4
        );
        for (system, name) in [
            (PersistenceSystem::PsxBeetle, "Beetle PSX"),
            (PersistenceSystem::PsxBeetleHw, "Beetle PSX HW"),
        ] {
            let psx = system.spec();
            assert_eq!(psx.name, name);
            assert_eq!(psx.version, "0.9.44.1 82d8e05");
            assert_eq!(psx.extension, "exe");
            assert!(psx.need_fullpath);
            assert_eq!(psx.pre_save_bytes, 131_072);
            assert_eq!(psx.post_save_bytes, 131_072);
            assert_eq!(psx.system_ram_bytes, 2_097_152);
            assert_eq!(psx.state_bytes, Some(16_777_216));
            assert_eq!(system.save_marker_offset(), 126_976);
            assert_eq!(
                system.save_observation_source(),
                "frontend-libretro-memory-card-buffer"
            );
        }
        assert!(
            PersistenceSystem::PsxBeetle
                .core_option_overrides()
                .is_empty()
        );
        assert_eq!(
            PersistenceSystem::PsxBeetleHw.core_option_overrides(),
            BTreeMap::from([("beetle_psx_hw_renderer".into(), "software".into())])
        );
        let gameboy = [
            (
                PersistenceSystem::GameboyGambatte,
                "Gambatte",
                "v0.5.0-netlink d9d6cd0",
                32_768,
                32_768,
                8_192,
                0,
                59_650,
            ),
            (
                PersistenceSystem::GameboyMgba,
                "mGBA",
                "0.11-219-e31759b",
                32_768,
                32_768,
                32_768,
                0,
                202_816,
            ),
            (
                PersistenceSystem::GameboySameboy,
                "SameBoy",
                "1.0.3 8230189",
                32_768,
                32_768,
                8_192,
                0,
                252_666,
            ),
            (
                PersistenceSystem::GameboySkyemu,
                "SkyEmu",
                "adacd0788964ed89f5c43dcbc1f3cc26deec996c",
                131_072,
                131_072,
                98_304,
                49_152,
                246_416,
            ),
            (
                PersistenceSystem::GameboyVbam,
                "VBA-M",
                "2.1.3 115defb",
                32_768,
                32_768,
                32_768,
                0,
                115_948,
            ),
        ];
        for (system, name, version, pre, post, ram, offset, state) in gameboy {
            let spec = system.spec();
            assert_eq!(spec.name, name);
            assert_eq!(spec.version, version);
            assert_eq!(spec.extension, "gb");
            assert!(!spec.need_fullpath);
            assert_eq!(spec.pre_save_bytes, pre);
            assert_eq!(spec.post_save_bytes, post);
            assert_eq!(spec.system_ram_bytes, ram);
            assert_eq!(spec.system_ram_offset, offset);
            assert_eq!(spec.memory_map, None);
            assert_eq!(spec.frame_limit, 240);
            assert_eq!(spec.first_observation, FIRST_SAVE_OBSERVATION);
            assert_eq!(spec.second_observation, SECOND_SAVE_OBSERVATION);
            assert_eq!(spec.state_bytes, Some(state));
        }
        assert!(validate_hash(&"a".repeat(64)).is_ok());
        assert!(validate_hash(&"a".repeat(63)).is_err());
        assert!(validate_hash(&format!("{}z", "a".repeat(63))).is_err());
    }

    #[test]
    fn caller_supplied_version_override_is_exact() {
        assert_eq!(
            resolve_expected_version(PersistenceSystem::NesFceumm, None).unwrap(),
            "(SVN) 5cd4a43"
        );
        assert_eq!(
            resolve_expected_version(PersistenceSystem::NesFceumm, Some("(SVN) 236ccdf")).unwrap(),
            "(SVN) 236ccdf"
        );
        assert!(resolve_expected_version(PersistenceSystem::NesFceumm, Some("")).is_err());
        assert!(
            resolve_expected_version(PersistenceSystem::NesFceumm, Some(&"x".repeat(257))).is_err()
        );
    }

    #[test]
    fn caller_supplied_state_size_override_is_exact_and_bounded() {
        assert_eq!(
            resolve_expected_state_bytes(PersistenceSystem::NesFceumm, None).unwrap(),
            Some(13_726)
        );
        assert_eq!(
            resolve_expected_state_bytes(PersistenceSystem::NesFceumm, Some(13_758)).unwrap(),
            Some(13_758)
        );
        assert!(resolve_expected_state_bytes(PersistenceSystem::NesFceumm, Some(0)).is_err());
        assert!(
            resolve_expected_state_bytes(PersistenceSystem::NesFceumm, Some(32 * 1024 * 1024 + 1))
                .is_err()
        );
    }

    #[test]
    fn supervisor_rejects_cross_worker_report_drift() {
        let system = PersistenceSystem::Gba;
        let spec = system.spec();
        let hash = "a".repeat(64);
        let identity = CoreIdentity {
            name: spec.name.into(),
            version: spec.version.into(),
            sha256: hash.clone(),
        };
        let base = WorkerReport {
            schema_version: REPORT_SCHEMA,
            phase: WorkerPhase::SaveCreate,
            system,
            core: identity,
            expected_state_bytes: spec.state_bytes,
            runtime_libraries: Vec::new(),
            firmware: Vec::new(),
            core_option_overrides: BTreeMap::new(),
            diagnostic_rom_sha256: "b".repeat(64),
            save_status: "pass".into(),
            pre_save_bytes: None,
            post_save_bytes: None,
            reported_system_ram_bytes: spec.system_ram_bytes,
            system_ram_source: "retro-memory-system-ram".into(),
            system_ram_emulated_address: None,
            system_ram_bytes: spec.system_ram_bytes,
            system_ram_offset: spec.system_ram_offset,
            save_observation_source: system.save_observation_source().into(),
            save_marker_offset: system.save_marker_offset(),
            observation_hex: None,
            save_sha256: None,
            state_bytes: None,
            state_sha256: None,
            before_marker_hex: None,
            restored_marker_hex: None,
        };
        let mut reports = std::array::from_fn(|_| base.clone());
        reports[0].pre_save_bytes = Some(spec.pre_save_bytes);
        reports[0].post_save_bytes = Some(spec.post_save_bytes);
        reports[0].observation_hex = Some(bytes_hex(FIRST_SAVE_OBSERVATION));
        reports[0].save_sha256 = Some("c".repeat(64));
        reports[1].phase = WorkerPhase::SaveReload;
        reports[1].pre_save_bytes = Some(spec.pre_save_bytes);
        reports[1].post_save_bytes = Some(spec.post_save_bytes);
        reports[1].observation_hex = Some(bytes_hex(SECOND_SAVE_OBSERVATION));
        reports[1].save_sha256 = Some("d".repeat(64));
        reports[2].phase = WorkerPhase::StateCreate;
        reports[2].state_bytes = spec.state_bytes;
        reports[2].state_sha256 = Some("e".repeat(64));
        reports[2].restored_marker_hex = Some(bytes_hex(STATE_MARKER));
        reports[3].phase = WorkerPhase::StateReload;
        reports[3].state_bytes = spec.state_bytes;
        reports[3].state_sha256 = Some("e".repeat(64));
        reports[3].before_marker_hex = Some(bytes_hex(MUTATED_MARKER));
        reports[3].restored_marker_hex = Some(bytes_hex(STATE_MARKER));
        assert!(validate_reports(system, &hash, spec.version, spec.state_bytes, &reports).is_ok());

        reports[3].diagnostic_rom_sha256 = "f".repeat(64);
        assert!(validate_reports(system, &hash, spec.version, spec.state_bytes, &reports).is_err());
        reports[3].diagnostic_rom_sha256 = reports[0].diagnostic_rom_sha256.clone();
        reports[3].core.version = "unexpected".into();
        assert!(validate_reports(system, &hash, spec.version, spec.state_bytes, &reports).is_err());
        for report in &mut reports {
            report.core.version = "caller-supplied-version".into();
        }
        assert!(
            validate_reports(
                system,
                &hash,
                "caller-supplied-version",
                spec.state_bytes,
                &reports
            )
            .is_ok()
        );
        assert!(validate_reports(system, &hash, spec.version, spec.state_bytes, &reports).is_err());
        for report in &mut reports {
            report.expected_state_bytes = Some(13_758);
        }
        reports[2].state_bytes = Some(13_758);
        reports[3].state_bytes = Some(13_758);
        assert!(
            validate_reports(
                system,
                &hash,
                "caller-supplied-version",
                Some(13_758),
                &reports
            )
            .is_ok()
        );
        assert!(
            validate_reports(
                system,
                &hash,
                "caller-supplied-version",
                spec.state_bytes,
                &reports
            )
            .is_err()
        );
    }
}
