//! SAME CD-i native input sequences, pinned to 9a589f6ba8c3.
use anyhow::{Result, ensure};
use sha2::{Digest, Sha256};
use std::io::{Read, Seek, Write};
use std::path::{Path, PathBuf};

pub(crate) enum DiscSnapshot {
    Iso(IsoSnapshot),
    Cue(CueDisc),
    Chd(ChdDisc),
}

impl DiscSnapshot {
    pub(crate) fn open(path: &Path) -> Result<Self> {
        let extension = path
            .extension()
            .and_then(|value| value.to_str())
            .unwrap_or_default()
            .to_ascii_lowercase();
        let disc = match extension.as_str() {
            "iso" => Self::Iso(IsoSnapshot::prepare(path)?),
            "cue" => Self::Cue(CueDisc::open(path)?),
            "chd" => Self::Chd(ChdDisc::open(path)?),
            _ => anyhow::bail!("SAME CD-i requires ISO, CUE or CHD disc content"),
        };
        disc.verify()?;
        Ok(disc)
    }

    pub(crate) fn verify(&self) -> Result<()> {
        match self {
            Self::Iso(disc) => disc.verify(),
            Self::Cue(disc) => disc.verify(),
            Self::Chd(disc) => disc.verify(),
        }
    }

    pub(crate) fn native_sector(&mut self, lba: u32) -> Result<[u8; 2560]> {
        match self {
            Self::Iso(disc) => disc.native_sector(lba),
            Self::Cue(disc) => disc.native_sector(lba),
            Self::Chd(disc) => disc.native_sector(lba),
        }
    }

    /// Caller owns read order and normalization state, as the core's byte-swap
    /// heuristic is sticky across sectors. False is an observation, not proof
    /// that every controller mode or the complete disc is unsupported.
    pub(crate) fn normalized_sector(
        &mut self,
        lba: u32,
        normalizer: &mut SectorNormalizer,
    ) -> Result<([u8; 2560], bool)> {
        let mut bytes = self.native_sector(lba)?;
        let valid = normalizer.normalize(lba, &mut bytes);
        Ok((bytes, valid))
    }
}

struct CueBacking {
    path: PathBuf,
    canonical: PathBuf,
    file: std::fs::File,
    metadata: std::fs::Metadata,
}

impl CueBacking {
    fn open(path: PathBuf) -> Result<Self> {
        let canonical = path.canonicalize()?;
        let metadata = std::fs::metadata(&canonical)?;
        ensure!(
            metadata.is_file() && metadata.len() > 0 && metadata.len() <= 4 * 1024 * 1024 * 1024,
            "SAME CD-i CUE backing file must be a bounded nonempty regular file"
        );
        let file = std::fs::File::open(&canonical)?;
        let backing = Self {
            path,
            canonical,
            file,
            metadata,
        };
        backing.verify()?;
        Ok(backing)
    }

    fn verify(&self) -> Result<()> {
        ensure!(
            self.path.canonicalize()? == self.canonical
                && unchanged_disc(&self.metadata, &self.file.metadata()?)
                && unchanged_disc(&self.metadata, &std::fs::metadata(&self.canonical)?),
            "SAME CD-i CUE backing file changed during preparation"
        );
        Ok(())
    }
}

pub(crate) struct CueDisc {
    path: PathBuf,
    stamp: Option<FileStamp>,
    backing: std::collections::BTreeMap<String, CueBacking>,
    pub(crate) tracks: Vec<CueTrack>,
    pub(crate) spans: Vec<CueSpan>,
    logical_starts: Vec<u64>,
    logical_end: u64,
}

impl CueDisc {
    pub(crate) fn open(path: &Path) -> Result<Self> {
        ensure!(path.is_absolute(), "SAME CD-i CUE path must be absolute");
        let text_path = path
            .to_str()
            .ok_or_else(|| anyhow::anyhow!("SAME CD-i CUE path must be UTF-8"))?;
        ensure!(
            !text_path.contains('\\'),
            "SAME CD-i CUE path uses ambiguous native path separators"
        );
        let (stamp, bytes) = read_native_file(path)?;
        ensure!(stamp.is_some(), "SAME CD-i CUE file is missing");
        let tracks = parse_cue(std::str::from_utf8(&bytes)?)?;
        let parent = path
            .parent()
            .ok_or_else(|| anyhow::anyhow!("SAME CD-i CUE has no directory"))?;
        let mut backing = std::collections::BTreeMap::new();
        let mut lengths = std::collections::BTreeMap::new();
        let mut wave_regions = std::collections::BTreeMap::new();
        for track in &tracks {
            let relative = Path::new(&track.file);
            ensure!(
                !relative.is_absolute() && !track.file.contains('\\') && !track.file.contains(':'),
                "SAME CD-i CUE requires unambiguous relative backing-file paths"
            );
            if !backing.contains_key(&track.file) {
                let file = CueBacking::open(parent.join(relative))?;
                lengths.insert(track.file.clone(), file.metadata.len());
                backing.insert(track.file.clone(), file);
            }
            if track.file_kind == CueFileKind::Wave && !wave_regions.contains_key(&track.file) {
                let file = backing.get_mut(&track.file).unwrap();
                let region = wave_region(&mut file.file)?;
                file.verify()?;
                wave_regions.insert(track.file.clone(), region);
            }
        }
        let spans = cue_spans(&tracks, &lengths, &wave_regions)?;
        let mut logical_starts = Vec::with_capacity(tracks.len());
        let mut logical_end = 0u64;
        for (track, span) in tracks.iter().zip(&spans) {
            if track.stored_pregap {
                logical_starts.push(logical_end + u64::from(track.pregap));
            } else {
                logical_end += u64::from(track.pregap);
                logical_starts.push(logical_end);
            }
            logical_end += u64::from(track.postgap) + u64::from(span.frames);
            ensure!(
                logical_end <= u64::from(u32::MAX),
                "SAME CD-i CUE logical addressing exceeds native counters"
            );
        }
        let disc = Self {
            path: path.to_owned(),
            stamp,
            backing,
            tracks,
            spans,
            logical_starts,
            logical_end,
        };
        disc.verify()?;
        Ok(disc)
    }

    pub(crate) fn native_sector(&mut self, lba: u32) -> Result<[u8; 2560]> {
        self.verify()?;
        let lba = u64::from(lba);
        ensure!(
            lba < self.logical_end,
            "SAME CD-i CUE sector is beyond disc lead-out"
        );
        let index = (0..self.tracks.len())
            .find(|index| {
                lba < self
                    .logical_starts
                    .get(index + 1)
                    .copied()
                    .unwrap_or(self.logical_end)
            })
            .ok_or_else(|| anyhow::anyhow!("SAME CD-i CUE sector has no native track"))?;
        let track = &self.tracks[index];
        let start = self.logical_starts[index];
        let mut output = [0; 2560];
        if !track.stored_pregap && lba < start {
            return Ok(output);
        }
        let relative = i64::try_from(lba)? - i64::try_from(start)?
            + if track.stored_pregap {
                i64::from(track.pregap)
            } else {
                0
            };
        let relative = u64::try_from(relative)
            .map_err(|_| anyhow::anyhow!("SAME CD-i CUE sector precedes its backing track"))?;
        let size = track.kind.data_size() as usize;
        let width = u64::from(track.kind.data_size() + track.subchannel_bytes);
        let offset = self.spans[index].offset + relative * width;
        let file = self.backing.get_mut(&track.file).unwrap();
        ensure!(
            offset + size as u64 <= file.metadata.len(),
            "SAME CD-i CUE sector exceeds its backing file"
        );
        file.file.seek(std::io::SeekFrom::Start(offset))?;
        file.file.read_exact(&mut output[..size])?;
        if track.swap {
            // Native loose-file RAW_DONTCARE swaps through byte 2351,
            // including the zero-initialized tail of shorter data sectors.
            for pair in output[..2352].chunks_exact_mut(2) {
                pair.swap(0, 1);
            }
        }
        self.verify()?;
        Ok(output)
    }

    pub(crate) fn verify(&self) -> Result<()> {
        ensure!(
            read_native_file(&self.path)?.0 == self.stamp,
            "SAME CD-i CUE declaration changed during preparation"
        );
        for file in self.backing.values() {
            file.verify()?;
        }
        Ok(())
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum CueFileKind {
    Binary,
    Motorola,
    Wave,
}

pub(crate) struct CueTrack {
    pub(crate) file: String,
    pub(crate) file_kind: CueFileKind,
    pub(crate) kind: TrackKind,
    pub(crate) subchannel_bytes: u32,
    pub(crate) swap: bool,
    pub(crate) index0: Option<u32>,
    pub(crate) index1: u32,
    pub(crate) pregap: u32,
    pub(crate) stored_pregap: bool,
    pub(crate) postgap: u32,
}

pub(crate) struct CueSpan {
    pub(crate) offset: u64,
    pub(crate) frames: u32,
}

/// Inspect the native WAVE subset without decoding or reading audio payloads.
pub(crate) fn wave_region(file: &mut std::fs::File) -> Result<(u64, u64)> {
    let metadata = file.metadata()?;
    ensure!(
        metadata.is_file() && metadata.len() <= 4 * 1024 * 1024 * 1024,
        "SAME CD-i WAVE must be a bounded regular file"
    );
    file.rewind()?;
    let mut header = [0; 12];
    file.read_exact(&mut header)?;
    ensure!(
        &header[..4] == b"RIFF" && &header[8..12] == b"WAVE",
        "SAME CD-i CUE WAVE lacks RIFF/WAVE headers"
    );
    let declared = u64::from(u32::from_le_bytes(header[4..8].try_into().unwrap()));
    ensure!(
        declared >= 4 && declared + 8 <= metadata.len(),
        "SAME CD-i WAVE RIFF size exceeds the file"
    );
    let mut offset = 12u64;
    let mut format_seen = false;
    for _ in 0..4096 {
        ensure!(
            offset + 8 <= declared + 8,
            "SAME CD-i WAVE chunk header exceeds RIFF bounds"
        );
        file.seek(std::io::SeekFrom::Start(offset))?;
        let mut chunk = [0; 8];
        file.read_exact(&mut chunk)?;
        let length = u64::from(u32::from_le_bytes(chunk[4..8].try_into().unwrap()));
        offset += 8;
        ensure!(
            offset + length <= declared + 8,
            "SAME CD-i WAVE chunk payload exceeds RIFF bounds"
        );
        if !format_seen && &chunk[..4] == b"fmt " {
            ensure!(length >= 16, "SAME CD-i WAVE format chunk is too short");
            let mut format = [0; 16];
            file.read_exact(&mut format)?;
            ensure!(
                u16::from_le_bytes(format[..2].try_into().unwrap()) == 1
                    && u16::from_le_bytes(format[2..4].try_into().unwrap()) == 2
                    && u32::from_le_bytes(format[4..8].try_into().unwrap()) == 44100
                    && u16::from_le_bytes(format[14..16].try_into().unwrap()) == 16,
                "SAME CD-i WAVE requires stereo 16-bit 44100 Hz PCM"
            );
            format_seen = true;
        } else if format_seen && &chunk[..4] == b"data" {
            ensure!(length > 0, "SAME CD-i WAVE data is empty");
            return Ok((offset, length));
        } else {
            // The pinned parser advances by declared bytes without RIFF's
            // usual odd-byte padding and compares against the RIFF size field.
            ensure!(
                offset + length < declared,
                "SAME CD-i native WAVE search reaches its size limit"
            );
        }
        offset += length;
    }
    anyhow::bail!("SAME CD-i WAVE has excessive chunks or no supported data payload")
}

/// Resolve the pinned parser's backing-file arithmetic. File identity is
/// compared by the declared name, not canonical aliases, just as in the core.
pub(crate) fn cue_spans(
    tracks: &[CueTrack],
    lengths: &std::collections::BTreeMap<String, u64>,
    wave_regions: &std::collections::BTreeMap<String, (u64, u64)>,
) -> Result<Vec<CueSpan>> {
    ensure!(
        !tracks.is_empty() && tracks.len() <= 99,
        "SAME CD-i CUE has invalid track count"
    );
    let mut spans: Vec<CueSpan> = Vec::with_capacity(tracks.len());
    for (index, track) in tracks.iter().enumerate() {
        let length = *lengths
            .get(&track.file)
            .ok_or_else(|| anyhow::anyhow!("Missing SAME CD-i CUE backing-file length"))?;
        ensure!(
            length > 0 && length <= 4 * 1024 * 1024 * 1024,
            "SAME CD-i CUE backing file is empty or oversized"
        );
        let width = u64::from(track.kind.data_size() + track.subchannel_bytes);
        let previous_end = || -> u64 {
            if index == 0 {
                0
            } else {
                let previous = &spans[index - 1];
                previous.offset
                    + u64::from(previous.frames)
                        * u64::from(
                            tracks[index - 1].kind.data_size() + tracks[index - 1].subchannel_bytes,
                        )
            }
        };
        let (offset, frames) = if track.file_kind == CueFileKind::Wave {
            let &(offset, bytes) = wave_regions
                .get(&track.file)
                .ok_or_else(|| anyhow::anyhow!("SAME CD-i WAVE payload has not been resolved"))?;
            ensure!(
                offset > 0 && offset.checked_add(bytes).is_some_and(|end| end <= length),
                "SAME CD-i WAVE payload is outside its file"
            );
            (offset, bytes / 2352)
        } else if index + 1 == tracks.len() {
            let offset = if index > 0 && tracks[index - 1].file == track.file {
                previous_end()
            } else {
                0
            };
            ensure!(
                offset <= length,
                "SAME CD-i final CUE track starts outside its file"
            );
            (offset, (length - offset) / width)
        } else if tracks[index + 1].file == track.file {
            let start = track
                .index0
                .ok_or_else(|| anyhow::anyhow!("Missing SAME CD-i CUE INDEX 00/01 offset"))?;
            let end = tracks[index + 1]
                .index0
                .ok_or_else(|| anyhow::anyhow!("Missing SAME CD-i next-track INDEX offset"))?;
            let frames = end.checked_sub(start).ok_or_else(|| {
                anyhow::anyhow!("SAME CD-i shared-file CUE indexes run backwards")
            })?;
            // Native source carries the previous track's end even at the
            // start of another shared-file group. Do not silently repair it.
            (previous_end(), u64::from(frames))
        } else {
            (0, length / width)
        };
        ensure!(
            frames > 0 && frames <= u64::from(u32::MAX),
            "SAME CD-i CUE track has invalid frame count"
        );
        ensure!(
            offset
                .checked_add(frames * width)
                .is_some_and(|end| end <= length),
            "SAME CD-i native CUE track span exceeds its backing file"
        );
        ensure!(
            u64::from(track.pregap) <= frames,
            "SAME CD-i CUE pregap exceeds its track frames"
        );
        spans.push(CueSpan {
            offset,
            frames: frames as u32,
        });
    }
    Ok(spans)
}

pub(crate) fn parse_cue(source: &str) -> Result<Vec<CueTrack>> {
    ensure!(
        !(source
            .lines()
            .any(|line| line.starts_with("REM SINGLE-DENSITY AREA"))
            && source
                .lines()
                .any(|line| line.starts_with("REM HIGH-DENSITY AREA"))),
        "GD-ROM multi-CUE content is not a CD-i disc"
    );
    ensure!(
        source.len() <= 8 * 1024 * 1024 && source.ends_with('\n'),
        "SAME CD-i CUE must be bounded and newline-terminated for the native reader"
    );
    let mut tracks: Vec<CueTrack> = Vec::new();
    let mut current_file: Option<(String, CueFileKind)> = None;
    let mut new_file = false;
    let mut seen_index0 = false;
    let mut seen_index1 = false;
    let mut seen_pregap = false;
    let mut seen_postgap = false;
    for line in source.lines() {
        let fields = cue_tokens(line)?;
        let Some(command) = fields.first().map(String::as_str) else {
            continue;
        };
        match command {
            "FILE" => {
                ensure!(
                    fields.len() == 3 && !fields[1].is_empty(),
                    "Malformed SAME CD-i CUE FILE"
                );
                ensure!(!new_file, "SAME CD-i CUE FILE has no following track");
                let kind = match fields[2].as_str() {
                    "BINARY" => CueFileKind::Binary,
                    "MOTOROLA" => CueFileKind::Motorola,
                    "WAVE" => CueFileKind::Wave,
                    _ => anyhow::bail!("Unsupported SAME CD-i CUE FILE type"),
                };
                current_file = Some((fields[1].clone(), kind));
                new_file = true;
            }
            "TRACK" => {
                ensure!(
                    (3..=4).contains(&fields.len()) && tracks.len() < 99,
                    "Malformed or excessive SAME CD-i CUE tracks"
                );
                ensure!(
                    tracks.is_empty() || seen_index1,
                    "SAME CD-i CUE track has no INDEX 01"
                );
                let number: usize = fields[1].parse()?;
                ensure!(
                    number == tracks.len() + 1,
                    "SAME CD-i CUE track numbers must be sequential"
                );
                let (file, file_kind) = current_file
                    .as_ref()
                    .ok_or_else(|| anyhow::anyhow!("SAME CD-i CUE TRACK precedes FILE"))?;
                let kind = TrackKind::parse(&fields[2])?;
                let subchannel_bytes = match fields.get(3).map(String::as_str).unwrap_or("NONE") {
                    "NONE" => 0,
                    "RW" | "RW_RAW" => 96,
                    _ => anyhow::bail!("Unknown SAME CD-i CUE subchannel type"),
                };
                ensure!(
                    *file_kind != CueFileKind::Wave || new_file,
                    "SAME CD-i WAVE reuse needs native byte-offset resolution"
                );
                tracks.push(CueTrack {
                    file: file.clone(),
                    file_kind: *file_kind,
                    kind,
                    subchannel_bytes,
                    swap: kind == TrackKind::Audio
                        || (new_file && *file_kind == CueFileKind::Motorola),
                    index0: None,
                    index1: 0,
                    pregap: 0,
                    stored_pregap: false,
                    postgap: 0,
                });
                new_file = false;
                seen_index0 = false;
                seen_index1 = false;
                seen_pregap = false;
                seen_postgap = false;
            }
            "INDEX" => {
                ensure!(
                    fields.len() == 3 && !new_file,
                    "Malformed SAME CD-i CUE INDEX"
                );
                let track = tracks
                    .last_mut()
                    .ok_or_else(|| anyhow::anyhow!("SAME CD-i CUE INDEX precedes TRACK"))?;
                let index: u32 = fields[1].parse()?;
                let frame = cue_frames(&fields[2])?;
                match index {
                    0 => {
                        ensure!(
                            !seen_index0 && !seen_index1,
                            "Repeated or out-of-order SAME CD-i INDEX 00"
                        );
                        track.index0 = Some(frame);
                        seen_index0 = true;
                    }
                    1 => {
                        ensure!(!seen_index1, "Repeated SAME CD-i INDEX 01");
                        track.index1 = frame;
                        seen_index1 = true;
                        if track.pregap == 0 && track.index0.is_some() {
                            track.pregap =
                                frame.checked_sub(track.index0.unwrap()).ok_or_else(|| {
                                    anyhow::anyhow!("SAME CD-i INDEX 01 precedes INDEX 00")
                                })?;
                            track.stored_pregap = true;
                        } else {
                            track.index0 = Some(frame);
                        }
                    }
                    2..=99 => {} // Native parser does not use later indexes.
                    _ => anyhow::bail!("Invalid SAME CD-i CUE INDEX number"),
                }
            }
            "PREGAP" | "POSTGAP" => {
                ensure!(
                    fields.len() == 2 && !new_file,
                    "Malformed SAME CD-i CUE gap"
                );
                let track = tracks
                    .last_mut()
                    .ok_or_else(|| anyhow::anyhow!("SAME CD-i CUE gap precedes TRACK"))?;
                let frames = cue_frames(&fields[1])?;
                if command == "PREGAP" {
                    ensure!(!seen_pregap, "Repeated SAME CD-i CUE PREGAP");
                    seen_pregap = true;
                    track.pregap = frames;
                } else {
                    ensure!(!seen_postgap, "Repeated SAME CD-i CUE POSTGAP");
                    seen_postgap = true;
                    track.postgap = frames;
                }
            }
            "REM" | "TITLE" | "PERFORMER" | "SONGWRITER" | "CATALOG" | "ISRC" | "FLAGS"
            | "CDTEXTFILE" => {}
            _ => anyhow::bail!("Unrecognized SAME CD-i CUE directive: {command}"),
        }
    }
    ensure!(
        !tracks.is_empty() && !new_file && seen_index1,
        "Incomplete SAME CD-i CUE track declarations"
    );
    Ok(tracks)
}

/// Native CUE tokens support either quote style, without backslash escapes.
/// Reject truncated/unterminated lines instead of emulating buffer overreads.
pub(crate) fn cue_tokens(line: &str) -> Result<Vec<String>> {
    ensure!(
        line.len() < 510 && !line.contains('\0'),
        "SAME CD-i CUE line exceeds native capacity or contains NUL"
    );
    let mut tokens = Vec::new();
    let mut token = String::new();
    let mut single = false;
    let mut double = false;
    let mut started = false;
    for character in line.chars() {
        if character == '"' && !single {
            double = !double;
            started = true;
        } else if character == '\'' && !double {
            single = !single;
            started = true;
        } else if character.is_ascii_whitespace() && !single && !double {
            if started {
                tokens.push(std::mem::take(&mut token));
                started = false;
            }
        } else {
            ensure!(
                !character.is_control(),
                "SAME CD-i CUE contains unsupported control characters"
            );
            token.push(character);
            started = true;
        }
    }
    ensure!(
        !single && !double,
        "SAME CD-i CUE has an unterminated quoted token"
    );
    if started {
        tokens.push(token);
    }
    Ok(tokens)
}

/// Match the native one-, two-, or three-part time interpretation. A single
/// integer is already frames; two integers are minutes and seconds.
pub(crate) fn cue_frames(value: &str) -> Result<u32> {
    let fields = value.split(':').collect::<Vec<_>>();
    ensure!(
        (1..=3).contains(&fields.len()),
        "Invalid SAME CD-i CUE time"
    );
    let numbers = fields
        .iter()
        .map(|field| {
            ensure!(
                !field.is_empty() && field.bytes().all(|byte| byte.is_ascii_digit()),
                "Invalid SAME CD-i CUE time field"
            );
            Ok(field.parse::<u64>()?)
        })
        .collect::<Result<Vec<_>>>()?;
    let total = match numbers.as_slice() {
        [frames] => Some(*frames),
        [minutes, seconds] => minutes
            .checked_mul(60)
            .and_then(|v| v.checked_add(*seconds))
            .and_then(|v| v.checked_mul(75)),
        [minutes, seconds, frames] => minutes
            .checked_mul(60)
            .and_then(|v| v.checked_add(*seconds))
            .and_then(|v| v.checked_mul(75))
            .and_then(|v| v.checked_add(*frames)),
        _ => unreachable!(),
    }
    .ok_or_else(|| anyhow::anyhow!("SAME CD-i CUE time overflows"))?;
    ensure!(
        total <= i32::MAX as u64,
        "SAME CD-i CUE time exceeds native frame counter"
    );
    Ok(total as u32)
}

pub(crate) type ChdMetadata = std::collections::BTreeMap<[u8; 4], Vec<Vec<u8>>>;

#[derive(Default)]
pub(crate) struct SectorNormalizer {
    byteswap: bool,
}

fn native_sector_valid(lba: u32, bytes: &[u8; 2560]) -> bool {
    let absolute = lba.wrapping_add(150);
    let bcd = |value: u8| ((value / 10) << 4) | (value % 10);
    bytes[12] == bcd((absolute / (60 * 75)) as u8)
        && bytes[13] == bcd(((absolute / 75) % 60) as u8)
        && bytes[14] == bcd((absolute % 75) as u8)
        && matches!(bytes[15], 1 | 2)
        && bytes[16..20] == bytes[20..24]
}

impl SectorNormalizer {
    /// Match CDIC preprocessing, including its sticky byte-swap detection and
    /// unusual duplicate-subheader check for both mode 1 and mode 2 sectors.
    /// The result is the native validity heuristic, not an EDC/ECC checksum.
    pub(crate) fn normalize(&mut self, lba: u32, bytes: &mut [u8; 2560]) -> bool {
        if bytes[0] == 0xff && bytes[1] == 0x00 {
            self.byteswap = true;
        }
        if self.byteswap {
            for pair in bytes.chunks_exact_mut(2) {
                pair.swap(0, 1);
            }
        }
        if native_sector_valid(lba, bytes) {
            return true;
        }
        let mut candidate = *bytes;
        // Standard CD scrambling sequence, starting 01 80 00 60 00 28,
        // corresponds to the native s_sector_scramble table at offset 12.
        let mut shift = 1u16;
        for byte in &mut candidate[12..2352] {
            let mut mask = 0u8;
            for bit in 0..8 {
                mask |= ((shift & 1) as u8) << bit;
                let feedback = (shift ^ (shift >> 1)) & 1;
                shift = (shift >> 1) | (feedback << 14);
            }
            *byte ^= mask;
        }
        if native_sector_valid(lba, &candidate) {
            *bytes = candidate;
            true
        } else {
            false
        }
    }
}

pub(crate) struct ChdDisc {
    path: PathBuf,
    canonical: PathBuf,
    retained_file: std::fs::File,
    metadata: std::fs::Metadata,
    image: chd::Chd<std::fs::File>,
    pub(crate) tracks: Vec<ChdTrack>,
    pub(crate) addresses: ChdAddressMap,
    decoded: Vec<u8>,
    compressed: Vec<u8>,
    cached_hunk: Option<u32>,
}

impl ChdDisc {
    /// Reproduce the CDIC's RAW_DONTCARE input buffer, before its stateful
    /// byte-swap/descramble processing. This is not an ISO cooked-sector view.
    pub(crate) fn native_sector(&mut self, lba: u32) -> Result<[u8; 2560]> {
        self.verify()?;
        let lba = u64::from(lba);
        ensure!(
            lba < self.addresses.logical_end,
            "SAME CD-i sector is beyond the disc lead-out"
        );
        let index = (0..self.addresses.tracks.len())
            .find(|index| {
                let next = self
                    .addresses
                    .tracks
                    .get(index + 1)
                    .map_or(self.addresses.logical_end, |track| track.logical_start);
                lba < next
            })
            .ok_or_else(|| anyhow::anyhow!("SAME CD-i sector has no native track"))?;
        let track = &self.tracks[index];
        let address = &self.addresses.tracks[index];
        let mut output = [0; 2560];
        if track.stored_pregap.is_none() && lba < address.logical_start {
            return Ok(output);
        }
        let stored = i64::try_from(lba)? - i64::try_from(address.logical_start)?
            + i64::try_from(address.stored_start)?
            + if track.stored_pregap.is_some() {
                i64::from(track.pregap)
            } else {
                0
            };
        let stored = u64::try_from(stored)
            .map_err(|_| anyhow::anyhow!("SAME CD-i native sector precedes stored data"))?;
        let size = track.kind.data_size() as usize;
        let frame = self.stored_frame(stored)?;
        output[..size].copy_from_slice(&frame[..size]);
        Ok(output)
    }

    pub(crate) fn open(path: &Path) -> Result<Self> {
        ensure!(path.is_absolute(), "SAME CD-i CHD path must be absolute");
        let canonical = path.canonicalize()?;
        let metadata = std::fs::metadata(&canonical)?;
        ensure!(metadata.is_file(), "SAME CD-i CHD must be a regular file");
        let mut file = std::fs::File::open(&canonical)?;
        let retained_file = file.try_clone()?;
        let header = validate_chd_header(&mut file)?;
        let records = read_chd_metadata(&mut file, &header)?;
        let tracks = parse_text_cd_tracks(&records)?;
        let addresses = chd_address_map(&tracks, header.logical_bytes())?;
        let image = chd::Chd::open(file, None)?;
        let decoded = image.get_hunksized_buffer();
        ensure!(
            decoded.len() == header.hunk_size() as usize,
            "SAME CD-i CHD decoder geometry changed during preparation"
        );
        let disc = Self {
            path: path.to_owned(),
            canonical,
            retained_file,
            metadata,
            image,
            tracks,
            addresses,
            decoded,
            compressed: Vec::new(),
            cached_hunk: None,
        };
        disc.verify()?;
        Ok(disc)
    }

    pub(crate) fn verify(&self) -> Result<()> {
        ensure!(
            self.path.canonicalize()? == self.canonical
                && unchanged_disc(&self.metadata, &self.retained_file.metadata()?)
                && unchanged_disc(&self.metadata, &std::fs::metadata(&self.canonical)?),
            "SAME CD-i CHD changed during launch preparation"
        );
        Ok(())
    }

    /// Read storage coordinates, not disc LBAs. A caller must use the native
    /// track map to exclude padding, audio and virtual gaps as appropriate.
    pub(crate) fn stored_frame(&mut self, frame: u64) -> Result<[u8; 2448]> {
        self.verify()?;
        ensure!(
            frame < self.addresses.stored_end,
            "SAME CD-i CHD frame is outside the stored image"
        );
        let frames_per_hunk = self.decoded.len() as u64 / 2448;
        let hunk = u32::try_from(frame / frames_per_hunk)?;
        let offset = usize::try_from((frame % frames_per_hunk) * 2448)?;
        if self.cached_hunk != Some(hunk) {
            self.cached_hunk = None;
            self.image
                .hunk(hunk)?
                .read_hunk_in(&mut self.compressed, &mut self.decoded)?;
            self.cached_hunk = Some(hunk);
        }
        let mut output = [0; 2448];
        output.copy_from_slice(&self.decoded[offset..offset + 2448]);
        self.verify()?;
        Ok(output)
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum TrackKind {
    Mode1,
    Mode1Raw,
    Mode2,
    Mode2Form1,
    Mode2Form2,
    Mode2Mixed,
    Mode2Raw,
    Audio,
}

impl TrackKind {
    pub(crate) fn parse(value: &str) -> Result<Self> {
        Ok(match value {
            "MODE1" | "MODE1/2048" => Self::Mode1,
            // Preserve the pinned native parser's first match, even though
            // CDI/2352 appears again in its later MODE2_RAW branch.
            "MODE1_RAW" | "MODE1/2352" | "CDI/2352" => Self::Mode1Raw,
            "MODE2" | "MODE2/2336" => Self::Mode2,
            "MODE2_FORM1" | "MODE2/2048" => Self::Mode2Form1,
            "MODE2_FORM2" | "MODE2/2324" => Self::Mode2Form2,
            "MODE2_FORM_MIX" => Self::Mode2Mixed,
            "MODE2_RAW" | "MODE2/2352" => Self::Mode2Raw,
            "AUDIO" => Self::Audio,
            _ => anyhow::bail!("Unknown SAME CD-i native track type: {value}"),
        })
    }

    pub(crate) fn data_size(self) -> u32 {
        match self {
            Self::Mode1 | Self::Mode2Form1 => 2048,
            Self::Mode2 | Self::Mode2Mixed => 2336,
            Self::Mode2Form2 => 2324,
            Self::Mode1Raw | Self::Mode2Raw | Self::Audio => 2352,
        }
    }
}

pub(crate) struct ChdTrack {
    pub(crate) kind: TrackKind,
    pub(crate) frames: u32,
    pub(crate) pregap: u32,
    pub(crate) postgap: u32,
    pub(crate) stored_pregap: Option<TrackKind>,
}

pub(crate) struct ChdTrackAddress {
    pub(crate) physical_start: u64,
    pub(crate) stored_start: u64,
    pub(crate) logical_start: u64,
    pub(crate) logical_frames: u64,
}

pub(crate) struct ChdAddressMap {
    pub(crate) tracks: Vec<ChdTrackAddress>,
    pub(crate) physical_end: u64,
    pub(crate) stored_end: u64,
    pub(crate) logical_end: u64,
}

/// Mirror cdrom_open's three independent cursors. Native track offsets are
/// not Beetle's -150-based disc cursor, and CHD padding is not playable data.
pub(crate) fn chd_address_map(tracks: &[ChdTrack], logical_bytes: u64) -> Result<ChdAddressMap> {
    ensure!(
        !tracks.is_empty() && tracks.len() <= 99,
        "SAME CD-i CHD needs 1 through 99 tracks"
    );
    let mut physical = 0u64;
    let mut stored = 0u64;
    let mut logical = 0u64;
    let mut addresses = Vec::with_capacity(tracks.len());
    for track in tracks {
        let frames = u64::from(track.frames);
        let pregap = u64::from(track.pregap);
        ensure!(
            frames > 0 && pregap <= frames,
            "SAME CD-i track pregap would underflow the native frame count"
        );
        let logical_start = if track.stored_pregap.is_some() {
            logical + pregap
        } else {
            logical += pregap;
            logical
        };
        addresses.push(ChdTrackAddress {
            physical_start: physical,
            stored_start: stored,
            logical_start,
            logical_frames: frames - pregap,
        });
        physical += frames;
        stored += frames.div_ceil(4) * 4;
        logical += u64::from(track.postgap) + frames;
        ensure!(
            physical <= u64::from(u32::MAX)
                && stored <= u64::from(u32::MAX)
                && logical <= u64::from(u32::MAX),
            "SAME CD-i track addressing exceeds native counters"
        );
    }
    ensure!(
        stored * 2448 == logical_bytes,
        "SAME CD-i track frames and padding do not cover the CHD logical image"
    );
    Ok(ChdAddressMap {
        tracks: addresses,
        physical_end: physical,
        stored_end: stored,
        logical_end: logical,
    })
}

pub(crate) fn parse_text_cd_tracks(metadata: &ChdMetadata) -> Result<Vec<ChdTrack>> {
    let old = metadata.get(b"CHTR");
    let modern = metadata.get(b"CHT2");
    let count = old.map_or(0, Vec::len).max(modern.map_or(0, Vec::len));
    ensure!(
        count > 0 && count <= 99,
        "SAME CD-i CHD needs text CD track metadata; legacy and GD records require separate interpretation"
    );
    (0..count)
        .map(|index| {
            if let Some(payload) = old.and_then(|tracks| tracks.get(index)) {
                parse_cd_track(payload, false, index)
            } else {
                parse_cd_track(&modern.unwrap()[index], true, index)
            }
        })
        .collect()
}

pub(crate) fn parse_cd_track(payload: &[u8], modern: bool, ordinal: usize) -> Result<ChdTrack> {
    let text = std::str::from_utf8(payload.split(|byte| *byte == 0).next().unwrap_or_default())?;
    let fields = text.split_ascii_whitespace().collect::<Vec<_>>();
    let keys: &[&str] = if modern {
        &[
            "TRACK", "TYPE", "SUBTYPE", "FRAMES", "PREGAP", "PGTYPE", "PGSUB", "POSTGAP",
        ]
    } else {
        &["TRACK", "TYPE", "SUBTYPE", "FRAMES"]
    };
    ensure!(
        fields.len() == keys.len(),
        "Malformed SAME CD-i track metadata field count"
    );
    let mut values = Vec::new();
    for (field, key) in fields.iter().zip(keys) {
        let (actual, value) = field
            .split_once(':')
            .ok_or_else(|| anyhow::anyhow!("Malformed SAME CD-i track field"))?;
        ensure!(
            actual == *key && !value.is_empty(),
            "Unexpected SAME CD-i track field order"
        );
        values.push(value);
    }
    let number: usize = values[0].parse()?;
    ensure!(
        number == ordinal + 1 && number <= 99,
        "SAME CD-i track numbering is not sequential"
    );
    let kind = TrackKind::parse(values[1])?;
    ensure!(
        matches!(values[2], "NONE" | "RW" | "RW_RAW"),
        "Unknown SAME CD-i subchannel type"
    );
    let frames: u32 = values[3].parse()?;
    const MAX_FRAMES: u64 = 4 * 1024 * 1024 * 1024 / 2448;
    ensure!(
        frames > 0 && u64::from(frames) <= MAX_FRAMES,
        "SAME CD-i track frame count exceeds disc limits"
    );
    let (pregap, postgap, stored_pregap) = if modern {
        let pregap: u32 = values[4].parse()?;
        let postgap: u32 = values[7].parse()?;
        ensure!(
            u64::from(pregap) <= MAX_FRAMES && u64::from(postgap) <= MAX_FRAMES,
            "SAME CD-i track gaps exceed disc limits"
        );
        ensure!(
            matches!(values[6], "NONE" | "RW" | "RW_RAW"),
            "Unknown SAME CD-i pregap subchannel type"
        );
        let stored = if pregap > 0 {
            values[5]
                .strip_prefix('V')
                .map(TrackKind::parse)
                .transpose()?
        } else {
            None
        };
        ensure!(
            stored.is_none() || pregap <= frames,
            "SAME CD-i stored pregap exceeds track frames"
        );
        (pregap, postgap, stored)
    } else {
        (0, 0, None)
    };
    Ok(ChdTrack {
        kind,
        frames,
        pregap,
        postgap,
        stored_pregap,
    })
}

/// Preserve tag-local ordering: SAME CD-i looks up CHTR before CHT2 at each
/// ordinal, unlike Beetle. Keep legacy CD and GD payloads for explicit handling.
pub(crate) fn read_chd_metadata(
    file: &mut std::fs::File,
    header: &chd::header::Header,
) -> Result<ChdMetadata> {
    let length = file.metadata()?.len();
    let mut offset = header.meta_offset().unwrap_or(0);
    let mut seen = std::collections::BTreeSet::new();
    let mut entries = ChdMetadata::new();
    let mut retained_bytes = 0usize;
    while offset != 0 {
        ensure!(
            seen.len() < 4096 && seen.insert(offset),
            "Cyclic or excessive SAME CD-i CHD metadata"
        );
        ensure!(
            offset.checked_add(16).is_some_and(|end| end <= length),
            "SAME CD-i CHD metadata header is outside the file"
        );
        file.seek(std::io::SeekFrom::Start(offset))?;
        let mut bytes = [0; 16];
        file.read_exact(&mut bytes)?;
        let tag: [u8; 4] = bytes[..4].try_into().unwrap();
        let size = u32::from_be_bytes(bytes[4..8].try_into().unwrap()) & 0x00ff_ffff;
        ensure!(
            offset
                .checked_add(16 + u64::from(size))
                .is_some_and(|end| end <= length),
            "SAME CD-i CHD metadata payload is outside the file"
        );
        if matches!(&tag, b"CHTR" | b"CHT2" | b"CHCD" | b"CHGT" | b"CHGD") {
            let limit = if &tag == b"CHCD" { 16384 } else { 1024 };
            ensure!(
                size > 0 && size <= limit,
                "SAME CD-i CHD track metadata has an unsupported size"
            );
            retained_bytes += size as usize;
            ensure!(
                retained_bytes <= 512 * 1024,
                "SAME CD-i CHD metadata exceeds retained-data limit"
            );
            let values = entries.entry(tag).or_default();
            ensure!(
                values.len() < 99,
                "SAME CD-i CHD has too many track metadata entries"
            );
            let mut payload = vec![0; size as usize];
            file.read_exact(&mut payload)?;
            values.push(payload);
        }
        offset = u64::from_be_bytes(bytes[8..16].try_into().unwrap());
    }
    ensure!(
        !entries.is_empty(),
        "SAME CD-i CHD has no CD track metadata"
    );
    file.rewind()?;
    Ok(entries)
}

/// Container preflight only. Track metadata and decoded sectors still need
/// separate validation before this can establish a usable CD-i disc.
pub(crate) fn validate_chd_header(file: &mut std::fs::File) -> Result<chd::header::Header> {
    let metadata = file.metadata()?;
    ensure!(
        metadata.is_file() && metadata.len() > 0 && metadata.len() <= 4 * 1024 * 1024 * 1024,
        "SAME CD-i CHD must be a bounded regular disc image"
    );
    file.rewind()?;
    let header = chd::header::Header::try_read_header(file)?;
    // cdrom_image_device opens direct content without a parent CHD. Do not
    // resolve a sibling parent using another core's loader semantics.
    ensure!(
        !header.has_parent(),
        "SAME CD-i direct-disc loader does not supply a parent CHD; use a standalone image"
    );
    const FRAME: u32 = 2448;
    ensure!(
        header.unit_bytes() == FRAME
            && header.hunk_size() >= FRAME
            && header.hunk_size() <= 16 * 1024 * 1024
            && header.hunk_size() % FRAME == 0,
        "SAME CD-i CHD requires 2448-byte CD units and aligned hunks"
    );
    ensure!(
        header.logical_bytes() > 0
            && header.logical_bytes() <= 4 * 1024 * 1024 * 1024
            && header.logical_bytes() % u64::from(FRAME) == 0
            && header.hunk_count() > 0
            && u64::from(header.hunk_count())
                == header
                    .logical_bytes()
                    .div_ceil(u64::from(header.hunk_size())),
        "SAME CD-i CHD has invalid logical disc geometry"
    );
    file.rewind()?;
    Ok(header)
}

/// Match chdcd_parse_iso's ordered size checks, including sizes divisible by
/// more than one sector width. Geometry alone does not prove a bootable CD-i.
pub(crate) fn iso_sector_size(length: u64) -> Result<u64> {
    ensure!(
        length > 0 && length <= 4 * 1024 * 1024 * 1024,
        "SAME CD-i ISO is empty or exceeds disc-image limits"
    );
    [2048, 2336, 2352]
        .into_iter()
        .find(|size| length % size == 0)
        .ok_or_else(|| {
            anyhow::anyhow!("SAME CD-i ISO length is not supported by the native sector parser")
        })
}

pub(crate) struct IsoSnapshot {
    path: PathBuf,
    canonical: PathBuf,
    file: std::fs::File,
    metadata: std::fs::Metadata,
    pub(crate) sector_size: u64,
}

fn unchanged_disc(left: &std::fs::Metadata, right: &std::fs::Metadata) -> bool {
    if !left.is_file()
        || !right.is_file()
        || left.len() != right.len()
        || left.modified().ok().is_none()
        || left.modified().ok() != right.modified().ok()
    {
        return false;
    }
    #[cfg(unix)]
    {
        use std::os::unix::fs::MetadataExt;
        if left.dev() != right.dev()
            || left.ino() != right.ino()
            || left.ctime() != right.ctime()
            || left.ctime_nsec() != right.ctime_nsec()
        {
            return false;
        }
    }
    true
}

impl IsoSnapshot {
    pub(crate) fn native_sector(&mut self, lba: u32) -> Result<[u8; 2560]> {
        self.verify()?;
        let offset = u64::from(lba) * self.sector_size;
        ensure!(
            offset + self.sector_size <= self.metadata.len(),
            "SAME CD-i ISO sector is beyond the image"
        );
        self.file.seek(std::io::SeekFrom::Start(offset))?;
        let mut output = [0; 2560];
        self.file
            .read_exact(&mut output[..self.sector_size as usize])?;
        self.verify()?;
        Ok(output)
    }

    pub(crate) fn prepare(path: &Path) -> Result<Self> {
        ensure!(path.is_absolute(), "SAME CD-i disc path must be absolute");
        let canonical = path.canonicalize()?;
        let before = std::fs::metadata(&canonical)?;
        ensure!(before.is_file(), "SAME CD-i ISO must be a regular file");
        let sector_size = iso_sector_size(before.len())?;
        let file = std::fs::File::open(&canonical)?;
        let snapshot = Self {
            path: path.to_owned(),
            canonical,
            file,
            metadata: before,
            sector_size,
        };
        snapshot.verify()?;
        Ok(snapshot)
    }

    pub(crate) fn verify(&self) -> Result<()> {
        ensure!(
            self.path.canonicalize()? == self.canonical
                && unchanged_disc(&self.metadata, &self.file.metadata()?)
                && unchanged_disc(&self.metadata, &std::fs::metadata(&self.canonical)?),
            "SAME CD-i ISO changed during launch preparation"
        );
        Ok(())
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Bios {
    Magnavox200,
    Philips220,
}

impl Bios {
    pub(crate) fn native_name(self) -> &'static str {
        match self {
            Self::Magnavox200 => "mcdi200",
            Self::Philips220 => "pcdi220",
        }
    }
}

/// Check user-supplied cdimono1.zip bytes against the pinned driver's ROM
/// declarations. This does not fetch firmware or establish disc compatibility.
/// Require canonical root member names so MAME's CRC-name fallback cannot pick
/// a different member from the one inspected here.
pub(crate) fn validate_bios_archive(bytes: &[u8]) -> Result<Bios> {
    ensure!(
        bytes.len() <= 8 * 1024 * 1024,
        "SAME CD-i BIOS archive exceeds 8 MiB"
    );
    let mut archive = zip::ZipArchive::new(std::io::Cursor::new(bytes))?;
    ensure!(
        archive.len() <= 1024,
        "SAME CD-i BIOS archive has too many members"
    );
    let requirements = [
        (
            "cdi200.rom",
            0x80000,
            "d961de803c89b3d1902d656ceb9ce7c02dccb40a",
        ),
        (
            "cdi220b.rom",
            0x80000,
            "53360a1f21ddac952e95306ced64186a3fc0b93e",
        ),
        (
            "zx405037p__cdi_servo_2.1__b43t__llek9215.mc68hc705c8a_withtestrom.7201",
            0x2000,
            "fdf8d78d6a0df4a56b5b963d72eabd39fcec163f",
        ),
        (
            "zx405042p__cdi_slave_2.0__b43t__zzmk9213.mc68hc705c8a_withtestrom.7206",
            0x2000,
            "56d0acd7caad51c7de703247cd6d842b36173079",
        ),
    ];
    let mut found = [false; 4];
    let mut names = std::collections::BTreeSet::new();
    for index in 0..archive.len() {
        let mut member = archive.by_index(index)?;
        let name = member.name().to_owned();
        ensure!(
            names.insert(name.to_ascii_lowercase()),
            "Duplicate SAME CD-i BIOS member: {name}"
        );
        let Some(slot) = requirements
            .iter()
            .position(|(expected, _, _)| name.eq_ignore_ascii_case(expected))
        else {
            continue;
        };
        let (expected, size, digest) = requirements[slot];
        ensure!(
            name == expected && !member.is_dir() && member.size() == size,
            "SAME CD-i BIOS member has unsupported name or size: {name}"
        );
        let mut data = Vec::new();
        (&mut member).take(size + 1).read_to_end(&mut data)?;
        ensure!(
            data.len() as u64 == size && format!("{:x}", sha1::Sha1::digest(&data)) == digest,
            "SAME CD-i BIOS checksum mismatch: {name}"
        );
        found[slot] = true;
    }
    ensure!(
        found[2] && found[3],
        "SAME CD-i BIOS archive requires the driver's servo and slave MCU dumps"
    );
    if found[0] {
        Ok(Bios::Magnavox200)
    } else if found[1] {
        Ok(Bios::Philips220)
    } else {
        anyhow::bail!(
            "SAME CD-i BIOS archive requires cdi200.rom or cdi220b.rom; the nonbooting alternate BIOS is unsupported"
        )
    }
}

pub(crate) fn mode_for_layout(layout: &str) -> Result<PointerMode> {
    match layout {
        "same-cdi-stick-pointer" => Ok(PointerMode::LeftStick),
        "same-cdi-dpad-pointer" => Ok(PointerMode::Dpad),
        _ => anyhow::bail!("Unknown SAME CD-i controller-driven pointer mode"),
    }
}

pub(crate) fn validate_options(options: &std::collections::BTreeMap<String, String>) -> Result<()> {
    for (key, expected) in [
        ("same_cdi_read_config", "disabled"),
        ("same_cdi_write_config", "disabled"),
        ("same_cdi_auto_save", "disabled"),
        ("same_cdi_boot_from_cli", "disabled"),
        ("same_cdi_mame_paths_enable", "disabled"),
        ("same_cdi_mouse_enable", "disabled"),
        ("same_cdi_lightgun_mode", "none"),
        ("same_cdi_buttons_profiles", "enabled"),
        ("same_cdi_mame_4way_enable", "disabled"),
    ] {
        ensure!(
            options.get(key).map(String::as_str) == Some(expected),
            "SAME CD-i fixed pointer contract requires {key} = {expected}"
        );
    }
    Ok(())
}

#[derive(PartialEq, Eq)]
struct FileStamp {
    canonical: PathBuf,
    digest: [u8; 32],
}

fn read_native_file(path: &Path) -> Result<(Option<FileStamp>, Vec<u8>)> {
    match std::fs::symlink_metadata(path) {
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
            return Ok((None, Vec::new()));
        }
        Err(error) => return Err(error.into()),
        Ok(_) => {}
    }
    let canonical = path.canonicalize()?;
    let before = std::fs::metadata(&canonical)?;
    const LIMIT: u64 = 8 * 1024 * 1024;
    ensure!(
        before.is_file() && before.len() <= LIMIT,
        "SAME CD-i configuration must be a bounded regular file: {}",
        path.display()
    );
    let file = std::fs::File::open(&canonical)?;
    let mut bytes = Vec::new();
    (&file).take(LIMIT + 1).read_to_end(&mut bytes)?;
    let after = file.metadata()?;
    ensure!(
        bytes.len() as u64 == before.len()
            && after.len() == before.len()
            && before.modified().ok().is_some()
            && before.modified().ok() == after.modified().ok()
            && path.canonicalize()? == canonical,
        "SAME CD-i configuration changed while reading"
    );
    Ok((
        Some(FileStamp {
            canonical,
            digest: Sha256::digest(&bytes).into(),
        }),
        bytes,
    ))
}

pub(crate) struct PreparedConfiguration {
    _directory: tempfile::TempDir,
    pub(crate) command_file: PathBuf,
    source_files: Vec<(PathBuf, Option<FileStamp>)>,
    owned_files: Vec<(PathBuf, Option<FileStamp>)>,
    roots: Vec<(PathBuf, PathBuf)>,
    media: DiscSnapshot,
}

impl PreparedConfiguration {
    pub(crate) fn verify(&self) -> Result<()> {
        self.media.verify()?;
        for (path, canonical) in &self.roots {
            ensure!(
                path.is_dir() && &path.canonicalize()? == canonical,
                "SAME CD-i native directory changed during preparation"
            );
        }
        for (path, expected) in self.source_files.iter().chain(&self.owned_files) {
            ensure!(
                &read_native_file(path)?.0 == expected,
                "SAME CD-i source or owned configuration changed: {}",
                path.display()
            );
        }
        Ok(())
    }
}

/// Materialize only session-owned files. Core writes remain inside this cfg
/// directory; neither original cfg files nor the NVRAM directory are moved.
pub(crate) fn prepare_configuration(
    system: &Path,
    save: &Path,
    content: &Path,
    mode: PointerMode,
) -> Result<PreparedConfiguration> {
    let media = DiscSnapshot::open(content)?;
    let mut roots = Vec::new();
    for path in [system, save] {
        ensure!(
            path.is_absolute() && path.is_dir(),
            "SAME CD-i requires existing absolute frontend directories"
        );
        roots.push((path.to_path_buf(), path.canonicalize()?));
    }
    let source = save.join("same_cdi/cfg");
    let default_path = source.join("default.cfg");
    let machine_path = source.join("cdimono1.cfg");
    let (default_stamp, default_bytes) = read_native_file(&default_path)?;
    let (machine_stamp, machine_bytes) = read_native_file(&machine_path)?;
    let machine_text = if machine_stamp.is_some() {
        Some(std::str::from_utf8(&machine_bytes)?)
    } else {
        None
    };
    let merged = merge_configuration(machine_text, mode)?;
    let has_default = default_stamp.is_some();
    let mut source_files = vec![(default_path, default_stamp), (machine_path, machine_stamp)];
    let content_parent = content
        .parent()
        .ok_or_else(|| anyhow::anyhow!("SAME CD-i disc has no parent directory"))?;
    let mut firmware = None;
    for path in [
        content_parent.join("cdimono1.zip"),
        system.join("same_cdi/bios/cdimono1.zip"),
    ] {
        let (stamp, bytes) = read_native_file(&path)?;
        let present = stamp.is_some();
        source_files.push((path, stamp));
        if present {
            let bios = validate_bios_archive(&bytes)?;
            firmware = Some((bios, bytes));
            break;
        }
    }
    let (bios, bios_bytes) = firmware.ok_or_else(|| {
        anyhow::anyhow!("SAME CD-i fixed pointer mode requires cdimono1.zip beside the disc or in system/same_cdi/bios")
    })?;
    let directory = tempfile::Builder::new().prefix("lunchbox-cdi-").tempdir()?;
    let cfg = directory.path().join("cfg");
    std::fs::create_dir(&cfg)?;
    let bios_directory = directory.path().join("bios");
    std::fs::create_dir(&bios_directory)?;
    let disc_name = content
        .file_name()
        .ok_or_else(|| anyhow::anyhow!("SAME CD-i disc has no filename"))?;
    // RetroArch names frontend states from the loaded content's stem. Keep
    // that stem when substituting the owned command file for the real disc.
    let command_file = directory.path().join(disc_name).with_extension("cmd");
    let command = command_line(content, &cfg, &bios_directory, bios)?;
    let mut outputs = vec![
        (cfg.join("cdimono1.cfg"), merged.into_bytes()),
        (command_file.clone(), command.into_bytes()),
        (bios_directory.join("cdimono1.zip"), bios_bytes),
    ];
    if has_default {
        outputs.push((cfg.join("default.cfg"), default_bytes));
    }
    let mut owned_files = Vec::new();
    for (path, bytes) in outputs {
        std::fs::OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&path)?
            .write_all(&bytes)?;
        let stamp = read_native_file(&path)?.0;
        ensure!(
            stamp.is_some(),
            "SAME CD-i owned file disappeared during preparation"
        );
        owned_files.push((path, stamp));
    }
    let prepared = PreparedConfiguration {
        _directory: directory,
        command_file,
        roots,
        source_files,
        owned_files,
        media,
    };
    prepared.verify()?;
    Ok(prepared)
}

/// Preserve all source bytes outside the five controlled input fields. The
/// result belongs in an owned cfg directory, never over the original file.
pub(crate) fn merge_configuration(source: Option<&str>, mode: PointerMode) -> Result<String> {
    use quick_xml::{Reader, events::Event};
    let input = retropad_input_xml(mode);
    let ports = input
        .strip_prefix("<input>\n")
        .unwrap()
        .strip_suffix("</input>\n")
        .unwrap();
    let Some(source) = source else {
        return Ok(format!(
            "<?xml version=\"1.0\"?>\n<mameconfig version=\"10\">\n<system name=\"cdimono1\">\n<video><target index=\"0\" view=\"Main Screen Standard (4:3)\" /></video>\n{input}</system>\n</mameconfig>\n"
        ));
    };
    ensure!(
        source.len() <= 8 * 1024 * 1024,
        "SAME CD-i configuration exceeds 8 MiB"
    );
    let mut reader = Reader::from_str(source);
    let mut stack: Vec<String> = Vec::new();
    let mut edits: Vec<(usize, usize, String)> = Vec::new();
    let mut roots = 0;
    let mut systems = 0;
    let mut active = false;
    let mut inputs = 0;
    let mut nodes = 0;
    loop {
        let start = reader.buffer_position() as usize;
        let event = reader.read_event()?;
        let end = reader.buffer_position() as usize;
        match event {
            Event::Start(ref element) | Event::Empty(ref element) => {
                let empty = matches!(event, Event::Empty(_));
                nodes += 1;
                ensure!(
                    nodes <= 100000 && stack.len() < 64,
                    "SAME CD-i XML exceeds structural limits"
                );
                let name = std::str::from_utf8(element.name().as_ref())?.to_owned();
                let mut attributes = std::collections::BTreeMap::new();
                for attribute in element.attributes() {
                    let attribute = attribute?;
                    let key = std::str::from_utf8(attribute.key.as_ref())?.to_owned();
                    let value = attribute.unescape_value()?.into_owned();
                    ensure!(
                        attributes.insert(key, value).is_none(),
                        "Duplicate SAME CD-i XML attribute"
                    );
                }
                let attr = |key: &str| attributes.get(key).map(String::as_str);
                if stack.is_empty() {
                    roots += 1;
                    ensure!(
                        name == "mameconfig" && attr("version") == Some("10") && roots == 1,
                        "SAME CD-i requires a version-10 mameconfig root"
                    );
                    if empty {
                        let opening = source[start..end].strip_suffix("/>").unwrap();
                        edits.push((
                            start,
                            end,
                            format!(
                                "{opening}><system name=\"cdimono1\">{input}</system></mameconfig>"
                            ),
                        ));
                    }
                }
                if stack.len() == 1 && name == "system" && attr("name") == Some("cdimono1") {
                    systems += 1;
                    ensure!(
                        systems == 1,
                        "Ambiguous duplicate cdimono1 configuration systems"
                    );
                    if empty {
                        // Preserve additional attributes by expanding the exact
                        // self-closing source start tag instead of serializing it.
                        let opening = source[start..end].strip_suffix("/>").unwrap();
                        edits.push((start, end, format!("{opening}>{input}</system>")));
                    } else {
                        active = true;
                    }
                }
                if active && stack.len() == 2 && name == "input" {
                    inputs += 1;
                    ensure!(inputs == 1, "Ambiguous duplicate SAME CD-i input sections");
                    if empty {
                        let opening = source[start..end].strip_suffix("/>").unwrap();
                        edits.push((start, end, format!("{opening}>{ports}</input>")));
                    }
                }
                if active && stack.len() == 3 && stack[2] == "input" && name == "port" {
                    let integer = |key: &str| -> Option<u32> {
                        let value = attr(key)?.trim();
                        if let Some(hex) = value
                            .strip_prefix("0x")
                            .or_else(|| value.strip_prefix("0X"))
                        {
                            u32::from_str_radix(hex, 16).ok()
                        } else {
                            value.parse().ok()
                        }
                    };
                    let controlled = match (attr("tag"), attr("type"), integer("mask")) {
                        (Some(":slave_hle:MOUSEX"), Some("P1_MOUSE_X"), Some(65535))
                        | (Some(":slave_hle:MOUSEY"), Some("P1_MOUSE_Y"), Some(65535))
                        | (Some(":slave_hle:MOUSEBTN"), Some("P1_BUTTON1"), Some(1))
                        | (Some(":slave_hle:MOUSEBTN"), Some("P1_BUTTON2"), Some(2))
                        | (Some(":slave_hle:MOUSEBTN"), Some("P1_BUTTON3"), Some(4)) => {
                            integer("defvalue").unwrap_or(0) == 0
                        }
                        _ => false,
                    };
                    if controlled {
                        if !empty {
                            reader.read_to_end(element.name())?;
                        }
                        edits.push((start, reader.buffer_position() as usize, String::new()));
                        continue;
                    }
                }
                if !empty {
                    stack.push(name);
                }
            }
            Event::End(_) => {
                if active && stack.len() == 3 && stack[2] == "input" {
                    edits.push((start, start, ports.to_owned()));
                }
                if active && stack.len() == 2 && stack[1] == "system" {
                    if inputs == 0 {
                        edits.push((start, start, input.clone()));
                    }
                    active = false;
                }
                if stack.len() == 1 && systems == 0 {
                    edits.push((
                        start,
                        start,
                        format!("<system name=\"cdimono1\">{input}</system>\n"),
                    ));
                }
                ensure!(stack.pop().is_some(), "Unbalanced SAME CD-i XML");
            }
            Event::DocType(_) => anyhow::bail!("SAME CD-i configuration DTDs are unsupported"),
            Event::Text(text) if stack.is_empty() => {
                ensure!(
                    text.unescape()?.trim().is_empty(),
                    "Text outside SAME CD-i XML root"
                );
            }
            Event::CData(_) if stack.is_empty() => {
                anyhow::bail!("CDATA outside SAME CD-i XML root")
            }
            Event::Eof => break,
            _ => {}
        }
    }
    ensure!(
        roots == 1 && stack.is_empty(),
        "Incomplete SAME CD-i XML document"
    );
    edits.sort_by_key(|(start, _, _)| *start);
    let mut output = source.to_owned();
    for (start, end, replacement) in edits.into_iter().rev() {
        output.replace_range(start..end, &replacement);
    }
    Ok(output)
}

/// Build the first line of an owned .cmd file. The wrapper appends these fixed
/// arguments after its usual path options, replacing cfg_directory and rompath;
/// save/NVRAM/state paths continue to use the effective frontend directories.
pub(crate) fn command_line(
    content: &Path,
    cfg_directory: &Path,
    bios_directory: &Path,
    bios: Bios,
) -> Result<String> {
    let path_text = |path: &Path| -> Result<String> {
        ensure!(
            path.is_absolute(),
            "SAME CD-i command paths must be absolute"
        );
        let value = path
            .to_str()
            .ok_or_else(|| anyhow::anyhow!("SAME CD-i command paths must be UTF-8"))?;
        ensure!(
            !value
                .chars()
                .any(|c| c.is_control() || matches!(c, '"' | ';')),
            "SAME CD-i path contains unsupported command or search-path delimiters"
        );
        Ok(value.to_owned())
    };
    ensure!(
        content
            .extension()
            .and_then(|v| v.to_str())
            .is_some_and(|v| ["chd", "iso", "cue"]
                .iter()
                .any(|extension| v.eq_ignore_ascii_case(extension))),
        "SAME CD-i requires CHD, ISO or CUE content"
    );
    let rom_path = path_text(bios_directory)?;
    let bios = bios.native_name();
    let config = path_text(cfg_directory)?;
    let content = path_text(content)?;
    // The last argument must remain the disc: execute_game_cmd extracts the
    // game name from it for per-game NVRAM routing.
    let line = format!(
        "cdimono1 -cfg_directory \"{config}\" -rp \"{rom_path}\" -bios {bios} -cdrom \"{content}\"\n"
    );
    ensure!(
        line.len() <= 511,
        "SAME CD-i command exceeds the core's first-line capacity; use shorter paths"
    );
    Ok(line)
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum PointerMode {
    LeftStick,
    Dpad,
}

/// The input subtree for cdimono1.cfg. A caller must merge this with preserved
/// native configuration and arrange an owned configuration directory; this is
/// not a complete configuration file and does not itself write user files.
pub(crate) fn retropad_input_xml(mode: PointerMode) -> String {
    let mut xml = String::from("<input>\n");
    for (axis, negative, positive) in [("X", "LEFT", "RIGHT"), ("Y", "UP", "DOWN")] {
        let (standard, decrement, increment) = match mode {
            PointerMode::LeftStick => (
                format!("JOYCODE_1_{axis}AXIS"),
                "NONE".into(),
                "NONE".into(),
            ),
            PointerMode::Dpad => (
                "NONE".into(),
                format!("JOYCODE_1_HAT1{negative}"),
                format!("JOYCODE_1_HAT1{positive}"),
            ),
        };
        xml.push_str(&format!(
            "  <port tag=\":slave_hle:MOUSE{axis}\" type=\"P1_MOUSE_{axis}\" mask=\"65535\" defvalue=\"0\" keydelta=\"2\" centerdelta=\"0\" sensitivity=\"100\" reverse=\"no\">\n    <newseq type=\"standard\">{standard}</newseq>\n    <newseq type=\"decrement\">{decrement}</newseq>\n    <newseq type=\"increment\">{increment}</newseq>\n  </port>\n"
        ));
    }
    // input_retro.cpp registers BUTTON1/2/3 from RetroPad A/B/X.
    // The CD-i slave translates field 3 to the buttons-1+2 protocol chord.
    for button in 1..=3 {
        let mask = 1 << (button - 1);
        xml.push_str(&format!(
            "  <port tag=\":slave_hle:MOUSEBTN\" type=\"P1_BUTTON{button}\" mask=\"{mask}\" defvalue=\"0\" value=\"0\" toggle=\"no\">\n    <newseq type=\"standard\">JOYCODE_1_BUTTON{button}</newseq>\n  </port>\n"
        ));
    }
    xml.push_str("</input>\n");
    xml
}
