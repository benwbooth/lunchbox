//! VirtualC64 6 standalone-native macOS controller writers.
//!
//! Pinned source: dirkwhoffmann/virtualc64 commit
//! daab78ff6059df44d3bf33d944b3fb2014c91338. `GUI/Defaults.swift` and
//! `VCCore/Infrastructure/Defaults.cpp` load `virtualc64.ini` and define
//! `[Peripherals] ControlPort1/ControlPort2`. `GUI/Input/DeviceDatabase.swift`
//! stores an SDL-gamecontrollerdb descriptor table as JSON-encoded
//! `[GUID:String]` data under the `Devices.Schemes` standard-defaults key.
//! Physical HID devices occupy dynamic slots 3 through 6, so the caller must
//! measure and recheck a slot immediately before launch; this module never
//! guesses one from a controller name or USB IDs alone.

use anyhow::{Context, Result, ensure};
use serde_json::{Map, Value};
use std::collections::BTreeSet;

pub(crate) const SOURCE_COMMIT: &str = "daab78ff6059df44d3bf33d944b3fb2014c91338";
pub(crate) const BUNDLE_ID: &str = "de.dirkwhoffmann.VC64";
pub(crate) const DEVICE_SCHEMES_KEY: &str = "Devices.Schemes";

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) struct AxisBinding {
    /// Zero-based HID axis index accepted by DeviceDatabase (`a0` through
    /// `a5`).
    pub axis: u8,
    /// Emit the source-supported `aN~` reversed-axis spelling.
    pub reversed: bool,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct DeviceScheme {
    /// VirtualC64's 32-hex-digit HID GUID. Device matching compares the
    /// vendor/product/version slices, so the writer canonicalizes it to lower
    /// case to match the source-generated IOHID GUID.
    pub guid: String,
    pub name: String,
    pub horizontal: AxisBinding,
    pub vertical: AxisBinding,
    /// Zero-based HID button index accepted by DeviceDatabase (`b0` through
    /// `b16`). Any of the source-recognized logical fire keys map to the one
    /// C64 fire action; this writer uses `a`.
    pub fire_button: u8,
}

fn canonical_guid(guid: &str) -> Result<String> {
    ensure!(
        guid.len() == 32,
        "VirtualC64 HID GUID must contain 32 hex digits"
    );
    ensure!(
        guid.bytes().all(|byte| byte.is_ascii_hexdigit()),
        "VirtualC64 HID GUID contains a non-hex character"
    );
    Ok(guid.to_ascii_lowercase())
}

impl DeviceScheme {
    fn descriptor(&self) -> Result<(String, String)> {
        let guid = canonical_guid(&self.guid)?;
        ensure!(
            !self.name.trim().is_empty() && self.name.len() <= 128,
            "VirtualC64 device name is empty or too long"
        );
        ensure!(
            !self
                .name
                .bytes()
                .any(|byte| byte == b',' || byte == b'\n' || byte == b'\r' || byte == 0),
            "VirtualC64 device name contains a descriptor delimiter"
        );
        ensure!(
            self.horizontal.axis <= 5 && self.vertical.axis <= 5,
            "VirtualC64 supports HID axes a0 through a5"
        );
        ensure!(
            self.horizontal.axis != self.vertical.axis,
            "VirtualC64 horizontal and vertical axes must be distinct"
        );
        ensure!(
            self.fire_button <= 16,
            "VirtualC64 supports HID buttons b0 through b16"
        );

        let axis = |binding: AxisBinding| {
            format!(
                "a{}{}",
                binding.axis,
                if binding.reversed { "~" } else { "" }
            )
        };
        let descriptor = format!(
            "{guid},{},leftx:{},lefty:{},a:b{},platform:Mac OS X",
            self.name,
            axis(self.horizontal),
            axis(self.vertical),
            self.fire_button
        );
        Ok((guid, descriptor))
    }
}

/// Encode the exact JSON shape Foundation uses for `[GUID:String]` when GUID
/// is a non-String `Codable` dictionary key: an unkeyed array alternating the
/// encoded key object (`{"guid":"..."}`) and its descriptor value. The
/// returned bytes are the `Data` value stored at `Devices.Schemes`, not a
/// property-list wrapper.
pub(crate) fn device_schemes_json(schemes: &[DeviceScheme]) -> Result<Vec<u8>> {
    ensure!(
        !schemes.is_empty() && schemes.len() <= 4,
        "VirtualC64 supports one through four physical HID devices"
    );
    let mut seen = BTreeSet::new();
    let mut encoded = Vec::with_capacity(schemes.len() * 2);
    for scheme in schemes {
        let (guid, descriptor) = scheme.descriptor()?;
        ensure!(
            seen.insert(guid.clone()),
            "VirtualC64 HID GUID is duplicated"
        );
        let mut key = Map::new();
        key.insert("guid".into(), Value::String(guid));
        encoded.push(Value::Object(key));
        encoded.push(Value::String(descriptor));
    }
    serde_json::to_vec(&Value::Array(encoded)).context("encode VirtualC64 device schemes")
}

/// Render the JSON data as the hexadecimal argument accepted by macOS
/// `defaults write <domain> <key> -data <hex>`. The launch layer is
/// responsible for writing the correct app-container defaults domain while
/// VirtualC64 is stopped and verifying the value through CFPreferences before
/// starting the app.
pub(crate) fn device_schemes_defaults_hex(schemes: &[DeviceScheme]) -> Result<String> {
    Ok(hex::encode(device_schemes_json(schemes)?))
}

fn slot_value(slot: Option<u8>) -> Result<i8> {
    match slot {
        Some(slot) => {
            ensure!(
                (3..=6).contains(&slot),
                "VirtualC64 physical HID slot must be 3 through 6"
            );
            Ok(slot as i8)
        }
        None => Ok(-1),
    }
}

/// Patch only the two source-defined controller-port keys in a copied
/// `virtualc64.ini`. All ROM paths, drives, media, capture, audio, video,
/// server and machine settings remain represented by the baseline. A physical
/// slot may be used by at most one emulated port.
pub(crate) fn patch_control_ports(
    baseline: &[u8],
    control_port_1: Option<u8>,
    control_port_2: Option<u8>,
) -> Result<String> {
    ensure!(baseline.len() <= 1024 * 1024, "VirtualC64 INI is too large");
    let port_1 = slot_value(control_port_1)?;
    let port_2 = slot_value(control_port_2)?;
    ensure!(
        port_1 < 0 || port_2 < 0 || port_1 != port_2,
        "VirtualC64 physical HID slot cannot drive both ports"
    );
    let original = std::str::from_utf8(baseline).context("VirtualC64 INI is not UTF-8")?;
    ensure!(
        !original.contains('\0'),
        "VirtualC64 INI contains a NUL byte"
    );

    let (bom, text) = original
        .strip_prefix('\u{feff}')
        .map_or(("", original), |text| ("\u{feff}", text));
    let newline = if text.contains("\r\n") { "\r\n" } else { "\n" };
    let fields = [
        ("ControlPort1", port_1.to_string()),
        ("ControlPort2", port_2.to_string()),
    ];
    let mut output = bom.to_owned();
    let mut active = false;
    let mut inserted = false;

    for line in text.split_inclusive('\n') {
        if let Some(header) = line
            .trim_start()
            .strip_prefix('[')
            .and_then(|line| line.split_once(']').map(|(name, _)| name.trim()))
        {
            active = header == "Peripherals";
            output.push_str(line);
            if active && !inserted {
                if !output.ends_with('\n') {
                    output.push_str(newline);
                }
                for (key, value) in &fields {
                    output.push_str(&format!("{key}={value}{newline}"));
                }
                inserted = true;
            }
            continue;
        }

        let owned = active
            && line
                .split_once('=')
                .is_some_and(|(key, _)| fields.iter().any(|(known, _)| key.trim() == *known));
        if !owned {
            output.push_str(line);
        }
    }

    if !inserted {
        if !output.is_empty() && !output.ends_with('\n') {
            output.push_str(newline);
        }
        output.push_str(&format!("[Peripherals]{newline}"));
        for (key, value) in &fields {
            output.push_str(&format!("{key}={value}{newline}"));
        }
    }
    Ok(output)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn scheme(guid: &str) -> DeviceScheme {
        DeviceScheme {
            guid: guid.into(),
            name: "Example Pad".into(),
            horizontal: AxisBinding {
                axis: 0,
                reversed: false,
            },
            vertical: AxisBinding {
                axis: 1,
                reversed: true,
            },
            fire_button: 2,
        }
    }

    #[test]
    fn emits_swift_dictionary_shape_and_source_descriptor() {
        let data = device_schemes_json(&[scheme("030000005e0400008e02000000010000")]).unwrap();
        assert_eq!(
            String::from_utf8(data).unwrap(),
            "[{\"guid\":\"030000005e0400008e02000000010000\"},\"030000005e0400008e02000000010000,Example Pad,leftx:a0,lefty:a1~,a:b2,platform:Mac OS X\"]"
        );
    }

    #[test]
    fn patches_only_peripheral_ports_and_preserves_newlines() {
        let baseline = b"# VirtualC64\r\n[Peripherals]\r\nDrive8=1\r\nControlPort1=-1\r\nControlPort2=1\r\n[Audio]\r\nVolume=75\r\n";
        let output = patch_control_ports(baseline, Some(3), Some(4)).unwrap();
        assert!(
            output.contains("[Peripherals]\r\nControlPort1=3\r\nControlPort2=4\r\nDrive8=1\r\n")
        );
        assert!(output.contains("[Audio]\r\nVolume=75\r\n"));
        assert!(!output.contains("ControlPort1=-1"));
    }

    #[test]
    fn refuses_unmeasured_or_ambiguous_identity() {
        assert!(patch_control_ports(b"", Some(2), None).is_err());
        assert!(patch_control_ports(b"", Some(3), Some(3)).is_err());
        assert!(device_schemes_json(&[scheme("not-a-guid")]).is_err());
        assert!(
            device_schemes_json(&[
                scheme("030000005e0400008e02000000010000"),
                scheme("030000005e0400008e02000000010000"),
            ])
            .is_err()
        );
    }
}
