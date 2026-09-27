//! Native Linux legacy-joystick identity and manager collision allocation.
use anyhow::{Result, ensure};
use std::collections::BTreeSet;

/// Joystick_Linux.cpp packs sysfs bus/vendor/product/version followed by
/// JSIOCGAXES/JSIOCGBUTTONS counts. Do not substitute evdev capability counts.
pub(crate) fn linux_base(
    bus: u16,
    vendor: u16,
    product: u16,
    version: u16,
    axes: u8,
    buttons: u8,
) -> [u8; 16] {
    let mut id = [0; 16];
    for (index, value) in [
        bus,
        vendor,
        product,
        version,
        u16::from(axes),
        u16::from(buttons),
    ]
    .into_iter()
    .enumerate()
    {
        id[index * 2..index * 2 + 2].copy_from_slice(&value.to_be_bytes());
    }
    id
}

/// Input order must be the native driver/cache enumeration order, including
/// other drivers if enabled. Same-model duplicates consume successive IDs.
pub(crate) fn allocate(base_ids: &[[u8; 16]]) -> Result<Vec<[u8; 16]>> {
    ensure!(
        base_ids.len() <= 1024,
        "Mednafen joystick inventory exceeds limit"
    );
    let mut used = BTreeSet::new();
    let mut result = Vec::new();
    for base in base_ids {
        let mut id = *base;
        while !used.insert(id) {
            let low = u64::from_be_bytes(id[8..].try_into()?);
            // Native uses uint64 addition, including wraparound.
            id[8..].copy_from_slice(&low.wrapping_add(1).to_be_bytes());
        }
        result.push(id);
    }
    Ok(result)
}

pub(crate) fn as_hex(id: &[u8; 16]) -> String {
    use std::fmt::Write;
    let mut result = String::from("0x");
    for byte in id {
        write!(result, "{byte:02x}").expect("writing to String");
    }
    result
}
