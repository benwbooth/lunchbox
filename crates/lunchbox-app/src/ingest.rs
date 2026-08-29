use std::fs;
use std::io::{Read, Write};
use std::path::{Path, PathBuf};

use anyhow::{Context, Result, bail};
use rusqlite::OptionalExtension;
use sha1::{Digest, Sha1};
use sha2::Sha256;

use crate::download_plan::{DownloadPlan, DownloadPlanMember};
use crate::settings::{AppSettings, DownloadJob, InstalledFileReceipt, SettingsStore};

const INSTALL_MANAGEMENT_PROBE_GAME_UID: &str = "9697a5eb-e0b4-4f24-8d43-672701414ee7";

pub(crate) fn seed_install_management_ui_probe() -> Result<PathBuf> {
    let state_path = crate::catalog::requested_path("--state-database", "LUNCHBOX_STATE_DATABASE")
        .context("the install-management UI probe requires --state-database")?;
    let fixture_root = crate::catalog::requested_path(
        "--install-management-fixture-root",
        "LUNCHBOX_INSTALL_MANAGEMENT_FIXTURE_ROOT",
    )
    .context("the install-management UI probe requires --install-management-fixture-root")?;
    if !fixture_root.is_absolute() {
        bail!("the install-management fixture root must be absolute");
    }
    let store = SettingsStore::at(state_path)?;
    let rom_root = fixture_root.join("roms");
    let installed = rom_root
        .join("Nintendo Entertainment System")
        .join("Super Mario Bros. (USA).nes");
    fs::create_dir_all(
        installed
            .parent()
            .context("install-management fixture has no parent")?,
    )?;
    let fixture_bytes = b"LUNCHBOX_INSTALL_MANAGEMENT_PROBE_ROM\n";
    if path_lexists(&installed)? {
        if fs::read(&installed)? != fixture_bytes {
            bail!(
                "refusing to replace a different install-management fixture at {}",
                installed.display()
            );
        }
    } else {
        fs::write(&installed, fixture_bytes)?;
    }
    let settings = AppSettings {
        rom_directory: rom_root,
        file_link_mode: "copy".to_owned(),
        ..AppSettings::default()
    };
    store.save(&settings)?;
    let job = DownloadJob::queued(crate::settings::NewDownloadJob {
        game_id: INSTALL_MANAGEMENT_PROBE_GAME_UID.to_owned(),
        launchbox_db_id: 140,
        title: "Super Mario Bros.".to_owned(),
        platform: "Nintendo Entertainment System".to_owned(),
        source_kind: "minerva".to_owned(),
        torrent_url: "https://example.invalid/install-management-probe.torrent".to_owned(),
        torrent_file_index: Some(0),
        torrent_file_path: "Super Mario Bros. (USA).nes".to_owned(),
        info_hash: "1".repeat(40),
        client_save_path: "/downloads/lunchbox".to_owned(),
        local_download_path: fixture_root.join("downloads/Super Mario Bros. (USA).nes"),
        local_target_path: installed.clone(),
        download_plan: String::new(),
    });
    let receipt = installed_file_receipt(&installed, true)?;
    store.record_installed(&job, &installed, &[receipt])?;
    Ok(installed)
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ManagedInstallationSummary {
    pub game_uid: String,
    pub source_kind: String,
    pub receipt_count: usize,
    pub owned_file_count: usize,
    pub deletable_file_count: usize,
    pub shared_file_count: usize,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ManagedInstallationResult {
    pub removed_file_count: usize,
    pub retained_file_count: usize,
    pub remaining_local_paths: Vec<PathBuf>,
    pub prepared_cache_removed: bool,
}

#[derive(Clone, Debug)]
struct ManagedInstallFile {
    receipt: InstalledFileReceipt,
    reference_count: usize,
}

#[derive(Clone, Debug)]
struct ManagedInstallation {
    game_uid: String,
    launchbox_db_id: i64,
    source_kind: String,
    files: Vec<ManagedInstallFile>,
}

impl ManagedInstallation {
    fn summary(&self) -> ManagedInstallationSummary {
        ManagedInstallationSummary {
            game_uid: self.game_uid.clone(),
            source_kind: self.source_kind.clone(),
            receipt_count: self.files.len(),
            owned_file_count: self
                .files
                .iter()
                .filter(|file| file.receipt.created_by_lunchbox)
                .count(),
            deletable_file_count: self
                .files
                .iter()
                .filter(|file| file.receipt.created_by_lunchbox && file.reference_count == 1)
                .count(),
            shared_file_count: self
                .files
                .iter()
                .filter(|file| file.reference_count > 1)
                .count(),
        }
    }
}

pub fn inspect_managed_installation(
    store: &SettingsStore,
    game_uid: &str,
) -> Result<Option<ManagedInstallationSummary>> {
    Ok(load_managed_installation(store, game_uid)?.map(|installation| installation.summary()))
}

pub fn uninstall_managed_installation(
    store: &SettingsStore,
    game_uid: &str,
    delete_owned_files: bool,
) -> Result<ManagedInstallationResult> {
    let installation = load_managed_installation(store, game_uid)?
        .with_context(|| format!("{game_uid} is not recorded as an installed game"))?;
    let delete_files = installation
        .files
        .iter()
        .filter(|file| {
            delete_owned_files && file.receipt.created_by_lunchbox && file.reference_count == 1
        })
        .collect::<Vec<_>>();

    // Validate the complete deletion set before changing the filesystem. A missing
    // path is an idempotent retry; a changed path remains the user's data.
    for file in &delete_files {
        if !path_lexists(&file.receipt.path)? {
            continue;
        }
        let current = installed_file_receipt(&file.receipt.path, true)?;
        if current.file_kind != file.receipt.file_kind
            || current.file_size != file.receipt.file_size
            || !current.sha256.eq_ignore_ascii_case(&file.receipt.sha256)
        {
            bail!(
                "refusing to delete modified installed file {}",
                file.receipt.path.display()
            );
        }
    }

    for file in &delete_files {
        if path_lexists(&file.receipt.path)? {
            fs::remove_file(&file.receipt.path).with_context(|| {
                format!("removing installed file {}", file.receipt.path.display())
            })?;
        }
    }

    let mut prepared_cache_removed = false;
    if delete_owned_files {
        let settings = store.load()?;
        for file in &delete_files {
            prune_empty_rom_directories(&file.receipt.path, &settings.rom_directory)?;
        }
        prepared_cache_removed =
            crate::exo_install::remove_cached_install(store, &settings, &installation.game_uid)?;
    }

    let mut connection = store.connection()?;
    let transaction = connection.transaction()?;
    let removed = transaction.execute(
        "DELETE FROM installed_games WHERE game_uid=?1",
        [&installation.game_uid],
    )?;
    if removed != 1 {
        bail!("the installed-game record changed during removal");
    }
    transaction.execute(
        "DELETE FROM installed_file_receipts
         WHERE NOT EXISTS (
             SELECT 1 FROM installed_game_files f
             WHERE f.path_encoding=installed_file_receipts.path_encoding
               AND f.path_bytes=installed_file_receipts.path_bytes
         )",
        [],
    )?;
    transaction.commit()?;

    let remaining_local_paths =
        imported_local_paths(store, &installation.game_uid, installation.launchbox_db_id)?;

    Ok(ManagedInstallationResult {
        removed_file_count: delete_files.len(),
        retained_file_count: installation.files.len().saturating_sub(delete_files.len()),
        remaining_local_paths,
        prepared_cache_removed,
    })
}

fn load_managed_installation(
    store: &SettingsStore,
    game_uid: &str,
) -> Result<Option<ManagedInstallation>> {
    let connection = store.connection()?;
    let installed = connection
        .query_row(
            "SELECT launchbox_db_id, import_source
             FROM installed_games WHERE game_uid=?1",
            [game_uid],
            |row| Ok((row.get::<_, i64>(0)?, row.get::<_, String>(1)?)),
        )
        .optional()?;
    let Some((launchbox_db_id, source_kind)) = installed else {
        return Ok(None);
    };
    let mut statement = connection.prepare(
        "SELECT r.path_encoding, r.path_bytes, r.path_display, r.file_kind,
                r.file_size, r.sha256, r.created_by_lunchbox,
                (SELECT count(*) FROM installed_game_files refs
                 WHERE refs.path_encoding=f.path_encoding
                   AND refs.path_bytes=f.path_bytes)
         FROM installed_game_files f
         JOIN installed_file_receipts r
           ON r.path_encoding=f.path_encoding AND r.path_bytes=f.path_bytes
         WHERE f.game_uid=?1
         ORDER BY f.is_primary DESC, r.path_display COLLATE NOCASE",
    )?;
    let rows = statement.query_map([game_uid], |row| {
        let encoding = row.get::<_, String>(0)?;
        let bytes = row.get::<_, Vec<u8>>(1)?;
        let display = row.get::<_, String>(2)?;
        Ok(ManagedInstallFile {
            receipt: InstalledFileReceipt {
                path: crate::local_import::decode_path(bytes, &encoding, display),
                file_kind: row.get(3)?,
                file_size: u64::try_from(row.get::<_, i64>(4)?).unwrap_or(0),
                sha256: row.get(5)?,
                created_by_lunchbox: row.get(6)?,
            },
            reference_count: usize::try_from(row.get::<_, i64>(7)?).unwrap_or(0),
        })
    })?;
    Ok(Some(ManagedInstallation {
        game_uid: game_uid.to_owned(),
        launchbox_db_id,
        source_kind,
        files: rows.collect::<rusqlite::Result<Vec<_>>>()?,
    }))
}

fn imported_local_paths(
    store: &SettingsStore,
    game_uid: &str,
    launchbox_db_id: i64,
) -> Result<Vec<PathBuf>> {
    let connection = store.connection()?;
    let mut statement = connection.prepare(
        "SELECT path_display, path_bytes, path_encoding
         FROM local_rom_files
         WHERE included=1 AND availability='present'
           AND (game_uid=?1 OR (?2 > 0 AND launchbox_db_id=?2))
         ORDER BY relative_path_display COLLATE NOCASE, id",
    )?;
    let rows = statement.query_map(rusqlite::params![game_uid, launchbox_db_id], |row| {
        Ok(crate::local_import::decode_path(
            row.get::<_, Vec<u8>>(1)?,
            &row.get::<_, String>(2)?,
            row.get::<_, String>(0)?,
        ))
    })?;
    let mut paths = rows.collect::<rusqlite::Result<Vec<_>>>()?;
    paths.retain(|path| path.is_file());
    paths.dedup();
    Ok(paths)
}

fn prune_empty_rom_directories(path: &Path, rom_root: &Path) -> Result<()> {
    if rom_root.as_os_str().is_empty() || !rom_root.is_absolute() {
        return Ok(());
    }
    let mut directory = match path.parent() {
        Some(parent) if parent != rom_root && parent.starts_with(rom_root) => parent.to_path_buf(),
        _ => return Ok(()),
    };
    while directory != rom_root && directory.starts_with(rom_root) {
        match fs::remove_dir(&directory) {
            Ok(()) => {}
            Err(error)
                if matches!(
                    error.kind(),
                    std::io::ErrorKind::DirectoryNotEmpty | std::io::ErrorKind::NotFound
                ) =>
            {
                break;
            }
            Err(error) => {
                return Err(error).with_context(|| {
                    format!("pruning empty ROM directory {}", directory.display())
                });
            }
        }
        let Some(parent) = directory.parent() else {
            break;
        };
        directory = parent.to_path_buf();
    }
    Ok(())
}

pub fn ingest_completed(
    settings: &AppSettings,
    store: &SettingsStore,
    job: &DownloadJob,
) -> Result<PathBuf> {
    if let Some(plan) = job.parsed_download_plan()? {
        return ingest_download_plan(settings, store, job, &plan);
    }
    let source = &job.local_download_path;
    let metadata = source
        .metadata()
        .with_context(|| format!("downloaded file is not available at {}", source.display()))?;
    if !metadata.is_file() {
        bail!(
            "downloaded path is not a regular file: {}",
            source.display()
        );
    }

    let (installed_path, created_by_lunchbox) = if settings.file_link_mode == "leave_in_place" {
        (source.clone(), false)
    } else {
        let target = &job.local_target_path;
        let parent = target
            .parent()
            .context("ROM target path has no parent directory")?;
        fs::create_dir_all(parent)
            .with_context(|| format!("creating ROM directory {}", parent.display()))?;

        let existed = path_lexists(target)?;
        if existed {
            if !files_equal(source, target)? {
                bail!(
                    "refusing to replace a different existing ROM at {}",
                    target.display()
                );
            }
        } else {
            install_file(&settings.file_link_mode, source, target)?;
        }
        (target.clone(), !existed)
    };

    let receipt = installed_file_receipt(&installed_path, created_by_lunchbox)?;
    store.record_installed(job, &installed_path, &[receipt])?;
    Ok(installed_path)
}

fn ingest_download_plan(
    settings: &AppSettings,
    store: &SettingsStore,
    job: &DownloadJob,
    plan: &DownloadPlan,
) -> Result<PathBuf> {
    plan.validate()?;
    let requires_staged_layout = plan.is_arcade_machine_layout();
    let source_root = plan_source_root(job, plan)?;
    let target_root = if plan.is_optical_multidisc() {
        job.local_target_path
            .parent()
            .context("download-plan playlist target has no parent directory")?
            .to_path_buf()
    } else {
        plan_target_root(job, plan)?
    };

    let planned_files = plan
        .members
        .iter()
        .map(|member| {
            let source = source_root.join(relative_path(&member.torrent_path));
            let metadata = source.metadata().with_context(|| {
                format!("planned download is not available at {}", source.display())
            })?;
            if !metadata.is_file() {
                bail!(
                    "planned download is not a regular file: {}",
                    source.display()
                );
            }
            let installed =
                if settings.file_link_mode == "leave_in_place" && !requires_staged_layout {
                    source.clone()
                } else {
                    target_root.join(relative_path(&member.target_relative_path))
                };
            let created_by_lunchbox = (settings.file_link_mode != "leave_in_place"
                || requires_staged_layout)
                && !path_lexists(&installed)?;
            Ok((member, source, installed, created_by_lunchbox))
        })
        .collect::<Result<Vec<_>>>()?;

    if settings.file_link_mode != "leave_in_place" || requires_staged_layout {
        let install_mode = if settings.file_link_mode == "leave_in_place" {
            "symlink"
        } else {
            settings.file_link_mode.as_str()
        };
        for (_, source, target, created_by_lunchbox) in &planned_files {
            if !*created_by_lunchbox && !files_equal(source, target)? {
                bail!(
                    "refusing to replace a different existing ROM at {}",
                    target.display()
                );
            }
        }
        for (_, source, target, created_by_lunchbox) in &planned_files {
            let parent = target
                .parent()
                .context("planned ROM target has no parent directory")?;
            fs::create_dir_all(parent)
                .with_context(|| format!("creating ROM directory {}", parent.display()))?;
            if *created_by_lunchbox {
                install_file(install_mode, source, target)?;
            }
        }
    }

    let mut receipts = planned_files
        .iter()
        .map(|(_, _, installed, created_by_lunchbox)| {
            installed_file_receipt(installed, *created_by_lunchbox)
        })
        .collect::<Result<Vec<_>>>()?;

    if !plan.is_optical_multidisc() {
        let installed_path = planned_files
            .iter()
            .find(|(member, _, _, _)| member.index == plan.representative_index)
            .map(|(_, _, installed, _)| installed.clone())
            .context("download plan representative was not staged")?;
        store.record_installed(job, &installed_path, &receipts)?;
        return Ok(installed_path);
    }
    let playlist_parent = if settings.file_link_mode == "leave_in_place" {
        common_parent(
            &planned_files
                .iter()
                .map(|(_, _, installed, _)| installed.clone())
                .collect::<Vec<_>>(),
        )
        .context("planned downloads do not have a common playlist directory")?
    } else {
        target_root.clone()
    };
    fs::create_dir_all(&playlist_parent)
        .with_context(|| format!("creating playlist directory {}", playlist_parent.display()))?;
    let playlist_path = playlist_parent.join(&plan.playlist_filename);
    let playlist_content = playlist_content(plan, &planned_files, &playlist_parent)?;
    let playlist_created = write_idempotent_playlist(&playlist_path, playlist_content.as_bytes())?;
    receipts.push(installed_file_receipt(&playlist_path, playlist_created)?);
    store.record_installed(job, &playlist_path, &receipts)?;
    Ok(playlist_path)
}

fn plan_source_root(job: &DownloadJob, plan: &DownloadPlan) -> Result<PathBuf> {
    let representative = plan
        .members
        .iter()
        .find(|member| member.index == plan.representative_index)
        .context("download plan has no representative member")?;
    let relative = relative_path(&representative.torrent_path);
    let mut root = job.local_download_path.clone();
    for _ in relative.components() {
        if !root.pop() {
            bail!("download plan source path is outside the configured download root");
        }
    }
    if root.join(&relative) != job.local_download_path {
        bail!("download plan source root does not match the representative file");
    }
    Ok(root)
}

fn plan_target_root(job: &DownloadJob, plan: &DownloadPlan) -> Result<PathBuf> {
    let representative = plan
        .representative_member()
        .context("download plan has no representative member")?;
    let relative = relative_path(&representative.target_relative_path);
    let mut root = job.local_target_path.clone();
    for _ in relative.components() {
        if !root.pop() {
            bail!("download plan target path is outside the configured archive cache");
        }
    }
    if root.join(&relative) != job.local_target_path {
        bail!("download plan target root does not match the representative file");
    }
    Ok(root)
}

fn relative_path(value: &str) -> PathBuf {
    PathBuf::from(value.replace('\\', "/"))
}

fn common_parent(paths: &[PathBuf]) -> Option<PathBuf> {
    let first = paths.first()?.parent()?.to_path_buf();
    let mut common = first.components().collect::<Vec<_>>();
    for path in paths.iter().skip(1) {
        let components = path.parent()?.components().collect::<Vec<_>>();
        let shared = common
            .iter()
            .zip(components.iter())
            .take_while(|(left, right)| left == right)
            .count();
        common.truncate(shared);
    }
    if common.is_empty() {
        return None;
    }
    let mut path = PathBuf::new();
    for component in common {
        path.push(component.as_os_str());
    }
    Some(path)
}

fn playlist_content(
    plan: &DownloadPlan,
    planned_files: &[(&DownloadPlanMember, PathBuf, PathBuf, bool)],
    playlist_parent: &Path,
) -> Result<String> {
    let mut content = String::new();
    for member in plan.playlist_members() {
        let installed = planned_files
            .iter()
            .find(|(candidate, _, _, _)| candidate.index == member.index)
            .map(|(_, _, installed, _)| installed)
            .with_context(|| format!("playlist member {} is missing", member.index))?;
        let relative = installed.strip_prefix(playlist_parent).unwrap_or(installed);
        content.push_str(&relative.to_string_lossy().replace('\\', "/"));
        content.push('\n');
    }
    Ok(content)
}

fn write_idempotent_playlist(path: &Path, content: &[u8]) -> Result<bool> {
    if path_lexists(path)? {
        if fs::read(path)? == content {
            return Ok(false);
        }
        bail!(
            "refusing to replace a different existing playlist at {}",
            path.display()
        );
    }
    let parent = path.parent().context("playlist path has no parent")?;
    let temporary = parent.join(format!(
        ".{}.{}.tmp",
        path.file_name()
            .and_then(|name| name.to_str())
            .unwrap_or("lunchbox-playlist"),
        uuid::Uuid::new_v4()
    ));
    let write_result = (|| {
        let mut file = fs::OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&temporary)
            .with_context(|| format!("creating playlist temporary file {}", temporary.display()))?;
        file.write_all(content)?;
        file.sync_all()?;
        fs::rename(&temporary, path)
            .with_context(|| format!("publishing playlist {}", path.display()))?;
        Ok(true)
    })();
    if write_result.is_err() {
        let _ = fs::remove_file(&temporary);
    }
    write_result
}

fn path_lexists(path: &Path) -> Result<bool> {
    match fs::symlink_metadata(path) {
        Ok(_) => Ok(true),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(false),
        Err(error) => {
            Err(error).with_context(|| format!("inspecting installed path {}", path.display()))
        }
    }
}

pub(crate) fn installed_file_receipt(
    path: &Path,
    created_by_lunchbox: bool,
) -> Result<InstalledFileReceipt> {
    let metadata = fs::symlink_metadata(path)
        .with_context(|| format!("inspecting installed file {}", path.display()))?;
    let (file_kind, file_size, sha256) = if metadata.file_type().is_symlink() {
        let target = fs::read_link(path)
            .with_context(|| format!("reading installed link {}", path.display()))?;
        let encoded = crate::local_import::encode_path(&target);
        let mut digest = Sha256::new();
        digest.update(encoded.encoding.as_bytes());
        digest.update([0]);
        digest.update(encoded.bytes);
        ("symlink".to_owned(), 0, hex::encode(digest.finalize()))
    } else if metadata.is_file() {
        let mut file = fs::File::open(path)
            .with_context(|| format!("opening installed file {}", path.display()))?;
        let mut digest = Sha256::new();
        let mut buffer = [0_u8; 1024 * 1024];
        loop {
            let read = file.read(&mut buffer)?;
            if read == 0 {
                break;
            }
            digest.update(&buffer[..read]);
        }
        (
            "regular".to_owned(),
            metadata.len(),
            hex::encode(digest.finalize()),
        )
    } else {
        bail!(
            "installed path is not a regular file or link: {}",
            path.display()
        );
    };
    Ok(InstalledFileReceipt {
        path: path.to_path_buf(),
        file_kind,
        file_size,
        sha256,
        created_by_lunchbox,
    })
}

fn install_file(mode: &str, source: &Path, target: &Path) -> Result<()> {
    match mode {
        "symlink" => create_file_symlink(source, target)
            .with_context(|| format!("linking {} to {}", target.display(), source.display()))?,
        "hardlink" => fs::hard_link(source, target).with_context(|| {
            format!("hard-linking {} to {}", target.display(), source.display())
        })?,
        "reflink" => reflink_copy::reflink(source, target)
            .with_context(|| format!("reflinking {} to {}", target.display(), source.display()))?,
        "copy" => {
            fs::copy(source, target)
                .with_context(|| format!("copying {} to {}", source.display(), target.display()))?;
        }
        unsupported => bail!("unsupported file link mode {unsupported}"),
    }
    Ok(())
}

#[cfg(unix)]
fn create_file_symlink(source: &Path, target: &Path) -> std::io::Result<()> {
    std::os::unix::fs::symlink(source.canonicalize()?, target)
}

#[cfg(windows)]
fn create_file_symlink(source: &Path, target: &Path) -> std::io::Result<()> {
    std::os::windows::fs::symlink_file(source.canonicalize()?, target)
}

fn files_equal(left: &Path, right: &Path) -> Result<bool> {
    if fs::symlink_metadata(right)?.file_type().is_symlink() {
        return Ok(left.canonicalize()? == right.canonicalize()?);
    }
    let left_metadata = left.metadata()?;
    let right_metadata = right.metadata()?;
    if !left_metadata.is_file()
        || !right_metadata.is_file()
        || left_metadata.len() != right_metadata.len()
    {
        return Ok(false);
    }
    Ok(file_sha1(left)? == file_sha1(right)?)
}

fn file_sha1(path: &Path) -> Result<[u8; 20]> {
    let mut file = fs::File::open(path)?;
    let mut digest = Sha1::new();
    let mut buffer = [0_u8; 1024 * 1024];
    loop {
        let read = file.read(&mut buffer)?;
        if read == 0 {
            break;
        }
        digest.update(&buffer[..read]);
    }
    Ok(digest.finalize().into())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::arcade_download::build_mame_laserdisc_plans;
    use crate::download_plan::{TorrentPlanFile, build_exo_plan, build_optical_plan};
    use crate::settings::NewDownloadJob;

    fn job(root: &Path) -> DownloadJob {
        DownloadJob::queued(NewDownloadJob {
            game_id: "game-1".into(),
            launchbox_db_id: 42,
            title: "Game".into(),
            platform: "Platform".into(),
            source_kind: "minerva".into(),
            torrent_url: "https://example.invalid/game.torrent".into(),
            torrent_file_index: Some(0),
            torrent_file_path: "Game.zip".into(),
            info_hash: "abc".into(),
            client_save_path: "/downloads/lunchbox".into(),
            local_download_path: root.join("downloads/Game.zip"),
            local_target_path: root.join("roms/Platform/Game.zip"),
            download_plan: String::new(),
        })
    }

    #[test]
    fn copy_ingestion_is_durable_and_idempotent() {
        let directory = tempfile::tempdir().unwrap();
        let store = SettingsStore::at(directory.path().join("state.db")).unwrap();
        let job = job(directory.path());
        fs::create_dir_all(job.local_download_path.parent().unwrap()).unwrap();
        fs::write(&job.local_download_path, b"rom bytes").unwrap();
        let settings = AppSettings {
            file_link_mode: "copy".into(),
            ..AppSettings::default()
        };

        let installed = ingest_completed(&settings, &store, &job).unwrap();
        assert_eq!(fs::read(&installed).unwrap(), b"rom bytes");
        assert_eq!(
            ingest_completed(&settings, &store, &job).unwrap(),
            installed
        );
        assert_eq!(
            inspect_managed_installation(&store, &job.game_id).unwrap(),
            Some(ManagedInstallationSummary {
                game_uid: job.game_id.clone(),
                source_kind: "minerva".into(),
                receipt_count: 1,
                owned_file_count: 1,
                deletable_file_count: 1,
                shared_file_count: 0,
            })
        );
    }

    #[test]
    fn rerecording_an_install_replaces_stale_file_memberships() {
        let directory = tempfile::tempdir().unwrap();
        let store = SettingsStore::at(directory.path().join("state.db")).unwrap();
        let mut job = job(directory.path());
        let first = directory.path().join("roms/Platform/first.rom");
        let second = directory.path().join("roms/Platform/second.rom");
        fs::create_dir_all(first.parent().unwrap()).unwrap();
        fs::write(&first, b"first install").unwrap();
        fs::write(&second, b"replacement install").unwrap();
        store
            .record_installed(
                &job,
                &first,
                &[installed_file_receipt(&first, true).unwrap()],
            )
            .unwrap();
        job.local_target_path = second.clone();
        store
            .record_installed(
                &job,
                &second,
                &[installed_file_receipt(&second, true).unwrap()],
            )
            .unwrap();

        let summary = inspect_managed_installation(&store, &job.game_id)
            .unwrap()
            .unwrap();
        assert_eq!(summary.receipt_count, 1);

        let result = uninstall_managed_installation(&store, &job.game_id, true).unwrap();
        assert_eq!(result.removed_file_count, 1);
        assert!(first.exists());
        assert!(!second.exists());
    }

    #[test]
    fn uninstall_deletes_only_verified_owned_copy() {
        let directory = tempfile::tempdir().unwrap();
        let store = SettingsStore::at(directory.path().join("state.db")).unwrap();
        let job = job(directory.path());
        fs::create_dir_all(job.local_download_path.parent().unwrap()).unwrap();
        fs::write(&job.local_download_path, b"rom bytes").unwrap();
        let settings = AppSettings {
            file_link_mode: "copy".into(),
            rom_directory: directory.path().join("roms"),
            ..AppSettings::default()
        };
        let installed = ingest_completed(&settings, &store, &job).unwrap();

        let result = uninstall_managed_installation(&store, &job.game_id, true).unwrap();

        assert_eq!(result.removed_file_count, 1);
        assert_eq!(result.retained_file_count, 0);
        assert!(!installed.exists());
        assert_eq!(fs::read(&job.local_download_path).unwrap(), b"rom bytes");
        assert_eq!(
            inspect_managed_installation(&store, &job.game_id).unwrap(),
            None
        );
    }

    #[test]
    fn leave_in_place_removal_detaches_without_deleting_user_file() {
        let directory = tempfile::tempdir().unwrap();
        let store = SettingsStore::at(directory.path().join("state.db")).unwrap();
        let job = job(directory.path());
        fs::create_dir_all(job.local_download_path.parent().unwrap()).unwrap();
        fs::write(&job.local_download_path, b"user rom").unwrap();
        let settings = AppSettings {
            file_link_mode: "leave_in_place".into(),
            ..AppSettings::default()
        };
        ingest_completed(&settings, &store, &job).unwrap();
        let summary = inspect_managed_installation(&store, &job.game_id)
            .unwrap()
            .unwrap();
        assert_eq!(summary.receipt_count, 1);
        assert_eq!(summary.owned_file_count, 0);

        let result = uninstall_managed_installation(&store, &job.game_id, false).unwrap();

        assert_eq!(result.removed_file_count, 0);
        assert_eq!(result.retained_file_count, 1);
        assert_eq!(fs::read(&job.local_download_path).unwrap(), b"user rom");
    }

    #[test]
    fn uninstall_refuses_a_modified_owned_file_without_detaching() {
        let directory = tempfile::tempdir().unwrap();
        let store = SettingsStore::at(directory.path().join("state.db")).unwrap();
        let job = job(directory.path());
        fs::create_dir_all(job.local_download_path.parent().unwrap()).unwrap();
        fs::write(&job.local_download_path, b"original").unwrap();
        let settings = AppSettings {
            file_link_mode: "copy".into(),
            ..AppSettings::default()
        };
        let installed = ingest_completed(&settings, &store, &job).unwrap();
        fs::write(&installed, b"modified").unwrap();

        let error = uninstall_managed_installation(&store, &job.game_id, true).unwrap_err();

        assert!(error.to_string().contains("refusing to delete modified"));
        assert_eq!(fs::read(&installed).unwrap(), b"modified");
        assert!(
            inspect_managed_installation(&store, &job.game_id)
                .unwrap()
                .is_some()
        );
    }

    #[test]
    fn shared_owned_file_survives_until_its_last_game_is_removed() {
        let directory = tempfile::tempdir().unwrap();
        let store = SettingsStore::at(directory.path().join("state.db")).unwrap();
        let shared = directory.path().join("roms/shared.dat");
        fs::create_dir_all(shared.parent().unwrap()).unwrap();
        fs::write(&shared, b"shared dependency").unwrap();
        let mut first = job(directory.path());
        first.game_id = "first-game".into();
        first.local_target_path = shared.clone();
        let mut second = first.clone();
        second.game_id = "second-game".into();
        let receipt = installed_file_receipt(&shared, true).unwrap();
        store
            .record_installed(&first, &shared, std::slice::from_ref(&receipt))
            .unwrap();
        store
            .record_installed(&second, &shared, std::slice::from_ref(&receipt))
            .unwrap();

        let first_result = uninstall_managed_installation(&store, &first.game_id, true).unwrap();
        assert_eq!(first_result.removed_file_count, 0);
        assert!(shared.exists());
        let second_summary = inspect_managed_installation(&store, &second.game_id)
            .unwrap()
            .unwrap();
        assert_eq!(second_summary.deletable_file_count, 1);

        let second_result = uninstall_managed_installation(&store, &second.game_id, true).unwrap();
        assert_eq!(second_result.removed_file_count, 1);
        assert!(!shared.exists());
    }

    #[test]
    fn preexisting_identical_target_is_never_claimed_or_deleted() {
        let directory = tempfile::tempdir().unwrap();
        let store = SettingsStore::at(directory.path().join("state.db")).unwrap();
        let job = job(directory.path());
        fs::create_dir_all(job.local_download_path.parent().unwrap()).unwrap();
        fs::create_dir_all(job.local_target_path.parent().unwrap()).unwrap();
        fs::write(&job.local_download_path, b"same bytes").unwrap();
        fs::write(&job.local_target_path, b"same bytes").unwrap();
        let settings = AppSettings {
            file_link_mode: "copy".into(),
            ..AppSettings::default()
        };
        ingest_completed(&settings, &store, &job).unwrap();
        assert_eq!(
            inspect_managed_installation(&store, &job.game_id)
                .unwrap()
                .unwrap()
                .owned_file_count,
            0
        );

        uninstall_managed_installation(&store, &job.game_id, true).unwrap();

        assert_eq!(fs::read(&job.local_target_path).unwrap(), b"same bytes");
    }

    #[test]
    fn ingestion_never_overwrites_different_existing_content() {
        let directory = tempfile::tempdir().unwrap();
        let store = SettingsStore::at(directory.path().join("state.db")).unwrap();
        let job = job(directory.path());
        fs::create_dir_all(job.local_download_path.parent().unwrap()).unwrap();
        fs::create_dir_all(job.local_target_path.parent().unwrap()).unwrap();
        fs::write(&job.local_download_path, b"new").unwrap();
        fs::write(&job.local_target_path, b"existing").unwrap();
        let settings = AppSettings {
            file_link_mode: "copy".into(),
            ..AppSettings::default()
        };

        assert!(ingest_completed(&settings, &store, &job).is_err());
        assert_eq!(fs::read(&job.local_target_path).unwrap(), b"existing");
    }

    #[test]
    fn mame_machine_plan_stages_rom_and_chd_before_recording_install() {
        let directory = tempfile::tempdir().unwrap();
        let store = SettingsStore::at(directory.path().join("state.db")).unwrap();
        let files = vec![
            TorrentPlanFile {
                index: 1,
                filename: "Minerva_Myrient/Laserdisc Collection/MAME/ROMs/dlair.zip".into(),
                byte_size: 3,
            },
            TorrentPlanFile {
                index: 2,
                filename: "Minerva_Myrient/Laserdisc Collection/MAME/CHD/dlair/dlair.chd".into(),
                byte_size: 3,
            },
        ];
        let plan = build_mame_laserdisc_plans(&files, "Dragon's Lair", &["dlair".into()])
            .pop()
            .unwrap();
        let source_root = directory.path().join("downloads");
        for (path, contents) in [
            (&files[0].filename, b"rom".as_slice()),
            (&files[1].filename, b"chd".as_slice()),
        ] {
            let source = source_root.join(path);
            fs::create_dir_all(source.parent().unwrap()).unwrap();
            fs::write(source, contents).unwrap();
        }
        let target = directory.path().join("roms/Arcade/MAME/roms/dlair.zip");
        let mut job = job(directory.path());
        job.game_id = "dragons-lair".into();
        job.title = "Dragon's Lair".into();
        job.platform = "Arcade".into();
        job.torrent_file_index = Some(1);
        job.torrent_file_path = files[0].filename.clone();
        job.local_download_path = source_root.join(&files[0].filename);
        job.local_target_path = target.clone();
        job.download_plan = serde_json::to_string(&plan).unwrap();
        let settings = AppSettings {
            file_link_mode: "copy".into(),
            ..AppSettings::default()
        };

        let installed = ingest_completed(&settings, &store, &job).unwrap();
        assert_eq!(installed, target);
        assert_eq!(fs::read(&installed).unwrap(), b"rom");
        assert_eq!(
            fs::read(
                directory
                    .path()
                    .join("roms/Arcade/MAME/roms/dlair/dlair.chd")
            )
            .unwrap(),
            b"chd"
        );
        assert_eq!(
            ingest_completed(&settings, &store, &job).unwrap(),
            installed
        );
    }

    fn optical_job(root: &Path) -> (DownloadJob, DownloadPlan) {
        let files = vec![
            TorrentPlanFile {
                index: 0,
                filename: "Bundle/Game (Disc 1).cue".into(),
                byte_size: 3,
            },
            TorrentPlanFile {
                index: 1,
                filename: "Bundle/Game (Disc 1) (Track 01).bin".into(),
                byte_size: 3,
            },
            TorrentPlanFile {
                index: 2,
                filename: "Bundle/Game (Disc 2).cue".into(),
                byte_size: 3,
            },
            TorrentPlanFile {
                index: 3,
                filename: "Bundle/Game (Disc 2) (Track 01).bin".into(),
                byte_size: 3,
            },
        ];
        let plan = build_optical_plan(&files, 0).unwrap();
        let mut job = job(root);
        job.torrent_file_index = Some(0);
        job.torrent_file_path = "Bundle/Game (Disc 1).cue".into();
        job.local_download_path = root.join("downloads/Bundle/Game (Disc 1).cue");
        job.local_target_path = root.join("roms/Platform/Game/Game.m3u");
        job.download_plan = serde_json::to_string(&plan).unwrap();
        (job, plan)
    }

    #[test]
    fn multidisc_ingestion_preserves_layout_and_publishes_playlist_last() {
        let directory = tempfile::tempdir().unwrap();
        let store = SettingsStore::at(directory.path().join("state.db")).unwrap();
        let (job, plan) = optical_job(directory.path());
        for member in &plan.members {
            let path = directory
                .path()
                .join("downloads")
                .join(relative_path(&member.torrent_path));
            fs::create_dir_all(path.parent().unwrap()).unwrap();
            fs::write(path, format!("{:03}", member.index)).unwrap();
        }
        let settings = AppSettings {
            file_link_mode: "copy".into(),
            ..AppSettings::default()
        };

        let playlist = ingest_completed(&settings, &store, &job).unwrap();
        assert_eq!(playlist, job.local_target_path);
        assert_eq!(
            fs::read_to_string(&playlist).unwrap(),
            "Game (Disc 1).cue\nGame (Disc 2).cue\n"
        );
        for member in &plan.members {
            assert!(
                playlist
                    .parent()
                    .unwrap()
                    .join(relative_path(&member.target_relative_path))
                    .is_file()
            );
        }
        assert_eq!(ingest_completed(&settings, &store, &job).unwrap(), playlist);
    }

    #[test]
    fn multidisc_preflight_does_not_publish_a_partial_layout() {
        let directory = tempfile::tempdir().unwrap();
        let store = SettingsStore::at(directory.path().join("state.db")).unwrap();
        let (job, plan) = optical_job(directory.path());
        let representative = plan
            .members
            .iter()
            .find(|member| member.index == plan.representative_index)
            .unwrap();
        fs::create_dir_all(job.local_download_path.parent().unwrap()).unwrap();
        fs::write(&job.local_download_path, b"cue").unwrap();
        assert_eq!(representative.torrent_path, "Bundle/Game (Disc 1).cue");
        let settings = AppSettings {
            file_link_mode: "copy".into(),
            ..AppSettings::default()
        };

        assert!(ingest_completed(&settings, &store, &job).is_err());
        assert!(!job.local_target_path.exists());
        assert!(!job.local_target_path.parent().unwrap().exists());
    }

    fn exo_job(root: &Path) -> (DownloadJob, DownloadPlan) {
        let files = vec![
            TorrentPlanFile {
                index: 0,
                filename: "Bundle/Content/GameData/eXoDOS/Prince of Persia (1990).zip".into(),
                byte_size: 3,
            },
            TorrentPlanFile {
                index: 1,
                filename: "Bundle/Content/!DOSmetadata.zip".into(),
                byte_size: 3,
            },
            TorrentPlanFile {
                index: 2,
                filename: "Bundle/eXo/eXoDOS/Prince of Persia (1990).zip".into(),
                byte_size: 3,
            },
            TorrentPlanFile {
                index: 3,
                filename: "Bundle/eXo/util/util.zip".into(),
                byte_size: 3,
            },
        ];
        let plan = build_exo_plan(&files, 2).unwrap();
        let representative = plan.representative_member().unwrap();
        let mut job = job(root);
        job.torrent_file_index = Some(2);
        job.torrent_file_path = representative.torrent_path.clone();
        job.local_download_path = root
            .join("downloads")
            .join(relative_path(&representative.torrent_path));
        job.local_target_path = root
            .join("roms/.lunchbox-pc-archives/test-hash")
            .join(relative_path(&representative.target_relative_path));
        job.download_plan = serde_json::to_string(&plan).unwrap();
        (job, plan)
    }

    #[test]
    fn exo_ingestion_stages_one_shared_layout_and_records_the_primary() {
        let directory = tempfile::tempdir().unwrap();
        let store = SettingsStore::at(directory.path().join("state.db")).unwrap();
        let (job, plan) = exo_job(directory.path());
        for member in &plan.members {
            let path = directory
                .path()
                .join("downloads")
                .join(relative_path(&member.torrent_path));
            fs::create_dir_all(path.parent().unwrap()).unwrap();
            fs::write(path, format!("{:03}", member.index)).unwrap();
        }
        let settings = AppSettings {
            file_link_mode: "copy".into(),
            ..AppSettings::default()
        };

        let installed = ingest_completed(&settings, &store, &job).unwrap();
        assert_eq!(installed, job.local_target_path);
        let target_root = plan_target_root(&job, &plan).unwrap();
        for member in &plan.members {
            assert!(
                target_root
                    .join(relative_path(&member.target_relative_path))
                    .is_file()
            );
        }
        assert_eq!(
            ingest_completed(&settings, &store, &job).unwrap(),
            installed
        );
        assert!(!target_root.join("Prince of Persia (1990).m3u").exists());
    }
}
