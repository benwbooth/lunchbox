use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

use anyhow::{Context, Result, bail};
use serde::Serialize;

use crate::audit;
use crate::ids::sha256_file;

#[derive(Debug, Serialize)]
pub struct CompressionResult {
    pub database: PathBuf,
    pub database_bytes: u64,
    pub database_sha256: String,
    pub archive: PathBuf,
    pub archive_bytes: u64,
    pub archive_sha256: String,
    pub maximum_bytes: u64,
    pub archive_tested: bool,
    pub checksum_manifest: PathBuf,
}

pub fn compress(database: &Path, output: &Path, maximum_bytes: u64) -> Result<CompressionResult> {
    let report = audit::audit_path(database)?;
    if !report.valid {
        bail!("refusing to package a database that fails strict audit");
    }
    if maximum_bytes == 0 {
        bail!("maximum archive size must be greater than zero");
    }

    let database_parent = database.parent().unwrap_or_else(|| Path::new("."));
    let database_name = database
        .file_name()
        .context("database path has no file name")?;
    let output_parent = output.parent().unwrap_or_else(|| Path::new("."));
    fs::create_dir_all(output_parent)?;
    let output_parent = fs::canonicalize(output_parent)?;
    let output_name = output
        .file_name()
        .context("archive path has no file name")?;
    let absolute_output = output_parent.join(output_name);

    remove_exact_file_if_present(&absolute_output)?;
    let status = Command::new(seven_zip_command())
        .current_dir(database_parent)
        .args([
            "a",
            "-t7z",
            "-mx=9",
            "-m0=lzma2",
            "-md=256m",
            "-mfb=273",
            "-ms=on",
            "-mmt=on",
            "-mtc=off",
            "-mtm=off",
            "-mta=off",
        ])
        .arg(&absolute_output)
        .arg(database_name)
        .status()
        .context("running 7z maximum compression")?;
    if !status.success() {
        bail!("7z compression failed with status {status}");
    }

    let archive_bytes = fs::metadata(&absolute_output)?.len();
    if archive_bytes >= maximum_bytes {
        bail!(
            "compressed database is {archive_bytes} bytes; it must be smaller than {maximum_bytes} bytes"
        );
    }

    let test_status = Command::new(seven_zip_command())
        .args(["t", "-bso0", "-bsp0"])
        .arg(&absolute_output)
        .status()
        .context("testing generated 7z archive")?;
    if !test_status.success() {
        bail!("7z archive verification failed with status {test_status}");
    }

    let archive_hash = sha256_file(&absolute_output)?;
    let manifest = PathBuf::from(format!("{}.sha256", absolute_output.display()));
    let manifest_contents = format!(
        "{}  {}\n",
        archive_hash,
        absolute_output
            .file_name()
            .and_then(|name| name.to_str())
            .context("archive filename is not valid UTF-8")?
    );
    fs::write(&manifest, manifest_contents)?;

    Ok(CompressionResult {
        database: database.to_path_buf(),
        database_bytes: fs::metadata(database)?.len(),
        database_sha256: sha256_file(database)?,
        archive: absolute_output,
        archive_bytes,
        archive_sha256: archive_hash,
        maximum_bytes,
        archive_tested: true,
        checksum_manifest: manifest,
    })
}

fn seven_zip_command() -> &'static str {
    option_env!("LUNCHBOX_7Z").unwrap_or("7z")
}

fn remove_exact_file_if_present(path: &Path) -> Result<()> {
    if path.exists() {
        if !path.is_file() {
            bail!(
                "refusing to remove non-file archive path: {}",
                path.display()
            );
        }
        fs::remove_file(path)?;
    }
    Ok(())
}
