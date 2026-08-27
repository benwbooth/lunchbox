use std::collections::HashSet;
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
    pub source_label: String,
}

#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct Filter {
    pub search: String,
    pub platform: String,
    pub availability: String,
}

pub fn requested_database_path() -> Option<PathBuf> {
    requested_path("--database", "LUNCHBOX_DATABASE")
        .filter(|path| !path.as_os_str().is_empty())
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

fn requested_discovery_database_path() -> Option<PathBuf> {
    requested_path("--games-database", "LUNCHBOX_GAMES_DATABASE").or_else(|| {
        existing_path([
            PathBuf::from("lunchbox-games.db"),
            PathBuf::from("db/games.db"),
            legacy_data_path("games.db"),
            project_data_path("games.db"),
        ])
    })
}

fn requested_minerva_database_path() -> Option<PathBuf> {
    requested_path("--minerva-database", "LUNCHBOX_MINERVA_DATABASE").or_else(|| {
        existing_path([
            PathBuf::from("minerva.db"),
            PathBuf::from("db/minerva.db"),
            legacy_data_path("minerva.db"),
            project_data_path("minerva.db"),
        ])
    })
}

fn requested_user_database_path() -> Option<PathBuf> {
    requested_path("--user-database", "LUNCHBOX_USER_DATABASE")
        .or_else(|| existing_path([legacy_data_path("user.db"), project_data_path("user.db")]))
}

fn requested_path(argument_name: &str, environment_name: &str) -> Option<PathBuf> {
    let equals_prefix = format!("{argument_name}=");
    let mut arguments = env::args_os().skip(1);
    while let Some(argument) = arguments.next() {
        if argument == argument_name {
            return arguments.next().map(PathBuf::from);
        }
        if let Some(argument) = argument.to_str()
            && let Some(path) = argument.strip_prefix(&equals_prefix)
        {
            return Some(PathBuf::from(path));
        }
    }

    env::var_os(environment_name)
        .filter(|path| !path.is_empty())
        .map(PathBuf::from)
}

fn existing_path<const N: usize>(paths: [PathBuf; N]) -> Option<PathBuf> {
    paths.into_iter().find(|path| path.is_file())
}

fn legacy_data_path(file_name: &str) -> PathBuf {
    directories::BaseDirs::new()
        .map(|dirs| dirs.data_dir().join("lunchbox").join(file_name))
        .unwrap_or_else(|| PathBuf::from(file_name))
}

fn project_data_path(file_name: &str) -> PathBuf {
    ProjectDirs::from("com", "Lunchbox", "Lunchbox")
        .map(|dirs| dirs.data_dir().join(file_name))
        .unwrap_or_else(|| PathBuf::from(file_name))
}

pub fn load(path: &Path) -> Result<Catalog> {
    let connection = open_read_only(path, "Lunchbox database")?;
    validate_canonical_schema(&connection)?;

    if let Some(discovery_path) = requested_discovery_database_path() {
        return load_discovery_catalog(
            &connection,
            &discovery_path,
            requested_minerva_database_path().as_deref(),
            requested_user_database_path().as_deref(),
        );
    }

    load_canonical_catalog(&connection)
}

fn open_read_only(path: &Path, description: &str) -> Result<Connection> {
    let connection = Connection::open_with_flags(
        path,
        OpenFlags::SQLITE_OPEN_READ_ONLY | OpenFlags::SQLITE_OPEN_NO_MUTEX,
    )
    .with_context(|| format!("opening {description} {}", path.display()))?;
    connection.busy_timeout(std::time::Duration::from_secs(2))?;
    connection.execute_batch(
        "PRAGMA foreign_keys = ON;\n\
         PRAGMA query_only = ON;\n\
         PRAGMA trusted_schema = OFF;\n\
         PRAGMA temp_store = MEMORY;\n\
         PRAGMA cache_size = -16384;\n\
         PRAGMA mmap_size = 268435456;",
    )?;
    Ok(connection)
}

fn validate_canonical_schema(connection: &Connection) -> Result<()> {
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
    Ok(())
}

fn load_canonical_catalog(connection: &Connection) -> Result<Catalog> {
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
        local_file_count: count(connection, "local_files", "availability = 'present'")?,
        offer_count: count(
            connection,
            "acquisition_offers",
            "availability <> 'unavailable'",
        )?,
        emulator_count: count(connection, "emulators", "1")?,
        source_label: "Lunchbox canonical catalog".to_owned(),
    })
}

#[derive(Default)]
struct InstalledGames {
    database_ids: HashSet<i64>,
    game_uids: HashSet<String>,
    file_count: usize,
}

#[derive(Default)]
struct MinervaCoverage {
    platform_ids: HashSet<i64>,
    platform_names: HashSet<String>,
    offer_count: usize,
}

fn load_discovery_catalog(
    canonical: &Connection,
    discovery_path: &Path,
    minerva_path: Option<&Path>,
    user_path: Option<&Path>,
) -> Result<Catalog> {
    let discovery = open_read_only(discovery_path, "Lunchbox discovery database")?;
    validate_discovery_schema(&discovery)?;
    let installed = load_installed_games(user_path)?;
    let minerva = load_minerva_coverage(minerva_path)?;

    let game_capacity = count(&discovery, "games", "1")?;
    let mut games = Vec::with_capacity(game_capacity);
    let mut statement = discovery.prepare(
        "SELECT g.id, g.title, p.name, coalesce(g.status, 'canonical'),
                coalesce(g.launchbox_db_id, 0), g.platform_id
         FROM games g
         JOIN platforms p ON p.id = g.platform_id
         ORDER BY coalesce(nullif(g.sort_title, ''), g.title) COLLATE NOCASE, g.id",
    )?;
    let rows = statement.query_map([], |row| {
        Ok((
            row.get::<_, String>(0)?,
            row.get::<_, String>(1)?,
            row.get::<_, String>(2)?,
            row.get::<_, String>(3)?,
            row.get::<_, i64>(4)?,
            row.get::<_, i64>(5)?,
        ))
    })?;
    for row in rows {
        let (id, title, platform, status, database_id, platform_id) = row?;
        let local = (database_id > 0 && installed.database_ids.contains(&database_id))
            || installed.game_uids.contains(&id);
        let minerva_covered = minerva.platform_ids.contains(&platform_id)
            || minerva
                .platform_names
                .contains(&normalize_platform_key(&platform));
        games.push(Game {
            id,
            search_key: format!("{}\n{}", title.to_lowercase(), platform.to_lowercase()),
            title,
            platform,
            status,
            local,
            downloadable: minerva_covered && !local,
        });
    }

    let mut platform_statement = discovery.prepare(
        "SELECT p.name, count(g.id)
         FROM platforms p
         LEFT JOIN games g ON g.platform_id = p.id
         GROUP BY p.id, p.name
         HAVING count(g.id) > 0
         ORDER BY p.name COLLATE NOCASE",
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
        local_file_count: installed.file_count,
        offer_count: minerva.offer_count,
        emulator_count: count(canonical, "emulators", "1")?,
        source_label: format!("Discovery catalog: {}", discovery_path.display()),
    })
}

fn validate_discovery_schema(connection: &Connection) -> Result<()> {
    for (table, column) in [
        ("games", "launchbox_db_id"),
        ("games", "platform_id"),
        ("platforms", "name"),
    ] {
        if !column_exists(connection, table, column)? {
            bail!("discovery database is missing required column {table}.{column}");
        }
    }
    Ok(())
}

fn load_installed_games(path: Option<&Path>) -> Result<InstalledGames> {
    let Some(path) = path else {
        return Ok(InstalledGames::default());
    };
    let connection = open_read_only(path, "Lunchbox user database")?;
    if !table_exists(&connection, "game_files")? {
        return Ok(InstalledGames::default());
    }

    let mut installed = InstalledGames::default();
    let has_game_uid = column_exists(&connection, "game_files", "game_uid")?;
    let query = if has_game_uid {
        "SELECT launchbox_db_id, game_uid FROM game_files"
    } else {
        "SELECT launchbox_db_id, NULL FROM game_files"
    };
    let mut statement = connection.prepare(query)?;
    let rows = statement.query_map([], |row| {
        Ok((row.get::<_, i64>(0)?, row.get::<_, Option<String>>(1)?))
    })?;
    for row in rows {
        let (database_id, game_uid) = row?;
        installed.file_count = installed.file_count.saturating_add(1);
        if database_id > 0 {
            installed.database_ids.insert(database_id);
        }
        if let Some(game_uid) = game_uid.filter(|value| !value.is_empty()) {
            installed.game_uids.insert(game_uid);
        }
    }
    Ok(installed)
}

fn load_minerva_coverage(path: Option<&Path>) -> Result<MinervaCoverage> {
    let Some(path) = path else {
        return Ok(MinervaCoverage::default());
    };
    let connection = open_read_only(path, "Minerva catalog")?;
    for table in ["minerva_torrents", "minerva_torrent_platforms"] {
        if !table_exists(&connection, table)? {
            bail!("Minerva catalog is missing required table {table}");
        }
    }

    let mut coverage = MinervaCoverage {
        offer_count: count(&connection, "minerva_torrents", "1")?,
        ..MinervaCoverage::default()
    };
    let mut statement = connection.prepare(
        "SELECT lunchbox_platform_id, lunchbox_platform_name, minerva_platform
         FROM minerva_torrent_platforms",
    )?;
    let rows = statement.query_map([], |row| {
        Ok((
            row.get::<_, Option<i64>>(0)?,
            row.get::<_, Option<String>>(1)?,
            row.get::<_, String>(2)?,
        ))
    })?;
    for row in rows {
        let (platform_id, mapped_name, provider_name) = row?;
        if let Some(platform_id) = platform_id {
            coverage.platform_ids.insert(platform_id);
        }
        if let Some(name) = mapped_name.filter(|name| !name.trim().is_empty()) {
            coverage
                .platform_names
                .insert(normalize_platform_key(&name));
        }
        coverage
            .platform_names
            .insert(normalize_platform_key(&provider_name));
    }

    // Preserve the two explicit fallbacks used by the legacy frontend. These
    // are provider mappings, not inferred game identity links.
    if coverage.platform_names.contains("atari") {
        coverage.platform_names.insert("atari-800".to_owned());
    }
    if ["roms-merged", "roms-split", "roms-non-merged"]
        .iter()
        .any(|name| coverage.platform_names.contains(*name))
    {
        coverage.platform_names.insert("arcade".to_owned());
    }

    Ok(coverage)
}

fn normalize_platform_key(value: &str) -> String {
    let mut normalized = String::with_capacity(value.len());
    let mut separator = false;
    for character in value.chars().flat_map(char::to_lowercase) {
        if character.is_alphanumeric() {
            if separator && !normalized.is_empty() {
                normalized.push('-');
            }
            normalized.push(character);
            separator = false;
        } else {
            separator = true;
        }
    }
    normalized
}

fn table_exists(connection: &Connection, table: &str) -> Result<bool> {
    let count: i64 = connection.query_row(
        "SELECT count(*) FROM sqlite_schema WHERE type='table' AND name=?1",
        [table],
        |row| row.get(0),
    )?;
    Ok(count == 1)
}

fn column_exists(connection: &Connection, table: &str, column: &str) -> Result<bool> {
    let query = format!(
        "SELECT count(*) FROM pragma_table_info('{}') WHERE name=?1",
        table.replace('\'', "''")
    );
    let count: i64 = connection.query_row(&query, [column], |row| row.get(0))?;
    Ok(count == 1)
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
                    "downloadable" => game.downloadable && !game.local,
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

        let connection = open_read_only(&path, "test catalog").unwrap();
        validate_canonical_schema(&connection).unwrap();
        let catalog = load_canonical_catalog(&connection).unwrap();
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

    #[test]
    fn discovery_catalog_distinguishes_installed_and_minerva_downloadable_games() {
        let directory = tempfile::tempdir().unwrap();
        let canonical_path = directory.path().join("canonical.db");
        let discovery_path = directory.path().join("games.db");
        let minerva_path = directory.path().join("minerva.db");
        let user_path = directory.path().join("user.db");

        let canonical = Connection::open(&canonical_path).unwrap();
        canonical
            .execute_batch(
                "CREATE TABLE emulators (id TEXT PRIMARY KEY);
                 INSERT INTO emulators VALUES ('emu-1');",
            )
            .unwrap();

        let discovery = Connection::open(&discovery_path).unwrap();
        discovery
            .execute_batch(
                "CREATE TABLE platforms (id INTEGER PRIMARY KEY, name TEXT NOT NULL);
                 CREATE TABLE games (
                   id TEXT PRIMARY KEY, title TEXT NOT NULL, sort_title TEXT,
                   status TEXT, launchbox_db_id INTEGER, platform_id INTEGER NOT NULL
                 );
                 INSERT INTO platforms VALUES (1, 'Nintendo Game Boy');
                 INSERT INTO platforms VALUES (2, 'Uncovered System');
                 INSERT INTO games VALUES
                   ('installed-id', 'Installed Game', NULL, 'Released', 11, 1),
                   ('download-id', 'Download Game', NULL, 'Released', 12, 1),
                   ('uncovered-id', 'Uncovered Game', NULL, 'Released', 13, 2),
                   ('installed-uid', 'ID-less Installed Game', NULL, 'Released', NULL, 1);",
            )
            .unwrap();
        drop(discovery);

        let minerva = Connection::open(&minerva_path).unwrap();
        minerva
            .execute_batch(
                "CREATE TABLE minerva_torrents (id INTEGER PRIMARY KEY);
                 CREATE TABLE minerva_torrent_platforms (
                   lunchbox_platform_id INTEGER, lunchbox_platform_name TEXT,
                   minerva_platform TEXT NOT NULL
                 );
                 INSERT INTO minerva_torrents VALUES (1);
                 INSERT INTO minerva_torrent_platforms
                   VALUES (1, 'Nintendo Game Boy', 'Nintendo - Game Boy');",
            )
            .unwrap();
        drop(minerva);

        let user = Connection::open(&user_path).unwrap();
        user.execute_batch(
            "CREATE TABLE game_files (launchbox_db_id INTEGER NOT NULL, game_uid TEXT);
             INSERT INTO game_files VALUES (11, NULL);
             INSERT INTO game_files VALUES (0, 'installed-uid');",
        )
        .unwrap();
        drop(user);

        let catalog = load_discovery_catalog(
            &canonical,
            &discovery_path,
            Some(&minerva_path),
            Some(&user_path),
        )
        .unwrap();
        assert_eq!(catalog.games.len(), 4);
        assert_eq!(catalog.local_file_count, 2);
        assert_eq!(catalog.offer_count, 1);
        assert_eq!(catalog.emulator_count, 1);
        assert_eq!(
            filter_indices(
                &catalog,
                &Filter {
                    availability: "local".into(),
                    ..Filter::default()
                }
            )
            .len(),
            2
        );
        let downloadable = filter_indices(
            &catalog,
            &Filter {
                availability: "downloadable".into(),
                ..Filter::default()
            },
        );
        assert_eq!(downloadable.len(), 1);
        assert_eq!(catalog.games[downloadable[0]].title, "Download Game");
    }
}
