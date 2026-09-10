//! Raw evdev capability catalog for frontends that read /dev/input/event*
//! directly (for example bsnes's udev input driver). Read-only: this module
//! never opens devices for writing and never injects events.
//!
//! The dump is generic kernel truth: capability bitmaps in code order, each
//! absolute axis's kernel absinfo, and the sysfs identity chain. It does not
//! interpret a frontend's device-id scheme or control ordering; consumers own
//! those contracts.
use anyhow::{Context, Result, ensure};
use serde::Serialize;
use std::{
    fs::File,
    io::Read,
    os::{fd::AsRawFd, unix::fs::FileTypeExt},
    path::{Path, PathBuf},
};

/// BTN_MISC from the kernel UAPI; button codes below the joystick range.
const BTN_MISC: u16 = 0x100;
/// BTN_JOYSTICK from the kernel UAPI; start of the first scanned range.
const BTN_JOYSTICK: u16 = 0x120;
/// KEY_MAX from the kernel UAPI; the first scan range stops before it.
const KEY_MAX: u16 = 0x2ff;
/// ABS_MISC from the kernel UAPI; absolute axes at or above it are ignored.
const ABS_MISC: u16 = 0x28;
/// ABS_HAT0X..=ABS_HAT3Y; the hat window inside the scanned axis range.
const ABS_HAT_WINDOW: std::ops::RangeInclusive<u16> = 0x10..=0x1b;

// Linux input UAPI ioctl requests, byte-for-byte as defined by evdev's ABI:
// EVIOCGBIT(ev, len) = _IOR('E', 0x20 + ev, len) and
// EVIOCGABS(ev) = _IOR('E', 0x40 + ev, sizeof(struct input_absinfo)).
const EVIOCGBIT_EV: libc::c_ulong = 0x8000_4520;
const EVIOCGBIT_KEY: libc::c_ulong = 0x8000_4521;
const EVIOCGBIT_ABS: libc::c_ulong = 0x8000_4523;
const EVIOCGABS: libc::c_ulong = 0x8018_4540;
const EV_KEY: u16 = 0x01;
const KEY_CNT: usize = 0x300;
const ABS_CNT: usize = 0x40;

#[derive(Debug, Clone, Serialize, PartialEq)]
pub struct AbsInfo {
    pub value: i32,
    pub minimum: i32,
    pub maximum: i32,
    pub fuzz: i32,
    pub flat: i32,
    pub resolution: i32,
}

#[derive(Debug, Clone, Serialize, PartialEq)]
pub struct UsbRoot {
    /// udev devpath of the enclosing usb_device, without a /sys prefix.
    pub devpath: String,
    pub id_vendor: String,
    pub id_product: String,
    pub manufacturer: Option<String>,
    pub product: Option<String>,
    pub serial: Option<String>,
}

#[derive(Debug, Clone, Serialize, PartialEq)]
pub struct SysfsIdentity {
    /// /sys path of the enclosing inputN device.
    pub input: PathBuf,
    pub bustype: String,
    pub vendor: String,
    pub product: String,
    pub version: String,
    pub usb_root: Option<UsbRoot>,
}

/// One /dev/input/event node's raw kernel view, in kernel code order.
#[derive(Debug, Clone, Serialize, PartialEq)]
pub struct EvdevDevice {
    /// The exact node that was read, as an absolute /dev/input path.
    pub event: PathBuf,
    /// Kernel button codes: [BTN_JOYSTICK..KEY_MAX) first, then
    /// [BTN_MISC..BTN_JOYSTICK), matching bsnes's udev scan order.
    pub buttons: Vec<u16>,
    /// Non-hat absolute axis codes below ABS_MISC, ascending.
    pub axes: Vec<AbsAxis>,
    /// Hat-window axis codes (ABS_HAT0X..=ABS_HAT3Y), ascending. Each axis
    /// code is a separate entry; frontends pair them.
    pub hats: Vec<AbsAxis>,
    pub identity: SysfsIdentity,
}

#[derive(Debug, Clone, Serialize, PartialEq)]
pub struct AbsAxis {
    pub code: u16,
    pub info: AbsInfo,
}

#[derive(Debug, Clone, Serialize, PartialEq)]
pub struct EvdevCatalog {
    pub devices: Vec<EvdevDevice>,
}

impl EvdevCatalog {
    pub fn device_at_event(&self, event: &Path) -> Result<&EvdevDevice> {
        let device = self
            .devices
            .iter()
            .find(|device| device.event == event)
            .with_context(|| format!("Evdev catalog has no {}", event.display()))?;
        ensure!(
            self.devices
                .iter()
                .filter(|candidate| candidate.event == event)
                .count()
                == 1,
            "Evdev catalog lists {} twice",
            event.display()
        );
        Ok(device)
    }
}

fn ioctl_bits(fd: &File, request: libc::c_ulong, buffer: &mut [u8]) -> Result<()> {
    let result = unsafe {
        libc::ioctl(
            fd.as_raw_fd(),
            request,
            buffer.as_mut_ptr().cast::<libc::c_void>(),
        )
    };
    ensure!(
        result >= 0,
        "Reading evdev capabilities: {}",
        std::io::Error::last_os_error()
    );
    ensure!(
        usize::try_from(result)? <= buffer.len(),
        "Evdev capability report exceeds the request size"
    );
    Ok(())
}

fn ioctl_absinfo(fd: &File, axis: u16) -> Result<AbsInfo> {
    let mut raw = [0i32; 6];
    let result = unsafe {
        libc::ioctl(
            fd.as_raw_fd(),
            EVIOCGABS + libc::c_ulong::from(axis),
            raw.as_mut_ptr().cast::<libc::c_void>(),
        )
    };
    ensure!(
        result >= 0,
        "Reading evdev axis {axis} bounds: {}",
        std::io::Error::last_os_error()
    );
    let [value, minimum, maximum, fuzz, flat, resolution] = raw;
    Ok(AbsInfo {
        value,
        minimum,
        maximum,
        fuzz,
        flat,
        resolution,
    })
}

fn test_bit(buffer: &[u8], bit: u16) -> bool {
    buffer
        .get(usize::from(bit >> 3))
        .is_some_and(|byte| byte & (1 << (bit & 7)) != 0)
}

/// Read one event node. `fd` must refer to the same open file throughout, so
/// capabilities and identity cannot be mixed from two nodes.
pub fn read_event(event: &Path) -> Result<EvdevDevice> {
    ensure!(
        event.parent() == Some(Path::new("/dev/input"))
            && event
                .file_name()
                .and_then(|name| name.to_str())
                .is_some_and(|name| {
                    name.strip_prefix("event").is_some_and(|suffix| {
                        !suffix.is_empty() && suffix.bytes().all(|b| b.is_ascii_digit())
                    })
                }),
        "Expected an absolute /dev/input/eventN path"
    );
    let file = File::open(event).context("Opening evdev node read-only")?;
    ensure!(
        file.metadata()?.file_type().is_char_device(),
        "Evdev path is not a character device"
    );
    let mut evbit = [0u8; 4];
    ioctl_bits(&file, EVIOCGBIT_EV, &mut evbit)?;
    ensure!(test_bit(&evbit, EV_KEY), "Evdev node reports no keys");
    let mut keybit = [0u8; KEY_CNT / 8];
    ioctl_bits(&file, EVIOCGBIT_KEY, &mut keybit)?;
    let mut absbit = [0u8; ABS_CNT / 8];
    ioctl_bits(&file, EVIOCGBIT_ABS, &mut absbit)?;
    // Re-read the primary capability words; a concurrent node swap between
    // the two reads must not blend two devices into one catalog entry.
    let mut verify_evbit = [0u8; 4];
    ioctl_bits(&file, EVIOCGBIT_EV, &mut verify_evbit)?;
    let mut verify_keybit = [0u8; KEY_CNT / 8];
    ioctl_bits(&file, EVIOCGBIT_KEY, &mut verify_keybit)?;
    ensure!(
        evbit == verify_evbit && keybit == verify_keybit,
        "Evdev capabilities changed while reading"
    );

    let mut buttons = Vec::new();
    for code in (BTN_JOYSTICK..KEY_MAX).chain(BTN_MISC..BTN_JOYSTICK) {
        if test_bit(&keybit, code) {
            buttons.push(code);
        }
    }
    let mut axes = Vec::new();
    let mut hats = Vec::new();
    for code in 0..ABS_MISC {
        if !test_bit(&absbit, code) {
            continue;
        }
        let axis = AbsAxis {
            code,
            info: ioctl_absinfo(&file, code)?,
        };
        if ABS_HAT_WINDOW.contains(&code) {
            hats.push(axis);
        } else {
            axes.push(axis);
        }
    }
    let identity = sysfs_identity(event)?;
    Ok(EvdevDevice {
        event: event.to_path_buf(),
        buttons,
        axes,
        hats,
        identity,
    })
}

fn read_small_text(path: &Path) -> Result<String> {
    let mut text = String::new();
    File::open(path)?.take(1025).read_to_string(&mut text)?;
    ensure!(
        text.lines().count() <= 1 && text.len() <= 1024,
        "Evdev sysfs attribute exceeds the one-line size limit"
    );
    Ok(text.trim().to_owned())
}

fn input_sysfs_root(event: &Path) -> Result<PathBuf> {
    let name = event
        .file_name()
        .and_then(|name| name.to_str())
        .context("Evdev node name is absent")?;
    for base in ["/sys/subsystem/input", "/sys/bus/input", "/sys/class/input"] {
        let candidate = Path::new(base).join(name);
        if let Ok(resolved) = candidate.canonicalize() {
            let input = resolved
                .ancestors()
                .find(|path| {
                    path.file_name()
                        .and_then(|name| name.to_str())
                        .and_then(|name| name.strip_prefix("input"))
                        .is_some_and(|suffix| {
                            !suffix.is_empty() && suffix.bytes().all(|b| b.is_ascii_digit())
                        })
                })
                .with_context(|| format!("{} has no enclosing inputN device", event.display()))?;
            return Ok(input.to_path_buf());
        }
    }
    anyhow::bail!("Evdev sysfs input subsystem is unavailable")
}

/// Mirror udev's parent walk: the enclosing usb_device, if one exists. Only
/// the raw sysfs view is reported; matching input ids against the USB root is
/// the consumer's contract decision.
fn sysfs_identity(event: &Path) -> Result<SysfsIdentity> {
    let input = input_sysfs_root(event)?;
    let attribute = |name: &str| read_small_text(&input.join("id").join(name));
    let bustype = attribute("bustype")?;
    let vendor = attribute("vendor")?;
    let product = attribute("product")?;
    let version = attribute("version")?;
    ensure!(
        [
            bustype.as_str(),
            vendor.as_str(),
            product.as_str(),
            version.as_str()
        ]
        .iter()
        .all(|value| !value.is_empty()),
        "Evdev input identity attributes are incomplete"
    );
    let usb_root = usb_device_parent(&input).map(|root| {
        let text = |name: &str| {
            root.join(name)
                .canonicalize()
                .ok()
                .and_then(|path| read_small_text(&path).ok())
        };
        UsbRoot {
            devpath: root
                .canonicalize()
                .ok()
                .and_then(|path| {
                    path.to_str()
                        .map(|text| text.strip_prefix("/sys").unwrap_or(text).to_owned())
                })
                .unwrap_or_default(),
            id_vendor: text("idVendor").unwrap_or_default(),
            id_product: text("idProduct").unwrap_or_default(),
            manufacturer: text("manufacturer"),
            product: text("product"),
            serial: text("serial"),
        }
    });
    Ok(SysfsIdentity {
        input,
        bustype,
        vendor,
        product,
        version,
        usb_root,
    })
}

fn usb_device_parent(input: &Path) -> Option<PathBuf> {
    input.ancestors().skip(1).find_map(|ancestor| {
        let subsystem = ancestor.join("subsystem").canonicalize().ok()?;
        let devtype = read_small_text(&ancestor.join("devtype")).ok()?;
        (subsystem.file_name()?.to_str()? == "usb" && devtype == "usb_device")
            .then(|| ancestor.to_path_buf())
    })
}

/// Read every requested node once, rejecting duplicates.
pub fn catalog(events: &[PathBuf]) -> Result<EvdevCatalog> {
    ensure!(!events.is_empty(), "Evdev catalog needs at least one node");
    let mut seen = std::collections::BTreeMap::new();
    let mut devices = Vec::new();
    for event in events {
        ensure!(
            seen.insert(event.clone(), ()).is_none(),
            "Evdev node requested twice: {}",
            event.display()
        );
        devices.push(read_event(event)?);
    }
    Ok(EvdevCatalog { devices })
}

/// Pure projection over dumped capabilities for consumers that only need the
/// kernel-order tables; kept here so tests can exercise it without devices.
pub fn button_order(codes: &[u16]) -> Vec<u16> {
    // The udev driver iterates a code-ordered set: [BTN_JOYSTICK..KEY_MAX)
    // ascending first, then [BTN_MISC..BTN_JOYSTICK) ascending, assigning
    // sequential indices in that order.
    let mut scan: Vec<u16> = codes
        .iter()
        .copied()
        .filter(|code| (BTN_JOYSTICK..KEY_MAX).contains(code))
        .collect();
    scan.sort_unstable();
    let mut low: Vec<u16> = codes
        .iter()
        .copied()
        .filter(|code| (BTN_MISC..BTN_JOYSTICK).contains(code))
        .collect();
    low.sort_unstable();
    scan.extend(low);
    scan
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn button_order_matches_the_udev_driver_scan() {
        let ordered = button_order(&[0x130, 0x101, 0x121, 0x2a2]);
        assert_eq!(ordered, [0x121, 0x130, 0x2a2, 0x101]);
        assert!(button_order(&[0x2ff, 0xff]).is_empty());
    }

    #[test]
    fn hat_window_is_the_four_hat_pairs() {
        assert_eq!(ABS_HAT_WINDOW.start(), &0x10);
        assert_eq!(ABS_HAT_WINDOW.end(), &0x1b);
    }
}
