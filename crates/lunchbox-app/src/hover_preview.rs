use std::fs;
use std::path::{Path, PathBuf};

use anyhow::{Context, Result, bail};

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CachedVideoPreview {
    pub path: PathBuf,
    pub source: String,
}

pub fn requested_cached_video(
    game_uid: &str,
    launchbox_db_id: i64,
) -> Result<Option<CachedVideoPreview>> {
    let media = crate::media::supplemental_media(game_uid, launchbox_db_id)?;
    Ok(media.video.map(|video| CachedVideoPreview {
        path: video.path,
        source: video.source,
    }))
}

pub fn prepare_ui_probe() -> Result<Option<PathBuf>> {
    let media_root =
        crate::catalog::requested_path("--media-directory", "LUNCHBOX_MEDIA_DIRECTORY")
            .context("the hover-preview UI probe requires an explicit --media-directory")?;
    prepare_artwork_fixture(&media_root)?;
    let Some(fixture) =
        crate::catalog::requested_path("--preview-video-fixture", "LUNCHBOX_PREVIEW_VIDEO_FIXTURE")
    else {
        return Ok(None);
    };
    let metadata = fixture
        .metadata()
        .with_context(|| format!("inspecting preview fixture {}", fixture.display()))?;
    if !metadata.is_file() || metadata.len() == 0 {
        bail!("preview fixture must be a non-empty regular file");
    }
    if metadata.len() > 64 * 1024 * 1024 {
        bail!("preview fixture must be at most 64 MiB");
    }
    let extension = fixture
        .extension()
        .and_then(|value| value.to_str())
        .map(str::to_ascii_lowercase)
        .context("preview fixture requires a supported video extension")?;
    if !matches!(
        extension.as_str(),
        "mp4" | "webm" | "mkv" | "mov" | "m4v" | "avi"
    ) {
        bail!("preview fixture has an unsupported video extension");
    }

    // The amalgamated catalog can legitimately assign a different numeric
    // provider ID after a database rebuild. Seed by Lunchbox's stable game
    // identity so this probe exercises the same title across catalog versions.
    let target_directory = media_root
        .join("game-9697a5eb-e0b4-4f24-8d43-672701414ee7")
        .join("ui-probe");
    fs::create_dir_all(&target_directory).with_context(|| {
        format!(
            "creating hover-preview probe directory {}",
            target_directory.display()
        )
    })?;
    let target = target_directory.join(format!("video.{extension}"));
    publish_probe_fixture(&fixture, &target)?;
    Ok(Some(target))
}

fn prepare_artwork_fixture(media_root: &Path) -> Result<Option<PathBuf>> {
    let Some(fixture) = crate::catalog::requested_path(
        "--preview-artwork-fixture",
        "LUNCHBOX_PREVIEW_ARTWORK_FIXTURE",
    ) else {
        return Ok(None);
    };
    let metadata = fixture
        .metadata()
        .with_context(|| format!("inspecting artwork fixture {}", fixture.display()))?;
    if !metadata.is_file() || metadata.len() == 0 {
        bail!("artwork fixture must be a non-empty regular file");
    }
    if metadata.len() > 16 * 1024 * 1024 {
        bail!("artwork fixture must be at most 16 MiB");
    }
    let extension = fixture
        .extension()
        .and_then(|value| value.to_str())
        .map(str::to_ascii_lowercase)
        .context("artwork fixture requires a supported image extension")?;
    if !matches!(extension.as_str(), "png" | "jpg" | "jpeg" | "webp") {
        bail!("artwork fixture has an unsupported image extension");
    }

    // This probe's catalog fixture resolves Super Mario Bros. to database ID
    // 140. Publish a deterministic local cover so the QML transition gate can
    // prove that the same decoded texture survives expansion and collapse.
    let target_directory = media_root.join("lb-140").join("local");
    fs::create_dir_all(&target_directory).with_context(|| {
        format!(
            "creating hover artwork probe directory {}",
            target_directory.display()
        )
    })?;
    let target = target_directory.join(format!("box-front.{extension}"));
    publish_probe_fixture(&fixture, &target)?;
    Ok(Some(target))
}

fn publish_probe_fixture(source: &Path, target: &Path) -> Result<()> {
    if target.is_file()
        && fs::metadata(source)?.len() == fs::metadata(target)?.len()
        && fs::read(source)? == fs::read(target)?
    {
        return Ok(());
    }
    let parent = target
        .parent()
        .context("preview probe target has no parent")?;
    let temporary = parent.join(format!(
        ".{}.tmp",
        target
            .file_name()
            .and_then(|value| value.to_str())
            .unwrap_or("video")
    ));
    fs::copy(source, &temporary).with_context(|| {
        format!(
            "copying hover-preview fixture {} to {}",
            source.display(),
            temporary.display()
        )
    })?;
    if target.is_file() {
        fs::remove_file(target).with_context(|| {
            format!(
                "removing the previous hover-preview fixture {}",
                target.display()
            )
        })?;
    }
    fs::rename(&temporary, target)
        .with_context(|| format!("publishing hover-preview fixture {}", target.display()))?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn probe_fixture_is_idempotent_and_replaces_changed_content() {
        let directory = tempfile::tempdir().unwrap();
        let source = directory.path().join("fixture.webm");
        let target = directory.path().join("cache/video.webm");
        fs::create_dir_all(target.parent().unwrap()).unwrap();
        fs::write(&source, b"first-video").unwrap();

        publish_probe_fixture(&source, &target).unwrap();
        publish_probe_fixture(&source, &target).unwrap();
        assert_eq!(fs::read(&target).unwrap(), b"first-video");

        fs::write(&source, b"replacement-video").unwrap();
        publish_probe_fixture(&source, &target).unwrap();
        assert_eq!(fs::read(&target).unwrap(), b"replacement-video");
        assert!(!target.with_file_name(".video.webm.tmp").exists());
    }
}
