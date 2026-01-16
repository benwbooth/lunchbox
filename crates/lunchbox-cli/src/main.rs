//! Lunchbox CLI - Command line interface for importing and scraping

mod download;
mod enrich;
mod launchbox;
mod unified_import;

use anyhow::{Context, Result};
use clap::{Parser, Subcommand};
use indicatif::{ProgressBar, ProgressStyle};
use std::io::{self, Write};
use std::path::PathBuf;

use lunchbox_core::import::{parse_dat_file, merge_dat_files, DatFile, LaunchBoxImporter};
use lunchbox_core::scanner::{Checksums, RomScanner};
use lunchbox_core::scraper::{
    get_screenscraper_platform_id, IGDBClient, IGDBConfig, ScreenScraperClient,
    ScreenScraperConfig, SteamGridDBClient, SteamGridDBConfig,
};

#[derive(Parser)]
#[command(name = "lunchbox-cli")]
#[command(author, version, about = "Lunchbox emulator frontend CLI", long_about = None)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Import games and platforms from LaunchBox
    Import {
        /// Path to LaunchBox installation directory
        #[arg(short, long)]
        path: PathBuf,

        /// Only show what would be imported (dry run)
        #[arg(long)]
        dry_run: bool,
    },

    /// Scan ROM directories for files
    Scan {
        /// Directories to scan for ROMs
        #[arg(required = true)]
        paths: Vec<PathBuf>,

        /// Calculate checksums for each ROM
        #[arg(long)]
        checksums: bool,
    },

    /// Scrape metadata from ScreenScraper
    Scrape {
        /// ROM file or directory to scrape
        #[arg(required = true)]
        path: PathBuf,

        /// Platform name (e.g., "Nintendo Entertainment System", "Sega Genesis")
        #[arg(short, long)]
        platform: String,

        /// ScreenScraper developer ID
        #[arg(long, env = "SCREENSCRAPER_DEV_ID")]
        dev_id: String,

        /// ScreenScraper developer password
        #[arg(long, env = "SCREENSCRAPER_DEV_PASSWORD")]
        dev_password: String,

        /// ScreenScraper user ID (optional, for higher rate limits)
        #[arg(long, env = "SCREENSCRAPER_USER_ID")]
        user_id: Option<String>,

        /// ScreenScraper user password
        #[arg(long, env = "SCREENSCRAPER_USER_PASSWORD")]
        user_password: Option<String>,
    },

    /// Show LaunchBox database statistics
    Stats {
        /// Path to LaunchBox installation directory
        #[arg(short, long)]
        path: PathBuf,
    },

    /// Interactive setup for metadata services
    Setup {
        /// Service to configure: screenscraper, steamgriddb, igdb, emumovies, or all
        #[arg(default_value = "all")]
        service: String,
    },

    /// Test connection to a metadata service
    Test {
        /// Service to test: screenscraper, steamgriddb, igdb, emumovies
        service: String,

        /// ScreenScraper developer ID
        #[arg(long, env = "SCREENSCRAPER_DEV_ID")]
        dev_id: Option<String>,

        /// ScreenScraper developer password
        #[arg(long, env = "SCREENSCRAPER_DEV_PASSWORD")]
        dev_password: Option<String>,

        /// User ID (ScreenScraper username or EmuMovies username)
        #[arg(long, env = "SCREENSCRAPER_USER_ID")]
        user_id: Option<String>,

        /// User password
        #[arg(long, env = "SCREENSCRAPER_USER_PASSWORD")]
        user_password: Option<String>,

        /// SteamGridDB API key
        #[arg(long, env = "STEAMGRIDDB_API_KEY")]
        api_key: Option<String>,

        /// IGDB/Twitch Client ID
        #[arg(long, env = "IGDB_CLIENT_ID")]
        client_id: Option<String>,

        /// IGDB/Twitch Client Secret
        #[arg(long, env = "IGDB_CLIENT_SECRET")]
        client_secret: Option<String>,
    },

    /// Build game database from LibRetro DAT files
    BuildDb {
        /// Path to local libretro-database clone
        #[arg(short, long)]
        libretro_path: PathBuf,

        /// Output SQLite database path
        #[arg(short, long, default_value = "lunchbox-games.db")]
        output: PathBuf,

        /// Only process specific platforms (comma-separated)
        #[arg(long)]
        platforms: Option<String>,
    },

    /// Enrich game database with metadata from OpenVGDB
    EnrichDb {
        /// Path to games database to enrich
        #[arg(short, long)]
        database: PathBuf,

        /// Path to OpenVGDB database
        #[arg(short, long)]
        openvgdb: PathBuf,

        /// Similarity threshold for fuzzy matching (0.0-1.0)
        #[arg(long, default_value = "0.85")]
        threshold: f64,

        /// Only analyze, don't update
        #[arg(long)]
        dry_run: bool,
    },

    /// Enrich game database with metadata from LaunchBox
    EnrichLaunchbox {
        /// Path to games database to enrich
        #[arg(short, long)]
        database: PathBuf,

        /// Path to LaunchBox Metadata.xml file
        #[arg(short, long)]
        metadata_xml: PathBuf,

        /// Similarity threshold for fuzzy matching (0.0-1.0)
        #[arg(long, default_value = "0.85")]
        threshold: f64,

        /// Only analyze, don't update
        #[arg(long)]
        dry_run: bool,
    },

    /// Build unified game database from multiple sources (LaunchBox + LibRetro + OpenVGDB)
    /// Import order: LaunchBox first (best metadata), then LibRetro (checksums), then OpenVGDB
    UnifiedBuild {
        /// Output SQLite database path (defaults to ~/.local/share/lunchbox/games.db)
        #[arg(short, long)]
        output: Option<PathBuf>,

        /// Path to LaunchBox Metadata.xml file (primary source, imported first)
        #[arg(long)]
        launchbox_xml: Option<PathBuf>,

        /// Path to libretro-database clone (secondary source)
        #[arg(long)]
        libretro_path: Option<PathBuf>,

        /// Path to OpenVGDB database (tertiary source)
        #[arg(long)]
        openvgdb: Option<PathBuf>,

        /// Similarity threshold for fuzzy matching (0.0-1.0)
        #[arg(long, default_value = "0.85")]
        threshold: f64,

        /// Download sources automatically if not provided
        #[arg(long)]
        download: bool,

        /// Directory for downloaded sources (used with --download)
        #[arg(long)]
        data_dir: Option<PathBuf>,
    },

    /// Download metadata sources (LaunchBox, LibRetro, OpenVGDB)
    Download {
        /// Directory to download sources to
        #[arg(short, long)]
        output: Option<PathBuf>,

        /// Only download LaunchBox
        #[arg(long)]
        launchbox_only: bool,

        /// Only download LibRetro
        #[arg(long)]
        libretro_only: bool,

        /// Only download OpenVGDB
        #[arg(long)]
        openvgdb_only: bool,
    },
}

#[tokio::main]
async fn main() -> Result<()> {
    // Initialize logging
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::from_default_env()
                .add_directive("lunchbox=info".parse().unwrap()),
        )
        .init();

    let cli = Cli::parse();

    match cli.command {
        Commands::Import { path, dry_run } => {
            cmd_import(&path, dry_run).await?;
        }
        Commands::Scan { paths, checksums } => {
            cmd_scan(&paths, checksums)?;
        }
        Commands::Scrape {
            path,
            platform,
            dev_id,
            dev_password,
            user_id,
            user_password,
        } => {
            cmd_scrape(&path, &platform, &dev_id, &dev_password, user_id, user_password).await?;
        }
        Commands::Stats { path } => {
            cmd_stats(&path).await?;
        }
        Commands::Setup { service } => {
            cmd_setup(&service).await?;
        }
        Commands::Test {
            service,
            dev_id,
            dev_password,
            user_id,
            user_password,
            api_key,
            client_id,
            client_secret,
        } => {
            cmd_test(&service, dev_id, dev_password, user_id, user_password, api_key, client_id, client_secret).await?;
        }
        Commands::BuildDb {
            libretro_path,
            output,
            platforms,
        } => {
            cmd_build_db(&libretro_path, &output, platforms).await?;
        }
        Commands::EnrichDb {
            database,
            openvgdb,
            threshold,
            dry_run,
        } => {
            cmd_enrich_db(&database, &openvgdb, threshold, dry_run).await?;
        }
        Commands::EnrichLaunchbox {
            database,
            metadata_xml,
            threshold,
            dry_run,
        } => {
            launchbox::enrich_from_launchbox(&database, &metadata_xml, threshold, dry_run).await?;
        }
        Commands::UnifiedBuild {
            output,
            launchbox_xml,
            libretro_path,
            openvgdb,
            threshold,
            download,
            data_dir,
        } => {
            cmd_unified_build(output, launchbox_xml, libretro_path, openvgdb, threshold, download, data_dir).await?;
        }
        Commands::Download {
            output,
            launchbox_only,
            libretro_only,
            openvgdb_only,
        } => {
            cmd_download(output, launchbox_only, libretro_only, openvgdb_only).await?;
        }
    }

    Ok(())
}

async fn cmd_import(launchbox_path: &PathBuf, dry_run: bool) -> Result<()> {
    let metadata_path = launchbox_path.join("Metadata").join("LaunchBox.Metadata.db");

    if !metadata_path.exists() {
        anyhow::bail!(
            "LaunchBox metadata database not found at: {}",
            metadata_path.display()
        );
    }

    println!("Connecting to LaunchBox database...");
    let importer = LaunchBoxImporter::connect(&metadata_path)
        .await
        .context("Failed to connect to LaunchBox database")?;

    let platform_count = importer.count_platforms().await?;
    let game_count = importer.count_games().await?;

    println!("\nLaunchBox Library:");
    println!("  Platforms: {}", platform_count);
    println!("  Games:     {}", game_count);

    if dry_run {
        println!("\n[Dry run] Would import {} platforms and {} games", platform_count, game_count);
        return Ok(());
    }

    println!("\nFetching platforms...");
    let platforms = importer.get_platforms().await?;

    let pb = ProgressBar::new(platforms.len() as u64);
    pb.set_style(
        ProgressStyle::default_bar()
            .template("{spinner:.green} [{elapsed_precise}] [{bar:40.cyan/blue}] {pos}/{len} {msg}")
            .unwrap()
            .progress_chars("#>-"),
    );

    for platform in &platforms {
        pb.set_message(platform.name.clone());
        pb.inc(1);
    }
    pb.finish_with_message("Done");

    println!("\nImported {} platforms", platforms.len());
    println!("\nTo import into Lunchbox app, configure the LaunchBox path in Settings.");

    Ok(())
}

fn cmd_scan(paths: &[PathBuf], calculate_checksums: bool) -> Result<()> {
    let scanner = RomScanner::new();

    println!("Scanning {} directories...\n", paths.len());

    let mut total_files = 0;
    let mut total_size: u64 = 0;

    for path in paths {
        if !path.exists() {
            println!("Warning: Path does not exist: {}", path.display());
            continue;
        }

        println!("Scanning: {}", path.display());
        let roms = scanner.scan_directories(&[path.clone()]);

        for rom in &roms {
            total_size += rom.size;

            if calculate_checksums {
                let rom_path = PathBuf::from(&rom.path);
                print!("  {} ", rom.file_name);

                match Checksums::calculate(&rom_path) {
                    Ok(checksums) => {
                        println!(
                            "\n    CRC32: {}\n    MD5:   {}\n    SHA1:  {}",
                            checksums.crc32, checksums.md5, checksums.sha1
                        );
                    }
                    Err(e) => {
                        println!("(checksum error: {})", e);
                    }
                }
            } else {
                println!(
                    "  {} ({}) - {}",
                    rom.file_name,
                    format_size(rom.size),
                    rom.clean_name
                );
            }
        }

        total_files += roms.len();
        println!();
    }

    println!("Total: {} files ({})", total_files, format_size(total_size));

    Ok(())
}

async fn cmd_scrape(
    path: &PathBuf,
    platform: &str,
    dev_id: &str,
    dev_password: &str,
    user_id: Option<String>,
    user_password: Option<String>,
) -> Result<()> {
    let config = ScreenScraperConfig {
        dev_id: dev_id.to_string(),
        dev_password: dev_password.to_string(),
        user_id,
        user_password,
    };

    let client = ScreenScraperClient::new(config);

    if !client.has_credentials() {
        anyhow::bail!("ScreenScraper credentials are required. Set via --dev-id/--dev-password or environment variables.");
    }

    let platform_id = get_screenscraper_platform_id(platform);
    if platform_id.is_none() {
        println!("Warning: Unknown platform '{}', scraping without platform filter", platform);
    }

    let files: Vec<PathBuf> = if path.is_dir() {
        let scanner = RomScanner::new();
        scanner
            .scan_directories(&[path.clone()])
            .into_iter()
            .map(|r| PathBuf::from(r.path))
            .collect()
    } else {
        vec![path.clone()]
    };

    println!("Scraping {} file(s) for platform: {}\n", files.len(), platform);

    let pb = ProgressBar::new(files.len() as u64);
    pb.set_style(
        ProgressStyle::default_bar()
            .template("{spinner:.green} [{elapsed_precise}] [{bar:40.cyan/blue}] {pos}/{len} {msg}")
            .unwrap()
            .progress_chars("#>-"),
    );

    let mut found = 0;
    let mut not_found = 0;
    let mut errors = 0;

    for file_path in &files {
        let file_name = file_path
            .file_name()
            .and_then(|s| s.to_str())
            .unwrap_or("unknown");

        pb.set_message(file_name.to_string());

        // Calculate checksums
        let checksums = match Checksums::calculate(file_path) {
            Ok(c) => c,
            Err(e) => {
                pb.println(format!("  Error calculating checksums for {}: {}", file_name, e));
                errors += 1;
                pb.inc(1);
                continue;
            }
        };

        // Query ScreenScraper
        match client
            .lookup_by_checksum(
                &checksums.crc32,
                &checksums.md5,
                &checksums.sha1,
                checksums.size,
                file_name,
                platform_id,
            )
            .await
        {
            Ok(Some(game)) => {
                pb.println(format!(
                    "  {} -> {} ({})",
                    file_name,
                    game.name,
                    game.release_date.as_deref().unwrap_or("unknown date")
                ));
                found += 1;
            }
            Ok(None) => {
                pb.println(format!("  {} -> Not found", file_name));
                not_found += 1;
            }
            Err(e) => {
                pb.println(format!("  {} -> Error: {}", file_name, e));
                errors += 1;
            }
        }

        pb.inc(1);

        // Rate limiting - ScreenScraper has limits
        tokio::time::sleep(tokio::time::Duration::from_millis(500)).await;
    }

    pb.finish_and_clear();

    println!("\nResults:");
    println!("  Found:     {}", found);
    println!("  Not found: {}", not_found);
    println!("  Errors:    {}", errors);

    Ok(())
}

async fn cmd_stats(launchbox_path: &PathBuf) -> Result<()> {
    let metadata_path = launchbox_path.join("Metadata").join("LaunchBox.Metadata.db");

    if !metadata_path.exists() {
        anyhow::bail!(
            "LaunchBox metadata database not found at: {}",
            metadata_path.display()
        );
    }

    let importer = LaunchBoxImporter::connect(&metadata_path)
        .await
        .context("Failed to connect to LaunchBox database")?;

    let platform_count = importer.count_platforms().await?;
    let game_count = importer.count_games().await?;

    println!("LaunchBox Statistics");
    println!("====================");
    println!("Database: {}", metadata_path.display());
    println!();
    println!("Platforms: {}", platform_count);
    println!("Games:     {}", game_count);
    println!();

    // List platforms with game counts
    println!("Platforms:");
    let platforms = importer.get_platforms().await?;
    for platform in platforms {
        let games = importer.get_games_by_platform(&platform.name).await?;
        println!("  {:40} {:>6} games", platform.name, games.len());
    }

    Ok(())
}

fn format_size(bytes: u64) -> String {
    const KB: u64 = 1024;
    const MB: u64 = KB * 1024;
    const GB: u64 = MB * 1024;

    if bytes >= GB {
        format!("{:.2} GB", bytes as f64 / GB as f64)
    } else if bytes >= MB {
        format!("{:.2} MB", bytes as f64 / MB as f64)
    } else if bytes >= KB {
        format!("{:.2} KB", bytes as f64 / KB as f64)
    } else {
        format!("{} B", bytes)
    }
}

async fn cmd_setup(service: &str) -> Result<()> {
    match service.to_lowercase().as_str() {
        "screenscraper" => setup_screenscraper().await?,
        "steamgriddb" => setup_steamgriddb().await?,
        "igdb" => setup_igdb().await?,
        "emumovies" => setup_emumovies().await?,
        "all" => {
            setup_screenscraper().await?;
            println!();
            setup_steamgriddb().await?;
            println!();
            setup_igdb().await?;
            println!();
            setup_emumovies().await?;
        }
        _ => {
            println!("Unknown service: {}", service);
            println!("Available services: screenscraper, steamgriddb, igdb, emumovies, all");
        }
    }
    Ok(())
}

async fn setup_screenscraper() -> Result<()> {
    println!("===========================================");
    println!("  ScreenScraper Setup");
    println!("===========================================");
    println!();
    println!("ScreenScraper (https://www.screenscraper.fr) provides game metadata,");
    println!("box art, screenshots, and videos for ROMs based on file checksums.");
    println!();
    println!("To use ScreenScraper, you need:");
    println!("  1. A developer account (for API access)");
    println!("  2. Optionally, a user account (for higher rate limits)");
    println!();
    println!("STEP 1: Get Developer Credentials");
    println!("----------------------------------");
    println!("Developer credentials are required for API access.");
    println!("You can request them by:");
    println!("  - Registering at: https://www.screenscraper.fr");
    println!("  - Going to your profile and requesting API access");
    println!("  - Or contact the ScreenScraper team");
    println!();

    print!("Do you have developer credentials? [y/N]: ");
    io::stdout().flush()?;
    let mut input = String::new();
    io::stdin().read_line(&mut input)?;

    if input.trim().to_lowercase() != "y" {
        println!();
        println!("Please visit https://www.screenscraper.fr to register and request API access.");
        println!("Once you have credentials, run this setup again or configure in the app Settings.");
        return Ok(());
    }

    println!();
    print!("Enter your Developer ID: ");
    io::stdout().flush()?;
    let mut dev_id = String::new();
    io::stdin().read_line(&mut dev_id)?;
    let dev_id = dev_id.trim().to_string();

    print!("Enter your Developer Password: ");
    io::stdout().flush()?;
    let mut dev_password = String::new();
    io::stdin().read_line(&mut dev_password)?;
    let dev_password = dev_password.trim().to_string();

    println!();
    println!("STEP 2: User Account (Optional)");
    println!("--------------------------------");
    println!("A user account gives you higher rate limits.");
    println!();
    print!("Do you have a ScreenScraper user account? [y/N]: ");
    io::stdout().flush()?;
    let mut input = String::new();
    io::stdin().read_line(&mut input)?;

    let (user_id, user_password) = if input.trim().to_lowercase() == "y" {
        print!("Enter your Username: ");
        io::stdout().flush()?;
        let mut user = String::new();
        io::stdin().read_line(&mut user)?;

        print!("Enter your Password: ");
        io::stdout().flush()?;
        let mut pass = String::new();
        io::stdin().read_line(&mut pass)?;

        (Some(user.trim().to_string()), Some(pass.trim().to_string()))
    } else {
        (None, None)
    };

    println!();
    println!("Testing connection...");

    let config = ScreenScraperConfig {
        dev_id: dev_id.clone(),
        dev_password: dev_password.clone(),
        user_id: user_id.clone(),
        user_password: user_password.clone(),
    };

    let client = ScreenScraperClient::new(config);

    match client.lookup_by_checksum("3337EC46", "", "", 40976, "test.nes", Some(3)).await {
        Ok(_) => {
            println!("Connection successful!");
            println!();
            println!("To use these credentials:");
            println!();
            println!("  Option 1: Set environment variables:");
            println!("    export SCREENSCRAPER_DEV_ID=\"{}\"", dev_id);
            println!("    export SCREENSCRAPER_DEV_PASSWORD=\"{}\"", dev_password);
            if let Some(ref u) = user_id {
                println!("    export SCREENSCRAPER_USER_ID=\"{}\"", u);
            }
            if let Some(ref p) = user_password {
                println!("    export SCREENSCRAPER_USER_PASSWORD=\"{}\"", p);
            }
            println!();
            println!("  Option 2: Configure in the Lunchbox app Settings panel");
        }
        Err(e) => {
            println!("Connection failed: {}", e);
            println!();
            println!("Please check your credentials and try again.");
        }
    }

    Ok(())
}

async fn setup_steamgriddb() -> Result<()> {
    println!("===========================================");
    println!("  SteamGridDB Setup");
    println!("===========================================");
    println!();
    println!("SteamGridDB (https://www.steamgriddb.com) provides custom game artwork:");
    println!("grids, heroes, logos, and icons for your game library.");
    println!();
    println!("STEP 1: Get API Key");
    println!("-------------------");
    println!("  1. Create an account at https://www.steamgriddb.com");
    println!("  2. Go to Preferences > API");
    println!("  3. Copy your API key");
    println!();

    print!("Do you have a SteamGridDB API key? [y/N]: ");
    io::stdout().flush()?;
    let mut input = String::new();
    io::stdin().read_line(&mut input)?;

    if input.trim().to_lowercase() != "y" {
        println!();
        println!("Please visit https://www.steamgriddb.com to create an account and get an API key.");
        return Ok(());
    }

    println!();
    print!("Enter your API key: ");
    io::stdout().flush()?;
    let mut api_key = String::new();
    io::stdin().read_line(&mut api_key)?;
    let api_key = api_key.trim().to_string();

    println!();
    println!("Testing connection...");

    let config = SteamGridDBConfig { api_key: api_key.clone() };
    let client = SteamGridDBClient::new(config);

    match client.test_connection().await {
        Ok(()) => {
            println!("Connection successful!");
            println!();
            println!("To use this API key:");
            println!();
            println!("  Option 1: Set environment variable:");
            println!("    export STEAMGRIDDB_API_KEY=\"{}\"", api_key);
            println!();
            println!("  Option 2: Configure in the Lunchbox app Settings panel");
        }
        Err(e) => {
            println!("Connection failed: {}", e);
            println!();
            println!("Please check your API key and try again.");
        }
    }

    Ok(())
}

async fn setup_igdb() -> Result<()> {
    println!("===========================================");
    println!("  IGDB Setup (via Twitch)");
    println!("===========================================");
    println!();
    println!("IGDB (https://www.igdb.com) provides comprehensive game metadata,");
    println!("ratings, release dates, and cover art. It's owned by Twitch and");
    println!("requires Twitch developer credentials.");
    println!();
    println!("STEP 1: Create Twitch Application");
    println!("---------------------------------");
    println!("  1. Go to https://dev.twitch.tv/console");
    println!("  2. Log in with your Twitch account");
    println!("  3. Click 'Register Your Application'");
    println!("  4. Name: anything (e.g., 'Lunchbox')");
    println!("  5. OAuth Redirect URLs: http://localhost");
    println!("  6. Category: Application Integration");
    println!("  7. Copy the Client ID");
    println!("  8. Generate a Client Secret");
    println!();

    print!("Do you have Twitch Client ID and Secret? [y/N]: ");
    io::stdout().flush()?;
    let mut input = String::new();
    io::stdin().read_line(&mut input)?;

    if input.trim().to_lowercase() != "y" {
        println!();
        println!("Please visit https://dev.twitch.tv/console to create an application.");
        return Ok(());
    }

    println!();
    print!("Enter your Client ID: ");
    io::stdout().flush()?;
    let mut client_id = String::new();
    io::stdin().read_line(&mut client_id)?;
    let client_id = client_id.trim().to_string();

    print!("Enter your Client Secret: ");
    io::stdout().flush()?;
    let mut client_secret = String::new();
    io::stdin().read_line(&mut client_secret)?;
    let client_secret = client_secret.trim().to_string();

    println!();
    println!("Testing connection...");

    let config = IGDBConfig {
        client_id: client_id.clone(),
        client_secret: client_secret.clone(),
    };
    let client = IGDBClient::new(config);

    match client.test_connection().await {
        Ok(found) => {
            println!("Connection successful! ({})", found);
            println!();
            println!("To use these credentials:");
            println!();
            println!("  Option 1: Set environment variables:");
            println!("    export IGDB_CLIENT_ID=\"{}\"", client_id);
            println!("    export IGDB_CLIENT_SECRET=\"{}\"", client_secret);
            println!();
            println!("  Option 2: Configure in the Lunchbox app Settings panel");
        }
        Err(e) => {
            println!("Connection failed: {}", e);
            println!();
            println!("Please check your credentials and try again.");
        }
    }

    Ok(())
}

async fn setup_emumovies() -> Result<()> {
    println!("===========================================");
    println!("  EmuMovies Setup");
    println!("===========================================");
    println!();
    println!("EmuMovies (https://emumovies.com) provides video previews,");
    println!("box art, and other media for games via FTP.");
    println!();
    println!("To use EmuMovies, you need a premium membership for FTP access.");
    println!();
    println!("STEP 1: Get an Account");
    println!("----------------------");
    println!("  1. Visit https://emumovies.com");
    println!("  2. Register for an account");
    println!("  3. Purchase a premium membership for FTP access");
    println!();

    print!("Do you have EmuMovies premium credentials? [y/N]: ");
    io::stdout().flush()?;
    let mut input = String::new();
    io::stdin().read_line(&mut input)?;

    if input.trim().to_lowercase() != "y" {
        println!();
        println!("Please visit https://emumovies.com to register and get premium access.");
        return Ok(());
    }

    println!();
    print!("Enter your EmuMovies username: ");
    io::stdout().flush()?;
    let mut username = String::new();
    io::stdin().read_line(&mut username)?;
    let username = username.trim().to_string();

    print!("Enter your EmuMovies password: ");
    io::stdout().flush()?;
    let mut password = String::new();
    io::stdin().read_line(&mut password)?;
    let password = password.trim().to_string();

    println!();
    println!("Testing FTP connection to ftp.emumovies.com...");

    match test_emumovies_ftp(&username, &password).await {
        Ok(()) => {
            println!("Connection successful!");
            println!();
            println!("To use these credentials, set environment variables:");
            println!("  export EMUMOVIES_USER=\"{}\"", username);
            println!("  export EMUMOVIES_PASSWORD=\"{}\"", password);
        }
        Err(e) => {
            println!("Connection failed: {}", e);
            println!();
            println!("Please check your credentials and ensure you have premium access.");
        }
    }

    Ok(())
}

async fn test_emumovies_ftp(username: &str, password: &str) -> Result<()> {
    use std::net::TcpStream;
    use std::io::{BufRead, BufReader};

    // Connect to FTP server
    let stream = TcpStream::connect("ftp.emumovies.com:21")
        .context("Failed to connect to ftp.emumovies.com")?;
    stream.set_read_timeout(Some(std::time::Duration::from_secs(10)))?;

    let mut reader = BufReader::new(stream.try_clone()?);
    let mut writer = stream;

    // Read welcome message
    let mut response = String::new();
    reader.read_line(&mut response)?;
    if !response.starts_with("220") {
        anyhow::bail!("Unexpected server response: {}", response.trim());
    }

    // Send USER command
    use std::io::Write;
    write!(writer, "USER {}\r\n", username)?;
    writer.flush()?;

    response.clear();
    reader.read_line(&mut response)?;
    if !response.starts_with("331") {
        anyhow::bail!("Username rejected: {}", response.trim());
    }

    // Send PASS command
    write!(writer, "PASS {}\r\n", password)?;
    writer.flush()?;

    response.clear();
    reader.read_line(&mut response)?;
    if !response.starts_with("230") {
        anyhow::bail!("Login failed: {}", response.trim());
    }

    // Send QUIT
    write!(writer, "QUIT\r\n")?;
    writer.flush()?;

    Ok(())
}

async fn cmd_test(
    service: &str,
    dev_id: Option<String>,
    dev_password: Option<String>,
    user_id: Option<String>,
    user_password: Option<String>,
    api_key: Option<String>,
    client_id: Option<String>,
    client_secret: Option<String>,
) -> Result<()> {
    match service.to_lowercase().as_str() {
        "screenscraper" => {
            let dev_id = dev_id.ok_or_else(|| anyhow::anyhow!("--dev-id is required"))?;
            let dev_password = dev_password.ok_or_else(|| anyhow::anyhow!("--dev-password is required"))?;

            println!("Testing ScreenScraper connection...");

            let config = ScreenScraperConfig {
                dev_id,
                dev_password,
                user_id: user_id.clone(),
                user_password,
            };

            let client = ScreenScraperClient::new(config);

            match client.lookup_by_checksum("3337EC46", "", "", 40976, "test.nes", Some(3)).await {
                Ok(_) => {
                    println!("Connection successful!");
                    if let Some(user) = user_id {
                        println!("Logged in as: {}", user);
                    }
                }
                Err(e) => {
                    let err_str = e.to_string();
                    if err_str.contains("401") || err_str.contains("403") {
                        println!("Authentication failed. Please check your credentials.");
                    } else if err_str.contains("429") {
                        println!("Rate limited, but credentials appear valid.");
                    } else {
                        println!("Connection failed: {}", e);
                    }
                }
            }
        }
        "steamgriddb" => {
            let api_key = api_key.ok_or_else(|| anyhow::anyhow!("--api-key is required for SteamGridDB"))?;

            println!("Testing SteamGridDB connection...");

            let config = SteamGridDBConfig { api_key };
            let client = SteamGridDBClient::new(config);

            match client.test_connection().await {
                Ok(()) => {
                    println!("Connection successful!");
                }
                Err(e) => {
                    let err_str = e.to_string();
                    if err_str.contains("401") || err_str.contains("403") {
                        println!("Authentication failed. Please check your API key.");
                    } else {
                        println!("Connection failed: {}", e);
                    }
                }
            }
        }
        "igdb" => {
            let client_id = client_id.ok_or_else(|| anyhow::anyhow!("--client-id is required for IGDB"))?;
            let client_secret = client_secret.ok_or_else(|| anyhow::anyhow!("--client-secret is required for IGDB"))?;

            println!("Testing IGDB connection...");

            let config = IGDBConfig { client_id, client_secret };
            let client = IGDBClient::new(config);

            match client.test_connection().await {
                Ok(found) => {
                    println!("Connection successful!");
                    println!("Found: {}", found);
                }
                Err(e) => {
                    let err_str = e.to_string();
                    if err_str.contains("401") || err_str.contains("403") || err_str.contains("invalid") {
                        println!("Authentication failed. Please check your Twitch credentials.");
                    } else {
                        println!("Connection failed: {}", e);
                    }
                }
            }
        }
        "emumovies" => {
            let username = user_id.ok_or_else(|| anyhow::anyhow!("--user-id is required for EmuMovies"))?;
            let password = user_password.ok_or_else(|| anyhow::anyhow!("--user-password is required for EmuMovies"))?;

            println!("Testing EmuMovies FTP connection...");

            match test_emumovies_ftp(&username, &password).await {
                Ok(()) => {
                    println!("Connection successful!");
                    println!("Logged in as: {}", username);
                }
                Err(e) => {
                    println!("Connection failed: {}", e);
                }
            }
        }
        _ => {
            println!("Unknown service: {}", service);
            println!("Available services: screenscraper, steamgriddb, igdb, emumovies");
        }
    }

    Ok(())
}

async fn cmd_build_db(
    libretro_path: &PathBuf,
    output_path: &PathBuf,
    platform_filter: Option<String>,
) -> Result<()> {
    use sqlx::sqlite::{SqliteConnectOptions, SqlitePoolOptions};
    use std::str::FromStr;
    use uuid::Uuid;

    println!("Building game database from LibRetro DAT files");
    println!("LibRetro path: {}", libretro_path.display());
    println!("Output: {}", output_path.display());
    println!();

    // Check if libretro-database exists
    let metadat_path = libretro_path.join("metadat");
    if !metadat_path.exists() {
        anyhow::bail!(
            "LibRetro database not found at {}. Clone it with:\n  git clone https://github.com/libretro/libretro-database {}",
            libretro_path.display(),
            libretro_path.display()
        );
    }

    // Parse platform filter
    let platform_filter: Option<Vec<String>> = platform_filter.map(|s| {
        s.split(',')
            .map(|p| p.trim().to_string())
            .collect()
    });

    // Find all DAT files in no-intro directory (primary game list)
    let no_intro_path = metadat_path.join("no-intro");
    let redump_path = metadat_path.join("redump");

    let mut dat_files: Vec<PathBuf> = Vec::new();

    // Collect DAT files from no-intro (cartridge-based systems)
    if no_intro_path.exists() {
        for entry in walkdir::WalkDir::new(&no_intro_path)
            .max_depth(1)
            .into_iter()
            .filter_map(|e| e.ok())
        {
            let path = entry.path();
            if path.extension().map(|e| e == "dat").unwrap_or(false) {
                dat_files.push(path.to_path_buf());
            }
        }
    }

    // Collect DAT files from redump (disc-based systems)
    if redump_path.exists() {
        for entry in walkdir::WalkDir::new(&redump_path)
            .max_depth(1)
            .into_iter()
            .filter_map(|e| e.ok())
        {
            let path = entry.path();
            if path.extension().map(|e| e == "dat").unwrap_or(false) {
                dat_files.push(path.to_path_buf());
            }
        }
    }

    println!("Found {} DAT files", dat_files.len());

    // Filter platforms if specified
    if let Some(ref filter) = platform_filter {
        dat_files.retain(|p| {
            let name = p.file_stem().and_then(|s| s.to_str()).unwrap_or("");
            filter.iter().any(|f| name.to_lowercase().contains(&f.to_lowercase()))
        });
        println!("After filtering: {} DAT files", dat_files.len());
    }

    if dat_files.is_empty() {
        println!("No DAT files to process.");
        return Ok(());
    }

    // Supplementary metadata directories
    let developer_path = metadat_path.join("developer");
    let publisher_path = metadat_path.join("publisher");
    let genre_path = metadat_path.join("genre");
    let releaseyear_path = metadat_path.join("releaseyear");

    // Create output database
    if output_path.exists() {
        std::fs::remove_file(output_path)?;
    }

    let db_url = format!("sqlite:{}?mode=rwc", output_path.display());
    let options = SqliteConnectOptions::from_str(&db_url)?
        .create_if_missing(true);
    let pool = SqlitePoolOptions::new()
        .max_connections(1)
        .connect_with(options)
        .await?;

    // Create tables
    sqlx::query(
        r#"
        CREATE TABLE IF NOT EXISTS platforms (
            id INTEGER PRIMARY KEY,
            name TEXT NOT NULL UNIQUE,
            libretro_name TEXT,
            screenscraper_id INTEGER,
            retroarch_core TEXT,
            file_extensions TEXT,
            created_at TEXT DEFAULT CURRENT_TIMESTAMP
        )
        "#,
    )
    .execute(&pool)
    .await?;

    sqlx::query(
        r#"
        CREATE TABLE IF NOT EXISTS games (
            id TEXT PRIMARY KEY,
            title TEXT NOT NULL,
            platform_id INTEGER REFERENCES platforms(id),
            libretro_crc32 TEXT,
            libretro_serial TEXT,
            screenscraper_id INTEGER,
            igdb_id INTEGER,
            steamgriddb_id INTEGER,
            description TEXT,
            release_date TEXT,
            release_year INTEGER,
            developer TEXT,
            publisher TEXT,
            genre TEXT,
            players TEXT,
            rating REAL,
            rating_count INTEGER,
            esrb TEXT,
            cooperative INTEGER,
            video_url TEXT,
            wikipedia_url TEXT,
            metadata_fetched INTEGER DEFAULT 0,
            metadata_source TEXT,
            created_at TEXT DEFAULT CURRENT_TIMESTAMP,
            updated_at TEXT DEFAULT CURRENT_TIMESTAMP
        )
        "#,
    )
    .execute(&pool)
    .await?;

    // Create indexes for fast lookup
    sqlx::query("CREATE INDEX IF NOT EXISTS idx_games_platform ON games(platform_id)")
        .execute(&pool)
        .await?;
    sqlx::query("CREATE INDEX IF NOT EXISTS idx_games_crc32 ON games(libretro_crc32)")
        .execute(&pool)
        .await?;
    sqlx::query("CREATE INDEX IF NOT EXISTS idx_games_title ON games(title)")
        .execute(&pool)
        .await?;

    let pb = ProgressBar::new(dat_files.len() as u64);
    pb.set_style(
        ProgressStyle::default_bar()
            .template("{spinner:.green} [{elapsed_precise}] [{bar:40.cyan/blue}] {pos}/{len} {msg}")
            .unwrap()
            .progress_chars("#>-"),
    );

    let mut total_games = 0;
    let mut total_platforms = 0;

    for dat_path in &dat_files {
        let platform_name = dat_path
            .file_stem()
            .and_then(|s| s.to_str())
            .unwrap_or("Unknown");

        pb.set_message(platform_name.to_string());

        // Parse base DAT file
        let base_dat = match parse_dat_file(dat_path) {
            Ok(dat) => dat,
            Err(e) => {
                pb.println(format!("  Error parsing {}: {}", platform_name, e));
                pb.inc(1);
                continue;
            }
        };

        // Look for supplementary metadata files
        let mut supplements: Vec<DatFile> = Vec::new();

        for (supp_path, _meta_type) in [
            (&developer_path, "developer"),
            (&publisher_path, "publisher"),
            (&genre_path, "genre"),
            (&releaseyear_path, "releaseyear"),
        ] {
            let supp_file = supp_path.join(format!("{}.dat", platform_name));
            if supp_file.exists() {
                if let Ok(supp_dat) = parse_dat_file(&supp_file) {
                    supplements.push(supp_dat);
                }
            }
        }

        // Merge supplementary data
        let merged = if supplements.is_empty() {
            base_dat
        } else {
            merge_dat_files(base_dat, supplements)
        };

        // Insert platform
        let platform_id: i64 = sqlx::query_scalar(
            "INSERT INTO platforms (name, libretro_name) VALUES (?, ?) ON CONFLICT(name) DO UPDATE SET name=name RETURNING id",
        )
        .bind(platform_name)
        .bind(platform_name)
        .fetch_one(&pool)
        .await?;

        total_platforms += 1;

        // Insert games
        for game in &merged.games {
            let game_id = Uuid::new_v4().to_string();

            // Get primary ROM CRC for matching
            let primary_crc = game.roms.first().and_then(|r| r.crc.clone());

            sqlx::query(
                r#"
                INSERT INTO games (id, title, platform_id, libretro_crc32, release_year, developer, publisher, genre)
                VALUES (?, ?, ?, ?, ?, ?, ?, ?)
                "#,
            )
            .bind(&game_id)
            .bind(&game.name)
            .bind(platform_id)
            .bind(&primary_crc)
            .bind(game.release_year.map(|y| y as i32))
            .bind(&game.developer)
            .bind(&game.publisher)
            .bind(&game.genre)
            .execute(&pool)
            .await?;

            total_games += 1;
        }

        pb.inc(1);
    }

    pb.finish_with_message("Done");

    println!();
    println!("Database built successfully!");
    println!("  Platforms: {}", total_platforms);
    println!("  Games:     {}", total_games);
    println!("  Output:    {}", output_path.display());

    // Print file size
    if let Ok(metadata) = std::fs::metadata(output_path) {
        println!("  Size:      {}", format_size(metadata.len()));
    }

    Ok(())
}

async fn cmd_enrich_db(
    database: &PathBuf,
    openvgdb: &PathBuf,
    threshold: f64,
    dry_run: bool,
) -> Result<()> {
    enrich::enrich_database(database, openvgdb, threshold, dry_run).await
}

async fn cmd_unified_build(
    output: Option<PathBuf>,
    launchbox_xml: Option<PathBuf>,
    libretro_path: Option<PathBuf>,
    openvgdb: Option<PathBuf>,
    threshold: f64,
    download: bool,
    data_dir: Option<PathBuf>,
) -> Result<()> {
    // Determine output path - default to data directory
    let data_dir = data_dir.unwrap_or_else(download::default_data_dir);
    let output = output.unwrap_or_else(|| data_dir.join("games.db"));

    // Create data directory if needed
    if let Some(parent) = output.parent() {
        std::fs::create_dir_all(parent)?;
    }

    // If download flag is set, download sources first
    let (launchbox_xml, libretro_path, openvgdb) = if download {
        println!("Downloading metadata sources...");
        println!();

        let sources = download::download_all(&data_dir).await?;
        println!();

        (
            Some(sources.launchbox_xml),
            Some(sources.libretro_path),
            Some(sources.openvgdb_path),
        )
    } else {
        (launchbox_xml, libretro_path, openvgdb)
    };

    unified_import::build_unified_database(
        &output,
        launchbox_xml.as_deref(),
        libretro_path.as_deref(),
        openvgdb.as_deref(),
        threshold,
    ).await
}

async fn cmd_download(
    output: Option<PathBuf>,
    launchbox_only: bool,
    libretro_only: bool,
    openvgdb_only: bool,
) -> Result<()> {
    let data_dir = output.unwrap_or_else(download::default_data_dir);

    println!("Download directory: {}", data_dir.display());
    println!();

    std::fs::create_dir_all(&data_dir)?;

    let download_all = !launchbox_only && !libretro_only && !openvgdb_only;

    if download_all || launchbox_only {
        println!("LaunchBox:");
        download::download_launchbox(&data_dir).await?;
        println!();
    }

    if download_all || libretro_only {
        println!("LibRetro:");
        download::download_libretro(&data_dir).await?;
        println!();
    }

    if download_all || openvgdb_only {
        println!("OpenVGDB:");
        download::download_openvgdb(&data_dir).await?;
        println!();
    }

    println!("Downloads complete!");
    println!();
    println!("To build the database, run:");
    println!("  lunchbox-cli unified-build \\");
    println!("    --launchbox-xml {}/launchbox-metadata/Metadata.xml \\", data_dir.display());
    println!("    --libretro-path {}/libretro-database \\", data_dir.display());
    println!("    --openvgdb {}/openvgdb.sqlite", data_dir.display());
    println!();
    println!("Or simply:");
    println!("  lunchbox-cli unified-build --download");

    Ok(())
}
