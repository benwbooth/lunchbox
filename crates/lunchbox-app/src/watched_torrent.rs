use std::collections::HashMap;
use std::fs;
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
    store: &SettingsStore,
) -> Result<WatchedTorrentScan> {
    scan_watched_torrents_cached(root, store, &[])
}

pub(crate) fn scan_watched_torrents_cached(
    root: &Path,
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
        let first = scan_watched_torrents(&root, &store).unwrap();
        assert_eq!(first.entries.len(), 1);
        assert_eq!(first.pending_count, 1);
        assert_eq!(first.registered_count, 0);
        assert_eq!(first.entries[0].status, "pending");

        let offer = external_torrent::inspect_torrent_file(&source).unwrap();
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

        let second = scan_watched_torrents_cached(&root, &store, &first.entries).unwrap();
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

        let scan = scan_watched_torrents(&root, &store).unwrap();
        assert_eq!(scan.invalid_count, 1);
        assert_eq!(scan.entries[0].status, "invalid");
        assert!(scan.entries[0].detail.contains("could not parse"));
    }
}
