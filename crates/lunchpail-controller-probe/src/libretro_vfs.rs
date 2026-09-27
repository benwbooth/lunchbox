//! Minimal read-only Libretro VFS metadata interface for direct-core oracles.
//!
//! Some cores require VFS v3 merely to classify an in-memory content path.
//! The oracle intentionally exposes only `stat`; native file reads and writes
//! stay unavailable, so content continues through the bounded Libretro buffer.

use std::ffi::{CStr, c_char, c_void};

#[repr(C)]
struct InterfaceInfo {
    required_interface_version: u32,
    iface: *const Interface,
}

#[repr(C)]
struct Interface {
    get_path: Option<unsafe extern "C" fn(*mut c_void) -> *const c_char>,
    open: Option<unsafe extern "C" fn(*const c_char, u32, u32) -> *mut c_void>,
    close: Option<unsafe extern "C" fn(*mut c_void) -> i32>,
    size: Option<unsafe extern "C" fn(*mut c_void) -> i64>,
    tell: Option<unsafe extern "C" fn(*mut c_void) -> i64>,
    seek: Option<unsafe extern "C" fn(*mut c_void, i64, i32) -> i64>,
    read: Option<unsafe extern "C" fn(*mut c_void, *mut c_void, u64) -> i64>,
    write: Option<unsafe extern "C" fn(*mut c_void, *const c_void, u64) -> i64>,
    flush: Option<unsafe extern "C" fn(*mut c_void) -> i32>,
    remove: Option<unsafe extern "C" fn(*const c_char) -> i32>,
    rename: Option<unsafe extern "C" fn(*const c_char, *const c_char) -> i32>,
    truncate: Option<unsafe extern "C" fn(*mut c_void, i64) -> i64>,
    stat: Option<unsafe extern "C" fn(*const c_char, *mut i32) -> i32>,
    mkdir: Option<unsafe extern "C" fn(*const c_char) -> i32>,
    opendir: Option<unsafe extern "C" fn(*const c_char, bool) -> *mut c_void>,
    readdir: Option<unsafe extern "C" fn(*mut c_void) -> bool>,
    dirent_get_name: Option<unsafe extern "C" fn(*mut c_void) -> *const c_char>,
    dirent_is_dir: Option<unsafe extern "C" fn(*mut c_void) -> bool>,
    closedir: Option<unsafe extern "C" fn(*mut c_void) -> i32>,
}

unsafe extern "C" fn stat(path: *const c_char, size: *mut i32) -> i32 {
    if path.is_null() {
        return 0;
    }
    let Ok(path) = unsafe { CStr::from_ptr(path) }.to_str() else {
        return 0;
    };
    let Ok(metadata) = std::fs::metadata(path) else {
        return 0;
    };
    if !size.is_null() {
        let Ok(bytes) = i32::try_from(metadata.len()) else {
            return 0;
        };
        unsafe { size.write(bytes) };
    }
    // RETRO_VFS_STAT_IS_VALID | RETRO_VFS_STAT_IS_DIRECTORY when applicable.
    1 | if metadata.is_dir() { 2 } else { 0 }
}

static INTERFACE: Interface = Interface {
    get_path: None,
    open: None,
    close: None,
    size: None,
    tell: None,
    seek: None,
    read: None,
    write: None,
    flush: None,
    remove: None,
    rename: None,
    truncate: None,
    stat: Some(stat),
    mkdir: None,
    opendir: None,
    readdir: None,
    dirent_get_name: None,
    dirent_is_dir: None,
    closedir: None,
};

/// Handle normalized environment command 45 (`GET_VFS_INTERFACE`).
///
/// Returns `None` for every other command and rejects null command-45 data.
///
/// # Safety
///
/// For command 45, non-null `data` must point to a writable, aligned
/// `retro_vfs_interface_info` supplied for the duration of the callback.
pub unsafe fn handle_environment(command: u32, data: *mut c_void) -> Option<bool> {
    if command != 45 {
        return None;
    }
    if data.is_null() {
        return Some(false);
    }
    let info = unsafe { &mut *data.cast::<InterfaceInfo>() };
    if info.required_interface_version > 3 {
        return Some(false);
    }
    info.required_interface_version = 3;
    info.iface = &INTERFACE;
    Some(true)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::ffi::CString;

    #[test]
    fn exposes_only_bounded_v3_metadata() {
        let mut info = InterfaceInfo {
            required_interface_version: 3,
            iface: std::ptr::null(),
        };
        assert_eq!(
            unsafe { handle_environment(45, (&mut info as *mut InterfaceInfo).cast()) },
            Some(true)
        );
        assert_eq!(info.required_interface_version, 3);
        let interface = unsafe { &*info.iface };
        assert!(interface.stat.is_some());
        assert!(interface.open.is_none());
        assert!(interface.read.is_none());
        assert!(interface.write.is_none());
        assert!(interface.remove.is_none());
    }

    #[test]
    fn stat_reports_regular_files_and_directories_without_opening_them() {
        let directory = tempfile::tempdir().unwrap();
        let file = directory.path().join("content.bin");
        std::fs::write(&file, b"LB26").unwrap();
        let file = CString::new(file.to_str().unwrap()).unwrap();
        let directory = CString::new(directory.path().to_str().unwrap()).unwrap();
        let mut size = -1;
        assert_eq!(unsafe { stat(file.as_ptr(), &mut size) }, 1);
        assert_eq!(size, 4);
        assert_eq!(unsafe { stat(directory.as_ptr(), std::ptr::null_mut()) }, 3);
        assert_eq!(unsafe { stat(std::ptr::null(), &mut size) }, 0);
    }

    #[test]
    fn rejects_unsupported_versions_and_null_requests() {
        let mut info = InterfaceInfo {
            required_interface_version: 4,
            iface: std::ptr::null(),
        };
        assert_eq!(
            unsafe { handle_environment(45, (&mut info as *mut InterfaceInfo).cast()) },
            Some(false)
        );
        assert!(info.iface.is_null());
        assert_eq!(
            unsafe { handle_environment(45, std::ptr::null_mut()) },
            Some(false)
        );
        assert_eq!(
            unsafe { handle_environment(44, std::ptr::null_mut()) },
            None
        );
    }
}
