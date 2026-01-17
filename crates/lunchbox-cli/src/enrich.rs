//! Database enrichment with OpenVGDB metadata
//!
//! This module handles enriching the LibRetro-based game database with
//! metadata from OpenVGDB, using CRC matching and fuzzy title matching.

use anyhow::{Context, Result};
use indicatif::{ProgressBar, ProgressStyle};
use sqlx::sqlite::SqlitePool;
use std::collections::{HashMap, HashSet};
use std::path::Path;

/// Metadata from OpenVGDB for a single game
#[derive(Debug, Clone)]
pub struct OpenVGDBGame {
    pub release_id: i64,
    pub crc: String,
    pub release_title: String,
    pub normalized_title: String,
    pub title_words: HashSet<String>,
}

/// Match result between LibRetro game and OpenVGDB
#[derive(Debug)]
pub struct MatchResult {
    pub game_id: String,
    pub game_title: String,
    pub openvgdb_release_id: i64,
    pub openvgdb_title: String,
    pub match_type: MatchType,
    pub similarity: f64,
}

#[derive(Debug, Clone, Copy)]
pub enum MatchType {
    ExactCrc,
    ExactTitle,
    FuzzyTitle,
}

impl std::fmt::Display for MatchType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            MatchType::ExactCrc => write!(f, "CRC"),
            MatchType::ExactTitle => write!(f, "Title"),
            MatchType::FuzzyTitle => write!(f, "Fuzzy"),
        }
    }
}

/// Normalize a game title for matching
///
/// - Converts to lowercase
/// - Removes content in parentheses (region tags, version info)
/// - Removes content in square brackets
/// Extract all parenthetical content from a title as a sorted, deduplicated list
/// "Game (USA) (Rev A)" -> ["Rev A", "USA"]
pub fn extract_tags(title: &str) -> Vec<String> {
    let mut tags = Vec::new();
    let mut current_tag = String::new();
    let mut depth = 0;

    for c in title.chars() {
        match c {
            '(' | '[' => {
                depth += 1;
                if depth == 1 {
                    current_tag.clear();
                } else {
                    current_tag.push(c);
                }
            }
            ')' | ']' => {
                if depth == 1 && !current_tag.trim().is_empty() {
                    tags.push(current_tag.trim().to_string());
                }
                depth = (depth - 1).max(0);
                if depth > 0 {
                    current_tag.push(c);
                }
            }
            _ if depth > 0 => current_tag.push(c),
            _ => {}
        }
    }

    tags.sort();
    tags.dedup();
    tags
}

/// Check if a tag is a variant-significant tag (region, version, etc.)
/// vs a metadata tag (publisher, etc.) that should be merged
///
/// Variant-significant tags indicate genuinely different game content:
/// - Regions (USA, Japan, Europe, etc.) - different localizations
/// - Versions (v1.0, Rev A, etc.) - bug fixes, content changes
/// - Discs/Parts (Disc 1, Side A, etc.) - multi-disc games
/// - Special releases (Beta, Proto, Demo, etc.) - different from retail
/// - Platform/Distribution (Virtual Console, PSN, etc.) - often different content
/// - Addons (DLC, Expansion) - different products
pub fn is_variant_tag(tag: &str) -> bool {
    let tag_lower = tag.to_lowercase();
    let tag_trimmed = tag_lower.trim();

    // === REGIONS ===
    // Single word regions
    let regions = [
        "usa", "europe", "japan", "world", "asia", "australia", "brazil",
        "canada", "china", "finland", "france", "germany", "greece",
        "hong kong", "italy", "korea", "mexico", "netherlands", "norway",
        "poland", "portugal", "russia", "scandinavia", "spain", "sweden",
        "taiwan", "uk", "argentina", "austria", "belgium", "chile",
        "colombia", "czech", "denmark", "hungary", "india", "indonesia",
        "ireland", "israel", "malaysia", "new zealand", "peru", "philippines",
        "romania", "singapore", "slovakia", "south africa", "switzerland",
        "thailand", "turkey", "ukraine", "vietnam",
    ];
    if regions.iter().any(|r| tag_trimmed == *r) {
        return true;
    }

    // Language codes (2-letter ISO or compound like "En,Fr,De,Es,It")
    if tag_trimmed.len() == 2 && tag_trimmed.chars().all(|c| c.is_ascii_alphabetic()) {
        return true; // En, Ja, De, Fr, etc.
    }
    if tag_trimmed.contains(',') && tag_trimmed.split(',').all(|p| p.trim().len() <= 3) {
        return true; // En,Fr,De,Es,It
    }

    // === VERSIONS ===
    if tag_trimmed.starts_with("v") && tag_trimmed.chars().nth(1).map(|c| c.is_ascii_digit()).unwrap_or(false) {
        return true; // v1.0, v1.1, etc.
    }
    if tag_trimmed.starts_with("rev") {
        return true; // Rev A, Rev 1, etc.
    }
    if tag_trimmed.starts_with("build") || tag_trimmed.starts_with("ver") {
        return true;
    }

    // === DISCS/PARTS ===
    if tag_trimmed.starts_with("disc") || tag_trimmed.starts_with("disk") {
        return true;
    }
    if tag_trimmed.starts_with("side") || tag_trimmed.starts_with("part") {
        return true;
    }

    // === SPECIAL RELEASES ===
    let special = [
        "beta", "proto", "prototype", "sample", "demo", "kiosk", "debug",
        "unl", "unlicensed", "pirate", "hack", "alt", "alternate",
        "ndc", "competition", "promo", "promotional", "preview",
        "alpha", "pre-release", "prerelease", "aftermarket",
    ];
    if special.iter().any(|s| tag_trimmed.contains(s)) {
        return true;
    }

    // === PLATFORM/DISTRIBUTION ===
    let platforms = [
        "virtual console", "psn", "xbla", "eshop", "steam", "gog",
        "switch online", "wiiware", "dsiware", "xbox live", "playstation network",
        "nintendo online", "game pass", "psplus", "arcade archives",
    ];
    if platforms.iter().any(|p| tag_trimmed.contains(p)) {
        return true;
    }

    // === ADDONS ===
    if tag_trimmed == "addon" || tag_trimmed == "dlc" || tag_trimmed.contains("expansion") {
        return true;
    }

    false
}

/// Extract only variant-significant tags from a title
pub fn extract_variant_tags(title: &str) -> Vec<String> {
    extract_tags(title).into_iter().filter(|t| is_variant_tag(t)).collect()
}

/// - Removes "the " prefix
/// - Removes punctuation
/// - Normalizes whitespace
pub fn normalize_title(title: &str) -> String {
    let mut result = title.to_lowercase();

    // Remove content in parentheses: "Game (USA)" -> "Game"
    while let Some(start) = result.find('(') {
        if let Some(end) = result[start..].find(')') {
            result = format!("{}{}", &result[..start], &result[start + end + 1..]);
        } else {
            break;
        }
    }

    // Remove content in square brackets: "Game [!]" -> "Game"
    while let Some(start) = result.find('[') {
        if let Some(end) = result[start..].find(']') {
            result = format!("{}{}", &result[..start], &result[start + end + 1..]);
        } else {
            break;
        }
    }

    // Remove "the " prefix
    if result.starts_with("the ") {
        result = result[4..].to_string();
    }

    // Remove punctuation and extra whitespace
    result = result
        .chars()
        .map(|c| if c.is_alphanumeric() || c.is_whitespace() { c } else { ' ' })
        .collect();

    // Normalize whitespace
    result.split_whitespace().collect::<Vec<_>>().join(" ")
}

/// Extract words from a normalized title (for indexing)
fn extract_words(normalized: &str) -> HashSet<String> {
    normalized
        .split_whitespace()
        .filter(|w| w.len() >= 2) // Skip single-char words
        .map(|w| w.to_string())
        .collect()
}

/// Calculate Levenshtein distance between two strings
fn levenshtein_distance(a: &str, b: &str) -> usize {
    let a_chars: Vec<char> = a.chars().collect();
    let b_chars: Vec<char> = b.chars().collect();
    let a_len = a_chars.len();
    let b_len = b_chars.len();

    if a_len == 0 {
        return b_len;
    }
    if b_len == 0 {
        return a_len;
    }

    let mut prev_row: Vec<usize> = (0..=b_len).collect();
    let mut curr_row: Vec<usize> = vec![0; b_len + 1];

    for i in 1..=a_len {
        curr_row[0] = i;
        for j in 1..=b_len {
            let cost = if a_chars[i - 1] == b_chars[j - 1] { 0 } else { 1 };
            curr_row[j] = (prev_row[j] + 1)
                .min(curr_row[j - 1] + 1)
                .min(prev_row[j - 1] + cost);
        }
        std::mem::swap(&mut prev_row, &mut curr_row);
    }

    prev_row[b_len]
}

/// Calculate similarity ratio between two strings (0.0 to 1.0)
pub fn similarity_ratio(a: &str, b: &str) -> f64 {
    let max_len = a.len().max(b.len());
    if max_len == 0 {
        return 1.0;
    }
    let distance = levenshtein_distance(a, b);
    1.0 - (distance as f64 / max_len as f64)
}

/// Load games from OpenVGDB
async fn load_openvgdb_games(openvgdb_path: &Path) -> Result<Vec<OpenVGDBGame>> {
    let db_url = format!("sqlite:{}?mode=ro", openvgdb_path.display());
    let pool = SqlitePool::connect(&db_url).await?;

    let rows = sqlx::query(
        r#"
        SELECT
            rel.releaseID,
            COALESCE(r.romHashCRC, '') as crc,
            COALESCE(rel.releaseTitleName, '') as release_title
        FROM ROMs r
        JOIN RELEASES rel ON r.romID = rel.romID
        WHERE r.romHashCRC IS NOT NULL AND r.romHashCRC != ''
        "#
    )
    .fetch_all(&pool)
    .await?;

    let games: Vec<OpenVGDBGame> = rows.iter().map(|row| {
        use sqlx::Row;
        let release_title: String = row.get("release_title");
        let normalized = normalize_title(&release_title);
        let words = extract_words(&normalized);
        OpenVGDBGame {
            release_id: row.get("releaseID"),
            crc: row.get("crc"),
            release_title,
            normalized_title: normalized,
            title_words: words,
        }
    }).collect();

    pool.close().await;
    Ok(games)
}

/// Game info from LibRetro database
#[derive(Debug, Clone)]
struct LibRetroGame {
    id: String,
    title: String,
    crc32: Option<String>,
}

/// Load games from LibRetro-based database
async fn load_libretro_games(db_path: &Path) -> Result<Vec<LibRetroGame>> {
    let db_url = format!("sqlite:{}?mode=ro", db_path.display());
    let pool = SqlitePool::connect(&db_url).await?;

    let rows = sqlx::query(
        r#"
        SELECT id, title, libretro_crc32
        FROM games
        "#
    )
    .fetch_all(&pool)
    .await?;

    let games: Vec<LibRetroGame> = rows.iter().map(|row| {
        use sqlx::Row;
        LibRetroGame {
            id: row.get("id"),
            title: row.get("title"),
            crc32: row.get("libretro_crc32"),
        }
    }).collect();

    pool.close().await;
    Ok(games)
}

/// Lookup indexes for fast matching
struct MatchMaps {
    /// CRC -> OpenVGDB game index (uppercase CRC)
    by_crc: HashMap<String, usize>,
    /// Normalized title -> OpenVGDB game index
    by_title: HashMap<String, usize>,
    /// Word -> list of OpenVGDB game indices (for fuzzy pre-filtering)
    by_word: HashMap<String, Vec<usize>>,
    /// All OpenVGDB games
    games: Vec<OpenVGDBGame>,
}

fn build_match_maps(openvgdb_games: Vec<OpenVGDBGame>) -> MatchMaps {
    let mut by_crc: HashMap<String, usize> = HashMap::new();
    let mut by_title: HashMap<String, usize> = HashMap::new();
    let mut by_word: HashMap<String, Vec<usize>> = HashMap::new();

    for (idx, game) in openvgdb_games.iter().enumerate() {
        // Index by CRC (uppercase) - first one wins
        let crc = game.crc.to_uppercase();
        if !crc.is_empty() {
            by_crc.entry(crc).or_insert(idx);
        }

        // Index by normalized title - first one wins
        if !game.normalized_title.is_empty() {
            by_title.entry(game.normalized_title.clone()).or_insert(idx);
        }

        // Index by each word for fuzzy matching
        for word in &game.title_words {
            by_word.entry(word.clone()).or_default().push(idx);
        }
    }

    MatchMaps {
        by_crc,
        by_title,
        by_word,
        games: openvgdb_games,
    }
}

/// Find candidate games that share at least one word with the query
fn find_fuzzy_candidates(normalized: &str, maps: &MatchMaps) -> HashSet<usize> {
    let query_words = extract_words(normalized);
    let mut candidates = HashSet::new();

    for word in &query_words {
        if let Some(indices) = maps.by_word.get(word) {
            candidates.extend(indices.iter().cloned());
        }
    }

    candidates
}

/// Find the best match for a LibRetro game in OpenVGDB
fn find_best_match(
    game: &LibRetroGame,
    maps: &MatchMaps,
    threshold: f64,
) -> Option<MatchResult> {
    let normalized_title = normalize_title(&game.title);

    // Try CRC match first (most reliable)
    if let Some(crc) = &game.crc32 {
        let crc_upper = crc.to_uppercase();
        if let Some(&idx) = maps.by_crc.get(&crc_upper) {
            let matched = &maps.games[idx];
            return Some(MatchResult {
                game_id: game.id.clone(),
                game_title: game.title.clone(),
                openvgdb_release_id: matched.release_id,
                openvgdb_title: matched.release_title.clone(),
                match_type: MatchType::ExactCrc,
                similarity: 1.0,
            });
        }
    }

    // Try exact normalized title match
    if let Some(&idx) = maps.by_title.get(&normalized_title) {
        let matched = &maps.games[idx];
        return Some(MatchResult {
            game_id: game.id.clone(),
            game_title: game.title.clone(),
            openvgdb_release_id: matched.release_id,
            openvgdb_title: matched.release_title.clone(),
            match_type: MatchType::ExactTitle,
            similarity: 1.0,
        });
    }

    // Skip fuzzy matching for very short titles
    if normalized_title.len() < 3 {
        return None;
    }

    // Fuzzy matching: only compare against candidates that share at least one word
    let candidates = find_fuzzy_candidates(&normalized_title, maps);

    if candidates.is_empty() {
        return None;
    }

    let mut best_match: Option<(usize, f64)> = None;

    for idx in candidates {
        let openvgdb_game = &maps.games[idx];
        let sim = similarity_ratio(&normalized_title, &openvgdb_game.normalized_title);

        if sim >= threshold {
            if best_match.is_none() || sim > best_match.unwrap().1 {
                best_match = Some((idx, sim));
            }
        }
    }

    best_match.map(|(idx, sim)| {
        let matched = &maps.games[idx];
        MatchResult {
            game_id: game.id.clone(),
            game_title: game.title.clone(),
            openvgdb_release_id: matched.release_id,
            openvgdb_title: matched.release_title.clone(),
            match_type: MatchType::FuzzyTitle,
            similarity: sim,
        }
    })
}

/// OpenVGDB release metadata for batch processing
struct ReleaseData {
    release_id: i64,
    description: Option<String>,
    developer: Option<String>,
    publisher: Option<String>,
    genre: Option<String>,
    release_date: Option<String>,
}

/// Update the games database with matched metadata using batch operations
async fn apply_matches(
    db_path: &Path,
    openvgdb_path: &Path,
    matches: &[MatchResult],
) -> Result<usize> {
    if matches.is_empty() {
        return Ok(0);
    }

    // Connect to both databases
    let db_url = format!("sqlite:{}", db_path.display());
    let pool = SqlitePool::connect(&db_url).await?;

    let openvgdb_url = format!("sqlite:{}?mode=ro", openvgdb_path.display());
    let openvgdb_pool = SqlitePool::connect(&openvgdb_url).await?;

    // Step 1: Batch fetch all release data from OpenVGDB
    println!("  Fetching OpenVGDB release data...");
    let release_ids: Vec<i64> = matches.iter().map(|m| m.openvgdb_release_id).collect();

    // Fetch in batches to avoid query size limits
    let mut release_data: HashMap<i64, ReleaseData> = HashMap::new();
    let batch_size = 500;

    for chunk in release_ids.chunks(batch_size) {
        // Build query with placeholders
        let placeholders: String = chunk.iter().map(|_| "?").collect::<Vec<_>>().join(",");
        let query = format!(
            "SELECT releaseID, releaseDescription, releaseDeveloper, releasePublisher, releaseGenre, releaseDate FROM RELEASES WHERE releaseID IN ({})",
            placeholders
        );

        let mut q = sqlx::query(&query);
        for id in chunk {
            q = q.bind(id);
        }

        let rows = q.fetch_all(&openvgdb_pool).await?;

        for row in rows {
            use sqlx::Row;
            let release_id: i64 = row.get("releaseID");
            release_data.insert(release_id, ReleaseData {
                release_id,
                description: row.get("releaseDescription"),
                developer: row.get("releaseDeveloper"),
                publisher: row.get("releasePublisher"),
                genre: row.get("releaseGenre"),
                release_date: row.get("releaseDate"),
            });
        }
    }

    openvgdb_pool.close().await;
    println!("  Fetched {} release records", release_data.len());

    // Step 2: Batch update our database using transactions
    let pb = ProgressBar::new(matches.len() as u64);
    pb.set_style(
        ProgressStyle::default_bar()
            .template("{spinner:.green} [{elapsed_precise}] [{bar:40.cyan/blue}] {pos}/{len} ({per_sec}) {msg}")
            .unwrap()
            .progress_chars("#>-"),
    );
    pb.set_message("Updating database");

    let mut updated = 0;
    let update_batch_size = 1000;

    for chunk in matches.chunks(update_batch_size) {
        let mut tx = pool.begin().await?;

        for m in chunk {
            if let Some(data) = release_data.get(&m.openvgdb_release_id) {
                // Only update if we have something to add
                let has_data = data.description.is_some() || data.developer.is_some() ||
                               data.publisher.is_some() || data.genre.is_some() || data.release_date.is_some();

                if has_data {
                    sqlx::query(
                        r#"
                        UPDATE games SET
                            description = COALESCE(?, description),
                            developer = COALESCE(?, developer),
                            publisher = COALESCE(?, publisher),
                            genre = COALESCE(?, genre),
                            release_date = COALESCE(?, release_date),
                            openvgdb_release_id = ?,
                            metadata_source = COALESCE(metadata_source, 'openvgdb'),
                            updated_at = CURRENT_TIMESTAMP
                        WHERE id = ?
                        "#
                    )
                    .bind(&data.description)
                    .bind(&data.developer)
                    .bind(&data.publisher)
                    .bind(&data.genre)
                    .bind(&data.release_date)
                    .bind(data.release_id)
                    .bind(&m.game_id)
                    .execute(&mut *tx)
                    .await?;

                    updated += 1;
                }
            }
        }

        tx.commit().await?;
        pb.inc(chunk.len() as u64);
    }

    pb.finish_with_message("Done");
    pool.close().await;

    Ok(updated)
}

/// Create a nice progress bar style
fn create_progress_style() -> ProgressStyle {
    ProgressStyle::default_bar()
        .template("{spinner:.green} [{elapsed_precise}] [{bar:40.cyan/blue}] {pos}/{len} ({per_sec}) ETA: {eta} {msg}")
        .unwrap()
        .progress_chars("#>-")
}

/// Main enrichment function
pub async fn enrich_database(
    database_path: &Path,
    openvgdb_path: &Path,
    threshold: f64,
    dry_run: bool,
) -> Result<()> {
    println!("Database Enrichment");
    println!("===================");
    println!("Games DB:  {}", database_path.display());
    println!("OpenVGDB:  {}", openvgdb_path.display());
    println!("Threshold: {:.0}%", threshold * 100.0);
    println!("Dry run:   {}", dry_run);
    println!();

    // Verify paths exist
    if !database_path.exists() {
        anyhow::bail!("Games database not found: {}", database_path.display());
    }
    if !openvgdb_path.exists() {
        anyhow::bail!("OpenVGDB not found: {}", openvgdb_path.display());
    }

    // Load both databases
    print!("Loading LibRetro games... ");
    std::io::Write::flush(&mut std::io::stdout())?;
    let libretro_games = load_libretro_games(database_path)
        .await
        .context("Failed to load LibRetro games")?;
    println!("{} games", libretro_games.len());

    print!("Loading OpenVGDB games... ");
    std::io::Write::flush(&mut std::io::stdout())?;
    let openvgdb_games = load_openvgdb_games(openvgdb_path)
        .await
        .context("Failed to load OpenVGDB games")?;
    println!("{} games", openvgdb_games.len());

    // Build lookup maps
    print!("Building match indexes... ");
    std::io::Write::flush(&mut std::io::stdout())?;
    let maps = build_match_maps(openvgdb_games);
    println!("done");
    println!("  CRC index:   {:>6} entries", maps.by_crc.len());
    println!("  Title index: {:>6} entries", maps.by_title.len());
    println!("  Word index:  {:>6} entries", maps.by_word.len());
    println!();

    // Find matches with progress
    println!("Finding matches...");
    let pb = ProgressBar::new(libretro_games.len() as u64);
    pb.set_style(create_progress_style());
    pb.set_message("Matching");

    let mut matches: Vec<MatchResult> = Vec::new();
    let mut crc_matches = 0;
    let mut title_matches = 0;
    let mut fuzzy_matches = 0;
    let mut no_match = 0;

    for game in &libretro_games {
        if let Some(m) = find_best_match(game, &maps, threshold) {
            match m.match_type {
                MatchType::ExactCrc => crc_matches += 1,
                MatchType::ExactTitle => title_matches += 1,
                MatchType::FuzzyTitle => fuzzy_matches += 1,
            }
            matches.push(m);
        } else {
            no_match += 1;
        }
        pb.inc(1);
    }

    pb.finish_with_message("Complete");
    println!();

    println!("Match Results:");
    println!("  CRC matches:   {:>6} ({:.1}%)", crc_matches, 100.0 * crc_matches as f64 / libretro_games.len() as f64);
    println!("  Title matches: {:>6} ({:.1}%)", title_matches, 100.0 * title_matches as f64 / libretro_games.len() as f64);
    println!("  Fuzzy matches: {:>6} ({:.1}%)", fuzzy_matches, 100.0 * fuzzy_matches as f64 / libretro_games.len() as f64);
    println!("  No match:      {:>6} ({:.1}%)", no_match, 100.0 * no_match as f64 / libretro_games.len() as f64);
    println!("  ─────────────────────────");
    println!("  Total matched: {:>6} / {} ({:.1}%)",
        matches.len(),
        libretro_games.len(),
        100.0 * matches.len() as f64 / libretro_games.len() as f64
    );
    println!();

    // Show some sample fuzzy matches for verification
    let fuzzy_samples: Vec<_> = matches.iter()
        .filter(|m| matches!(m.match_type, MatchType::FuzzyTitle))
        .take(10)
        .collect();

    if !fuzzy_samples.is_empty() {
        println!("Sample fuzzy matches (for verification):");
        for m in &fuzzy_samples {
            println!("  {:.0}% \"{}\"", m.similarity * 100.0, m.game_title);
            println!("      -> \"{}\"", m.openvgdb_title);
        }
        println!();
    }

    if dry_run {
        println!("[Dry run] Would update {} games with metadata", matches.len());
    } else {
        println!("Applying metadata updates...");
        let updated = apply_matches(database_path, openvgdb_path, &matches).await?;
        println!();
        println!("Updated {} games with metadata", updated);
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_normalize_title() {
        assert_eq!(normalize_title("Super Mario Bros. (USA)"), "super mario bros");
        assert_eq!(normalize_title("The Legend of Zelda (Europe) [!]"), "legend of zelda");
        assert_eq!(normalize_title("Sonic the Hedgehog 2 (World)"), "sonic hedgehog 2");
        assert_eq!(normalize_title("Street Fighter II: Champion Edition"), "street fighter ii champion edition");
    }

    #[test]
    fn test_similarity_ratio() {
        assert_eq!(similarity_ratio("hello", "hello"), 1.0);
        assert!(similarity_ratio("hello", "hallo") > 0.7);
        assert!(similarity_ratio("super mario", "super mario bros") > 0.7);
        assert!(similarity_ratio("completely different", "nothing alike") < 0.5);
    }

    #[test]
    fn test_extract_words() {
        let words = extract_words("super mario bros");
        assert!(words.contains("super"));
        assert!(words.contains("mario"));
        assert!(words.contains("bros"));
        assert_eq!(words.len(), 3);
    }
}
