use std::fs::{self, OpenOptions};
use std::io::Write;
use std::path::{Path, PathBuf};

use anyhow::{Context, Result, bail};
use lava_torrent::torrent::v1::Torrent;
use rusqlite::OptionalExtension;

use crate::qbittorrent::{self, DownloadSelection, QbittorrentClient};
use crate::settings::{self, MediaTransfer, SettingsStore};

const MANUAL_MEDIA_KIND: &str = "manual";
const MINERVA_PROVIDER: &str = "minerva";
const TORRENT_TRANSPORT: &str = "torrent";
const MANUAL_COLLECTION: &str = "No-Intro";
const MANUAL_PLATFORM: &str = "Unofficial - Video Game Manual Scans (JPEG)";
const MINERVA_MANUAL_MATCH_THRESHOLD: f64 = 0.90;

#[derive(Clone, Debug, PartialEq)]
struct ManualCandidate {
    torrent_url: String,
    torrent_bytes: Vec<u8>,
    info_hash: String,
    file_index: u32,
    file_path: String,
    file_size: u64,
    match_score: f64,
}

#[derive(Clone, Debug, PartialEq)]
pub struct MediaRefresh {
    pub transfer: MediaTransfer,
    pub published: bool,
}

pub fn load_manual_transfer(game_uid: &str) -> Result<Option<MediaTransfer>> {
    SettingsStore::open_default()?.media_transfer(game_uid, MANUAL_MEDIA_KIND)
}

pub fn manual_candidate_probe(game_title: &str) -> Result<String> {
    let candidate = find_minerva_manual_candidate(game_title)?.with_context(|| {
        format!("No exact manual scan for {game_title} is present in Minerva's current archive")
    })?;
    Ok(format!(
        "info_hash={} file_index={} bytes={} score={:.3} path={}",
        candidate.info_hash,
        candidate.file_index,
        candidate.file_size,
        candidate.match_score,
        candidate.file_path
    ))
}

pub fn enqueue_manual(
    game_uid: &str,
    launchbox_db_id: i64,
    title: &str,
    platform: &str,
) -> Result<MediaTransfer> {
    validate_game(game_uid, launchbox_db_id, title, platform)?;
    if crate::media::supplemental_media(game_uid, launchbox_db_id)?
        .manual
        .is_some()
    {
        bail!("A cached manual is already ready for this game");
    }

    let store = SettingsStore::open_default()?;
    if let Some(existing) = store.media_transfer(game_uid, MANUAL_MEDIA_KIND)?
        && transfer_needs_refresh(&existing.state)
    {
        return Ok(existing);
    }

    let candidate = find_minerva_manual_candidate(title)?.with_context(|| {
        format!("No exact manual scan for {title} is present in Minerva's current manual archive")
    })?;
    let app_settings = store.load()?;
    let password = settings::load_password()?.unwrap_or_default();
    let (native_root, client_root) = media_download_roots(&app_settings)?;
    fs::create_dir_all(&native_root).with_context(|| {
        format!(
            "creating native supplemental-media download directory {}",
            native_root.display()
        )
    })?;

    let existing_selection = active_selection(
        store.media_transfers_for_remote_job(MINERVA_PROVIDER, &candidate.info_hash)?,
    );
    let mut selected = existing_selection.clone();
    selected.push((candidate.file_index, candidate.file_path.clone()));
    selected.sort_by(|left, right| left.0.cmp(&right.0).then_with(|| left.1.cmp(&right.1)));
    selected.dedup();

    let client = QbittorrentClient::authenticated(&app_settings, &password)?;
    let actual_files = client.add(
        &candidate.torrent_bytes,
        &candidate.info_hash,
        &client_root,
        &DownloadSelection::Exact(selected),
        &[(candidate.file_index, candidate.file_path.clone())],
    )?;
    let actual_path = actual_files
        .into_iter()
        .find(|(index, _)| *index == candidate.file_index)
        .map(|(_, path)| path)
        .context("qBittorrent did not return the exact reviewed manual file")?;
    let source_path = native_root.join(qbittorrent::safe_torrent_relative_path(&actual_path)?);
    let target_path = crate::media::game_media_directory(game_uid, launchbox_db_id)?
        .join(MINERVA_PROVIDER)
        .join("manual.zip");
    let now = settings::unix_timestamp();
    let transfer = MediaTransfer {
        game_uid: game_uid.to_owned(),
        launchbox_db_id,
        title: title.to_owned(),
        platform: platform.to_owned(),
        media_kind: MANUAL_MEDIA_KIND.to_owned(),
        provider: MINERVA_PROVIDER.to_owned(),
        transport: TORRENT_TRANSPORT.to_owned(),
        source_url: candidate.torrent_url,
        remote_job_id: candidate.info_hash,
        source_item_index: candidate.file_index,
        source_item_path: actual_path,
        local_download_path: source_path,
        local_target_path: target_path,
        state: "queued".to_owned(),
        progress: 0.0,
        download_speed: 0,
        downloaded_bytes: 0,
        total_bytes: candidate.file_size,
        message: format!(
            "Queued an exact Minerva manual match ({:.0}% confidence)",
            candidate.match_score * 100.0
        ),
        created_at: now,
        updated_at: now,
    };
    if let Err(error) = store.upsert_media_transfer(&transfer) {
        if existing_selection.is_empty() {
            let _ = client.cancel_owned(&transfer.remote_job_id);
        } else {
            let _ = client.reconfigure(
                &transfer.remote_job_id,
                &DownloadSelection::Exact(existing_selection),
                true,
            );
        }
        return Err(error).context("recording the manual transfer after qBittorrent accepted it");
    }
    Ok(transfer)
}

pub fn refresh_manual(game_uid: &str) -> Result<MediaRefresh> {
    let store = SettingsStore::open_default()?;
    let mut transfer = store
        .media_transfer(game_uid, MANUAL_MEDIA_KIND)?
        .context("No manual transfer is active for this game")?;
    if transfer.state == "published" {
        return Ok(MediaRefresh {
            transfer,
            published: true,
        });
    }
    if !transfer_needs_refresh(&transfer.state) {
        return Ok(MediaRefresh {
            transfer,
            published: false,
        });
    }
    ensure_minerva_torrent_transfer(&transfer)?;

    let app_settings = store.load()?;
    let password = settings::load_password()?.unwrap_or_default();
    rehydrate_local_paths(&app_settings, &mut transfer)?;
    if transfer.state == "complete" {
        let published = match publish_completed_manual(&app_settings, &transfer) {
            Ok(()) => {
                transfer.state = "published".to_owned();
                transfer.progress = 1.0;
                transfer.message = "Manual downloaded and added to Game Media".to_owned();
                if app_settings.seeding_policy == "pause_after_import"
                    && let Err(error) = QbittorrentClient::authenticated(&app_settings, &password)
                        .and_then(|client| client.pause_owned(&transfer.remote_job_id))
                {
                    transfer.message = format!(
                        "Manual is ready; the torrent could not be paused and remains managed by qBittorrent: {error}"
                    );
                }
                true
            }
            Err(error) => {
                transfer.message = format!(
                    "Download complete; publishing the manual will retry automatically: {error}"
                );
                false
            }
        };
        transfer.updated_at = settings::unix_timestamp();
        store.upsert_media_transfer(&transfer)?;
        return Ok(MediaRefresh {
            transfer,
            published,
        });
    }
    let client = QbittorrentClient::authenticated(&app_settings, &password)?;
    let selection = [(
        transfer.source_item_index,
        transfer.source_item_path.clone(),
    )];
    let snapshot = client.snapshot(&transfer.remote_job_id, Some(&selection))?;
    transfer.state = snapshot.state;
    transfer.progress = snapshot.progress;
    transfer.download_speed = snapshot.download_speed;
    transfer.downloaded_bytes = snapshot.downloaded_bytes;
    transfer.total_bytes = snapshot.total_bytes;
    transfer.message = snapshot.message;
    transfer.updated_at = settings::unix_timestamp();

    let mut published = false;
    if transfer.state == "complete" {
        match publish_completed_manual(&app_settings, &transfer) {
            Ok(()) => {
                transfer.state = "published".to_owned();
                transfer.progress = 1.0;
                transfer.message = "Manual downloaded and added to Game Media".to_owned();
                published = true;
                if app_settings.seeding_policy == "pause_after_import"
                    && let Err(error) = client.pause_owned(&transfer.remote_job_id)
                {
                    transfer.message = format!(
                        "Manual is ready; the torrent could not be paused and remains managed by qBittorrent: {error}"
                    );
                }
            }
            Err(error) => {
                transfer.message = format!(
                    "Download complete; publishing the manual will retry automatically: {error}"
                );
            }
        }
    }
    transfer.updated_at = settings::unix_timestamp();
    store.upsert_media_transfer(&transfer)?;
    Ok(MediaRefresh {
        transfer,
        published,
    })
}

pub fn cancel_manual(game_uid: &str) -> Result<MediaTransfer> {
    let store = SettingsStore::open_default()?;
    let mut transfer = store
        .media_transfer(game_uid, MANUAL_MEDIA_KIND)?
        .context("No manual transfer is available to cancel")?;
    ensure_minerva_torrent_transfer(&transfer)?;
    if !transfer_needs_refresh(&transfer.state) {
        return Ok(transfer);
    }

    let app_settings = store.load()?;
    let password = settings::load_password()?.unwrap_or_default();
    let client = QbittorrentClient::authenticated(&app_settings, &password)?;
    let remaining = store
        .media_transfers_for_remote_job(MINERVA_PROVIDER, &transfer.remote_job_id)?
        .into_iter()
        .filter(|candidate| {
            candidate.game_uid != transfer.game_uid
                && transfer_needs_refresh(&candidate.state)
                && candidate.state != "complete"
        })
        .collect::<Vec<_>>();
    if remaining.is_empty() {
        client.cancel_owned(&transfer.remote_job_id)?;
        transfer.message = "Cancelled; qBittorrent files were preserved".to_owned();
    } else {
        let selection = active_selection(remaining);
        client.reconfigure(
            &transfer.remote_job_id,
            &DownloadSelection::Exact(selection),
            true,
        )?;
        transfer.message = "Cancelled this manual; other media downloads continue".to_owned();
    }
    transfer.state = "cancelled".to_owned();
    transfer.progress = 0.0;
    transfer.download_speed = 0;
    transfer.updated_at = settings::unix_timestamp();
    store.upsert_media_transfer(&transfer)?;
    Ok(transfer)
}

pub fn transfer_needs_refresh(state: &str) -> bool {
    matches!(state, "queued" | "downloading" | "paused" | "complete")
}

fn validate_game(game_uid: &str, launchbox_db_id: i64, title: &str, platform: &str) -> Result<()> {
    if game_uid.trim().is_empty() || game_uid.chars().count() > 512 {
        bail!("A bounded stable game identity is required to download media");
    }
    if launchbox_db_id < 0 {
        bail!("LaunchBox database ID cannot be negative");
    }
    if title.trim().is_empty() || title.chars().count() > 2_048 {
        bail!("A bounded game title is required to find a manual");
    }
    if platform.trim().is_empty() || platform.chars().count() > 1_024 {
        bail!("A bounded platform is required to find a manual");
    }
    Ok(())
}

fn media_download_roots(app_settings: &settings::AppSettings) -> Result<(PathBuf, String)> {
    if app_settings
        .torrent_library_directory
        .as_os_str()
        .is_empty()
    {
        bail!("Choose a native torrent library directory in Settings before downloading media");
    }
    if app_settings
        .qbittorrent_container_torrent_library_directory
        .trim()
        .is_empty()
    {
        bail!("Enter qBittorrent's torrent-library path in Settings before downloading media");
    }
    Ok((
        app_settings
            .torrent_library_directory
            .join("lunchbox-media"),
        qbittorrent::client_path_join(
            &app_settings.qbittorrent_container_torrent_library_directory,
            "lunchbox-media",
        ),
    ))
}

fn rehydrate_local_paths(
    app_settings: &settings::AppSettings,
    transfer: &mut MediaTransfer,
) -> Result<()> {
    let (native_root, _) = media_download_roots(app_settings)?;
    transfer.local_download_path = native_root.join(qbittorrent::safe_torrent_relative_path(
        &transfer.source_item_path,
    )?);
    transfer.local_target_path =
        crate::media::game_media_directory(&transfer.game_uid, transfer.launchbox_db_id)?
            .join(MINERVA_PROVIDER)
            .join("manual.zip");
    Ok(())
}

fn active_selection(transfers: Vec<MediaTransfer>) -> Vec<(u32, String)> {
    let mut selected = transfers
        .into_iter()
        .filter(|transfer| {
            transfer.transport == TORRENT_TRANSPORT
                && transfer_needs_refresh(&transfer.state)
                && transfer.state != "complete"
        })
        .map(|transfer| (transfer.source_item_index, transfer.source_item_path))
        .collect::<Vec<_>>();
    selected.sort_by(|left, right| left.0.cmp(&right.0).then_with(|| left.1.cmp(&right.1)));
    selected.dedup();
    selected
}

fn ensure_minerva_torrent_transfer(transfer: &MediaTransfer) -> Result<()> {
    if transfer.media_kind != MANUAL_MEDIA_KIND
        || transfer.provider != MINERVA_PROVIDER
        || transfer.transport != TORRENT_TRANSPORT
    {
        bail!("The persisted media transfer is not a supported Minerva manual job");
    }
    Ok(())
}

fn find_minerva_manual_candidate(game_title: &str) -> Result<Option<ManualCandidate>> {
    let path = crate::catalog::requested_minerva_database_path().context(
        "The Minerva catalog is unavailable; pass --minerva-database or install minerva.db",
    )?;
    let connection = crate::catalog::open_read_only(&path, "Minerva catalog")?;
    let torrent_url = connection
        .query_row(
            "SELECT t.torrent_url
             FROM minerva_torrent_platforms tp
             JOIN minerva_torrents t ON t.id=tp.torrent_id
             WHERE t.collection=?1 AND tp.minerva_platform=?2
             ORDER BY t.id
             LIMIT 1",
            [MANUAL_COLLECTION, MANUAL_PLATFORM],
            |row| row.get::<_, String>(0),
        )
        .optional()?
        .context("Minerva's dedicated manual-scan torrent is not present in the catalog")?;
    let bytes = crate::game_details::torrent_bytes_for_url(&torrent_url)?;
    manual_candidate_from_bytes(&torrent_url, &bytes, game_title)
}

fn manual_candidate_from_bytes(
    torrent_url: &str,
    bytes: &[u8],
    game_title: &str,
) -> Result<Option<ManualCandidate>> {
    let torrent = Torrent::read_from_bytes(bytes)
        .map_err(|error| anyhow::anyhow!("could not parse Minerva manual torrent: {error}"))?;
    let info_hash = torrent.info_hash();
    let files = if let Some(files) = torrent.files {
        files
            .into_iter()
            .enumerate()
            .map(|(index, file)| {
                let file_path = file
                    .path
                    .components()
                    .map(|component| component.as_os_str().to_string_lossy())
                    .collect::<Vec<_>>()
                    .join("/");
                Ok((
                    u32::try_from(index).context("manual torrent file index is too large")?,
                    file_path,
                    u64::try_from(file.length)
                        .context("manual torrent file has a negative byte length")?,
                ))
            })
            .collect::<Result<Vec<_>>>()?
    } else {
        vec![(
            0,
            torrent.name,
            u64::try_from(torrent.length)
                .context("manual torrent file has a negative byte length")?,
        )]
    };

    let mut candidates = files
        .into_iter()
        .filter(|(_, path, size)| {
            *size > 0
                && Path::new(path)
                    .extension()
                    .and_then(|extension| extension.to_str())
                    .is_some_and(|extension| extension.eq_ignore_ascii_case("zip"))
        })
        .filter_map(|(file_index, file_path, file_size)| {
            let match_score = crate::game_details::torrent_file_match_score(&file_path, game_title);
            (match_score >= MINERVA_MANUAL_MATCH_THRESHOLD).then_some(ManualCandidate {
                torrent_url: torrent_url.to_owned(),
                torrent_bytes: bytes.to_vec(),
                info_hash: info_hash.clone(),
                file_index,
                file_path,
                file_size,
                match_score,
            })
        })
        .collect::<Vec<_>>();
    candidates.sort_by(|left, right| {
        right
            .match_score
            .total_cmp(&left.match_score)
            .then_with(|| left.file_size.cmp(&right.file_size))
            .then_with(|| left.file_path.cmp(&right.file_path))
    });
    Ok(candidates.into_iter().next())
}

fn publish_completed_manual(
    app_settings: &settings::AppSettings,
    transfer: &MediaTransfer,
) -> Result<()> {
    ensure_minerva_torrent_transfer(transfer)?;
    let expected_target =
        crate::media::game_media_directory(&transfer.game_uid, transfer.launchbox_db_id)?
            .join(MINERVA_PROVIDER)
            .join("manual.zip");
    if transfer.local_target_path != expected_target {
        bail!("Persisted manual target does not match the owned media cache path");
    }

    let (native_root, _) = media_download_roots(app_settings)?;
    let media_root = crate::media::requested_media_directory();
    publish_completed_manual_from_roots(&native_root, &media_root, &expected_target, transfer)
}

fn publish_completed_manual_from_roots(
    native_root: &Path,
    media_root: &Path,
    expected_target: &Path,
    transfer: &MediaTransfer,
) -> Result<()> {
    let canonical_root = native_root.canonicalize().with_context(|| {
        format!(
            "canonicalizing supplemental-media download directory {}",
            native_root.display()
        )
    })?;
    let canonical_source = transfer
        .local_download_path
        .canonicalize()
        .with_context(|| {
            format!(
                "locating completed manual download {}",
                transfer.local_download_path.display()
            )
        })?;
    if !canonical_source.starts_with(&canonical_root) {
        bail!("Completed manual download escapes the configured torrent library");
    }
    let source_metadata = canonical_source.metadata().with_context(|| {
        format!(
            "inspecting downloaded manual {}",
            canonical_source.display()
        )
    })?;
    if !source_metadata.is_file() || source_metadata.len() == 0 {
        bail!("Completed manual download is not a non-empty regular file");
    }
    let archive_file = fs::File::open(&canonical_source)
        .with_context(|| format!("opening downloaded manual {}", canonical_source.display()))?;
    let archive = zip::ZipArchive::new(archive_file).with_context(|| {
        format!(
            "validating downloaded manual {}",
            canonical_source.display()
        )
    })?;
    if archive.is_empty() {
        bail!("Completed manual ZIP contains no files");
    }
    drop(archive);

    fs::create_dir_all(media_root)
        .with_context(|| format!("creating media directory {}", media_root.display()))?;
    let target_parent = expected_target
        .parent()
        .context("manual target has no parent directory")?;
    fs::create_dir_all(target_parent).with_context(|| {
        format!(
            "creating manual cache directory {}",
            target_parent.display()
        )
    })?;
    let canonical_media_root = media_root
        .canonicalize()
        .with_context(|| format!("canonicalizing media directory {}", media_root.display()))?;
    let canonical_target_parent = target_parent.canonicalize().with_context(|| {
        format!(
            "canonicalizing manual directory {}",
            target_parent.display()
        )
    })?;
    if !canonical_target_parent.starts_with(&canonical_media_root) {
        bail!("Manual target escapes the configured media directory");
    }
    if expected_target
        .metadata()
        .is_ok_and(|metadata| metadata.is_file() && metadata.len() > 0)
    {
        return Ok(());
    }

    let temporary = target_parent.join(format!(".manual.{}.tmp", uuid::Uuid::new_v4()));
    let result = (|| -> Result<()> {
        let mut source = fs::File::open(&canonical_source)
            .with_context(|| format!("opening downloaded manual {}", canonical_source.display()))?;
        let mut target = OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&temporary)
            .with_context(|| format!("creating temporary manual {}", temporary.display()))?;
        let copied = std::io::copy(&mut source, &mut target)
            .with_context(|| format!("copying manual to {}", temporary.display()))?;
        if copied != source_metadata.len() {
            bail!("Manual copy length changed while publishing");
        }
        target
            .flush()
            .with_context(|| format!("flushing manual {}", temporary.display()))?;
        target
            .sync_all()
            .with_context(|| format!("syncing manual {}", temporary.display()))?;
        drop(target);
        if expected_target.exists() {
            bail!("A manual appeared in the media cache while publishing");
        }
        fs::rename(&temporary, expected_target).with_context(|| {
            format!(
                "publishing manual {} to {}",
                temporary.display(),
                expected_target.display()
            )
        })?;
        Ok(())
    })();
    if result.is_err() {
        let _ = fs::remove_file(&temporary);
    }
    result
}

#[cfg(test)]
mod tests {
    use std::fs;
    use std::io::Write as _;
    use std::path::PathBuf;

    use lava_torrent::torrent::v1::{File, Torrent};
    use zip::write::SimpleFileOptions;

    use super::{
        MINERVA_MANUAL_MATCH_THRESHOLD, active_selection, manual_candidate_from_bytes,
        publish_completed_manual_from_roots, rehydrate_local_paths, transfer_needs_refresh,
    };
    use crate::settings::{AppSettings, MediaTransfer};

    fn transfer(game: &str, index: u32, state: &str) -> MediaTransfer {
        MediaTransfer {
            game_uid: game.into(),
            launchbox_db_id: 1,
            title: game.into(),
            platform: "Sega Pico".into(),
            media_kind: "manual".into(),
            provider: "minerva".into(),
            transport: "torrent".into(),
            source_url: "https://example.invalid/manuals.torrent".into(),
            remote_job_id: "abcd".into(),
            source_item_index: index,
            source_item_path: format!("Manuals/{game}.zip"),
            local_download_path: PathBuf::from(format!("/downloads/{game}.zip")),
            local_target_path: PathBuf::from(format!("/media/{game}/manual.zip")),
            state: state.into(),
            progress: 0.0,
            download_speed: 0,
            downloaded_bytes: 0,
            total_bytes: 100,
            message: "Queued".into(),
            created_at: 1,
            updated_at: 1,
        }
    }

    #[test]
    fn manual_candidate_requires_an_exact_zip_title() {
        let torrent = Torrent {
            announce: Some("https://tracker.example/announce".into()),
            announce_list: None,
            length: 230,
            files: Some(vec![
                File {
                    length: 100,
                    path: PathBuf::from(
                        "Manuals/Cooking Pico - Minna to Issho ni Hajimete Cooking! (Japan).zip",
                    ),
                    extra_fields: None,
                },
                File {
                    length: 80,
                    path: PathBuf::from("Manuals/Cooking Pico 2 (Japan).zip"),
                    extra_fields: None,
                },
                File {
                    length: 50,
                    path: PathBuf::from("Manuals/Unrelated Game.pdf"),
                    extra_fields: None,
                },
            ]),
            name: "Manual Archive".into(),
            piece_length: 16_384,
            pieces: vec![vec![0xff; 20]],
            extra_fields: None,
            extra_info_fields: None,
        };
        let bytes = torrent.encode().unwrap();
        let selected = manual_candidate_from_bytes(
            "https://example.invalid/manuals.torrent",
            &bytes,
            "Cooking Pico - Minna to Issho ni Hajimete Cooking! (Japan)",
        )
        .unwrap()
        .unwrap();
        assert_eq!(selected.file_index, 0);
        assert!(selected.match_score >= MINERVA_MANUAL_MATCH_THRESHOLD);
        assert!(
            manual_candidate_from_bytes(
                "https://example.invalid/manuals.torrent",
                &bytes,
                "Cooking Pico"
            )
            .unwrap()
            .is_none()
        );
    }

    #[test]
    fn shared_torrent_selection_keeps_other_active_manuals() {
        let selection = active_selection(vec![
            transfer("first", 3, "downloading"),
            transfer("second", 7, "paused"),
            transfer("finished", 9, "published"),
        ]);
        assert_eq!(
            selection,
            vec![
                (3, "Manuals/first.zip".into()),
                (7, "Manuals/second.zip".into())
            ]
        );
        assert!(transfer_needs_refresh("complete"));
        assert!(!transfer_needs_refresh("published"));
    }

    #[test]
    fn durable_transfer_paths_rehydrate_for_the_current_host() {
        let directory = tempfile::tempdir().unwrap();
        let mut persisted = transfer("game", 3, "downloading");
        persisted.local_download_path = PathBuf::from(r"D:\old\Manuals\game.zip");
        persisted.local_target_path = PathBuf::from(r"D:\old\media\manual.zip");
        let settings = AppSettings {
            torrent_library_directory: directory.path().join("downloads"),
            qbittorrent_container_torrent_library_directory: "/client/downloads".into(),
            ..AppSettings::default()
        };

        rehydrate_local_paths(&settings, &mut persisted).unwrap();

        assert_eq!(
            persisted.local_download_path,
            directory
                .path()
                .join("downloads")
                .join("lunchbox-media")
                .join("Manuals")
                .join("game.zip")
        );
        assert!(
            persisted
                .local_target_path
                .ends_with(PathBuf::from("lb-1").join("minerva").join("manual.zip"))
        );
    }

    #[test]
    fn completed_manual_is_validated_and_atomically_published() {
        let directory = tempfile::tempdir().unwrap();
        let native_root = directory.path().join("downloads");
        let media_root = directory.path().join("media");
        fs::create_dir_all(native_root.join("Manuals")).unwrap();
        let source = native_root.join("Manuals/Game.zip");
        let archive_file = fs::File::create(&source).unwrap();
        let mut archive = zip::ZipWriter::new(archive_file);
        archive
            .start_file("page-001.jpg", SimpleFileOptions::default())
            .unwrap();
        archive.write_all(b"jpeg fixture").unwrap();
        archive.finish().unwrap();

        let expected_target = media_root.join("lb-42/minerva/manual.zip");
        let mut transfer = transfer("game", 3, "complete");
        transfer.local_download_path = source;
        transfer.local_target_path = expected_target.clone();
        publish_completed_manual_from_roots(&native_root, &media_root, &expected_target, &transfer)
            .unwrap();
        assert!(expected_target.is_file());
        assert_eq!(
            zip::ZipArchive::new(fs::File::open(&expected_target).unwrap())
                .unwrap()
                .len(),
            1
        );
        assert_eq!(
            fs::read_dir(expected_target.parent().unwrap())
                .unwrap()
                .count(),
            1
        );

        let invalid_source = native_root.join("Manuals/Invalid.zip");
        fs::write(&invalid_source, b"not a zip").unwrap();
        transfer.local_download_path = invalid_source;
        transfer.local_target_path = media_root.join("lb-43/minerva/manual.zip");
        assert!(
            publish_completed_manual_from_roots(
                &native_root,
                &media_root,
                &transfer.local_target_path,
                &transfer,
            )
            .is_err()
        );
    }
}
