//! Native SDL3 polling on a dedicated process main thread. No Qt event-loop or
//! GilRs numbering is shared with this runtime. Opening a controller may change
//! its lizard mode; SDL restores it when the process closes the gamepad.
use anyhow::{Context, Result, ensure};
use libloading::Library;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::ffi::{c_char, c_int, c_void};
use std::io::Write;
use std::path::Path;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Pad {
    pub instance: u32,
    pub name: String,
    pub path: Option<String>,
    pub serial: Option<String>,
    pub vendor: u16,
    pub product: u16,
    pub mapping: String,
    pub buttons: Vec<bool>,
    pub axes: Vec<i16>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Frame {
    pub version: i32,
    pub pads: Vec<Pad>,
}

pub fn is_sc2(vendor: u16, product: u16) -> bool {
    vendor == 0x28de && matches!(product, 0x1302..=0x1305)
}

#[repr(C)]
struct DisplayMode {
    display_id: u32,
    format: u32,
    w: c_int,
    h: c_int,
    pixel_density: f32,
    refresh_rate: f32,
    refresh_rate_numerator: c_int,
    refresh_rate_denominator: c_int,
    internal: *mut c_void,
}

/// Query the host's current primary display mode without creating a window.
/// This runs in a short-lived helper process because SDL's video API requires
/// its process main thread, while Lunchbox prepares launches on a worker.
pub fn primary_display_pixels(path: &Path) -> Result<(u32, u32)> {
    const SDL_INIT_VIDEO: u32 = 0x0000_0020;
    unsafe {
        let library = Library::new(path).context("Loading SDL3 display runtime")?;
        let set_main_ready = *library.get::<unsafe extern "C" fn()>(b"SDL_SetMainReady\0")?;
        let init = *library.get::<unsafe extern "C" fn(u32) -> bool>(b"SDL_InitSubSystem\0")?;
        let quit = *library.get::<unsafe extern "C" fn(u32)>(b"SDL_QuitSubSystem\0")?;
        let primary = *library.get::<unsafe extern "C" fn() -> u32>(b"SDL_GetPrimaryDisplay\0")?;
        let mode = *library.get::<unsafe extern "C" fn(u32) -> *const DisplayMode>(
            b"SDL_GetCurrentDisplayMode\0",
        )?;
        let error = *library.get::<unsafe extern "C" fn() -> *const c_char>(b"SDL_GetError\0")?;
        set_main_ready();
        ensure!(
            init(SDL_INIT_VIDEO),
            "SDL3 video initialization failed: {}",
            super::string(error())?.unwrap_or_default()
        );
        struct VideoGuard(unsafe extern "C" fn(u32));
        impl Drop for VideoGuard {
            fn drop(&mut self) {
                unsafe { (self.0)(SDL_INIT_VIDEO) }
            }
        }
        let _guard = VideoGuard(quit);
        let display = primary();
        ensure!(
            display != 0,
            "SDL3 found no primary display: {}",
            super::string(error())?.unwrap_or_default()
        );
        let mode = mode(display);
        ensure!(
            !mode.is_null(),
            "SDL3 found no current display mode: {}",
            super::string(error())?.unwrap_or_default()
        );
        let mode = &*mode;
        ensure!(
            mode.w > 0 && mode.h > 0 && mode.pixel_density.is_finite() && mode.pixel_density > 0.0,
            "SDL3 returned an invalid display mode"
        );
        let width = (f64::from(mode.w) * f64::from(mode.pixel_density)).round();
        let height = (f64::from(mode.h) * f64::from(mode.pixel_density)).round();
        ensure!(
            width > 0.0
                && height > 0.0
                && width <= f64::from(u32::MAX)
                && height <= f64::from(u32::MAX),
            "SDL3 display pixels are out of range"
        );
        Ok((width as u32, height as u32))
    }
}

/// The runtime owns every pointer and is used only on this process's main
/// thread. Drop ordering closes handles, quits SDL, then unloads the library.
pub struct Runtime {
    handles: BTreeMap<u32, *mut c_void>,
    close: unsafe extern "C" fn(*mut c_void),
    quit: unsafe extern "C" fn(u32),
    pump: unsafe extern "C" fn(),
    update: unsafe extern "C" fn(),
    enumerate: unsafe extern "C" fn(*mut c_int) -> *mut u32,
    free: super::Free,
    vendor: unsafe extern "C" fn(u32) -> u16,
    product: unsafe extern "C" fn(u32) -> u16,
    open: unsafe extern "C" fn(u32) -> *mut c_void,
    serial: unsafe extern "C" fn(*mut c_void) -> *const c_char,
    path: unsafe extern "C" fn(*mut c_void) -> *const c_char,
    mapping: unsafe extern "C" fn(*mut c_void) -> *mut c_char,
    button: unsafe extern "C" fn(*mut c_void, c_int) -> bool,
    axis: unsafe extern "C" fn(*mut c_void, c_int) -> i16,
    version: i32,
    _library: Library,
}

impl Runtime {
    pub fn open(path: &Path) -> Result<Self> {
        // All signatures below are the SDL3 public C ABI; SDL2 cannot satisfy
        // the gamepad-specific symbols. Resolve everything before initializing.
        unsafe {
            let library = Library::new(path).context("Loading SDL3 input runtime")?;
            let _: libloading::Symbol<unsafe extern "C" fn(*mut c_int) -> *mut u32> =
                library.get(b"SDL_GetGamepads\0")?;
            let version = library.get::<unsafe extern "C" fn() -> c_int>(b"SDL_GetVersion\0")?();
            ensure!(
                (3_004_012..4_000_000).contains(&version),
                "SDL3 3.4.12 or newer is required (found {version})"
            );
            let mut runtime = Self {
                handles: BTreeMap::new(),
                close: *library.get(b"SDL_CloseGamepad\0")?,
                quit: *library.get(b"SDL_QuitSubSystem\0")?,
                pump: *library.get(b"SDL_PumpEvents\0")?,
                update: *library.get(b"SDL_UpdateGamepads\0")?,
                enumerate: *library.get(b"SDL_GetGamepads\0")?,
                free: *library.get(b"SDL_free\0")?,
                vendor: *library.get(b"SDL_GetGamepadVendorForID\0")?,
                product: *library.get(b"SDL_GetGamepadProductForID\0")?,
                open: *library.get(b"SDL_OpenGamepad\0")?,
                serial: *library.get(b"SDL_GetGamepadSerial\0")?,
                path: *library.get(b"SDL_GetGamepadPath\0")?,
                mapping: *library.get(b"SDL_GetGamepadMapping\0")?,
                button: *library.get(b"SDL_GetGamepadButton\0")?,
                axis: *library.get(b"SDL_GetGamepadAxis\0")?,
                version,
                _library: library,
            };
            let hint = *runtime
                ._library
                .get::<unsafe extern "C" fn(*const c_char, *const c_char) -> bool>(
                    b"SDL_SetHint\0",
                )?;
            hint(c"SDL_JOYSTICK_HIDAPI_STEAM".as_ptr(), c"1".as_ptr());
            hint(
                c"SDL_JOYSTICK_ALLOW_BACKGROUND_EVENTS".as_ptr(),
                c"1".as_ptr(),
            );
            runtime
                ._library
                .get::<unsafe extern "C" fn()>(b"SDL_SetMainReady\0")?();
            let init = *runtime
                ._library
                .get::<unsafe extern "C" fn(u32) -> bool>(b"SDL_InitSubSystem\0")?;
            ensure!(init(super::SUBSYSTEMS), "SDL3 initialization failed");
            // Initialize enumeration before the first frame.
            runtime.sample()?;
            Ok(runtime)
        }
    }

    pub fn sample(&mut self) -> Result<Frame> {
        unsafe {
            (self.pump)();
            (self.update)();
            let mut count = 0;
            let ids = (self.enumerate)(&mut count);
            let allocation = super::Allocation {
                pointer: ids.cast(),
                free: self.free,
            };
            ensure!((0..=128).contains(&count), "Invalid SDL3 device count");
            ensure!(count == 0 || !ids.is_null(), "SDL3 enumeration failed");
            let ids = if count == 0 {
                Vec::new()
            } else {
                std::slice::from_raw_parts(ids, count as usize).to_vec()
            };
            drop(allocation);
            self.handles.retain(|id, handle| {
                if ids.contains(id) {
                    true
                } else {
                    (self.close)(*handle);
                    false
                }
            });
            let mut pads = Vec::new();
            for id in ids {
                let vendor = (self.vendor)(id);
                let product = (self.product)(id);
                if !is_sc2(vendor, product) {
                    continue;
                }
                let handle = *self.handles.entry(id).or_insert_with(|| (self.open)(id));
                if handle.is_null() {
                    self.handles.remove(&id);
                    anyhow::bail!(
                        "SDL3 found Steam Controller 2 but could not open it; check HID permissions or another application's exclusive access"
                    );
                }
                let raw_mapping = (self.mapping)(handle);
                let allocation = super::Allocation {
                    pointer: raw_mapping.cast(),
                    free: self.free,
                };
                let mapping = super::string(raw_mapping)?
                    .context("SDL3 did not resolve a gamepad mapping")?;
                drop(allocation);
                pads.push(Pad {
                    instance: id,
                    vendor,
                    product,
                    name: "Steam Controller 2 (2026)".into(),
                    path: super::string((self.path)(handle))?,
                    serial: super::string((self.serial)(handle))?.filter(|s| !s.trim().is_empty()),
                    mapping,
                    buttons: (0..26).map(|index| (self.button)(handle, index)).collect(),
                    axes: (0..6).map(|index| (self.axis)(handle, index)).collect(),
                });
            }
            Ok(Frame {
                version: self.version,
                pads,
            })
        }
    }
}

impl Drop for Runtime {
    fn drop(&mut self) {
        unsafe {
            for (_, handle) in std::mem::take(&mut self.handles) {
                if !handle.is_null() {
                    (self.close)(handle);
                }
            }
            (self.quit)(super::SUBSYSTEMS);
        }
    }
}

pub fn stream(path: &Path, once: bool) -> Result<()> {
    let stopped = std::sync::Arc::new(std::sync::atomic::AtomicBool::new(false));
    if !once {
        let stop = stopped.clone();
        std::thread::spawn(move || {
            use std::io::Read;
            let _ = std::io::stdin().read(&mut [0u8; 1]);
            stop.store(true, std::sync::atomic::Ordering::Release);
        });
    }
    let mut runtime = Runtime::open(path)?;
    let mut output = std::io::stdout().lock();
    loop {
        serde_json::to_writer(&mut output, &runtime.sample()?)?;
        writeln!(output)?;
        output.flush()?;
        if once || stopped.load(std::sync::atomic::Ordering::Acquire) {
            return Ok(());
        }
        std::thread::sleep(std::time::Duration::from_millis(16));
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn never_confuse_legacy_or_virtual_steam_devices_with_sc2() {
        for product in 0x1302..=0x1305 {
            assert!(is_sc2(0x28de, product));
        }
        for product in [0x1102, 0x1142, 0x1201, 0x11ff] {
            assert!(!is_sc2(0x28de, product));
        }
        assert!(!is_sc2(0x045e, 0x1302));
    }
}
