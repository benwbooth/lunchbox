use std::fs;
use std::io::{Read, Write};
use std::path::{Path, PathBuf};

use anyhow::{Context, Result, bail};

use crate::media::ArtworkKind;

const MAX_IMAGE_BYTES: u64 = 16 * 1024 * 1024;

pub(crate) fn copy_and_publish(
    source: &Path,
    media_root: &Path,
    database_id: i64,
    provider: &str,
    kind: ArtworkKind,
    candidate_id: &str,
) -> Result<PathBuf> {
    if database_id <= 0 {
        bail!("catalog database ID must be positive");
    }
    validate_provider(provider)?;
    let metadata = fs::metadata(source)
        .with_context(|| format!("reading artwork metadata from {}", source.display()))?;
    if !metadata.is_file() {
        bail!("selected artwork is not a regular file");
    }
    if metadata.len() > MAX_IMAGE_BYTES {
        bail!("artwork exceeds the 16 MiB safety limit");
    }
    let mut bytes = Vec::with_capacity(usize::try_from(metadata.len()).unwrap_or_default());
    fs::File::open(source)
        .with_context(|| format!("opening artwork {}", source.display()))?
        .take(MAX_IMAGE_BYTES + 1)
        .read_to_end(&mut bytes)
        .with_context(|| format!("reading artwork {}", source.display()))?;
    publish_validated_bytes(
        &bytes,
        media_root,
        database_id,
        provider,
        kind,
        candidate_id,
    )
}

pub(crate) fn download_and_publish(
    agent: &ureq::Agent,
    url: &str,
    media_root: &Path,
    database_id: i64,
    provider: &str,
    kind: ArtworkKind,
    candidate_id: &str,
) -> Result<PathBuf> {
    if database_id <= 0 {
        bail!("catalog database ID must be positive");
    }
    validate_provider(provider)?;
    if !approved_url(url) {
        bail!("artwork provider returned an unsupported image URL");
    }

    let mut response = agent
        .get(url)
        .header("User-Agent", "Lunchbox/0.1 reviewed artwork client")
        .call()
        .context("downloading reviewed provider artwork")?;
    let status = response.status().as_u16();
    if !(200..300).contains(&status) {
        bail!("artwork download failed with HTTP {status}");
    }
    if response
        .headers()
        .get(ureq::http::header::CONTENT_LENGTH)
        .and_then(|value| value.to_str().ok())
        .and_then(|value| value.parse::<u64>().ok())
        .is_some_and(|length| length > MAX_IMAGE_BYTES)
    {
        bail!("artwork exceeds the 16 MiB safety limit");
    }

    let mut bytes = Vec::new();
    response
        .body_mut()
        .as_reader()
        .take(MAX_IMAGE_BYTES + 1)
        .read_to_end(&mut bytes)
        .context("reading reviewed provider artwork")?;
    if bytes.len() as u64 > MAX_IMAGE_BYTES {
        bail!("artwork exceeds the 16 MiB safety limit");
    }
    publish_validated_bytes(
        &bytes,
        media_root,
        database_id,
        provider,
        kind,
        candidate_id,
    )
}

fn validate_provider(provider: &str) -> Result<()> {
    if provider.is_empty()
        || !provider
            .bytes()
            .all(|byte| byte.is_ascii_lowercase() || byte.is_ascii_digit() || byte == b'-')
    {
        bail!("media provider cache key is invalid");
    }
    Ok(())
}

fn publish_validated_bytes(
    bytes: &[u8],
    media_root: &Path,
    database_id: i64,
    provider: &str,
    kind: ArtworkKind,
    candidate_id: &str,
) -> Result<PathBuf> {
    if bytes.len() as u64 > MAX_IMAGE_BYTES {
        bail!("artwork exceeds the 16 MiB safety limit");
    }
    let extension = validated_image_extension(bytes)?;
    let directory = media_root.join(format!("lb-{database_id}")).join(provider);
    let stem = cache_file_stem(kind);
    let output = directory.join(format!("{stem}.{extension}"));
    fs::create_dir_all(&directory)
        .with_context(|| format!("creating media cache {}", directory.display()))?;
    let safe_candidate_id: String = candidate_id
        .bytes()
        .filter(|byte| byte.is_ascii_alphanumeric() || *byte == b'-' || *byte == b'_')
        .take(80)
        .map(char::from)
        .collect();
    let temporary = directory.join(format!(
        ".{stem}.{}.{}.download",
        std::process::id(),
        if safe_candidate_id.is_empty() {
            "candidate"
        } else {
            &safe_candidate_id
        }
    ));
    let mut file = fs::File::create(&temporary)
        .with_context(|| format!("creating temporary artwork {}", temporary.display()))?;
    file.write_all(&bytes)
        .with_context(|| format!("writing temporary artwork {}", temporary.display()))?;
    file.sync_all()
        .with_context(|| format!("syncing temporary artwork {}", temporary.display()))?;
    drop(file);
    replace_atomically(&temporary, &output)?;
    for prior_extension in ["png", "jpg", "webp"] {
        let prior = directory.join(format!("{stem}.{prior_extension}"));
        if prior != output {
            fs::remove_file(prior).ok();
        }
    }
    Ok(output)
}

pub(crate) fn approved_url(url: &str) -> bool {
    let Ok(uri) = url.parse::<ureq::http::Uri>() else {
        return false;
    };
    matches!((uri.scheme_str(), uri.host()), (Some("https"), Some(_))) || loopback_http_url(url)
}

pub(crate) fn loopback_http_url(url: &str) -> bool {
    let Ok(uri) = url.parse::<ureq::http::Uri>() else {
        return false;
    };
    matches!(
        (uri.scheme_str(), uri.host()),
        (Some("http"), Some("127.0.0.1" | "localhost"))
    )
}

pub(crate) fn validated_image_extension(bytes: &[u8]) -> Result<&'static str> {
    if bytes.starts_with(b"\x89PNG\r\n\x1a\n") {
        Ok("png")
    } else if bytes.starts_with(&[0xff, 0xd8, 0xff]) {
        Ok("jpg")
    } else if bytes.len() >= 12 && &bytes[..4] == b"RIFF" && &bytes[8..12] == b"WEBP" {
        Ok("webp")
    } else {
        bail!("artwork is not a supported PNG, JPEG, or WebP image")
    }
}

fn cache_file_stem(kind: ArtworkKind) -> &'static str {
    match kind {
        ArtworkKind::BoxFront => "box-front",
        ArtworkKind::BoxBack => "box-back",
        ArtworkKind::Box3d => "box-3d",
        ArtworkKind::Screenshot => "screenshot-gameplay",
        ArtworkKind::TitleScreen => "screenshot-game-title",
        ArtworkKind::Fanart => "fanart-background",
        ArtworkKind::ClearLogo => "clear-logo",
        ArtworkKind::CartFront => "cart-front",
        ArtworkKind::CartBack => "cart-back",
        ArtworkKind::Cart3d => "cart-3d",
        ArtworkKind::Disc => "disc",
        ArtworkKind::Marquee => "marquee",
        ArtworkKind::Banner => "banner",
        ArtworkKind::Flyer => "advertisement-flyer",
        ArtworkKind::GameOverScreen => "screenshot-game-over",
    }
}

fn replace_atomically(temporary: &Path, output: &Path) -> Result<()> {
    if output.is_file() {
        let backup = output.with_extension(format!(
            "{}.previous",
            output
                .extension()
                .and_then(|value| value.to_str())
                .unwrap_or("image")
        ));
        fs::remove_file(&backup).ok();
        fs::rename(output, &backup)
            .with_context(|| format!("preparing to replace artwork {}", output.display()))?;
        match fs::rename(temporary, output) {
            Ok(()) => {
                fs::remove_file(backup).ok();
                Ok(())
            }
            Err(error) => {
                let _ = fs::rename(&backup, output);
                Err(error).with_context(|| format!("publishing artwork {}", output.display()))
            }
        }
    } else {
        fs::rename(temporary, output)
            .with_context(|| format!("publishing artwork {}", output.display()))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn published_extensions_follow_validated_signatures() {
        assert_eq!(
            validated_image_extension(b"\x89PNG\r\n\x1a\nfixture").unwrap(),
            "png"
        );
        assert_eq!(
            validated_image_extension(b"\xff\xd8\xfffixture").unwrap(),
            "jpg"
        );
        assert_eq!(
            validated_image_extension(b"RIFF\x04\x00\x00\x00WEBPfixture").unwrap(),
            "webp"
        );
        assert!(validated_image_extension(b"GIF89a").is_err());
    }

    #[test]
    fn provider_and_transport_paths_are_bounded() {
        assert!(approved_url("https://images.example/art.jpg"));
        assert!(approved_url("http://127.0.0.1:1234/art.jpg"));
        assert!(approved_url("http://localhost:1234/art.jpg"));
        assert!(!approved_url("http://images.example/art.jpg"));
        assert!(!approved_url("http://localhost:80@images.example/art.jpg"));
        assert!(!approved_url("file:///tmp/art.jpg"));
    }

    #[test]
    fn reviewed_local_artwork_uses_the_same_validation_and_owned_cache() {
        let source_directory = tempfile::tempdir().unwrap();
        let media_directory = tempfile::tempdir().unwrap();
        let source = source_directory.path().join("misleading.gif");
        fs::write(&source, b"\x89PNG\r\n\x1a\nfixture").unwrap();

        let output = copy_and_publish(
            &source,
            media_directory.path(),
            42,
            "websearch",
            ArtworkKind::BoxFront,
            "local-fixture",
        )
        .unwrap();

        assert_eq!(
            output,
            media_directory.path().join("lb-42/websearch/box-front.png")
        );
        assert_eq!(fs::read(output).unwrap(), b"\x89PNG\r\n\x1a\nfixture");
        fs::write(&source, b"GIF89a").unwrap();
        assert!(
            copy_and_publish(
                &source,
                media_directory.path(),
                42,
                "websearch",
                ArtworkKind::BoxFront,
                "bad-fixture",
            )
            .is_err()
        );
    }
}
