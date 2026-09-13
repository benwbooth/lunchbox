//! Real-core diagnostic, not a replacement libretro implementation. Run only in
//! a fresh helper process with an explicitly trusted core, never inside Qt.
use anyhow::{Context, Result, ensure};
use libloading::Library;
use serde::Serialize;
use std::collections::BTreeMap;
use std::ffi::{CStr, CString, c_char, c_void};
use std::path::{Path, PathBuf};
use std::sync::{
    Mutex,
    atomic::{AtomicBool, AtomicI16, AtomicU16, AtomicU64, Ordering},
};

use crate::libretro_memory_map::{ExactMapping, MemoryMapSnapshot};

static ACTIVE: AtomicBool = AtomicBool::new(false);
static SYSTEM_DIRECTORY: Mutex<Option<CString>> = Mutex::new(None);
static SAVE_DIRECTORY: Mutex<Option<CString>> = Mutex::new(None);
static INSPECTION_OPTIONS: Mutex<Option<crate::libretro_options::OptionEnvironment>> =
    Mutex::new(None);
static MEMORY_MAP: Mutex<Result<Option<MemoryMapSnapshot>, String>> = Mutex::new(Ok(None));
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize, serde::Deserialize)]
#[serde(deny_unknown_fields)]
pub struct InputQuery {
    pub port: u32,
    pub device: u32,
    pub index: u32,
    pub id: u32,
}

struct QueryCapture {
    calls: u64,
    addresses: std::collections::BTreeSet<InputQuery>,
    failure: Option<String>,
}
static INSPECTION_QUERIES: Mutex<QueryCapture> = Mutex::new(QueryCapture {
    calls: 0,
    addresses: std::collections::BTreeSet::new(),
    failure: None,
});
type ControllerChoices = Vec<Vec<ControllerChoice>>;
static CONTROLLER_CHOICES: Mutex<Result<Option<ControllerChoices>, String>> = Mutex::new(Ok(None));

#[derive(Clone, Debug, PartialEq, Eq, Serialize, serde::Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ControllerChoice {
    pub description: String,
    pub id: u32,
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

unsafe fn capture_controller_choices(
    data: *const NativeControllerInfo,
) -> Result<ControllerChoices> {
    ensure!(!data.is_null(), "Core supplied a null controller-info list");
    let mut ports = Vec::new();
    for port in 0..=16 {
        let info = unsafe { &*data.add(port) };
        if info.types.is_null() {
            ensure!(
                info.count == 0,
                "Core controller-info terminator has a nonzero type count"
            );
            return Ok(ports);
        }
        ensure!(
            port < 16 && info.count <= 64,
            "Core controller choices exceed capture limits"
        );
        let mut choices = Vec::new();
        for index in 0..info.count as usize {
            let choice = unsafe { &*info.types.add(index) };
            if choice.description.is_null() {
                // Some released cores include their conventional {NULL, 0}
                // C-array terminator in `count`. Accept only that exact final
                // entry; a null label anywhere else is malformed.
                ensure!(
                    index + 1 == info.count as usize && choice.id == 0,
                    "Core controller choice has an invalid null-label terminator"
                );
                break;
            }
            let mut bytes = Vec::new();
            let mut terminated = false;
            for offset in 0..=1024 {
                let byte = unsafe { *choice.description.add(offset) } as u8;
                if byte == 0 {
                    terminated = true;
                    break;
                }
                bytes.push(byte);
            }
            ensure!(
                terminated && bytes.len() <= 1024,
                "Core controller choice label exceeds capture limit"
            );
            choices.push(ControllerChoice {
                description: String::from_utf8(bytes)
                    .context("Core controller choice is not UTF-8")?,
                id: choice.id,
            });
        }
        ports.push(choices);
    }
    anyhow::bail!("Core controller-info list is not terminated")
}

fn controller_choice_snapshot() -> Result<Option<ControllerChoices>> {
    CONTROLLER_CHOICES
        .lock()
        .map_err(|_| anyhow::anyhow!("Controller choice capture lock poisoned"))?
        .clone()
        .map_err(anyhow::Error::msg)
}
struct DescriptorCapture {
    updates: u64,
    descriptors: Result<Vec<InputDescriptor>, String>,
}
static INPUT_DESCRIPTORS: Mutex<DescriptorCapture> = Mutex::new(DescriptorCapture {
    updates: 0,
    descriptors: Ok(Vec::new()),
});

#[derive(Clone, Debug, Serialize, serde::Deserialize)]
#[serde(deny_unknown_fields)]
pub struct InputDescriptor {
    pub port: u32,
    pub device: u32,
    pub index: u32,
    pub id: u32,
    pub description: String,
}

#[repr(C)]
struct NativeInputDescriptor {
    port: u32,
    device: u32,
    index: u32,
    id: u32,
    description: *const c_char,
}

// The explicitly trusted core owns valid pointers for this callback's duration.
// Bound traversal and copy strings immediately; never retain native pointers.
unsafe fn capture_input_descriptors(
    data: *const NativeInputDescriptor,
) -> Result<Vec<InputDescriptor>> {
    let mut captured = Vec::new();
    for index in 0..4096 {
        let descriptor = unsafe { &*data.add(index) };
        if descriptor.description.is_null() {
            return Ok(captured);
        }
        let mut label = Vec::new();
        let mut terminated = false;
        for offset in 0..=1024 {
            let byte = unsafe { *descriptor.description.add(offset) } as u8;
            if byte == 0 {
                terminated = true;
                break;
            }
            label.push(byte);
        }
        ensure!(
            terminated && label.len() <= 1024,
            "Core input descriptor label exceeds capture limit"
        );
        captured.push(InputDescriptor {
            port: descriptor.port,
            device: descriptor.device,
            index: descriptor.index,
            id: descriptor.id,
            description: String::from_utf8(label)
                .context("Core input descriptor label is not UTF-8")?,
        });
    }
    anyhow::bail!("Core input descriptors have no terminator within capture limit")
}

fn input_descriptor_snapshot() -> Result<(u64, Vec<InputDescriptor>)> {
    let capture = INPUT_DESCRIPTORS
        .lock()
        .map_err(|_| anyhow::anyhow!("Input descriptor capture lock poisoned"))?;
    Ok((
        capture.updates,
        capture.descriptors.clone().map_err(anyhow::Error::msg)?,
    ))
}
// Four NES pad ports plus the Famicom expansion port. Other diagnostics use
// only the first one or two entries.
static PRESSED: [AtomicU16; 5] = [
    AtomicU16::new(0),
    AtomicU16::new(0),
    AtomicU16::new(0),
    AtomicU16::new(0),
    AtomicU16::new(0),
];
// [port 0 LX, LY, RX, RY, port 1 LX, LY, RX, RY].
static ANALOG: [AtomicI16; 8] = [
    AtomicI16::new(0),
    AtomicI16::new(0),
    AtomicI16::new(0),
    AtomicI16::new(0),
    AtomicI16::new(0),
    AtomicI16::new(0),
    AtomicI16::new(0),
    AtomicI16::new(0),
];
static BITMASK: AtomicBool = AtomicBool::new(false);
static POLLS: AtomicU64 = AtomicU64::new(0);
static MASK_REQUESTS: AtomicU64 = AtomicU64::new(0);
static SINGLE_REQUESTS: AtomicU64 = AtomicU64::new(0);
static ANALOG_REQUESTS: AtomicU64 = AtomicU64::new(0);
static ANALOG_REQUESTS_BY_PORT: [AtomicU64; 2] = [AtomicU64::new(0), AtomicU64::new(0)];
static JOYPAD_REQUESTS_BY_PORT: [AtomicU64; 5] = [
    AtomicU64::new(0),
    AtomicU64::new(0),
    AtomicU64::new(0),
    AtomicU64::new(0),
    AtomicU64::new(0),
];

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

// Callback values are static or retained until after core deinitialization.
unsafe extern "C" fn environment(command: u32, data: *mut c_void) -> bool {
    let command = command & !0x10000; // libretro experimental flag
    {
        let Ok(mut options) = INSPECTION_OPTIONS.lock() else {
            return false;
        };
        if let Some(options) = options.as_mut()
            && let Some(result) = unsafe { options.handle(command, data) }
        {
            return result;
        }
    }
    if command == 51 {
        return BITMASK.load(Ordering::Relaxed);
    }
    if data.is_null() {
        return false;
    }
    match command {
        3 => {
            unsafe {
                data.cast::<bool>().write(true);
            }
            true
        }
        9 | 31 => {
            let directory = if command == 9 {
                &SYSTEM_DIRECTORY
            } else {
                &SAVE_DIRECTORY
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
        10 => unsafe { *data.cast::<u32>() <= 2 },
        11 => {
            let captured = unsafe { capture_input_descriptors(data.cast()) }
                .map_err(|error| error.to_string());
            let valid = captured.is_ok();
            let Ok(mut slot) = INPUT_DESCRIPTORS.lock() else {
                return false;
            };
            let Some(updates) = slot.updates.checked_add(1) else {
                slot.descriptors = Err("Input descriptor update count overflowed".into());
                return false;
            };
            slot.updates = updates;
            slot.descriptors = captured;
            valid
        }
        35 => {
            let captured = unsafe { capture_controller_choices(data.cast()) }
                .map(Some)
                .map_err(|error| error.to_string());
            let valid = captured.is_ok();
            let Ok(mut choices) = CONTROLLER_CHOICES.lock() else {
                return false;
            };
            *choices = captured;
            valid
        }
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
        16 | 37 => true, // options/geometry notifications
        15 => {
            let variable = unsafe { &mut *data.cast::<Variable>() };
            if variable.key.is_null() {
                return false;
            }
            let key = unsafe { CStr::from_ptr(variable.key) }.to_bytes();
            variable.value = match key {
                b"mgba_use_bios" => c"OFF".as_ptr(),
                b"mgba_skip_bios" | b"mgba_allow_opposing_directions" => c"ON".as_ptr(),
                b"mgba_idle_optimization" => c"Don't Remove".as_ptr(),
                b"genesis_plus_gx_system_hw" => c"game gear".as_ptr(),
                b"genesis_plus_gx_bios" => c"disabled".as_ptr(),
                b"sameboy_model" => c"Game Boy".as_ptr(),
                b"mesen_shift_buttons_clockwise" => c"disabled".as_ptr(),
                b"system_core_override" => c"Automatic".as_ptr(),
                b"system_gb_bios_enable"
                | b"system_gba_bios_enable"
                | b"system_nds_bios_enable" => c"OFF".as_ptr(),
                _ => std::ptr::null(),
            };
            !variable.value.is_null()
        }
        17 => {
            unsafe {
                data.cast::<bool>().write(false);
            }
            true
        }
        24 => {
            unsafe {
                data.cast::<u64>().write(1 << 1);
            }
            true
        }
        27 => unsafe { crate::libretro_log::install(data) },
        39 | 52 => {
            unsafe {
                data.cast::<u32>().write(0);
            }
            true
        }
        47 => {
            unsafe {
                data.cast::<i32>().write(3);
            }
            true
        }
        _ => unsafe { crate::libretro_vfs::handle_environment(command, data) }.unwrap_or(false),
    }
}
unsafe extern "C" fn input(port: u32, device: u32, index: u32, id: u32) -> i16 {
    if port as usize >= PRESSED.len() {
        return 0;
    }
    match (device, index, id) {
        (1, 0, 256) if BITMASK.load(Ordering::Relaxed) => {
            MASK_REQUESTS.fetch_add(1, Ordering::Relaxed);
            JOYPAD_REQUESTS_BY_PORT[port as usize].fetch_add(1, Ordering::Relaxed);
            PRESSED[port as usize].load(Ordering::Relaxed) as i16
        }
        (1, 0, 0..=15) => {
            SINGLE_REQUESTS.fetch_add(1, Ordering::Relaxed);
            JOYPAD_REQUESTS_BY_PORT[port as usize].fetch_add(1, Ordering::Relaxed);
            i16::from(PRESSED[port as usize].load(Ordering::Relaxed) & (1 << id) != 0)
        }
        // RETRO_DEVICE_ANALOG, left/right index, X/Y id.
        (5, 0..=1, 0..=1) if port < 2 => {
            ANALOG_REQUESTS.fetch_add(1, Ordering::Relaxed);
            ANALOG_REQUESTS_BY_PORT[port as usize].fetch_add(1, Ordering::Relaxed);
            let axis = port as usize * 4 + index as usize * 2 + id as usize;
            ANALOG[axis].load(Ordering::Relaxed)
        }
        _ => 0,
    }
}
unsafe extern "C" fn poll() {
    POLLS.fetch_add(1, Ordering::Relaxed);
}
unsafe extern "C" fn video(_: *const c_void, _: u32, _: u32, _: usize) {}
unsafe extern "C" fn audio(_: i16, _: i16) {}
unsafe extern "C" fn audio_batch(_: *const i16, frames: usize) -> usize {
    frames
}

struct Lease;
impl Drop for Lease {
    fn drop(&mut self) {
        if let Ok(mut directory) = SYSTEM_DIRECTORY.lock() {
            *directory = None;
        }
        if let Ok(mut directory) = SAVE_DIRECTORY.lock() {
            *directory = None;
        }
        if let Ok(mut options) = INSPECTION_OPTIONS.lock() {
            *options = None;
        }
        if let Ok(mut memory_map) = MEMORY_MAP.lock() {
            *memory_map = Ok(None);
        }
        ACTIVE.store(false, Ordering::Release);
    }
}

// Some native cores use C stdio directly (Snes9x 1.63 prints its selected ROM
// map). Keep the helper's stdout machine-readable; diagnostics still reach
// stderr and the structured report. The core is dropped before this guard.
#[cfg(unix)]
struct NativeStdoutSilencer {
    saved_fd: std::os::fd::RawFd,
}

#[cfg(unix)]
impl NativeStdoutSilencer {
    fn new() -> Result<Self> {
        use std::io::Write;
        use std::os::fd::AsRawFd;

        std::io::stdout().flush()?;
        unsafe {
            libc::fflush(std::ptr::null_mut());
        }
        let saved_fd = unsafe { libc::dup(libc::STDOUT_FILENO) };
        ensure!(saved_fd >= 0, "Could not preserve diagnostic stdout");
        let null = std::fs::OpenOptions::new().write(true).open("/dev/null")?;
        if unsafe { libc::dup2(null.as_raw_fd(), libc::STDOUT_FILENO) } < 0 {
            unsafe {
                libc::close(saved_fd);
            }
            anyhow::bail!("Could not isolate native-core stdout");
        }
        Ok(Self { saved_fd })
    }
}

#[cfg(unix)]
impl Drop for NativeStdoutSilencer {
    fn drop(&mut self) {
        unsafe {
            libc::fflush(std::ptr::null_mut());
            libc::dup2(self.saved_fd, libc::STDOUT_FILENO);
            libc::close(self.saved_fd);
        }
    }
}

struct Core {
    _library: Library,
    deinit: unsafe extern "C" fn(),
    unload: unsafe extern "C" fn(),
    run: unsafe extern "C" fn(),
    memory: unsafe extern "C" fn(u32) -> *mut c_void,
    memory_size: unsafe extern "C" fn(u32) -> usize,
    serialize: unsafe extern "C" fn(*mut c_void, usize) -> bool,
    serialize_size: unsafe extern "C" fn() -> usize,
    set_device: unsafe extern "C" fn(u32, u32),
    initialized: bool,
    loaded: bool,
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
    }
}

/// Original ARM program: continuously copy KEYINPUT to EWRAM, alongside a
/// completion marker. No Nintendo logo, firmware, or game bytes are included.
pub fn gba_diagnostic_rom() -> Vec<u8> {
    let mut rom = vec![0; 0x1000];
    rom[..4].copy_from_slice(&0xea00002eu32.to_le_bytes()); // b 0x080000c0
    rom[0xa0..0xac].copy_from_slice(b"LUNCHBOXTEST");
    rom[0xb2] = 0x96; // format marker, not logo data
    rom[0xbd] = rom[0xa0..0xbd]
        .iter()
        .fold(0u8, |sum, b| sum.wrapping_sub(*b))
        .wrapping_sub(0x19);
    for (i, word) in [
        0xe59f0018u32, // ldr r0, [pc, #24] -> KEYINPUT address at e0
        0xe59f2018,    // ldr r2, [pc, #24] -> EWRAM address at e4
        0xe59f3018,    // ldr r3, [pc, #24] -> marker at e8
        0xe5823004,    // str r3, [r2, #4]
        0xe1d010b0,    // loop: ldrh r1, [r0]
        0xe1c210b0,    // strh r1, [r2]
        0xeafffffc,    // b loop
        0xe1a00000,    // nop (literal alignment)
        0x04000130,
        0x02000000,
        0x4c42494e,
    ]
    .into_iter()
    .enumerate()
    {
        rom[0xc0 + i * 4..0xc4 + i * 4].copy_from_slice(&word.to_le_bytes());
    }
    rom
}

/// Original 6502/NROM program: latch the NES controller ports, sample their
/// serial bits, and publish active-low bytes plus `LBN` in zero-page RAM.
/// Bytes 0 and 4 are players 1 and 2; bytes 5 and 6 are the second bytes from
/// the two ports (players 3 and 4 when Four Score is enabled). The ROM contains
/// no Nintendo program, logo, firmware, or game data.
pub fn nes_diagnostic_rom() -> Vec<u8> {
    const HEADER_SIZE: usize = 16;
    const PRG_SIZE: usize = 16 * 1024;
    const CHR_SIZE: usize = 8 * 1024;
    let mut rom = vec![0; HEADER_SIZE + PRG_SIZE + CHR_SIZE];
    rom[..16].copy_from_slice(&[b'N', b'E', b'S', 0x1a, 1, 1, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0]);
    let program = [
        0x78, // sei
        0xd8, // cld
        0xa2, 0xff, // ldx #$ff
        0x9a, // txs
        0xa9, 0x00, // lda #0
        0x8d, 0x00, 0x20, // sta $2000
        0x8d, 0x01, 0x20, // sta $2001
        0xa9, b'L', 0x85, 0x01, // marker $0001..$0003 = LBN
        0xa9, b'B', 0x85, 0x02, 0xa9, b'N', 0x85, 0x03, 0xa9, 0x01, // poll: lda #1
        0x8d, 0x16, 0x40, // sta $4016
        0xa9, 0x00, 0x8d, 0x16, 0x40, // sta $4016
        0x85, 0x07, // clear player 1 accumulator
        0xa2, 0x00, // ldx #0
        0xad, 0x16, 0x40, // read: lda $4016
        0x4a, // lsr a; serial bit -> carry
        0x66, 0x07, // ror $07; A ends at bit 0 after eight reads
        0xe8, // inx
        0xe0, 0x08, // cpx #8
        0xd0, 0xf5, // bne read
        0xa5, 0x07, 0x49, 0xff, 0x85, 0x07, // make player 1 active-low
        0xa9, 0x00, 0x85, 0x08, // clear player 2 accumulator
        0xa2, 0x00, // ldx #0
        0xad, 0x17, 0x40, // read2: lda $4017
        0x4a, 0x66, 0x08, 0xe8, 0xe0, 0x08, // lsr; ror $08; inx; cpx #8
        0xd0, 0xf5, // bne read2
        0xa5, 0x08, 0x49, 0xff, 0x85, 0x08, // make player 2 active-low
        0xa9, 0x00, 0x85, 0x09, // clear player 3 accumulator
        0xa2, 0x00, // ldx #0
        0xad, 0x16, 0x40, // read3: lda $4016 (second serial byte)
        0x4a, 0x66, 0x09, 0xe8, 0xe0, 0x08, // lsr; ror $09; inx; cpx #8
        0xd0, 0xf5, // bne read3
        0xa5, 0x09, 0x49, 0xff, 0x85, 0x09, // make player 3 active-low
        0xa9, 0x00, 0x85, 0x0a, // clear player 4 accumulator
        0xa2, 0x00, // ldx #0
        0xad, 0x17, 0x40, // read4: lda $4017 (second serial byte)
        0x4a, 0x66, 0x0a, 0xe8, 0xe0, 0x08, // lsr; ror $0a; inx; cpx #8
        0xd0, 0xf5, // bne read4
        0xa5, 0x0a, 0x49, 0xff, 0x85, 0x0a, // make player 4 active-low
        // Publish only complete samples. This keeps frontend snapshots out of
        // the transient shift-accumulation phase of the tight polling loop.
        0xa5, 0x07, 0x85, 0x00, // player 1
        0xa5, 0x08, 0x85, 0x04, // player 2
        0xa5, 0x09, 0x85, 0x05, // player 3
        0xa5, 0x0a, 0x85, 0x06, // player 4
        0x4c, 0x19, 0x80, // jmp poll
    ];
    rom[HEADER_SIZE..HEADER_SIZE + program.len()].copy_from_slice(&program);
    for vector in [0x3ffa, 0x3ffc, 0x3ffe] {
        rom[HEADER_SIZE + vector..HEADER_SIZE + vector + 2]
            .copy_from_slice(&0x8000u16.to_le_bytes());
    }
    rom
}

const SNES_MARKER: &[u8; 16] = b"LBN-SNES-INPUT!!";
const SNES_GENERATION_OFFSET: usize = 0x2e;
const SNES_MARKER_OFFSET: usize = 0x30;
const SNES_RESULTS_OFFSET: usize = 0x40;

/// Original 65C816 LoROM program that manually clocks both SNES controller
/// ports. It also selects both halves of a port-two Super Multitap, publishing
/// five completed 16-bit samples to WRAM behind an even generation counter.
pub fn snes_diagnostic_rom() -> Vec<u8> {
    fn branch_back(program: &mut Vec<u8>, target: usize) {
        let from = program.len() + 2;
        let displacement = isize::try_from(target).unwrap() - isize::try_from(from).unwrap();
        assert!((-128..=127).contains(&displacement));
        program.extend([0xd0, displacement as i8 as u8]); // bne target
    }

    let mut program = vec![
        0x78, // sei
        0xd8, // cld
        0xa2,
        0xff, // ldx #$ff
        0x9a, // txs
        0xa9,
        0x00,
        0x8d,
        0x00,
        0x42, // disable NMI/auto joypad
        0x85,
        SNES_GENERATION_OFFSET as u8, // generation = 0
    ];
    for (index, &byte) in SNES_MARKER.iter().enumerate() {
        program.extend([
            0xa9,
            byte,
            0x85,
            u8::try_from(SNES_MARKER_OFFSET + index).unwrap(),
        ]);
    }

    let poll = program.len();
    program.extend([
        0xa9, 0x01, 0x8d, 0x16, 0x40, // latch high
        0xa9, 0x00, 0x8d, 0x16, 0x40, // latch low
        0xa9, 0x80, 0x8d, 0x01, 0x42, // multitap select: players 2/3
        0xa9, 0x00,
    ]);
    for address in 0x60..=0x69 {
        program.extend([0x85, address]); // clear five temporary words
    }
    program.extend([0xa2, 0x00]); // ldx #0
    let first_pair = program.len();
    program.extend([
        0xad, 0x16, 0x40, // player 1 serial bit
        0x4a, 0x66, 0x61, 0x66, 0x60, // lsr; ror high; ror low
        0xad, 0x17, 0x40, 0x85, 0x6a, // players 2/3 serial bits
        0x4a, 0x66, 0x63, 0x66, 0x62, // bit 0 -> player 2
        0xa5, 0x6a, 0x4a, 0x4a, 0x66, 0x65, 0x66, 0x64, // bit 1 -> player 3
        0xe8, 0xe0, 0x10, // inx; cpx #16
    ]);
    branch_back(&mut program, first_pair);
    program.extend([
        0xa9, 0x00, 0x8d, 0x01, 0x42, // multitap select: players 4/5
        0xa2, 0x00, // ldx #0
    ]);
    let second_pair = program.len();
    program.extend([
        0xad, 0x17, 0x40, 0x85, 0x6a, // players 4/5 serial bits
        0x4a, 0x66, 0x67, 0x66, 0x66, // bit 0 -> player 4
        0xa5, 0x6a, 0x4a, 0x4a, 0x66, 0x69, 0x66, 0x68, // bit 1 -> player 5
        0xe8, 0xe0, 0x10, // inx; cpx #16
    ]);
    branch_back(&mut program, second_pair);
    program.extend([0xe6, SNES_GENERATION_OFFSET as u8]); // odd: publishing
    for index in 0..10 {
        program.extend([
            0xa5,
            u8::try_from(0x60 + index).unwrap(),
            0x85,
            u8::try_from(SNES_RESULTS_OFFSET + index).unwrap(),
        ]);
    }
    program.extend([
        0xe6,
        SNES_GENERATION_OFFSET as u8, // even: complete
        0xad,
        0x12,
        0x42,
        0x30,
        0xfb, // wait while already in vertical blank
        0xad,
        0x12,
        0x42,
        0x10,
        0xfb, // wait for the next vertical blank
        0x4c,
    ]);
    program.extend(u16::try_from(0x8000 + poll).unwrap().to_le_bytes());

    assert!(program.len() < 0x7fc0);
    let mut rom = vec![0u8; 32 * 1024];
    rom[..program.len()].copy_from_slice(&program);
    rom[0x7fc0..0x7fd5].copy_from_slice(b"LUNCHBOX SNES INPUT  ");
    rom[0x7fd5..0x7fdc].copy_from_slice(&[0x20, 0x00, 0x08, 0x00, 0x01, 0x33, 0x00]);
    for vector in (0x7fe4..0x8000).step_by(2) {
        rom[vector..vector + 2].copy_from_slice(&0x8000u16.to_le_bytes());
    }
    let checksum = rom
        .iter()
        .fold(0u16, |sum, &byte| sum.wrapping_add(u16::from(byte)))
        .wrapping_add(0x01fe);
    rom[0x7fdc..0x7fde].copy_from_slice(&(!checksum).to_le_bytes());
    rom[0x7fde..0x7fe0].copy_from_slice(&checksum.to_le_bytes());
    rom
}

/// Original Z80 loop: sample Game Gear ports DC (directions/buttons) and 00
/// (Start), and store both in work RAM with the same execution marker as GBA.
pub fn gamegear_diagnostic_rom() -> Vec<u8> {
    let mut rom = vec![0; 0x8000];
    let program = [
        0xf3, // di
        0xdb, 0xdc, // loop: in a, (dc)
        0x32, 0x00, 0xc0, // ld (c000), a
        0xdb, 0x00, // in a, (00)
        0x32, 0x01, 0xc0, // ld (c001), a
        0x21, 0x4e, 0x49, // ld hl, 494e
        0x22, 0x04, 0xc0, // ld (c004), hl
        0x21, 0x42, 0x4c, // ld hl, 4c42
        0x22, 0x06, 0xc0, // ld (c006), hl
        0xc3, 0x01, 0x00, // jp loop
    ];
    rom[..program.len()].copy_from_slice(&program);
    rom[0x7ff0..0x7ff8].copy_from_slice(b"TMR SEGA"); // cartridge format signature
    rom[0x7fff] = 0x6c; // international Game Gear, 32 KiB
    let checksum = rom[..0x7ff0]
        .iter()
        .fold(0u16, |sum, byte| sum.wrapping_add(u16::from(*byte)));
    rom[0x7ffa..0x7ffc].copy_from_slice(&checksum.to_le_bytes());
    rom
}

/// Original LR35902 loop. Sample the two active-low JOYP nibbles separately:
/// buttons at C000 and directions at C001. The 48-byte cartridge-format logo
/// signature is required by several independent cores; no boot firmware or
/// game program is embedded.
pub fn gameboy_diagnostic_rom() -> Vec<u8> {
    const CARTRIDGE_LOGO: [u8; 48] = [
        0xce, 0xed, 0x66, 0x66, 0xcc, 0x0d, 0x00, 0x0b, 0x03, 0x73, 0x00, 0x83, 0x00, 0x0c, 0x00,
        0x0d, 0x00, 0x08, 0x11, 0x1f, 0x88, 0x89, 0x00, 0x0e, 0xdc, 0xcc, 0x6e, 0xe6, 0xdd, 0xdd,
        0xd9, 0x99, 0xbb, 0xbb, 0x67, 0x63, 0x6e, 0x0e, 0xec, 0xcc, 0xdd, 0xdc, 0x99, 0x9f, 0xbb,
        0xb9, 0x33, 0x3e,
    ];
    let mut rom = vec![0; 0x8000];
    rom[0x100..0x104].copy_from_slice(&[0x00, 0xc3, 0x50, 0x01]); // nop; jp 0150
    rom[0x104..0x134].copy_from_slice(&CARTRIDGE_LOGO);
    rom[0x134..0x141].copy_from_slice(b"LUNCHBOXINPUT");
    let program = [
        0xf3, // di
        0x21, 0x04, 0xc0, // ld hl, c004 (execution marker)
        0x36, 0x4e, 0x23, 0x36, 0x49, 0x23, 0x36, 0x42, 0x23, 0x36, 0x4c, 0x3e,
        0x10, // loop at 015f: ld a, 10 (select buttons)
        0xe0, 0x00, // ldh (00), a
        0xf0, 0x00, 0xf0, 0x00, 0xf0, 0x00, 0xf0, 0x00, // settle then sample
        0xea, 0x00, 0xc0, // ld (c000), a
        0x3e, 0x20, // ld a, 20 (select directions)
        0xe0, 0x00, 0xf0, 0x00, 0xf0, 0x00, 0xf0, 0x00, 0xf0, 0x00, 0xea, 0x01,
        0xc0, // ld (c001), a
        0xc3, 0x5f, 0x01, // jp loop
    ];
    rom[0x150..0x150 + program.len()].copy_from_slice(&program);
    rom[0x14d] = rom[0x134..0x14d]
        .iter()
        .fold(0u8, |sum, byte| sum.wrapping_sub(*byte).wrapping_sub(1));
    let checksum = rom
        .iter()
        .fold(0u16, |sum, byte| sum.wrapping_add(u16::from(*byte)));
    rom[0x14e..0x150].copy_from_slice(&checksum.to_be_bytes());
    rom
}

/// Original 6507 loop for a 4 KiB Atari 2600 cartridge. It samples both
/// joystick ports, both trigger inputs, and the console switches into RIOT RAM.
/// The core exposes that 128-byte RAM through `RETRO_MEMORY_SYSTEM_RAM`, so the
/// frontend can verify hardware-visible input without inspecting Stella's
/// internal event state.
pub fn atari2600_diagnostic_rom() -> Vec<u8> {
    let mut rom = vec![0xea; 4 * 1024]; // NOP-filled 4K cartridge
    let program = [
        0x78, // sei
        0xd8, // cld
        0xa9, b'L', 0x85, 0x84, // marker $84..$87 = LB26
        0xa9, b'B', 0x85, 0x85, 0xa9, b'2', 0x85, 0x86, 0xa9, b'6', 0x85, 0x87, 0xad, 0x80, 0x02,
        0x85, 0x80, // loop: SWCHA -> $80
        0xad, 0x0c, 0x00, 0x85, 0x81, // INPT4 -> $81
        0xad, 0x0d, 0x00, 0x85, 0x82, // INPT5 -> $82
        0xad, 0x82, 0x02, 0x85, 0x83, // SWCHB -> $83
        // Emit an ordinary 262-scanline frame. Depending on an emulator's
        // runaway-scanline guard made the same ROM return quickly on one host
        // while remaining inside a single retro_run call on another.
        0xa9, 0x02, 0x85, 0x00, // VSYNC high
        0xa2, 0x03, // three VSYNC scanlines
        0x85, 0x02, 0xca, 0xd0, 0xfb, // sta WSYNC; dex; bne
        0xa9, 0x00, 0x85, 0x00, // VSYNC low
        0xa2, 0x00, // 256 scanlines
        0x85, 0x02, 0xca, 0xd0, 0xfb, 0xa0, 0x03, // final three scanlines
        0x85, 0x02, 0x88, 0xd0, 0xfb, 0x4c, 0x12, 0xf0, // jmp loop
    ];
    rom[..program.len()].copy_from_slice(&program);
    for vector in [0x0ffa, 0x0ffc, 0x0ffe] {
        rom[vector..vector + 2].copy_from_slice(&0xf000u16.to_le_bytes());
    }
    rom
}

fn mips_i(op: u32, rs: u32, rt: u32, immediate: i16) -> u32 {
    (op << 26) | (rs << 21) | (rt << 16) | u32::from(immediate as u16)
}

fn mips_r(rs: u32, rt: u32, rd: u32, shift: u32, function: u32) -> u32 {
    (rs << 21) | (rt << 16) | (rd << 11) | (shift << 6) | function
}

fn mips_j(op: u32, address: u32) -> u32 {
    (op << 26) | ((address >> 2) & 0x03ff_ffff)
}

/// Original MIPS R3000 program. It asks the real SCPH BIOS controller driver to
/// poll both physical ports into 34-byte buffers at 0x80020020/0x80020060.
///
/// The executable format and I/O protocol are independently reviewable; no
/// Sony program or game bytes are embedded. Packet semantics are cross-checked
/// against the pinned Beetle source revision used by the runtime diagnostic:
/// <https://github.com/libretro/beetle-psx-libretro/tree/82d8e051d1c7741a18d930be90e458b48abaa9a1>
pub fn psx_diagnostic_exe() -> Vec<u8> {
    // Register aliases used below.
    const ZERO: u32 = 0;
    const A0: u32 = 4;
    const A1: u32 = 5;
    const A2: u32 = 6;
    const A3: u32 = 7;
    const T0: u32 = 8;
    const T1: u32 = 9;
    const T2: u32 = 10;
    const S1: u32 = 17;
    const RA: u32 = 31;
    const BASE: u32 = 0x8001_0000;

    let mut words = Vec::new();
    words.extend([
        mips_i(0x0f, ZERO, S1, -32766), // lui s1, 0x8002 (result RAM)
        mips_i(0x0f, ZERO, T0, 0x5350), // marker bytes "LBPS"
        mips_i(0x0d, T0, T0, 0x424c),
        mips_i(0x2b, S1, T0, 0),
        mips_i(0x09, S1, A0, 0x20), // InitPAD(port1, 34, port2, 34)
        mips_i(0x09, ZERO, A1, 34),
        mips_i(0x09, S1, A2, 0x60),
        mips_i(0x09, ZERO, A3, 34),
        mips_i(0x09, ZERO, T2, 0x00b0),
        mips_i(0x09, ZERO, T1, 0x0012),
        mips_r(T2, ZERO, RA, 0, 0x09), // InitPAD(port1, 34, port2, 34)
        0,
        mips_i(0x09, ZERO, T1, 0x0013),
        mips_i(0x09, ZERO, T2, 0x00b0),
        mips_r(T2, ZERO, RA, 0, 0x09), // StartPAD()
        0,
        mips_i(0x09, ZERO, T1, 0x005b),
        mips_i(0x09, ZERO, A0, 0),
        mips_i(0x09, ZERO, T2, 0x00b0),
        mips_r(T2, ZERO, RA, 0, 0x09), // ChangeClearPAD(0)
        0,
    ]);
    let main_loop = words.len();
    words.extend([
        mips_i(0x23, S1, T0, 4), // generation++
        0,
        mips_i(0x09, T0, T0, 1),
        mips_i(0x2b, S1, T0, 4),
        mips_j(0x02, BASE + u32::try_from(main_loop * 4).unwrap()),
        0,
    ]);

    assert!(words.len() * 4 <= 0x800);
    let mut exe = vec![0; 0x1000];
    exe[..8].copy_from_slice(b"PS-X EXE");
    exe[0x10..0x14].copy_from_slice(&BASE.to_le_bytes());
    exe[0x18..0x1c].copy_from_slice(&BASE.to_le_bytes());
    exe[0x1c..0x20].copy_from_slice(&0x800u32.to_le_bytes());
    exe[0x30..0x34].copy_from_slice(&0x801f_ff00u32.to_le_bytes());
    exe[0x4c..0x63].copy_from_slice(b"LUNCHBOX PSX INPUT TEST");
    for (index, word) in words.into_iter().enumerate() {
        exe[0x800 + index * 4..0x804 + index * 4].copy_from_slice(&word.to_le_bytes());
    }
    exe
}

#[derive(Clone, Copy, Debug, clap::ValueEnum)]
pub enum Diagnostic {
    Gba,
    Gamegear,
    Gameboy,
    Atari2600,
    Nes,
    Snes,
    Psx,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, clap::ValueEnum)]
pub enum NesTopology {
    #[default]
    TwoPlayer,
    FourScore,
}

impl NesTopology {
    fn connected_ports(self) -> usize {
        match self {
            Self::TwoPlayer => 2,
            Self::FourScore => 4,
        }
    }

    fn name(self) -> &'static str {
        match self {
            Self::TwoPlayer => "two-player",
            Self::FourScore => "four-score",
        }
    }
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, clap::ValueEnum)]
pub enum SnesTopology {
    #[default]
    TwoPlayer,
    Multitap,
}

impl SnesTopology {
    fn connected_ports(self) -> usize {
        match self {
            Self::TwoPlayer => 2,
            Self::Multitap => 5,
        }
    }

    fn name(self) -> &'static str {
        match self {
            Self::TwoPlayer => "two-player",
            Self::Multitap => "port-two-multitap",
        }
    }
}

impl Diagnostic {
    fn identity(self) -> (&'static str, &'static str, u32, u16) {
        match self {
            Self::Gba => ("mGBA or VBA-M", "input.gba", 1, 0x3ff),
            Self::Gamegear => ("Genesis Plus GX", "input.gg", 769, 0x803f),
            // The exact device is selected from the loaded core's contract.
            Self::Gameboy => (
                "Gambatte, mGBA, SameBoy, SkyEmu, or VBA-M",
                "input.gb",
                1,
                0x0f0f,
            ),
            Self::Atari2600 => ("Stella", "input.bin", 1, 0),
            // NES supports two exact core identities whose advertised explicit
            // standard-controller subclasses differ. Select that device only
            // after querying the loaded core identity.
            Self::Nes => ("FCEUmm or Mesen", "input.nes", 0, 0x00ff),
            Self::Snes => ("bsnes, Snes9x, or Mesen-S", "input.sfc", 0, 0x0fff),
            Self::Psx => ("Beetle PSX", "input.exe", 517, 0xffff),
        }
    }

    fn cases(self) -> &'static [(&'static str, u16, u16)] {
        match self {
            Self::Gba => &[
                ("released", 0, 0),
                ("A", 1 << 8, 1 << 0),
                ("B", 1 << 0, 1 << 1),
                ("Select", 1 << 2, 1 << 2),
                ("Start", 1 << 3, 1 << 3),
                ("Right", 1 << 7, 1 << 4),
                ("Left", 1 << 6, 1 << 5),
                ("Up", 1 << 4, 1 << 6),
                ("Down", 1 << 5, 1 << 7),
                ("R", 1 << 11, 1 << 8),
                ("L", 1 << 10, 1 << 9),
                ("A+B", (1 << 8) | 1, 3),
                ("L+R", (1 << 10) | (1 << 11), 0x300),
            ],
            // Low byte is DC bits 0..5; high byte is 00 bit 7 (Start).
            // Unused region/link bits are not gameplay inputs.
            Self::Gamegear => &[
                ("released", 0, 0),
                ("1", 1, 1 << 4),
                ("2", 1 << 8, 1 << 5),
                ("Start", 1 << 3, 1 << 15),
                ("Up", 1 << 4, 1),
                ("Down", 1 << 5, 1 << 1),
                ("Left", 1 << 6, 1 << 2),
                ("Right", 1 << 7, 1 << 3),
                ("1+2", 1 | (1 << 8), 0x30),
                ("1+Start", 1 | (1 << 3), 0x8010),
                ("Select is unassigned", 1 << 2, 0),
            ],
            // JOYP buttons in the low byte, directions in the high byte.
            Self::Gameboy => &[
                ("released", 0, 0),
                ("A", 1 << 8, 1),
                ("B", 1, 1 << 1),
                ("Select", 1 << 2, 1 << 2),
                ("Start", 1 << 3, 1 << 3),
                ("Right", 1 << 7, 1 << 8),
                ("Left", 1 << 6, 1 << 9),
                ("Up", 1 << 4, 1 << 10),
                ("Down", 1 << 5, 1 << 11),
                ("A+B", (1 << 8) | 1, 3),
                ("A+Right", (1 << 8) | (1 << 7), 0x101),
                ("Shoulders are unassigned", (1 << 10) | (1 << 11), 0),
            ],
            Self::Atari2600 => &[],
            Self::Nes => &[
                ("released", 0, 0),
                ("A", 1 << 8, 1 << 0),
                ("B", 1 << 0, 1 << 1),
                ("Select", 1 << 2, 1 << 2),
                ("Start", 1 << 3, 1 << 3),
                ("Up", 1 << 4, 1 << 4),
                ("Down", 1 << 5, 1 << 5),
                ("Left", 1 << 6, 1 << 6),
                ("Right", 1 << 7, 1 << 7),
                ("A+B", (1 << 8) | 1, 3),
                ("Up+Left", (1 << 4) | (1 << 6), 0x50),
            ],
            Self::Snes => &[
                ("released", 0, 0),
                ("B", 1 << 0, 1 << 0),
                ("Y", 1 << 1, 1 << 1),
                ("Select", 1 << 2, 1 << 2),
                ("Start", 1 << 3, 1 << 3),
                ("Up", 1 << 4, 1 << 4),
                ("Down", 1 << 5, 1 << 5),
                ("Left", 1 << 6, 1 << 6),
                ("Right", 1 << 7, 1 << 7),
                ("A", 1 << 8, 1 << 8),
                ("X", 1 << 9, 1 << 9),
                ("L", 1 << 10, 1 << 10),
                ("R", 1 << 11, 1 << 11),
                ("B+Y", (1 << 0) | (1 << 1), 3),
                ("Up+Left", (1 << 4) | (1 << 6), 0x50),
            ],
            Self::Psx => &[],
        }
    }
}

#[derive(Debug, Serialize)]
pub struct Observation {
    pub name: String,
    pub retropad_mask: u16,
    pub expected_keyinput: u16,
    pub observed_keyinput: u16,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, serde::Deserialize)]
#[serde(deny_unknown_fields)]
pub struct FirmwareIdentity {
    pub filename: String,
    pub sha256: String,
    pub bytes: u64,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, serde::Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RuntimeLibraryIdentity {
    pub path: PathBuf,
    pub sha256: String,
}

#[derive(Debug, Serialize)]
pub struct PsxObservation {
    pub port: u32,
    pub controller_device: u32,
    pub name: String,
    pub retropad_mask: u16,
    /// Left X, left Y, right X, right Y in libretro's signed axis domain.
    pub analog: [i16; 4],
    /// Meaningful native BIOS `InitPAD` response bytes. Standard pads have
    /// status, type/length, and buttons; DualShock adds RX, RY, LX, and LY.
    pub expected_pad_buffer: Vec<u8>,
    pub observed_pad_buffer: Vec<u8>,
}

#[derive(Debug, Serialize)]
pub struct NesObservation {
    pub topology: &'static str,
    /// One-based port under test.
    pub target_port: u32,
    pub controller_device: u32,
    pub name: String,
    /// One mask and active-low register byte per connected player.
    pub retropad_masks: Vec<u16>,
    pub expected_registers: Vec<u8>,
    pub observed_registers: Vec<u8>,
}

#[derive(Debug, Serialize)]
pub struct SnesObservation {
    pub topology: &'static str,
    pub target_port: u32,
    pub name: String,
    pub retropad_masks: Vec<u16>,
    pub expected_registers: Vec<u16>,
    pub observed_registers: Vec<u16>,
    pub readback: &'static str,
}

#[derive(Debug, Serialize)]
pub struct Atari2600Observation {
    /// One-based joystick port under test; zero denotes a console-switch case.
    pub target_port: u32,
    pub name: String,
    pub retropad_masks: [u16; 2],
    /// SWCHA, INPT4, INPT5, SWCHB as sampled by the original 6507 program.
    pub expected_registers: [u8; 4],
    pub observed_registers: [u8; 4],
    /// Relevant bits in each register. Unrelated switch/TIA bits are ignored.
    pub comparison_masks: [u8; 4],
}

#[derive(Debug, Serialize, serde::Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CoreIdentity {
    pub core_sha256: String,
    pub core_name: String,
    pub core_version: String,
    pub valid_extensions: String,
    pub need_fullpath: bool,
    pub block_extract: bool,
}

#[derive(Debug, Serialize)]
pub struct Report {
    pub schema_version: u32,
    pub diagnostic: &'static str,
    pub core_sha256: String,
    pub core_name: String,
    pub core_version: String,
    pub input_descriptors: Vec<InputDescriptor>,
    pub input_descriptor_updates: u64,
    pub controller_choices: Option<ControllerChoices>,
    pub input_mode: &'static str,
    pub input_polls: u64,
    pub bitmask_requests: u64,
    pub individual_requests: u64,
    pub analog_requests: u64,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub joypad_requests_by_port: Vec<u64>,
    pub reported_system_ram_bytes: usize,
    pub observation_memory_source: &'static str,
    pub observation_memory_bytes: usize,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub observation_memory_address: Option<usize>,
    pub observations: Vec<Observation>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub nes_observations: Vec<NesObservation>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub snes_observations: Vec<SnesObservation>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub atari2600_observations: Vec<Atari2600Observation>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub psx_observations: Vec<PsxObservation>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub firmware: Vec<FirmwareIdentity>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub runtime_libraries: Vec<RuntimeLibraryIdentity>,
    #[serde(skip_serializing_if = "Option::is_none")]
    /// Reference used to define expectations, not a provenance assertion about
    /// an arbitrary caller-supplied core binary. Its actual hash/version above
    /// remain the runtime evidence.
    pub contract_source_revision: Option<&'static str>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub contract_source_url: Option<&'static str>,
}

#[derive(Clone, Copy)]
struct NesCoreContract {
    controller_device: u32,
    controller_label: &'static str,
    source_revision: &'static str,
    source_url: &'static str,
}

#[derive(Clone, Copy)]
struct GameboyCoreContract {
    device_modes: &'static [u32],
    supports_bitmask: bool,
    system_ram_offset: usize,
    source_revision: &'static str,
    source_url: &'static str,
}

#[derive(Clone, Copy)]
struct GbaCoreContract {
    supports_bitmask: bool,
    observation_memory: GbaObservationMemory,
    source_revision: &'static str,
    source_url: &'static str,
}

#[derive(Clone, Copy)]
enum GbaObservationMemory {
    SystemRam { bytes: usize, offset: usize },
    MemoryMap(ExactMapping),
}

fn gba_core_contract(core_name: &str) -> Result<GbaCoreContract> {
    match core_name {
        "mGBA" => Ok(GbaCoreContract {
            supports_bitmask: true,
            observation_memory: GbaObservationMemory::SystemRam {
                bytes: 32 * 1024,
                offset: 0,
            },
            source_revision: "e31759b24e7a4e3899285ff720d7b573ac328ae7",
            source_url: "https://github.com/libretro/mgba/blob/e31759b24e7a4e3899285ff720d7b573ac328ae7/src/platform/libretro/libretro.c",
        }),
        "VBA-M" => Ok(GbaCoreContract {
            supports_bitmask: true,
            observation_memory: GbaObservationMemory::SystemRam {
                bytes: 256 * 1024,
                offset: 0,
            },
            source_revision: "115defb3a318258ab84746d45258a1aec19d0b4b",
            source_url: "https://github.com/libretro/vbam-libretro/blob/115defb3a318258ab84746d45258a1aec19d0b4b/src/libretro/libretro.cpp",
        }),
        "SkyEmu" => Ok(GbaCoreContract {
            supports_bitmask: false,
            observation_memory: GbaObservationMemory::MemoryMap(ExactMapping {
                address: 0x0200_0000,
                bytes: 256 * 1024,
                select: 0xff00_0000,
            }),
            source_revision: "adacd0788964ed89f5c43dcbc1f3cc26deec996c",
            source_url: "https://github.com/skylersaleh/SkyEmu/blob/adacd0788964ed89f5c43dcbc1f3cc26deec996c/src/libretro.c",
        }),
        _ => anyhow::bail!("GBA diagnostic supports only exact mGBA, VBA-M, or SkyEmu identities"),
    }
}

unsafe fn gba_observation_memory(core: &Core, contract: GbaCoreContract) -> Result<&[u8]> {
    match contract.observation_memory {
        GbaObservationMemory::SystemRam { bytes, offset } => {
            ensure!(
                unsafe { (core.memory_size)(2) } == bytes,
                "Unexpected exposed GBA RAM size: {}",
                unsafe { (core.memory_size)(2) }
            );
            ensure!(
                offset + 8 <= bytes,
                "GBA diagnostic window exceeds system RAM"
            );
            let pointer = unsafe { (core.memory)(2) }.cast::<u8>();
            ensure!(!pointer.is_null(), "No system RAM exposed by core");
            Ok(unsafe { std::slice::from_raw_parts(pointer.add(offset), bytes - offset) })
        }
        GbaObservationMemory::MemoryMap(expected) => {
            ensure!(
                unsafe { (core.memory_size)(2) } == 0 && unsafe { (core.memory)(2) }.is_null(),
                "SkyEmu GBA system-memory ABI changed; re-audit its exact contract"
            );
            let snapshot = MEMORY_MAP
                .lock()
                .map_err(|_| anyhow::anyhow!("Memory-map capture lock poisoned"))?
                .clone()
                .map_err(anyhow::Error::msg)?
                .context("Core did not publish a memory map")?;
            let region = snapshot.exact_writable_region(expected)?;
            Ok(unsafe { std::slice::from_raw_parts(region.pointer(), region.bytes()) })
        }
    }
}

fn gameboy_core_contract(core_name: &str) -> Result<GameboyCoreContract> {
    match core_name {
        "Gambatte" => Ok(GameboyCoreContract {
            device_modes: &[1],
            supports_bitmask: true,
            system_ram_offset: 0,
            source_revision: "d9d6cd06382d1ced30de34d56d3609452323dab1",
            source_url: "https://github.com/libretro/gambatte-libretro/tree/d9d6cd06382d1ced30de34d56d3609452323dab1",
        }),
        "mGBA" => Ok(GameboyCoreContract {
            device_modes: &[1],
            supports_bitmask: true,
            system_ram_offset: 0,
            source_revision: "e31759b24e7a4e3899285ff720d7b573ac328ae7",
            source_url: "https://github.com/libretro/mgba/blob/e31759b24e7a4e3899285ff720d7b573ac328ae7/src/platform/libretro/libretro.c",
        }),
        "SameBoy" => Ok(GameboyCoreContract {
            // SameBoy advertises subclass 257 but deliberately accepts and
            // polls the base joypad device as well. Exercise both contracts.
            device_modes: &[1, 257],
            supports_bitmask: true,
            system_ram_offset: 0,
            source_revision: "8230189896a8bb6598574d302ba0ad3658f98ab4",
            source_url: "https://github.com/LIJI32/SameBoy/blob/8230189896a8bb6598574d302ba0ad3658f98ab4/libretro/libretro.c",
        }),
        "SkyEmu" => Ok(GameboyCoreContract {
            device_modes: &[1],
            supports_bitmask: false,
            // SkyEmu exposes its complete 64 KiB address-space image before
            // banked WRAM through RETRO_MEMORY_SYSTEM_RAM. The diagnostic's
            // C000 bytes therefore begin at this documented offset.
            system_ram_offset: 0xc000,
            source_revision: "adacd0788964ed89f5c43dcbc1f3cc26deec996c",
            source_url: "https://github.com/skylersaleh/SkyEmu/blob/adacd0788964ed89f5c43dcbc1f3cc26deec996c/src/libretro.c",
        }),
        "VBA-M" => Ok(GameboyCoreContract {
            device_modes: &[1],
            supports_bitmask: true,
            system_ram_offset: 0,
            source_revision: "115defb3a318258ab84746d45258a1aec19d0b4b",
            source_url: "https://github.com/libretro/vbam-libretro/blob/115defb3a318258ab84746d45258a1aec19d0b4b/src/libretro/libretro.cpp",
        }),
        _ => anyhow::bail!(
            "Game Boy diagnostic supports only exact Gambatte, mGBA, SameBoy, SkyEmu, or VBA-M identities"
        ),
    }
}

fn nes_core_contract(core_name: &str) -> Result<NesCoreContract> {
    match core_name {
        "FCEUmm" => Ok(NesCoreContract {
            controller_device: 513,
            controller_label: "Gamepad",
            source_revision: "5cd4a43e16a7f3cd35628d481c347a0a98cfdfa2",
            source_url: "https://github.com/libretro/libretro-fceumm/tree/5cd4a43e16a7f3cd35628d481c347a0a98cfdfa2",
        }),
        "Mesen" => Ok(NesCoreContract {
            controller_device: 257,
            controller_label: "Standard Controller",
            source_revision: "0102910c39ad1a62bc3f784466f3f67ca9eae335",
            source_url: "https://github.com/libretro/Mesen/blob/0102910c39ad1a62bc3f784466f3f67ca9eae335/Libretro/libretro.cpp",
        }),
        _ => anyhow::bail!("NES diagnostic supports only exact FCEUmm or Mesen identities"),
    }
}

#[derive(Clone, Copy)]
enum SnesReadback {
    SystemRam,
    SerializedState,
}

impl SnesReadback {
    fn name(self) -> &'static str {
        match self {
            Self::SystemRam => "retro_get_memory_data(RETRO_MEMORY_SYSTEM_RAM)",
            Self::SerializedState => "retro_serialize WRAM marker",
        }
    }
}

#[derive(Clone, Copy)]
struct SnesCoreContract {
    supports_bitmask: bool,
    readback: SnesReadback,
    joypad_device: u32,
    multitap_device: u32,
    selects_virtual_multitap_ports: bool,
    controller_label: &'static str,
    describes_start_select: bool,
    source_revision: &'static str,
    source_url: &'static str,
}

fn snes_core_contract(core_name: &str) -> Result<SnesCoreContract> {
    match core_name {
        "bsnes" => Ok(SnesCoreContract {
            supports_bitmask: false,
            readback: SnesReadback::SerializedState,
            joypad_device: 1,
            multitap_device: 257,
            selects_virtual_multitap_ports: false,
            controller_label: "SNES Joypad",
            describes_start_select: true,
            source_revision: "8e80d2f8a43e34a82931e25143b279e5fbcfaedc",
            source_url: "https://github.com/libretro/bsnes-libretro/tree/8e80d2f8a43e34a82931e25143b279e5fbcfaedc",
        }),
        "Snes9x" => Ok(SnesCoreContract {
            supports_bitmask: true,
            readback: SnesReadback::SystemRam,
            joypad_device: 1,
            multitap_device: 257,
            selects_virtual_multitap_ports: false,
            controller_label: "SNES Joypad",
            describes_start_select: true,
            source_revision: "185488cd83aaf274752a742c94d45561cbecb7af",
            source_url: "https://github.com/snes9xgit/snes9x/tree/185488cd83aaf274752a742c94d45561cbecb7af",
        }),
        "Mesen-S" => Ok(SnesCoreContract {
            supports_bitmask: false,
            readback: SnesReadback::SystemRam,
            joypad_device: 257,
            multitap_device: 513,
            selects_virtual_multitap_ports: true,
            controller_label: "SNES Controller",
            // The 0.4.0 libretro adapter maps both buttons but omits them from
            // its published descriptor table.
            describes_start_select: false,
            source_revision: "dd0287088c53e1e96e5818ed81160f6a958646a8",
            source_url: "https://github.com/libretro/Mesen-S/tree/dd0287088c53e1e96e5818ed81160f6a958646a8",
        }),
        _ => anyhow::bail!(
            "SNES diagnostic supports only exact bsnes, Snes9x, or Mesen-S identities"
        ),
    }
}

fn joypad_requests_by_port() -> Vec<u64> {
    JOYPAD_REQUESTS_BY_PORT
        .iter()
        .map(|requests| requests.load(Ordering::Relaxed))
        .collect()
}

fn validate_nes_metadata(
    core_name: &str,
    topology: NesTopology,
    controller_device: u32,
    controller_choices: Option<&[Vec<ControllerChoice>]>,
    descriptors: &[InputDescriptor],
) -> Result<()> {
    let contract = nes_core_contract(core_name)?;
    ensure!(
        contract.controller_device == controller_device,
        "NES controller contract changed during the run"
    );
    let choices = controller_choices.context("NES core did not advertise controller choices")?;
    let required_descriptors = [
        (0, "B"),
        (8, "A"),
        (2, "Select"),
        (3, "Start"),
        (4, "D-Pad Up"),
        (5, "D-Pad Down"),
        (6, "D-Pad Left"),
        (7, "D-Pad Right"),
    ];
    for port in 0..topology.connected_ports() {
        ensure!(
            choices.get(port).is_some_and(|port_choices| {
                port_choices.iter().any(|choice| {
                    choice.id == controller_device
                        && choice.description == contract.controller_label
                })
            }),
            "NES port {} did not advertise {} as device {controller_device}",
            port + 1,
            contract.controller_label
        );
        for &(id, label) in &required_descriptors {
            ensure!(
                descriptors.iter().any(|descriptor| {
                    descriptor.port == port as u32
                        && descriptor.device == 1
                        && descriptor.index == 0
                        && descriptor.id == id
                        && descriptor.description == label
                }),
                "NES port {} is missing descriptor {label} (joypad id {id})",
                port + 1
            );
        }
    }
    Ok(())
}

fn validate_snes_metadata(
    topology: SnesTopology,
    contract: SnesCoreContract,
    controller_choices: Option<&[Vec<ControllerChoice>]>,
    descriptors: &[InputDescriptor],
) -> Result<()> {
    let choices = controller_choices.context("SNES core did not advertise controller choices")?;
    for (port, device, label) in [
        (0usize, contract.joypad_device, contract.controller_label),
        (
            1,
            if topology == SnesTopology::Multitap {
                contract.multitap_device
            } else {
                contract.joypad_device
            },
            if topology == SnesTopology::Multitap {
                "Multitap"
            } else {
                contract.controller_label
            },
        ),
    ] {
        ensure!(
            choices.get(port).is_some_and(|port_choices| {
                port_choices
                    .iter()
                    .any(|choice| choice.id == device && choice.description == label)
            }),
            "SNES physical port {} did not advertise {label} as device {device}",
            port + 1
        );
    }
    let required_descriptors = [
        (0, "B"),
        (1, "Y"),
        (4, "D-Pad Up"),
        (5, "D-Pad Down"),
        (6, "D-Pad Left"),
        (7, "D-Pad Right"),
        (8, "A"),
        (9, "X"),
        (10, "L"),
        (11, "R"),
    ];
    for port in 0..topology.connected_ports() {
        for &(id, label) in &required_descriptors {
            ensure!(
                descriptors.iter().any(|descriptor| {
                    descriptor.port == port as u32
                        && descriptor.device == 1
                        && descriptor.index == 0
                        && descriptor.id == id
                        && descriptor.description == label
                }),
                "SNES frontend port {} is missing descriptor {label} (joypad id {id})",
                port + 1
            );
        }
        if contract.describes_start_select {
            for (id, label) in [(2, "Select"), (3, "Start")] {
                ensure!(
                    descriptors.iter().any(|descriptor| {
                        descriptor.port == port as u32
                            && descriptor.device == 1
                            && descriptor.index == 0
                            && descriptor.id == id
                            && descriptor.description == label
                    }),
                    "SNES frontend port {} is missing descriptor {label} (joypad id {id})",
                    port + 1
                );
            }
        }
    }
    Ok(())
}

/// Query a trusted native core's libretro identity without initializing it.
pub fn core_identity(path: &Path, expected_sha256: &str) -> Result<CoreIdentity> {
    let path = path.canonicalize()?;
    let hash = crate::file_hash(&path)?;
    ensure!(
        hash.eq_ignore_ascii_case(expected_sha256),
        "Core SHA256 mismatch"
    );
    unsafe {
        let library = Library::new(&path).context("Loading trusted libretro core")?;
        let version = *library.get::<unsafe extern "C" fn() -> u32>(b"retro_api_version\0")?;
        ensure!(version() == 1, "Unsupported libretro API version");
        let info_fn =
            *library.get::<unsafe extern "C" fn(*mut SystemInfo)>(b"retro_get_system_info\0")?;
        let mut info = SystemInfo::default();
        info_fn(&mut info);
        ensure!(
            !info.name.is_null() && !info.version.is_null() && !info.extensions.is_null(),
            "Missing core identity"
        );
        Ok(CoreIdentity {
            core_sha256: hash,
            core_name: CStr::from_ptr(info.name).to_str()?.to_owned(),
            core_version: CStr::from_ptr(info.version).to_str()?.to_owned(),
            valid_extensions: CStr::from_ptr(info.extensions).to_str()?.to_owned(),
            need_fullpath: info.need_fullpath,
            block_extract: info.block_extract,
        })
    }
}

fn runtime_library_inventory(paths: &[PathBuf]) -> Result<Vec<RuntimeLibraryIdentity>> {
    let mut inventory = Vec::new();
    for path in paths {
        ensure!(path.is_absolute(), "Runtime library path must be absolute");
        let path = path
            .canonicalize()
            .with_context(|| format!("Canonicalizing runtime library {}", path.display()))?;
        ensure!(
            !inventory
                .iter()
                .any(|library: &RuntimeLibraryIdentity| library.path == path),
            "Duplicate runtime library {}",
            path.display()
        );
        inventory.push(RuntimeLibraryIdentity {
            sha256: crate::file_hash(&path)?,
            path,
        });
    }
    Ok(inventory)
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

#[derive(Debug, Serialize, serde::Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ContentControllerReport {
    pub schema_version: u32,
    pub core: CoreIdentity,
    pub content_filename: String,
    pub staged_files: Vec<FirmwareIdentity>,
    pub requested_devices: BTreeMap<u32, u32>,
    pub effective_options: BTreeMap<String, String>,
    pub controller_choices: ControllerChoices,
    pub input_descriptor_updates_before_refresh: u64,
    pub input_descriptor_updates: u64,
    pub input_descriptors: Vec<InputDescriptor>,
    /// Unique input callback addresses requested during the idle refresh frame.
    /// Conditional paths that require held input are not exhaustively observed.
    pub input_queries: Vec<InputQuery>,
    pub input_query_calls: u64,
    pub refresh_frames: u32,
}

/// Load real content in a fresh helper process, with private content, system
/// and save directories. Explicit dependencies preserve their original basenames.
/// This is descriptor inspection, not a controller-behavior or gameplay test.
/// Only full-path cores are accepted. The caller must trust the pinned core and
/// enforce an external process timeout; native loading/running cannot be safely
/// interrupted inside this function.
#[allow(clippy::too_many_arguments)]
pub fn inspect_content_controllers(
    path: &Path,
    expected_sha256: &str,
    expected_core_name: &str,
    content: &Path,
    content_dependencies: &[std::path::PathBuf],
    system_files: &[std::path::PathBuf],
    overrides: BTreeMap<String, String>,
    requested_devices: BTreeMap<u32, u32>,
) -> Result<ContentControllerReport> {
    ensure!(
        !requested_devices.is_empty() && requested_devices.keys().all(|port| *port < 16),
        "Inspection requires explicit device selections on bounded ports"
    );
    ensure!(
        content_dependencies.len() <= 256 && system_files.len() <= 256,
        "Too many inspection dependency files"
    );
    let options = crate::libretro_options::OptionEnvironment::new(overrides)?;
    let path = path.canonicalize()?;
    ensure!(
        ACTIVE
            .compare_exchange(false, true, Ordering::Acquire, Ordering::Relaxed)
            .is_ok(),
        "Only one core inspection may run in this process"
    );
    let _lease = Lease;
    #[cfg(unix)]
    let _native_stdout = NativeStdoutSilencer::new()?;
    let directory = tempfile::tempdir()?;
    let content_directory = directory.path().join("content");
    let system_directory = directory.path().join("system");
    let save_directory = directory.path().join("save");
    for target in [&content_directory, &system_directory, &save_directory] {
        std::fs::create_dir(target)?;
    }
    let mut staged_files = Vec::new();
    let mut stage =
        |source: &Path, target_directory: &Path, group: &str| -> Result<std::path::PathBuf> {
            let source = source.canonicalize()?;
            ensure!(
                source.is_file(),
                "Inspection dependency is not a regular file"
            );
            let filename = source.file_name().context("Dependency has no filename")?;
            let target = target_directory.join(filename);
            let mut input = std::fs::File::open(&source)?;
            let mut output = std::fs::OpenOptions::new()
                .write(true)
                .create_new(true)
                .open(&target)
                .with_context(|| {
                    format!("Staging unique inspection dependency {}", target.display())
                })?;
            let bytes = std::io::copy(&mut input, &mut output)?;
            drop(output);
            staged_files.push(FirmwareIdentity {
                filename: format!(
                    "{group}/{}",
                    filename.to_str().context("UTF-8 filename required")?
                ),
                sha256: crate::file_hash(&target)?,
                bytes,
            });
            Ok(target)
        };
    let staged_content = stage(content, &content_directory, "content")?;
    for source in content_dependencies {
        stage(source, &content_directory, "content")?;
    }
    for source in system_files {
        stage(source, &system_directory, "system")?;
    }
    let core_identity = core_identity(&path, expected_sha256)?;
    ensure!(
        core_identity.core_name == expected_core_name,
        "Unexpected core identity"
    );
    ensure!(
        core_identity.need_fullpath,
        "Content inspection requires a full-path core"
    );
    *SYSTEM_DIRECTORY
        .lock()
        .map_err(|_| anyhow::anyhow!("System-directory lock poisoned"))? = Some(CString::new(
        system_directory
            .to_str()
            .context("UTF-8 system path required")?,
    )?);
    *SAVE_DIRECTORY
        .lock()
        .map_err(|_| anyhow::anyhow!("Save-directory lock poisoned"))? = Some(CString::new(
        save_directory
            .to_str()
            .context("UTF-8 save path required")?,
    )?);
    *INSPECTION_OPTIONS
        .lock()
        .map_err(|_| anyhow::anyhow!("Core-option lock poisoned"))? = Some(options);
    *CONTROLLER_CHOICES
        .lock()
        .map_err(|_| anyhow::anyhow!("Controller-choice lock poisoned"))? = Ok(None);
    *INPUT_DESCRIPTORS
        .lock()
        .map_err(|_| anyhow::anyhow!("Descriptor lock poisoned"))? = DescriptorCapture {
        updates: 0,
        descriptors: Ok(Vec::new()),
    };
    BITMASK.store(false, Ordering::Relaxed);
    let content_path = CString::new(
        staged_content
            .to_str()
            .context("UTF-8 content path required")?,
    )?;
    unsafe {
        let library = Library::new(&path).context("Loading trusted inspection core")?;
        let init = *library.get::<unsafe extern "C" fn()>(b"retro_init\0")?;
        let load =
            *library.get::<unsafe extern "C" fn(*const GameInfo) -> bool>(b"retro_load_game\0")?;
        let mut core = Core {
            deinit: *library.get(b"retro_deinit\0")?,
            unload: *library.get(b"retro_unload_game\0")?,
            run: *library.get(b"retro_run\0")?,
            memory: *library.get(b"retro_get_memory_data\0")?,
            memory_size: *library.get(b"retro_get_memory_size\0")?,
            serialize: *library.get(b"retro_serialize\0")?,
            serialize_size: *library.get(b"retro_serialize_size\0")?,
            set_device: *library.get(b"retro_set_controller_port_device\0")?,
            initialized: false,
            loaded: false,
            _library: library,
        };
        macro_rules! callback {
            ($symbol:literal, $ty:ty, $callback:ident) => {
                core._library.get::<unsafe extern "C" fn($ty)>($symbol)?($callback);
            };
        }
        callback!(b"retro_set_environment\0", Environment, environment);
        // The environment callback must be available during initialization.
        // Mesen creates the objects that own its other callback slots in
        // retro_init, so installing those callbacks first dereferences an
        // uninitialized core-global pointer.
        init();
        core.initialized = true;
        callback!(b"retro_set_video_refresh\0", Video, video);
        callback!(b"retro_set_audio_sample\0", Audio, audio);
        callback!(b"retro_set_audio_sample_batch\0", AudioBatch, audio_batch);
        callback!(b"retro_set_input_poll\0", Poll, poll);
        callback!(b"retro_set_input_state\0", Input, neutral_input);
        ensure!(
            load(&GameInfo {
                path: content_path.as_ptr(),
                data: std::ptr::null(),
                size: 0,
                meta: std::ptr::null()
            }),
            "Core rejected staged inspection content"
        );
        core.loaded = true;
        let choices =
            controller_choice_snapshot()?.context("Core did not advertise controller choices")?;
        for (&port, &device) in &requested_devices {
            ensure!(
                choices
                    .get(port as usize)
                    .is_some_and(|choices| choices.iter().any(|choice| choice.id == device)),
                "Requested device {device} is not advertised on port {port}"
            );
            (core.set_device)(port, device);
        }
        let (before, _) = input_descriptor_snapshot()?;
        *INSPECTION_QUERIES
            .lock()
            .map_err(|_| anyhow::anyhow!("Input-query lock poisoned"))? = QueryCapture {
            calls: 0,
            addresses: std::collections::BTreeSet::new(),
            failure: None,
        };
        (core.run)();
        let (input_descriptor_updates, input_descriptors) = input_descriptor_snapshot()?;
        ensure!(
            input_descriptor_updates > before,
            "Core did not refresh input descriptors during the inspection frame"
        );
        let controller_choices =
            controller_choice_snapshot()?.context("Core withdrew controller choices")?;
        for (&port, &device) in &requested_devices {
            ensure!(
                controller_choices
                    .get(port as usize)
                    .is_some_and(|choices| choices.iter().any(|choice| choice.id == device)),
                "Requested device {device} is no longer advertised on port {port}"
            );
        }
        let effective_options = INSPECTION_OPTIONS
            .lock()
            .map_err(|_| anyhow::anyhow!("Core-option lock poisoned"))?
            .as_ref()
            .context("Inspection options disappeared")?
            .effective_values()?;
        let (input_queries, input_query_calls) = {
            let queries = INSPECTION_QUERIES
                .lock()
                .map_err(|_| anyhow::anyhow!("Input-query lock poisoned"))?;
            ensure!(
                queries.failure.is_none(),
                "Input-query capture failed: {:?}",
                queries.failure
            );
            ensure!(
                queries.calls > 0,
                "Core did not query input during the inspection frame"
            );
            (queries.addresses.iter().cloned().collect(), queries.calls)
        };
        Ok(ContentControllerReport {
            schema_version: 1,
            core: core_identity,
            content_filename: staged_content
                .file_name()
                .and_then(|name| name.to_str())
                .context("UTF-8 content filename required")?
                .to_owned(),
            staged_files,
            requested_devices,
            effective_options,
            controller_choices,
            input_descriptor_updates_before_refresh: before,
            input_descriptor_updates,
            input_descriptors,
            input_queries,
            input_query_calls,
            refresh_frames: 1,
        })
    }
}

// Inspection deliberately presents an idle frontend, with no host devices.
unsafe extern "C" fn neutral_input(port: u32, device: u32, index: u32, id: u32) -> i16 {
    if let Ok(mut queries) = INSPECTION_QUERIES.lock()
        && queries.failure.is_none()
    {
        if port >= 16 || queries.calls >= 65536 {
            queries.failure = Some("Input-query address/call limit exceeded".into());
        } else {
            queries.calls += 1;
            queries.addresses.insert(InputQuery {
                port,
                device,
                index,
                id,
            });
            if queries.addresses.len() > 4096 {
                queries.failure = Some("Too many distinct input-query addresses".into());
            }
        }
    }
    0
}

/// Loads native executable code. Caller must trust the supplied core. This
/// diagnostic uses synthetic frontend input, not OS controller enumeration or
/// the Lunchbox-to-RetroArch configuration layer. The CLI bounds wall time.
pub fn inspect_mgba(path: &Path, expected_sha256: &str, bitmask: bool) -> Result<Report> {
    inspect(path, expected_sha256, bitmask, Diagnostic::Gba)
}

pub fn inspect(
    path: &Path,
    expected_sha256: &str,
    bitmask: bool,
    diagnostic: Diagnostic,
) -> Result<Report> {
    ensure!(
        !matches!(diagnostic, Diagnostic::Psx),
        "PSX diagnostic requires an explicit BIOS directory"
    );
    inspect_with_system_directory(path, expected_sha256, bitmask, diagnostic, None)
}

pub fn inspect_with_system_directory(
    path: &Path,
    expected_sha256: &str,
    bitmask: bool,
    diagnostic: Diagnostic,
    system_directory: Option<&Path>,
) -> Result<Report> {
    inspect_with_options(
        path,
        expected_sha256,
        bitmask,
        diagnostic,
        system_directory,
        NesTopology::default(),
        SnesTopology::default(),
    )
}

pub fn inspect_with_options(
    path: &Path,
    expected_sha256: &str,
    bitmask: bool,
    diagnostic: Diagnostic,
    system_directory: Option<&Path>,
    nes_topology: NesTopology,
    snes_topology: SnesTopology,
) -> Result<Report> {
    inspect_with_runtime_options(
        path,
        expected_sha256,
        bitmask,
        diagnostic,
        system_directory,
        nes_topology,
        snes_topology,
        &[],
    )
}

#[allow(clippy::too_many_arguments)]
pub fn inspect_with_runtime_options(
    path: &Path,
    expected_sha256: &str,
    bitmask: bool,
    diagnostic: Diagnostic,
    system_directory: Option<&Path>,
    nes_topology: NesTopology,
    snes_topology: SnesTopology,
    runtime_library_paths: &[PathBuf],
) -> Result<Report> {
    ensure!(
        matches!(diagnostic, Diagnostic::Nes) || nes_topology == NesTopology::TwoPlayer,
        "NES topology is only applicable to the NES diagnostic"
    );
    ensure!(
        matches!(diagnostic, Diagnostic::Snes) || snes_topology == SnesTopology::TwoPlayer,
        "SNES topology is only applicable to the SNES diagnostic"
    );
    let (expected_core, filename, device, register_mask) = diagnostic.identity();
    let path = path.canonicalize()?;
    let hash = crate::file_hash(&path)?;
    ensure!(
        hash.eq_ignore_ascii_case(expected_sha256),
        "Core SHA256 mismatch"
    );
    let runtime_libraries = runtime_library_inventory(runtime_library_paths)?;
    let mut _runtime_dependencies = Vec::new();
    for library in &runtime_libraries {
        _runtime_dependencies.push(load_runtime_dependency(&library.path)?);
    }
    ensure!(
        ACTIVE
            .compare_exchange(false, true, Ordering::Acquire, Ordering::Relaxed)
            .is_ok(),
        "Only one core diagnostic may run in this process"
    );
    let _lease = Lease;
    #[cfg(unix)]
    let _native_stdout = NativeStdoutSilencer::new()?;
    let directory = tempfile::tempdir()?;
    let (system_path, firmware) = if matches!(diagnostic, Diagnostic::Psx) {
        let system_path = system_directory
            .context("PSX diagnostic requires an explicit BIOS directory")?
            .canonicalize()
            .context("Resolving PSX BIOS directory")?;
        ensure!(system_path.is_dir(), "PSX BIOS path is not a directory");
        let mut firmware = Vec::new();
        for filename in ["scph5500.bin", "scph5501.bin", "scph5502.bin"] {
            let bios = system_path.join(filename);
            let metadata = std::fs::metadata(&bios)
                .with_context(|| format!("Reading required BIOS {}", bios.display()))?;
            ensure!(
                metadata.is_file() && metadata.len() == 512 * 1024,
                "Expected a 512 KiB BIOS at {}",
                bios.display()
            );
            firmware.push(FirmwareIdentity {
                filename: filename.into(),
                sha256: crate::file_hash(&bios)?,
                bytes: metadata.len(),
            });
        }
        (system_path, firmware)
    } else {
        (directory.path().to_owned(), Vec::new())
    };
    *SYSTEM_DIRECTORY
        .lock()
        .map_err(|_| anyhow::anyhow!("Poisoned diagnostic context"))? = Some(CString::new(
        system_path
            .to_str()
            .context("UTF-8 diagnostic directory required")?,
    )?);
    *SAVE_DIRECTORY
        .lock()
        .map_err(|_| anyhow::anyhow!("Poisoned diagnostic context"))? = Some(CString::new(
        directory
            .path()
            .to_str()
            .context("UTF-8 diagnostic save directory required")?,
    )?);
    for pressed in &PRESSED {
        pressed.store(0, Ordering::Relaxed);
    }
    for axis in &ANALOG {
        axis.store(0, Ordering::Relaxed);
    }
    POLLS.store(0, Ordering::Relaxed);
    *MEMORY_MAP
        .lock()
        .map_err(|_| anyhow::anyhow!("Memory-map capture lock poisoned"))? = Ok(None);
    *CONTROLLER_CHOICES
        .lock()
        .map_err(|_| anyhow::anyhow!("Controller choice capture lock poisoned"))? = Ok(None);
    *INPUT_DESCRIPTORS
        .lock()
        .map_err(|_| anyhow::anyhow!("Input descriptor capture lock poisoned"))? =
        DescriptorCapture {
            updates: 0,
            descriptors: Ok(Vec::new()),
        };
    MASK_REQUESTS.store(0, Ordering::Relaxed);
    SINGLE_REQUESTS.store(0, Ordering::Relaxed);
    ANALOG_REQUESTS.store(0, Ordering::Relaxed);
    for requests in &ANALOG_REQUESTS_BY_PORT {
        requests.store(0, Ordering::Relaxed);
    }
    for requests in &JOYPAD_REQUESTS_BY_PORT {
        requests.store(0, Ordering::Relaxed);
    }
    BITMASK.store(bitmask, Ordering::Relaxed);
    // These buffers must outlive unload_game, including every error path.
    let rom = match diagnostic {
        Diagnostic::Gba => gba_diagnostic_rom(),
        Diagnostic::Gamegear => gamegear_diagnostic_rom(),
        Diagnostic::Gameboy => gameboy_diagnostic_rom(),
        Diagnostic::Atari2600 => atari2600_diagnostic_rom(),
        Diagnostic::Nes => nes_diagnostic_rom(),
        Diagnostic::Snes => snes_diagnostic_rom(),
        Diagnostic::Psx => psx_diagnostic_exe(),
    };
    let core_rom_path = if matches!(diagnostic, Diagnostic::Atari2600) {
        // Stella's libretro filesystem backend consumes the in-memory image,
        // but its ROM-type detector expects a simple content filename.
        PathBuf::from(filename)
    } else {
        directory.path().join(filename)
    };
    let rom_path = CString::new(core_rom_path.to_str().context("UTF-8 ROM path required")?)?;
    unsafe {
        let library = Library::new(&path).context("Loading trusted libretro core")?;
        let version = *library.get::<unsafe extern "C" fn() -> u32>(b"retro_api_version\0")?;
        ensure!(version() == 1, "Unsupported libretro API version");
        let info_fn =
            *library.get::<unsafe extern "C" fn(*mut SystemInfo)>(b"retro_get_system_info\0")?;
        let mut info = SystemInfo::default();
        info_fn(&mut info);
        ensure!(
            !info.name.is_null() && !info.version.is_null(),
            "Missing core identity"
        );
        let name = CStr::from_ptr(info.name).to_str()?.to_owned();
        let revision = CStr::from_ptr(info.version).to_str()?.to_owned();
        let core_name_matches = match diagnostic {
            Diagnostic::Gba => matches!(name.as_str(), "mGBA" | "VBA-M" | "SkyEmu"),
            Diagnostic::Gameboy => {
                matches!(
                    name.as_str(),
                    "Gambatte" | "mGBA" | "SameBoy" | "SkyEmu" | "VBA-M"
                )
            }
            Diagnostic::Atari2600 => name == "Stella",
            Diagnostic::Nes => matches!(name.as_str(), "FCEUmm" | "Mesen"),
            Diagnostic::Snes => matches!(name.as_str(), "bsnes" | "Snes9x" | "Mesen-S"),
            Diagnostic::Psx => name == expected_core || name == "Beetle PSX HW",
            _ => name == expected_core,
        };
        ensure!(core_name_matches, "Expected {expected_core}, got {name}");
        let gba_contract = matches!(diagnostic, Diagnostic::Gba)
            .then(|| gba_core_contract(&name))
            .transpose()?;
        if let Some(contract) = gba_contract {
            ensure!(
                !bitmask || contract.supports_bitmask,
                "The {name} core does not negotiate libretro joypad bitmask input; rerun without --bitmask"
            );
        }
        let gameboy_contract = matches!(diagnostic, Diagnostic::Gameboy)
            .then(|| gameboy_core_contract(&name))
            .transpose()?;
        if let Some(contract) = gameboy_contract {
            ensure!(
                !bitmask || contract.supports_bitmask,
                "The {name} core does not negotiate libretro joypad bitmask input; rerun without --bitmask"
            );
        }
        let nes_contract = matches!(diagnostic, Diagnostic::Nes)
            .then(|| nes_core_contract(&name))
            .transpose()?;
        let snes_contract = matches!(diagnostic, Diagnostic::Snes)
            .then(|| snes_core_contract(&name))
            .transpose()?;
        if let Some(contract) = snes_contract {
            ensure!(
                !bitmask || contract.supports_bitmask,
                "The {name} core does not negotiate libretro joypad bitmask input; rerun without --bitmask"
            );
        }
        // Stella advertises `need_fullpath=false` and consumes the supplied
        // buffer, but its current ROM-name validator still asks the host file
        // backend whether the path exists before reading that buffer.
        if info.need_fullpath || matches!(diagnostic, Diagnostic::Atari2600) {
            std::fs::write(directory.path().join(filename), &rom)?;
        }
        let init = *library.get::<unsafe extern "C" fn()>(b"retro_init\0")?;
        let load =
            *library.get::<unsafe extern "C" fn(*const GameInfo) -> bool>(b"retro_load_game\0")?;
        let set_device = *library
            .get::<unsafe extern "C" fn(u32, u32)>(b"retro_set_controller_port_device\0")?;
        let mut core = Core {
            deinit: *library.get(b"retro_deinit\0")?,
            unload: *library.get(b"retro_unload_game\0")?,
            run: *library.get(b"retro_run\0")?,
            memory: *library.get(b"retro_get_memory_data\0")?,
            memory_size: *library.get(b"retro_get_memory_size\0")?,
            serialize: *library.get(b"retro_serialize\0")?,
            serialize_size: *library.get(b"retro_serialize_size\0")?,
            set_device,
            initialized: false,
            loaded: false,
            _library: library,
        };
        macro_rules! callback {
            ($symbol:literal, $ty:ty, $callback:ident) => {
                core._library.get::<unsafe extern "C" fn($ty)>($symbol)?($callback);
            };
        }
        callback!(b"retro_set_environment\0", Environment, environment);
        // The environment callback must be available during initialization.
        // Mesen creates the objects that own its other callback slots in
        // retro_init, so installing those callbacks first dereferences an
        // uninitialized core-global pointer.
        init();
        core.initialized = true;
        callback!(b"retro_set_video_refresh\0", Video, video);
        callback!(b"retro_set_audio_sample\0", Audio, audio);
        callback!(b"retro_set_audio_sample_batch\0", AudioBatch, audio_batch);
        callback!(b"retro_set_input_poll\0", Poll, poll);
        callback!(b"retro_set_input_state\0", Input, input);
        if !matches!(
            diagnostic,
            Diagnostic::Gameboy | Diagnostic::Nes | Diagnostic::Snes
        ) {
            set_device(0, device);
        }
        ensure!(
            load(&GameInfo {
                path: rom_path.as_ptr(),
                data: if info.need_fullpath {
                    std::ptr::null()
                } else {
                    rom.as_ptr().cast()
                },
                size: if info.need_fullpath { 0 } else { rom.len() },
                meta: std::ptr::null()
            }),
            "Core rejected original diagnostic ROM"
        );
        core.loaded = true;
        if let Some(contract) = gameboy_contract {
            set_device(0, contract.device_modes[0]);
        } else if let Some(contract) = nes_contract {
            for port in 0..5 {
                set_device(
                    port,
                    if port < nes_topology.connected_ports() as u32 {
                        contract.controller_device
                    } else {
                        0
                    },
                );
            }
        } else if let Some(contract) = snes_contract {
            set_device(0, contract.joypad_device);
            set_device(
                1,
                if snes_topology == SnesTopology::Multitap {
                    contract.multitap_device
                } else {
                    contract.joypad_device
                },
            );
            if snes_topology == SnesTopology::Multitap && contract.selects_virtual_multitap_ports {
                for port in 2..5 {
                    set_device(port, contract.joypad_device);
                }
            }
        } else {
            set_device(0, device);
            set_device(
                1,
                if matches!(diagnostic, Diagnostic::Psx) {
                    device
                } else {
                    0
                },
            );
        }
        let reported_system_ram_bytes = (core.memory_size)(2);
        // This pinned mGBA frontend reports the GB RAM size even for GBA.
        // Read only our eight diagnostic bytes, within its reported bounds;
        // never infer that the full 256 KiB hardware RAM is exposed by this API.
        if let Some(contract) = gba_contract {
            let memory = gba_observation_memory(&core, contract)?;
            ensure!(memory.len() >= 8, "GBA diagnostic memory is too small");
        } else if matches!(diagnostic, Diagnostic::Psx) {
            ensure!(
                reported_system_ram_bytes == 2 * 1024 * 1024,
                "Unexpected exposed PSX RAM size: {reported_system_ram_bytes}"
            );
        } else if !matches!(diagnostic, Diagnostic::Snes) {
            ensure!(
                (8..=256 * 1024).contains(&reported_system_ram_bytes),
                "Unexpected exposed system RAM size: {reported_system_ram_bytes}"
            );
        }
        let (observation_memory_source, observation_memory_bytes, observation_memory_address) =
            match gba_contract.map(|contract| contract.observation_memory) {
                Some(GbaObservationMemory::SystemRam { bytes, .. }) => {
                    ("retro-memory-system-ram", bytes, None)
                }
                Some(GbaObservationMemory::MemoryMap(mapping)) => (
                    "retro-environment-memory-map",
                    mapping.bytes,
                    Some(mapping.address),
                ),
                None => ("retro-memory-system-ram", reported_system_ram_bytes, None),
            };
        if matches!(diagnostic, Diagnostic::Atari2600) {
            ensure!(
                reported_system_ram_bytes == 128,
                "Unexpected exposed Atari 2600 RIOT RAM size: {reported_system_ram_bytes}"
            );
            let atari2600_observations = run_atari2600_observations(&core)?;
            let requests = joypad_requests_by_port();
            ensure!(
                requests[0] > 0 && requests[1] > 0,
                "Stella did not query both joystick frontend ports"
            );
            let bitmask_requests = MASK_REQUESTS.load(Ordering::Relaxed);
            let individual_requests = SINGLE_REQUESTS.load(Ordering::Relaxed);
            ensure!(
                if bitmask {
                    bitmask_requests > 0 && individual_requests == 0
                } else {
                    bitmask_requests == 0 && individual_requests > 0
                },
                "Stella did not exercise the requested input callback mode"
            );
            let (input_descriptor_updates, input_descriptors) = input_descriptor_snapshot()?;
            let controller_choices = controller_choice_snapshot()?;
            let (contract_source_revision, contract_source_url) =
                validate_atari2600_metadata(controller_choices.as_deref(), &input_descriptors)?;
            return Ok(Report {
                schema_version: 9,
                diagnostic: "atari2600-joysticks-console-switches",
                core_sha256: hash,
                core_name: name,
                core_version: revision,
                input_descriptors,
                input_descriptor_updates,
                controller_choices,
                input_mode: if bitmask { "bitmask" } else { "individual" },
                input_polls: POLLS.load(Ordering::Relaxed),
                bitmask_requests,
                individual_requests,
                analog_requests: ANALOG_REQUESTS.load(Ordering::Relaxed),
                joypad_requests_by_port: requests,
                reported_system_ram_bytes,
                observation_memory_source,
                observation_memory_bytes,
                observation_memory_address,
                observations: Vec::new(),
                nes_observations: Vec::new(),
                snes_observations: Vec::new(),
                atari2600_observations,
                psx_observations: Vec::new(),
                firmware,
                runtime_libraries: runtime_libraries.clone(),
                contract_source_revision: Some(contract_source_revision),
                contract_source_url: Some(contract_source_url),
            });
        }
        if matches!(diagnostic, Diagnostic::Psx) {
            let psx_observations = run_psx_observations(&core)?;
            let bitmask_requests = MASK_REQUESTS.load(Ordering::Relaxed);
            let individual_requests = SINGLE_REQUESTS.load(Ordering::Relaxed);
            ensure!(
                if bitmask {
                    bitmask_requests > 0
                } else {
                    bitmask_requests == 0 && individual_requests > 0
                },
                "Core did not exercise the requested input callback mode"
            );
            let (input_descriptor_updates, input_descriptors) = input_descriptor_snapshot()?;
            return Ok(Report {
                schema_version: 3,
                diagnostic: "psx-bios-pad-devices",
                core_sha256: hash,
                core_name: name,
                core_version: revision,
                input_descriptors,
                input_descriptor_updates,
                controller_choices: controller_choice_snapshot()?,
                input_mode: if bitmask { "bitmask" } else { "individual" },
                input_polls: POLLS.load(Ordering::Relaxed),
                bitmask_requests,
                individual_requests,
                analog_requests: ANALOG_REQUESTS.load(Ordering::Relaxed),
                joypad_requests_by_port: joypad_requests_by_port(),
                reported_system_ram_bytes,
                observation_memory_source,
                observation_memory_bytes,
                observation_memory_address,
                observations: Vec::new(),
                nes_observations: Vec::new(),
                snes_observations: Vec::new(),
                atari2600_observations: Vec::new(),
                psx_observations,
                firmware,
                runtime_libraries: runtime_libraries.clone(),
                contract_source_revision: Some("82d8e051d1c7741a18d930be90e458b48abaa9a1"),
                contract_source_url: Some(
                    "https://github.com/libretro/beetle-psx-libretro/tree/82d8e051d1c7741a18d930be90e458b48abaa9a1",
                ),
            });
        }
        if let Some(contract) = nes_contract {
            let nes_observations =
                run_nes_observations(&core, nes_topology, contract.controller_device)?;
            let requests = joypad_requests_by_port();
            for (port, &count) in requests.iter().enumerate() {
                if port < nes_topology.connected_ports() {
                    ensure!(count > 0, "NES port {} was never queried", port + 1);
                }
            }
            let bitmask_requests = MASK_REQUESTS.load(Ordering::Relaxed);
            let individual_requests = SINGLE_REQUESTS.load(Ordering::Relaxed);
            ensure!(
                if bitmask {
                    // FCEUmm uses the mask for standard-pad state but still
                    // issues individual auxiliary turbo/hotkey queries.
                    bitmask_requests > 0
                } else {
                    bitmask_requests == 0 && individual_requests > 0
                },
                "Core did not exercise the requested input callback mode"
            );
            let (input_descriptor_updates, input_descriptors) = input_descriptor_snapshot()?;
            let controller_choices = controller_choice_snapshot()?;
            validate_nes_metadata(
                &name,
                nes_topology,
                contract.controller_device,
                controller_choices.as_deref(),
                &input_descriptors,
            )?;
            return Ok(Report {
                schema_version: 4,
                diagnostic: "nes-controller-ports",
                core_sha256: hash,
                core_name: name,
                core_version: revision,
                input_descriptors,
                input_descriptor_updates,
                controller_choices,
                input_mode: if bitmask { "bitmask" } else { "individual" },
                input_polls: POLLS.load(Ordering::Relaxed),
                bitmask_requests,
                individual_requests,
                analog_requests: ANALOG_REQUESTS.load(Ordering::Relaxed),
                joypad_requests_by_port: requests,
                reported_system_ram_bytes,
                observation_memory_source,
                observation_memory_bytes,
                observation_memory_address,
                observations: Vec::new(),
                nes_observations,
                snes_observations: Vec::new(),
                atari2600_observations: Vec::new(),
                psx_observations: Vec::new(),
                firmware,
                runtime_libraries: runtime_libraries.clone(),
                contract_source_revision: Some(contract.source_revision),
                contract_source_url: Some(contract.source_url),
            });
        }
        if let Some(contract) = snes_contract {
            match contract.readback {
                SnesReadback::SystemRam => ensure!(
                    reported_system_ram_bytes == 128 * 1024,
                    "Unexpected exposed SNES WRAM size: {reported_system_ram_bytes}"
                ),
                SnesReadback::SerializedState => ensure!(
                    reported_system_ram_bytes == 0 && (core.memory)(2).is_null(),
                    "bsnes unexpectedly changed its standard memory-interface contract"
                ),
            }
            let snes_observations = run_snes_observations(&core, snes_topology, contract)?;
            let requests = joypad_requests_by_port();
            for (port, &count) in requests.iter().enumerate() {
                if port < snes_topology.connected_ports() {
                    ensure!(
                        count > 0,
                        "SNES frontend port {} was never queried",
                        port + 1
                    );
                } else {
                    ensure!(
                        count == 0,
                        "SNES disconnected frontend port {} was unexpectedly queried",
                        port + 1
                    );
                }
            }
            let bitmask_requests = MASK_REQUESTS.load(Ordering::Relaxed);
            let individual_requests = SINGLE_REQUESTS.load(Ordering::Relaxed);
            ensure!(
                if bitmask {
                    contract.supports_bitmask && bitmask_requests > 0 && individual_requests == 0
                } else {
                    bitmask_requests == 0 && individual_requests > 0
                },
                "Core did not exercise the requested input callback mode"
            );
            let (input_descriptor_updates, input_descriptors) = input_descriptor_snapshot()?;
            let controller_choices = controller_choice_snapshot()?;
            validate_snes_metadata(
                snes_topology,
                contract,
                controller_choices.as_deref(),
                &input_descriptors,
            )?;
            return Ok(Report {
                schema_version: 5,
                diagnostic: "snes-controller-ports",
                core_sha256: hash,
                core_name: name,
                core_version: revision,
                input_descriptors,
                input_descriptor_updates,
                controller_choices,
                input_mode: if bitmask { "bitmask" } else { "individual" },
                input_polls: POLLS.load(Ordering::Relaxed),
                bitmask_requests,
                individual_requests,
                analog_requests: ANALOG_REQUESTS.load(Ordering::Relaxed),
                joypad_requests_by_port: requests,
                reported_system_ram_bytes,
                observation_memory_source,
                observation_memory_bytes,
                observation_memory_address,
                observations: Vec::new(),
                nes_observations: Vec::new(),
                snes_observations,
                atari2600_observations: Vec::new(),
                psx_observations: Vec::new(),
                firmware,
                runtime_libraries: runtime_libraries.clone(),
                contract_source_revision: Some(contract.source_revision),
                contract_source_url: Some(contract.source_url),
            });
        }
        if matches!(diagnostic, Diagnostic::Gameboy) {
            let contract = gameboy_contract.context("Missing Game Boy core contract")?;
            // Allow the real built-in open boot ROM to finish. Do not patch
            // CPU state or substitute memory for an executed diagnostic.
            let mut booted = false;
            for _ in 0..240 {
                (core.run)();
                ensure!(
                    (core.memory_size)(2) >= contract.system_ram_offset + 8,
                    "Game Boy RAM unavailable during boot"
                );
                let memory = (core.memory)(2).cast::<u8>();
                ensure!(!memory.is_null(), "Game Boy RAM pointer unavailable");
                let bytes = std::slice::from_raw_parts(memory.add(contract.system_ram_offset), 8);
                if u32::from_le_bytes(bytes[4..8].try_into().unwrap()) == 0x4c42494e {
                    booted = true;
                    break;
                }
            }
            ensure!(
                booted,
                "Original Game Boy diagnostic did not boot in 240 frames"
            );
        }
        let mut observations = Vec::new();
        let device_modes = gameboy_contract
            .map(|contract| contract.device_modes)
            .unwrap_or(std::slice::from_ref(&device));
        for &test_device in device_modes {
            if matches!(diagnostic, Diagnostic::Gameboy) {
                set_device(0, test_device);
            }
            // Hardware register bits are independent of the core's RetroPad IDs.
            for &(name, rp, hardware) in diagnostic.cases() {
                for (step_name, mask, expected) in [
                    (name, rp, register_mask ^ hardware),
                    ("release", 0, register_mask),
                ] {
                    PRESSED[0].store(mask, Ordering::Relaxed);
                    for _ in 0..4 {
                        (core.run)();
                    }
                    let bytes = if let Some(contract) = gba_contract {
                        &gba_observation_memory(&core, contract)?[..8]
                    } else {
                        let memory_offset = gameboy_contract
                            .map(|contract| contract.system_ram_offset)
                            .unwrap_or(0);
                        ensure!(
                            (core.memory_size)(2) >= memory_offset + 8,
                            "Diagnostic memory became unavailable"
                        );
                        let memory = (core.memory)(2).cast::<u8>();
                        ensure!(!memory.is_null(), "No system RAM exposed by core");
                        std::slice::from_raw_parts(memory.add(memory_offset), 8)
                    };
                    ensure!(
                        u32::from_le_bytes(bytes[4..8].try_into().unwrap()) == 0x4c42494e,
                        "Diagnostic program did not execute"
                    );
                    let observed = u16::from_le_bytes([bytes[0], bytes[1]]) & register_mask;
                    ensure!(
                        observed == expected,
                        "{step_name}: expected input register bits {expected:#06x}, observed {observed:#06x}"
                    );
                    observations.push(Observation {
                        name: if matches!(diagnostic, Diagnostic::Gameboy) {
                            format!("{step_name} (device {test_device})")
                        } else {
                            step_name.into()
                        },
                        retropad_mask: mask,
                        expected_keyinput: expected,
                        observed_keyinput: observed,
                    });
                }
            }
        }
        ensure!(
            POLLS.load(Ordering::Relaxed) > 0,
            "Core never polled frontend input"
        );
        let bitmask_requests = MASK_REQUESTS.load(Ordering::Relaxed);
        let individual_requests = SINGLE_REQUESTS.load(Ordering::Relaxed);
        ensure!(
            if bitmask {
                bitmask_requests > 0
            } else {
                bitmask_requests == 0 && individual_requests > 0
            },
            "Core did not exercise the requested input callback mode"
        );
        let (input_descriptor_updates, input_descriptors) = input_descriptor_snapshot()?;
        Ok(Report {
            schema_version: if gba_contract.is_some() {
                8
            } else if gameboy_contract.is_some() {
                6
            } else {
                1
            },
            diagnostic: match diagnostic {
                Diagnostic::Gba => "gba-keyinput",
                Diagnostic::Gamegear => "gamegear-dc-00",
                Diagnostic::Gameboy => "gameboy-joyp",
                Diagnostic::Atari2600 => unreachable!(),
                Diagnostic::Nes => unreachable!(),
                Diagnostic::Snes => unreachable!(),
                Diagnostic::Psx => unreachable!(),
            },
            core_sha256: hash,
            core_name: name,
            core_version: revision,
            input_descriptors,
            input_descriptor_updates,
            controller_choices: controller_choice_snapshot()?,
            input_mode: if bitmask { "bitmask" } else { "individual" },
            input_polls: POLLS.load(Ordering::Relaxed),
            bitmask_requests,
            individual_requests,
            analog_requests: ANALOG_REQUESTS.load(Ordering::Relaxed),
            joypad_requests_by_port: joypad_requests_by_port(),
            reported_system_ram_bytes,
            observation_memory_source,
            observation_memory_bytes,
            observation_memory_address,
            observations,
            nes_observations: Vec::new(),
            snes_observations: Vec::new(),
            atari2600_observations: Vec::new(),
            psx_observations: Vec::new(),
            firmware,
            runtime_libraries,
            contract_source_revision: gba_contract
                .map(|contract| contract.source_revision)
                .or_else(|| gameboy_contract.map(|contract| contract.source_revision)),
            contract_source_url: gba_contract
                .map(|contract| contract.source_url)
                .or_else(|| gameboy_contract.map(|contract| contract.source_url)),
        })
    }
}

const NES_RESULT_OFFSETS: [usize; 4] = [0, 4, 5, 6];
const NES_RETROPAD_IDS: [u32; 8] = [8, 0, 2, 3, 4, 5, 6, 7];

fn validate_atari2600_metadata(
    controller_choices: Option<&[Vec<ControllerChoice>]>,
    descriptors: &[InputDescriptor],
) -> Result<(&'static str, &'static str)> {
    let choices = controller_choices.context("Stella did not advertise controller choices")?;
    ensure!(
        choices.len() == 4,
        "Stella did not advertise four input ports"
    );
    let primary_choices = [
        ("Automatic (from ROM database)", 1),
        ("Joystick", 257),
        ("BoosterGrip", 513),
        ("Genesis", 769),
        ("Joy 2B+", 1025),
        ("Paddles", 1281),
        ("Driving", 1537),
        ("Keyboard", 1793),
        ("TrakBall", 2049),
        ("Amiga Mouse", 2305),
        ("Atari Mouse", 2561),
        ("Lightgun", 2817),
        ("QuadTari", 3073),
        ("MindLink", 3329),
        ("AtariVox", 3585),
        ("SaveKey", 3841),
        ("KidVid", 4097),
        ("None", 0),
    ];
    let legacy = choices.iter().all(|choices| {
        choices
            == &[
                ControllerChoice {
                    description: "Automatic".into(),
                    id: 1,
                },
                ControllerChoice {
                    description: "None".into(),
                    id: 0,
                },
            ]
    });
    for (port, choices) in choices.iter().enumerate() {
        let expected: Vec<ControllerChoice> = if port < 2 {
            if legacy {
                [("Automatic", 1), ("None", 0)]
                    .into_iter()
                    .map(|(description, id)| ControllerChoice {
                        description: description.into(),
                        id,
                    })
                    .collect()
            } else {
                primary_choices
                    .iter()
                    .map(|&(description, id)| ControllerChoice {
                        description: description.into(),
                        id,
                    })
                    .collect()
            }
        } else {
            [("Automatic", 1), ("None", 0)]
                .into_iter()
                .map(|(description, id)| ControllerChoice {
                    description: description.into(),
                    id,
                })
                .collect()
        };
        ensure!(
            choices == &expected,
            "Unexpected Stella controller choices on port {}: {choices:?}",
            port + 1
        );
    }
    ensure!(
        descriptors.len() == 40,
        "Unexpected Stella input descriptor count: {}",
        descriptors.len()
    );
    let required = [
        (4, "Up"),
        (5, "Down"),
        (6, "Left"),
        (7, "Right"),
        (0, "Fire"),
        (8, "Trigger"),
        (1, "Booster"),
        (2, "Select"),
        (3, "Reset"),
        (10, "Left Difficulty A"),
        (11, "Right Difficulty A"),
        (12, "Left Difficulty B"),
        (13, "Right Difficulty B"),
        (14, "Color"),
        (15, "Black/White"),
    ];
    for port in 0..2 {
        for &(id, description) in &required {
            ensure!(
                descriptors.iter().any(|entry| {
                    entry.port == port
                        && entry.device == 1
                        && entry.index == 0
                        && entry.id == id
                        && entry.description == description
                }),
                "Stella is missing the expected {description} descriptor on port {}",
                port + 1
            );
        }
    }
    Ok(if legacy {
        (
            "ba52c43b9eda950eb0c0eec69cda9b17dee8c39b",
            "https://github.com/libretro/stella/tree/ba52c43b9eda950eb0c0eec69cda9b17dee8c39b",
        )
    } else {
        (
            "c65c845c8686c81698ffbd2fc9dfc5ccea5b32a1",
            "https://github.com/stella-emu/stella/tree/c65c845c8686c81698ffbd2fc9dfc5ccea5b32a1",
        )
    })
}

fn atari2600_expected_registers(port_masks: [u16; 2], swchb: u8) -> [u8; 4] {
    let mut swcha = 0xff;
    for (port, mask) in port_masks.into_iter().enumerate() {
        for (retropad_id, direction) in [(4, 0), (5, 1), (6, 2), (7, 3)] {
            if mask & (1 << retropad_id) != 0 {
                let riot_bit = if port == 0 { direction + 4 } else { direction };
                swcha &= !(1 << riot_bit);
            }
        }
    }
    [
        swcha,
        if port_masks[0] & 1 == 0 { 0x80 } else { 0 },
        if port_masks[1] & 1 == 0 { 0x80 } else { 0 },
        swchb,
    ]
}

fn read_atari2600_registers(core: &Core) -> Result<[u8; 4]> {
    unsafe {
        ensure!(
            (core.memory_size)(2) == 128,
            "Atari 2600 RIOT RAM became unavailable"
        );
        let memory = (core.memory)(2).cast::<u8>();
        ensure!(!memory.is_null(), "No Atari 2600 RIOT RAM exposed by core");
        let bytes = std::slice::from_raw_parts(memory, 8);
        ensure!(
            &bytes[4..8] == b"LB26",
            "Atari 2600 diagnostic program did not execute"
        );
        Ok(bytes[..4].try_into().unwrap())
    }
}

fn wait_for_atari2600_registers(core: &Core) -> Result<[u8; 4]> {
    let mut last = None;
    for _ in 0..120 {
        unsafe { (core.run)() };
        match read_atari2600_registers(core) {
            Ok(registers) => return Ok(registers),
            Err(error) => last = Some(error),
        }
    }
    let detail = last.map(|error| format!(": {error:#}")).unwrap_or_default();
    anyhow::bail!("Original Atari 2600 diagnostic did not boot in 120 frames{detail}")
}

fn compare_atari2600_registers(
    name: &str,
    observed: [u8; 4],
    expected: [u8; 4],
    masks: [u8; 4],
) -> Result<()> {
    for index in 0..4 {
        ensure!(
            observed[index] & masks[index] == expected[index] & masks[index],
            "Atari 2600 {name}: register {index} expected {:#04x} under mask {:#04x}, observed {:#04x}",
            expected[index],
            masks[index],
            observed[index]
        );
    }
    Ok(())
}

fn run_atari2600_observations(core: &Core) -> Result<Vec<Atari2600Observation>> {
    for pressed in &PRESSED {
        pressed.store(0, Ordering::Relaxed);
    }
    let initial = wait_for_atari2600_registers(core)?;
    let joystick_masks = [0xff, 0x80, 0x80, 0];
    let released = atari2600_expected_registers([0, 0], initial[3]);
    compare_atari2600_registers("initial release", initial, released, joystick_masks)?;

    let cases = [
        ("released", 0),
        ("Fire", 1 << 0),
        ("Up", 1 << 4),
        ("Down", 1 << 5),
        ("Left", 1 << 6),
        ("Right", 1 << 7),
        ("Up+Left", (1 << 4) | (1 << 6)),
    ];
    let mut observations = Vec::new();
    for target in 0..2 {
        for (case_index, &(name, target_mask)) in cases.iter().enumerate() {
            let other = 1 - target;
            let mut masks = [0u16; 2];
            masks[target] = target_mask;
            // A distinct held direction+fire on the other port makes routing
            // errors visible in every target-port observation.
            masks[other] = if (case_index + target) % 2 == 0 {
                (1 << 7) | 1
            } else {
                (1 << 6) | 1
            };
            for (port, mask) in masks.into_iter().enumerate() {
                PRESSED[port].store(mask, Ordering::Relaxed);
            }
            for _ in 0..4 {
                unsafe { (core.run)() };
            }
            let observed = read_atari2600_registers(core)?;
            let expected = atari2600_expected_registers(masks, observed[3]);
            compare_atari2600_registers(name, observed, expected, joystick_masks)?;
            observations.push(Atari2600Observation {
                target_port: u32::try_from(target + 1).unwrap(),
                name: name.into(),
                retropad_masks: masks,
                expected_registers: expected,
                observed_registers: observed,
                comparison_masks: joystick_masks,
            });

            if target_mask != 0 {
                PRESSED[0].store(0, Ordering::Relaxed);
                PRESSED[1].store(0, Ordering::Relaxed);
                for _ in 0..4 {
                    unsafe { (core.run)() };
                }
                let observed = read_atari2600_registers(core)?;
                let expected = atari2600_expected_registers([0, 0], observed[3]);
                compare_atari2600_registers(
                    &format!("release after port {} {name}", target + 1),
                    observed,
                    expected,
                    joystick_masks,
                )?;
                observations.push(Atari2600Observation {
                    target_port: u32::try_from(target + 1).unwrap(),
                    name: format!("release after {name}"),
                    retropad_masks: [0, 0],
                    expected_registers: expected,
                    observed_registers: observed,
                    comparison_masks: joystick_masks,
                });
            }
        }
    }

    // Console switches are driven from frontend port zero regardless of the
    // emulated joystick port. Difficulty and color/BW are latched settings;
    // Select and Reset are active only while held.
    let switch_cases = [
        ("Select", 1 << 2, 0x02, 0x00),
        ("Reset", 1 << 3, 0x01, 0x00),
        ("Left difficulty A", 1 << 10, 0x40, 0x40),
        ("Left difficulty B", 1 << 12, 0x40, 0x00),
        ("Right difficulty A", 1 << 11, 0x80, 0x80),
        ("Right difficulty B", 1 << 13, 0x80, 0x00),
        ("Color", 1 << 14, 0x08, 0x08),
        ("Black/White", 1 << 15, 0x08, 0x00),
    ];
    for &(name, retropad_mask, switch_mask, expected_switch_bits) in &switch_cases {
        PRESSED[0].store(retropad_mask, Ordering::Relaxed);
        PRESSED[1].store(0, Ordering::Relaxed);
        for _ in 0..4 {
            unsafe { (core.run)() };
        }
        let observed = read_atari2600_registers(core)?;
        let expected = atari2600_expected_registers([retropad_mask, 0], expected_switch_bits);
        let comparison_masks = [0xff, 0x80, 0x80, switch_mask];
        compare_atari2600_registers(name, observed, expected, comparison_masks)?;
        observations.push(Atari2600Observation {
            target_port: 0,
            name: name.into(),
            retropad_masks: [retropad_mask, 0],
            expected_registers: expected,
            observed_registers: observed,
            comparison_masks,
        });

        PRESSED[0].store(0, Ordering::Relaxed);
        for _ in 0..4 {
            unsafe { (core.run)() };
        }
        if switch_mask <= 0x02 {
            let observed = read_atari2600_registers(core)?;
            let expected = atari2600_expected_registers([0, 0], switch_mask);
            compare_atari2600_registers(
                &format!("release after {name}"),
                observed,
                expected,
                comparison_masks,
            )?;
            observations.push(Atari2600Observation {
                target_port: 0,
                name: format!("release after {name}"),
                retropad_masks: [0, 0],
                expected_registers: expected,
                observed_registers: observed,
                comparison_masks,
            });
        }
    }
    Ok(observations)
}

fn nes_hardware_bits(retropad_mask: u16) -> u8 {
    NES_RETROPAD_IDS
        .iter()
        .enumerate()
        .fold(0u8, |bits, (nes_bit, &retropad_id)| {
            bits | (((retropad_mask >> retropad_id) & 1) as u8) << nes_bit
        })
}

fn read_nes_registers(core: &Core, connected_ports: usize) -> Result<Vec<u8>> {
    unsafe {
        ensure!(
            (1..=NES_RESULT_OFFSETS.len()).contains(&connected_ports) && (core.memory_size)(2) >= 7,
            "NES diagnostic memory became unavailable"
        );
        let memory = (core.memory)(2).cast::<u8>();
        ensure!(!memory.is_null(), "No NES system RAM exposed by core");
        let bytes = std::slice::from_raw_parts(memory, 7);
        ensure!(
            &bytes[1..4] == b"LBN",
            "NES diagnostic program did not execute"
        );
        Ok(NES_RESULT_OFFSETS[..connected_ports]
            .iter()
            .map(|&offset| bytes[offset])
            .collect())
    }
}

fn wait_for_nes_registers(core: &Core, connected_ports: usize) -> Result<Vec<u8>> {
    for _ in 0..120 {
        unsafe { (core.run)() };
        unsafe {
            if (core.memory_size)(2) >= 7 {
                let memory = (core.memory)(2).cast::<u8>();
                if !memory.is_null() && std::slice::from_raw_parts(memory.add(1), 3) == b"LBN" {
                    return read_nes_registers(core, connected_ports);
                }
            }
        }
    }
    anyhow::bail!("Original NES diagnostic did not boot in 120 frames")
}

fn run_nes_observations(
    core: &Core,
    topology: NesTopology,
    controller_device: u32,
) -> Result<Vec<NesObservation>> {
    let connected_ports = topology.connected_ports();
    for pressed in &PRESSED {
        pressed.store(0, Ordering::Relaxed);
    }
    wait_for_nes_registers(core, connected_ports)?;
    // The marker is written before the first polling pass. Let the core finish
    // complete frames before treating the published controller bytes as data.
    for _ in 0..4 {
        unsafe { (core.run)() };
    }
    let initial = read_nes_registers(core, connected_ports)?;
    ensure!(
        initial.iter().all(|&value| value == 0xff),
        "NES released state was not active-low FF: {initial:02x?}"
    );

    let mut observations = Vec::new();
    for target_port in 0..connected_ports {
        for (case_index, &(name, target_mask, expected_bits)) in
            Diagnostic::Nes.cases().iter().enumerate()
        {
            let mut masks = Vec::with_capacity(connected_ports);
            for port in 0..connected_ports {
                let mask = if port == target_port {
                    target_mask
                } else {
                    1u16 << NES_RETROPAD_IDS[(port + target_port + case_index + 1) % 8]
                };
                PRESSED[port].store(mask, Ordering::Relaxed);
                masks.push(mask);
            }
            // Keep synthetic input asserted even on disconnected frontend
            // ports. The hardware readback proves those values did not enter
            // this topology; the report also preserves whether a core queried
            // the frontend port before discarding its state.
            for port in connected_ports..PRESSED.len() {
                PRESSED[port].store(1 << NES_RETROPAD_IDS[port % 8], Ordering::Relaxed);
            }
            for _ in 0..4 {
                unsafe { (core.run)() };
            }
            let observed = read_nes_registers(core, connected_ports)?;
            let expected: Vec<u8> = masks.iter().map(|&mask| !nes_hardware_bits(mask)).collect();
            ensure!(
                expected[target_port] == !(expected_bits as u8),
                "Internal NES case mapping mismatch for {name}"
            );
            ensure!(
                observed == expected,
                "NES {} port {} {name}: expected active-low bytes {expected:02x?}, observed {observed:02x?}",
                topology.name(),
                target_port + 1
            );
            observations.push(NesObservation {
                topology: topology.name(),
                target_port: u32::try_from(target_port + 1).unwrap(),
                controller_device,
                name: name.into(),
                retropad_masks: masks,
                expected_registers: expected,
                observed_registers: observed,
            });

            if target_mask != 0 {
                for pressed in &PRESSED {
                    pressed.store(0, Ordering::Relaxed);
                }
                for _ in 0..4 {
                    unsafe { (core.run)() };
                }
                let observed = read_nes_registers(core, connected_ports)?;
                let expected = vec![0xff; connected_ports];
                ensure!(
                    observed == expected,
                    "NES {} release after port {} {name}: expected {expected:02x?}, observed {observed:02x?}",
                    topology.name(),
                    target_port + 1
                );
                observations.push(NesObservation {
                    topology: topology.name(),
                    target_port: u32::try_from(target_port + 1).unwrap(),
                    controller_device,
                    name: format!("release after {name}"),
                    retropad_masks: vec![0; connected_ports],
                    expected_registers: expected,
                    observed_registers: observed,
                });
            }
        }
    }
    Ok(observations)
}

fn parse_snes_wram(bytes: &[u8], marker_offset: usize) -> Result<Vec<u16>> {
    ensure!(
        marker_offset >= SNES_MARKER_OFFSET_DELTA
            && marker_offset + SNES_MARKER.len() + 10 <= bytes.len(),
        "SNES diagnostic WRAM marker has invalid bounds"
    );
    let generation = bytes[marker_offset - SNES_MARKER_OFFSET_DELTA];
    ensure!(
        generation != 0 && generation & 1 == 0,
        "SNES diagnostic sample is incomplete (generation {generation})"
    );
    let results = marker_offset + SNES_MARKER.len();
    Ok((0..5)
        .map(|port| {
            u16::from_le_bytes(
                bytes[results + port * 2..results + port * 2 + 2]
                    .try_into()
                    .unwrap(),
            )
        })
        .collect())
}

const SNES_MARKER_OFFSET_DELTA: usize = SNES_MARKER_OFFSET - SNES_GENERATION_OFFSET;

fn read_snes_registers(core: &Core, contract: SnesCoreContract) -> Result<Vec<u16>> {
    unsafe {
        match contract.readback {
            SnesReadback::SystemRam => {
                ensure!(
                    (core.memory_size)(2) >= SNES_RESULTS_OFFSET + 10,
                    "SNES diagnostic WRAM became unavailable"
                );
                let memory = (core.memory)(2).cast::<u8>();
                ensure!(!memory.is_null(), "No SNES system RAM exposed by core");
                let bytes = std::slice::from_raw_parts(memory, SNES_RESULTS_OFFSET + 10);
                ensure!(
                    &bytes[SNES_MARKER_OFFSET..SNES_MARKER_OFFSET + SNES_MARKER.len()]
                        == SNES_MARKER,
                    "SNES diagnostic program did not execute"
                );
                parse_snes_wram(bytes, SNES_MARKER_OFFSET)
            }
            SnesReadback::SerializedState => {
                let size = (core.serialize_size)();
                ensure!(
                    (SNES_MARKER.len() + 12..=64 * 1024 * 1024).contains(&size),
                    "SNES serialized-state size is outside diagnostic bounds: {size}"
                );
                let mut state = vec![0u8; size];
                ensure!(
                    (core.serialize)(state.as_mut_ptr().cast(), state.len()),
                    "SNES core rejected a correctly sized state snapshot"
                );
                let matches: Vec<usize> = state
                    .windows(SNES_MARKER.len())
                    .enumerate()
                    .filter_map(|(offset, bytes)| (bytes == SNES_MARKER).then_some(offset))
                    .collect();
                ensure!(
                    matches.len() == 1,
                    "Expected one diagnostic WRAM marker in serialized state, found {}",
                    matches.len()
                );
                parse_snes_wram(&state, matches[0])
            }
        }
    }
}

fn wait_for_snes_registers(core: &Core, contract: SnesCoreContract) -> Result<Vec<u16>> {
    let mut last_error = None;
    for _ in 0..240 {
        unsafe { (core.run)() };
        match read_snes_registers(core, contract) {
            Ok(registers) => return Ok(registers),
            Err(error) => last_error = Some(error),
        }
    }
    let detail = last_error
        .map(|error| format!(": {error:#}"))
        .unwrap_or_default();
    anyhow::bail!("Original SNES diagnostic did not boot in 240 frames{detail}")
}

fn run_snes_observations(
    core: &Core,
    topology: SnesTopology,
    contract: SnesCoreContract,
) -> Result<Vec<SnesObservation>> {
    let connected_ports = topology.connected_ports();
    for pressed in &PRESSED {
        pressed.store(0, Ordering::Relaxed);
    }
    let initial = wait_for_snes_registers(core, contract)?;
    ensure!(
        initial[..connected_ports].iter().all(|&value| value == 0),
        "SNES released state was not zero: {:04x?}",
        &initial[..connected_ports]
    );

    let mut observations = Vec::new();
    for target_port in 0..connected_ports {
        for (case_index, &(name, target_mask, expected_bits)) in
            Diagnostic::Snes.cases().iter().enumerate()
        {
            let masks: Vec<u16> = (0..connected_ports)
                .map(|port| {
                    if port == target_port {
                        target_mask
                    } else {
                        1u16 << ((port + target_port + case_index + 1) % 12)
                    }
                })
                .collect();
            for (port, &mask) in masks.iter().enumerate() {
                PRESSED[port].store(mask, Ordering::Relaxed);
            }
            for (port, pressed) in PRESSED.iter().enumerate().skip(connected_ports) {
                pressed.store(1 << ((port + case_index) % 12), Ordering::Relaxed);
            }
            for _ in 0..4 {
                unsafe { (core.run)() };
            }
            let observed = read_snes_registers(core, contract)?;
            let expected: Vec<u16> = masks.iter().map(|mask| mask & 0x0fff).collect();
            ensure!(
                expected[target_port] == expected_bits,
                "Internal SNES case mapping mismatch for {name}"
            );
            ensure!(
                observed[..connected_ports] == expected,
                "SNES {} port {} {name}: expected words {expected:04x?}, observed {:04x?}",
                topology.name(),
                target_port + 1,
                &observed[..connected_ports]
            );
            observations.push(SnesObservation {
                topology: topology.name(),
                target_port: u32::try_from(target_port + 1).unwrap(),
                name: name.into(),
                retropad_masks: masks,
                expected_registers: expected.clone(),
                observed_registers: observed[..connected_ports].to_vec(),
                readback: contract.readback.name(),
            });

            if target_mask != 0 {
                for pressed in &PRESSED {
                    pressed.store(0, Ordering::Relaxed);
                }
                for _ in 0..4 {
                    unsafe { (core.run)() };
                }
                let observed = read_snes_registers(core, contract)?;
                let expected = vec![0; connected_ports];
                ensure!(
                    observed[..connected_ports] == expected,
                    "SNES {} release after port {} {name}: expected zeroes, observed {:04x?}",
                    topology.name(),
                    target_port + 1,
                    &observed[..connected_ports]
                );
                observations.push(SnesObservation {
                    topology: topology.name(),
                    target_port: u32::try_from(target_port + 1).unwrap(),
                    name: format!("release after {name}"),
                    retropad_masks: vec![0; connected_ports],
                    expected_registers: expected,
                    observed_registers: observed[..connected_ports].to_vec(),
                    readback: contract.readback.name(),
                });
            }
        }
    }
    Ok(observations)
}

const PSX_RESULT_OFFSET: usize = 0x20000;
const PSX_PAD_BUFFER_OFFSETS: [usize; 2] = [0x20, 0x60];

fn set_psx_state(port: usize, retropad_mask: u16, analog: [i16; 4]) {
    PRESSED[port].store(retropad_mask, Ordering::Relaxed);
    for (axis, value) in analog.into_iter().enumerate() {
        ANALOG[port * 4 + axis].store(value, Ordering::Relaxed);
    }
}

fn dualshock_axis(value: i16) -> u8 {
    let value = i32::from(value);
    let positive = value.max(0);
    let negative = (-value).max(0);
    let native = 32768 + positive - negative * 32768 / 32767;
    (native >> 8) as u8
}

fn expected_dualshock_buffer(psx_buttons: u16, analog: [i16; 4]) -> [u8; 8] {
    let buttons = !psx_buttons;
    [
        0x00,
        0x73,
        buttons as u8,
        (buttons >> 8) as u8,
        dualshock_axis(analog[2]),
        dualshock_axis(analog[3]),
        dualshock_axis(analog[0]),
        dualshock_axis(analog[1]),
    ]
}

fn read_psx_pad_buffer(core: &Core, port: usize) -> Result<[u8; 8]> {
    unsafe {
        ensure!(
            port < PSX_PAD_BUFFER_OFFSETS.len()
                && (core.memory_size)(2) >= PSX_RESULT_OFFSET + PSX_PAD_BUFFER_OFFSETS[port] + 8,
            "Diagnostic memory became unavailable"
        );
        let memory = (core.memory)(2).cast::<u8>();
        ensure!(!memory.is_null(), "No PSX system RAM exposed by core");
        let packet = memory.add(PSX_RESULT_OFFSET + PSX_PAD_BUFFER_OFFSETS[port]);
        Ok(std::slice::from_raw_parts(packet, 8).try_into().unwrap())
    }
}

fn wait_for_psx_pad_buffer(core: &Core, port: usize, expected: &[u8]) -> Result<Vec<u8>> {
    ensure!(
        !expected.is_empty() && expected.len() <= 8,
        "Invalid expected PAD buffer"
    );
    let mut observed = read_psx_pad_buffer(core, port)?;
    for _ in 0..12 {
        unsafe { (core.run)() };
        observed = read_psx_pad_buffer(core, port)?;
        if &observed[..expected.len()] == expected {
            return Ok(observed[..expected.len()].to_vec());
        }
    }
    Ok(observed[..expected.len()].to_vec())
}

fn psx_debug_words(core: &Core) -> [u32; 8] {
    unsafe {
        let memory = (core.memory)(2).cast::<u8>();
        std::array::from_fn(|index| {
            u32::from_le_bytes(
                std::slice::from_raw_parts(memory.add(PSX_RESULT_OFFSET + index * 4), 4)
                    .try_into()
                    .unwrap(),
            )
        })
    }
}

const PSX_BUTTON_CASES: [(&str, u32, u32); 16] = [
    ("B / Cross", 0, 14),
    ("Y / Square", 1, 15),
    ("Select", 2, 0),
    ("Start", 3, 3),
    ("Up", 4, 4),
    ("Down", 5, 6),
    ("Left", 6, 7),
    ("Right", 7, 5),
    ("A / Circle", 8, 13),
    ("X / Triangle", 9, 12),
    ("L / L1", 10, 10),
    ("R / R1", 11, 11),
    ("L2", 12, 8),
    ("R2", 13, 9),
    ("L3", 14, 1),
    ("R3", 15, 2),
];

const PSX_ANALOG_CASES: [(&str, usize, i16); 8] = [
    ("left X positive", 0, 16384),
    ("left X negative", 0, -16384),
    ("left Y positive", 1, 16384),
    ("left Y negative", 1, -16384),
    ("right X positive", 2, 16384),
    ("right X negative", 2, -16384),
    ("right Y positive", 3, 16384),
    ("right Y negative", 3, -16384),
];

fn observe_psx_case(
    core: &Core,
    port: usize,
    controller_device: u32,
    name: &str,
    retropad_mask: u16,
    analog: [i16; 4],
    expected: &[u8],
) -> Result<PsxObservation> {
    set_psx_state(port, retropad_mask, analog);
    let observed = wait_for_psx_pad_buffer(core, port, expected)?;
    ensure!(
        observed == expected,
        "port {} device {controller_device} {name}: expected PAD buffer {:02x?}, observed {:02x?}; debug words {:08x?}",
        port + 1,
        expected,
        observed,
        psx_debug_words(core)
    );
    Ok(PsxObservation {
        port: u32::try_from(port + 1).unwrap(),
        controller_device,
        name: name.into(),
        retropad_mask,
        analog,
        expected_pad_buffer: expected.to_vec(),
        observed_pad_buffer: observed,
    })
}

fn run_dualshock_observations(core: &Core, port: usize) -> Result<Vec<PsxObservation>> {
    set_psx_state(0, 0, [0; 4]);
    set_psx_state(1, 0, [0; 4]);
    let mut observations = vec![observe_psx_case(
        core,
        port,
        517,
        "released",
        0,
        [0; 4],
        &expected_dualshock_buffer(0, [0; 4]),
    )?];
    for &(name, retropad_id, psx_bit) in &PSX_BUTTON_CASES {
        let mask = 1 << retropad_id;
        observations.push(observe_psx_case(
            core,
            port,
            517,
            name,
            mask,
            [0; 4],
            &expected_dualshock_buffer(1 << psx_bit, [0; 4]),
        )?);
    }
    for &(name, axis, value) in &PSX_ANALOG_CASES {
        let mut analog = [0; 4];
        analog[axis] = value;
        observations.push(observe_psx_case(
            core,
            port,
            517,
            name,
            0,
            analog,
            &expected_dualshock_buffer(0, analog),
        )?);
    }
    set_psx_state(port, 0, [0; 4]);
    Ok(observations)
}

fn run_digital_pad_observations(core: &Core, port: usize) -> Result<Vec<PsxObservation>> {
    set_psx_state(0, 0, [0; 4]);
    set_psx_state(1, 0, [0; 4]);
    let expected_released = [0x00, 0x41, 0xff, 0xff];
    let mut observations = vec![observe_psx_case(
        core,
        port,
        1,
        "released",
        0,
        [0; 4],
        &expected_released,
    )?];
    for &(name, retropad_id, psx_bit) in &PSX_BUTTON_CASES[..14] {
        let mask = 1 << retropad_id;
        let buttons = !(1u16 << psx_bit);
        let expected = [0x00, 0x41, buttons as u8, (buttons >> 8) as u8];
        observations.push(observe_psx_case(
            core, port, 1, name, mask, [0; 4], &expected,
        )?);
    }
    set_psx_state(port, 0, [0; 4]);
    Ok(observations)
}

fn run_psx_observations(core: &Core) -> Result<Vec<PsxObservation>> {
    // The BIOS boot path is real emulated execution. Wait up to 360 emulated
    // frames for our RAM marker instead of assuming a host-time delay.
    let mut booted = false;
    for _ in 0..360 {
        unsafe { (core.run)() };
        unsafe {
            let memory = (core.memory)(2).cast::<u8>();
            if !memory.is_null()
                && u32::from_le_bytes(
                    std::slice::from_raw_parts(memory.add(PSX_RESULT_OFFSET), 4)
                        .try_into()
                        .unwrap(),
                ) == 0x5350_424c
            {
                booted = true;
                break;
            }
        }
    }
    ensure!(
        booted,
        "PS-X EXE did not execute within 360 emulated frames"
    );
    let mut generation = 0;
    for _ in 0..30 {
        unsafe { (core.run)() };
        unsafe {
            let memory = (core.memory)(2).cast::<u8>();
            generation = u32::from_le_bytes(
                std::slice::from_raw_parts(memory.add(PSX_RESULT_OFFSET + 4), 4)
                    .try_into()
                    .unwrap(),
            );
        }
        if generation != 0 {
            break;
        }
    }
    if generation == 0 {
        unsafe {
            let memory = (core.memory)(2).cast::<u8>();
            let word = |offset| {
                u32::from_le_bytes(
                    std::slice::from_raw_parts(memory.add(PSX_RESULT_OFFSET + offset), 4)
                        .try_into()
                        .unwrap(),
                )
            };
            anyhow::bail!(
                "PS-X EXE did not complete a two-port poll: error={:#010x}, port={}, byte={}, JOY_STAT={:#010x}",
                word(0x0c),
                word(0x10),
                word(0x14),
                word(0x18)
            );
        }
    }

    let mut observations = Vec::new();
    for port in 0..2 {
        observations.extend(run_dualshock_observations(core, port)?);
    }
    unsafe {
        (core.set_device)(0, 1);
        (core.set_device)(1, 517);
    }
    let digital_analog_before = ANALOG_REQUESTS_BY_PORT[0].load(Ordering::Relaxed);
    observations.extend(run_digital_pad_observations(core, 0)?);
    ensure!(
        ANALOG_REQUESTS_BY_PORT[0].load(Ordering::Relaxed) == digital_analog_before,
        "Standard digital controller unexpectedly requested analog axes"
    );
    observations.extend(run_dualshock_observations(core, 1)?);
    ensure!(
        POLLS.load(Ordering::Relaxed) > 0,
        "Core never polled frontend input"
    );
    Ok(observations)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn controller_capture_accepts_only_a_counted_terminal_null_choice() {
        let label = CString::new("PlayStation Controller").unwrap();
        let valid_choices = [
            NativeControllerChoice {
                description: label.as_ptr(),
                id: 1,
            },
            NativeControllerChoice {
                description: std::ptr::null(),
                id: 0,
            },
        ];
        let valid_ports = [
            NativeControllerInfo {
                types: valid_choices.as_ptr(),
                count: valid_choices.len() as u32,
            },
            NativeControllerInfo {
                types: std::ptr::null(),
                count: 0,
            },
        ];
        let captured = unsafe { capture_controller_choices(valid_ports.as_ptr()) }.unwrap();
        assert_eq!(captured.len(), 1);
        assert_eq!(captured[0].len(), 1);
        assert_eq!(captured[0][0].description, "PlayStation Controller");
        assert_eq!(captured[0][0].id, 1);

        let misplaced_null = [
            NativeControllerChoice {
                description: std::ptr::null(),
                id: 0,
            },
            NativeControllerChoice {
                description: label.as_ptr(),
                id: 1,
            },
        ];
        let misplaced_ports = [
            NativeControllerInfo {
                types: misplaced_null.as_ptr(),
                count: misplaced_null.len() as u32,
            },
            NativeControllerInfo {
                types: std::ptr::null(),
                count: 0,
            },
        ];
        assert!(unsafe { capture_controller_choices(misplaced_ports.as_ptr()) }.is_err());

        let nonzero_terminator = [
            NativeControllerChoice {
                description: label.as_ptr(),
                id: 1,
            },
            NativeControllerChoice {
                description: std::ptr::null(),
                id: 7,
            },
        ];
        let nonzero_ports = [
            NativeControllerInfo {
                types: nonzero_terminator.as_ptr(),
                count: nonzero_terminator.len() as u32,
            },
            NativeControllerInfo {
                types: std::ptr::null(),
                count: 0,
            },
        ];
        assert!(unsafe { capture_controller_choices(nonzero_ports.as_ptr()) }.is_err());
        assert!(unsafe { capture_controller_choices(std::ptr::null()) }.is_err());
    }

    #[test]
    fn psx_exe_is_reproducible_and_reloads_the_bios_vector() {
        let exe = psx_diagnostic_exe();
        assert_eq!(exe, psx_diagnostic_exe());
        assert_eq!(exe.len(), 0x1000);
        assert_eq!(&exe[..8], b"PS-X EXE");
        assert_eq!(
            u32::from_le_bytes(exe[0x10..0x14].try_into().unwrap()),
            0x8001_0000
        );
        assert_eq!(
            u32::from_le_bytes(exe[0x18..0x1c].try_into().unwrap()),
            0x8001_0000
        );
        assert_eq!(
            u32::from_le_bytes(exe[0x1c..0x20].try_into().unwrap()),
            0x800
        );
        assert_eq!(
            u32::from_le_bytes(exe[0x30..0x34].try_into().unwrap()),
            0x801f_ff00
        );

        let word = |index: usize| {
            u32::from_le_bytes(
                exe[0x800 + index * 4..0x804 + index * 4]
                    .try_into()
                    .unwrap(),
            )
        };
        let load_b0 = mips_i(0x09, 0, 10, 0x00b0);
        let call_t2 = mips_r(10, 0, 31, 0, 0x09);
        assert_eq!([word(8), word(13), word(18)], [load_b0; 3]);
        assert_eq!([word(10), word(14), word(19)], [call_t2; 3]);
        assert_eq!(word(9), mips_i(0x09, 0, 9, 0x0012));
        assert_eq!(word(12), mips_i(0x09, 0, 9, 0x0013));
        assert_eq!(word(16), mips_i(0x09, 0, 9, 0x005b));
        assert_eq!(PSX_PAD_BUFFER_OFFSETS, [0x20, 0x60]);
    }
    #[test]
    fn psx_expected_buffer_uses_bios_layout_and_beetle_axis_scaling() {
        assert_eq!(
            expected_dualshock_buffer(1 << 14, [0, 0, 16384, -16384]),
            [0x00, 0x73, 0xff, 0xbf, 0xc0, 0x40, 0x80, 0x80]
        );
    }
    #[test]
    fn gameboy_rom_samples_both_joyp_halves_with_original_header() {
        let rom = gameboy_diagnostic_rom();
        assert_eq!(rom, gameboy_diagnostic_rom());
        assert_eq!(rom.len(), 32768);
        assert_eq!(
            &rom[0x104..0x10c],
            &[0xce, 0xed, 0x66, 0x66, 0xcc, 0x0d, 0x00, 0x0b]
        );
        assert_eq!(&rom[0x100..0x104], &[0, 0xc3, 0x50, 1]);
        assert_eq!(&rom[0x15f..0x163], &[0x3e, 0x10, 0xe0, 0]);
        assert_eq!(&rom[0x16e..0x172], &[0x3e, 0x20, 0xe0, 0]);
        assert_eq!(&rom[0x17d..0x180], &[0xc3, 0x5f, 1]);
        assert_eq!(
            rom[0x14d],
            rom[0x134..0x14d]
                .iter()
                .fold(0u8, |sum, byte| sum.wrapping_sub(*byte).wrapping_sub(1))
        );
        let checksum = rom
            .iter()
            .enumerate()
            .filter(|(index, _)| ![0x14e, 0x14f].contains(index))
            .fold(0u16, |sum, (_, byte)| sum.wrapping_add(u16::from(*byte)));
        assert_eq!(
            u16::from_be_bytes(rom[0x14e..0x150].try_into().unwrap()),
            checksum
        );
        let (_, _, device, mask) = Diagnostic::Gameboy.identity();
        assert_eq!((device, mask), (1, 0x0f0f));
        let singles = &Diagnostic::Gameboy.cases()[1..9];
        assert_eq!(singles.len(), 8);
        let mut hardware_bits = 0;
        for &(_, _, hardware) in singles {
            assert_eq!(hardware.count_ones(), 1);
            assert_eq!(hardware_bits & hardware, 0);
            hardware_bits |= hardware;
        }
        assert_eq!(hardware_bits, mask);
    }

    #[test]
    fn atari2600_rom_and_register_mapping_are_reproducible() {
        let rom = atari2600_diagnostic_rom();
        assert_eq!(rom, atari2600_diagnostic_rom());
        assert_eq!(rom.len(), 4 * 1024);
        for vector in [0x0ffa, 0x0ffc, 0x0ffe] {
            assert_eq!(
                u16::from_le_bytes(rom[vector..vector + 2].try_into().unwrap()),
                0xf000
            );
        }
        assert_eq!(
            &rom[2..18],
            &[
                0xa9, b'L', 0x85, 0x84, 0xa9, b'B', 0x85, 0x85, 0xa9, b'2', 0x85, 0x86, 0xa9, b'6',
                0x85, 0x87
            ]
        );
        assert!(
            rom[..0x60]
                .windows(3)
                .any(|bytes| bytes == [0x4c, 0x12, 0xf0])
        );

        assert_eq!(
            atari2600_expected_registers([0, 0], 0xff),
            [0xff, 0x80, 0x80, 0xff]
        );
        assert_eq!(
            atari2600_expected_registers([(1 << 4) | (1 << 6) | 1, (1 << 5) | (1 << 7) | 1], 0x49),
            [0xa5, 0x00, 0x00, 0x49]
        );
        let (_, filename, device, mask) = Diagnostic::Atari2600.identity();
        assert_eq!((filename, device, mask), ("input.bin", 1, 0));
        assert!(Diagnostic::Atari2600.cases().is_empty());
    }

    #[test]
    fn gameboy_contracts_cover_every_catalog_core() {
        for (name, devices) in [
            ("Gambatte", &[1][..]),
            ("mGBA", &[1][..]),
            ("SameBoy", &[1, 257][..]),
            ("SkyEmu", &[1][..]),
            ("VBA-M", &[1][..]),
        ] {
            let contract = gameboy_core_contract(name).unwrap();
            assert_eq!(contract.device_modes, devices);
            assert_eq!(contract.supports_bitmask, name != "SkyEmu");
            assert_eq!(
                contract.system_ram_offset,
                usize::from(name == "SkyEmu") * 0xc000
            );
            assert_eq!(contract.source_revision.len(), 40);
            assert!(contract.source_url.contains(contract.source_revision));
        }
        assert!(gameboy_core_contract("unknown").is_err());
    }

    #[test]
    fn gba_contracts_cover_each_supported_core() {
        for (name, ram_bytes) in [
            ("mGBA", Some(32 * 1024)),
            ("VBA-M", Some(256 * 1024)),
            ("SkyEmu", None),
        ] {
            let contract = gba_core_contract(name).unwrap();
            assert_eq!(contract.supports_bitmask, name != "SkyEmu");
            match contract.observation_memory {
                GbaObservationMemory::SystemRam { bytes, offset } => {
                    assert_eq!(Some(bytes), ram_bytes);
                    assert_eq!(offset, 0);
                }
                GbaObservationMemory::MemoryMap(mapping) => {
                    assert_eq!(name, "SkyEmu");
                    assert_eq!(ram_bytes, None);
                    assert_eq!(mapping.address, 0x0200_0000);
                    assert_eq!(mapping.bytes, 256 * 1024);
                    assert_eq!(mapping.select, 0xff00_0000);
                }
            }
            assert_eq!(contract.source_revision.len(), 40);
            assert!(contract.source_url.contains(contract.source_revision));
        }
        assert!(gba_core_contract("unknown").is_err());
    }

    #[test]
    fn runtime_library_inventory_is_canonical_hashed_and_unique() {
        let directory = tempfile::tempdir().unwrap();
        let library = directory.path().join("dependency.so");
        std::fs::write(&library, b"trusted test dependency").unwrap();

        let inventory = runtime_library_inventory(std::slice::from_ref(&library)).unwrap();
        assert_eq!(inventory.len(), 1);
        assert_eq!(inventory[0].path, library.canonicalize().unwrap());
        assert_eq!(inventory[0].sha256, crate::file_hash(&library).unwrap());
        assert!(
            runtime_library_inventory(&[library.clone(), library.clone()])
                .unwrap_err()
                .to_string()
                .contains("Duplicate runtime library")
        );
        assert!(runtime_library_inventory(&[PathBuf::from("relative.so")]).is_err());
        assert!(runtime_library_inventory(&[directory.path().join("missing.so")]).is_err());
    }

    #[test]
    fn gamegear_rom_is_reproducible_with_valid_header_and_register_cases() {
        let rom = gamegear_diagnostic_rom();
        assert_eq!(rom, gamegear_diagnostic_rom());
        assert_eq!(rom.len(), 32768);
        assert_eq!(&rom[0x7ff0..0x7ff8], b"TMR SEGA");
        assert_eq!(rom[0x7fff], 0x6c);
        assert_eq!(
            u16::from_le_bytes(rom[0x7ffa..0x7ffc].try_into().unwrap()),
            rom[..0x7ff0].iter().map(|b| u16::from(*b)).sum::<u16>()
        );
        assert_eq!(&rom[..6], &[0xf3, 0xdb, 0xdc, 0x32, 0x00, 0xc0]);
        assert_eq!(&rom[23..26], &[0xc3, 0x01, 0x00]);
        let (_, _, device, mask) = Diagnostic::Gamegear.identity();
        assert_eq!(device, 769);
        assert_eq!(mask, 0x803f);
        for &(_, _, hardware) in Diagnostic::Gamegear.cases() {
            assert_eq!(hardware & !mask, 0);
        }
    }
    #[test]
    fn original_rom_has_valid_branch_literals_and_no_logo() {
        let rom = gba_diagnostic_rom();
        assert_eq!(rom, gba_diagnostic_rom());
        assert!(rom[4..0xa0].iter().all(|byte| *byte == 0));
        assert_eq!(rom[0xb2], 0x96);
        assert_eq!(
            rom[0xa0..=0xbd]
                .iter()
                .fold(0u8, |sum, b| sum.wrapping_add(*b)),
            0xe7
        );
        for (instruction, expected) in [
            (0xc0, 0x04000130u32),
            (0xc4, 0x02000000),
            (0xc8, 0x4c42494e),
        ] {
            let opcode = u32::from_le_bytes(rom[instruction..instruction + 4].try_into().unwrap());
            assert_eq!(opcode & 0xffff0fff, 0xe59f0018);
            let literal = instruction + 8 + (opcode & 0xfff) as usize;
            assert_eq!(
                u32::from_le_bytes(rom[literal..literal + 4].try_into().unwrap()),
                expected
            );
        }
    }

    #[test]
    fn nes_rom_is_reproducible_nrom_with_controller_loop_and_vectors() {
        let rom = nes_diagnostic_rom();
        assert_eq!(rom, nes_diagnostic_rom());
        assert_eq!(rom.len(), 16 + 16 * 1024 + 8 * 1024);
        assert_eq!(&rom[..8], &[b'N', b'E', b'S', 0x1a, 1, 1, 0, 0]);
        assert_eq!(&rom[16 + 25..16 + 30], &[0xa9, 1, 0x8d, 0x16, 0x40]);
        assert_eq!(&rom[16 + 39..16 + 45], &[0xad, 0x16, 0x40, 0x4a, 0x66, 7]);
        assert_eq!(&rom[16 + 48..16 + 51], &[0xd0, 0xf5, 0xa5]);
        assert_eq!(
            [
                &rom[16 + 39..16 + 41],
                &rom[16 + 62..16 + 64],
                &rom[16 + 85..16 + 87],
                &rom[16 + 108..16 + 110],
            ],
            [&[0xad, 0x16], &[0xad, 0x17], &[0xad, 0x16], &[0xad, 0x17],]
        );
        for branch in [48, 71, 94, 117] {
            assert_eq!(&rom[16 + branch..16 + branch + 2], &[0xd0, 0xf5]);
        }
        assert_eq!(
            &rom[16 + 125..16 + 141],
            &[
                0xa5, 7, 0x85, 0, 0xa5, 8, 0x85, 4, 0xa5, 9, 0x85, 5, 0xa5, 10, 0x85, 6
            ]
        );
        assert_eq!(&rom[16 + 141..16 + 144], &[0x4c, 0x19, 0x80]);
        for vector in [0x3ffa, 0x3ffc, 0x3ffe] {
            assert_eq!(
                u16::from_le_bytes(rom[16 + vector..16 + vector + 2].try_into().unwrap()),
                0x8000
            );
        }
        assert!(rom[16 + 144..16 + 0x3ffa].iter().all(|byte| *byte == 0));
        assert!(rom[16 + 16 * 1024..].iter().all(|byte| *byte == 0));
    }

    #[test]
    fn nes_cases_cover_each_hardware_bit_and_both_topologies() {
        let singles = &Diagnostic::Nes.cases()[1..9];
        let mut hardware_bits = 0u8;
        for &(_, retropad, hardware) in singles {
            assert_eq!(retropad.count_ones(), 1);
            assert_eq!(hardware.count_ones(), 1);
            assert_eq!(nes_hardware_bits(retropad), hardware as u8);
            assert_eq!(hardware_bits & hardware as u8, 0);
            hardware_bits |= hardware as u8;
        }
        assert_eq!(hardware_bits, 0xff);
        assert_eq!(NesTopology::TwoPlayer.connected_ports(), 2);
        assert_eq!(NesTopology::FourScore.connected_ports(), 4);
    }

    #[test]
    fn nes_core_contracts_use_exact_advertised_standard_devices() {
        let fceumm = nes_core_contract("FCEUmm").unwrap();
        assert_eq!(
            (fceumm.controller_device, fceumm.controller_label),
            (513, "Gamepad")
        );
        let mesen = nes_core_contract("Mesen").unwrap();
        assert_eq!(
            (mesen.controller_device, mesen.controller_label),
            (257, "Standard Controller")
        );
        assert!(nes_core_contract("Nestopia").is_err());
    }

    #[test]
    fn snes_rom_is_reproducible_lorom_with_serial_loops_and_valid_checksum() {
        let rom = snes_diagnostic_rom();
        assert_eq!(rom, snes_diagnostic_rom());
        assert_eq!(rom.len(), 32 * 1024);
        assert_eq!(&rom[0x7fc0..0x7fd5], b"LUNCHBOX SNES INPUT  ");
        assert_eq!(
            &rom[0x7fd5..0x7fdc],
            &[0x20, 0x00, 0x08, 0x00, 0x01, 0x33, 0x00]
        );
        for vector in (0x7fe4..0x8000).step_by(2) {
            assert_eq!(
                u16::from_le_bytes(rom[vector..vector + 2].try_into().unwrap()),
                0x8000
            );
        }
        let complement = u16::from_le_bytes(rom[0x7fdc..0x7fde].try_into().unwrap());
        let checksum = u16::from_le_bytes(rom[0x7fde..0x7fe0].try_into().unwrap());
        assert_eq!(complement ^ checksum, 0xffff);
        assert_eq!(
            rom.iter()
                .fold(0u16, |sum, byte| sum.wrapping_add(u16::from(*byte))),
            checksum
        );
        assert!(
            rom.windows(5)
                .any(|bytes| bytes == [0xa9, 1, 0x8d, 0x16, 0x40])
        );
        assert!(
            rom.windows(5)
                .any(|bytes| bytes == [0xa9, 0x80, 0x8d, 0x01, 0x42])
        );
        assert!(
            rom.windows(5)
                .any(|bytes| bytes == [0xa9, 0x00, 0x8d, 0x01, 0x42])
        );
        assert!(
            rom.windows(10)
                .any(|bytes| bytes == [0xad, 0x12, 0x42, 0x30, 0xfb, 0xad, 0x12, 0x42, 0x10, 0xfb])
        );

        let loops: Vec<usize> = rom
            .windows(4)
            .enumerate()
            .filter_map(|(offset, bytes)| (bytes == [0xe8, 0xe0, 0x10, 0xd0]).then_some(offset))
            .collect();
        assert_eq!(loops.len(), 2);
        for (index, &offset) in loops.iter().enumerate() {
            let branch = offset + 3;
            let target = (branch + 2) as isize + rom[branch + 1] as i8 as isize;
            assert_eq!(
                &rom[target as usize..target as usize + 3],
                if index == 0 {
                    &[0xad, 0x16, 0x40]
                } else {
                    &[0xad, 0x17, 0x40]
                }
            );
        }
    }

    #[test]
    fn snes_cases_cover_serial_word_and_contracts_are_explicit() {
        let singles = &Diagnostic::Snes.cases()[1..13];
        let mut hardware_bits = 0u16;
        for &(_, retropad, hardware) in singles {
            assert_eq!(retropad.count_ones(), 1);
            assert_eq!(hardware.count_ones(), 1);
            assert_eq!(retropad, hardware);
            assert_eq!(hardware_bits & hardware, 0);
            hardware_bits |= hardware;
        }
        assert_eq!(hardware_bits, 0x0fff);
        assert_eq!(SnesTopology::TwoPlayer.connected_ports(), 2);
        assert_eq!(SnesTopology::Multitap.connected_ports(), 5);

        let bsnes = snes_core_contract("bsnes").unwrap();
        assert!(!bsnes.supports_bitmask);
        assert!(matches!(bsnes.readback, SnesReadback::SerializedState));
        assert_eq!((bsnes.joypad_device, bsnes.multitap_device), (1, 257));
        let snes9x = snes_core_contract("Snes9x").unwrap();
        assert!(snes9x.supports_bitmask);
        assert!(matches!(snes9x.readback, SnesReadback::SystemRam));
        let mesen_s = snes_core_contract("Mesen-S").unwrap();
        assert!(!mesen_s.supports_bitmask);
        assert_eq!((mesen_s.joypad_device, mesen_s.multitap_device), (257, 513));
        assert!(snes_core_contract("bsnes-hd").is_err());
    }

    #[test]
    fn snes_wram_parser_requires_a_complete_generation() {
        let marker_offset = 123;
        let mut state = vec![0u8; marker_offset + SNES_MARKER.len() + 10];
        state[marker_offset - SNES_MARKER_OFFSET_DELTA] = 2;
        state[marker_offset..marker_offset + SNES_MARKER.len()].copy_from_slice(SNES_MARKER);
        for (port, value) in [0x0000u16, 0x0001, 0x0100, 0x0800, 0x0fff]
            .into_iter()
            .enumerate()
        {
            let offset = marker_offset + SNES_MARKER.len() + port * 2;
            state[offset..offset + 2].copy_from_slice(&value.to_le_bytes());
        }
        assert_eq!(
            parse_snes_wram(&state, marker_offset).unwrap(),
            [0x0000, 0x0001, 0x0100, 0x0800, 0x0fff]
        );
        state[marker_offset - SNES_MARKER_OFFSET_DELTA] = 3;
        assert!(parse_snes_wram(&state, marker_offset).is_err());
    }

    #[test]
    fn callback_is_port_scoped_and_preserves_mask_bits() {
        // Pure identity checks do not mutate the running diagnostic's globals.
        assert_eq!(unsafe { input(1, 1, 0, 0) }, 0);
        assert_eq!(unsafe { input(0, 5, 0, 0) }, 0);
        assert_eq!(unsafe { input(0, 1, 1, 0) }, 0);
        assert_eq!(unsafe { input(0, 1, 0, 17) }, 0);
    }
}
