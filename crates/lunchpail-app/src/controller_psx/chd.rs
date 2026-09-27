//! Cooked-sector adapter for Beetle's supported CD CHDs. The codec is provided
//! by the BSD-licensed `chd` crate; track addressing follows the pinned core's
//! CDAccess_CHD contract, not the filename or CHD's descriptive metadata.
use super::CookedSectorReader;
use ::chd::{Chd, header::Header};
use anyhow::{Context, Result, ensure};
use std::collections::{BTreeMap, BTreeSet};
use std::fs::File;
use std::io::{Read, Seek, SeekFrom};
use std::path::Path;

const FRAME: u64 = 2448;

#[derive(Debug)]
struct Track {
    start: i64,
    data_start: i64,
    end: i64,
    post_end: i64,
    file_start: u64,
    audio: bool,
}

pub(super) struct ChdDisc {
    image: Chd<File>,
    tracks: Vec<Track>,
    decoded: Vec<u8>,
    compressed: Vec<u8>,
    cached: Option<u32>,
}

fn checked_header(file: &mut File) -> Result<Header> {
    file.rewind()?;
    let header = Header::try_read_header(file)?;
    ensure!(
        header.hunk_size() >= FRAME as u32
            && header.hunk_size() <= 16 * 1024 * 1024
            && u64::from(header.hunk_size()) % FRAME == 0,
        "Invalid CD CHD hunk geometry"
    );
    ensure!(
        header.hunk_count() > 0
            && header.hunk_count() <= 1_000_000
            && header.logical_bytes() <= 4 * 1024 * 1024 * 1024,
        "CHD exceeds CD image limits"
    );
    Ok(header)
}

fn open_chain(
    path: &Path,
    directory: &Path,
    visiting: &mut BTreeSet<std::path::PathBuf>,
) -> Result<Chd<File>> {
    let path = path.canonicalize()?;
    ensure!(
        visiting.len() < 8 && visiting.insert(path.clone()),
        "Cyclic or overlong CHD parent chain"
    );
    let mut file = File::open(&path)?;
    let header = checked_header(&mut file)?;
    let parent = if header.has_parent() {
        let wanted = header.parent_sha1().context("CHD parent has no SHA-1")?;
        ensure!(wanted != [0; 20], "CHD parent SHA-1 is empty");
        let mut matched = None;
        let mut candidates_seen = BTreeSet::new();
        for entry in std::fs::read_dir(directory)? {
            let entry = entry?;
            let candidate = entry.path();
            if !candidate
                .extension()
                .is_some_and(|s| s.eq_ignore_ascii_case("chd"))
                || !candidate.is_file()
            {
                continue;
            }
            let candidate = candidate.canonicalize()?;
            if candidate == path || !candidates_seen.insert(candidate.clone()) {
                continue;
            }
            let Ok(mut parent_file) = File::open(&candidate) else {
                continue;
            };
            let Ok(parent_header) = checked_header(&mut parent_file) else {
                continue;
            };
            if parent_header.sha1() == Some(wanted) {
                ensure!(
                    matched.is_none(),
                    "Multiple CHDs claim the required parent identity"
                );
                matched = Some(candidate);
            }
        }
        let parent = matched.context("Required parent CHD was not found in the disc directory")?;
        Some(Box::new(open_chain(&parent, directory, visiting)?))
    } else {
        None
    };
    file.rewind()?;
    let image = Chd::open(file, parent)?;
    visiting.remove(&path);
    Ok(image)
}

// Use a bounded metadata walk instead of an iterator that can silently truncate
// on I/O errors. Unknown metadata is skipped without allocating its payload.
fn track_metadata(file: &mut File, header: &Header) -> Result<Vec<String>> {
    let length = file.metadata()?.len();
    let mut offset = header.meta_offset().unwrap_or(0);
    let mut seen = BTreeSet::new();
    let mut entries: BTreeMap<[u8; 4], Vec<String>> = BTreeMap::new();
    while offset != 0 {
        ensure!(
            seen.len() < 4096 && seen.insert(offset),
            "Cyclic or excessive CHD metadata"
        );
        ensure!(
            offset.checked_add(16).is_some_and(|end| end <= length),
            "CHD metadata header extends outside file"
        );
        file.seek(SeekFrom::Start(offset))?;
        let mut bytes = [0; 16];
        file.read_exact(&mut bytes)?;
        let tag: [u8; 4] = bytes[..4].try_into().unwrap();
        let size = u32::from_be_bytes(bytes[4..8].try_into().unwrap()) & 0x00ff_ffff;
        ensure!(
            offset
                .checked_add(16 + u64::from(size))
                .is_some_and(|end| end <= length),
            "CHD metadata payload extends outside file"
        );
        if matches!(&tag, b"CHT2" | b"CHTR") {
            ensure!(size <= 256, "CHD track metadata is too large");
            let values = entries.entry(tag).or_default();
            ensure!(values.len() < 99, "Too many CHD tracks");
            let mut value = vec![0; size as usize];
            file.read_exact(&mut value)?;
            let text = value.split(|b| *b == 0).next().unwrap();
            values.push(std::str::from_utf8(text)?.to_owned());
        }
        offset = u64::from_be_bytes(bytes[8..16].try_into().unwrap());
    }
    let modern = entries.remove(b"CHT2").unwrap_or_default();
    let old = entries.remove(b"CHTR").unwrap_or_default();
    // Beetle looks up each track by ordinal in CHT2, falling back to CHTR.
    Ok((0..modern.len().max(old.len()))
        .map(|i| modern.get(i).or_else(|| old.get(i)).unwrap().clone())
        .collect())
}

fn tracks(metadata: &[String], logical_bytes: u64) -> Result<Vec<Track>> {
    ensure!(
        !metadata.is_empty(),
        "CHD has no supported CD track metadata"
    );
    let mut result = Vec::new();
    let mut disc_cursor = -150i64;
    let mut file_cursor = 0u64;
    for (index, text) in metadata.iter().enumerate() {
        let mut fields = BTreeMap::new();
        for field in text.split_ascii_whitespace() {
            let (key, value) = field
                .split_once(':')
                .context("Malformed CHD track metadata")?;
            ensure!(
                fields.insert(key, value).is_none(),
                "Duplicate CHD track field"
            );
        }
        let number = |name| -> Result<u64> {
            Ok(fields
                .get(name)
                .context("Missing CHD track field")?
                .parse()?)
        };
        let expected = if fields.contains_key("PREGAP") {
            [
                "TRACK", "TYPE", "SUBTYPE", "FRAMES", "PREGAP", "PGTYPE", "PGSUB", "POSTGAP",
            ]
            .as_slice()
        } else {
            ["TRACK", "TYPE", "SUBTYPE", "FRAMES"].as_slice()
        };
        ensure!(
            fields.len() == expected.len() && expected.iter().all(|key| fields.contains_key(key)),
            "Incomplete or unknown CHD track fields"
        );
        ensure!(
            number("TRACK")? == index as u64 + 1,
            "CHD tracks are not consecutive from track 1"
        );
        let mode = *fields.get("TYPE").context("Missing CHD track type")?;
        let subtype = *fields
            .get("SUBTYPE")
            .context("Missing CHD subchannel type")?;
        ensure!(
            matches!(mode, "MODE2_RAW" | "AUDIO") && matches!(subtype, "NONE" | "RW" | "RW_RAW"),
            "CHD track format is not supported by Beetle PSX"
        );
        let frames = number("FRAMES")?;
        let pregap = if fields.contains_key("PREGAP") {
            number("PREGAP")?
        } else {
            0
        };
        let postgap = if fields.contains_key("POSTGAP") {
            number("POSTGAP")?
        } else {
            0
        };
        ensure!(
            frames > 0 && frames <= 2_000_000 && pregap <= frames && postgap <= 2_000_000,
            "Invalid CHD track extent"
        );
        let recorded_pregap = if fields.get("PGTYPE").is_some_and(|t| t.starts_with('V')) {
            pregap
        } else {
            0
        };
        let synthetic_pregap = if index == 0 {
            150
        } else {
            pregap - recorded_pregap
        };
        let data_start = disc_cursor + synthetic_pregap as i64;
        let start = data_start + recorded_pregap as i64;
        let end = start + (frames - recorded_pregap) as i64;
        let post_end = end + postgap as i64;
        let file_start = file_cursor + recorded_pregap;
        ensure!(
            (file_cursor + frames)
                .checked_mul(FRAME)
                .is_some_and(|bytes| bytes <= logical_bytes),
            "CHD track extends outside logical image"
        );
        result.push(Track {
            start,
            data_start,
            end,
            post_end,
            file_start,
            audio: mode == "AUDIO",
        });
        disc_cursor = post_end;
        file_cursor += frames + postgap + (4 - frames % 4) % 4;
    }
    Ok(result)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn track_addressing_accounts_for_recorded_and_synthetic_gaps() {
        let metadata = vec![
            "TRACK:1 TYPE:MODE2_RAW SUBTYPE:RW_RAW FRAMES:101 PREGAP:5 PGTYPE:V PG SUBTYPE:ignored"
                .to_owned(),
        ];
        assert!(tracks(&metadata, 1000 * FRAME).is_err());
        let metadata = vec![
            "TRACK:1 TYPE:MODE2_RAW SUBTYPE:RW_RAW FRAMES:101 PREGAP:5 PGTYPE:V PGSUB:NONE POSTGAP:2".into(),
            "TRACK:2 TYPE:MODE2_RAW SUBTYPE:NONE FRAMES:50 PREGAP:7 PGTYPE:MODE2_RAW PGSUB:NONE POSTGAP:0".into(),
        ];
        let actual = tracks(&metadata, 1000 * FRAME).unwrap();
        assert_eq!(
            (
                actual[0].data_start,
                actual[0].start,
                actual[0].end,
                actual[0].post_end,
                actual[0].file_start
            ),
            (0, 5, 101, 103, 5)
        );
        assert_eq!(
            (
                actual[1].data_start,
                actual[1].start,
                actual[1].end,
                actual[1].file_start
            ),
            (110, 110, 160, 106)
        );
        assert!(tracks(&metadata, 105 * FRAME).is_err());
        assert!(
            tracks(
                &["TRACK:2 TYPE:MODE2_RAW SUBTYPE:NONE FRAMES:32".into()],
                32 * FRAME
            )
            .is_err()
        );
        assert!(
            tracks(
                &["TRACK:1 TYPE:MODE1_RAW SUBTYPE:NONE FRAMES:32".into()],
                32 * FRAME
            )
            .is_err()
        );
        assert!(
            tracks(
                &["TRACK:1 TYPE:MODE2_RAW SUBTYPE:NONE FRAMES:32 PREGAP:0".into()],
                32 * FRAME
            )
            .is_err()
        );
    }
}

impl ChdDisc {
    pub(super) fn open(path: &Path) -> Result<Self> {
        let mut image = open_chain(
            path,
            path.parent().context("CHD has no parent directory")?,
            &mut BTreeSet::new(),
        )?;
        let header = image.header().clone();
        let metadata = track_metadata(image.inner(), &header)?;
        let tracks = tracks(&metadata, header.logical_bytes())?;
        let decoded = image.get_hunksized_buffer();
        Ok(Self {
            image,
            tracks,
            decoded,
            compressed: Vec::new(),
            cached: None,
        })
    }
}

impl CookedSectorReader for ChdDisc {
    fn sector(&mut self, lba: u32) -> Result<[u8; 2048]> {
        let lba = i64::from(lba);
        let track = self
            .tracks
            .iter()
            .find(|track| lba < track.post_end)
            .context("ISO sector lies beyond CHD tracks")?;
        ensure!(
            !track.audio && lba >= track.data_start && lba < track.end,
            "ISO sector lies in audio or synthesized CHD gap"
        );
        let frame = (lba - track.start + track.file_start as i64) as u64;
        let frames_per_hunk = self.decoded.len() as u64 / FRAME;
        let hunk = u32::try_from(frame / frames_per_hunk)?;
        let offset = ((frame % frames_per_hunk) * FRAME) as usize;
        if self.cached != Some(hunk) {
            self.image
                .hunk(hunk)?
                .read_hunk_in(&mut self.compressed, &mut self.decoded)?;
            self.cached = Some(hunk);
        }
        super::cooked_raw(&self.decoded[offset..offset + 2352])
    }
}
