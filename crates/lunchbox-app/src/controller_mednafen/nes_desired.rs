//! Pinned Mednafen iNES InputDB device overrides, not ROM title heuristics.
//! Source: f0ee9d595db68ad5247ba5ac6a8367fdced9c3fc/src/nes/ines.cpp SetInput.
//! CRC must be the native loader's iNESGameCRC32, not a whole-file checksum.
use anyhow::{Result, ensure};
use std::collections::BTreeMap;

pub(crate) type Desired = [Option<&'static str>; 5];

pub(crate) fn ines(native_crc: u32) -> Desired {
    match native_crc {
        0x62c67984 => [
            Some("gamepad"),
            Some("gamepad"),
            Some("gamepad"),
            Some("gamepad"),
            Some("4player"),
        ],
        0x3a1694f9 => [
            Some("gamepad"),
            Some("gamepad"),
            Some("gamepad"),
            Some("gamepad"),
            Some("4player"),
        ],
        0xc3c0811d => [
            Some("gamepad"),
            Some("gamepad"),
            Some("gamepad"),
            Some("gamepad"),
            Some("oekakids"),
        ],
        0x9d048ea4 => [
            Some("gamepad"),
            Some("gamepad"),
            Some("gamepad"),
            Some("gamepad"),
            Some("oekakids"),
        ],
        0xaf4010ea => [
            Some("gamepad"),
            Some("powerpadb"),
            Some("gamepad"),
            Some("gamepad"),
            None,
        ],
        0xd74b2719 => [
            Some("gamepad"),
            Some("powerpadb"),
            Some("gamepad"),
            Some("gamepad"),
            None,
        ],
        0x61d86167 => [
            Some("gamepad"),
            Some("powerpadb"),
            Some("gamepad"),
            Some("gamepad"),
            None,
        ],
        0x6435c095 => [
            Some("gamepad"),
            Some("powerpadb"),
            Some("gamepad"),
            Some("gamepad"),
            None,
        ],
        0x48ca0ee1 => [
            Some("gamepad"),
            Some("gamepad"),
            Some("gamepad"),
            Some("gamepad"),
            Some("bworld"),
        ],
        0x9f8f200a => [
            Some("gamepad"),
            Some("gamepad"),
            Some("gamepad"),
            Some("gamepad"),
            Some("ftrainera"),
        ],
        0x9044550e => [
            Some("gamepad"),
            Some("gamepad"),
            Some("gamepad"),
            Some("gamepad"),
            Some("ftrainera"),
        ],
        0x2f128512 => [
            Some("gamepad"),
            Some("gamepad"),
            Some("gamepad"),
            Some("gamepad"),
            Some("ftrainera"),
        ],
        0x60ad090a => [
            Some("gamepad"),
            Some("gamepad"),
            Some("gamepad"),
            Some("gamepad"),
            Some("ftrainera"),
        ],
        0x8a12a7d9 => [
            Some("gamepad"),
            Some("gamepad"),
            Some("gamepad"),
            Some("gamepad"),
            Some("ftrainerb"),
        ],
        0xea90f3e2 => [
            Some("gamepad"),
            Some("gamepad"),
            Some("gamepad"),
            Some("gamepad"),
            Some("ftrainerb"),
        ],
        0x370ceb65 => [
            Some("gamepad"),
            Some("gamepad"),
            Some("gamepad"),
            Some("gamepad"),
            Some("ftrainerb"),
        ],
        0x6cca1c1f => [
            Some("gamepad"),
            Some("gamepad"),
            Some("gamepad"),
            Some("gamepad"),
            Some("ftrainerb"),
        ],
        0x29de87af => [
            Some("gamepad"),
            Some("gamepad"),
            Some("gamepad"),
            Some("gamepad"),
            Some("ftrainerb"),
        ],
        0xbba58be5 => [
            Some("gamepad"),
            Some("gamepad"),
            Some("gamepad"),
            Some("gamepad"),
            Some("ftrainerb"),
        ],
        0xd9f45be9 => [
            Some("gamepad"),
            Some("gamepad"),
            Some("gamepad"),
            Some("gamepad"),
            Some("partytap"),
        ],
        0x1545bd13 => [
            Some("gamepad"),
            Some("gamepad"),
            Some("gamepad"),
            Some("gamepad"),
            Some("partytap"),
        ],
        0x7b44fb2a => [
            Some("gamepad"),
            Some("gamepad"),
            Some("gamepad"),
            Some("gamepad"),
            Some("mahjong"),
        ],
        0x9fae4d46 => [
            Some("gamepad"),
            Some("gamepad"),
            Some("gamepad"),
            Some("gamepad"),
            Some("mahjong"),
        ],
        0x980be936 => [
            Some("gamepad"),
            Some("gamepad"),
            Some("gamepad"),
            Some("gamepad"),
            Some("hypershot"),
        ],
        0x21f85681 => [
            Some("gamepad"),
            Some("gamepad"),
            Some("gamepad"),
            Some("gamepad"),
            Some("hypershot"),
        ],
        0x915a53a7 => [
            Some("gamepad"),
            Some("gamepad"),
            Some("gamepad"),
            Some("gamepad"),
            Some("hypershot"),
        ],
        0xad9c63e2 => [
            Some("gamepad"),
            None,
            Some("gamepad"),
            Some("gamepad"),
            Some("shadow"),
        ],
        0x4d68cfb1 => [
            None,
            Some("zapper"),
            Some("gamepad"),
            Some("gamepad"),
            Some("none"),
        ],
        0xbbe40dc4 => [
            None,
            Some("zapper"),
            Some("gamepad"),
            Some("gamepad"),
            Some("none"),
        ],
        0x24598791 => [
            None,
            Some("zapper"),
            Some("gamepad"),
            Some("gamepad"),
            Some("none"),
        ],
        0xff24d794 => [
            None,
            Some("zapper"),
            Some("gamepad"),
            Some("gamepad"),
            Some("none"),
        ],
        0xbeb8ab01 => [
            None,
            Some("zapper"),
            Some("gamepad"),
            Some("gamepad"),
            Some("none"),
        ],
        0xde8fd935 => [
            None,
            Some("zapper"),
            Some("gamepad"),
            Some("gamepad"),
            Some("none"),
        ],
        0xedc3662b => [
            None,
            Some("zapper"),
            Some("gamepad"),
            Some("gamepad"),
            Some("none"),
        ],
        0x2a6559a1 => [
            None,
            Some("zapper"),
            Some("gamepad"),
            Some("gamepad"),
            Some("none"),
        ],
        0x23d17f5e => [
            Some("gamepad"),
            Some("zapper"),
            Some("gamepad"),
            Some("gamepad"),
            Some("none"),
        ],
        0xb8b9aca3 => [
            None,
            Some("zapper"),
            Some("gamepad"),
            Some("gamepad"),
            Some("none"),
        ],
        0x5112dc21 => [
            None,
            Some("zapper"),
            Some("gamepad"),
            Some("gamepad"),
            Some("none"),
        ],
        0x4318a2f8 => [
            None,
            Some("zapper"),
            Some("gamepad"),
            Some("gamepad"),
            Some("none"),
        ],
        0x5ee6008e => [
            None,
            Some("zapper"),
            Some("gamepad"),
            Some("gamepad"),
            Some("none"),
        ],
        0x3e58a87e => [
            None,
            Some("zapper"),
            Some("gamepad"),
            Some("gamepad"),
            Some("none"),
        ],
        0x851eb9be => [
            Some("gamepad"),
            Some("zapper"),
            Some("gamepad"),
            Some("gamepad"),
            Some("none"),
        ],
        0x74bea652 => [
            Some("gamepad"),
            Some("zapper"),
            Some("gamepad"),
            Some("gamepad"),
            Some("none"),
        ],
        0x32fb0583 => [
            None,
            Some("arkanoid"),
            Some("gamepad"),
            Some("gamepad"),
            Some("none"),
        ],
        0xd89e5a67 => [
            None,
            None,
            Some("gamepad"),
            Some("gamepad"),
            Some("arkanoid"),
        ],
        0x0f141525 => [
            None,
            None,
            Some("gamepad"),
            Some("gamepad"),
            Some("arkanoid"),
        ],
        0x912989dc => [None, None, Some("gamepad"), Some("gamepad"), Some("fkb")],
        0xf7606810 => [None, None, Some("gamepad"), Some("gamepad"), Some("fkb")],
        0x895037bc => [None, None, Some("gamepad"), Some("gamepad"), Some("fkb")],
        0xb2530afc => [None, None, Some("gamepad"), Some("gamepad"), Some("fkb")],
        _ => [None; 5],
    }
}

/// Native frontend uses a non-null DesiredInput before the saved device setting.
/// An unused port can still be forced to a gamepad by native ROM metadata.
/// Keep that device with every binding cleared; never invent an active player.
/// Work on a copy so any remaining conflict leaves the original untouched.
pub(crate) fn reconcile_unused_pads(
    desired: &Desired,
    settings: &mut BTreeMap<String, String>,
) -> Result<()> {
    let mut reconciled = settings.clone();
    for (index, forced) in desired[..4].iter().enumerate() {
        let prefix = format!("nes.input.port{}", index + 1);
        if *forced == Some("gamepad") && reconciled.get(&prefix).map(String::as_str) == Some("none")
        {
            reconciled.insert(prefix.clone(), "gamepad".into());
            for control in super::nes::CONTROLS
                .iter()
                .copied()
                .chain(["rapid_a", "rapid_b"])
            {
                reconciled.insert(format!("{prefix}.gamepad.{control}"), String::new());
            }
        }
    }
    verify_device_selection(desired, &reconciled)?;
    *settings = reconciled;
    Ok(())
}

/// Native frontend uses a non-null DesiredInput before the saved device setting.
/// None means inherit, while Some("none") explicitly disconnects a device.
pub(crate) fn verify_device_selection(
    desired: &Desired,
    settings: &BTreeMap<String, String>,
) -> Result<()> {
    for (port, forced) in ["port1", "port2", "port3", "port4", "fcexp"]
        .iter()
        .zip(desired)
    {
        if let Some(forced) = forced {
            let key = format!("nes.input.{port}");
            ensure!(
                settings.get(&key).map(String::as_str) == Some(*forced),
                "Mednafen ROM forces NES {port} to {forced}, conflicting with the saved controller setup"
            );
        }
    }
    Ok(())
}

/// Apply a UNIF CTRL payload to the loader's current desired inputs. Later
/// chunks replace only ports 1/2; the native code does not infer peripherals
/// from other bits. Chunk framing remains the content reader's responsibility.
pub(crate) fn unif_ctrl(desired: &mut Desired, payload: &[u8]) -> Result<()> {
    let flags = *payload
        .first()
        .ok_or_else(|| anyhow::anyhow!("Empty UNIF CTRL chunk"))?;
    desired[0] = Some(if flags & 1 != 0 { "gamepad" } else { "none" });
    desired[1] = Some(if flags & 2 != 0 {
        "zapper"
    } else if flags & 1 != 0 {
        "gamepad"
    } else {
        "none"
    });
    Ok(())
}
