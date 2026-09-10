//! Explicit launch-time libudev enumeration, in the order used by Dolphin.
use super::{Node, qualify};
use crate::controller_dolphin::standalone::evdev::{DeviceObservation, capture_node};
use anyhow::{Context, Result, ensure};
use std::{
    ffi::{CStr, c_char, c_void},
    os::{fd::AsRawFd, unix::ffi::OsStrExt},
    path::PathBuf,
};

#[link(name = "udev")]
unsafe extern "C" {
    fn udev_new() -> *mut c_void;
    fn udev_unref(value: *mut c_void) -> *mut c_void;
    fn udev_enumerate_new(value: *mut c_void) -> *mut c_void;
    fn udev_enumerate_unref(value: *mut c_void) -> *mut c_void;
    fn udev_enumerate_add_match_subsystem(value: *mut c_void, subsystem: *const c_char) -> i32;
    fn udev_enumerate_scan_devices(value: *mut c_void) -> i32;
    fn udev_enumerate_get_list_entry(value: *mut c_void) -> *mut c_void;
    fn udev_list_entry_get_next(value: *mut c_void) -> *mut c_void;
    fn udev_list_entry_get_name(value: *mut c_void) -> *const c_char;
    fn udev_device_new_from_syspath(udev: *mut c_void, path: *const c_char) -> *mut c_void;
    fn udev_device_unref(value: *mut c_void) -> *mut c_void;
    fn udev_device_get_devnode(value: *mut c_void) -> *const c_char;
}

struct Owned(
    *mut c_void,
    unsafe extern "C" fn(*mut c_void) -> *mut c_void,
);
impl Drop for Owned {
    fn drop(&mut self) {
        unsafe {
            (self.1)(self.0);
        }
    }
}

fn identity(file: &std::fs::File, number: u64) -> Result<Option<String>> {
    let mut bytes = [0_u8; 1024];
    let count = unsafe {
        libc::ioctl(
            file.as_raw_fd(),
            (0x84004500 | number) as libc::c_ulong,
            bytes.as_mut_ptr(),
        )
    };
    if count < 0 {
        let error = std::io::Error::last_os_error();
        if error.raw_os_error() == Some(libc::ENOENT) {
            return Ok(None);
        }
        return Err(error.into());
    }
    ensure!(
        (count as usize) < bytes.len(),
        "Dolphin evdev identity exceeds limit"
    );
    let end = bytes
        .iter()
        .position(|byte| *byte == 0)
        .context("Unterminated evdev identity")?;
    Ok(Some(std::str::from_utf8(&bytes[..end])?.to_owned()))
}

pub(crate) fn capture() -> Result<Vec<DeviceObservation>> {
    let context = unsafe { udev_new() };
    ensure!(
        !context.is_null(),
        "Creating Dolphin udev inventory context failed"
    );
    let context = Owned(context, udev_unref);
    let enumeration = unsafe { udev_enumerate_new(context.0) };
    ensure!(
        !enumeration.is_null(),
        "Creating Dolphin udev enumeration failed"
    );
    let enumeration = Owned(enumeration, udev_enumerate_unref);
    ensure!(
        unsafe { udev_enumerate_add_match_subsystem(enumeration.0, c"input".as_ptr()) } >= 0
            && unsafe { udev_enumerate_scan_devices(enumeration.0) } >= 0,
        "Enumerating Dolphin input devices failed"
    );
    let mut entry = unsafe { udev_enumerate_get_list_entry(enumeration.0) };
    let mut nodes = Vec::new();
    let mut visited = 0;
    while !entry.is_null() {
        visited += 1;
        ensure!(visited <= 4096, "Dolphin udev enumeration exceeds limit");
        let syspath = unsafe { udev_list_entry_get_name(entry) };
        ensure!(!syspath.is_null(), "Dolphin udev entry has no path");
        let device = unsafe { udev_device_new_from_syspath(context.0, syspath) };
        ensure!(
            !device.is_null(),
            "Dolphin input disappeared during enumeration"
        );
        let device = Owned(device, udev_device_unref);
        let path = unsafe { udev_device_get_devnode(device.0) };
        if !path.is_null() {
            let path_c = unsafe { CStr::from_ptr(path) };
            let path_buf = PathBuf::from(std::ffi::OsStr::from_bytes(path_c.to_bytes()));
            let is_event = path_buf
                .file_name()
                .and_then(|name| name.to_str())
                .and_then(|name| name.strip_prefix("event"))
                .is_some_and(|suffix| {
                    !suffix.is_empty() && suffix.bytes().all(|byte| byte.is_ascii_digit())
                });
            // Dolphin opens O_RDWR. Check effective permissions without opening
            // writable or issuing force-feedback writes during our capture.
            if is_event
                && unsafe {
                    libc::faccessat(
                        libc::AT_FDCWD,
                        path_c.as_ptr(),
                        libc::R_OK | libc::W_OK,
                        libc::AT_EACCESS,
                    )
                } == 0
            {
                let file = std::fs::File::open(&path_buf)?;
                let name = identity(&file, 0x06)?.context("Dolphin evdev device has no name")?;
                let physical_location = identity(&file, 0x07)?;
                let unique_id = identity(&file, 0x08)?;
                let observation = capture_node(&path_buf)?;
                use std::os::unix::fs::MetadataExt;
                let opened = file.metadata()?;
                let current = std::fs::metadata(&path_buf)?;
                ensure!(
                    opened.dev() == current.dev()
                        && opened.ino() == current.ino()
                        && opened.rdev() == current.rdev(),
                    "Dolphin input identity changed during capture"
                );
                nodes.push(Node {
                    name,
                    physical_location,
                    unique_id,
                    observation,
                });
            }
        }
        entry = unsafe { udev_list_entry_get_next(entry) };
    }
    qualify(nodes)
}
