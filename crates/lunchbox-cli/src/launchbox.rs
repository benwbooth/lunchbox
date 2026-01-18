//! LaunchBox metadata enrichment
//!
//! This module parses the LaunchBox Metadata.xml and enriches our game database
//! with descriptions, developers, publishers, genres, and ratings.

use anyhow::{Context, Result};
use indicatif::{ProgressBar, ProgressStyle};
use quick_xml::events::Event;
use quick_xml::Reader;
use sqlx::sqlite::SqlitePool;
use std::collections::HashMap;
use std::fs::File;
use std::io::BufReader;
use std::path::Path;

use crate::enrich::{normalize_title, similarity_ratio};

/// Extract words from a normalized title (for indexing)
fn extract_words(normalized: &str) -> Vec<String> {
    normalized
        .split_whitespace()
        .filter(|w| w.len() >= 2) // Skip single-char words
        .map(|w| w.to_string())
        .collect()
}

/// Game metadata from LaunchBox
#[derive(Debug, Clone, Default)]
pub struct LaunchBoxGame {
    pub name: String,
    pub platform: String,
    pub overview: Option<String>,
    pub developer: Option<String>,
    pub publisher: Option<String>,
    pub genres: Option<String>,
    pub release_date: Option<String>,
    pub release_year: Option<i32>,
    pub max_players: Option<String>,
    pub rating: Option<f64>,
    pub rating_count: Option<i64>,
    pub esrb: Option<String>,
    pub cooperative: Option<bool>,
    pub video_url: Option<String>,
    pub wikipedia_url: Option<String>,
    pub database_id: Option<i64>,
    pub release_type: Option<String>,
    pub steam_app_id: Option<i64>,
    pub notes: Option<String>,
}

/// Alternate name for a game (regional titles, etc.)
#[derive(Debug, Clone, Default)]
pub struct GameAlternateName {
    pub database_id: i64,
    pub alternate_name: String,
    pub region: Option<String>,
}

/// Parse LaunchBox Metadata.xml and return games indexed by normalized title
pub fn parse_launchbox_metadata(path: &Path) -> Result<Vec<LaunchBoxGame>> {
    let file = File::open(path).context("Failed to open Metadata.xml")?;
    let file_size = file.metadata()?.len();
    let reader = BufReader::new(file);

    let mut xml_reader = Reader::from_reader(reader);
    xml_reader.config_mut().trim_text(true);

    let mut games = Vec::new();
    let mut current_game: Option<LaunchBoxGame> = None;
    let mut current_field: Option<String> = None;
    let mut buf = Vec::new();

    let pb = ProgressBar::new(file_size);
    pb.set_style(
        ProgressStyle::default_bar()
            .template("{spinner:.green} [{elapsed_precise}] [{bar:40.cyan/blue}] {bytes}/{total_bytes} ({bytes_per_sec}) {msg}")
            .unwrap()
            .progress_chars("#>-"),
    );
    pb.set_message("Parsing XML");

    loop {
        match xml_reader.read_event_into(&mut buf) {
            Ok(Event::Start(ref e)) => {
                let tag_name = String::from_utf8_lossy(e.name().as_ref()).to_string();

                if tag_name == "Game" {
                    current_game = Some(LaunchBoxGame::default());
                } else if current_game.is_some() {
                    current_field = Some(tag_name);
                }
            }
            Ok(Event::End(ref e)) => {
                let tag_name = String::from_utf8_lossy(e.name().as_ref()).to_string();

                if tag_name == "Game" {
                    if let Some(game) = current_game.take() {
                        if !game.name.is_empty() && !game.platform.is_empty() {
                            games.push(game);
                        }
                    }
                }
                current_field = None;
            }
            Ok(Event::Text(ref e)) => {
                if let (Some(ref mut game), Some(ref field)) = (&mut current_game, &current_field) {
                    let text = e.unescape().unwrap_or_default().to_string();
                    if !text.is_empty() {
                        match field.as_str() {
                            "Name" => game.name = text,
                            "Platform" => game.platform = text,
                            "Overview" => game.overview = Some(text),
                            "Developer" => game.developer = Some(text),
                            "Publisher" => game.publisher = Some(text),
                            "Genres" => game.genres = Some(text),
                            "ReleaseDate" => game.release_date = Some(text),
                            "ReleaseYear" => game.release_year = text.parse().ok(),
                            "MaxPlayers" => game.max_players = Some(text),
                            "CommunityRating" => game.rating = text.parse().ok(),
                            "CommunityRatingCount" => game.rating_count = text.parse().ok(),
                            "ESRB" => game.esrb = Some(text),
                            "Cooperative" => game.cooperative = Some(text.to_lowercase() == "true"),
                            "VideoURL" => game.video_url = Some(text),
                            "WikipediaURL" => game.wikipedia_url = Some(text),
                            "DatabaseID" => game.database_id = text.parse().ok(),
                            "ReleaseType" => game.release_type = Some(text),
                            "SteamAppId" => game.steam_app_id = text.parse().ok(),
                            "Notes" => game.notes = Some(text),
                            _ => {}
                        }
                    }
                }
            }
            Ok(Event::Eof) => break,
            Err(e) => {
                // Log error but continue parsing
                tracing::warn!("XML parse error at position {}: {}", xml_reader.buffer_position(), e);
            }
            _ => {}
        }

        // Update progress periodically
        if games.len() % 10000 == 0 {
            pb.set_position(xml_reader.buffer_position() as u64);
        }

        buf.clear();
    }

    pb.finish_with_message(format!("Parsed {} games", games.len()));

    Ok(games)
}

/// Parse GameAlternateName entries from LaunchBox Metadata.xml
pub fn parse_alternate_names(path: &Path) -> Result<Vec<GameAlternateName>> {
    let file = File::open(path).context("Failed to open Metadata.xml")?;
    let file_size = file.metadata()?.len();
    let reader = BufReader::new(file);

    let mut xml_reader = Reader::from_reader(reader);
    xml_reader.config_mut().trim_text(true);

    let mut alt_names = Vec::new();
    let mut current_alt: Option<GameAlternateName> = None;
    let mut current_field: Option<String> = None;
    let mut buf = Vec::new();

    let pb = ProgressBar::new(file_size);
    pb.set_style(
        ProgressStyle::default_bar()
            .template("{spinner:.green} [{elapsed_precise}] [{bar:40.cyan/blue}] {bytes}/{total_bytes} ({bytes_per_sec}) {msg}")
            .unwrap()
            .progress_chars("#>-"),
    );
    pb.set_message("Parsing alternate names");

    loop {
        match xml_reader.read_event_into(&mut buf) {
            Ok(Event::Start(ref e)) => {
                let tag_name = String::from_utf8_lossy(e.name().as_ref()).to_string();

                if tag_name == "GameAlternateName" {
                    current_alt = Some(GameAlternateName::default());
                } else if current_alt.is_some() {
                    current_field = Some(tag_name);
                }
            }
            Ok(Event::End(ref e)) => {
                let tag_name = String::from_utf8_lossy(e.name().as_ref()).to_string();

                if tag_name == "GameAlternateName" {
                    if let Some(alt) = current_alt.take() {
                        // Only keep entries with valid data
                        if alt.database_id > 0 && !alt.alternate_name.is_empty() {
                            alt_names.push(alt);
                        }
                    }
                }
                current_field = None;
            }
            Ok(Event::Text(ref e)) => {
                if let (Some(ref mut alt), Some(ref field)) = (&mut current_alt, &current_field) {
                    let text = e.unescape().unwrap_or_default().to_string();
                    if !text.is_empty() {
                        match field.as_str() {
                            "DatabaseID" => alt.database_id = text.parse().unwrap_or(0),
                            "AlternateName" => alt.alternate_name = text,
                            "Region" => alt.region = Some(text),
                            _ => {}
                        }
                    }
                }
            }
            Ok(Event::Eof) => break,
            Err(e) => {
                tracing::warn!("XML parse error at position {}: {}", xml_reader.buffer_position(), e);
            }
            _ => {}
        }

        // Update progress periodically
        if alt_names.len() % 10000 == 0 {
            pb.set_position(xml_reader.buffer_position() as u64);
        }

        buf.clear();
    }

    pb.finish_with_message(format!("Parsed {} alternate names", alt_names.len()));

    Ok(alt_names)
}

/// Image metadata from LaunchBox
#[derive(Debug, Clone, Default)]
pub struct GameImage {
    pub database_id: i64,
    pub filename: String,  // UUID filename like "3c4cc1f6-051a-43f5-b904-b60eed55b074.jpg"
    pub image_type: String,  // "Box - Front", "Screenshot - Gameplay", etc.
    pub region: Option<String>,
    pub crc32: Option<String>,
}

/// Parse GameImage entries from LaunchBox Metadata.xml
pub fn parse_game_images(path: &Path) -> Result<Vec<GameImage>> {
    let file = File::open(path).context("Failed to open Metadata.xml")?;
    let file_size = file.metadata()?.len();
    let reader = BufReader::new(file);

    let mut xml_reader = Reader::from_reader(reader);
    xml_reader.config_mut().trim_text(true);

    let mut images = Vec::new();
    let mut current_image: Option<GameImage> = None;
    let mut current_field: Option<String> = None;
    let mut buf = Vec::new();

    let pb = ProgressBar::new(file_size);
    pb.set_style(
        ProgressStyle::default_bar()
            .template("{spinner:.green} [{elapsed_precise}] [{bar:40.cyan/blue}] {bytes}/{total_bytes} ({bytes_per_sec}) {msg}")
            .unwrap()
            .progress_chars("#>-"),
    );
    pb.set_message("Parsing game images");

    loop {
        match xml_reader.read_event_into(&mut buf) {
            Ok(Event::Start(ref e)) => {
                let tag_name = String::from_utf8_lossy(e.name().as_ref()).to_string();

                if tag_name == "GameImage" {
                    current_image = Some(GameImage::default());
                } else if current_image.is_some() {
                    current_field = Some(tag_name);
                }
            }
            Ok(Event::End(ref e)) => {
                let tag_name = String::from_utf8_lossy(e.name().as_ref()).to_string();

                if tag_name == "GameImage" {
                    if let Some(img) = current_image.take() {
                        // Only keep entries with valid data
                        if img.database_id > 0 && !img.filename.is_empty() && !img.image_type.is_empty() {
                            images.push(img);
                        }
                    }
                }
                current_field = None;
            }
            Ok(Event::Text(ref e)) => {
                if let (Some(ref mut img), Some(ref field)) = (&mut current_image, &current_field) {
                    let text = e.unescape().unwrap_or_default().to_string();
                    if !text.is_empty() {
                        match field.as_str() {
                            "DatabaseID" => img.database_id = text.parse().unwrap_or(0),
                            "FileName" => img.filename = text,
                            "Type" => img.image_type = text,
                            "Region" => img.region = Some(text),
                            "CRC32" => img.crc32 = Some(text),
                            _ => {}
                        }
                    }
                }
            }
            Ok(Event::Eof) => break,
            Err(e) => {
                tracing::warn!("XML parse error at position {}: {}", xml_reader.buffer_position(), e);
            }
            _ => {}
        }

        // Update progress periodically
        if images.len() % 50000 == 0 {
            pb.set_position(xml_reader.buffer_position() as u64);
        }

        buf.clear();
    }

    pb.finish_with_message(format!("Parsed {} game images", images.len()));

    Ok(images)
}

/// Build lookup indexes for LaunchBox games
pub struct LaunchBoxIndex {
    /// Normalized title + platform -> game index
    by_title_platform: HashMap<(String, String), usize>,
    /// Normalized title -> list of game indices (for fuzzy matching)
    by_title: HashMap<String, Vec<usize>>,
    /// Word -> list of game indices (for fuzzy pre-filtering)
    by_word: HashMap<String, Vec<usize>>,
    /// All games
    pub games: Vec<LaunchBoxGame>,
    /// Normalized titles for each game (pre-computed)
    normalized_titles: Vec<String>,
}

/// Normalize platform names to match between LibRetro and LaunchBox
fn normalize_platform(platform: &str) -> String {
    let p = platform.to_lowercase();

    // Common mappings
    match p.as_str() {
        "nintendo entertainment system" | "nintendo nes" | "nes" => "nes".to_string(),
        "super nintendo entertainment system" | "super nintendo" | "snes" => "snes".to_string(),
        "nintendo 64" | "n64" => "n64".to_string(),
        "nintendo game boy" | "game boy" | "gameboy" => "gb".to_string(),
        "nintendo game boy color" | "game boy color" | "gbc" => "gbc".to_string(),
        "nintendo game boy advance" | "game boy advance" | "gba" => "gba".to_string(),
        "nintendo ds" | "nds" => "nds".to_string(),
        "nintendo gamecube" | "gamecube" | "gc" => "gamecube".to_string(),
        "nintendo wii" | "wii" => "wii".to_string(),
        "sega genesis" | "sega mega drive" | "mega drive" | "genesis" => "genesis".to_string(),
        "sega master system" | "master system" | "sms" => "sms".to_string(),
        "sega game gear" | "game gear" | "gg" => "gamegear".to_string(),
        "sega saturn" | "saturn" => "saturn".to_string(),
        "sega dreamcast" | "dreamcast" | "dc" => "dreamcast".to_string(),
        "sega cd" | "mega cd" => "segacd".to_string(),
        "sony playstation" | "playstation" | "psx" | "ps1" => "psx".to_string(),
        "sony playstation 2" | "playstation 2" | "ps2" => "ps2".to_string(),
        "sony playstation portable" | "playstation portable" | "psp" => "psp".to_string(),
        "microsoft xbox" | "xbox" => "xbox".to_string(),
        "atari 2600" | "atari vcs" => "atari2600".to_string(),
        "atari 7800" => "atari7800".to_string(),
        "atari lynx" | "lynx" => "lynx".to_string(),
        "neo geo" | "neogeo" | "snk neo geo aes" => "neogeo".to_string(),
        "turbografx-16" | "pc engine" | "turbografx 16" => "pce".to_string(),
        "arcade" | "mame" => "arcade".to_string(),
        "commodore 64" | "c64" => "c64".to_string(),
        "amstrad cpc" | "cpc" => "cpc".to_string(),
        "zx spectrum" | "sinclair zx spectrum" => "zxspectrum".to_string(),
        "msx" | "msx2" => "msx".to_string(),
        _ => p.replace(' ', "").replace('-', ""),
    }
}

impl LaunchBoxIndex {
    pub fn new(games: Vec<LaunchBoxGame>) -> Self {
        let mut by_title_platform: HashMap<(String, String), usize> = HashMap::new();
        let mut by_title: HashMap<String, Vec<usize>> = HashMap::new();
        let mut by_word: HashMap<String, Vec<usize>> = HashMap::new();
        let mut normalized_titles: Vec<String> = Vec::with_capacity(games.len());

        for (idx, game) in games.iter().enumerate() {
            let normalized_title = normalize_title(&game.name);
            let normalized_platform = normalize_platform(&game.platform);

            // Index by title + platform (first one wins)
            let key = (normalized_title.clone(), normalized_platform);
            by_title_platform.entry(key).or_insert(idx);

            // Index by title only for fallback matching
            by_title.entry(normalized_title.clone()).or_default().push(idx);

            // Index by each word for fuzzy pre-filtering
            for word in extract_words(&normalized_title) {
                by_word.entry(word).or_default().push(idx);
            }

            normalized_titles.push(normalized_title);
        }

        LaunchBoxIndex {
            by_title_platform,
            by_title,
            by_word,
            games,
            normalized_titles,
        }
    }

    /// Find candidate games that share at least one word with the query
    fn find_fuzzy_candidates(&self, normalized_title: &str) -> Vec<usize> {
        let query_words = extract_words(normalized_title);
        let mut candidates: std::collections::HashSet<usize> = std::collections::HashSet::new();

        for word in &query_words {
            if let Some(indices) = self.by_word.get(word) {
                candidates.extend(indices.iter().cloned());
            }
        }

        candidates.into_iter().collect()
    }

    /// Find a match for a game title and platform
    pub fn find_match(&self, title: &str, platform: &str, threshold: f64) -> Option<(usize, f64)> {
        let normalized_title = normalize_title(title);
        let normalized_platform = normalize_platform(platform);

        // Try exact title + platform match
        if let Some(&idx) = self.by_title_platform.get(&(normalized_title.clone(), normalized_platform.clone())) {
            return Some((idx, 1.0));
        }

        // Try exact title match (any platform)
        if let Some(indices) = self.by_title.get(&normalized_title) {
            if let Some(&idx) = indices.first() {
                return Some((idx, 1.0));
            }
        }

        // Skip fuzzy for very short titles
        if normalized_title.len() < 3 {
            return None;
        }

        // Fuzzy match only against candidates that share at least one word
        let candidates = self.find_fuzzy_candidates(&normalized_title);

        if candidates.is_empty() {
            return None;
        }

        let mut best_match: Option<(usize, f64)> = None;

        for idx in candidates {
            let sim = similarity_ratio(&normalized_title, &self.normalized_titles[idx]);
            if sim >= threshold {
                if best_match.is_none() || sim > best_match.unwrap().1 {
                    best_match = Some((idx, sim));
                }
            }
        }

        best_match
    }
}

/// Game info from our database
#[derive(Debug)]
struct OurGame {
    id: String,
    title: String,
    platform: String,
}

/// Enrich our database with LaunchBox metadata
pub async fn enrich_from_launchbox(
    database_path: &Path,
    launchbox_xml_path: &Path,
    threshold: f64,
    dry_run: bool,
) -> Result<()> {
    println!("LaunchBox Metadata Enrichment");
    println!("==============================");
    println!("Games DB:    {}", database_path.display());
    println!("LaunchBox:   {}", launchbox_xml_path.display());
    println!("Threshold:   {:.0}%", threshold * 100.0);
    println!("Dry run:     {}", dry_run);
    println!();

    // Verify paths
    if !database_path.exists() {
        anyhow::bail!("Games database not found: {}", database_path.display());
    }
    if !launchbox_xml_path.exists() {
        anyhow::bail!("LaunchBox XML not found: {}", launchbox_xml_path.display());
    }

    // Parse LaunchBox XML
    println!("Parsing LaunchBox metadata...");
    let launchbox_games = parse_launchbox_metadata(launchbox_xml_path)?;
    println!("  Loaded {} games", launchbox_games.len());

    // Build index
    print!("Building search index... ");
    std::io::Write::flush(&mut std::io::stdout())?;
    let index = LaunchBoxIndex::new(launchbox_games);
    println!("done");
    println!("  Title+Platform index: {} entries", index.by_title_platform.len());
    println!("  Title index: {} entries", index.by_title.len());
    println!();

    // Load our games
    print!("Loading games from database... ");
    std::io::Write::flush(&mut std::io::stdout())?;

    let db_url = format!("sqlite:{}?mode=ro", database_path.display());
    let pool = SqlitePool::connect(&db_url).await?;

    let rows = sqlx::query(
        r#"
        SELECT g.id, g.title, p.name as platform
        FROM games g
        JOIN platforms p ON g.platform_id = p.id
        "#
    )
    .fetch_all(&pool)
    .await?;

    let our_games: Vec<OurGame> = rows.iter().map(|row| {
        use sqlx::Row;
        OurGame {
            id: row.get("id"),
            title: row.get("title"),
            platform: row.get("platform"),
        }
    }).collect();

    pool.close().await;
    println!("{} games", our_games.len());
    println!();

    // Find matches
    println!("Finding matches...");
    let pb = ProgressBar::new(our_games.len() as u64);
    pb.set_style(
        ProgressStyle::default_bar()
            .template("{spinner:.green} [{elapsed_precise}] [{bar:40.cyan/blue}] {pos}/{len} ({per_sec}) ETA: {eta}")
            .unwrap()
            .progress_chars("#>-"),
    );

    let mut matches: Vec<(String, usize, f64)> = Vec::new(); // (our_id, launchbox_idx, similarity)
    let mut exact_matches = 0;
    let mut fuzzy_matches = 0;

    for game in &our_games {
        if let Some((idx, sim)) = index.find_match(&game.title, &game.platform, threshold) {
            if sim >= 1.0 {
                exact_matches += 1;
            } else {
                fuzzy_matches += 1;
            }
            matches.push((game.id.clone(), idx, sim));
        }
        pb.inc(1);
    }

    pb.finish_with_message("Complete");
    println!();

    println!("Match Results:");
    println!("  Exact matches: {:>6} ({:.1}%)", exact_matches, 100.0 * exact_matches as f64 / our_games.len() as f64);
    println!("  Fuzzy matches: {:>6} ({:.1}%)", fuzzy_matches, 100.0 * fuzzy_matches as f64 / our_games.len() as f64);
    println!("  No match:      {:>6} ({:.1}%)", our_games.len() - matches.len(), 100.0 * (our_games.len() - matches.len()) as f64 / our_games.len() as f64);
    println!("  ─────────────────────────");
    println!("  Total matched: {:>6} / {} ({:.1}%)",
        matches.len(),
        our_games.len(),
        100.0 * matches.len() as f64 / our_games.len() as f64
    );
    println!();

    // Show sample matches
    let samples: Vec<_> = matches.iter()
        .filter(|(_, _, sim)| *sim < 1.0)
        .take(10)
        .collect();

    if !samples.is_empty() {
        println!("Sample fuzzy matches:");
        for (our_id, lb_idx, sim) in &samples {
            let our_game = our_games.iter().find(|g| g.id == *our_id).unwrap();
            let lb_game = &index.games[*lb_idx];
            println!("  {:.0}% \"{}\" ({}) -> \"{}\" ({})",
                sim * 100.0,
                our_game.title,
                our_game.platform,
                lb_game.name,
                lb_game.platform
            );
        }
        println!();
    }

    if dry_run {
        println!("[Dry run] Would update {} games with metadata", matches.len());
        return Ok(());
    }

    // Apply updates
    println!("Applying metadata updates...");
    let db_url = format!("sqlite:{}", database_path.display());
    let pool = SqlitePool::connect(&db_url).await?;

    let pb = ProgressBar::new(matches.len() as u64);
    pb.set_style(
        ProgressStyle::default_bar()
            .template("{spinner:.green} [{elapsed_precise}] [{bar:40.cyan/blue}] {pos}/{len} ({per_sec})")
            .unwrap()
            .progress_chars("#>-"),
    );

    let mut updated = 0;

    for (our_id, lb_idx, _) in &matches {
        let lb_game = &index.games[*lb_idx];

        // Only update if we have useful metadata
        let has_data = lb_game.overview.is_some() || lb_game.developer.is_some() ||
                       lb_game.publisher.is_some() || lb_game.genres.is_some() ||
                       lb_game.release_year.is_some() || lb_game.esrb.is_some() ||
                       lb_game.video_url.is_some() || lb_game.wikipedia_url.is_some();

        if has_data {
            sqlx::query(
                r#"
                UPDATE games SET
                    description = COALESCE(?, description),
                    developer = COALESCE(?, developer),
                    publisher = COALESCE(?, publisher),
                    genre = COALESCE(?, genre),
                    release_date = COALESCE(?, release_date),
                    release_year = COALESCE(?, release_year),
                    players = COALESCE(?, players),
                    rating = COALESCE(?, rating),
                    rating_count = COALESCE(?, rating_count),
                    esrb = COALESCE(?, esrb),
                    cooperative = COALESCE(?, cooperative),
                    video_url = COALESCE(?, video_url),
                    wikipedia_url = COALESCE(?, wikipedia_url),
                    metadata_fetched = 1,
                    metadata_source = COALESCE(metadata_source, 'launchbox'),
                    updated_at = CURRENT_TIMESTAMP
                WHERE id = ?
                "#
            )
            .bind(&lb_game.overview)
            .bind(&lb_game.developer)
            .bind(&lb_game.publisher)
            .bind(&lb_game.genres)
            .bind(&lb_game.release_date)
            .bind(lb_game.release_year)
            .bind(&lb_game.max_players)
            .bind(lb_game.rating)
            .bind(lb_game.rating_count)
            .bind(&lb_game.esrb)
            .bind(lb_game.cooperative)
            .bind(&lb_game.video_url)
            .bind(&lb_game.wikipedia_url)
            .bind(our_id)
            .execute(&pool)
            .await?;

            updated += 1;
        }

        pb.inc(1);
    }

    pb.finish_with_message("Done");
    pool.close().await;

    println!();
    println!("Updated {} games with LaunchBox metadata", updated);

    Ok(())
}
