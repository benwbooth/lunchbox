use std::env;
use std::path::{Path, PathBuf};

use anyhow::{Context, Result, bail};
use directories::ProjectDirs;
use rusqlite::{Connection, OpenFlags};

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Game {
    pub id: String,
    pub title: String,
    pub platform: String,
    pub status: String,
    pub local: bool,
    pub downloadable: bool,
    search_key: String,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Platform {
    pub name: String,
    pub game_count: usize,
}

#[derive(Clone, Debug, Default)]
pub struct Catalog {
    pub games: Vec<Game>,
    pub platforms: Vec<Platform>,
    pub local_file_count: usize,
    pub offer_count: usize,
    pub emulator_count: usize,
}

#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct Filter {
    pub search: String,
    pub platform: String,
    pub availability: String,
}

pub fn requested_database_path() -> Option<PathBuf> {
    let mut arguments = env::args_os().skip(1);
    while let Some(argument) = arguments.next() {
        if argument == "--database" {
            return arguments.next().map(PathBuf::from);
        }
        if let Some(argument) = argument.to_str()
            && let Some(path) = argument.strip_prefix("--database=")
        {
            return Some(PathBuf::from(path));
        }
    }

    env::var_os("LUNCHBOX_DATABASE")
        .filter(|path| !path.is_empty())
        .map(PathBuf::from)
        .or_else(|| {
            [
                PathBuf::from("build/lunchbox.db"),
                PathBuf::from("lunchbox.db"),
            ]
            .into_iter()
            .find(|path| path.is_file())
        })
        .or_else(|| {
            let executable = env::current_exe().ok()?;
            executable
                .parent()?
                .join("../share/lunchbox/lunchbox.db")
                .canonicalize()
                .ok()
        })
        .or_else(|| {
            ProjectDirs::from("com", "Lunchbox", "Lunchbox")
                .map(|dirs| dirs.data_dir().join("lunchbox.db"))
                .filter(|path| path.is_file())
        })
}

pub fn load(path: &Path) -> Result<Catalog> {
    let connection = Connection::open_with_flags(
        path,
        OpenFlags::SQLITE_OPEN_READ_ONLY | OpenFlags::SQLITE_OPEN_NO_MUTEX,
    )
    .with_context(|| format!("opening Lunchbox database {}", path.display()))?;
    connection.busy_timeout(std::time::Duration::from_secs(2))?;
    connection.execute_batch(
        "PRAGMA foreign_keys = ON;\n\
         PRAGMA query_only = ON;\n\
         PRAGMA trusted_schema = OFF;\n\
         PRAGMA temp_store = MEMORY;\n\
         PRAGMA cache_size = -16384;\n\
         PRAGMA mmap_size = 268435456;",
    )?;

    let schema_version: i64 = connection
        .query_row(
            "SELECT coalesce(max(version), 0) FROM schema_migrations",
            [],
            |row| row.get(0),
        )
        .context("reading Lunchbox schema version")?;
    if schema_version != 3 {
        bail!("unsupported Lunchbox schema version {schema_version}; expected version 3");
    }

    let mut statement = connection.prepare(
        "WITH local_games AS (\n\
             SELECT DISTINCT releases.game_id\n\
             FROM local_files\n\
             JOIN release_artifacts USING (artifact_id)\n\
             JOIN releases ON releases.id = release_artifacts.release_id\n\
             WHERE local_files.availability = 'present'\n\
         ), offer_games AS (\n\
             SELECT DISTINCT releases.game_id\n\
             FROM acquisition_offers\n\
             JOIN release_artifacts USING (artifact_id)\n\
             JOIN releases ON releases.id = release_artifacts.release_id\n\
             WHERE acquisition_offers.availability <> 'unavailable'\n\
         )\n\
         SELECT games.id, games.canonical_title,\n\
                coalesce(platforms.canonical_name, ''), games.status,\n\
                local_games.game_id IS NOT NULL, offer_games.game_id IS NOT NULL\n\
         FROM games\n\
         LEFT JOIN releases ON releases.id = (\n\
             SELECT candidate.id FROM releases AS candidate\n\
             WHERE candidate.game_id = games.id\n\
             ORDER BY candidate.status = 'canonical' DESC, candidate.id\n\
             LIMIT 1\n\
         )\n\
         LEFT JOIN platforms ON platforms.id = releases.platform_id\n\
         LEFT JOIN local_games ON local_games.game_id = games.id\n\
         LEFT JOIN offer_games ON offer_games.game_id = games.id\n\
         WHERE games.status NOT IN ('deprecated', 'merged')\n\
         ORDER BY coalesce(nullif(games.sort_title, ''), games.canonical_title) COLLATE NOCASE,\n\
                  games.id",
    )?;
    let game_rows = statement.query_map([], |row| {
        let title: String = row.get(1)?;
        let platform: String = row.get(2)?;
        Ok(Game {
            id: row.get(0)?,
            search_key: format!("{}\n{}", title.to_lowercase(), platform.to_lowercase()),
            title,
            platform,
            status: row.get(3)?,
            local: row.get(4)?,
            downloadable: row.get(5)?,
        })
    })?;
    let games = game_rows.collect::<rusqlite::Result<Vec<_>>>()?;

    let mut platform_statement = connection.prepare(
        "SELECT platforms.canonical_name, count(DISTINCT games.id)\n\
         FROM platforms\n\
         LEFT JOIN releases ON releases.platform_id = platforms.id\n\
         LEFT JOIN games ON games.id = releases.game_id\n\
                          AND games.status NOT IN ('deprecated', 'merged')\n\
         WHERE platforms.status <> 'deprecated'\n\
         GROUP BY platforms.id, platforms.canonical_name\n\
         ORDER BY platforms.canonical_name COLLATE NOCASE",
    )?;
    let platform_rows = platform_statement.query_map([], |row| {
        Ok(Platform {
            name: row.get(0)?,
            game_count: row.get(1)?,
        })
    })?;
    let platforms = platform_rows.collect::<rusqlite::Result<Vec<_>>>()?;

    Ok(Catalog {
        games,
        platforms,
        local_file_count: count(&connection, "local_files", "availability = 'present'")?,
        offer_count: count(
            &connection,
            "acquisition_offers",
            "availability <> 'unavailable'",
        )?,
        emulator_count: count(&connection, "emulators", "1")?,
    })
}

fn count(connection: &Connection, table: &str, predicate: &str) -> Result<usize> {
    let query = format!("SELECT count(*) FROM {table} WHERE {predicate}");
    let value: i64 = connection.query_row(&query, [], |row| row.get(0))?;
    usize::try_from(value).context("database count exceeded addressable memory")
}

pub fn filter_indices(catalog: &Catalog, filter: &Filter) -> Vec<usize> {
    let search = filter.search.trim().to_lowercase();
    catalog
        .games
        .iter()
        .enumerate()
        .filter(|(_, game)| {
            (search.is_empty() || game.search_key.contains(&search))
                && (filter.platform.is_empty() || game.platform == filter.platform)
                && match filter.availability.as_str() {
                    "local" => game.local,
                    "downloadable" => game.downloadable,
                    _ => true,
                }
        })
        .map(|(index, _)| index)
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn fixture_catalog() -> Catalog {
        Catalog {
            games: vec![
                Game {
                    id: "metroid".into(),
                    title: "Metroid".into(),
                    platform: "Nintendo Entertainment System".into(),
                    status: "canonical".into(),
                    local: true,
                    downloadable: false,
                    search_key: "metroid\nnintendo entertainment system".into(),
                },
                Game {
                    id: "outrun".into(),
                    title: "OutRun".into(),
                    platform: "Arcade".into(),
                    status: "canonical".into(),
                    local: false,
                    downloadable: true,
                    search_key: "outrun\narcade".into(),
                },
            ],
            ..Catalog::default()
        }
    }

    #[test]
    fn combines_search_platform_and_availability_without_fuzzy_identity_logic() {
        let catalog = fixture_catalog();
        assert_eq!(
            filter_indices(
                &catalog,
                &Filter {
                    search: "nintendo".into(),
                    platform: "Nintendo Entertainment System".into(),
                    availability: "local".into(),
                }
            ),
            vec![0]
        );
        assert!(
            filter_indices(
                &catalog,
                &Filter {
                    search: "metrooid".into(),
                    ..Filter::default()
                }
            )
            .is_empty()
        );
    }

    #[test]
    fn availability_filters_are_explicit() {
        let catalog = fixture_catalog();
        assert_eq!(
            filter_indices(
                &catalog,
                &Filter {
                    availability: "downloadable".into(),
                    ..Filter::default()
                }
            ),
            vec![1]
        );
    }

    #[test]
    fn loads_schema_three_catalog_with_linked_local_and_downloadable_state() {
        let directory = tempfile::tempdir().unwrap();
        let path = directory.path().join("catalog.db");
        let connection = Connection::open(&path).unwrap();
        connection
            .execute_batch(
                "CREATE TABLE schema_migrations (version INTEGER);\n\
                 INSERT INTO schema_migrations VALUES (3);\n\
                 CREATE TABLE games (\n\
                     id TEXT PRIMARY KEY, canonical_title TEXT, sort_title TEXT, status TEXT\n\
                 );\n\
                 CREATE TABLE platforms (id TEXT PRIMARY KEY, canonical_name TEXT, status TEXT);\n\
                 CREATE TABLE releases (\n\
                     id TEXT PRIMARY KEY, game_id TEXT, platform_id TEXT, status TEXT\n\
                 );\n\
                 CREATE TABLE release_artifacts (release_id TEXT, artifact_id TEXT);\n\
                 CREATE TABLE local_files (artifact_id TEXT, availability TEXT);\n\
                 CREATE TABLE acquisition_offers (artifact_id TEXT, availability TEXT);\n\
                 CREATE TABLE emulators (id TEXT PRIMARY KEY);\n\
                 INSERT INTO games VALUES ('game-1', 'Metroid', 'Metroid', 'canonical');\n\
                 INSERT INTO platforms VALUES ('nes', 'Nintendo Entertainment System', 'canonical');\n\
                 INSERT INTO releases VALUES ('release-1', 'game-1', 'nes', 'canonical');\n\
                 INSERT INTO release_artifacts VALUES ('release-1', 'artifact-1');\n\
                 INSERT INTO local_files VALUES ('artifact-1', 'present');\n\
                 INSERT INTO acquisition_offers VALUES ('artifact-1', 'available');\n\
                 INSERT INTO emulators VALUES ('emulator-1');",
            )
            .unwrap();
        drop(connection);

        let catalog = load(&path).unwrap();
        assert_eq!(catalog.games.len(), 1);
        assert_eq!(catalog.games[0].title, "Metroid");
        assert_eq!(catalog.games[0].platform, "Nintendo Entertainment System");
        assert!(catalog.games[0].local);
        assert!(catalog.games[0].downloadable);
        assert_eq!(catalog.platforms[0].game_count, 1);
        assert_eq!(catalog.local_file_count, 1);
        assert_eq!(catalog.offer_count, 1);
        assert_eq!(catalog.emulator_count, 1);
    }
}
