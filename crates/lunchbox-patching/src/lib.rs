//! Checked patch decoders. Callers own the output: originals are never opened
//! for writing. PPF and xdelta stream disc images; cartridge patches are bounded.
use anyhow::{Context, Result, bail, ensure};
use std::fs::{self, File, OpenOptions};
use std::io::{Read, Seek, SeekFrom, Write};
use std::path::Path;
use std::sync::atomic::{AtomicBool, Ordering};

pub const MAX_PATCH: u64 = 512 * 1024 * 1024;
const MAX_ROM: usize = 512 * 1024 * 1024;
const MAX_DISC: u64 = 128 * 1024 * 1024 * 1024;

pub fn format(bytes: &[u8]) -> Result<&'static str> {
    Ok(if bytes.starts_with(b"PATCH") {
        "IPS"
    } else if bytes.starts_with(b"IPS32") {
        "IPS32"
    } else if bytes.starts_with(b"BPS1") {
        "BPS"
    } else if bytes.starts_with(b"UPS1") {
        "UPS"
    } else if bytes.starts_with(b"PPF10")
        || bytes.starts_with(b"PPF20")
        || bytes.starts_with(b"PPF30")
    {
        "PPF"
    } else if bytes.starts_with(&[0xd6, 0xc3, 0xc4, 0]) {
        "xdelta"
    } else {
        bail!(
            "Unsupported patch. Supported: IPS, IPS32, BPS, UPS, PPF, xdelta/VCDIFF. Extract ZIP/7z downloads first."
        )
    })
}

fn cancelled(flag: &AtomicBool) -> Result<()> {
    ensure!(!flag.load(Ordering::Relaxed), "Patch preparation cancelled");
    Ok(())
}

pub fn apply(source: &Path, patch: &Path, output: &Path, cancel: &AtomicBool) -> Result<()> {
    cancelled(cancel)?;
    ensure!(!output.exists(), "Patch output already exists");
    ensure!(
        fs::metadata(patch)?.len() <= MAX_PATCH,
        "Patch exceeds 512 MiB limit"
    );
    let data = fs::read(patch)?;
    let kind = format(&data)?;
    let result = (|| {
        match kind {
            "xdelta" => xdelta(source, patch, output, cancel)?,
            "PPF" => ppf(source, &data, output, cancel)?,
            _ => {
                ensure!(
                    fs::metadata(source)?.len() <= MAX_ROM as u64,
                    "Cartridge patch input exceeds 512 MiB; use PPF or xdelta for large disc images"
                );
                let original = fs::read(source)?;
                let bytes = match kind {
                    "IPS" | "IPS32" => ips(&original, &data, cancel)?,
                    "BPS" => bps(&original, &data, cancel)?,
                    "UPS" => ups(&original, &data, cancel)?,
                    _ => unreachable!(),
                };
                cancelled(cancel)?;
                OpenOptions::new()
                    .write(true)
                    .create_new(true)
                    .open(output)?
                    .write_all(&bytes)?;
            }
        }
        cancelled(cancel)
    })();
    if result.is_err() {
        let _ = fs::remove_file(output);
    }
    result.with_context(|| format!("Applying {kind} patch {}", patch.display()))
}

struct Cursor<'a> {
    data: &'a [u8],
    pos: usize,
}
impl<'a> Cursor<'a> {
    fn take(&mut self, n: usize) -> Result<&'a [u8]> {
        let end = self.pos.checked_add(n).context("Patch offset overflow")?;
        let bytes = self.data.get(self.pos..end).context("Truncated patch")?;
        self.pos = end;
        Ok(bytes)
    }
    fn byte(&mut self) -> Result<u8> {
        Ok(self.take(1)?[0])
    }
    fn number(&mut self, n: usize, little: bool) -> Result<u64> {
        let bytes = self.take(n)?;
        let mut result = 0u64;
        if little {
            for (i, b) in bytes.iter().enumerate() {
                result |= u64::from(*b) << (i * 8);
            }
        } else {
            for b in bytes {
                result = (result << 8) | u64::from(*b);
            }
        }
        Ok(result)
    }
    fn varint(&mut self) -> Result<usize> {
        let mut value = 0usize;
        let mut shift = 1usize;
        loop {
            let b = self.byte()?;
            value = value
                .checked_add(
                    usize::from(b & 127)
                        .checked_mul(shift)
                        .context("Patch integer overflow")?,
                )
                .context("Patch integer overflow")?;
            if b & 128 != 0 {
                return Ok(value);
            }
            shift = shift.checked_mul(128).context("Patch integer overflow")?;
            value = value.checked_add(shift).context("Patch integer overflow")?;
        }
    }
}

fn ips(source: &[u8], data: &[u8], cancel: &AtomicBool) -> Result<Vec<u8>> {
    let wide = data.starts_with(b"IPS32");
    let offset_len = if wide { 4 } else { 3 };
    let end: &[u8] = if wide { b"EEOF" } else { b"EOF" };
    let mut c = Cursor { data, pos: 5 };
    let mut output = source.to_vec();
    loop {
        cancelled(cancel)?;
        if c.data.get(c.pos..c.pos + end.len()) == Some(end) {
            c.pos += end.len();
            if c.pos < data.len() {
                let size = c.number(offset_len, false)? as usize;
                ensure!(size <= MAX_ROM, "IPS output exceeds 512 MiB");
                output.resize(size, 0);
            }
            ensure!(c.pos == data.len(), "Unexpected IPS trailing bytes");
            return Ok(output);
        }
        let offset = c.number(offset_len, false)? as usize;
        let size = c.number(2, false)? as usize;
        let (size, run) = if size == 0 {
            (c.number(2, false)? as usize, Some(c.byte()?))
        } else {
            (size, None)
        };
        let end = offset.checked_add(size).context("IPS range overflow")?;
        ensure!(size > 0 && end <= MAX_ROM, "Invalid IPS output range");
        output.resize(output.len().max(end), 0);
        if let Some(byte) = run {
            output[offset..end].fill(byte);
        } else {
            output[offset..end].copy_from_slice(c.take(size)?);
        }
    }
}

fn checked_footer(data: &[u8], source: &[u8]) -> Result<(u32, usize)> {
    ensure!(data.len() >= 16, "Truncated checksummed patch");
    let footer = data.len() - 12;
    let read = |at| u32::from_le_bytes(data[at..at + 4].try_into().unwrap());
    ensure!(
        crc32fast::hash(&data[..data.len() - 4]) == read(data.len() - 4),
        "Patch checksum mismatch: download may be damaged"
    );
    ensure!(
        crc32fast::hash(source) == read(footer),
        "Base ROM checksum mismatch. Use the patch author's exact region, revision and header format"
    );
    Ok((read(footer + 4), footer))
}

fn bps(source: &[u8], data: &[u8], cancel: &AtomicBool) -> Result<Vec<u8>> {
    let (expected, footer) = checked_footer(data, source)?;
    let mut c = Cursor {
        data: &data[..footer],
        pos: 4,
    };
    ensure!(c.varint()? == source.len(), "BPS base size mismatch");
    let size = c.varint()?;
    ensure!(size <= MAX_ROM, "BPS output exceeds 512 MiB");
    let metadata = c.varint()?;
    c.take(metadata)?;
    let mut output = Vec::with_capacity(size);
    let mut source_offset = 0i64;
    let mut target_offset = 0i64;
    while output.len() < size {
        cancelled(cancel)?;
        let action = c.varint()?;
        let count = (action >> 2).checked_add(1).context("BPS size overflow")?;
        ensure!(
            count <= size - output.len(),
            "BPS instruction exceeds output size"
        );
        match action & 3 {
            0 => {
                let start = output.len();
                output.extend_from_slice(
                    source
                        .get(start..start + count)
                        .context("BPS source-read outside input")?,
                );
            }
            1 => output.extend_from_slice(c.take(count)?),
            mode => {
                let encoded = c.varint()?;
                let delta = i64::try_from(encoded >> 1)? * if encoded & 1 == 0 { 1 } else { -1 };
                let offset = if mode == 2 {
                    &mut source_offset
                } else {
                    &mut target_offset
                };
                *offset = offset.checked_add(delta).context("BPS copy overflow")?;
                let start = usize::try_from(*offset).context("Negative BPS copy offset")?;
                if mode == 2 {
                    let end = start
                        .checked_add(count)
                        .context("BPS source range overflow")?;
                    output.extend_from_slice(
                        source
                            .get(start..end)
                            .context("BPS source-copy outside input")?,
                    );
                } else {
                    ensure!(
                        start < output.len(),
                        "BPS target-copy references unwritten bytes"
                    );
                    // Overlapping target copies are intentional in BPS.
                    for i in 0..count {
                        if i % 65536 == 0 {
                            cancelled(cancel)?;
                        }
                        let byte = output[start + i];
                        output.push(byte);
                    }
                }
                *offset = offset
                    .checked_add(i64::try_from(count)?)
                    .context("BPS copy overflow")?;
            }
        }
    }
    ensure!(c.pos == footer, "Unexpected BPS trailing instructions");
    ensure!(
        crc32fast::hash(&output) == expected,
        "BPS output checksum mismatch"
    );
    Ok(output)
}

fn ups(source: &[u8], data: &[u8], cancel: &AtomicBool) -> Result<Vec<u8>> {
    let (expected, footer) = checked_footer(data, source)?;
    let mut c = Cursor {
        data: &data[..footer],
        pos: 4,
    };
    ensure!(c.varint()? == source.len(), "UPS base size mismatch");
    let size = c.varint()?;
    ensure!(size <= MAX_ROM, "UPS output exceeds 512 MiB");
    let mut output = source.to_vec();
    output.resize(size, 0);
    let mut at = 0usize;
    while c.pos < footer {
        cancelled(cancel)?;
        at = at.checked_add(c.varint()?).context("UPS offset overflow")?;
        loop {
            let byte = c.byte()?;
            if byte != 0 {
                ensure!(at < size.max(source.len()), "UPS XOR outside content");
                if at < size {
                    output[at] ^= byte;
                } else {
                    ensure!(source[at] ^ byte == 0, "Invalid UPS truncated tail");
                }
            }
            at = at.checked_add(1).context("UPS offset overflow")?;
            if byte == 0 {
                break;
            }
        }
    }
    ensure!(
        crc32fast::hash(&output) == expected,
        "UPS output checksum mismatch"
    );
    Ok(output)
}

fn ppf(source: &Path, data: &[u8], output: &Path, cancel: &AtomicBool) -> Result<()> {
    let version = data[3] - b'0';
    let mut c = Cursor { data, pos: 5 };
    ensure!(c.byte()? == version - 1, "Invalid PPF version");
    c.take(50)?;
    let mut original = File::open(source)?;
    let source_size = original.metadata()?.len();
    ensure!(source_size <= MAX_DISC, "Disc image exceeds 128 GiB limit");
    let (block, undo, check_offset) = if version == 3 {
        let image = c.byte()?;
        ensure!(image <= 1, "Unsupported PPF image type");
        let block = c.byte()?;
        let undo = c.byte()?;
        ensure!(block <= 1 && undo <= 1, "Invalid PPF flags");
        c.byte()?;
        (
            block == 1,
            undo == 1,
            if image == 0 { 0x9320 } else { 0x80a0 },
        )
    } else if version == 2 {
        ensure!(
            c.number(4, true)? == source_size,
            "PPF base image size mismatch"
        );
        (true, false, 0x9320)
    } else {
        (false, false, 0)
    };
    if block {
        let expected = c.take(1024)?;
        let mut actual = [0; 1024];
        original.seek(SeekFrom::Start(check_offset))?;
        original
            .read_exact(&mut actual)
            .context("PPF base image too short for block check")?;
        ensure!(
            expected == actual,
            "PPF base image block mismatch; wrong revision or image layout"
        );
    }
    original.rewind()?;
    let mut target = OpenOptions::new()
        .create_new(true)
        .write(true)
        .open(output)?;
    let mut buffer = vec![0; 1024 * 1024];
    loop {
        cancelled(cancel)?;
        let n = original.read(&mut buffer)?;
        if n == 0 {
            break;
        }
        target.write_all(&buffer[..n])?;
    }
    while c.pos < data.len() {
        cancelled(cancel)?;
        // PPF's optional file description follows the final record.
        if data[c.pos..].starts_with(b"@BEGIN_FILE_ID.DIZ") {
            break;
        }
        let at = c.number(if version == 3 { 8 } else { 4 }, true)?;
        let count = usize::from(c.byte()?);
        ensure!(
            count > 0 && at.checked_add(count as u64).is_some_and(|n| n <= MAX_DISC),
            "Invalid PPF write range"
        );
        let replacement = c.take(count)?;
        if undo {
            let expected = c.take(count)?;
            original.seek(SeekFrom::Start(at))?;
            let mut actual = vec![0; count];
            original.read_exact(&mut actual)?;
            ensure!(
                actual == expected,
                "PPF undo data does not match the base image (wrong or already patched input)"
            );
        }
        target.seek(SeekFrom::Start(at))?;
        target.write_all(replacement)?;
    }
    Ok(())
}

fn xdelta(source: &Path, patch: &Path, output: &Path, cancel: &AtomicBool) -> Result<()> {
    use std::process::{Command, Stdio};
    let tool = std::env::var_os("LUNCHBOX_XDELTA3").unwrap_or_else(|| "xdelta3".into());
    let mut command = Command::new(tool);
    command
        .env_remove("XDELTA")
        .args(["-d", "-D", "-R", "-s"])
        .arg(source)
        .arg(patch)
        .arg(output)
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null());
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        command.creation_flags(0x08000000);
    }
    let mut child = command.spawn().context("xdelta3 is required for this patch. Install xdelta3 or set LUNCHBOX_XDELTA3 to its executable path")?;
    let started = std::time::Instant::now();
    loop {
        if cancel.load(Ordering::Relaxed)
            || started.elapsed().as_secs() > 600
            || fs::metadata(output).is_ok_and(|m| m.len() > MAX_DISC)
        {
            let _ = child.kill();
            let _ = child.wait();
            bail!("xdelta cancelled, exceeded ten minutes, or exceeded 128 GiB output");
        }
        if let Some(status) = child.try_wait()? {
            ensure!(
                status.success() && output.is_file(),
                "xdelta failed: verify the base image region, revision, checksum and extracted disc format"
            );
            return Ok(());
        }
        std::thread::sleep(std::time::Duration::from_millis(25));
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    fn flag() -> AtomicBool {
        AtomicBool::new(false)
    }
    fn var(mut n: usize, output: &mut Vec<u8>) {
        loop {
            let b = (n & 127) as u8;
            n >>= 7;
            if n == 0 {
                output.push(b | 128);
                break;
            }
            output.push(b);
            n -= 1;
        }
    }
    fn footer(patch: &mut Vec<u8>, source: &[u8], target: &[u8]) {
        patch.extend(crc32fast::hash(source).to_le_bytes());
        patch.extend(crc32fast::hash(target).to_le_bytes());
        patch.extend(crc32fast::hash(patch).to_le_bytes());
    }
    #[test]
    fn ips_literals_runs_and_truncate() {
        let data = b"PATCH\0\0\x01\0\x02Hi\0\0\x04\0\0\0\x03!EOF\0\0\x08";
        assert_eq!(ips(b"abcdefghij", data, &flag()).unwrap(), b"aHid!!!h");
        assert!(ips(b"", b"PATCH\0\0", &flag()).is_err());
    }
    #[test]
    fn ips32_and_limits() {
        assert_eq!(
            ips(b"abc", b"IPS32\0\0\0\x01\0\x01XEEOF", &flag()).unwrap(),
            b"aXc"
        );
        assert!(ips(b"", b"IPS32\xff\xff\xff\xff\0\x01XEEOF", &flag()).is_err());
    }
    #[test]
    fn bps_all_operations_and_checksums() {
        let source = b"abcdef";
        let target = b"abXYefefef";
        let mut p = b"BPS1".to_vec();
        var(6, &mut p);
        var(10, &mut p);
        var(0, &mut p);
        var(4, &mut p); // source read ab
        var(5, &mut p);
        p.extend(b"XY");
        var(6, &mut p);
        var(8, &mut p); // source copy ef
        var(15, &mut p);
        var(8, &mut p); // overlapping target copy efef
        footer(&mut p, source, target);
        assert_eq!(bps(source, &p, &flag()).unwrap(), target);
        assert!(bps(b"wrong!", &p, &flag()).is_err());
        p[10] ^= 1;
        assert!(bps(source, &p, &flag()).is_err());
    }
    #[test]
    fn ups_checksums_and_xor() {
        let mut p = b"UPS1".to_vec();
        var(3, &mut p);
        var(3, &mut p);
        var(1, &mut p);
        p.extend([b'b' ^ b'X', 0]);
        footer(&mut p, b"abc", b"aXc");
        assert_eq!(ups(b"abc", &p, &flag()).unwrap(), b"aXc");
        assert!(ups(b"xyz", &p, &flag()).is_err());
    }
    #[test]
    fn ppf_streams_and_preserves_original() {
        let dir = tempfile::tempdir().unwrap();
        let source = dir.path().join("disc.iso");
        let patch = dir.path().join("mod.ppf");
        let out = dir.path().join("result.iso");
        fs::write(&source, b"abcdefgh").unwrap();
        let mut p = b"PPF30\x02".to_vec();
        p.extend([0; 50]);
        p.extend([0, 0, 1, 0]);
        p.extend(2u64.to_le_bytes());
        p.push(2);
        p.extend(b"XYcd");
        fs::write(&patch, &p).unwrap();
        apply(&source, &patch, &out, &flag()).unwrap();
        assert_eq!(fs::read(&source).unwrap(), b"abcdefgh");
        assert_eq!(fs::read(&out).unwrap(), b"abXYefgh");
        assert!(apply(&source, &patch, &out, &flag()).is_err());
    }
    #[test]
    #[ignore = "requires xdelta3 on PATH"]
    fn xdelta_stream_round_trip() {
        let dir = tempfile::tempdir().unwrap();
        let source = dir.path().join("source.iso");
        let modified = dir.path().join("modified.iso");
        let patch = dir.path().join("change.xdelta");
        let output = dir.path().join("patched.iso");
        let original = vec![42; 1024 * 1024];
        let mut target = original.clone();
        target[150..160].fill(9);
        fs::write(&source, &original).unwrap();
        fs::write(&modified, &target).unwrap();
        assert!(
            std::process::Command::new("xdelta3")
                .env_remove("XDELTA")
                .args(["-e", "-s"])
                .arg(&source)
                .arg(&modified)
                .arg(&patch)
                .status()
                .unwrap()
                .success()
        );
        apply(&source, &patch, &output, &flag()).unwrap();
        assert_eq!(fs::read(output).unwrap(), target);
        assert_eq!(fs::read(source).unwrap(), original);
    }
    #[test]
    fn malformed_and_cancelled_patches_fail_without_output() {
        let dir = tempfile::tempdir().unwrap();
        let s = dir.path().join("s");
        let p = dir.path().join("p");
        let o = dir.path().join("o");
        fs::write(&s, b"abc").unwrap();
        fs::write(&p, b"PATCH\0\0").unwrap();
        assert!(apply(&s, &p, &o, &flag()).is_err());
        assert!(!o.exists());
        assert!(apply(&s, &p, &o, &AtomicBool::new(true)).is_err());
        assert!(!o.exists());
    }
}
