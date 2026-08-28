use std::collections::HashSet;
use std::env;
use std::path::{Path, PathBuf};
use std::sync::Arc;

use anyhow::{Context, Result, bail};
use directories::ProjectDirs;
use rusqlite::{Connection, OpenFlags};

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Game {
    pub id: String,
    pub launchbox_db_id: i64,
    pub title: String,
    pub platform: String,
    pub status: String,
    pub local: bool,
    pub downloadable: bool,
    pub non_retail: bool,
    pub adult: bool,
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
    pub hide_non_retail: bool,
    pub hide_adult: bool,
    pub favorite_game_ids: Arc<HashSet<String>>,
    pub collection_game_ids: Arc<HashSet<String>>,
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

pub(crate) fn requested_discovery_database_path() -> Option<PathBuf> {
    requested_path("--games-database", "LUNCHBOX_GAMES_DATABASE").or_else(|| {
        existing_path([
            PathBuf::from("lunchbox-games.db"),
            PathBuf::from("db/games.db"),
            legacy_data_path("games.db"),
            project_data_path("games.db"),
        ])
    })
}

pub(crate) fn requested_minerva_database_path() -> Option<PathBuf> {
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

pub(crate) fn requested_path(argument_name: &str, environment_name: &str) -> Option<PathBuf> {
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

pub(crate) fn open_read_only(path: &Path, description: &str) -> Result<Connection> {
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
    if schema_version != 5 {
        bail!("unsupported Lunchbox schema version {schema_version}; expected version 5");
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
            launchbox_db_id: 0,
            search_key: format!("{}\n{}", title.to_lowercase(), platform.to_lowercase()),
            title,
            platform,
            status: row.get(3)?,
            local: row.get(4)?,
            downloadable: row.get(5)?,
            non_retail: false,
            adult: false,
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
    local_only_games: Vec<Game>,
    native_file_paths: HashSet<PathBuf>,
    file_count: usize,
}

#[derive(Default)]
struct MinervaCoverage {
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
    let release_type = optional_game_column(&discovery, "release_type")?;
    let esrb = optional_game_column(&discovery, "esrb")?;
    let genre = optional_game_column(&discovery, "genre")?;
    let query = format!(
        "SELECT g.id, g.title, p.name, coalesce(g.status, 'canonical'),
                coalesce(g.launchbox_db_id, 0), {release_type}, {esrb}, {genre}
         FROM games g
         JOIN platforms p ON p.id = g.platform_id
         ORDER BY coalesce(nullif(g.sort_title, ''), g.title) COLLATE NOCASE, g.id"
    );
    let mut statement = discovery.prepare(&query)?;
    let rows = statement.query_map([], |row| {
        Ok((
            row.get::<_, String>(0)?,
            row.get::<_, String>(1)?,
            row.get::<_, String>(2)?,
            row.get::<_, String>(3)?,
            row.get::<_, i64>(4)?,
            row.get::<_, Option<String>>(5)?,
            row.get::<_, Option<String>>(6)?,
            row.get::<_, Option<String>>(7)?,
        ))
    })?;
    for row in rows {
        let (id, title, platform, status, database_id, release_type, esrb, genre) = row?;
        let local = (database_id > 0 && installed.database_ids.contains(&database_id))
            || installed.game_uids.contains(&id);
        let minerva_covered = minerva
            .platform_names
            .contains(&normalize_platform_key(&platform));
        let non_retail = is_non_retail_game(&title, release_type.as_deref());
        let adult = is_adult_game(&title, esrb.as_deref(), genre.as_deref());
        games.push(Game {
            id,
            launchbox_db_id: database_id,
            search_key: format!("{}\n{}", title.to_lowercase(), platform.to_lowercase()),
            title,
            platform,
            status,
            local,
            downloadable: minerva_covered && !local,
            non_retail,
            adult,
        });
    }
    games.extend(installed.local_only_games.iter().cloned());

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
    let mut platforms = platform_rows.collect::<rusqlite::Result<Vec<_>>>()?;
    for local_game in &installed.local_only_games {
        if let Some(platform) = platforms
            .iter_mut()
            .find(|platform| platform.name == local_game.platform)
        {
            platform.game_count = platform.game_count.saturating_add(1);
        } else {
            platforms.push(Platform {
                name: local_game.platform.clone(),
                game_count: 1,
            });
        }
    }
    platforms.sort_by(|left, right| {
        left.name
            .to_lowercase()
            .cmp(&right.name.to_lowercase())
            .then_with(|| left.name.cmp(&right.name))
    });

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

fn optional_game_column(connection: &Connection, column: &str) -> Result<&'static str> {
    if column_exists(connection, "games", column)? {
        Ok(match column {
            "release_type" => "g.release_type",
            "esrb" => "g.esrb",
            "genre" => "g.genre",
            _ => unreachable!("optional game columns are fixed by the caller"),
        })
    } else {
        Ok("NULL")
    }
}

fn load_installed_games(path: Option<&Path>) -> Result<InstalledGames> {
    let mut installed = InstalledGames::default();
    if let Some(path) = path {
        let connection = open_read_only(path, "Lunchbox user database")?;
        if table_exists(&connection, "game_files")? {
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
                add_installed_identity(&mut installed, row?);
            }
        }
    }
    load_native_installed_games(&mut installed)?;
    Ok(installed)
}

fn load_native_installed_games(installed: &mut InstalledGames) -> Result<()> {
    let path = crate::settings::state_database_path()?;
    if !path.is_file() {
        return Ok(());
    }
    load_native_installed_games_at(installed, &path)
}

fn load_native_installed_games_at(installed: &mut InstalledGames, path: &Path) -> Result<()> {
    let connection = open_read_only(path, "Lunchbox state database")?;
    if !table_exists(&connection, "installed_games")? {
        return Ok(());
    }
    let mut statement = connection.prepare(
        "SELECT launchbox_db_id, game_uid, file_path, import_source FROM installed_games",
    )?;
    let rows = statement.query_map([], |row| {
        Ok((
            row.get::<_, i64>(0)?,
            row.get::<_, Option<String>>(1)?,
            row.get::<_, String>(2)?,
            row.get::<_, String>(3)?,
        ))
    })?;
    for row in rows {
        let (database_id, game_uid, file_path, import_source) = row?;
        let file_path = PathBuf::from(file_path);
        if import_source != "local"
            && file_path.is_file()
            && installed.native_file_paths.insert(file_path)
        {
            add_installed_identity(installed, (database_id, game_uid));
        }
    }
    drop(statement);

    if !table_exists(&connection, "local_rom_files")? {
        return Ok(());
    }
    let mut statement = connection.prepare(
        "SELECT id, path_display, path_bytes, path_encoding, game_uid,
                launchbox_db_id, display_title, platform
         FROM local_rom_files
         WHERE included=1 AND availability='present'
         ORDER BY display_title COLLATE NOCASE, id",
    )?;
    let rows = statement.query_map([], |row| {
        Ok((
            row.get::<_, String>(0)?,
            row.get::<_, String>(1)?,
            row.get::<_, Vec<u8>>(2)?,
            row.get::<_, String>(3)?,
            row.get::<_, Option<String>>(4)?,
            row.get::<_, i64>(5)?,
            row.get::<_, String>(6)?,
            row.get::<_, String>(7)?,
        ))
    })?;
    for row in rows {
        let (id, display, bytes, encoding, game_uid, database_id, title, platform) = row?;
        let path = crate::local_import::decode_path(bytes, &encoding, display);
        if !path.is_file() || !installed.native_file_paths.insert(path) {
            continue;
        }
        installed.file_count = installed.file_count.saturating_add(1);
        if database_id > 0 {
            installed.database_ids.insert(database_id);
        }
        if let Some(game_uid) = game_uid.filter(|value| !value.is_empty()) {
            installed.game_uids.insert(game_uid);
        } else {
            installed.local_only_games.push(Game {
                id: format!("local-file:{id}"),
                launchbox_db_id: 0,
                search_key: format!("{}\n{}", title.to_lowercase(), platform.to_lowercase()),
                title,
                platform,
                status: "local-only".to_owned(),
                local: true,
                downloadable: false,
                non_retail: false,
                adult: false,
            });
        }
    }
    Ok(())
}

fn add_installed_identity(
    installed: &mut InstalledGames,
    (database_id, game_uid): (i64, Option<String>),
) {
    installed.file_count = installed.file_count.saturating_add(1);
    if database_id > 0 {
        installed.database_ids.insert(database_id);
    }
    if let Some(game_uid) = game_uid.filter(|value| !value.is_empty()) {
        installed.game_uids.insert(game_uid);
    }
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
        "SELECT tp.lunchbox_platform_name, tp.minerva_platform,
                coalesce(t.collection, '')
         FROM minerva_torrent_platforms tp
         JOIN minerva_torrents t ON t.id=tp.torrent_id",
    )?;
    let rows = statement.query_map([], |row| {
        Ok((
            row.get::<_, Option<String>>(0)?,
            row.get::<_, String>(1)?,
            row.get::<_, String>(2)?,
        ))
    })?;
    let mut atari_800_fallback = false;
    let mut arcade_fallback = false;
    for row in rows {
        let (mapped_name, provider_name, collection) = row?;
        if let Some(name) = mapped_name.filter(|name| !name.trim().is_empty()) {
            coverage
                .platform_names
                .insert(normalize_platform_key(&name));
        }
        let provider_key = normalize_platform_key(&provider_name);
        coverage.platform_names.insert(provider_key.clone());
        atari_800_fallback |= provider_key == "atari" && collection == "TOSEC";
        arcade_fallback |= collection == "MAME"
            && matches!(
                provider_key.as_str(),
                "roms-merged" | "roms-split" | "roms-non-merged"
            );
    }

    // Preserve the two explicit fallbacks used by the legacy frontend. These
    // are provider mappings, not inferred game identity links.
    if atari_800_fallback {
        coverage.platform_names.insert("atari-800".to_owned());
    }
    if arcade_fallback {
        coverage.platform_names.insert("arcade".to_owned());
    }

    Ok(coverage)
}

pub(crate) fn normalize_platform_key(value: &str) -> String {
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

const NON_RETAIL_RELEASE_TYPES: [&str; 3] = ["homebrew", "rom hack", "unlicensed"];
const NON_RETAIL_TITLE_TAGS: [&str; 6] = [
    "homebrew",
    "hack",
    "pirate",
    "bootleg",
    "unl",
    "aftermarket",
];

fn is_non_retail_game(title: &str, release_type: Option<&str>) -> bool {
    if release_type.is_some_and(|value| {
        NON_RETAIL_RELEASE_TYPES
            .iter()
            .any(|expected| value.trim().eq_ignore_ascii_case(expected))
    }) {
        return true;
    }

    let mut remainder = title;
    while let Some(open) = remainder.find('(') {
        remainder = &remainder[open + 1..];
        let Some(close) = remainder.find(')') else {
            break;
        };
        let tag = remainder[..close].trim();
        if NON_RETAIL_TITLE_TAGS
            .iter()
            .any(|expected| tag.eq_ignore_ascii_case(expected))
        {
            return true;
        }
        remainder = &remainder[close + 1..];
    }
    false
}

fn contains_ascii_phrase(text: &str, phrase: &str) -> bool {
    text.as_bytes()
        .windows(phrase.len())
        .any(|window| window.eq_ignore_ascii_case(phrase.as_bytes()))
}

fn contains_adult_token(text: &str) -> bool {
    text.split(|character: char| !character.is_alphanumeric())
        .any(|part| {
            ["adult", "hentai", "erotic", "porn", "sex"]
                .iter()
                .any(|token| part.eq_ignore_ascii_case(token))
        })
}

fn is_adult_game(title: &str, esrb: Option<&str>, genre: Option<&str>) -> bool {
    if esrb.is_some_and(|value| {
        let value = value.trim();
        value
            .as_bytes()
            .get(..2)
            .is_some_and(|prefix| prefix.eq_ignore_ascii_case(b"ao"))
            || contains_ascii_phrase(value, "adults only")
    }) {
        return true;
    }
    if contains_ascii_phrase(title, "adults only")
        || genre.is_some_and(|value| contains_ascii_phrase(value, "adults only"))
    {
        return true;
    }

    contains_adult_token(title) || genre.is_some_and(contains_adult_token)
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
                && (!filter.hide_non_retail || !game.non_retail)
                && (!filter.hide_adult || !game.adult)
                && match filter.availability.as_str() {
                    "local" => game.local,
                    "downloadable" => game.downloadable && !game.local,
                    "favorites" => filter.favorite_game_ids.contains(&game.id),
                    availability if availability.starts_with("collection:") => {
                        filter.collection_game_ids.contains(&game.id)
                    }
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
                    launchbox_db_id: 1,
                    title: "Metroid".into(),
                    platform: "Nintendo Entertainment System".into(),
                    status: "canonical".into(),
                    local: true,
                    downloadable: false,
                    non_retail: false,
                    adult: false,
                    search_key: "metroid\nnintendo entertainment system".into(),
                },
                Game {
                    id: "outrun".into(),
                    launchbox_db_id: 2,
                    title: "OutRun".into(),
                    platform: "Arcade".into(),
                    status: "canonical".into(),
                    local: false,
                    downloadable: true,
                    non_retail: false,
                    adult: false,
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
                    ..Filter::default()
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
        assert_eq!(
            filter_indices(
                &catalog,
                &Filter {
                    availability: "favorites".into(),
                    favorite_game_ids: Arc::new(HashSet::from(["metroid".to_owned()])),
                    ..Filter::default()
                }
            ),
            vec![0]
        );
        assert_eq!(
            filter_indices(
                &catalog,
                &Filter {
                    search: "arcade".into(),
                    platform: "Arcade".into(),
                    availability: "collection:co-op".into(),
                    collection_game_ids: Arc::new(HashSet::from(["outrun".to_owned()])),
                    ..Filter::default()
                }
            ),
            vec![1]
        );
    }

    #[test]
    fn legacy_content_classification_is_bounded_and_composable() {
        assert!(is_non_retail_game("Game", Some("Homebrew")));
        assert!(is_non_retail_game("Game (USA) (Unl)", None));
        assert!(!is_non_retail_game("Hack and Slash", None));
        assert!(is_adult_game("Game", Some("AO - Adults Only"), None));
        assert!(is_adult_game("Late Night", None, Some("Adult; Puzzle")));
        assert!(!is_adult_game("Sexxion", None, Some("Action")));

        let mut catalog = fixture_catalog();
        catalog.games[0].non_retail = true;
        catalog.games[1].adult = true;
        assert_eq!(
            filter_indices(
                &catalog,
                &Filter {
                    hide_non_retail: true,
                    ..Filter::default()
                }
            ),
            vec![1]
        );
        assert_eq!(
            filter_indices(
                &catalog,
                &Filter {
                    availability: "downloadable".into(),
                    hide_adult: true,
                    ..Filter::default()
                }
            ),
            Vec::<usize>::new()
        );
    }

    #[test]
    fn loads_schema_five_catalog_with_linked_local_and_downloadable_state() {
        let directory = tempfile::tempdir().unwrap();
        let path = directory.path().join("catalog.db");
        let connection = Connection::open(&path).unwrap();
        connection
            .execute_batch(
                "CREATE TABLE schema_migrations (version INTEGER);\n\
                 INSERT INTO schema_migrations VALUES (5);\n\
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
                "CREATE TABLE minerva_torrents (
                   id INTEGER PRIMARY KEY, collection TEXT
                 );
                 CREATE TABLE minerva_torrent_platforms (
                   torrent_id INTEGER NOT NULL,
                   lunchbox_platform_id INTEGER, lunchbox_platform_name TEXT,
                   minerva_platform TEXT NOT NULL
                 );
                 INSERT INTO minerva_torrents VALUES (1, 'No-Intro');
                 INSERT INTO minerva_torrent_platforms
                   VALUES (1, 2, 'Nintendo Game Boy', 'Nintendo - Game Boy');",
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

    #[test]
    fn native_local_inventory_keeps_exact_and_unmatched_files_distinct() {
        let directory = tempfile::tempdir().unwrap();
        let state_path = directory.path().join("state.db");
        let exact_path = directory.path().join("Exact.nes");
        let unmatched_path = directory.path().join("Unknown.nes");
        std::fs::write(&exact_path, b"exact").unwrap();
        std::fs::write(&unmatched_path, b"unknown").unwrap();
        let exact = crate::local_import::encode_path(&exact_path);
        let unmatched = crate::local_import::encode_path(&unmatched_path);
        let connection = Connection::open(&state_path).unwrap();
        connection
            .execute_batch(
                "CREATE TABLE installed_games (
                     launchbox_db_id INTEGER, game_uid TEXT, file_path TEXT, import_source TEXT
                 );
                 CREATE TABLE local_rom_files (
                     id TEXT, path_display TEXT, path_bytes BLOB, path_encoding TEXT,
                     game_uid TEXT, launchbox_db_id INTEGER, display_title TEXT,
                     platform TEXT, included INTEGER, availability TEXT
                 );",
            )
            .unwrap();
        connection
            .execute(
                "INSERT INTO local_rom_files VALUES (
                     'exact-file', ?1, ?2, ?3, 'catalog-game', 42,
                     'Exact', 'System', 1, 'present'
                 )",
                rusqlite::params![exact.display, exact.bytes, exact.encoding],
            )
            .unwrap();
        connection
            .execute(
                "INSERT INTO local_rom_files VALUES (
                     'overlapping-root-copy', ?1, ?2, ?3, NULL, 0,
                     'Unknown', 'Unassigned', 1, 'present'
                 )",
                rusqlite::params![unmatched.display, unmatched.bytes, unmatched.encoding],
            )
            .unwrap();
        connection
            .execute(
                "INSERT INTO local_rom_files VALUES (
                     'unknown-file', ?1, ?2, ?3, NULL, 0,
                     'Unknown', 'Unassigned', 1, 'present'
                 )",
                rusqlite::params![unmatched.display, unmatched.bytes, unmatched.encoding],
            )
            .unwrap();
        drop(connection);

        let mut installed = InstalledGames::default();
        load_native_installed_games_at(&mut installed, &state_path).unwrap();
        assert_eq!(installed.file_count, 2);
        assert!(installed.database_ids.contains(&42));
        assert!(installed.game_uids.contains("catalog-game"));
        assert_eq!(installed.local_only_games.len(), 1);
        assert_eq!(installed.local_only_games[0].title, "Unknown");
        assert_eq!(installed.local_only_games[0].status, "local-only");
    }
}
