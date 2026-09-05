//! Real-core diagnostic, not a replacement libretro implementation. Run only in
//! a fresh helper process with an explicitly trusted core, never inside Qt.
use anyhow::{Context, Result, ensure};
use libloading::Library;
use serde::Serialize;
use std::ffi::{CStr, CString, c_char, c_void};
use std::path::Path;
use std::sync::{
    Mutex,
    atomic::{AtomicBool, AtomicU16, AtomicU64, Ordering},
};

static ACTIVE: AtomicBool = AtomicBool::new(false);
static DIRECTORY: Mutex<Option<CString>> = Mutex::new(None);
static PRESSED: AtomicU16 = AtomicU16::new(0);
static BITMASK: AtomicBool = AtomicBool::new(false);
static POLLS: AtomicU64 = AtomicU64::new(0);
static MASK_REQUESTS: AtomicU64 = AtomicU64::new(0);
static SINGLE_REQUESTS: AtomicU64 = AtomicU64::new(0);

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
            let Ok(directory) = DIRECTORY.lock() else {
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
        11 | 16 | 35 | 36 | 37 => true, // descriptors/options/maps/geometry notifications
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
    if port != 0 || device != 1 || index != 0 {
        return 0;
    }
    let pressed = PRESSED.load(Ordering::Relaxed);
    if id == 256 && BITMASK.load(Ordering::Relaxed) {
        MASK_REQUESTS.fetch_add(1, Ordering::Relaxed);
        return pressed as i16;
    }
    if id < 16 {
        SINGLE_REQUESTS.fetch_add(1, Ordering::Relaxed);
        i16::from(pressed & (1 << id) != 0)
    } else {
        0
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
        if let Ok(mut directory) = DIRECTORY.lock() {
            *directory = None;
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

#[derive(Debug, Serialize)]
pub struct Observation {
    pub name: String,
    pub retropad_mask: u16,
    pub expected_keyinput: u16,
    pub observed_keyinput: u16,
}
#[derive(Debug, Serialize)]
pub struct Report {
    pub schema_version: u32,
    pub core_sha256: String,
    pub core_name: String,
    pub core_version: String,
    pub input_mode: &'static str,
    pub input_polls: u64,
    pub bitmask_requests: u64,
    pub individual_requests: u64,
    pub reported_system_ram_bytes: usize,
    pub observations: Vec<Observation>,
}

/// Loads native executable code. Caller must trust the supplied core. This
/// diagnostic uses synthetic frontend input, not OS controller enumeration or
/// the Lunchbox-to-RetroArch configuration layer. The CLI bounds wall time.
pub fn inspect_mgba(path: &Path, expected_sha256: &str, bitmask: bool) -> Result<Report> {
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
    *DIRECTORY
        .lock()
        .map_err(|_| anyhow::anyhow!("Poisoned diagnostic context"))? = Some(CString::new(
        directory
            .path()
            .to_str()
            .context("UTF-8 diagnostic directory required")?,
    )?);
    PRESSED.store(0, Ordering::Relaxed);
    POLLS.store(0, Ordering::Relaxed);
    MASK_REQUESTS.store(0, Ordering::Relaxed);
    SINGLE_REQUESTS.store(0, Ordering::Relaxed);
    BITMASK.store(bitmask, Ordering::Relaxed);
    // These buffers must outlive unload_game, including every error path.
    let rom = gba_diagnostic_rom();
    let rom_path = CString::new(
        directory
            .path()
            .join("input.gba")
            .to_str()
            .context("UTF-8 ROM path required")?,
    )?;
    unsafe {
        let library = Library::new(&path).context("Loading trusted mGBA core")?;
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
            name == "mGBA" && !info.need_fullpath,
            "Expected an in-memory mGBA core"
        );
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
        set_device(0, 1);
        ensure!(
            load(&GameInfo {
                path: rom_path.as_ptr(),
                data: rom.as_ptr().cast(),
                size: rom.len(),
                meta: std::ptr::null()
            }),
            "Core rejected original diagnostic ROM"
        );
        core.loaded = true;
        let reported_system_ram_bytes = (core.memory_size)(2);
        // This pinned mGBA frontend reports the GB RAM size even for GBA.
        // Read only our eight diagnostic bytes, within its reported bounds;
        // never infer that the full 256 KiB hardware RAM is exposed by this API.
        ensure!(
            (8..=256 * 1024).contains(&reported_system_ram_bytes),
            "Unexpected exposed system RAM size: {reported_system_ram_bytes}"
        );
        let mut observations = Vec::new();
        // GBA KEYINPUT bit order is hardware order, separate from RetroPad IDs.
        for (name, rp, gba) in [
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
        ] {
            for (step_name, mask, expected) in [(name, rp, 0x3ff ^ gba), ("release", 0, 0x3ff)] {
                PRESSED.store(mask, Ordering::Relaxed);
                for _ in 0..4 {
                    (core.run)();
                }
                ensure!(
                    (core.memory_size)(2) >= 8,
                    "Diagnostic memory became unavailable"
                );
                let memory = (core.memory)(2).cast::<u8>();
                ensure!(!memory.is_null(), "No EWRAM exposed by core");
                let bytes = std::slice::from_raw_parts(memory, 8);
                ensure!(
                    u32::from_le_bytes(bytes[4..8].try_into().unwrap()) == 0x4c42494e,
                    "Diagnostic program did not execute"
                );
                let observed = u16::from_le_bytes([bytes[0], bytes[1]]) & 0x3ff;
                ensure!(
                    observed == expected,
                    "{step_name}: expected KEYINPUT {expected:#05x}, observed {observed:#05x}"
                );
                observations.push(Observation {
                    name: step_name.into(),
                    retropad_mask: mask,
                    expected_keyinput: expected,
                    observed_keyinput: observed,
                });
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
        Ok(Report {
            schema_version: 1,
            core_sha256: hash,
            core_name: name,
            core_version: revision,
            input_mode: if bitmask { "bitmask" } else { "individual" },
            input_polls: POLLS.load(Ordering::Relaxed),
            bitmask_requests,
            individual_requests,
            reported_system_ram_bytes,
            observations,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
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
