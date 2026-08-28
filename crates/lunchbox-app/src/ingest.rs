use std::fs;
use std::io::{Read, Write};
use std::path::{Path, PathBuf};

use anyhow::{Context, Result, bail};
use sha1::{Digest, Sha1};

use crate::download_plan::{DownloadPlan, DownloadPlanMember};
use crate::settings::{AppSettings, DownloadJob, SettingsStore};

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

    let installed_path = if settings.file_link_mode == "leave_in_place" {
        source.clone()
    } else {
        let target = &job.local_target_path;
        let parent = target
            .parent()
            .context("ROM target path has no parent directory")?;
        fs::create_dir_all(parent)
            .with_context(|| format!("creating ROM directory {}", parent.display()))?;

        if target.exists() {
            if !files_equal(source, target)? {
                bail!(
                    "refusing to replace a different existing ROM at {}",
                    target.display()
                );
            }
        } else {
            install_file(&settings.file_link_mode, source, target)?;
        }
        target.clone()
    };

    store.record_installed(job, &installed_path)?;
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
            Ok((member, source, installed))
        })
        .collect::<Result<Vec<_>>>()?;

    if settings.file_link_mode != "leave_in_place" || requires_staged_layout {
        let install_mode = if settings.file_link_mode == "leave_in_place" {
            "symlink"
        } else {
            settings.file_link_mode.as_str()
        };
        for (_, source, target) in &planned_files {
            if target.exists() && !files_equal(source, target)? {
                bail!(
                    "refusing to replace a different existing ROM at {}",
                    target.display()
                );
            }
        }
        for (_, source, target) in &planned_files {
            let parent = target
                .parent()
                .context("planned ROM target has no parent directory")?;
            fs::create_dir_all(parent)
                .with_context(|| format!("creating ROM directory {}", parent.display()))?;
            if !target.exists() {
                install_file(install_mode, source, target)?;
            }
        }
    }

    if !plan.is_optical_multidisc() {
        let installed_path = planned_files
            .iter()
            .find(|(member, _, _)| member.index == plan.representative_index)
            .map(|(_, _, installed)| installed.clone())
            .context("download plan representative was not staged")?;
        store.record_installed(job, &installed_path)?;
        return Ok(installed_path);
    }
    let playlist_parent = if settings.file_link_mode == "leave_in_place" {
        common_parent(
            &planned_files
                .iter()
                .map(|(_, _, installed)| installed.clone())
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
    write_idempotent_playlist(&playlist_path, playlist_content.as_bytes())?;
    store.record_installed(job, &playlist_path)?;
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
    planned_files: &[(&DownloadPlanMember, PathBuf, PathBuf)],
    playlist_parent: &Path,
) -> Result<String> {
    let mut content = String::new();
    for member in plan.playlist_members() {
        let installed = planned_files
            .iter()
            .find(|(candidate, _, _)| candidate.index == member.index)
            .map(|(_, _, installed)| installed)
            .with_context(|| format!("playlist member {} is missing", member.index))?;
        let relative = installed.strip_prefix(playlist_parent).unwrap_or(installed);
        content.push_str(&relative.to_string_lossy().replace('\\', "/"));
        content.push('\n');
    }
    Ok(content)
}

fn write_idempotent_playlist(path: &Path, content: &[u8]) -> Result<()> {
    if path.exists() {
        if fs::read(path)? == content {
            return Ok(());
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
        Ok(())
    })();
    if write_result.is_err() {
        let _ = fs::remove_file(&temporary);
    }
    write_result
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
