//! Arcade adult-content metadata. Provider data is downloaded to the user's
//! cache, not redistributed in the executable or inferred from game genres.
use std::collections::{HashMap, HashSet};
use std::fs;
use std::io::{Read, Write};
use std::path::PathBuf;
use std::sync::{Mutex, OnceLock};
use std::time::Duration;

use anyhow::{Context, Result, bail};
use directories::ProjectDirs;
use rusqlite::Connection;
use sha2::{Digest, Sha256};

const SOURCE_URL: &str = "https://raw.githubusercontent.com/AntoPISA/MAME_SupportFiles/bca9d8a74079f74a4a40298e06bf1634c820c7cb/catver.ini/mature.ini";
const SOURCE_SHA256: &str = "172af9967a614679bca756e74fc4254aa6f5fc3a45265dac0475622bbdba7d53";
const MAX_SOURCE_BYTES: u64 = 1024 * 1024;
static INDEX: OnceLock<AdultIndex> = OnceLock::new();
static LOAD_LOCK: Mutex<()> = Mutex::new(());

#[derive(Default)]
struct AdultIndex {
    romsets: HashSet<String>,
    exact_titles: HashMap<String, bool>,
    family_titles: HashSet<String>,
    matched_romsets: HashSet<String>,
}

impl AdultIndex {
    fn new(source: &str) -> Result<Self> {
        let mut romsets = HashSet::new();
        let mut in_root = false;
        for line in source.lines().map(str::trim) {
            if line.starts_with('[') {
                in_root = line == "[ROOT_FOLDER]";
            } else if in_root && !line.is_empty() && !line.starts_with(';') {
                if !line
                    .bytes()
                    .all(|byte| byte.is_ascii_alphanumeric() || byte == b'_')
                {
                    bail!("Invalid ROM-set name in arcade adult-content metadata");
                }
                romsets.insert(line.to_ascii_lowercase());
            }
        }
        if romsets.is_empty() {
            bail!("Arcade adult-content metadata has no ROM sets");
        }
        Ok(Self {
            romsets,
            ..Self::default()
        })
    }

    fn add_title(&mut self, title: &str, romset: &str) {
        let romset = romset.to_ascii_lowercase();
        let adult = self.romsets.contains(&romset);
        // Preserve a known clean revision when the title identifies it exactly;
        // an unqualified family is adult if any included revision is adult.
        let exact = title_key(title);
        if !exact.is_empty() {
            self.exact_titles
                .entry(exact)
                .and_modify(|value| *value |= adult)
                .or_insert(adult);
        }
        if adult {
            self.matched_romsets.insert(romset);
            self.family_titles.insert(family_key(title));
        }
    }

    fn is_adult(&self, title: &str) -> bool {
        let stem = title
            .trim()
            .trim_end_matches(".zip")
            .trim_end_matches(".7z")
            .to_ascii_lowercase();
        if self.romsets.contains(&stem) {
            return true;
        }
        let exact = title_key(title);
        // A parenthetical suffix is a concrete release, not a family request.
        if title.contains('(') || title.contains('[') {
            if let Some(adult) = self.exact_titles.get(&exact) {
                return *adult;
            }
        }
        self.exact_titles.get(&exact).copied().unwrap_or_default()
            || self.family_titles.contains(&family_key(title))
    }
}

fn title_key(title: &str) -> String {
    title
        .chars()
        .flat_map(char::to_lowercase)
        .filter(|c| c.is_alphanumeric())
        .collect()
}

fn family_key(title: &str) -> String {
    let mut depth = 0_u32;
    let base: String = title
        .chars()
        .filter(|character| match character {
            '(' | '[' => {
                depth += 1;
                false
            }
            ')' | ']' => {
                depth = depth.saturating_sub(1);
                false
            }
            _ => depth == 0,
        })
        .collect();
    title_key(&base)
}

fn source_cache_path() -> Result<PathBuf> {
    let dirs = crate::app_paths::project_dirs().context("Finding the Lunchpail metadata cache")?;
    Ok(dirs
        .cache_dir()
        .join("arcade-content")
        .join(format!("{SOURCE_SHA256}-mature.ini")))
}

fn source_is_valid(bytes: &[u8]) -> bool {
    format!("{:x}", Sha256::digest(bytes)) == SOURCE_SHA256
}

fn load_source() -> Result<String> {
    let path = source_cache_path()?;
    if let Ok(bytes) = fs::read(&path) {
        if source_is_valid(&bytes) {
            return String::from_utf8(bytes)
                .context("Reading cached arcade adult-content metadata");
        }
    }
    if !crate::media::media_retrieval_enabled() {
        bail!("Arcade adult-content metadata has not been cached (offline mode)");
    }
    let agent: ureq::Agent = ureq::Agent::config_builder()
        .timeout_connect(Some(Duration::from_secs(3)))
        .timeout_global(Some(Duration::from_secs(10)))
        .build()
        .into();
    let mut bytes = Vec::new();
    agent
        .get(SOURCE_URL)
        .call()?
        .body_mut()
        .as_reader()
        .take(MAX_SOURCE_BYTES + 1)
        .read_to_end(&mut bytes)?;
    if bytes.len() as u64 > MAX_SOURCE_BYTES || !source_is_valid(&bytes) {
        bail!("Arcade adult-content metadata failed its pinned SHA-256 check");
    }
    let parent = path
        .parent()
        .context("Finding the arcade metadata cache directory")?;
    fs::create_dir_all(parent)?;
    let mut temporary = tempfile::NamedTempFile::new_in(parent)?;
    temporary.write_all(&bytes)?;
    temporary.persist(&path)?;
    String::from_utf8(bytes).context("Decoding arcade adult-content metadata")
}

/// Called only from background catalog loading, never from a filter/paint path.
/// Cache failures are not memoized, so the next catalog refresh can retry.
pub fn initialize(connection: &Connection) -> Result<()> {
    if INDEX.get().is_some() {
        return Ok(());
    }
    let _guard = LOAD_LOCK
        .lock()
        .map_err(|_| anyhow::anyhow!("Arcade metadata lock unavailable"))?;
    if INDEX.get().is_some() {
        return Ok(());
    }
    let mut index = AdultIndex::new(&load_source()?)?;
    let mut statement = connection.prepare(
        "SELECT DISTINCT r.title, r.file_name FROM libretro_records r
         JOIN libretro_databases d ON d.id = r.database_id
         WHERE (d.source_name = 'MAME' OR d.source_name LIKE 'MAME %'
                OR d.source_name = 'FBNeo - Arcade Games')
           AND r.title IS NOT NULL AND r.file_name IS NOT NULL",
    )?;
    let rows = statement.query_map([], |row| {
        Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?))
    })?;
    for row in rows {
        let (title, file) = row?;
        let romset = file
            .strip_suffix(".zip")
            .or_else(|| file.strip_suffix(".7z"))
            .unwrap_or(&file);
        index.add_title(&title, romset);
    }
    // Optional local LaunchBox data supplies additional exact display-name
    // aliases. Classification itself does not depend on that private export.
    for entry in crate::arcade::ARCADE_LOOKUP {
        if !entry.preferred_lookup.is_empty() {
            index.add_title(entry.title, entry.preferred_lookup);
        }
    }
    println!(
        "LUNCHPAIL_ARCADE_ADULT_METADATA sets={} matched_sets={} title_families={} source=Mature.ini-0.289",
        index.romsets.len(),
        index.matched_romsets.len(),
        index.family_titles.len()
    );
    let _ = INDEX.set(index);
    Ok(())
}

pub fn is_adult(title: &str, platform: &str) -> bool {
    // Never apply an arcade title match to unrelated console/computer games.
    let arcade = crate::arcade::is_arcade_family_platform(platform)
        || matches!(
            platform.trim(),
            "Sammy Atomiswave"
                | "Sega Naomi"
                | "Sega Naomi 2"
                | "SNK Neo Geo MVS"
                | "Sega Model 2"
                | "Sega Model 3"
                | "Sega ST-V"
                | "Taito Type X"
                | "Capcom CPS-1"
                | "Capcom CPS-2"
                | "Capcom CPS-3"
        );
    arcade && INDEX.get().is_some_and(|index| index.is_adult(title))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn fixture() -> AdultIndex {
        let mut index = AdultIndex::new("[FOLDER_SETTINGS]\nRootFolderIcon mame\n[ROOT_FOLDER]\nexcelsr\nexcelsra\ngalpanic\npuzzadult\n").unwrap();
        index.add_title("Excelsior (set 1)", "excelsr");
        index.add_title("Excelsior (set 2)", "excelsra");
        index.add_title("Gals Panic (World)", "galpanic");
        index.add_title("Example Puzzle (Adult)", "puzzadult");
        index.add_title("Example Puzzle (Clean)", "puzzclean");
        index.add_title("Mahjong Example", "mjexample");
        index
    }

    #[test]
    fn recognizes_titles_romsets_revisions_and_punctuation() {
        let index = fixture();
        for title in [
            "Excelsior",
            "Excelsior (set 2)",
            "excelsr.zip",
            "Gals Panic",
            "Gals-Panic (Japan)",
        ] {
            assert!(index.is_adult(title), "{title}");
        }
    }

    #[test]
    fn does_not_guess_from_genres_prefixes_or_substrings() {
        let index = fixture();
        for title in [
            "Mahjong Example",
            "Excelsior Phase One: Lysandia",
            "Gals Panic Sequel",
            "Street Fighter II",
        ] {
            assert!(!index.is_adult(title), "{title}");
        }
        assert!(!is_adult("Excelsior", "MS-DOS"));
    }

    #[test]
    fn mixed_family_is_adult_but_known_clean_release_is_not() {
        let index = fixture();
        assert!(index.is_adult("Example Puzzle"));
        assert!(index.is_adult("Example Puzzle (Adult)"));
        assert!(!index.is_adult("Example Puzzle (Clean)"));
    }

    #[test]
    fn rejects_invalid_or_empty_metadata_and_checks_checksum() {
        assert!(AdultIndex::new("<html>error</html>").is_err());
        assert!(AdultIndex::new("[ROOT_FOLDER]\n../game").is_err());
        assert!(!source_is_valid(b"incomplete download"));
    }

    #[test]
    #[ignore = "requires the local catalog and downloaded provider metadata"]
    fn live_catalog_classification_covers_arcade_library() {
        let path = crate::catalog::requested_database_path().expect("local catalog");
        let catalog = crate::catalog::load(&path).unwrap();
        let index = INDEX
            .get()
            .expect("adult metadata initialized successfully");
        let arcade: Vec<_> = catalog
            .games
            .iter()
            .filter(|game| crate::arcade::is_arcade_family_platform(&game.platform))
            .collect();
        let adults: Vec<_> = arcade.iter().filter(|game| game.adult).collect();
        let newly_classified: Vec<_> = adults
            .iter()
            .filter(|game| !crate::catalog::is_adult_game(&game.title, None, None))
            .collect();
        let excelsior = arcade
            .iter()
            .find(|game| game.title == "Excelsior")
            .expect("Excelsior in arcade catalog");
        assert!(excelsior.adult);
        assert!(!is_adult("Excelsior", "MS-DOS"));
        assert!(index.romsets.len() > 700);
        assert!(adults.len() > 300);
        println!(
            "ARCADE_ADULT_COVERAGE arcade_games={} adult_games={} non_keyword_titles={} source_romsets={} matched_romsets={}",
            arcade.len(),
            adults.len(),
            newly_classified.len(),
            index.romsets.len(),
            index.matched_romsets.len()
        );
        println!(
            "ARCADE_ADULT_EXAMPLES {:?}",
            newly_classified
                .iter()
                .take(20)
                .map(|game| &game.title)
                .collect::<Vec<_>>()
        );
        println!(
            "ARCADE_ADULT_UNMATCHED_ROMSETS {:?}",
            index
                .romsets
                .difference(&index.matched_romsets)
                .collect::<Vec<_>>()
        );
    }
}
