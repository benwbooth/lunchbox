use std::collections::HashMap;
use std::fs::{self, OpenOptions};
use std::io::Write;
use std::path::{Path, PathBuf};
use std::time::UNIX_EPOCH;

use anyhow::{Context, Result, bail};

use crate::external_torrent;
use crate::settings::SettingsStore;

const MAX_WATCHED_TORRENTS: usize = 10_000;
const MAX_WATCH_DEPTH: usize = 24;

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct WatchedTorrentEntry {
    pub path: PathBuf,
    pub file_name: String,
    pub torrent_name: String,
    pub info_hash: String,
    pub file_count: usize,
    pub total_bytes: u64,
    pub modified_at: i64,
    source_size: u64,
    modified_nanos: u128,
    pub status: String,
    pub detail: String,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct WatchedTorrentScan {
    pub root: PathBuf,
    pub entries: Vec<WatchedTorrentEntry>,
    pub pending_count: usize,
    pub registered_count: usize,
    pub invalid_count: usize,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct ArchivedTorrent {
    pub source: PathBuf,
    pub destination: PathBuf,
    pub reused_existing: bool,
}

impl WatchedTorrentScan {
    pub(crate) fn message(&self) -> String {
        if self.root.as_os_str().is_empty() {
            return "Choose a watched torrent inbox. Lunchbox will detect .torrent files without queueing their payloads."
                .to_owned();
        }
        if self.entries.is_empty() {
            return format!("Watching {} · no .torrent files found", self.root.display());
        }
        format!(
            "Watching {} · {} ready for review · {} registered · {} need attention",
            self.root.display(),
            self.pending_count,
            self.registered_count,
            self.invalid_count
        )
    }
}

pub(crate) fn scan_watched_torrents(
    root: &Path,
    archive_root: &Path,
    store: &SettingsStore,
) -> Result<WatchedTorrentScan> {
    scan_watched_torrents_cached(root, archive_root, store, &[])
}

pub(crate) fn scan_watched_torrents_cached(
    root: &Path,
    archive_root: &Path,
    store: &SettingsStore,
    previous: &[WatchedTorrentEntry],
) -> Result<WatchedTorrentScan> {
    if root.as_os_str().is_empty() {
        return Ok(WatchedTorrentScan {
            root: PathBuf::new(),
            entries: Vec::new(),
            pending_count: 0,
            registered_count: 0,
            invalid_count: 0,
        });
    }

    let configured_root = root.to_path_buf();
    let root = root
        .canonicalize()
        .with_context(|| format!("opening watched torrent inbox {}", root.display()))?;
    if !root.is_dir() {
        bail!(
            "watched torrent inbox {} is not a directory",
            root.display()
        );
    }
    let archive_root = canonical_archive_for_scan(archive_root, &root)?;

    let mut paths = Vec::new();
    let mut pending_directories = vec![(root.clone(), 0_usize)];
    while let Some((directory, depth)) = pending_directories.pop() {
        let mut children = fs::read_dir(&directory)
            .with_context(|| format!("reading watched torrent folder {}", directory.display()))?
            .collect::<std::io::Result<Vec<_>>>()?;
        children.sort_by_key(fs::DirEntry::file_name);
        for child in children {
            let file_type = child.file_type().with_context(|| {
                format!("reading watched torrent entry {}", child.path().display())
            })?;
            if file_type.is_symlink() {
                continue;
            }
            if file_type.is_dir() {
                if archive_root.as_ref().is_some_and(|archive| {
                    child.path().canonicalize().ok().as_ref() == Some(archive)
                }) {
                    continue;
                }
                if depth < MAX_WATCH_DEPTH {
                    pending_directories.push((child.path(), depth + 1));
                }
                continue;
            }
            if !file_type.is_file() || !is_torrent_path(&child.path()) {
                continue;
            }
            if paths.len() >= MAX_WATCHED_TORRENTS {
                bail!(
                    "watched torrent inbox contains more than {MAX_WATCHED_TORRENTS} .torrent files; narrow the folder before scanning"
                );
            }
            paths.push(child.path());
        }
    }

    let previous_by_path = previous
        .iter()
        .map(|entry| (entry.path.clone(), entry))
        .collect::<HashMap<_, _>>();
    let mut entries = paths
        .into_iter()
        .map(|path| {
            let metadata = entry_metadata(&path);
            let cached = previous_by_path.get(&path).filter(|entry| {
                metadata.is_some_and(|(source_size, modified_nanos, _)| {
                    entry.source_size == source_size && entry.modified_nanos == modified_nanos
                })
            });
            if let Some(cached) = cached {
                let mut entry = (**cached).clone();
                if !entry.info_hash.is_empty() {
                    refresh_registration(&mut entry, store);
                }
                entry
            } else {
                inspect_entry(path, metadata, store)
            }
        })
        .collect::<Vec<_>>();
    entries.sort_by(|left, right| {
        status_rank(&left.status)
            .cmp(&status_rank(&right.status))
            .then_with(|| right.modified_at.cmp(&left.modified_at))
            .then_with(|| {
                left.file_name
                    .to_lowercase()
                    .cmp(&right.file_name.to_lowercase())
            })
            .then_with(|| left.path.cmp(&right.path))
    });

    Ok(WatchedTorrentScan {
        root: configured_root,
        pending_count: entries
            .iter()
            .filter(|entry| entry.status == "pending")
            .count(),
        registered_count: entries
            .iter()
            .filter(|entry| entry.status == "registered")
            .count(),
        invalid_count: entries
            .iter()
            .filter(|entry| entry.status == "invalid")
            .count(),
        entries,
    })
}

fn canonical_archive_for_scan(archive_root: &Path, watched_root: &Path) -> Result<Option<PathBuf>> {
    if archive_root.as_os_str().is_empty() {
        return Ok(None);
    }
    let archive_root = match archive_root.canonicalize() {
        Ok(path) => path,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(None),
        Err(error) => {
            return Err(error).with_context(|| {
                format!("opening watched torrent archive {}", archive_root.display())
            });
        }
    };
    if !archive_root.is_dir() {
        bail!(
            "watched torrent archive {} is not a directory",
            archive_root.display()
        );
    }
    if archive_root == watched_root {
        bail!("watched torrent inbox and archive must be different folders");
    }
    Ok(Some(archive_root))
}

pub(crate) fn archive_registered_torrent(
    source: &Path,
    watched_root: &Path,
    archive_root: &Path,
    store: &SettingsStore,
) -> Result<Option<ArchivedTorrent>> {
    if archive_root.as_os_str().is_empty() {
        return Ok(None);
    }
    if watched_root.as_os_str().is_empty() {
        bail!("choose a watched torrent inbox before enabling its archive");
    }
    let source_metadata = fs::symlink_metadata(source)
        .with_context(|| format!("opening watched torrent {}", source.display()))?;
    if source_metadata.file_type().is_symlink() || !source_metadata.is_file() {
        bail!("only a regular watched .torrent file can be archived");
    }
    if !is_torrent_path(source) {
        bail!("only watched .torrent metadata can be archived");
    }
    let watched_root = watched_root
        .canonicalize()
        .with_context(|| format!("opening watched torrent inbox {}", watched_root.display()))?;
    let source = source
        .canonicalize()
        .with_context(|| format!("opening watched torrent {}", source.display()))?;
    if !source.starts_with(&watched_root) {
        bail!("the reviewed torrent is outside the configured watched inbox");
    }

    fs::create_dir_all(archive_root).with_context(|| {
        format!(
            "creating watched torrent archive {}",
            archive_root.display()
        )
    })?;
    let archive_root = archive_root
        .canonicalize()
        .with_context(|| format!("opening watched torrent archive {}", archive_root.display()))?;
    if archive_root == watched_root {
        bail!("watched torrent inbox and archive must be different folders");
    }
    if source.starts_with(&archive_root) {
        bail!("the reviewed torrent is already inside the configured archive");
    }

    let offer = external_torrent::inspect_torrent_file(&source)?;
    if store.registered_torrent_source(&offer.info_hash)?.is_none() {
        bail!("the torrent is not registered; review and add its exact platform first");
    }
    let file_name = source
        .file_name()
        .context("watched torrent path has no filename")?;
    let preferred = archive_root.join(file_name);
    let destination = choose_archive_destination(&preferred, &archive_root, &offer)?;
    let reused_existing = destination.exists();
    if reused_existing {
        verify_archived_copy(&destination, &offer)?;
    } else {
        write_archived_copy(&destination, &offer)?;
    }

    let current = external_torrent::inspect_torrent_file(&source)
        .context("rechecking watched torrent before removing its inbox copy")?;
    if current.info_hash != offer.info_hash || current.torrent_sha256 != offer.torrent_sha256 {
        bail!("the watched torrent changed during archival; its inbox copy was preserved");
    }
    fs::remove_file(&source)
        .with_context(|| format!("removing archived inbox copy {}", source.display()))?;
    Ok(Some(ArchivedTorrent {
        source,
        destination,
        reused_existing,
    }))
}

fn choose_archive_destination(
    preferred: &Path,
    archive_root: &Path,
    offer: &external_torrent::ManualTorrentOffer,
) -> Result<PathBuf> {
    if !preferred.exists() {
        return Ok(preferred.to_path_buf());
    }
    if verify_archived_copy(preferred, offer).is_ok() {
        return Ok(preferred.to_path_buf());
    }
    let stem = preferred
        .file_stem()
        .and_then(|value| value.to_str())
        .map(portable_archive_stem)
        .filter(|value| !value.is_empty())
        .unwrap_or_else(|| "torrent".to_owned());
    let alternate = archive_root.join(format!("{stem}-{}.torrent", offer.torrent_sha256));
    if alternate.exists() {
        verify_archived_copy(&alternate, offer)?;
    }
    Ok(alternate)
}

fn portable_archive_stem(value: &str) -> String {
    value
        .chars()
        .map(|character| {
            if character.is_ascii_alphanumeric() || matches!(character, '-' | '_' | '.') {
                character
            } else {
                '_'
            }
        })
        .take(180)
        .collect()
}

fn write_archived_copy(
    destination: &Path,
    offer: &external_torrent::ManualTorrentOffer,
) -> Result<()> {
    let mut output = OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(destination)
        .with_context(|| format!("creating archived torrent {}", destination.display()))?;
    let result = output
        .write_all(&offer.torrent_bytes)
        .with_context(|| format!("writing archived torrent {}", destination.display()))
        .and_then(|()| {
            output
                .sync_all()
                .with_context(|| format!("syncing archived torrent {}", destination.display()))
        });
    drop(output);
    if result.is_err() {
        let _ = fs::remove_file(destination);
        return result;
    }
    let result = verify_archived_copy(destination, offer);
    if result.is_err() {
        let _ = fs::remove_file(destination);
    }
    result
}

fn verify_archived_copy(
    destination: &Path,
    offer: &external_torrent::ManualTorrentOffer,
) -> Result<()> {
    let metadata = fs::symlink_metadata(destination)
        .with_context(|| format!("opening archived torrent {}", destination.display()))?;
    if metadata.file_type().is_symlink() || !metadata.is_file() {
        bail!("the archive destination is not a regular file");
    }
    let archived = external_torrent::inspect_torrent_file(destination)?;
    if archived.info_hash != offer.info_hash || archived.torrent_sha256 != offer.torrent_sha256 {
        bail!("the archive destination contains different torrent metadata");
    }
    Ok(())
}

fn inspect_entry(
    path: PathBuf,
    metadata: Option<(u64, u128, i64)>,
    store: &SettingsStore,
) -> WatchedTorrentEntry {
    let file_name = path
        .file_name()
        .map(|name| name.to_string_lossy().into_owned())
        .unwrap_or_else(|| path.to_string_lossy().into_owned());
    let (source_size, modified_nanos, modified_at) = metadata.unwrap_or_default();

    match external_torrent::inspect_torrent_file(&path) {
        Ok(offer) => {
            let mut entry = WatchedTorrentEntry {
                path,
                file_name,
                torrent_name: offer.torrent_name,
                info_hash: offer.info_hash,
                file_count: offer.files.len(),
                total_bytes: offer.total_bytes,
                modified_at,
                source_size,
                modified_nanos,
                status: String::new(),
                detail: String::new(),
            };
            refresh_registration(&mut entry, store);
            entry
        }
        Err(error) => invalid_entry(
            path,
            file_name,
            source_size,
            modified_nanos,
            modified_at,
            error.to_string(),
        ),
    }
}

fn refresh_registration(entry: &mut WatchedTorrentEntry, store: &SettingsStore) {
    match store.registered_torrent_source(&entry.info_hash) {
        Ok(Some(registered)) => {
            entry.status = "registered".to_owned();
            entry.detail = format!(
                "Registered for {} · {} files · {}",
                registered.platform,
                entry.file_count,
                crate::game_details::format_bytes(entry.total_bytes)
            );
        }
        Ok(None) => {
            entry.status = "pending".to_owned();
            entry.detail = format!(
                "Ready for platform review · {} files · {}",
                entry.file_count,
                crate::game_details::format_bytes(entry.total_bytes)
            );
        }
        Err(error) => {
            entry.status = "invalid".to_owned();
            entry.detail = format!("Could not check registration: {error}");
        }
    }
}

fn entry_metadata(path: &Path) -> Option<(u64, u128, i64)> {
    let metadata = fs::metadata(path).ok()?;
    let modified = metadata.modified().ok()?;
    let duration = modified.duration_since(UNIX_EPOCH).ok()?;
    Some((
        metadata.len(),
        duration.as_nanos(),
        i64::try_from(duration.as_secs()).unwrap_or(i64::MAX),
    ))
}

fn invalid_entry(
    path: PathBuf,
    file_name: String,
    source_size: u64,
    modified_nanos: u128,
    modified_at: i64,
    detail: String,
) -> WatchedTorrentEntry {
    WatchedTorrentEntry {
        path,
        file_name,
        torrent_name: String::new(),
        info_hash: String::new(),
        file_count: 0,
        total_bytes: 0,
        modified_at,
        source_size,
        modified_nanos,
        status: "invalid".to_owned(),
        detail,
    }
}

fn is_torrent_path(path: &Path) -> bool {
    path.extension()
        .and_then(|extension| extension.to_str())
        .is_some_and(|extension| extension.eq_ignore_ascii_case("torrent"))
}

fn status_rank(status: &str) -> u8 {
    match status {
        "pending" => 0,
        "invalid" => 1,
        "registered" => 2,
        _ => 3,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::settings::{RegisteredTorrentCatalog, RegisteredTorrentMember, unix_timestamp};

    fn write_probe(path: &Path) {
        fs::write(path, external_torrent::probe_fixture_torrent_bytes()).unwrap();
    }

    fn register_probe(
        store: &SettingsStore,
        source: &Path,
    ) -> external_torrent::ManualTorrentOffer {
        let offer = external_torrent::inspect_torrent_file(source).unwrap();
        store
            .register_torrent_catalog(&RegisteredTorrentCatalog {
                platform: "Nintendo Switch".into(),
                platform_key: crate::catalog::normalize_platform_key("Nintendo Switch"),
                source_kind: "file".into(),
                source_label: offer.source_label.clone(),
                source_file_name: offer.source_file_name.clone(),
                torrent_name: offer.torrent_name.clone(),
                torrent_sha256: offer.torrent_sha256.clone(),
                info_hash: offer.info_hash.clone(),
                torrent_bytes: offer.torrent_bytes.clone(),
                total_bytes: offer.total_bytes,
                members: offer
                    .files
                    .iter()
                    .map(|file| RegisteredTorrentMember {
                        index: file.index,
                        path: file.path.clone(),
                        byte_size: file.byte_size,
                        title_key: crate::game_details::torrent_member_title_key(&file.path),
                    })
                    .collect(),
                registered_at: unix_timestamp(),
            })
            .unwrap();
        offer
    }

    #[test]
    fn watched_scan_is_recursive_skips_symlinks_and_marks_registered_identity() {
        let directory = tempfile::tempdir().unwrap();
        let root = directory.path().join("inbox");
        let nested = root.join("Switch");
        fs::create_dir_all(&nested).unwrap();
        let source = nested.join("sample.TORRENT");
        write_probe(&source);
        fs::write(root.join("ignore.txt"), b"not torrent metadata").unwrap();

        #[cfg(unix)]
        std::os::unix::fs::symlink(&source, root.join("duplicate.torrent")).unwrap();

        let store = SettingsStore::at(directory.path().join("state.db")).unwrap();
        let first = scan_watched_torrents(&root, Path::new(""), &store).unwrap();
        assert_eq!(first.entries.len(), 1);
        assert_eq!(first.pending_count, 1);
        assert_eq!(first.registered_count, 0);
        assert_eq!(first.entries[0].status, "pending");

        register_probe(&store, &source);

        let second =
            scan_watched_torrents_cached(&root, Path::new(""), &store, &first.entries).unwrap();
        assert_eq!(second.pending_count, 0);
        assert_eq!(second.registered_count, 1);
        assert_eq!(second.entries[0].status, "registered");
        assert!(second.entries[0].detail.contains("Nintendo Switch"));
    }

    #[test]
    fn watched_scan_keeps_invalid_metadata_visible_and_actionable() {
        let directory = tempfile::tempdir().unwrap();
        let root = directory.path().join("inbox");
        fs::create_dir_all(&root).unwrap();
        fs::write(root.join("broken.torrent"), b"not bencode").unwrap();
        let store = SettingsStore::at(directory.path().join("state.db")).unwrap();

        let scan = scan_watched_torrents(&root, Path::new(""), &store).unwrap();
        assert_eq!(scan.invalid_count, 1);
        assert_eq!(scan.entries[0].status, "invalid");
        assert!(scan.entries[0].detail.contains("could not parse"));
    }

    #[test]
    fn registered_metadata_archives_idempotently_and_nested_archive_is_not_scanned() {
        let directory = tempfile::tempdir().unwrap();
        let root = directory.path().join("inbox");
        let archive = root.join("added");
        fs::create_dir_all(&archive).unwrap();
        let source = root.join("switch collection.torrent");
        let colliding_destination = archive.join("switch collection.torrent");
        write_probe(&source);
        fs::write(&colliding_destination, b"unrelated torrent metadata").unwrap();
        let store = SettingsStore::at(directory.path().join("state.db")).unwrap();

        let error = archive_registered_torrent(&source, &root, &archive, &store).unwrap_err();
        assert!(error.to_string().contains("not registered"));
        assert!(source.is_file());

        let offer = register_probe(&store, &source);
        let first = archive_registered_torrent(&source, &root, &archive, &store)
            .unwrap()
            .unwrap();
        assert!(!first.reused_existing);
        assert_ne!(first.destination, colliding_destination);
        assert!(
            first
                .destination
                .file_name()
                .unwrap()
                .to_string_lossy()
                .contains(&offer.torrent_sha256)
        );
        assert_eq!(
            fs::read(&colliding_destination).unwrap(),
            b"unrelated torrent metadata"
        );
        assert!(!source.exists());
        let archived = external_torrent::inspect_torrent_file(&first.destination).unwrap();
        assert_eq!(archived.info_hash, offer.info_hash);
        assert_eq!(archived.torrent_sha256, offer.torrent_sha256);

        write_probe(&source);
        let repeated = archive_registered_torrent(&source, &root, &archive, &store)
            .unwrap()
            .unwrap();
        assert!(repeated.reused_existing);
        assert_eq!(repeated.destination, first.destination);
        assert!(!source.exists());

        let scan = scan_watched_torrents(&root, &archive, &store).unwrap();
        assert!(scan.entries.is_empty());
    }
}
