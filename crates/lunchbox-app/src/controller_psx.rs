//! Beetle PSX's requested/effective controller distinction, with read-only
//! content identity. No title matching or broad compatibility-option disabling.
//! Functional contract: beetle-psx-libretro 56f4732070835bb81078dd8ecab7246e203612a1.
use crate::emulator::PreparedRetroarchContent;
use anyhow::{Context, Result, bail, ensure};
use sha2::{Digest, Sha256};
use std::collections::BTreeSet;
use std::ffi::OsString;
use std::fs::File;
use std::io::{Read, Seek, SeekFrom};
use std::path::{Path, PathBuf};

mod chd;

pub fn validate_arguments(
    arguments: &[OsString],
    prepared: &PreparedRetroarchContent,
) -> Result<()> {
    ensure!(
        prepared.core.is_absolute() && prepared.content.is_absolute(),
        "PlayStation launch identity requires absolute prepared paths"
    );
    let mut core = false;
    let mut content = false;
    let mut index = 0;
    while index < arguments.len() {
        let argument = &arguments[index];
        match argument.to_str() {
            Some("--verbose" | "-v" | "--fullscreen" | "-f") => {}
            Some("-L" | "--libretro") => {
                index += 1;
                ensure!(
                    !core
                        && arguments
                            .get(index)
                            .is_some_and(|path| path == prepared.core.as_os_str()),
                    "Custom command selects a different or duplicate PlayStation core"
                );
                core = true;
            }
            Some("--device" | "-d" | "--nodevice" | "-N" | "--dualanalog" | "-A") => {
                index += 1;
                ensure!(index < arguments.len(), "Missing controller-mode argument");
            }
            Some("--") => {
                ensure!(
                    !content
                        && arguments.len() == index + 2
                        && arguments[index + 1] == prepared.content.as_os_str(),
                    "Custom command changes PlayStation content"
                );
                content = true;
                break;
            }
            Some(text) if text.starts_with("--libretro=") || text.starts_with("-L") => {
                let path = text
                    .strip_prefix("--libretro=")
                    .or_else(|| text.strip_prefix("-L"))
                    .unwrap();
                ensure!(
                    !core && Path::new(path) == prepared.core,
                    "Custom command selects a different or duplicate PlayStation core"
                );
                core = true;
            }
            Some(text)
                if [
                    "--device=",
                    "--nodevice=",
                    "--dualanalog=",
                    "-d",
                    "-N",
                    "-A",
                ]
                .iter()
                .any(|prefix| text.starts_with(prefix) && text.len() > prefix.len()) => {}
            _ => {
                ensure!(
                    !content && argument == prepared.content.as_os_str(),
                    "Custom PlayStation arguments need content-identity resolution before calibrated launch"
                );
                content = true;
            }
        }
        index += 1;
    }
    ensure!(
        core && content,
        "Custom command omits the prepared PlayStation core or content"
    );
    Ok(())
}

fn compatibility_enabled(options: &str, core: &str) -> Result<bool> {
    let key = match core {
        "mednafen_psx" => "beetle_psx_compatibility_settings",
        "mednafen_psx_hw" => "beetle_psx_hw_compatibility_settings",
        _ => bail!("Not a Beetle PSX core"),
    };
    let mut result = None;
    for line in options.lines().map(str::trim) {
        ensure!(
            !line.starts_with("#include"),
            "Included core options need compatibility resolution"
        );
        let Some((name, value)) = line.split_once('=') else {
            continue;
        };
        if name.trim() != key {
            continue;
        }
        ensure!(
            name.ends_with(|c: char| c.is_ascii_whitespace()),
            "RetroArch requires whitespace before '=' in {key}"
        );
        ensure!(
            result.is_none(),
            "Duplicate PlayStation compatibility option"
        );
        let value = value.trim();
        let value = if let Some(quoted) = value.strip_prefix('"') {
            let (value, rest) = quoted
                .split_once('"')
                .context("Unterminated compatibility option")?;
            ensure!(
                rest.trim().is_empty() || rest.trim_start().starts_with('#'),
                "Invalid compatibility option suffix"
            );
            value
        } else {
            value.split('#').next().unwrap().trim()
        };
        result = Some(match value {
            "enabled" => true,
            "disabled" => false,
            _ => bail!("Unknown PlayStation compatibility option value"),
        });
    }
    Ok(result.unwrap_or(true))
}

fn verify_compatibility_build(path: &Path, core: &str) -> Result<()> {
    // These official buildbot binaries have been tied to source revision
    // 56f4732. A base version number cannot distinguish older behavior.
    let expected = match core {
        "mednafen_psx" => "767bb60bd96d3f19806a9311d96638c9ca39272d1236035a752952bb4b4c1968",
        "mednafen_psx_hw" => "54d48450def74a79669e883aebaaffd3218a1165ee51b6bef6c45e3ec5e11222",
        _ => bail!("Not a Beetle PSX core"),
    };
    let file = File::open(path)?;
    ensure!(
        file.metadata()?.len() <= 512 * 1024 * 1024,
        "PlayStation core is too large to verify"
    );
    let mut reader = file.take(512 * 1024 * 1024 + 1);
    let mut hash = Sha256::new();
    let mut buffer = [0; 64 * 1024];
    loop {
        let count = reader.read(&mut buffer)?;
        if count == 0 {
            break;
        }
        hash.update(&buffer[..count]);
    }
    ensure!(
        hex::encode(hash.finalize()) == expected,
        "This Beetle PSX build's automatic controller overrides have not been verified yet; its native configuration has not been changed"
    );
    Ok(())
}

pub fn launch_binding_modes(
    prepared: &PreparedRetroarchContent,
    core: &str,
    requested: &[u32],
    options: &str,
) -> Result<Vec<u32>> {
    let compatibility = compatibility_enabled(options, core)?;
    if compatibility && requested.first().is_some_and(|mode| *mode != 0) {
        verify_compatibility_build(&prepared.core, core)?;
    }
    binding_modes(&prepared.content, requested, compatibility)
}

trait CookedSectorReader {
    fn sector(&mut self, lba: u32) -> Result<[u8; 2048]>;
}

fn sector_edc(bytes: &[u8]) -> u32 {
    let mut crc = 0u32;
    for byte in bytes {
        crc ^= u32::from(*byte);
        for _ in 0..8 {
            crc = (crc >> 1) ^ if crc & 1 != 0 { 0xd801_8001 } else { 0 };
        }
    }
    crc
}

fn cooked_raw(bytes: &[u8]) -> Result<[u8; 2048]> {
    ensure!(bytes.len() == 2352, "Invalid raw CD sector size");
    let (payload, checked_start, edc_at) = match bytes[15] {
        1 => (16, 0, 2064),
        2 => (24, 16, 2072),
        _ => bail!("Raw CD sector is not Mode 1 or Mode 2 data"),
    };
    ensure!(
        sector_edc(&bytes[checked_start..edc_at]) == le32(bytes, edc_at),
        "CD sector requires error correction before controller identity can be verified"
    );
    Ok(bytes[payload..payload + 2048].try_into().unwrap())
}

/// Bind a superset if discs in one launch can use different effective modes.
/// The caller must still write the original requested device IDs to RetroArch.
pub fn binding_modes(content: &Path, requested: &[u32], compatibility: bool) -> Result<Vec<u32>> {
    ensure!(
        requested.len() == 2 && requested.iter().all(|m| matches!(m, 0 | 1 | 517)),
        "Unsupported Beetle PSX port topology or device mode"
    );
    let mut result = requested.to_vec();
    if !compatibility || requested[0] == 0 {
        return Ok(result);
    }
    let serials = content_serials(content, &mut BTreeSet::new(), 0)?;
    ensure!(!serials.is_empty(), "Empty PlayStation launch media");
    result[0] = if serials
        .iter()
        .any(|serial| effective_mode(requested[0], serial.as_deref()) == 517)
    {
        517
    } else {
        1
    };
    Ok(result)
}

fn effective_mode(requested: u32, serial: Option<&str>) -> u32 {
    match serial {
        Some("SCUS-94900") => 1,
        Some("SCUS-9418A") => 517,
        _ => requested,
    }
}

fn small_text(path: &Path) -> Result<String> {
    let file = File::open(path)?;
    ensure!(
        file.metadata()?.len() <= 1024 * 1024,
        "Controller identity control file is too large"
    );
    let mut text = String::new();
    file.take(1024 * 1024 + 1).read_to_string(&mut text)?;
    ensure!(
        text.len() <= 1024 * 1024,
        "Controller identity control file grew while reading"
    );
    Ok(text)
}

fn content_serials(
    path: &Path,
    visiting: &mut BTreeSet<PathBuf>,
    depth: usize,
) -> Result<Vec<Option<String>>> {
    ensure!(depth < 16, "PlayStation playlist nesting is too deep");
    let path = path
        .canonicalize()
        .with_context(|| format!("Resolving controller identity from {}", path.display()))?;
    ensure!(visiting.insert(path.clone()), "Cyclic PlayStation playlist");
    let extension = path
        .extension()
        .and_then(|e| e.to_str())
        .unwrap_or_default()
        .to_ascii_lowercase();
    let result = match extension.as_str() {
        "m3u" => {
            let mut result = Vec::new();
            for line in small_text(&path)?
                .trim_start_matches('\u{feff}')
                .lines()
                .map(str::trim)
            {
                if line.is_empty() || line.starts_with('#') {
                    continue;
                }
                let child = path.parent().context("Playlist has no parent")?.join(line);
                result.extend(content_serials(&child, visiting, depth + 1)?);
                ensure!(result.len() <= 100, "Too many PlayStation playlist discs");
            }
            Ok(result)
        }
        "cue" => read_iso_serial(&mut cue_data(&path)?).map(|serial| vec![serial]),
        "chd" => read_iso_serial(&mut chd::ChdDisc::open(&path)?).map(|serial| vec![serial]),
        "exe" => {
            let mut file = File::open(&path)?;
            let mut magic = [0; 8];
            file.read_exact(&mut magic)?;
            ensure!(
                &magic == b"PS-X EXE" && file.metadata()?.len() >= 0x800,
                "Invalid PlayStation executable"
            );
            Ok(vec![None]) // The core has no disc compatibility record for EXEs.
        }
        _ => bail!(
            "Beetle controller compatibility requires verified disc identity; {extension} media identity is not implemented yet (compatibility fixes have not been disabled)"
        ),
    };
    visiting.remove(&path);
    result
}

struct DataTrack {
    file: File,
    first: u64,
    end: u64,
    stride: u64,
    payload: u64,
}

impl CookedSectorReader for DataTrack {
    fn sector(&mut self, lba: u32) -> Result<[u8; 2048]> {
        let position = self
            .first
            .checked_add(u64::from(lba))
            .context("CD sector overflow")?;
        ensure!(
            position < self.end,
            "ISO directory points outside the data track"
        );
        let offset = position
            .checked_mul(self.stride)
            .context("CD file offset overflow")?;
        if self.stride == 2352 {
            let mut raw = [0; 2352];
            self.file.seek(SeekFrom::Start(offset))?;
            self.file.read_exact(&mut raw)?;
            return cooked_raw(&raw);
        }
        let offset = offset
            .checked_add(self.payload)
            .context("CD file offset overflow")?;
        let mut bytes = [0; 2048];
        self.file.seek(SeekFrom::Start(offset))?;
        self.file.read_exact(&mut bytes)?;
        Ok(bytes)
    }
}

fn cue_fields(line: &str) -> Result<Vec<String>> {
    let mut words = Vec::new();
    let mut word = String::new();
    let mut quoted = false;
    for c in line.chars() {
        if c == '"' {
            quoted = !quoted;
        } else if c.is_ascii_whitespace() && !quoted {
            if !word.is_empty() {
                words.push(std::mem::take(&mut word));
            }
        } else {
            word.push(c);
        }
    }
    ensure!(!quoted, "Unclosed CUE filename quote");
    if !word.is_empty() {
        words.push(word);
    }
    Ok(words)
}

fn cue_frame(value: &str) -> Result<u64> {
    let fields = value
        .split(':')
        .map(str::parse::<u64>)
        .collect::<std::result::Result<Vec<_>, _>>()?;
    ensure!(
        fields.len() == 3 && fields[1] < 60 && fields[2] < 75,
        "Invalid CUE timestamp"
    );
    fields[0]
        .checked_mul(60)
        .and_then(|v| v.checked_add(fields[1]))
        .and_then(|v| v.checked_mul(75))
        .and_then(|v| v.checked_add(fields[2]))
        .context("CUE timestamp overflow")
}

fn cue_data(path: &Path) -> Result<DataTrack> {
    struct Track {
        file: PathBuf,
        mode: String,
        zero: Option<u64>,
        one: Option<u64>,
    }
    let mut tracks: Vec<Track> = Vec::new();
    let mut current_file = None;
    let mut awaiting_track = false;
    for line in small_text(path)?.lines() {
        let words = cue_fields(line)?;
        let Some(command) = words.first() else {
            continue;
        };
        match command.to_ascii_uppercase().as_str() {
            "FILE" => {
                ensure!(words.len() == 3, "Malformed CUE FILE");
                current_file = Some((
                    path.parent()
                        .context("CUE has no parent")?
                        .join(&words[1])
                        .canonicalize()?,
                    words[2].clone(),
                ));
                awaiting_track = true;
            }
            "TRACK" => {
                ensure!(words.len() == 3 && tracks.len() < 99, "Malformed CUE TRACK");
                let number = words[1].parse::<usize>()?;
                ensure!(
                    number == tracks.len() + 1,
                    "CUE tracks must begin at 1 and be consecutive"
                );
                let (file, kind) = current_file.as_ref().context("CUE TRACK precedes FILE")?;
                if tracks.is_empty() {
                    ensure!(
                        kind.eq_ignore_ascii_case("BINARY"),
                        "First CUE track is not binary data"
                    );
                }
                tracks.push(Track {
                    file: file.clone(),
                    mode: words[2].to_ascii_uppercase(),
                    zero: None,
                    one: None,
                });
                awaiting_track = false;
            }
            "INDEX" => {
                ensure!(words.len() == 3, "Malformed CUE INDEX");
                let track = tracks.last_mut().context("CUE INDEX precedes TRACK")?;
                ensure!(!awaiting_track, "CUE INDEX follows FILE without a TRACK");
                let slot = match words[1].as_str() {
                    "00" => &mut track.zero,
                    "01" => &mut track.one,
                    _ => continue,
                };
                ensure!(slot.is_none(), "Duplicate CUE index");
                *slot = Some(cue_frame(&words[2])?);
            }
            _ => {}
        }
    }
    let first = tracks.first().context("CUE has no tracks")?;
    let (stride, payload) = match first.mode.as_str() {
        "MODE1/2048" => (2048, 0),
        "MODE1/2352" => (2352, 16),
        "MODE2/2352" => (2352, 24),
        _ => bail!("Unsupported first CUE data track mode"),
    };
    let file = File::open(&first.file)?;
    let start = first.one.context("First CUE track has no INDEX 01")?;
    let end = if let Some(next) = tracks.get(1).filter(|next| next.file == first.file) {
        ensure!(
            stride == 2352 || next.mode == "MODE1/2048",
            "Mixed CUE sector sizes need a byte-accurate track adapter"
        );
        next.zero
            .or(next.one)
            .context("Second CUE track has no index")?
    } else {
        file.metadata()?.len() / stride
    };
    ensure!(
        start < end
            && end
                .checked_mul(stride)
                .is_some_and(|end| end <= file.metadata().map(|m| m.len()).unwrap_or(0)),
        "CUE data track extends outside its file"
    );
    Ok(DataTrack {
        file,
        first: start,
        end,
        stride,
        payload,
    })
}

fn le32(bytes: &[u8], at: usize) -> u32 {
    u32::from_le_bytes(bytes[at..at + 4].try_into().unwrap())
}

fn read_iso_serial(track: &mut impl CookedSectorReader) -> Result<Option<String>> {
    let mut primary = None;
    for lba in 16..48 {
        let sector = track.sector(lba)?;
        if &sector[1..6] != b"CD001" || sector[0] == 255 {
            return Ok(None);
        }
        if sector[0] == 1 {
            primary = Some(sector);
            break;
        }
    }
    let Some(primary) = primary else {
        return Ok(None);
    };
    let root = le32(&primary, 158);
    let length = le32(&primary, 166);
    ensure!(
        length < 10 * 1024 * 1024,
        "PlayStation root directory is too large"
    );
    let mut offset = 0u32;
    let mut cached = None;
    let mut sector = [0; 2048];
    while offset < length {
        let lba = root
            .checked_add(offset / 2048)
            .context("Root directory sector overflow")?;
        if cached != Some(lba) {
            sector = track.sector(lba)?;
            cached = Some(lba);
        }
        let at = (offset % 2048) as usize;
        let record_len = usize::from(sector[at]);
        // Matches the pinned core's stop-on-zero behavior, not a title heuristic.
        if record_len == 0 {
            break;
        }
        ensure!(
            record_len >= 34 && at + record_len <= 2048 && offset + record_len as u32 <= length,
            "Malformed PlayStation ISO directory record"
        );
        let record = &sector[at..at + record_len];
        let name_len = usize::from(record[32]);
        ensure!(33 + name_len <= record_len, "Truncated ISO filename");
        if &record[33..33 + name_len] == b"SYSTEM.CNF;1" {
            return Ok(boot_serial(&track.sector(le32(record, 2))?));
        }
        offset += record_len as u32;
    }
    Ok(None)
}

fn boot_serial(bytes: &[u8]) -> Option<String> {
    let text = bytes.split(|b| *b == 0).next()?;
    let start = text.windows(4).position(|s| s == b"BOOT")? + 4;
    // SYSTEM.CNF is a byte buffer in the core. Bytes after the boot filename
    // need not be UTF-8 and must not change an otherwise recognized identity.
    let whitespace = |b: &u8| matches!(b, b' ' | b'\t');
    let value = &text[start..];
    let skipped = value.iter().take_while(|b| whitespace(b)).count();
    let value = &value[skipped..];
    let value = value.strip_prefix(b"=")?;
    let skipped = value.iter().take_while(|b| whitespace(b)).count();
    let value = &value[skipped..];
    let prefix = value.get(..7)?;
    if !prefix.eq_ignore_ascii_case(b"cdrom:\\") {
        return None;
    }
    let serial = value.get(7..)?;
    let suffix = if serial.get(8) == Some(&b'.') { 9 } else { 8 };
    if serial.len() < suffix + 2
        || !serial[..4].iter().all(u8::is_ascii_alphabetic)
        || !matches!(serial[4], b'_' | b'-')
        || !serial[5..8].iter().all(u8::is_ascii_digit)
        || !serial[suffix].is_ascii_digit()
        || !serial[suffix + 1].is_ascii_alphanumeric()
    {
        return None;
    }
    Some(format!(
        "{}-{}{}{}",
        std::str::from_utf8(&serial[..4]).ok()?.to_ascii_uppercase(),
        std::str::from_utf8(&serial[5..8]).ok()?,
        serial[suffix] as char,
        (serial[suffix + 1] as char).to_ascii_uppercase()
    ))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn final_arguments_must_retain_the_prepared_core_and_content() {
        let prepared = PreparedRetroarchContent {
            core: "/cores/beetle.so".into(),
            content: "/cache/prepared.m3u".into(),
        };
        for values in [
            vec![
                "--verbose",
                "-L",
                "/cores/beetle.so",
                "--device",
                "1:517",
                "/cache/prepared.m3u",
            ],
            vec![
                "--libretro=/cores/beetle.so",
                "-N2",
                "--",
                "/cache/prepared.m3u",
            ],
        ] {
            let args = values.iter().map(OsString::from).collect::<Vec<_>>();
            validate_arguments(&args, &prepared).unwrap();
        }
        for values in [
            vec!["-L", "/cores/other.so", "/cache/prepared.m3u"],
            vec!["-L", "/cores/beetle.so", "/original/game.zip"],
            vec![
                "-L",
                "/cores/beetle.so",
                "/cache/prepared.m3u",
                "/other/disc.chd",
            ],
            vec![
                "-L",
                "/cores/beetle.so",
                "--subsystem",
                "other",
                "/cache/prepared.m3u",
            ],
            vec!["--", "/cache/prepared.m3u"],
        ] {
            let args = values.iter().map(OsString::from).collect::<Vec<_>>();
            assert!(validate_arguments(&args, &prepared).is_err(), "{values:?}");
        }
    }

    #[test]
    fn compatibility_option_is_exact_and_never_rewritten_for_convenience() {
        assert!(compatibility_enabled("", "mednafen_psx").unwrap());
        assert!(
            !compatibility_enabled(
                "beetle_psx_compatibility_settings = \"disabled\" # user choice",
                "mednafen_psx"
            )
            .unwrap()
        );
        assert!(
            compatibility_enabled(
                "beetle_psx_compatibility_settings = \"disabled\"",
                "mednafen_psx_hw"
            )
            .unwrap()
        );
        for value in [
            "#include \"other.opt\"",
            "beetle_psx_compatibility_settings = enabled\nbeetle_psx_compatibility_settings = disabled",
            "beetle_psx_compatibility_settings = maybe",
            "beetle_psx_compatibility_settings=disabled",
        ] {
            assert!(compatibility_enabled(value, "mednafen_psx").is_err());
        }
        let directory = tempfile::tempdir().unwrap();
        let core = directory.path().join("unverified.so");
        std::fs::write(
            &core,
            b"Not executable; identity check must not load native code",
        )
        .unwrap();
        assert!(verify_compatibility_build(&core, "mednafen_psx").is_err());
        let prepared = PreparedRetroarchContent {
            core,
            content: directory.path().join("unneeded.chd"),
        };
        assert_eq!(
            launch_binding_modes(
                &prepared,
                "mednafen_psx",
                &[1, 517],
                "beetle_psx_compatibility_settings = disabled"
            )
            .unwrap(),
            [1, 517]
        );
        assert!(launch_binding_modes(&prepared, "mednafen_psx", &[1, 517], "").is_err());
    }

    #[test]
    #[ignore = "Requires the vetted core at LUNCHBOX_BEETLE_PSX_CORE"]
    fn vetted_core_and_prepared_disc_resolve_launch_binding_modes() {
        let core = PathBuf::from(
            std::env::var_os("LUNCHBOX_BEETLE_PSX_CORE").expect("Set LUNCHBOX_BEETLE_PSX_CORE"),
        );
        let directory = tempfile::tempdir().unwrap();
        let digital = disc(directory.path(), "digital", "SCUS_949.00", "MODE2/2352", 0);
        let analog = disc(directory.path(), "analog", "SCUS_941.8A", "MODE2/2352", 0);
        let prepared = PreparedRetroarchContent {
            core,
            content: digital,
        };
        assert_eq!(
            launch_binding_modes(&prepared, "mednafen_psx", &[517, 1], "").unwrap(),
            [1, 1]
        );
        assert_eq!(
            launch_binding_modes(
                &PreparedRetroarchContent {
                    content: analog,
                    ..prepared
                },
                "mednafen_psx",
                &[1, 517],
                ""
            )
            .unwrap(),
            [517, 517]
        );
    }

    // Original minimal ISO directory fixtures, not redistributed game data.
    fn disc(directory: &Path, name: &str, serial: &str, mode: &str, pregap: usize) -> PathBuf {
        let (stride, payload) = match mode {
            "MODE1/2048" => (2048, 0),
            "MODE1/2352" => (2352, 16),
            "MODE2/2352" => (2352, 24),
            _ => unreachable!(),
        };
        let mut bytes = vec![0u8; (pregap + 32) * stride];
        let pvd = (pregap + 16) * stride + payload;
        bytes[pvd] = 1;
        bytes[pvd + 1..pvd + 6].copy_from_slice(b"CD001");
        bytes[pvd + 158..pvd + 162].copy_from_slice(&20u32.to_le_bytes());
        bytes[pvd + 166..pvd + 170].copy_from_slice(&2048u32.to_le_bytes());
        let root = (pregap + 20) * stride + payload;
        bytes[root] = 46;
        bytes[root + 2..root + 6].copy_from_slice(&21u32.to_le_bytes());
        bytes[root + 32] = 12;
        bytes[root + 33..root + 45].copy_from_slice(b"SYSTEM.CNF;1");
        let cnf = (pregap + 21) * stride + payload;
        let boot = format!("BOOT = cdrom:\\{serial};1\r\n");
        bytes[cnf..cnf + boot.len()].copy_from_slice(boot.as_bytes());
        if stride == 2352 {
            for raw in bytes.chunks_exact_mut(2352) {
                raw[1..11].fill(255);
                raw[15] = if mode == "MODE1/2352" { 1 } else { 2 };
                let (begin, end) = if raw[15] == 1 { (0, 2064) } else { (16, 2072) };
                let crc = sector_edc(&raw[begin..end]);
                raw[end..end + 4].copy_from_slice(&crc.to_le_bytes());
            }
        }
        std::fs::write(directory.join(format!("{name}.bin")), bytes).unwrap();
        let cue = directory.join(format!("{name}.cue"));
        std::fs::write(
            &cue,
            format!(
                "FILE \"{name}.bin\" BINARY\n TRACK 01 {mode}\n INDEX 01 {:02}:{:02}:{:02}\n",
                pregap / 4500,
                pregap / 75 % 60,
                pregap % 75
            ),
        )
        .unwrap();
        cue
    }

    #[test]
    fn cue_sector_modes_and_index_offsets_match_iso_identity() {
        let directory = tempfile::tempdir().unwrap();
        for mode in ["MODE1/2048", "MODE1/2352", "MODE2/2352"] {
            for pregap in [0, 150] {
                let cue = disc(
                    directory.path(),
                    "disc with spaces",
                    "SCUS_949.00",
                    mode,
                    pregap,
                );
                assert_eq!(
                    binding_modes(&cue, &[517, 517], true).unwrap(),
                    [1, 517],
                    "{mode}, {pregap}"
                );
                assert_eq!(binding_modes(&cue, &[517, 1], false).unwrap(), [517, 1]);
            }
        }
    }

    #[test]
    fn raw_sector_mode_and_edc_are_verified_before_identity() {
        let directory = tempfile::tempdir().unwrap();
        let cue = disc(directory.path(), "disc", "SCUS_949.00", "MODE1/2352", 0);
        // Raw-sector mode wins over the track label, as in CDIF_ReadSector.
        let text = std::fs::read_to_string(&cue)
            .unwrap()
            .replace("MODE1/2352", "MODE2/2352");
        std::fs::write(&cue, text).unwrap();
        assert_eq!(binding_modes(&cue, &[517, 1], true).unwrap(), [1, 1]);
        let bin = directory.path().join("disc.bin");
        let mut bytes = std::fs::read(&bin).unwrap();
        bytes[21 * 2352 + 16 + 25] ^= 1;
        std::fs::write(&bin, bytes).unwrap();
        assert!(
            binding_modes(&cue, &[517, 1], true)
                .unwrap_err()
                .to_string()
                .contains("error correction")
        );
    }

    #[test]
    #[ignore = "Requires LUNCHBOX_CHDMAN pointing to a real chdman executable"]
    fn chdman_compressed_and_raw_discs_share_cue_identity() {
        let tool = std::env::var_os("LUNCHBOX_CHDMAN").expect("Set LUNCHBOX_CHDMAN");
        let directory = tempfile::tempdir().unwrap();
        let cue = disc(directory.path(), "disc", "SCUS_949.00", "MODE2/2352", 0);
        // An odd-sized first track exercises CHD's four-frame storage padding.
        let bin = directory.path().join("disc.bin");
        let mut data = std::fs::read(&bin).unwrap();
        data.extend_from_slice(&[0; 2352]);
        std::fs::write(&bin, data).unwrap();
        std::fs::write(directory.path().join("audio.bin"), vec![0u8; 5 * 2352]).unwrap();
        let mut cuesheet = std::fs::read_to_string(&cue).unwrap();
        cuesheet.push_str("FILE audio.bin BINARY\n TRACK 02 AUDIO\n INDEX 01 00:00:00\n");
        std::fs::write(&cue, cuesheet).unwrap();
        for (name, compression) in [
            ("zlib", "cdzl"),
            ("lzma", "cdlz"),
            ("flac", "cdfl"),
            ("zstd", "cdzs"),
            ("raw", "none"),
        ] {
            let output = directory.path().join(format!("{name}.chd"));
            let result = std::process::Command::new(&tool)
                .args(["createcd", "-i"])
                .arg(&cue)
                .arg("-o")
                .arg(&output)
                .args(["-c", compression])
                .output()
                .unwrap();
            assert!(
                result.status.success(),
                "{}\n{}",
                String::from_utf8_lossy(&result.stdout),
                String::from_utf8_lossy(&result.stderr)
            );
            assert_eq!(binding_modes(&output, &[517, 1], true).unwrap(), [1, 1]);
            let mut reader = chd::ChdDisc::open(&output).unwrap();
            assert_eq!(
                reader.sector(16).unwrap(),
                cue_data(&cue).unwrap().sector(16).unwrap()
            );
            assert!(reader.sector(33).is_err(), "Audio track is not ISO data");
            if compression == "cdzl" {
                let child = directory.path().join("child.chd");
                let result = std::process::Command::new(&tool)
                    .args(["createcd", "-i"])
                    .arg(&cue)
                    .arg("-o")
                    .arg(&child)
                    .arg("-op")
                    .arg(&output)
                    .args(["-c", "cdzl"])
                    .output()
                    .unwrap();
                assert!(
                    result.status.success(),
                    "{}\n{}",
                    String::from_utf8_lossy(&result.stdout),
                    String::from_utf8_lossy(&result.stderr)
                );
                assert_eq!(binding_modes(&child, &[517, 1], true).unwrap(), [1, 1]);
                let hidden = directory.path().join("parent.saved");
                std::fs::rename(&output, &hidden).unwrap();
                assert!(
                    binding_modes(&child, &[517, 1], true)
                        .unwrap_err()
                        .to_string()
                        .contains("parent CHD")
                );
                #[cfg(unix)]
                {
                    let linked = directory.path().join("linked-parent.chd");
                    let alias = directory.path().join("alias-parent.chd");
                    std::os::unix::fs::symlink(&hidden, &linked).unwrap();
                    std::os::unix::fs::symlink(&hidden, &alias).unwrap();
                    assert_eq!(binding_modes(&child, &[517, 1], true).unwrap(), [1, 1]);
                    std::fs::remove_file(linked).unwrap();
                    std::fs::remove_file(alias).unwrap();
                }
                std::fs::rename(&hidden, &output).unwrap();
                // Exclude the child from the next fixture's parent search.
                std::fs::rename(child, directory.path().join("child.saved")).unwrap();
            }
        }
    }

    #[test]
    fn playlist_unions_effective_bindings_without_changing_port_two() {
        let directory = tempfile::tempdir().unwrap();
        disc(directory.path(), "digital", "SCUS_949.00", "MODE2/2352", 0);
        disc(directory.path(), "analog", "SCUS_941.8A", "MODE1/2048", 0);
        let list = directory.path().join("game.m3u");
        std::fs::write(&list, "# mixed modes\ndigital.cue\nanalog.cue\n").unwrap();
        assert_eq!(binding_modes(&list, &[1, 1], true).unwrap(), [517, 1]);
        assert_eq!(binding_modes(&list, &[517, 0], true).unwrap(), [517, 0]);
        std::fs::write(
            directory.path().join("nested.m3u"),
            "game.m3u\ndigital.cue\n",
        )
        .unwrap();
        assert_eq!(
            binding_modes(&directory.path().join("nested.m3u"), &[1, 1], true).unwrap(),
            [517, 1]
        );
        std::fs::write(&list, "game.m3u\n").unwrap();
        assert!(
            binding_modes(&list, &[1, 1], true)
                .unwrap_err()
                .to_string()
                .contains("Cyclic")
        );
        std::fs::write(&list, "# empty\n").unwrap();
        assert!(binding_modes(&list, &[1, 1], true).is_err());
    }

    #[test]
    fn exe_has_no_disc_override_and_invalid_media_is_not_guessed() {
        let directory = tempfile::tempdir().unwrap();
        let exe = directory.path().join("SCUS-94900.exe");
        let mut bytes = vec![0; 2048];
        bytes[..8].copy_from_slice(b"PS-X EXE");
        std::fs::write(&exe, bytes).unwrap();
        assert_eq!(binding_modes(&exe, &[517, 1], true).unwrap(), [517, 1]);
        std::fs::write(&exe, b"PS-X EXE").unwrap();
        assert!(binding_modes(&exe, &[1, 1], true).is_err());
        assert!(binding_modes(&exe, &[261, 1], false).is_err());
        assert!(binding_modes(&exe, &[1], false).is_err());
    }

    #[test]
    fn iso_walk_stops_at_zero_like_the_core_and_rejects_bad_extents() {
        let directory = tempfile::tempdir().unwrap();
        let cue = disc(directory.path(), "disc", "SCUS_949.00", "MODE1/2048", 0);
        let bin = directory.path().join("disc.bin");
        let original = std::fs::read(&bin).unwrap();
        let mut bytes = original.clone();
        // A valid-looking entry later in the root must not be found after zero.
        bytes[20 * 2048] = 0;
        std::fs::write(&bin, &bytes).unwrap();
        assert_eq!(binding_modes(&cue, &[517, 1], true).unwrap(), [517, 1]);
        bytes = original.clone();
        bytes[20 * 2048 + 2..20 * 2048 + 6].copy_from_slice(&32u32.to_le_bytes());
        std::fs::write(&bin, &bytes).unwrap();
        assert!(binding_modes(&cue, &[517, 1], true).is_err());
        bytes = original;
        bytes[20 * 2048 + 32] = 200;
        std::fs::write(&bin, &bytes).unwrap();
        assert!(binding_modes(&cue, &[517, 1], true).is_err());
    }

    #[test]
    fn cue_cannot_read_identity_from_following_track_or_unowned_index() {
        let directory = tempfile::tempdir().unwrap();
        let cue = disc(directory.path(), "disc", "SCUS_949.00", "MODE2/2352", 0);
        let original = std::fs::read_to_string(&cue).unwrap();
        std::fs::write(
            &cue,
            format!("{original}TRACK 02 AUDIO\nINDEX 00 00:00:21\nINDEX 01 00:00:22\n"),
        )
        .unwrap();
        assert!(binding_modes(&cue, &[517, 1], true).is_err());
        std::fs::write(
            &cue,
            "FILE disc.bin BINARY\nTRACK 01 MODE2/2352\nFILE disc.bin BINARY\nINDEX 01 00:00:00\n",
        )
        .unwrap();
        assert!(
            binding_modes(&cue, &[517, 1], true)
                .unwrap_err()
                .to_string()
                .contains("without a TRACK")
        );
    }
    #[test]
    fn compatibility_preserves_request_except_exact_core_records() {
        assert_eq!(effective_mode(517, Some("SCUS-94900")), 1);
        assert_eq!(effective_mode(1, Some("SCUS-9418A")), 517);
        assert_eq!(effective_mode(517, Some("SCUS-9418A")), 517);
        assert_eq!(effective_mode(1, Some("SCUS-94900")), 1);
        assert_eq!(effective_mode(1, Some("SCUS-94180")), 1);
        assert_eq!(effective_mode(517, None), 517);
        assert_eq!(
            binding_modes(Path::new("/missing.chd"), &[1, 517], false).unwrap(),
            [1, 517]
        );
        assert_eq!(
            binding_modes(Path::new("/missing.chd"), &[0, 517], true).unwrap(),
            [0, 517]
        );
    }
    #[test]
    fn boot_identity_comes_from_exact_system_cnf_syntax() {
        assert_eq!(
            boot_serial(b"BOOT = cdrom:\\scus_949.00;1\r\n"),
            Some("SCUS-94900".into())
        );
        assert_eq!(
            boot_serial(b"BOOT=cdrom:\\SCUS-9418a;1"),
            Some("SCUS-9418A".into())
        );
        assert_eq!(
            boot_serial(b"BOOT=cdrom:\\SCUS_949.00;1\xff"),
            Some("SCUS-94900".into())
        );
        assert_eq!(boot_serial(b"BOOT\n=cdrom:\\SCUS_949.00;1"), None);
        for bad in [
            b"Crash Bandicoot SCUS-94900".as_slice(),
            b"BOOT cdrom:\\SCUS_949.00;1",
            b"boot=cdrom:\\SCUS_949.00;1",
            b"BOOT=cdrom:\\SCUS_94X.00;1",
            b"BOOT=cdrom:\\SCUS_941.8",
            b"\0BOOT=cdrom:\\SCUS_949.00;1",
        ] {
            assert!(boot_serial(bad).is_none(), "{bad:?}");
        }
    }
}
