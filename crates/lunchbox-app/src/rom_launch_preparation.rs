use std::borrow::Cow;
use std::collections::{BTreeMap, BTreeSet};
use std::fs::{self, File};
use std::io::{Read, Write};
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};

use anyhow::{Context, Result, anyhow, bail};
use directories::ProjectDirs;
use rars::{ArchiveFamily, ArchiveReadOptions, ArchiveReader};
use uuid::Uuid;

pub const LAUNCH_CANCELLED_ERROR: &str = "launch preparation was cancelled";

const MAX_PLAYLIST_BYTES: u64 = 1024 * 1024;
const MAX_PLAYLIST_ENTRIES: usize = 100;
const MAX_ARCHIVE_MEMBERS: usize = 100_000;
const MAX_EXTRACTED_BYTES: u64 = 128 * 1024 * 1024 * 1024;
const MIN_FREE_SPACE_RESERVE: u64 = 512 * 1024 * 1024;
const SESSION_PREFIX: &str = "session-";

const ARCHIVE_EXTENSIONS: &[&str] = &["zip", "7z", "rar"];
const CONTROL_EXTENSIONS: &[&str] = &["m3u", "cue", "gdi", "ccd", "mds"];
const PRIMARY_IMAGE_EXTENSIONS: &[&str] = &[
    "chd", "iso", "cso", "ciso", "gcm", "gcz", "rvz", "wbfs", "pbp", "pkg", "xci", "nsp", "nsz",
    "xcz",
];
const AUXILIARY_IMAGE_EXTENSIONS: &[&str] = &["bin", "img", "sub", "mdf", "raw"];
const AUDIO_EXTENSIONS: &[&str] = &["wav", "flac", "ape", "ogg", "mp3"];
const ROM_EXTENSIONS: &[&str] = &[
    "a26", "a52", "a78", "atr", "atx", "car", "cas", "cia", "col", "dsi", "fds", "fig", "gba",
    "gbc", "gen", "gg", "int", "j64", "jag", "lnx", "md", "n64", "nds", "nes", "ngc", "ngp", "nro",
    "nso", "pce", "sfc", "sgb", "sgx", "smc", "smd", "sms", "swc", "unf", "unh", "unif", "v64",
    "vec", "wad", "ws", "wsc", "xex", "xfd", "z64", "32x", "3ds",
];

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PreparedLaunchMedia {
    pub path: PathBuf,
    pub cleanup_paths: Vec<PathBuf>,
    pub access_roots: Vec<PathBuf>,
    pub archive_count: usize,
    pub playlist_entries: usize,
}

#[derive(Clone, Debug)]
struct ArchiveMember {
    relative_path: PathBuf,
    key: String,
    extension: String,
    size: u64,
}

#[derive(Clone, Debug)]
struct ArchivePlan {
    members: Vec<ArchiveMember>,
    primary: ArchiveMember,
    selected_keys: BTreeSet<String>,
    total_bytes: u64,
}

pub fn prepare_for_launch(
    path: &Path,
    preserve_direct_archive: bool,
    cancelled: &Arc<AtomicBool>,
) -> Result<PreparedLaunchMedia> {
    let project = ProjectDirs::from("com", "Lunchbox", "Lunchbox")
        .context("the operating system did not provide a Lunchbox cache directory")?;
    prepare_for_launch_in(
        path,
        preserve_direct_archive,
        cancelled,
        project.cache_dir(),
    )
}

fn prepare_for_launch_in(
    path: &Path,
    preserve_direct_archive: bool,
    cancelled: &Arc<AtomicBool>,
    cache_root: &Path,
) -> Result<PreparedLaunchMedia> {
    check_cancelled(cancelled)?;
    if !path.is_file() {
        bail!("local game file is missing: {}", path.display());
    }
    let path = fs::canonicalize(path)
        .with_context(|| format!("resolving local game file {}", path.display()))?;
    let extension = extension(&path);
    if extension.as_deref().is_some_and(|value| value == "m3u") {
        return prepare_playlist(&path, cancelled, cache_root);
    }
    if extension
        .as_deref()
        .is_some_and(|value| ARCHIVE_EXTENSIONS.contains(&value))
        && !preserve_direct_archive
    {
        return prepare_direct_archive(&path, cancelled, cache_root);
    }
    Ok(PreparedLaunchMedia {
        access_roots: path.parent().map(Path::to_path_buf).into_iter().collect(),
        path,
        cleanup_paths: Vec::new(),
        archive_count: 0,
        playlist_entries: 0,
    })
}

fn prepare_playlist(
    playlist_path: &Path,
    cancelled: &Arc<AtomicBool>,
    cache_root: &Path,
) -> Result<PreparedLaunchMedia> {
    let entries = read_playlist(playlist_path)?;
    let playlist_directory = playlist_path
        .parent()
        .context("the local playlist has no containing directory")?;
    let mut resolved = Vec::with_capacity(entries.len());
    let mut has_archive = false;
    for entry in entries {
        check_cancelled(cancelled)?;
        let entry_path = portable_playlist_path(playlist_directory, &entry)?;
        if !entry_path.is_file() {
            bail!(
                "playlist {} refers to missing file {}",
                playlist_path.display(),
                entry_path.display()
            );
        }
        let entry_path = fs::canonicalize(&entry_path)
            .with_context(|| format!("resolving playlist entry {}", entry_path.display()))?;
        has_archive |= extension(&entry_path)
            .as_deref()
            .is_some_and(|value| ARCHIVE_EXTENSIONS.contains(&value));
        resolved.push(entry_path);
    }

    if !has_archive {
        let access_roots = deduplicated_parents(
            std::iter::once(playlist_path.to_path_buf()).chain(resolved.iter().cloned()),
        );
        return Ok(PreparedLaunchMedia {
            path: playlist_path.to_path_buf(),
            cleanup_paths: Vec::new(),
            access_roots,
            archive_count: 0,
            playlist_entries: resolved.len(),
        });
    }

    let session = create_session(cache_root)?;
    let outcome = (|| -> Result<PreparedLaunchMedia> {
        let mut prepared_entries = Vec::with_capacity(resolved.len());
        let mut archive_count = 0usize;
        for entry in resolved {
            check_cancelled(cancelled)?;
            if extension(&entry)
                .as_deref()
                .is_some_and(|value| ARCHIVE_EXTENSIONS.contains(&value))
            {
                archive_count += 1;
                let output = session.join(format!("archive-{archive_count:03}"));
                prepared_entries.push(extract_archive(&entry, &output, cancelled)?);
            } else {
                prepared_entries.push(entry);
            }
        }
        let generated = session.join(prepared_playlist_filename(playlist_path));
        write_playlist(&generated, &prepared_entries)?;
        let access_roots = deduplicated_parents(
            std::iter::once(generated.clone()).chain(prepared_entries.iter().cloned()),
        );
        Ok(PreparedLaunchMedia {
            path: generated,
            cleanup_paths: vec![session.clone()],
            access_roots,
            archive_count,
            playlist_entries: prepared_entries.len(),
        })
    })();
    finish_session_outcome(outcome, &session, cancelled)
}

fn prepared_playlist_filename(playlist_path: &Path) -> String {
    // Library playlists already carry the game's name. Keep it when rewriting
    // their entries for extracted media, inside the unique launch session.
    let stem = playlist_path
        .file_stem()
        .unwrap_or_default()
        .to_string_lossy();
    let title = stem.trim_matches(|character: char| character.is_whitespace() || character == '.');
    let title = if title.is_empty() { "Game" } else { title };
    format!("{}.m3u", crate::qbittorrent::safe_path_component(title))
}

fn prepare_direct_archive(
    archive_path: &Path,
    cancelled: &Arc<AtomicBool>,
    cache_root: &Path,
) -> Result<PreparedLaunchMedia> {
    let session = create_session(cache_root)?;
    let outcome = (|| -> Result<PreparedLaunchMedia> {
        let output = session.join("archive-001");
        let prepared = extract_archive(archive_path, &output, cancelled)?;
        Ok(PreparedLaunchMedia {
            access_roots: prepared
                .parent()
                .map(Path::to_path_buf)
                .into_iter()
                .collect(),
            path: prepared,
            cleanup_paths: vec![session.clone()],
            archive_count: 1,
            playlist_entries: 0,
        })
    })();
    finish_session_outcome(outcome, &session, cancelled)
}

fn finish_session_outcome(
    outcome: Result<PreparedLaunchMedia>,
    session: &Path,
    cancelled: &Arc<AtomicBool>,
) -> Result<PreparedLaunchMedia> {
    match outcome {
        Ok(prepared) => Ok(prepared),
        Err(error) => {
            let _ = fs::remove_dir_all(session);
            if cancelled.load(Ordering::Relaxed) {
                bail!(LAUNCH_CANCELLED_ERROR);
            }
            Err(error)
        }
    }
}

fn extract_archive(
    archive_path: &Path,
    output: &Path,
    cancelled: &Arc<AtomicBool>,
) -> Result<PathBuf> {
    let archive_extension = extension(archive_path).with_context(|| {
        format!(
            "archive {} has no portable extension",
            archive_path.display()
        )
    })?;
    let plan = match archive_extension.as_str() {
        "zip" => inspect_zip(archive_path, cancelled)?,
        "7z" => inspect_seven_zip(archive_path, cancelled)?,
        "rar" => inspect_rar(archive_path, cancelled)?,
        _ => bail!("unsupported local archive format .{archive_extension}"),
    };
    let staging_parent = output
        .parent()
        .context("launch staging folder has no containing directory")?;
    let available = fs2::available_space(staging_parent).with_context(|| {
        format!(
            "checking free space for launch staging below {}",
            staging_parent.display()
        )
    })?;
    let usable = available.saturating_sub(MIN_FREE_SPACE_RESERVE);
    if plan.total_bytes > usable {
        bail!(
            "launch staging needs {} bytes but only {usable} bytes are available after the safety reserve",
            plan.total_bytes
        );
    }
    fs::create_dir_all(output)
        .with_context(|| format!("creating launch staging folder {}", output.display()))?;
    match archive_extension.as_str() {
        "zip" => extract_zip(archive_path, output, &plan, cancelled)?,
        "7z" => extract_seven_zip(archive_path, output, &plan, cancelled)?,
        "rar" => extract_rar(archive_path, output, &plan, cancelled)?,
        _ => unreachable!("archive extension was checked above"),
    }
    let primary = output.join(&plan.primary.relative_path);
    if !primary.is_file() {
        bail!(
            "archive launch entry {} was not extracted",
            plan.primary.relative_path.display()
        );
    }
    validate_control_file(&primary, output)?;
    Ok(primary)
}

fn inspect_zip(path: &Path, cancelled: &Arc<AtomicBool>) -> Result<ArchivePlan> {
    let file = File::open(path).with_context(|| format!("opening ZIP {}", path.display()))?;
    let mut archive = zip::ZipArchive::new(file)
        .with_context(|| format!("reading ZIP directory {}", path.display()))?;
    if archive.len() > MAX_ARCHIVE_MEMBERS {
        bail!("ZIP contains more than {MAX_ARCHIVE_MEMBERS} members");
    }
    let mut members = Vec::new();
    for index in 0..archive.len() {
        check_cancelled(cancelled)?;
        let entry = archive
            .by_index(index)
            .with_context(|| format!("reading ZIP member #{index} in {}", path.display()))?;
        if entry.is_dir() {
            continue;
        }
        if entry
            .unix_mode()
            .is_some_and(|mode| mode & 0o170000 == 0o120000)
        {
            bail!("ZIP contains a symbolic link, which cannot be staged safely");
        }
        if entry.encrypted() {
            bail!("ZIP contains encrypted members; password prompting is not supported");
        }
        members.push(archive_member(entry.name(), entry.size())?);
    }
    build_archive_plan(members)
}

fn inspect_seven_zip(path: &Path, cancelled: &Arc<AtomicBool>) -> Result<ArchivePlan> {
    let archive = sevenz_rust2::ArchiveReader::open(path, sevenz_rust2::Password::empty())
        .with_context(|| format!("reading 7z directory {}", path.display()))?;
    if archive.archive().files.len() > MAX_ARCHIVE_MEMBERS {
        bail!("7z archive contains more than {MAX_ARCHIVE_MEMBERS} members");
    }
    let mut members = Vec::new();
    for entry in &archive.archive().files {
        check_cancelled(cancelled)?;
        if entry.is_directory() || entry.is_anti_item || !entry.has_stream {
            continue;
        }
        members.push(archive_member(entry.name(), entry.size())?);
    }
    build_archive_plan(members)
}

fn inspect_rar(path: &Path, cancelled: &Arc<AtomicBool>) -> Result<ArchivePlan> {
    let options = ArchiveReadOptions::new().with_rar50_buffered_decode_limit(64 * 1024 * 1024);
    let archive = ArchiveReader::read_path_with_options(path, options)
        .with_context(|| format!("reading RAR directory {}", path.display()))?;
    let mut inspected = Vec::new();
    for (index, member) in archive.members().enumerate() {
        if index >= MAX_ARCHIVE_MEMBERS {
            bail!("RAR archive contains more than {MAX_ARCHIVE_MEMBERS} members");
        }
        check_cancelled(cancelled)?;
        if member.meta.is_encrypted {
            bail!("RAR contains encrypted members; password prompting is not supported");
        }
        if member.meta.is_split_before || member.meta.is_split_after {
            bail!("RAR is part of a multi-volume set, which cannot be staged safely");
        }
        if member.meta.is_directory {
            continue;
        }
        if rar_member_is_symlink(&member) {
            bail!("RAR contains a symbolic link, which cannot be staged safely");
        }
        let name = std::str::from_utf8(member.meta.name_bytes())
            .context("RAR member names must be valid UTF-8 for portable launch staging")?;
        inspected.push(archive_member(name, member.meta.unpacked_size)?);
    }
    build_archive_plan(inspected)
}

fn archive_member(name: &str, size: u64) -> Result<ArchiveMember> {
    let relative_path = safe_member_path(name)?;
    let extension = extension(&relative_path).unwrap_or_default();
    let key = portable_path_key(&relative_path);
    Ok(ArchiveMember {
        relative_path,
        key,
        extension,
        size,
    })
}

fn build_archive_plan(members: Vec<ArchiveMember>) -> Result<ArchivePlan> {
    if members.is_empty() {
        bail!("archive contains no regular files");
    }
    let mut names = BTreeSet::new();
    for member in &members {
        if !names.insert(member.key.clone()) {
            bail!(
                "archive contains duplicate or case-colliding path {}",
                member.relative_path.display()
            );
        }
    }
    let mut candidates = members
        .iter()
        .filter_map(|member| launch_priority(&member.extension).map(|rank| (rank, member)))
        .collect::<Vec<_>>();
    candidates.sort_by(|left, right| {
        left.0
            .cmp(&right.0)
            .then_with(|| left.1.key.cmp(&right.1.key))
    });
    let Some((best_rank, best)) = candidates.first().copied() else {
        bail!("archive contains no supported emulator launch file");
    };
    let equally_ranked = candidates
        .iter()
        .take_while(|(rank, _)| *rank == best_rank)
        .count();
    if equally_ranked != 1 {
        let examples = candidates
            .iter()
            .take(equally_ranked.min(4))
            .map(|(_, member)| member.relative_path.display().to_string())
            .collect::<Vec<_>>()
            .join(", ");
        bail!(
            "archive contains {equally_ranked} equally plausible launch files ({examples}); select an exact file instead"
        );
    }
    let primary = best.clone();
    let control = CONTROL_EXTENSIONS.contains(&primary.extension.as_str());
    let selected_keys = members
        .iter()
        .filter(|member| {
            member.key == primary.key || (control && is_launch_support_extension(&member.extension))
        })
        .map(|member| member.key.clone())
        .collect::<BTreeSet<_>>();
    let total_bytes = members
        .iter()
        .filter(|member| selected_keys.contains(&member.key))
        .try_fold(0u64, |total, member| total.checked_add(member.size))
        .context("archive member sizes overflowed")?;
    if total_bytes > MAX_EXTRACTED_BYTES {
        bail!(
            "archive launch payload is larger than the {} GiB staging safety limit",
            MAX_EXTRACTED_BYTES / 1024 / 1024 / 1024
        );
    }
    Ok(ArchivePlan {
        members,
        primary,
        selected_keys,
        total_bytes,
    })
}

fn extract_zip(
    path: &Path,
    output: &Path,
    plan: &ArchivePlan,
    cancelled: &Arc<AtomicBool>,
) -> Result<()> {
    let file = File::open(path).with_context(|| format!("opening ZIP {}", path.display()))?;
    let mut archive = zip::ZipArchive::new(file)
        .with_context(|| format!("reading ZIP directory {}", path.display()))?;
    let mut remaining = plan.selected_keys.clone();
    let mut copied = 0u64;
    for index in 0..archive.len() {
        check_cancelled(cancelled)?;
        let mut entry = archive
            .by_index(index)
            .with_context(|| format!("reading ZIP member #{index} in {}", path.display()))?;
        if entry.is_dir() {
            continue;
        }
        let relative = safe_member_path(entry.name())?;
        let key = portable_path_key(&relative);
        if !remaining.remove(&key) {
            continue;
        }
        let size = entry.size();
        copy_member(
            &mut entry,
            &output.join(relative),
            size,
            &mut copied,
            cancelled,
        )?;
    }
    ensure_all_extracted(remaining, "ZIP")?;
    ensure_declared_payload_size(copied, plan.total_bytes, "ZIP")
}

fn extract_seven_zip(
    path: &Path,
    output: &Path,
    plan: &ArchivePlan,
    cancelled: &Arc<AtomicBool>,
) -> Result<()> {
    let mut archive = sevenz_rust2::ArchiveReader::open(path, sevenz_rust2::Password::empty())
        .with_context(|| format!("reading 7z directory {}", path.display()))?;
    let mut remaining = plan.selected_keys.clone();
    let mut copied = 0u64;
    archive
        .for_each_entries(|entry, reader| {
            if cancelled.load(Ordering::Relaxed) {
                return Err(std::io::Error::other(LAUNCH_CANCELLED_ERROR).into());
            }
            if entry.is_directory() || entry.is_anti_item || !entry.has_stream {
                return Ok(true);
            }
            let relative = safe_member_path(entry.name())
                .map_err(|error| std::io::Error::other(error.to_string()))?;
            let key = portable_path_key(&relative);
            if remaining.remove(&key) {
                copy_member(
                    reader,
                    &output.join(relative),
                    entry.size(),
                    &mut copied,
                    cancelled,
                )
                .map_err(|error| std::io::Error::other(error.to_string()))?;
            } else {
                std::io::copy(reader, &mut std::io::sink())?;
            }
            Ok(true)
        })
        .with_context(|| format!("decoding 7z launch payload {}", path.display()))?;
    ensure_all_extracted(remaining, "7z")?;
    ensure_declared_payload_size(copied, plan.total_bytes, "7z")
}

struct RarLaunchWriter {
    file: File,
    expected_size: u64,
    written: u64,
    total_written: Arc<AtomicU64>,
    cancelled: Arc<AtomicBool>,
}

impl Write for RarLaunchWriter {
    fn write(&mut self, bytes: &[u8]) -> std::io::Result<usize> {
        if self.cancelled.load(Ordering::Relaxed) {
            return Err(std::io::Error::other(LAUNCH_CANCELLED_ERROR));
        }
        let next_member = self
            .written
            .checked_add(u64::try_from(bytes.len()).map_err(std::io::Error::other)?)
            .ok_or_else(|| std::io::Error::other("RAR member size overflowed"))?;
        if next_member > self.expected_size {
            return Err(std::io::Error::other(
                "RAR member exceeded its declared size",
            ));
        }
        let added = u64::try_from(bytes.len()).map_err(std::io::Error::other)?;
        self.total_written
            .fetch_update(Ordering::Relaxed, Ordering::Relaxed, |total| {
                total
                    .checked_add(added)
                    .filter(|next| *next <= MAX_EXTRACTED_BYTES)
            })
            .map_err(|_| std::io::Error::other("RAR payload exceeded the staging limit"))?;
        self.file.write_all(bytes)?;
        self.written = next_member;
        Ok(bytes.len())
    }

    fn flush(&mut self) -> std::io::Result<()> {
        self.file.flush()
    }
}

fn extract_rar(
    path: &Path,
    output: &Path,
    plan: &ArchivePlan,
    cancelled: &Arc<AtomicBool>,
) -> Result<()> {
    let options = ArchiveReadOptions::new().with_rar50_buffered_decode_limit(64 * 1024 * 1024);
    let archive = ArchiveReader::read_path_with_options(path, options)
        .with_context(|| format!("reading RAR directory {}", path.display()))?;
    let selected = plan
        .members
        .iter()
        .filter(|member| plan.selected_keys.contains(&member.key))
        .map(|member| (member.key.clone(), member.clone()))
        .collect::<BTreeMap<_, _>>();
    let written = Arc::new(std::sync::Mutex::new(BTreeSet::<String>::new()));
    let written_for_extract = Arc::clone(&written);
    let total_written = Arc::new(AtomicU64::new(0));
    let total_written_for_extract = Arc::clone(&total_written);
    archive
        .extract_to_with_options(options, |meta| {
            if cancelled.load(Ordering::Relaxed) {
                return Err(std::io::Error::other(LAUNCH_CANCELLED_ERROR).into());
            }
            if meta.is_directory {
                return Ok(Box::new(std::io::sink()) as Box<dyn Write>);
            }
            let raw = std::str::from_utf8(&meta.name)
                .map_err(|error| rars::Error::from(std::io::Error::other(error)))?;
            let relative = safe_member_path(raw)
                .map_err(|error| rars::Error::from(std::io::Error::other(error.to_string())))?;
            let key = portable_path_key(&relative);
            let Some(member) = selected.get(&key) else {
                return Ok(Box::new(std::io::sink()) as Box<dyn Write>);
            };
            let destination = output.join(&member.relative_path);
            if let Some(parent) = destination.parent() {
                fs::create_dir_all(parent).map_err(rars::Error::from)?;
            }
            let file = File::create(&destination).map_err(rars::Error::from)?;
            written_for_extract
                .lock()
                .map_err(|_| {
                    rars::Error::from(std::io::Error::other("RAR launch output set was poisoned"))
                })?
                .insert(key);
            Ok(Box::new(RarLaunchWriter {
                file,
                expected_size: member.size,
                written: 0,
                total_written: Arc::clone(&total_written_for_extract),
                cancelled: Arc::clone(cancelled),
            }) as Box<dyn Write>)
        })
        .with_context(|| format!("decoding RAR launch payload {}", path.display()))?;
    check_cancelled(cancelled)?;
    let written = written
        .lock()
        .map_err(|_| anyhow!("RAR launch output set was poisoned"))?;
    let remaining = plan
        .selected_keys
        .difference(&written)
        .cloned()
        .collect::<BTreeSet<_>>();
    ensure_all_extracted(remaining, "RAR")?;
    ensure_declared_payload_size(
        total_written.load(Ordering::Relaxed),
        plan.total_bytes,
        "RAR",
    )?;
    for member in selected.values() {
        let size = fs::metadata(output.join(&member.relative_path))?.len();
        if size != member.size {
            bail!(
                "RAR member {} decoded {size} bytes after declaring {}",
                member.relative_path.display(),
                member.size
            );
        }
    }
    Ok(())
}

fn copy_member(
    reader: &mut (impl Read + ?Sized),
    destination: &Path,
    expected_size: u64,
    copied: &mut u64,
    cancelled: &Arc<AtomicBool>,
) -> Result<()> {
    if let Some(parent) = destination.parent() {
        fs::create_dir_all(parent)
            .with_context(|| format!("creating launch staging folder {}", parent.display()))?;
    }
    let mut file = File::create(destination)
        .with_context(|| format!("creating launch file {}", destination.display()))?;
    let mut member_bytes = 0u64;
    let mut buffer = vec![0u8; 1024 * 1024];
    loop {
        check_cancelled(cancelled)?;
        let read_length = reader
            .read(&mut buffer)
            .with_context(|| format!("extracting launch file {}", destination.display()))?;
        if read_length == 0 {
            break;
        }
        let read = u64::try_from(read_length).context("archive read size overflowed")?;
        member_bytes = member_bytes
            .checked_add(read)
            .context("archive member size overflowed")?;
        *copied = copied
            .checked_add(read)
            .context("archive payload size overflowed")?;
        if member_bytes > expected_size || *copied > MAX_EXTRACTED_BYTES {
            bail!("archive exceeded its declared launch staging size");
        }
        file.write_all(&buffer[..read_length])?;
    }
    file.sync_all()?;
    if member_bytes != expected_size {
        bail!(
            "archive member {} decoded {member_bytes} bytes after declaring {expected_size}",
            destination.display()
        );
    }
    Ok(())
}

fn ensure_all_extracted(remaining: BTreeSet<String>, format: &str) -> Result<()> {
    if let Some(missing) = remaining.first() {
        bail!("{format} launch member {missing} disappeared during extraction");
    }
    Ok(())
}

fn ensure_declared_payload_size(copied: u64, expected: u64, format: &str) -> Result<()> {
    if copied != expected {
        bail!("{format} launch payload decoded {copied} bytes after declaring {expected}");
    }
    Ok(())
}

fn validate_control_file(path: &Path, extraction_root: &Path) -> Result<()> {
    validate_control_file_inner(path, extraction_root, &mut BTreeSet::new())
}

fn validate_control_file_inner(
    path: &Path,
    extraction_root: &Path,
    visited: &mut BTreeSet<String>,
) -> Result<()> {
    let relative = path.strip_prefix(extraction_root).with_context(|| {
        format!(
            "launch control file {} escaped its staging root",
            path.display()
        )
    })?;
    if !visited.insert(portable_path_key(relative)) {
        return Ok(());
    }
    let Some(control_extension) = extension(path) else {
        return Ok(());
    };
    let references = match control_extension.as_str() {
        "m3u" => read_playlist(path)?,
        "cue" => cue_references(path)?,
        "gdi" => gdi_references(path)?,
        "ccd" => same_stem_references(path, &["img"], &["sub"]),
        "mds" => same_stem_references(path, &["mdf"], &[]),
        _ => return Ok(()),
    };
    let base = path
        .parent()
        .context("control file has no containing directory")?;
    for reference in references {
        let referenced = contained_control_path(base, extraction_root, &reference)?;
        let referenced = ensure_control_reference_case(&referenced, extraction_root)?;
        if !referenced.is_file() {
            bail!(
                "launch control file {} refers to unavailable companion {}",
                path.display(),
                referenced.display()
            );
        }
        if extension(&referenced)
            .as_deref()
            .is_some_and(|value| CONTROL_EXTENSIONS.contains(&value))
        {
            validate_control_file_inner(&referenced, extraction_root, visited)?;
        }
    }
    Ok(())
}

fn contained_control_path(base: &Path, root: &Path, value: &str) -> Result<PathBuf> {
    if value.is_empty()
        || value.contains('\0')
        || value.contains(['\r', '\n'])
        || value.starts_with(['/', '\\'])
        || looks_like_windows_drive_path(value)
    {
        bail!("archive control file contains unsafe path {value:?}");
    }
    let base = base
        .strip_prefix(root)
        .context("archive control file escaped its staging root")?;
    let mut components = base
        .components()
        .map(|component| component.as_os_str().to_owned())
        .collect::<Vec<_>>();
    for component in value.replace('\\', "/").split('/') {
        match component {
            "" | "." => {}
            ".." => {
                if components.pop().is_none() {
                    bail!("archive control file path {value:?} escapes its staging root");
                }
            }
            value if value.contains(':') => {
                bail!("archive control file contains unsafe path {value:?}");
            }
            value => components.push(value.into()),
        }
    }
    if components.is_empty() {
        bail!("archive control file contains an empty path");
    }
    Ok(components
        .into_iter()
        .fold(root.to_path_buf(), |path, component| path.join(component)))
}

fn ensure_control_reference_case(requested: &Path, root: &Path) -> Result<PathBuf> {
    if requested.is_file() {
        return Ok(requested.to_path_buf());
    }
    let relative = requested
        .strip_prefix(root)
        .context("archive control reference escaped its staging root")?;
    let key = portable_path_key(relative);
    let mut matches = Vec::new();
    for entry in walkdir::WalkDir::new(root)
        .follow_links(false)
        .max_depth(32)
        .into_iter()
    {
        let entry =
            entry.with_context(|| format!("reviewing staged path below {}", root.display()))?;
        if !entry.file_type().is_file() {
            continue;
        }
        let candidate = entry
            .path()
            .strip_prefix(root)
            .context("staged file escaped its launch root")?;
        if portable_path_key(candidate) == key {
            matches.push(entry.into_path());
            if matches.len() > 1 {
                bail!(
                    "staged archive contains case-ambiguous companion {}",
                    relative.display()
                );
            }
        }
    }
    let Some(actual) = matches.pop() else {
        return Ok(requested.to_path_buf());
    };
    if let Some(parent) = requested.parent() {
        fs::create_dir_all(parent)
            .with_context(|| format!("creating control-file path {}", parent.display()))?;
    }
    fs::hard_link(&actual, requested).with_context(|| {
        format!(
            "creating case-compatible control-file companion {} from {}",
            requested.display(),
            actual.display()
        )
    })?;
    Ok(requested.to_path_buf())
}

fn cue_references(path: &Path) -> Result<Vec<String>> {
    let text = read_small_text(path, "CUE")?;
    let mut references = Vec::new();
    for line in text.lines() {
        let line = line.trim_start();
        if line.len() < 5 || !line[..4].eq_ignore_ascii_case("FILE") {
            continue;
        }
        let value = line[4..].trim_start();
        if let Some(reference) = first_quoted_or_token(value) {
            references.push(reference);
        }
    }
    Ok(references)
}

fn gdi_references(path: &Path) -> Result<Vec<String>> {
    let text = read_small_text(path, "GDI")?;
    let mut references = Vec::new();
    for (index, line) in text.lines().enumerate() {
        if index == 0 || line.trim().is_empty() {
            continue;
        }
        let fields = portable_fields(line)?;
        if fields.len() < 6 {
            bail!("GDI track line {} has fewer than six fields", index + 1);
        }
        references.push(fields[fields.len() - 2].clone());
    }
    Ok(references)
}

fn same_stem_references(path: &Path, required: &[&str], optional: &[&str]) -> Vec<String> {
    let stem = path
        .file_stem()
        .and_then(|value| value.to_str())
        .unwrap_or_default();
    let mut references = required
        .iter()
        .map(|extension| format!("{stem}.{extension}"))
        .collect::<Vec<_>>();
    references.extend(optional.iter().filter_map(|extension| {
        let candidate = path.with_extension(extension);
        candidate.is_file().then(|| format!("{stem}.{extension}"))
    }));
    references
}

fn portable_fields(line: &str) -> Result<Vec<String>> {
    let mut fields = Vec::new();
    let mut current = String::new();
    let mut quoted = false;
    for character in line.chars() {
        match character {
            '"' => quoted = !quoted,
            value if value.is_whitespace() && !quoted => {
                if !current.is_empty() {
                    fields.push(std::mem::take(&mut current));
                }
            }
            value => current.push(value),
        }
    }
    if quoted {
        bail!("control file contains an unterminated quoted path");
    }
    if !current.is_empty() {
        fields.push(current);
    }
    Ok(fields)
}

fn first_quoted_or_token(value: &str) -> Option<String> {
    if let Some(value) = value.strip_prefix('"') {
        return value.split_once('"').map(|(path, _)| path.to_owned());
    }
    value.split_whitespace().next().map(str::to_owned)
}

fn read_playlist(path: &Path) -> Result<Vec<String>> {
    let text = read_small_text(path, "M3U playlist")?;
    let mut entries = Vec::new();
    for (index, line) in text.lines().enumerate() {
        let line = if index == 0 {
            line.trim_start_matches('\u{feff}')
        } else {
            line
        };
        let line = line.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        entries.push(line.trim_matches('"').to_owned());
        if entries.len() > MAX_PLAYLIST_ENTRIES {
            bail!("playlist contains more than {MAX_PLAYLIST_ENTRIES} game entries");
        }
    }
    if entries.is_empty() {
        bail!("playlist {} contains no game entries", path.display());
    }
    Ok(entries)
}

fn read_small_text(path: &Path, kind: &str) -> Result<String> {
    let metadata = fs::metadata(path).with_context(|| format!("reading {kind} metadata"))?;
    if metadata.len() > MAX_PLAYLIST_BYTES {
        bail!("{kind} {} is larger than 1 MiB", path.display());
    }
    fs::read_to_string(path)
        .with_context(|| format!("reading portable UTF-8 {kind} {}", path.display()))
}

fn write_playlist(path: &Path, entries: &[PathBuf]) -> Result<()> {
    let mut text = String::from("#EXTM3U\n");
    for entry in entries {
        let entry = entry
            .to_str()
            .with_context(|| format!("playlist path {} is not valid UTF-8", entry.display()))?;
        if entry.contains(['\r', '\n']) {
            bail!("playlist path contains a line break");
        }
        text.push_str(entry);
        text.push('\n');
    }
    fs::write(path, text).with_context(|| format!("writing launch playlist {}", path.display()))
}

fn portable_playlist_path(base: &Path, value: &str) -> Result<PathBuf> {
    if value.is_empty() || value.contains('\0') || value.contains(['\r', '\n']) {
        bail!("playlist contains an empty or unsafe path");
    }
    if looks_like_windows_drive_path(value) && !cfg!(windows) {
        bail!(
            "playlist path {value:?} is Windows-specific; reimport it with portable relative paths"
        );
    }
    let normalized = if cfg!(windows) {
        Cow::Borrowed(value)
    } else {
        Cow::Owned(value.replace('\\', "/"))
    };
    let value = Path::new(normalized.as_ref());
    Ok(if value.is_absolute() {
        value.to_path_buf()
    } else {
        base.join(value)
    })
}

fn safe_member_path(raw: &str) -> Result<PathBuf> {
    let normalized = raw.replace('\\', "/");
    if normalized.is_empty()
        || normalized.starts_with('/')
        || normalized.contains('\0')
        || looks_like_windows_drive_path(&normalized)
    {
        bail!("archive contains unsafe member path {raw:?}");
    }
    let mut path = PathBuf::new();
    for component in normalized.split('/') {
        if component.is_empty() || component == "." || component == ".." || component.contains(':')
        {
            bail!("archive contains unsafe member path {raw:?}");
        }
        path.push(component);
    }
    Ok(path)
}

fn looks_like_windows_drive_path(value: &str) -> bool {
    let bytes = value.as_bytes();
    bytes.len() >= 3
        && bytes[0].is_ascii_alphabetic()
        && bytes[1] == b':'
        && matches!(bytes[2], b'/' | b'\\')
}

fn portable_path_key(path: &Path) -> String {
    path.to_string_lossy().replace('\\', "/").to_lowercase()
}

fn extension(path: &Path) -> Option<String> {
    path.extension()
        .and_then(|value| value.to_str())
        .map(str::to_ascii_lowercase)
}

fn launch_priority(extension: &str) -> Option<u8> {
    match extension {
        "m3u" => Some(0),
        value if CONTROL_EXTENSIONS.contains(&value) => Some(1),
        value if PRIMARY_IMAGE_EXTENSIONS.contains(&value) => Some(2),
        value if ROM_EXTENSIONS.contains(&value) => Some(3),
        value if AUXILIARY_IMAGE_EXTENSIONS.contains(&value) => Some(4),
        _ => None,
    }
}

fn is_launch_support_extension(extension: &str) -> bool {
    launch_priority(extension).is_some()
        || AUDIO_EXTENSIONS.contains(&extension)
        || matches!(extension, "toc" | "sbi")
}

fn create_session(cache_root: &Path) -> Result<PathBuf> {
    let parent = cache_root.join("launch-preparation");
    fs::create_dir_all(&parent)
        .with_context(|| format!("creating launch cache {}", parent.display()))?;
    for _ in 0..4 {
        let session = parent.join(format!("{SESSION_PREFIX}{}", Uuid::new_v4()));
        match fs::create_dir(&session) {
            Ok(()) => return Ok(session),
            Err(error) if error.kind() == std::io::ErrorKind::AlreadyExists => continue,
            Err(error) => {
                return Err(error)
                    .with_context(|| format!("creating launch session {}", session.display()));
            }
        }
    }
    bail!("could not allocate a unique launch preparation session")
}

fn deduplicated_parents(paths: impl IntoIterator<Item = PathBuf>) -> Vec<PathBuf> {
    let mut roots = BTreeMap::<String, PathBuf>::new();
    for path in paths {
        if let Some(parent) = path.parent() {
            let parent = fs::canonicalize(parent).unwrap_or_else(|_| parent.to_path_buf());
            roots.entry(portable_path_key(&parent)).or_insert(parent);
        }
    }
    roots.into_values().collect()
}

fn check_cancelled(cancelled: &Arc<AtomicBool>) -> Result<()> {
    if cancelled.load(Ordering::Relaxed) {
        bail!(LAUNCH_CANCELLED_ERROR);
    }
    Ok(())
}

fn rar_member_is_symlink(member: &rars::ArchiveMember) -> bool {
    let unix_attributes = match member.meta.family {
        ArchiveFamily::Rar15To40 => matches!(member.meta.host_os, Some(3) | Some(5)),
        ArchiveFamily::Rar50Plus => member.meta.host_os == Some(1),
        ArchiveFamily::Rar13 => false,
        _ => false,
    };
    unix_attributes && member.meta.file_attr & 0o170000 == 0o120000
}

pub fn cleanup_owned_path(path: &Path) -> bool {
    let Some(project) = ProjectDirs::from("com", "Lunchbox", "Lunchbox") else {
        return false;
    };
    cleanup_owned_path_in(path, project.cache_dir())
}

fn cleanup_owned_path_in(path: &Path, cache_root: &Path) -> bool {
    let parent = cache_root.join("launch-preparation");
    if path.parent() != Some(parent.as_path()) {
        return false;
    }
    let Some(name) = path.file_name().and_then(|value| value.to_str()) else {
        return false;
    };
    let Some(id) = name.strip_prefix(SESSION_PREFIX) else {
        return false;
    };
    Uuid::parse_str(id).is_ok()
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;

    fn zip_fixture(path: &Path, members: &[(&str, &[u8])]) {
        let file = File::create(path).unwrap();
        let mut archive = zip::ZipWriter::new(file);
        for (name, content) in members {
            archive
                .start_file(*name, zip::write::SimpleFileOptions::default())
                .unwrap();
            archive.write_all(content).unwrap();
        }
        archive.finish().unwrap();
    }

    fn seven_zip_fixture(path: &Path, members: &[(&str, &[u8])]) {
        let mut archive = sevenz_rust2::ArchiveWriter::create(path).unwrap();
        for (name, content) in members {
            archive
                .push_archive_entry(sevenz_rust2::ArchiveEntry::new_file(name), Some(*content))
                .unwrap();
        }
        archive.finish().unwrap();
    }

    fn rar_fixture(path: &Path, members: &[(&str, &[u8])]) {
        let mut archive = rars::Builder::new(rars::ArchiveVersion::Rar50);
        for (name, content) in members {
            archive
                .add_bytes(name.as_bytes().to_vec(), content.to_vec(), None, None)
                .unwrap();
        }
        archive.write_to_path(path, None).unwrap();
    }

    #[test]
    fn prepares_multi_disc_zip_playlist_and_cleans_only_owned_session() {
        let temp = tempfile::tempdir().unwrap();
        let disc_one = temp.path().join("Disc 1.zip");
        let disc_two = temp.path().join("Disc 2.zip");
        zip_fixture(
            &disc_one,
            &[
                ("Game (Disc 1).cue", b"FILE \"Game (Disc 1).bin\" BINARY\n"),
                ("Game (Disc 1).bin", b"one"),
            ],
        );
        zip_fixture(
            &disc_two,
            &[
                ("Game (Disc 2).cue", b"FILE \"Game (Disc 2).bin\" BINARY\n"),
                ("Game (Disc 2).bin", b"two"),
            ],
        );
        let playlist = temp.path().join("Game.m3u");
        fs::write(&playlist, "Disc 1.zip\nDisc 2.zip\n").unwrap();
        let cache = temp.path().join("cache");
        let cancelled = Arc::new(AtomicBool::new(false));

        let prepared = prepare_for_launch_in(&playlist, false, &cancelled, &cache).unwrap();
        assert_eq!(prepared.path.file_name().unwrap(), "Game.m3u");
        assert_ne!(prepared.path, playlist);
        assert_eq!(
            fs::read_to_string(&playlist).unwrap(),
            "Disc 1.zip\nDisc 2.zip\n"
        );
        assert_eq!(prepared.archive_count, 2);
        assert_eq!(prepared.playlist_entries, 2);
        let generated = fs::read_to_string(&prepared.path).unwrap();
        assert!(generated.contains("Game (Disc 1).cue"));
        assert!(generated.contains("Game (Disc 2).cue"));
        for line in generated.lines().filter(|line| !line.starts_with('#')) {
            assert!(Path::new(line).is_file());
        }
        let session = &prepared.cleanup_paths[0];
        assert!(cleanup_owned_path_in(session, &cache));
        assert!(!cleanup_owned_path_in(temp.path(), &cache));
        fs::remove_dir_all(session).unwrap();
        assert!(!session.exists());
    }

    #[test]
    fn prepared_playlist_keeps_game_title_in_a_portable_filename() {
        for (source, expected) in [
            ("Final Fantasy VII (USA).m3u", "Final Fantasy VII (USA).m3u"),
            (
                "ファイナルファンタジーVII.m3u",
                "ファイナルファンタジーVII.m3u",
            ),
            ("Game: Subtitle?.m3u", "Game_ Subtitle_.m3u"),
            ("CON.m3u", "_CON.m3u"),
            ("Game. .m3u", "Game.m3u"),
            ("...m3u", "Game.m3u"),
        ] {
            assert_eq!(prepared_playlist_filename(Path::new(source)), expected);
        }
    }

    #[test]
    fn prepares_direct_seven_zip_control_file_with_companion() {
        let temp = tempfile::tempdir().unwrap();
        let archive = temp.path().join("Game.7z");
        seven_zip_fixture(
            &archive,
            &[
                ("media/Game.cue", b"FILE \"Game.bin\" BINARY\n"),
                ("media/Game.bin", b"disc"),
            ],
        );
        let cancelled = Arc::new(AtomicBool::new(false));
        let prepared =
            prepare_for_launch_in(&archive, false, &cancelled, &temp.path().join("cache")).unwrap();
        assert_eq!(prepared.path.file_name().unwrap(), "Game.cue");
        assert!(prepared.path.with_file_name("Game.bin").is_file());
    }

    #[test]
    fn prepares_direct_rar_rom_without_external_tools() {
        let temp = tempfile::tempdir().unwrap();
        let archive = temp.path().join("Game.rar");
        rar_fixture(&archive, &[("Game.nes", b"rom")]);
        let cancelled = Arc::new(AtomicBool::new(false));
        let prepared =
            prepare_for_launch_in(&archive, false, &cancelled, &temp.path().join("cache")).unwrap();
        assert_eq!(prepared.path.file_name().unwrap(), "Game.nes");
        assert_eq!(fs::read(&prepared.path).unwrap(), b"rom");
    }

    #[test]
    fn control_files_resolve_contained_parent_paths_and_case_safely() {
        let temp = tempfile::tempdir().unwrap();
        let archive = temp.path().join("Game.zip");
        zip_fixture(
            &archive,
            &[
                ("media/Game.cue", b"FILE \"../tracks/Track.BIN\" BINARY\n"),
                ("tracks/track.bin", b"disc"),
            ],
        );
        let cancelled = Arc::new(AtomicBool::new(false));
        let prepared =
            prepare_for_launch_in(&archive, false, &cancelled, &temp.path().join("cache")).unwrap();
        let alias = prepared.path.parent().unwrap().join("../tracks/Track.BIN");
        assert_eq!(fs::read(alias).unwrap(), b"disc");
    }

    #[test]
    fn control_file_cannot_escape_the_owned_staging_root() {
        let temp = tempfile::tempdir().unwrap();
        let archive = temp.path().join("Game.zip");
        zip_fixture(
            &archive,
            &[
                ("media/Game.cue", b"FILE \"../../outside.bin\" BINARY\n"),
                ("outside.bin", b"disc"),
            ],
        );
        let cancelled = Arc::new(AtomicBool::new(false));
        let error = prepare_for_launch_in(&archive, false, &cancelled, &temp.path().join("cache"))
            .unwrap_err();
        assert!(error.to_string().contains("escapes its staging root"));
    }

    #[test]
    fn leaves_plain_playlist_and_arcade_archive_untouched() {
        let temp = tempfile::tempdir().unwrap();
        let disc = temp.path().join("disc.chd");
        fs::write(&disc, b"disc").unwrap();
        let playlist = temp.path().join("game.m3u");
        fs::write(&playlist, "disc.chd\n").unwrap();
        let archive = temp.path().join("romset.zip");
        fs::write(&archive, b"not parsed when preserved").unwrap();
        let cancelled = Arc::new(AtomicBool::new(false));
        let plain = prepare_for_launch_in(&playlist, false, &cancelled, &temp.path().join("cache"))
            .unwrap();
        assert_eq!(plain.path, fs::canonicalize(&playlist).unwrap());
        assert!(plain.cleanup_paths.is_empty());
        let preserved =
            prepare_for_launch_in(&archive, true, &cancelled, &temp.path().join("cache")).unwrap();
        assert_eq!(preserved.path, fs::canonicalize(&archive).unwrap());
        assert!(preserved.cleanup_paths.is_empty());
    }

    #[test]
    fn rejects_ambiguous_collections_and_unsafe_members() {
        let temp = tempfile::tempdir().unwrap();
        let ambiguous = temp.path().join("collection.zip");
        zip_fixture(&ambiguous, &[("one.nes", b"one"), ("two.nes", b"two")]);
        let unsafe_archive = temp.path().join("unsafe.zip");
        zip_fixture(&unsafe_archive, &[("../outside.nes", b"no")]);
        let cancelled = Arc::new(AtomicBool::new(false));
        let cache = temp.path().join("cache");
        assert!(
            prepare_for_launch_in(&ambiguous, false, &cancelled, &cache)
                .unwrap_err()
                .to_string()
                .contains("equally plausible")
        );
        assert!(
            prepare_for_launch_in(&unsafe_archive, false, &cancelled, &cache)
                .unwrap_err()
                .to_string()
                .contains("unsafe member")
        );
    }

    #[test]
    fn cancellation_is_exact_and_leaves_no_session() {
        let temp = tempfile::tempdir().unwrap();
        let archive = temp.path().join("game.zip");
        zip_fixture(&archive, &[("game.nes", b"rom")]);
        let cancelled = Arc::new(AtomicBool::new(true));
        let cache = temp.path().join("cache");
        let error = prepare_for_launch_in(&archive, false, &cancelled, &cache).unwrap_err();
        assert_eq!(error.to_string(), LAUNCH_CANCELLED_ERROR);
        assert!(!cache.join("launch-preparation").exists());
    }
}
