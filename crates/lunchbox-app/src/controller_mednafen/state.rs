//! Read physical state without consuming events or changing calibration.
use anyhow::{Result, ensure};
use std::{
    collections::BTreeMap,
    os::{fd::AsRawFd, unix::fs::FileTypeExt},
};

pub(crate) fn capture(
    identity: &super::sysfs::Identity,
    map: &super::physical::Map,
) -> Result<BTreeMap<u32, i32>> {
    identity.verify()?;
    let path = identity.event_path()?;
    let file = std::fs::File::open(&path)?;
    ensure!(
        file.metadata()?.file_type().is_char_device(),
        "Mednafen event path is not a character device"
    );
    let mut keys = [0u8; 96];
    ensure!(
        unsafe {
            libc::ioctl(
                file.as_raw_fd(),
                0x80604518 as libc::c_ulong,
                keys.as_mut_ptr(),
            )
        } >= 0,
        "Cannot read Mednafen physical button state: {}",
        std::io::Error::last_os_error()
    );
    let mut result = BTreeMap::new();
    for code in &map.buttons {
        ensure!(*code < 768, "Invalid Mednafen physical button code");
        let pressed = keys[usize::from(*code) / 8] & (1 << (*code % 8)) != 0;
        result.insert((1 << 16) | u32::from(*code), i32::from(pressed));
    }
    for code in &map.axes {
        ensure!(*code < 64, "Invalid Mednafen physical axis code");
        let mut info = [0i32; 6];
        ensure!(
            unsafe {
                libc::ioctl(
                    file.as_raw_fd(),
                    (0x80184540u64 + u64::from(*code)) as libc::c_ulong,
                    info.as_mut_ptr(),
                )
            } >= 0,
            "Cannot read Mednafen physical axis state: {}",
            std::io::Error::last_os_error()
        );
        result.insert((3 << 16) | u32::from(*code), info[0]);
    }
    identity.verify()?;
    ensure!(
        identity.event_path()? == path,
        "Mednafen event association changed during capture"
    );
    Ok(result)
}
