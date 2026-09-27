//! Native iNES checksum semantics from the pinned nes/ines.cpp iNESLoad.
use anyhow::{Result, ensure};

/// Mednafen hashes PRG and CHR separately padded to power-of-two bank counts
/// with 0xff. The header, trainer and unused trailing bytes are not hashed.
/// This deliberately follows the pinned loader's legacy size interpretation,
/// including PRG byte zero meaning 256 banks, rather than substituting NES 2.0.
pub(crate) fn ines_crc(content: &[u8]) -> Result<u32> {
    ensure!(
        content.len() >= 16 && &content[..4] == b"NES\x1a",
        "Mednafen NES content requires a complete iNES header"
    );
    let prg_banks = if content[4] == 0 {
        256
    } else {
        usize::from(content[4])
    };
    let chr_banks = usize::from(content[5]);
    let trainer = if content[6] & 4 != 0 { 512 } else { 0 };
    let prg_start = 16 + trainer;
    let prg_size = prg_banks * 0x4000;
    let chr_size = chr_banks * 0x2000;
    let chr_start = prg_start + prg_size;
    ensure!(
        content.len() >= chr_start + chr_size,
        "Mednafen iNES content is shorter than its declared PRG/CHR data"
    );
    let mut checksum = crc32fast::Hasher::new();
    checksum.update(&content[prg_start..chr_start]);
    padding(
        &mut checksum,
        (prg_banks.next_power_of_two() - prg_banks) * 0x4000,
    );
    if chr_banks != 0 {
        checksum.update(&content[chr_start..chr_start + chr_size]);
        padding(
            &mut checksum,
            (chr_banks.next_power_of_two() - chr_banks) * 0x2000,
        );
    }
    Ok(checksum.finalize())
}

fn padding(checksum: &mut crc32fast::Hasher, mut count: usize) {
    let fill = [0xff; 4096];
    while count != 0 {
        let amount = count.min(fill.len());
        checksum.update(&fill[..amount]);
        count -= amount;
    }
}

/// Evaluate device overrides from ROM bytes, never from a title or file hash.
pub(crate) fn ines_desired(content: &[u8]) -> Result<super::nes_desired::Desired> {
    Ok(super::nes_desired::ines(ines_crc(content)?))
}

/// Read conventional UNIF framing. The pinned native loader consumes fixed
/// lengths for metadata handlers, regardless of the declared chunk size; reject
/// mismatches rather than predict a different subsequent CTRL chunk boundary.
pub(crate) fn unif_desired(content: &[u8]) -> Result<super::nes_desired::Desired> {
    ensure!(
        content.len() >= 32 && &content[..4] == b"UNIF",
        "Incomplete UNIF header"
    );
    let mut desired = [None, None, Some("gamepad"), Some("gamepad"), None];
    let mut offset = 32usize;
    let mut chunks = 0;
    while offset < content.len() {
        chunks += 1;
        ensure!(
            chunks <= 65536 && content.len() - offset >= 8,
            "Invalid or excessive UNIF chunks"
        );
        let id = &content[offset..offset + 4];
        let size = u32::from_le_bytes(content[offset + 4..offset + 8].try_into()?) as usize;
        offset += 8;
        let end = offset
            .checked_add(size)
            .ok_or_else(|| anyhow::anyhow!("UNIF chunk size overflow"))?;
        ensure!(end <= content.len(), "Truncated UNIF chunk");
        let fixed = match id {
            b"CTRL" | b"TVCI" | b"BATR" | b"MIRR" => Some(1),
            b"DINF" => Some(204),
            _ => None,
        };
        ensure!(
            fixed.is_none_or(|expected| size == expected),
            "UNIF metadata length differs from native handler consumption"
        );
        if id == b"CTRL" {
            super::nes_desired::unif_ctrl(&mut desired, &content[offset..end])?;
        }
        offset = end;
    }
    Ok(desired)
}

pub(crate) fn desired(content: &[u8]) -> Result<super::nes_desired::Desired> {
    if content.starts_with(b"UNIF") {
        unif_desired(content)
    } else if is_fds(content) {
        // FDSLoad leaves DesiredInput empty. Firmware and disk insertion
        // remain native runtime duties, separate from pad mapping.
        fds_sides(content)?;
        Ok([None; 5])
    } else {
        ines_desired(content)
    }
}

fn is_fds(content: &[u8]) -> bool {
    content.len() >= 16 && (content.starts_with(b"FDS\x1a") || &content[1..15] == b"*NINTENDO-HVC*")
}

/// FDS SubLoad caps headered images to 1..8 sides and ignores partial final
/// sides. Require one complete side rather than preparing an empty image.
pub(crate) fn fds_sides(content: &[u8]) -> Result<usize> {
    ensure!(is_fds(content), "Unrecognized FDS disk image header");
    let (offset, maximum) = if content.starts_with(b"FDS\x1a") {
        (16, usize::from(content[4]).min(8).max(1))
    } else {
        (0, 8)
    };
    let sides = ((content.len() - offset) / 65500).min(maximum);
    ensure!(sides > 0, "FDS image has no complete disk sides");
    Ok(sides)
}
