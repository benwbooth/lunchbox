//! Real-core diagnostic, not a replacement libretro implementation. Run only in
//! a fresh helper process with an explicitly trusted core, never inside Qt.
use anyhow::{Context, Result, ensure};
use libloading::Library;
use serde::Serialize;
use std::collections::BTreeMap;
use std::ffi::{CStr, CString, c_char, c_void};
use std::path::Path;
use std::sync::{
    Mutex,
    atomic::{AtomicBool, AtomicI16, AtomicU16, AtomicU64, Ordering},
};

static ACTIVE: AtomicBool = AtomicBool::new(false);
static SYSTEM_DIRECTORY: Mutex<Option<CString>> = Mutex::new(None);
static SAVE_DIRECTORY: Mutex<Option<CString>> = Mutex::new(None);
static INSPECTION_OPTIONS: Mutex<Option<crate::libretro_options::OptionEnvironment>> =
    Mutex::new(None);
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

#[derive(Clone, Debug, Serialize, serde::Deserialize)]
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
            ensure!(
                !choice.description.is_null(),
                "Core controller choice has no label"
            );
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
static PRESSED: [AtomicU16; 2] = [AtomicU16::new(0), AtomicU16::new(0)];
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
        if let Some(options) = options.as_mut() {
            if let Some(result) = unsafe { options.handle(command, data) } {
                return result;
            }
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
        16 | 36 | 37 => true, // options/maps/geometry notifications
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
        _ => false,
    }
}
unsafe extern "C" fn input(port: u32, device: u32, index: u32, id: u32) -> i16 {
    if port >= 2 {
        return 0;
    }
    match (device, index, id) {
        (1, 0, 256) if BITMASK.load(Ordering::Relaxed) => {
            MASK_REQUESTS.fetch_add(1, Ordering::Relaxed);
            PRESSED[port as usize].load(Ordering::Relaxed) as i16
        }
        (1, 0, 0..=15) => {
            SINGLE_REQUESTS.fetch_add(1, Ordering::Relaxed);
            i16::from(PRESSED[port as usize].load(Ordering::Relaxed) & (1 << id) != 0)
        }
        // RETRO_DEVICE_ANALOG, left/right index, X/Y id.
        (5, 0..=1, 0..=1) => {
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
        ACTIVE.store(false, Ordering::Release);
    }
}
struct Core {
    _library: Library,
    deinit: unsafe extern "C" fn(),
    unload: unsafe extern "C" fn(),
    run: unsafe extern "C" fn(),
    memory: unsafe extern "C" fn(u32) -> *mut c_void,
    memory_size: unsafe extern "C" fn(u32) -> usize,
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
/// buttons at C000 and directions at C001. SameBoy's built-in open boot ROM
/// accepts our blank logo area; no Nintendo logo or firmware is embedded.
pub fn gameboy_diagnostic_rom() -> Vec<u8> {
    let mut rom = vec![0; 0x8000];
    rom[0x100..0x104].copy_from_slice(&[0x00, 0xc3, 0x50, 0x01]); // nop; jp 0150
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
/// <https://github.com/libretro/beetle-psx-libretro/tree/56f4732070835bb81078dd8ecab7246e203612a1>
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
    Psx,
}

impl Diagnostic {
    fn identity(self) -> (&'static str, &'static str, u32, u16) {
        match self {
            Self::Gba => ("mGBA", "input.gba", 1, 0x3ff),
            Self::Gamegear => ("Genesis Plus GX", "input.gg", 769, 0x803f),
            Self::Gameboy => ("SameBoy", "input.gb", 257, 0x0f0f),
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
    pub reported_system_ram_bytes: usize,
    pub observations: Vec<Observation>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub psx_observations: Vec<PsxObservation>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub firmware: Vec<FirmwareIdentity>,
    #[serde(skip_serializing_if = "Option::is_none")]
    /// Reference used to define expectations, not a provenance assertion about
    /// an arbitrary caller-supplied core binary. Its actual hash/version above
    /// remain the runtime evidence.
    pub contract_source_revision: Option<&'static str>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub contract_source_url: Option<&'static str>,
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
        callback!(b"retro_set_video_refresh\0", Video, video);
        callback!(b"retro_set_audio_sample\0", Audio, audio);
        callback!(b"retro_set_audio_sample_batch\0", AudioBatch, audio_batch);
        callback!(b"retro_set_input_poll\0", Poll, poll);
        callback!(b"retro_set_input_state\0", Input, neutral_input);
        init();
        core.initialized = true;
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
    if let Ok(mut queries) = INSPECTION_QUERIES.lock() {
        if queries.failure.is_none() {
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
    let (expected_core, filename, device, register_mask) = diagnostic.identity();
    let path = path.canonicalize()?;
    let hash = crate::file_hash(&path)?;
    ensure!(
        hash.eq_ignore_ascii_case(expected_sha256),
        "Core SHA256 mismatch"
    );
    ensure!(
        ACTIVE
            .compare_exchange(false, true, Ordering::Acquire, Ordering::Relaxed)
            .is_ok(),
        "Only one core diagnostic may run in this process"
    );
    let _lease = Lease;
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
    BITMASK.store(bitmask, Ordering::Relaxed);
    // These buffers must outlive unload_game, including every error path.
    let rom = match diagnostic {
        Diagnostic::Gba => gba_diagnostic_rom(),
        Diagnostic::Gamegear => gamegear_diagnostic_rom(),
        Diagnostic::Gameboy => gameboy_diagnostic_rom(),
        Diagnostic::Psx => psx_diagnostic_exe(),
    };
    let rom_path = CString::new(
        directory
            .path()
            .join(filename)
            .to_str()
            .context("UTF-8 ROM path required")?,
    )?;
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
        ensure!(
            name == expected_core
                || (matches!(diagnostic, Diagnostic::Psx) && name == "Beetle PSX HW"),
            "Expected {expected_core}, got {name}"
        );
        if info.need_fullpath {
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
        callback!(b"retro_set_video_refresh\0", Video, video);
        callback!(b"retro_set_audio_sample\0", Audio, audio);
        callback!(b"retro_set_audio_sample_batch\0", AudioBatch, audio_batch);
        callback!(b"retro_set_input_poll\0", Poll, poll);
        callback!(b"retro_set_input_state\0", Input, input);
        init();
        core.initialized = true;
        set_device(0, device);
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
        set_device(0, device);
        set_device(
            1,
            if matches!(diagnostic, Diagnostic::Psx) {
                device
            } else {
                0
            },
        );
        let reported_system_ram_bytes = (core.memory_size)(2);
        // This pinned mGBA frontend reports the GB RAM size even for GBA.
        // Read only our eight diagnostic bytes, within its reported bounds;
        // never infer that the full 256 KiB hardware RAM is exposed by this API.
        if matches!(diagnostic, Diagnostic::Psx) {
            ensure!(
                reported_system_ram_bytes == 2 * 1024 * 1024,
                "Unexpected exposed PSX RAM size: {reported_system_ram_bytes}"
            );
        } else {
            ensure!(
                (8..=256 * 1024).contains(&reported_system_ram_bytes),
                "Unexpected exposed system RAM size: {reported_system_ram_bytes}"
            );
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
                reported_system_ram_bytes,
                observations: Vec::new(),
                psx_observations,
                firmware,
                contract_source_revision: Some("56f4732070835bb81078dd8ecab7246e203612a1"),
                contract_source_url: Some(
                    "https://github.com/libretro/beetle-psx-libretro/tree/56f4732070835bb81078dd8ecab7246e203612a1",
                ),
            });
        }
        if matches!(diagnostic, Diagnostic::Gameboy) {
            // Allow the real built-in open boot ROM to finish. Do not patch
            // CPU state or substitute memory for an executed diagnostic.
            let mut booted = false;
            for _ in 0..240 {
                (core.run)();
                ensure!(
                    (core.memory_size)(2) >= 8,
                    "Game Boy RAM unavailable during boot"
                );
                let memory = (core.memory)(2).cast::<u8>();
                ensure!(!memory.is_null(), "Game Boy RAM pointer unavailable");
                let bytes = std::slice::from_raw_parts(memory, 8);
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
        let device_modes = if matches!(diagnostic, Diagnostic::Gameboy) {
            vec![1, 257]
        } else {
            vec![device]
        };
        for test_device in device_modes {
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
                    ensure!(
                        (core.memory_size)(2) >= 8,
                        "Diagnostic memory became unavailable"
                    );
                    let memory = (core.memory)(2).cast::<u8>();
                    ensure!(!memory.is_null(), "No system RAM exposed by core");
                    let bytes = std::slice::from_raw_parts(memory, 8);
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
            schema_version: 1,
            diagnostic: match diagnostic {
                Diagnostic::Gba => "gba-keyinput",
                Diagnostic::Gamegear => "gamegear-dc-00",
                Diagnostic::Gameboy => "gameboy-joyp",
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
            reported_system_ram_bytes,
            observations,
            psx_observations: Vec::new(),
            firmware,
            contract_source_revision: matches!(diagnostic, Diagnostic::Gameboy).then_some("8230189896a8bb6598574d302ba0ad3658f98ab4"),
            contract_source_url: matches!(diagnostic, Diagnostic::Gameboy).then_some("https://github.com/LIJI32/SameBoy/blob/8230189896a8bb6598574d302ba0ad3658f98ab4/libretro/libretro.c"),
        })
    }
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
        assert!(rom[0x104..0x134].iter().all(|b| *b == 0));
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
        assert_eq!((device, mask), (257, 0x0f0f));
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
    fn callback_is_port_scoped_and_preserves_mask_bits() {
        // Pure identity checks do not mutate the running diagnostic's globals.
        assert_eq!(unsafe { input(1, 1, 0, 0) }, 0);
        assert_eq!(unsafe { input(0, 5, 0, 0) }, 0);
        assert_eq!(unsafe { input(0, 1, 1, 0) }, 0);
        assert_eq!(unsafe { input(0, 1, 0, 17) }, 0);
    }
}
