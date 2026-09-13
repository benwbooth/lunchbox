//! Real C-ABI logging callback shared by the bounded libretro oracles.

use std::ffi::{c_char, c_void};

type Log = unsafe extern "C" fn(i32, *const c_char, ...);

#[repr(C)]
struct LogCallback {
    log: Log,
}

unsafe extern "C" {
    fn lunchbox_retro_log(level: i32, format: *const c_char, ...);
}

/// Install the libretro logging interface into a validated, non-null command
/// payload. The core owns only the static function pointer, never Rust state.
///
/// # Safety
///
/// `data` must be either null or point to writable, properly aligned storage
/// for the libretro `retro_log_callback` structure supplied with environment
/// command 27.
pub unsafe fn install(data: *mut c_void) -> bool {
    if data.is_null() {
        return false;
    }
    unsafe {
        data.cast::<LogCallback>().write(LogCallback {
            log: lunchbox_retro_log,
        });
    }
    true
}
