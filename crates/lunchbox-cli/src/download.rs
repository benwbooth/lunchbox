//! Download metadata sources for database building
//!
//! Sources:
//! - LaunchBox Metadata.xml (~100MB compressed)
//! - LibRetro database (~200MB git repo)
//! - OpenVGDB (~50MB)

use anyhow::{Context, Result};
use futures_util::StreamExt;
use indicatif::{ProgressBar, ProgressStyle};
use std::fs::{self, File};
use std::io::{BufReader, Write};
use std::path::{Path, PathBuf};
use std::process::Command;

const LAUNCHBOX_METADATA_URL: &str = "https://gamesdb.launchbox-app.com/Metadata.zip";
const LIBRETRO_DATABASE_URL: &str = "https://github.com/libretro/libretro-database";
const OPENVGDB_URL: &str = "https://github.com/OpenVGDB/OpenVGDB/releases/latest/download/openvgdb.zip";

/// Get the default data directory
pub fn default_data_dir() -> PathBuf {
    if let Some(dirs) = directories::ProjectDirs::from("", "", "lunchbox") {
        dirs.data_dir().to_path_buf()
    } else {
        PathBuf::from("data")
    }
}

/// Download a file with progress bar
async fn download_file(url: &str, dest: &Path) -> Result<()> {
    let client = reqwest::Client::new();
    let response = client
        .get(url)
        .send()
        .await
        .context("Failed to send request")?
        .error_for_status()
        .context("HTTP error")?;

    let total_size = response.content_length().unwrap_or(0);

    let pb = ProgressBar::new(total_size);
    pb.set_style(
        ProgressStyle::default_bar()
            .template("{spinner:.green} [{elapsed_precise}] [{bar:40.cyan/blue}] {bytes}/{total_bytes} ({bytes_per_sec})")
            .unwrap()
            .progress_chars("#>-"),
    );

    let mut file = File::create(dest).context("Failed to create file")?;
    let mut stream = response.bytes_stream();

    while let Some(chunk_result) = stream.next().await {
        let chunk = chunk_result.context("Error downloading chunk")?;
        file.write_all(&chunk)?;
        pb.inc(chunk.len() as u64);
    }

    pb.finish();
    Ok(())
}

/// Download and extract LaunchBox Metadata.xml
pub async fn download_launchbox(data_dir: &Path) -> Result<PathBuf> {
    let launchbox_dir = data_dir.join("launchbox-metadata");
    let metadata_xml = launchbox_dir.join("Metadata.xml");

    if metadata_xml.exists() {
        println!("  LaunchBox metadata already exists");
        return Ok(metadata_xml);
    }

    fs::create_dir_all(&launchbox_dir)?;

    let zip_path = data_dir.join("launchbox-metadata.zip");

    println!("  Downloading LaunchBox metadata (~100MB)...");
    download_file(LAUNCHBOX_METADATA_URL, &zip_path).await?;

    println!("  Extracting...");
    let file = File::open(&zip_path)?;
    let mut archive = zip::ZipArchive::new(BufReader::new(file))?;

    for i in 0..archive.len() {
        let mut file = archive.by_index(i)?;
        let outpath = launchbox_dir.join(file.name());

        if file.is_dir() {
            fs::create_dir_all(&outpath)?;
        } else {
            if let Some(parent) = outpath.parent() {
                fs::create_dir_all(parent)?;
            }
            let mut outfile = File::create(&outpath)?;
            std::io::copy(&mut file, &mut outfile)?;
        }
    }

    fs::remove_file(&zip_path)?;
    println!("  LaunchBox metadata ready");

    Ok(metadata_xml)
}

/// Clone or update libretro-database
pub async fn download_libretro(data_dir: &Path) -> Result<PathBuf> {
    let libretro_dir = data_dir.join("libretro-database");

    if libretro_dir.join(".git").exists() {
        println!("  Updating libretro-database...");
        let status = Command::new("git")
            .args(["pull", "--quiet"])
            .current_dir(&libretro_dir)
            .status()
            .context("Failed to run git pull")?;

        if !status.success() {
            println!("  Warning: git pull failed, using existing data");
        } else {
            println!("  libretro-database updated");
        }
    } else {
        println!("  Cloning libretro-database (~200MB)...");
        fs::create_dir_all(data_dir)?;

        let status = Command::new("git")
            .args([
                "clone",
                "--depth",
                "1",
                LIBRETRO_DATABASE_URL,
                libretro_dir.to_str().unwrap(),
            ])
            .status()
            .context("Failed to run git clone")?;

        if !status.success() {
            anyhow::bail!("git clone failed");
        }
        println!("  libretro-database ready");
    }

    Ok(libretro_dir)
}

/// Download and extract OpenVGDB
pub async fn download_openvgdb(data_dir: &Path) -> Result<PathBuf> {
    let openvgdb_path = data_dir.join("openvgdb.sqlite");

    if openvgdb_path.exists() {
        println!("  OpenVGDB already exists");
        return Ok(openvgdb_path);
    }

    let zip_path = data_dir.join("openvgdb.zip");

    println!("  Downloading OpenVGDB...");
    download_file(OPENVGDB_URL, &zip_path).await?;

    println!("  Extracting...");
    let file = File::open(&zip_path)?;
    let mut archive = zip::ZipArchive::new(BufReader::new(file))?;

    for i in 0..archive.len() {
        let mut file = archive.by_index(i)?;
        let name = file.name().to_lowercase();

        // Look for the sqlite file
        if name.ends_with(".sqlite") {
            let mut outfile = File::create(&openvgdb_path)?;
            std::io::copy(&mut file, &mut outfile)?;
            break;
        }
    }

    fs::remove_file(&zip_path)?;
    println!("  OpenVGDB ready");

    Ok(openvgdb_path)
}

/// Download all sources
pub async fn download_all(data_dir: &Path) -> Result<DownloadedSources> {
    println!("Downloading metadata sources to: {}", data_dir.display());
    println!();

    fs::create_dir_all(data_dir)?;

    println!("LaunchBox:");
    let launchbox_xml = download_launchbox(data_dir).await?;

    println!();
    println!("LibRetro:");
    let libretro_path = download_libretro(data_dir).await?;

    println!();
    println!("OpenVGDB:");
    let openvgdb_path = download_openvgdb(data_dir).await?;

    Ok(DownloadedSources {
        launchbox_xml,
        libretro_path,
        openvgdb_path,
    })
}

/// Paths to downloaded sources
pub struct DownloadedSources {
    pub launchbox_xml: PathBuf,
    pub libretro_path: PathBuf,
    pub openvgdb_path: PathBuf,
}
