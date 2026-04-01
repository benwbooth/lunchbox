//! Minerva Archive torrent index builder
//!
//! Downloads the minerva-archive.org hashes.db and extracts the unique
//! torrent-to-platform mapping. Links minerva platforms to lunchbox platforms
//! via fuzzy name matching. The result is a small minerva.db that maps
//! lunchbox platforms to minerva torrent URLs.
//!
//! At download time, the app fetches the platform torrent and uses librqbit's
//! list_only mode to scan for the specific ROM file.

use anyhow::{Context, Result};
use indicatif::{ProgressBar, ProgressStyle};
use sqlx::sqlite::{SqliteConnectOptions, SqlitePool, SqlitePoolOptions};
use std::collections::HashMap;
use std::io::Write;
use std::path::Path;

const MINERVA_HASHES_URL: &str = "https://minerva-archive.org/assets/hashes.db";
const MINERVA_ASSETS_BASE: &str = "https://minerva-archive.org/assets";

// ============================================================================
// Schema
// ============================================================================

async fn create_minerva_db(pool: &SqlitePool) -> Result<()> {
    sqlx::query(
        "CREATE TABLE IF NOT EXISTS minerva_torrents (
            id INTEGER PRIMARY KEY,
            torrent_file TEXT NOT NULL UNIQUE,
            torrent_url TEXT NOT NULL,
            collection TEXT,
            rom_count INTEGER DEFAULT 0,
            total_size INTEGER DEFAULT 0
        )",
    )
    .execute(pool)
    .await?;

    sqlx::query(
        "CREATE TABLE IF NOT EXISTS minerva_torrent_platforms (
            torrent_id INTEGER NOT NULL REFERENCES minerva_torrents(id),
            minerva_platform TEXT NOT NULL,
            lunchbox_platform_id INTEGER,
            lunchbox_platform_name TEXT,
            rom_count INTEGER DEFAULT 0,
            PRIMARY KEY (torrent_id, minerva_platform)
        )",
    )
    .execute(pool)
    .await?;

    sqlx::query("CREATE INDEX IF NOT EXISTS idx_mtp_lunchbox ON minerva_torrent_platforms(lunchbox_platform_id)")
        .execute(pool)
        .await?;
    sqlx::query("CREATE INDEX IF NOT EXISTS idx_mtp_torrent ON minerva_torrent_platforms(torrent_id)")
        .execute(pool)
        .await?;

    Ok(())
}

// ============================================================================
// Platform name mapping
// ============================================================================

fn map_platform_name(minerva_name: &str, platform_lookup: &HashMap<String, (i64, String)>) -> Option<String> {
    let aliases = crate::unified_import::get_platform_aliases();
    let lower = minerva_name.to_lowercase();

    // 1. Direct alias lookup on full name
    if let Some(canonical) = aliases.get(lower.as_str()) {
        return Some(canonical.to_string());
    }

    // 2. Direct match against lunchbox platform names (case-insensitive)
    if platform_lookup.contains_key(&lower) {
        return Some(platform_lookup[&lower].1.clone());
    }

    // 3. Strip manufacturer prefix: "Nintendo - Game Boy Advance" -> "Game Boy Advance"
    let without_prefix = if let Some(pos) = minerva_name.find(" - ") {
        Some(&minerva_name[pos + 3..])
    } else {
        None
    };

    if let Some(name) = without_prefix {
        let name_lower = name.to_lowercase();
        if let Some(canonical) = aliases.get(name_lower.as_str()) {
            return Some(canonical.to_string());
        }
        if platform_lookup.contains_key(&name_lower) {
            return Some(platform_lookup[&name_lower].1.clone());
        }
    }

    // 4. Strip parenthetical suffixes: "Nintendo Entertainment System (Headered)" -> "Nintendo Entertainment System"
    let stripped = strip_parens(minerva_name);
    if stripped != minerva_name {
        let stripped_lower = stripped.to_lowercase();
        if let Some(canonical) = aliases.get(stripped_lower.as_str()) {
            return Some(canonical.to_string());
        }
        if platform_lookup.contains_key(&stripped_lower) {
            return Some(platform_lookup[&stripped_lower].1.clone());
        }
        // Also try stripping prefix + parens
        if let Some(pos) = stripped.find(" - ") {
            let inner = &stripped[pos + 3..];
            let inner_lower = inner.to_lowercase();
            if let Some(canonical) = aliases.get(inner_lower.as_str()) {
                return Some(canonical.to_string());
            }
            if platform_lookup.contains_key(&inner_lower) {
                return Some(platform_lookup[&inner_lower].1.clone());
            }
        }
    }

    // 5. Try "Manufacturer Platform" format (join with space instead of " - ")
    if let Some(name) = without_prefix {
        let pos = minerva_name.find(" - ").unwrap();
        let manufacturer = &minerva_name[..pos];
        let joined = format!("{} {}", manufacturer, strip_parens(name));
        let joined_lower = joined.to_lowercase();
        if let Some(canonical) = aliases.get(joined_lower.as_str()) {
            return Some(canonical.to_string());
        }
        if platform_lookup.contains_key(&joined_lower) {
            return Some(platform_lookup[&joined_lower].1.clone());
        }
    }

    None
}

fn strip_parens(s: &str) -> &str {
    match s.find('(') {
        Some(pos) => s[..pos].trim(),
        None => s,
    }
}

// ============================================================================
// Download hashes.db
// ============================================================================

async fn download_hashes_db(output: &Path) -> Result<()> {
    println!("Downloading hashes.db from minerva-archive.org...");

    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(3600))
        .user_agent("Mozilla/5.0 (X11; Linux x86_64; rv:128.0) Gecko/20100101 Firefox/128.0")
        .build()?;

    let response = client
        .get(MINERVA_HASHES_URL)
        .send()
        .await
        .context("failed to fetch hashes.db")?;

    let total_size = response.content_length().unwrap_or(0);
    let pb = ProgressBar::new(total_size);
    pb.set_style(
        ProgressStyle::default_bar()
            .template("{spinner:.green} [{bar:40.cyan/blue}] {bytes}/{total_bytes} ({eta})")
            .unwrap()
            .progress_chars("#>-"),
    );

    let mut file = std::fs::File::create(output)?;
    let mut downloaded: u64 = 0;

    use futures_util::StreamExt;
    let mut stream = response.bytes_stream();
    while let Some(chunk) = stream.next().await {
        let chunk = chunk.context("error reading response stream")?;
        file.write_all(&chunk)?;
        downloaded += chunk.len() as u64;
        pb.set_position(downloaded);
    }

    pb.finish_with_message("download complete");
    println!("Downloaded {:.1} MB", downloaded as f64 / (1024.0 * 1024.0));
    Ok(())
}

// ============================================================================
// Main build
// ============================================================================

pub async fn cmd_minerva_build(
    output: &Path,
    games_db_path: Option<&Path>,
    collections_filter: Option<&str>,
    hashes_db_path: Option<&Path>,
    csv_output: Option<&Path>,
) -> Result<()> {
    // Get or download hashes.db
    let hashes_path = if let Some(path) = hashes_db_path {
        if !path.exists() {
            anyhow::bail!("hashes.db not found at {}", path.display());
        }
        path.to_path_buf()
    } else {
        let default_path = std::env::temp_dir().join("minerva-hashes.db");
        if !default_path.exists() {
            download_hashes_db(&default_path).await?;
        } else {
            println!("Using cached hashes.db at {}", default_path.display());
        }
        default_path
    };

    // Open hashes.db
    let hashes_pool = SqlitePoolOptions::new()
        .max_connections(1)
        .connect_with(
            SqliteConnectOptions::new()
                .filename(&hashes_path)
                .read_only(true),
        )
        .await
        .context("failed to open hashes.db")?;

    // Open games.db for platform matching
    let games_db = if let Some(path) = games_db_path {
        if path.exists() {
            Some(
                SqlitePoolOptions::new()
                    .max_connections(1)
                    .connect_with(
                        SqliteConnectOptions::new()
                            .filename(path)
                            .read_only(true),
                    )
                    .await
                    .context("failed to open games database")?,
            )
        } else {
            println!("Games database not found at {}, skipping platform linking", path.display());
            None
        }
    } else {
        println!("No --games-db specified, skipping platform linking");
        None
    };

    // Build platform lookup from games.db
    let platform_lookup: HashMap<String, (i64, String)> = if let Some(ref gdb) = games_db {
        let rows: Vec<(i64, String)> = sqlx::query_as("SELECT id, name FROM platforms")
            .fetch_all(gdb)
            .await?;
        let mut lookup = HashMap::new();
        for (id, name) in &rows {
            lookup.insert(name.to_lowercase(), (*id, name.clone()));
        }
        lookup
    } else {
        HashMap::new()
    };

    // Filter collections
    let filter_names: Option<Vec<String>> = collections_filter.map(|f| {
        f.split(',').map(|s| s.trim().to_string()).collect()
    });

    // Extract unique (collection, platform, torrent_file) tuples with stats
    println!("Extracting torrent-to-platform mapping...");
    let rows: Vec<(String, String, i64, i64)> = sqlx::query_as(
        "SELECT
            full_path,
            COALESCE(torrents, '') as torrent_file,
            CAST(size AS INTEGER) as file_size,
            1 as cnt
         FROM files
         WHERE torrents IS NOT NULL AND torrents != ''"
    )
    .fetch_all(&hashes_pool)
    .await?;

    // Aggregate by (torrent_file, collection, platform)
    struct PlatformStats {
        collection: String,
        platform: String,
        torrent_file: String,
        rom_count: i64,
        total_size: i64,
    }

    let mut torrent_platforms: HashMap<(String, String), PlatformStats> = HashMap::new();

    let pb = ProgressBar::new(rows.len() as u64);
    pb.set_style(
        ProgressStyle::default_bar()
            .template("{prefix:.bold} [{bar:40.green/black}] {pos}/{len} ({per_sec})")
            .unwrap()
            .progress_chars("##-"),
    );
    pb.set_prefix("Scanning");

    for (full_path, torrent_file, file_size, _) in &rows {
        pb.inc(1);

        let path = full_path.strip_prefix("./").unwrap_or(full_path);
        let mut parts = path.splitn(3, '/');
        let collection = match parts.next() {
            Some(c) if !c.is_empty() => c,
            _ => continue,
        };
        let platform = match parts.next() {
            Some(p) if !p.is_empty() => p,
            _ => continue,
        };

        // Apply collection filter
        if let Some(ref filters) = filter_names {
            if !filters.iter().any(|f| f.eq_ignore_ascii_case(collection)) {
                continue;
            }
        }

        let key = (torrent_file.clone(), platform.to_string());
        let entry = torrent_platforms.entry(key).or_insert_with(|| PlatformStats {
            collection: collection.to_string(),
            platform: platform.to_string(),
            torrent_file: torrent_file.clone(),
            rom_count: 0,
            total_size: 0,
        });
        entry.rom_count += 1;
        entry.total_size += file_size;
    }

    pb.finish_and_clear();

    // Create output database
    let output_abs = output.canonicalize().unwrap_or_else(|_| {
        let parent = output.parent().unwrap_or(Path::new("."));
        let parent = parent.canonicalize().unwrap_or_else(|_| std::env::current_dir().unwrap());
        parent.join(output.file_name().unwrap())
    });
    if output_abs.exists() {
        std::fs::remove_file(&output_abs)?;
    }
    let out_pool = SqlitePoolOptions::new()
        .max_connections(1)
        .connect_with(
            SqliteConnectOptions::new()
                .filename(&output_abs)
                .create_if_missing(true),
        )
        .await
        .context("failed to create output database")?;

    sqlx::query("PRAGMA journal_mode=WAL").execute(&out_pool).await?;
    create_minerva_db(&out_pool).await?;

    // Group by torrent file to get torrent-level stats
    let mut torrent_stats: HashMap<String, (String, i64, i64)> = HashMap::new(); // torrent -> (collection, total_roms, total_size)
    for stats in torrent_platforms.values() {
        let entry = torrent_stats
            .entry(stats.torrent_file.clone())
            .or_insert_with(|| (stats.collection.clone(), 0, 0));
        entry.1 += stats.rom_count;
        entry.2 += stats.total_size;
    }

    // Insert torrents
    let mut torrent_id_map: HashMap<String, i64> = HashMap::new();
    for (torrent_file, (collection, rom_count, total_size)) in &torrent_stats {
        let torrent_url = format!("{MINERVA_ASSETS_BASE}/{torrent_file}");

        sqlx::query(
            "INSERT INTO minerva_torrents (torrent_file, torrent_url, collection, rom_count, total_size) VALUES (?, ?, ?, ?, ?)"
        )
        .bind(torrent_file)
        .bind(&torrent_url)
        .bind(collection)
        .bind(rom_count)
        .bind(total_size)
        .execute(&out_pool)
        .await?;

        let (id,): (i64,) = sqlx::query_as("SELECT last_insert_rowid()")
            .fetch_one(&out_pool)
            .await?;
        torrent_id_map.insert(torrent_file.clone(), id);
    }

    // Insert platform mappings
    let mut matched = 0i64;
    let mut unmatched = 0i64;

    // CSV output
    let mut csv_writer = if let Some(csv_path) = csv_output {
        let file = std::fs::File::create(csv_path)?;
        let mut wtr = csv::Writer::from_writer(file);
        wtr.write_record([
            "collection", "minerva_platform", "torrent_file", "torrent_url",
            "rom_count", "total_size", "lunchbox_platform_id", "lunchbox_platform_name",
        ])?;
        Some(wtr)
    } else {
        None
    };

    for stats in torrent_platforms.values() {
        let torrent_id = match torrent_id_map.get(&stats.torrent_file) {
            Some(&id) => id,
            None => continue,
        };

        // Match to lunchbox platform
        let (lunchbox_id, lunchbox_name) = if let Some(canonical) = map_platform_name(&stats.platform, &platform_lookup) {
            if let Some((id, name)) = platform_lookup.get(&canonical.to_lowercase()) {
                matched += 1;
                (Some(*id), Some(name.clone()))
            } else {
                unmatched += 1;
                (None, None)
            }
        } else {
            unmatched += 1;
            (None, None)
        };

        sqlx::query(
            "INSERT OR IGNORE INTO minerva_torrent_platforms (torrent_id, minerva_platform, lunchbox_platform_id, lunchbox_platform_name, rom_count) VALUES (?, ?, ?, ?, ?)"
        )
        .bind(torrent_id)
        .bind(&stats.platform)
        .bind(lunchbox_id)
        .bind(&lunchbox_name)
        .bind(stats.rom_count)
        .execute(&out_pool)
        .await?;

        let torrent_url = format!("{MINERVA_ASSETS_BASE}/{}", stats.torrent_file);
        if let Some(ref mut wtr) = csv_writer {
            wtr.write_record([
                &stats.collection,
                &stats.platform,
                &stats.torrent_file,
                &torrent_url,
                &stats.rom_count.to_string(),
                &stats.total_size.to_string(),
                &lunchbox_id.map(|id| id.to_string()).unwrap_or_default(),
                lunchbox_name.as_deref().unwrap_or(""),
            ])?;
        }
    }

    if let Some(ref mut wtr) = csv_writer {
        wtr.flush()?;
    }

    out_pool.close().await;

    // Summary
    let total_torrents = torrent_stats.len();
    let total_platforms = torrent_platforms.len();
    println!("\nMinerva torrent index built:");
    println!("  Torrents:   {total_torrents}");
    println!("  Platforms:  {total_platforms}");
    println!("  Matched:    {matched} ({:.1}%)", if total_platforms > 0 { matched as f64 / total_platforms as f64 * 100.0 } else { 0.0 });
    println!("  Unmatched:  {unmatched}");
    println!("  Output:     {}", output_abs.display());
    if let Some(csv_path) = csv_output {
        println!("  CSV:        {}", csv_path.display());
    }

    Ok(())
}
