use std::collections::BTreeMap;
use std::ffi::OsStr;
use std::fs::{self, File};
use std::io::{self, Read, Write};
use std::path::{Component, Path, PathBuf};

use anyhow::{Context, Result, bail};
use flate2::read::GzDecoder;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

use crate::ids::sha256_file;

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct LibretroSourceManifest {
    pub schema_version: u32,
    pub provider: String,
    pub repository: String,
    pub revision: String,
    pub archive_url: String,
    pub archive_bytes: u64,
    pub archive_sha256: String,
    pub rdb_file_count: usize,
    pub rdb_bytes: u64,
    pub rdb_manifest_sha256: String,
    pub data_license: String,
    pub license_url: String,
    #[serde(default)]
    pub platform_overrides: BTreeMap<String, String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct PreparedLibretroSource {
    pub revision: String,
    pub archive: PathBuf,
    pub archive_bytes: u64,
    pub archive_sha256: String,
    pub rdb_directory: PathBuf,
    pub rdb_file_count: usize,
    pub rdb_bytes: u64,
    pub rdb_manifest_sha256: String,
    pub reused: bool,
}

#[derive(Debug, Clone)]
pub struct SourceTreeSummary {
    pub file_count: usize,
    pub bytes: u64,
    pub manifest_sha256: String,
}

pub fn load_libretro_manifest(path: &Path) -> Result<LibretroSourceManifest> {
    let bytes = fs::read(path)
        .with_context(|| format!("reading Libretro source manifest {}", path.display()))?;
    let manifest: LibretroSourceManifest = serde_json::from_slice(&bytes)
        .with_context(|| format!("parsing Libretro source manifest {}", path.display()))?;
    validate_manifest(&manifest)?;
    Ok(manifest)
}

pub fn prepare_libretro(manifest_path: &Path, cache: &Path) -> Result<PreparedLibretroSource> {
    let manifest = load_libretro_manifest(manifest_path)?;
    fs::create_dir_all(cache)
        .with_context(|| format!("creating source cache {}", cache.display()))?;

    let archive = cache.join(format!("libretro-database-{}.tar.gz", manifest.revision));
    let archive_reused = archive_matches(&archive, &manifest)?;
    if !archive_reused {
        download_archive(&manifest, &archive)?;
    }

    let destination = cache.join(format!("libretro-database-{}", manifest.revision));
    let rdb_directory = destination.join("rdb");
    let tree_reused = tree_matches(&rdb_directory, &manifest).unwrap_or(false);
    if !tree_reused {
        extract_rdb_files(&archive, &destination, &manifest)?;
    }

    let summary = summarize_rdb_tree(&rdb_directory)?;
    require_expected_tree(&summary, &manifest)?;
    Ok(PreparedLibretroSource {
        revision: manifest.revision,
        archive: archive.clone(),
        archive_bytes: fs::metadata(&archive)?.len(),
        archive_sha256: sha256_file(&archive)?,
        rdb_directory,
        rdb_file_count: summary.file_count,
        rdb_bytes: summary.bytes,
        rdb_manifest_sha256: summary.manifest_sha256,
        reused: archive_reused && tree_reused,
    })
}

pub fn verify_prepared_libretro(
    manifest: &LibretroSourceManifest,
    rdb_directory: &Path,
) -> Result<SourceTreeSummary> {
    let summary = summarize_rdb_tree(rdb_directory)?;
    require_expected_tree(&summary, manifest)?;
    Ok(summary)
}

fn validate_manifest(manifest: &LibretroSourceManifest) -> Result<()> {
    if manifest.schema_version != 1 {
        bail!(
            "unsupported Libretro source manifest version {}",
            manifest.schema_version
        );
    }
    if manifest.provider != "libretro-database" {
        bail!("Libretro manifest has unexpected provider slug");
    }
    if manifest.revision.len() != 40
        || !manifest
            .revision
            .bytes()
            .all(|value| value.is_ascii_hexdigit())
    {
        bail!("Libretro revision must be a full 40-character Git commit");
    }
    if manifest.archive_sha256.len() != 64
        || manifest.rdb_manifest_sha256.len() != 64
        || !manifest
            .archive_sha256
            .bytes()
            .chain(manifest.rdb_manifest_sha256.bytes())
            .all(|value| value.is_ascii_hexdigit())
        || manifest.archive_bytes == 0
        || manifest.rdb_file_count == 0
        || manifest.rdb_bytes == 0
    {
        bail!("Libretro manifest is missing complete integrity metadata");
    }
    if !manifest.archive_url.starts_with("https://") || !manifest.repository.starts_with("https://")
    {
        bail!("Libretro manifest source URLs must use HTTPS");
    }
    Ok(())
}

fn archive_matches(path: &Path, manifest: &LibretroSourceManifest) -> Result<bool> {
    if !path.exists() {
        return Ok(false);
    }
    if !path.is_file() || fs::metadata(path)?.len() != manifest.archive_bytes {
        return Ok(false);
    }
    Ok(sha256_file(path)? == manifest.archive_sha256)
}

fn download_archive(manifest: &LibretroSourceManifest, destination: &Path) -> Result<()> {
    if destination.exists() {
        fs::remove_file(destination).with_context(|| {
            format!("removing invalid cached archive {}", destination.display())
        })?;
    }
    let temporary = destination.with_extension("tar.gz.part");
    if temporary.exists() {
        fs::remove_file(&temporary)
            .with_context(|| format!("removing interrupted download {}", temporary.display()))?;
    }

    let mut response = ureq::get(&manifest.archive_url)
        .header("User-Agent", "lunchbox-db/0.1")
        .call()
        .with_context(|| format!("downloading {}", manifest.archive_url))?;
    let mut output = File::create(&temporary)
        .with_context(|| format!("creating download {}", temporary.display()))?;
    let maximum_download_bytes = manifest
        .archive_bytes
        .checked_add(1)
        .context("Libretro archive size limit overflow")?;
    let mut reader = response.body_mut().as_reader().take(maximum_download_bytes);
    io::copy(&mut reader, &mut output).context("streaming Libretro source archive")?;
    output.flush()?;
    output.sync_all()?;

    if !archive_matches(&temporary, manifest)? {
        bail!("downloaded Libretro archive does not match the pinned size and SHA-256");
    }
    fs::rename(&temporary, destination).with_context(|| {
        format!(
            "publishing verified archive {} to {}",
            temporary.display(),
            destination.display()
        )
    })?;
    Ok(())
}

fn extract_rdb_files(
    archive_path: &Path,
    destination: &Path,
    manifest: &LibretroSourceManifest,
) -> Result<()> {
    let parent = destination.parent().unwrap_or_else(|| Path::new("."));
    let temporary = parent.join(format!(".libretro-{}.building", manifest.revision));
    remove_generated_directory(&temporary, parent)?;
    fs::create_dir_all(temporary.join("rdb"))?;

    let file = File::open(archive_path)
        .with_context(|| format!("opening source archive {}", archive_path.display()))?;
    let decoder = GzDecoder::new(file);
    let mut archive = tar::Archive::new(decoder);
    for entry in archive.entries().context("reading Libretro tar entries")? {
        let mut entry = entry?;
        if !entry.header().entry_type().is_file() {
            continue;
        }
        let entry_path = entry.path()?.into_owned();
        let components: Vec<_> = entry_path.components().collect();
        if components.len() != 3
            || !matches!(components[0], Component::Normal(_))
            || components[1] != Component::Normal(OsStr::new("rdb"))
        {
            continue;
        }
        let Component::Normal(file_name) = components[2] else {
            continue;
        };
        if Path::new(file_name).extension() != Some(OsStr::new("rdb")) {
            continue;
        }
        let output_path = temporary.join("rdb").join(file_name);
        let mut output = File::create(&output_path)
            .with_context(|| format!("creating extracted file {}", output_path.display()))?;
        io::copy(&mut entry, &mut output)
            .with_context(|| format!("extracting {}", entry_path.display()))?;
    }

    let summary = summarize_rdb_tree(&temporary.join("rdb"))?;
    require_expected_tree(&summary, manifest)?;
    remove_generated_directory(destination, parent)?;
    fs::rename(&temporary, destination).with_context(|| {
        format!(
            "publishing verified source tree {} to {}",
            temporary.display(),
            destination.display()
        )
    })?;
    Ok(())
}

fn tree_matches(path: &Path, manifest: &LibretroSourceManifest) -> Result<bool> {
    if !path.is_dir() {
        return Ok(false);
    }
    let summary = summarize_rdb_tree(path)?;
    Ok(summary.file_count == manifest.rdb_file_count
        && summary.bytes == manifest.rdb_bytes
        && summary.manifest_sha256 == manifest.rdb_manifest_sha256)
}

fn require_expected_tree(
    summary: &SourceTreeSummary,
    manifest: &LibretroSourceManifest,
) -> Result<()> {
    if summary.file_count != manifest.rdb_file_count
        || summary.bytes != manifest.rdb_bytes
        || summary.manifest_sha256 != manifest.rdb_manifest_sha256
    {
        bail!(
            "Libretro RDB tree failed verification: files={} bytes={} manifest_sha256={}",
            summary.file_count,
            summary.bytes,
            summary.manifest_sha256
        );
    }
    Ok(())
}

pub fn summarize_rdb_tree(path: &Path) -> Result<SourceTreeSummary> {
    let mut files = Vec::new();
    for entry in
        fs::read_dir(path).with_context(|| format!("reading RDB directory {}", path.display()))?
    {
        let entry = entry?;
        let file_type = entry.file_type()?;
        if file_type.is_file() && entry.path().extension() == Some(OsStr::new("rdb")) {
            files.push(entry.path());
        } else {
            bail!(
                "unexpected entry in verified RDB directory: {}",
                entry.path().display()
            );
        }
    }
    files.sort_by(|left, right| left.file_name().cmp(&right.file_name()));

    let mut total_bytes = 0_u64;
    let mut manifest_bytes = Vec::new();
    for file in &files {
        let metadata = fs::metadata(file)?;
        total_bytes = total_bytes
            .checked_add(metadata.len())
            .context("Libretro RDB byte count overflow")?;
        let name = file
            .file_name()
            .and_then(|value| value.to_str())
            .context("Libretro RDB filename is not valid UTF-8")?;
        writeln!(manifest_bytes, "{}  rdb/{name}", sha256_file(file)?)?;
    }

    Ok(SourceTreeSummary {
        file_count: files.len(),
        bytes: total_bytes,
        manifest_sha256: hex::encode(Sha256::digest(&manifest_bytes)),
    })
}

fn remove_generated_directory(path: &Path, expected_parent: &Path) -> Result<()> {
    if !path.exists() {
        return Ok(());
    }
    if path.parent() != Some(expected_parent) || !path.is_dir() {
        bail!(
            "refusing to replace unexpected cache path {}",
            path.display()
        );
    }
    fs::remove_dir_all(path)
        .with_context(|| format!("replacing generated cache directory {}", path.display()))?;
    Ok(())
}
