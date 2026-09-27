//! Non-retail release evidence from the existing, offline Libretro catalog.
//! Never infer homebrew from a game's age, developer name, or free-text notes.
use std::collections::{HashMap, HashSet};
use std::fs;
use std::io::Write;
use std::path::Path;
use std::path::PathBuf;
use std::sync::{Mutex, OnceLock};

use anyhow::Result;
use rusqlite::Connection;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

static INDEX: OnceLock<ReleaseIndex> = OnceLock::new();
static LOAD_LOCK: Mutex<()> = Mutex::new(());

fn release_label(value: &str) -> bool {
    let value = value.trim();
    [
        "homebrew",
        "home brew",
        "home-brew",
        "hb",
        "rom hack",
        "rom-hack",
        "rom_hack",
        "romhack",
        "hack",
        "hacked",
        "mod",
        "unlicensed",
        "unl",
        "aftermarket",
        "pirate",
        "pirated",
        "bootleg",
    ]
    .iter()
    .any(|label| value.eq_ignore_ascii_case(label))
}

fn tags(value: &str) -> impl Iterator<Item = &str> {
    // Splitting on both delimiter kinds also handles nested MAME qualifiers.
    value
        .split(['(', '['])
        .skip(1)
        .filter_map(|part| part.find([')', ']']).map(|end| part[..end].trim()))
}

fn tag_is_non_retail(tag: &str) -> bool {
    if release_label(tag) {
        return true;
    }
    let lower = tag.to_ascii_lowercase();
    if lower
        .split(|c: char| !c.is_ascii_alphanumeric())
        .any(|word| {
            matches!(
                word,
                "homebrew"
                    | "hb"
                    | "hack"
                    | "hacked"
                    | "bootleg"
                    | "pirate"
                    | "pirated"
                    | "unlicensed"
                    | "unl"
                    | "aftermarket"
            )
        })
    {
        return true;
    }
    false
}

fn has_hacked_dump_tag(title: &str) -> bool {
    // GoodTools/TOSEC: [h], [h1], [h2 XYZ]. Do not confuse [a], [b],
    // [t], [f], demos or prototypes with independently authored games.
    title
        .split('[')
        .skip(1)
        .filter_map(|part| part.find(']').map(|end| &part[..end]))
        .any(|tag| {
            let lower = tag.trim().to_ascii_lowercase();
            let token = lower.split_whitespace().next().unwrap_or_default();
            token
                .strip_prefix('h')
                .is_some_and(|rest| rest.bytes().all(|b| b.is_ascii_digit()))
        })
}

pub fn explicit(title: &str, release_type: Option<&str>, version: Option<&str>) -> bool {
    release_type.is_some_and(|value| value.split([';', '/', ',']).any(release_label))
        || tags(title).any(tag_is_non_retail)
        || has_hacked_dump_tag(title)
        || version.is_some_and(|value| tag_is_non_retail(value) || has_hacked_dump_tag(value))
}

fn family_label(title: &str) -> bool {
    // A hacked dump is not evidence that the original title is homebrew.
    // Only independent/unlicensed release labels may identify an untagged title.
    tags(title).any(|tag| {
        matches!(
            tag.to_ascii_lowercase().as_str(),
            "homebrew" | "home brew" | "hb" | "unlicensed" | "unl" | "aftermarket" | "pirate"
        )
    })
}

fn platform_key(platform: &str) -> String {
    let platform = platform.trim();
    if crate::arcade::is_arcade_family_platform(platform)
        || platform == "HBMAME"
        || platform == "FBNeo - Arcade Games"
        || platform == "MAME"
        || platform.starts_with("MAME ")
    {
        return "arcade".into();
    }
    crate::media::libretro_platform_name(platform)
        .unwrap_or(platform)
        .to_lowercase()
}

fn title_key(title: &str) -> String {
    if title.is_ascii() {
        let mut key = String::with_capacity(title.len());
        for byte in title.bytes() {
            if byte.is_ascii_alphanumeric() {
                key.push(byte.to_ascii_lowercase() as char);
            }
        }
        return key;
    }
    title
        .chars()
        .flat_map(char::to_lowercase)
        .filter(|c| c.is_alphanumeric())
        .collect()
}

fn family_key(title: &str) -> String {
    if title.is_ascii() {
        let mut key = String::with_capacity(title.len());
        let mut depth = 0_u32;
        for byte in title.bytes() {
            match byte {
                b'(' | b'[' => depth += 1,
                b')' | b']' => depth = depth.saturating_sub(1),
                _ if depth == 0 && byte.is_ascii_alphanumeric() => {
                    key.push(byte.to_ascii_lowercase() as char)
                }
                _ => (),
            }
        }
        return key;
    }
    let mut depth = 0_u32;
    let plain: String = title
        .chars()
        .filter(|c| match c {
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
    title_key(&plain)
}

fn rom_stem(file: &str) -> String {
    Path::new(file)
        .file_stem()
        .and_then(|v| v.to_str())
        .unwrap_or(file)
        .to_lowercase()
}

#[derive(Default, Deserialize, Serialize)]
struct PlatformIndex {
    exact: HashSet<String>,
    families: HashSet<String>,
    romsets: HashSet<String>,
}

#[derive(Default, Deserialize, Serialize)]
struct ReleaseIndex {
    platforms: HashMap<String, PlatformIndex>,
}

impl ReleaseIndex {
    fn load(connection: &Connection) -> Result<Self> {
        let mut index = Self::default();
        let mut databases = HashMap::new();
        let mut statement = connection.prepare("SELECT id, source_name FROM libretro_databases")?;
        for row in statement.query_map([], |row| {
            Ok((row.get::<_, i64>(0)?, row.get::<_, String>(1)?))
        })? {
            let (id, source) = row?;
            databases.insert(id, (platform_key(&source), source));
        }
        // HBMAME carries original MAME parent sets as dependencies. Their
        // presence in that database is not a homebrew/hack classification.
        let mut original_romsets = HashSet::new();
        let mut statement = connection.prepare(
            "SELECT r.title, r.file_name FROM libretro_records r
             JOIN libretro_databases d ON d.id=r.database_id
             WHERE d.source_name='MAME' AND r.title IS NOT NULL AND r.file_name IS NOT NULL",
        )?;
        for row in statement.query_map([], |row| {
            Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?))
        })? {
            let (title, file) = row?;
            if !explicit(&title, None, None) {
                original_romsets.insert(rom_stem(&file));
            }
        }

        let mut statement = connection.prepare(
            "SELECT database_id, title, coalesce(file_name, '') FROM libretro_records WHERE title IS NOT NULL ORDER BY rowid")?;
        // First collect positive evidence, then remove ambiguous title aliases
        // using clean releases. This makes results independent of database order
        // without retaining every ROM record in memory.
        let mut classifications = Vec::new();
        for positive_pass in [true, false] {
            let mut rows = statement.query([])?;
            let mut ordinal = 0;
            while let Some(row) = rows.next()? {
                let row_ordinal = ordinal;
                ordinal += 1;
                let id: i64 = row.get(0)?;
                let (platform, source) = databases
                    .get(&id)
                    .ok_or_else(|| anyhow::anyhow!("ROM metadata refers to a missing database"))?;
                if !positive_pass && !index.platforms.contains_key(platform) {
                    continue;
                }
                let title = row.get_ref(1)?.as_str()?;
                let file = row.get_ref(2)?.as_str()?;
                let hbmame = positive_pass
                    && source == "HBMAME"
                    && !file.is_empty()
                    && !original_romsets.contains(&rom_stem(file));
                let non_retail = if positive_pass {
                    let classified =
                        explicit(title, None, None) || explicit(file, None, None) || hbmame;
                    classifications.push(classified);
                    classified
                } else {
                    classifications[row_ordinal]
                };
                if positive_pass && non_retail {
                    let entry = index.platforms.entry(platform.clone()).or_default();
                    entry.exact.insert(title_key(title));
                    if family_label(title)
                        || family_label(file)
                        || (hbmame && !title.contains(['(', '[']))
                    {
                        entry.families.insert(family_key(title));
                    }
                    if platform == "arcade" && !file.is_empty() {
                        entry.romsets.insert(rom_stem(file));
                    }
                } else if !positive_pass && !non_retail {
                    if let Some(entry) = index.platforms.get_mut(platform) {
                        entry.exact.remove(&title_key(title));
                        entry.families.remove(&family_key(title));
                        if platform == "arcade" {
                            entry.romsets.remove(&rom_stem(file));
                        }
                    }
                }
            }
        }
        Ok(index)
    }

    fn is_non_retail(&self, title: &str, platform: &str) -> bool {
        let Some(index) = self.platforms.get(&platform_key(platform)) else {
            return false;
        };
        let exact = title_key(title);
        let family = family_key(title);
        index.romsets.contains(&rom_stem(title))
            || (exact != family && index.exact.contains(&exact))
            || index.families.contains(&family)
    }
}

fn cache_path(connection: &Connection) -> Option<PathBuf> {
    // Only persistent, unmodified metadata databases are cached. Memory-backed
    // fixtures bypass this path. A new import or classifier version invalidates it.
    let database = Path::new(connection.path()?).canonicalize().ok()?;
    let metadata = database.metadata().ok()?;
    let modified = metadata
        .modified()
        .ok()?
        .duration_since(std::time::UNIX_EPOCH)
        .ok()?
        .as_nanos();
    let identity = format!("v1\n{}\n{}\n{modified}", database.display(), metadata.len());
    let hash = format!("{:x}", Sha256::digest(identity.as_bytes()));
    Some(
        crate::app_paths::project_dirs()?
            .cache_dir()
            .join("release-content")
            .join(format!("{hash}.json")),
    )
}

fn read_cache(path: &Path) -> Option<ReleaseIndex> {
    if path.metadata().ok()?.len() > 16 * 1024 * 1024 {
        return None;
    }
    serde_json::from_slice(&fs::read(path).ok()?).ok()
}

fn write_cache(path: &Path, index: &ReleaseIndex) -> Result<()> {
    let parent = path
        .parent()
        .ok_or_else(|| anyhow::anyhow!("Missing metadata cache directory"))?;
    fs::create_dir_all(parent)?;
    let mut file = tempfile::NamedTempFile::new_in(parent)?;
    file.write_all(&serde_json::to_vec(index)?)?;
    file.persist(path)?;
    Ok(())
}

pub fn initialize(connection: &Connection) -> Result<()> {
    if INDEX.get().is_some() {
        return Ok(());
    }
    let _guard = LOAD_LOCK
        .lock()
        .map_err(|_| anyhow::anyhow!("Release metadata lock unavailable"))?;
    if INDEX.get().is_none() {
        let start = std::time::Instant::now();
        let path = cache_path(connection);
        let cached = path.as_deref().and_then(read_cache);
        let from_cache = cached.is_some();
        let index = if let Some(index) = cached {
            index
        } else {
            let index = ReleaseIndex::load(connection)?;
            if let Some(path) = path.as_deref() {
                if let Err(error) = write_cache(path, &index) {
                    eprintln!("LUNCHPAIL_NON_RETAIL_CACHE_UNAVAILABLE {error:#}");
                }
            }
            index
        };
        println!(
            "LUNCHPAIL_NON_RETAIL_METADATA platforms={} exact_titles={} families={} cached={from_cache} elapsed_ms={}",
            index.platforms.len(),
            index
                .platforms
                .values()
                .map(|p| p.exact.len())
                .sum::<usize>(),
            index
                .platforms
                .values()
                .map(|p| p.families.len())
                .sum::<usize>(),
            start.elapsed().as_millis()
        );
        let _ = INDEX.set(index);
    }
    Ok(())
}

pub fn is_non_retail(
    title: &str,
    platform: &str,
    release_type: Option<&str>,
    version: Option<&str>,
) -> bool {
    explicit(title, release_type, version)
        || INDEX
            .get()
            .is_some_and(|index| index.is_non_retail(title, platform))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn recognizes_release_metadata_and_qualified_tags() {
        for title in [
            "A (HB)",
            "A [Homebrew]",
            "A (2024 Homebrew)",
            "A (Speedup Hack)",
            "A [h]",
            "A [h2 Author]",
            "A [Hack by Author]",
            "A (bootleg, set 2)",
            "A (Unlicensed)",
        ] {
            assert!(explicit(title, None, None), "{title}");
        }
        for kind in [
            "Homebrew",
            "ROM Hack",
            "ROM-Hack",
            "Unlicensed",
            "Aftermarket",
            "Pirate",
            "mod",
        ] {
            assert!(explicit("Ordinary title", Some(kind), None), "{kind}");
        }
        assert!(explicit("Ordinary title", None, Some("Homebrew")));
    }

    #[test]
    fn does_not_classify_prototypes_dump_quality_or_title_words() {
        for title in [
            "Hack and Slash",
            "Hacker",
            "The Pirates",
            "Homebrew Software",
            "Game (Proto)",
            "Game (Demo)",
            "Game [b1]",
            "Game [a]",
            "Game [t1]",
            "Game [f]",
            "Game (USA) (Rev 1)",
            "Game (World) (Hannover)",
            "Game (H)",
        ] {
            assert!(!explicit(title, Some("Released"), None), "{title}");
        }
    }

    fn fixture() -> Connection {
        let connection = Connection::open_in_memory().unwrap();
        connection.execute_batch("CREATE TABLE libretro_databases(id INTEGER, source_name TEXT);
            CREATE TABLE libretro_records(database_id INTEGER, title TEXT, file_name TEXT);
            INSERT INTO libretro_databases VALUES(1,'MAME'),(2,'HBMAME'),(3,'Nintendo - Nintendo Entertainment System');
            INSERT INTO libretro_records VALUES
            (1,'Original (World)','original.zip'),
            (2,'Original','original.zip'),
            (2,'Original (Unusual remix)','remix.zip'),
            (2,'New Arcade Creation','newarc.zip'),
            (3,'Independent Adventure (World) (Homebrew)','indie.nes'),
            (3,'Famous Game (USA)','famous.nes'),
            (3,'Famous Game (USA) [h1]','famous-h.nes'),
            (3,'Mixed Game (World) (Unl)','mixed-unl.nes'),
            (3,'Mixed Game (USA)','mixed.nes');").unwrap();
        connection
    }

    #[test]
    fn metadata_recovers_untagged_homebrew_but_preserves_official_parents() {
        let index = ReleaseIndex::load(&fixture()).unwrap();
        for title in ["Original", "Original (World)", "original.zip"] {
            assert!(!index.is_non_retail(title, "Arcade"), "{title}");
        }
        for title in [
            "New Arcade Creation",
            "newarc.zip",
            "remix.zip",
            "Original (Unusual remix)",
        ] {
            assert!(index.is_non_retail(title, "Arcade"), "{title}");
        }
        assert!(index.is_non_retail("Independent Adventure", "Nintendo Entertainment System"));
        assert!(!index.is_non_retail("Independent Adventure", "Windows"));
        for title in ["Famous Game", "Famous Game (USA)", "Mixed Game"] {
            assert!(
                !index.is_non_retail(title, "Nintendo Entertainment System"),
                "{title}"
            );
        }
    }

    #[test]
    fn cached_classification_preserves_results_and_rejects_partial_writes() {
        let index = ReleaseIndex::load(&fixture()).unwrap();
        let directory = tempfile::tempdir().unwrap();
        let path = directory.path().join("metadata.json");
        write_cache(&path, &index).unwrap();
        let cached = read_cache(&path).unwrap();
        assert!(cached.is_non_retail("Independent Adventure", "Nintendo Entertainment System"));
        assert!(!cached.is_non_retail("Original", "Arcade"));
        fs::write(&path, b"{incomplete").unwrap();
        assert!(read_cache(&path).is_none());
        assert!(cache_path(&fixture()).is_none());
    }

    #[test]
    #[ignore = "requires local catalog; set LUNCHPAIL_DATABASE and LUNCHPAIL_GAMES_DATABASE"]
    fn live_catalog_classification() {
        let path = crate::catalog::requested_database_path().unwrap();
        let catalog = crate::catalog::load(&path).unwrap();
        assert!(INDEX.get().is_some());
        let mut platforms = HashMap::<&str, usize>::new();
        for game in catalog.games.iter().filter(|g| g.non_retail) {
            *platforms.entry(&game.platform).or_default() += 1;
        }
        for title in [
            "Metal Slug 2",
            "Metal Slug X",
            "Street Fighter II",
            "Pac-Man",
        ] {
            for game in catalog
                .games
                .iter()
                .filter(|g| g.platform == "Arcade" && g.title == title)
            {
                assert!(
                    !game.non_retail,
                    "official arcade game incorrectly classified: {title}"
                );
            }
        }
        let speedup = catalog
            .games
            .iter()
            .find(|g| g.title == "Pac-Man (Speedup Hack)")
            .unwrap();
        assert!(speedup.non_retail);
        println!(
            "NON_RETAIL_COVERAGE games={} non_retail={} mixed_families={} platforms={}",
            catalog.games.len(),
            catalog.games.iter().filter(|g| g.non_retail).count(),
            catalog
                .games
                .iter()
                .filter(|g| !g.non_retail && g.has_non_retail_release)
                .count(),
            platforms.len()
        );
    }
}
