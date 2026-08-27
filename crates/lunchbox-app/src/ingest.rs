use std::fs;
use std::io::Read;
use std::path::{Path, PathBuf};

use anyhow::{Context, Result, bail};
use sha1::{Digest, Sha1};

use crate::settings::{AppSettings, DownloadJob, SettingsStore};

pub fn ingest_completed(
    settings: &AppSettings,
    store: &SettingsStore,
    job: &DownloadJob,
) -> Result<PathBuf> {
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
}
