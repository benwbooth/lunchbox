use std::borrow::Cow;
use std::cmp::Ordering;
use std::collections::{HashMap, HashSet};
use std::env;
use std::path::{Path, PathBuf};
use std::sync::Arc;

use anyhow::{Context, Result, bail};
use directories::ProjectDirs;
use rusqlite::{Connection, OpenFlags, params};

use crate::list_view::{ListColumn, ListColumnFilter, ListMetadata, ListMetadataBuilder};
use crate::settings::GameMetadataOverride;

#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct Game {
    pub id: String,
    pub launchbox_db_id: i64,
    pub title: String,
    pub platform: String,
    pub status: String,
    pub local: bool,
    pub downloadable: bool,
    pub non_retail: bool,
    pub has_non_retail_release: bool,
    pub adult: bool,
    pub has_usa_release: bool,
    pub has_japan_release: bool,
    pub cooperative: String,
    pub(crate) search_key: String,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Platform {
    pub name: String,
    pub game_count: usize,
    pub(crate) search_key: String,
}

#[derive(Clone, Debug, Default)]
pub struct Catalog {
    pub games: Vec<Game>,
    pub platforms: Vec<Platform>,
    pub(crate) list_metadata: ListMetadata,
    pub local_file_count: usize,
    pub offer_count: usize,
    pub emulator_count: usize,
    pub source_label: String,
}

#[derive(Clone, Debug)]
pub struct CatalogPreview {
    pub catalog: Catalog,
    pub total_game_count: usize,
}

#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct CatalogPreviewFocus {
    pub platform: String,
    pub game_uid: String,
}

#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct Filter {
    pub search: String,
    pub platform: String,
    pub availability: String,
    pub tag: String,
    pub hide_non_retail: bool,
    pub hide_adult: bool,
    pub include_usa_releases: bool,
    pub include_japan_releases: bool,
    pub include_adult_releases: bool,
    pub include_non_retail_releases: bool,
    pub favorite_game_ids: Arc<HashSet<String>>,
    pub collection_game_ids: Arc<HashSet<String>>,
    pub collection_game_order: Arc<std::collections::HashMap<String, usize>>,
    pub recent_game_order: Arc<std::collections::HashMap<String, i64>>,
    pub display_titles: Arc<std::collections::HashMap<String, String>>,
    pub game_tags: Arc<HashMap<String, Vec<String>>>,
    pub game_custom_fields: Arc<HashMap<String, Vec<String>>>,
    pub metadata_overrides: Arc<HashMap<String, GameMetadataOverride>>,
    pub list_column_filters: Arc<HashMap<ListColumn, ListColumnFilter>>,
    pub sort_field: String,
    pub sort_descending: bool,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct ListFacetValue {
    pub value: String,
    pub game_count: usize,
}

#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub(crate) struct ListFacetResult {
    pub values: Vec<ListFacetValue>,
    pub total_distinct: usize,
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

pub(crate) fn requested_value(argument_name: &str, environment_name: &str) -> Option<String> {
    let equals_prefix = format!("{argument_name}=");
    let mut arguments = env::args().skip(1);
    while let Some(argument) = arguments.next() {
        if argument == argument_name {
            return arguments.next().filter(|value| !value.is_empty());
        }
        if let Some(value) = argument.strip_prefix(&equals_prefix) {
            return (!value.is_empty()).then(|| value.to_owned());
        }
    }

    env::var(environment_name)
        .ok()
        .filter(|value| !value.is_empty())
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

/// Load enough of the discovery catalog to paint and interact with the first
/// screen while the complete in-memory search index is built. This is never a
/// substitute for the full load: the library model replaces it atomically.
pub fn load_preview(path: &Path, focus: &CatalogPreviewFocus) -> Result<Option<CatalogPreview>> {
    let canonical = open_read_only(path, "Lunchbox database")?;
    validate_canonical_schema(&canonical)?;
    let Some(discovery_path) = requested_discovery_database_path() else {
        return Ok(None);
    };
    let native_state_path = crate::settings::state_database_path()?;
    load_preview_from_sources(
        &canonical,
        &discovery_path,
        requested_minerva_database_path().as_deref(),
        requested_user_database_path().as_deref(),
        native_state_path
            .is_file()
            .then_some(native_state_path.as_path()),
        focus,
    )
    .map(Some)
}

fn load_preview_from_sources(
    canonical: &Connection,
    discovery_path: &Path,
    minerva_path: Option<&Path>,
    user_path: Option<&Path>,
    native_state_path: Option<&Path>,
    focus: &CatalogPreviewFocus,
) -> Result<CatalogPreview> {
    let discovery = open_read_only(&discovery_path, "Lunchbox discovery database")?;
    validate_discovery_schema(&discovery)?;
    let installed = load_installed_games_with_native_state(user_path, native_state_path)?;
    let minerva = load_minerva_coverage(minerva_path)?;
    let total_game_count =
        count(&discovery, "games", "1")?.saturating_add(installed.local_only_games.len());
    let order = if column_exists(&discovery, "games", "sort_title")? {
        "coalesce(nullif(g.sort_title, ''), g.title)"
    } else {
        "g.title"
    };
    let query = format!(
        "SELECT g.id, g.title, p.name, coalesce(g.status, 'canonical'),
                coalesce(g.launchbox_db_id, 0)
         FROM games g
         JOIN platforms p ON p.id = g.platform_id
         WHERE (?1 = '' OR p.name = ?1)
         ORDER BY (g.id = ?2) DESC, {order} COLLATE NOCASE, g.id
         LIMIT 240"
    );
    let mut statement = discovery.prepare(&query)?;
    let rows = statement.query_map(params![focus.platform, focus.game_uid], |row| {
        Ok((
            row.get::<_, String>(0)?,
            row.get::<_, String>(1)?,
            row.get::<_, String>(2)?,
            row.get::<_, String>(3)?,
            row.get::<_, i64>(4)?,
        ))
    })?;
    let mut games = Vec::with_capacity(240 + installed.local_only_games.len());
    let mut list_metadata = ListMetadataBuilder::with_capacity(games.capacity());
    for row in rows {
        let (id, title, platform, status, database_id) = row?;
        let local = (database_id > 0 && installed.database_ids.contains(&database_id))
            || installed.game_uids.contains(&id);
        let non_retail = is_non_retail_game(&title, None);
        let adult = is_adult_game(&title, None, None);
        let (has_usa_release, has_japan_release) = release_region_membership(&title, None);
        games.push(Game {
            search_key: format!("{}\n{}", title.to_lowercase(), platform.to_lowercase()),
            downloadable: !local
                && minerva
                    .platform_names
                    .contains(&normalize_platform_key(&platform)),
            id,
            launchbox_db_id: database_id,
            title,
            platform,
            status,
            local,
            non_retail,
            has_non_retail_release: non_retail,
            adult,
            has_usa_release,
            has_japan_release,
            cooperative: "unknown".to_owned(),
        });
        list_metadata.push_empty();
    }
    for game in &installed.local_only_games {
        games.push(game.clone());
        list_metadata.push_empty();
    }
    apply_alternate_release_regions(&discovery, &mut games)?;
    let mut list_metadata = list_metadata.finish();
    apply_release_families(&mut games, &mut list_metadata);

    let canonical_aliases = canonical_platform_aliases(&canonical)?;
    let mut platform_statement = discovery.prepare(
        "SELECT p.name, count(g.id)
         FROM platforms p
         LEFT JOIN games g ON g.platform_id = p.id
         GROUP BY p.id, p.name
         HAVING count(g.id) > 0
         ORDER BY p.name COLLATE NOCASE",
    )?;
    let platform_rows = platform_statement.query_map([], |row| {
        let name = row.get::<_, String>(0)?;
        let aliases = canonical_aliases
            .get(&normalize_platform_key(&name))
            .map(String::as_str);
        Ok(Platform {
            search_key: platform_search_key(&name, aliases),
            name,
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
                search_key: platform_search_key(&local_game.platform, None),
            });
        }
    }
    platforms.sort_by(|left, right| {
        left.name
            .to_lowercase()
            .cmp(&right.name.to_lowercase())
            .then_with(|| left.name.cmp(&right.name))
    });
    apply_grouped_platform_counts(&games, &mut platforms);

    Ok(CatalogPreview {
        catalog: Catalog {
            games,
            platforms,
            list_metadata,
            local_file_count: installed.file_count,
            offer_count: minerva.offer_count,
            emulator_count: count(&canonical, "emulators", "1")?,
            source_label: format!("Discovery catalog: {}", discovery_path.display()),
        },
        total_game_count,
    })
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
        let non_retail = is_non_retail_game(&title, None);
        let adult = is_adult_game(&title, None, None);
        let (has_usa_release, has_japan_release) = release_region_membership(&title, None);
        Ok(Game {
            id: row.get(0)?,
            launchbox_db_id: 0,
            search_key: format!("{}\n{}", title.to_lowercase(), platform.to_lowercase()),
            title,
            platform,
            status: row.get(3)?,
            local: row.get(4)?,
            downloadable: row.get(5)?,
            non_retail,
            has_non_retail_release: non_retail,
            adult,
            has_usa_release,
            has_japan_release,
            cooperative: "unknown".to_owned(),
        })
    })?;
    let mut games = game_rows.collect::<rusqlite::Result<Vec<_>>>()?;
    let mut list_metadata = ListMetadataBuilder::with_capacity(games.len());
    for _ in &games {
        list_metadata.push_empty();
    }

    let has_platform_aliases = table_exists(connection, "platform_aliases")?;
    let alias_column = if has_platform_aliases {
        "coalesce(group_concat(DISTINCT platform_aliases.alias), '')"
    } else {
        "''"
    };
    let alias_join = if has_platform_aliases {
        "LEFT JOIN platform_aliases ON platform_aliases.platform_id = platforms.id"
    } else {
        ""
    };
    let platform_query = format!(
        "SELECT platforms.canonical_name, count(DISTINCT games.id),\n\
                {alias_column}\n\
         FROM platforms\n\
         {alias_join}\n\
         LEFT JOIN releases ON releases.platform_id = platforms.id\n\
         LEFT JOIN games ON games.id = releases.game_id\n\
                          AND games.status NOT IN ('deprecated', 'merged')\n\
         WHERE platforms.status <> 'deprecated'\n\
         GROUP BY platforms.id, platforms.canonical_name\n\
         ORDER BY platforms.canonical_name COLLATE NOCASE"
    );
    let mut platform_statement = connection.prepare(&platform_query)?;
    let platform_rows = platform_statement.query_map([], |row| {
        let name = row.get::<_, String>(0)?;
        let aliases = row.get::<_, String>(2)?;
        Ok(Platform {
            search_key: platform_search_key(&name, Some(&aliases)),
            name,
            game_count: row.get(1)?,
        })
    })?;
    let mut platforms = platform_rows.collect::<rusqlite::Result<Vec<_>>>()?;
    let mut list_metadata = list_metadata.finish();
    apply_release_families(&mut games, &mut list_metadata);
    apply_grouped_platform_counts(&games, &mut platforms);

    Ok(Catalog {
        games,
        platforms,
        list_metadata,
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
    let native_state_path = crate::settings::state_database_path()?;
    load_discovery_catalog_with_native_state(
        canonical,
        discovery_path,
        minerva_path,
        user_path,
        native_state_path
            .is_file()
            .then_some(native_state_path.as_path()),
    )
}

/// Reads a nullable text column as a borrowed slice, avoiding a `String`
/// allocation per row for values that repeat across games.
fn text_column<'a>(row: &'a rusqlite::Row<'_>, index: usize) -> Option<&'a str> {
    match row.get_ref(index) {
        Ok(rusqlite::types::ValueRef::Text(text)) => std::str::from_utf8(text).ok(),
        _ => None,
    }
}

fn load_discovery_catalog_with_native_state(
    canonical: &Connection,
    discovery_path: &Path,
    minerva_path: Option<&Path>,
    user_path: Option<&Path>,
    native_state_path: Option<&Path>,
) -> Result<Catalog> {
    let discovery = open_read_only(discovery_path, "Lunchbox discovery database")?;
    validate_discovery_schema(&discovery)?;
    let installed = load_installed_games_with_native_state(user_path, native_state_path)?;
    let minerva = load_minerva_coverage(minerva_path)?;

    let game_capacity = count(&discovery, "games", "1")?;
    let mut games = Vec::with_capacity(game_capacity);
    let mut list_metadata = ListMetadataBuilder::with_capacity(game_capacity);
    let release_type = optional_game_column(&discovery, "release_type")?;
    let esrb = optional_game_column(&discovery, "esrb")?;
    let genre = optional_game_column(&discovery, "genre")?;
    let cooperative = optional_game_column(&discovery, "cooperative")?;
    let developer = optional_game_column(&discovery, "developer")?;
    let publisher = optional_game_column(&discovery, "publisher")?;
    let release_date = optional_game_column(&discovery, "release_date")?;
    let release_year = optional_game_column(&discovery, "release_year")?;
    let players = optional_game_column(&discovery, "players")?;
    let rating = optional_game_column(&discovery, "rating")?;
    let sort_title = optional_game_column(&discovery, "sort_title")?;
    let series = optional_game_column(&discovery, "series")?;
    let region = optional_game_column(&discovery, "region")?;
    let play_mode = optional_game_column(&discovery, "play_mode")?;
    let version = optional_game_column(&discovery, "version")?;
    let notes = optional_game_column(&discovery, "notes")?;
    let query = format!(
        "SELECT g.id, g.title, p.name, coalesce(g.status, 'canonical'),
                coalesce(g.launchbox_db_id, 0), {release_type}, {esrb}, {genre},
                {cooperative}, {developer}, {publisher}, {release_date},
                {release_year}, {players}, {rating}, {sort_title}, {series}, {region},
                {play_mode}, {version},
                CASE
                    WHEN lower(trim(coalesce(g.status, ''))) IN
                         ('', 'canonical', 'deprecated', 'merged') THEN NULL
                    ELSE trim(g.status)
                END,
                {notes}
         FROM games g
         JOIN platforms p ON p.id = g.platform_id
         ORDER BY coalesce(nullif(g.sort_title, ''), g.title) COLLATE NOCASE, g.id"
    );
    let mut statement = discovery.prepare(&query)?;
    let mut rows = statement.query([])?;
    // Platforms repeat across rows; derive their normalized and lowercased
    // forms once per distinct platform instead of per game.
    let mut platform_forms: HashMap<String, (String, bool)> = HashMap::new();
    while let Some(row) = rows.next()? {
        // Metadata text columns are read as borrowed values and interned
        // directly; only the per-game identity strings are materialized.
        let id: String = row.get(0)?;
        let title: String = row.get(1)?;
        let platform: String = row.get(2)?;
        let status: String = row.get(3)?;
        let database_id: i64 = row.get(4)?;
        let release_type = text_column(&row, 5);
        let esrb = text_column(&row, 6);
        let genre = text_column(&row, 7);
        let region = text_column(&row, 17);
        let cooperative: Option<i64> = row.get(8)?;
        let local = (database_id > 0 && installed.database_ids.contains(&database_id))
            || installed.game_uids.contains(&id);
        let (platform_lower, minerva_covered) = {
            let entry = platform_forms.entry(platform.clone());
            let (lower, covered) = entry.or_insert_with(|| {
                (
                    platform.to_lowercase(),
                    minerva
                        .platform_names
                        .contains(&normalize_platform_key(&platform)),
                )
            });
            (lower.as_str(), *covered)
        };
        let non_retail = is_non_retail_game(&title, release_type.as_deref());
        let adult = is_adult_game(&title, esrb.as_deref(), genre.as_deref());
        let (has_usa_release, has_japan_release) =
            release_region_membership(&title, region.as_deref());
        let mut search_key = String::with_capacity(title.len() + platform_lower.len() + 1);
        search_key.extend(title.chars().flat_map(char::to_lowercase));
        search_key.push('\n');
        search_key.push_str(platform_lower);
        games.push(Game {
            id,
            launchbox_db_id: database_id,
            search_key,
            title,
            platform,
            status,
            local,
            downloadable: minerva_covered && !local,
            non_retail,
            has_non_retail_release: non_retail,
            adult,
            has_usa_release,
            has_japan_release,
            cooperative: cooperative_status(cooperative).to_owned(),
        });
        let release_year: Option<i64> = row.get(12)?;
        let rating: Option<f64> = row.get(14)?;
        list_metadata.push_text(
            text_column(&row, 15),
            text_column(&row, 9),
            text_column(&row, 10),
            text_column(&row, 11),
            genre.as_deref(),
            text_column(&row, 13),
            esrb.as_deref(),
            release_type.as_deref(),
            text_column(&row, 16),
            region.as_deref(),
            text_column(&row, 18),
            text_column(&row, 19),
            text_column(&row, 20),
            text_column(&row, 21),
            i32::try_from(release_year.unwrap_or_default()).unwrap_or_default(),
            rating,
        )?;
    }
    for game in &installed.local_only_games {
        games.push(game.clone());
        list_metadata.push_empty();
    }
    apply_alternate_release_regions(&discovery, &mut games)?;
    let mut list_metadata = list_metadata.finish();
    apply_release_families(&mut games, &mut list_metadata);

    let canonical_aliases = canonical_platform_aliases(canonical)?;
    let mut platform_statement = discovery.prepare(
        "SELECT p.name, count(g.id)
         FROM platforms p
         LEFT JOIN games g ON g.platform_id = p.id
         GROUP BY p.id, p.name
         HAVING count(g.id) > 0
         ORDER BY p.name COLLATE NOCASE",
    )?;
    let platform_rows = platform_statement.query_map([], |row| {
        let name = row.get::<_, String>(0)?;
        let aliases = canonical_aliases
            .get(&normalize_platform_key(&name))
            .map(String::as_str);
        Ok(Platform {
            search_key: platform_search_key(&name, aliases),
            name,
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
                search_key: platform_search_key(&local_game.platform, None),
            });
        }
    }
    platforms.sort_by(|left, right| {
        left.name
            .to_lowercase()
            .cmp(&right.name.to_lowercase())
            .then_with(|| left.name.cmp(&right.name))
    });
    apply_grouped_platform_counts(&games, &mut platforms);

    Ok(Catalog {
        games,
        platforms,
        list_metadata,
        local_file_count: installed.file_count,
        offer_count: minerva.offer_count,
        emulator_count: count(canonical, "emulators", "1")?,
        source_label: format!("Discovery catalog: {}", discovery_path.display()),
    })
}

fn canonical_platform_aliases(connection: &Connection) -> Result<HashMap<String, String>> {
    let mut aliases = HashMap::new();
    if !table_exists(connection, "platforms")? || !table_exists(connection, "platform_aliases")? {
        return Ok(aliases);
    }
    let mut statement = connection.prepare(
        "SELECT p.canonical_name, coalesce(group_concat(a.alias, ','), '')
         FROM platforms p
         LEFT JOIN platform_aliases a ON a.platform_id = p.id
         WHERE p.status <> 'deprecated'
         GROUP BY p.id, p.canonical_name",
    )?;
    let rows = statement.query_map([], |row| {
        Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?))
    })?;
    for row in rows {
        let (name, values) = row?;
        aliases.insert(normalize_platform_key(&name), values);
    }
    Ok(aliases)
}

fn platform_search_key(name: &str, database_aliases: Option<&str>) -> String {
    let mut key = name.to_lowercase();
    if let Some(aliases) = database_aliases.filter(|aliases| !aliases.trim().is_empty()) {
        key.push('\n');
        key.push_str(&aliases.to_lowercase());
    }
    if let Some(aliases) = legacy_platform_search_aliases(name) {
        key.push('\n');
        key.push_str(&aliases.to_lowercase());
    }
    key
}

pub(crate) fn legacy_platform_search_aliases(name: &str) -> Option<&'static str> {
    Some(match name {
        "Nintendo Entertainment System" => "NES,Famicom,FC",
        "Super Nintendo Entertainment System" => "SNES,Super Famicom,SFC,snesna",
        "Nintendo 64" => "N64",
        "Nintendo GameCube" => "GC,NGC,GameCube",
        "Nintendo Game Boy" => "GB,Game Boy",
        "Nintendo Game Boy Color" => "GBC,Game Boy Color",
        "Nintendo Game Boy Advance" => "GBA,Game Boy Advance",
        "Nintendo DS" => "NDS,DS",
        "Nintendo 3DS" => "3DS,N3DS",
        "Nintendo Wii U" => "Wii U,WiiU",
        "Nintendo Switch" => "Switch,NS",
        "Nintendo Virtual Boy" => "VB,Virtual Boy,virtualboy",
        "Sega Master System" => "SMS,Master System,mastersystem",
        "Sega Genesis" => "MD,Mega Drive,Genesis,megadrive",
        "Sega CD" => "SCD,Mega CD,Sega CD,segacd,megacd",
        "Sega 32X" => "32X,sega32x",
        "Sega Saturn" => "SS,Saturn",
        "Sega Dreamcast" => "DC,Dreamcast",
        "Sega Game Gear" => "GG,Game Gear,gamegear",
        "Sony Playstation" => "PS1,PSX,PS,PlayStation",
        "Sony Playstation 2" => "PS2,PlayStation 2",
        "Sony Playstation 3" => "PS3,PlayStation 3",
        "Sony PSP" => "PSP,PlayStation Portable",
        "Sony Playstation Vita" => "PSV,Vita,PS Vita,psvita",
        "NEC TurboGrafx-16" => "PCE,PC Engine,TG16,TurboGrafx-16,pcengine",
        "NEC TurboGrafx-CD" => "PCECD,PC Engine CD,TG-CD,TurboGrafx-CD,pcenginecd",
        "NEC PC-98" => "PC98,PC-98",
        "SNK Neo Geo Pocket" => "NGP,Neo Geo Pocket",
        "SNK Neo Geo Pocket Color" => "NGPC,Neo Geo Pocket Color",
        "SNK Neo Geo AES" => "AES,MVS,Neo Geo,neogeo",
        "SNK Neo Geo CD" => "Neo Geo CD,neogeocd,neogeocdjp",
        "Atari 2600" => "2600,VCS,atari2600",
        "Atari 5200" => "5200,atari5200",
        "Atari 7800" => "7800,atari7800",
        "Atari Jaguar" => "Jaguar,Jag,atarijaguar",
        "Atari Jaguar CD" => "Jaguar CD,atarijaguarcd",
        "Commodore 64" => "C64",
        "Commodore VIC-20" => "VIC-20,VIC20",
        "Commodore 16" => "C16",
        "MS-DOS" => "DOS",
        "Microsoft Xbox 360" => "X360,360,Xbox 360,xbox360",
        "Sinclair ZX Spectrum" => "ZX,ZX Spectrum,zxspectrum",
        "Amstrad CPC" => "CPC,amstradcpc",
        "Arcade" => "MAME,arcade,fbneo",
        "Arcade Laserdisc" => "Laserdisc,Daphne,Singe,arcade laserdisc",
        "Arcade Pinball" => "Pinball,arcade pinball,MAME pinball",
        "Panasonic 3DO" => "3DO",
        "Philips CD-i" => "CD-i,CDi,cdimono1",
        "Bandai WonderSwan" => "WS,WonderSwan",
        "Bandai WonderSwan Color" => "WSC,WonderSwan Color,wonderswancolor",
        "Coleco ColecoVision" => "Coleco,ColecoVision",
        "GCE Vectrex" => "Vectrex",
        "Sharp X68000" => "X68000",
        "ScummVM" => "ScummVM",
        _ => return None,
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
            "cooperative" => "g.cooperative",
            "developer" => "g.developer",
            "publisher" => "g.publisher",
            "release_date" => "g.release_date",
            "release_year" => "g.release_year",
            "players" => "g.players",
            "rating" => "g.rating",
            "sort_title" => "g.sort_title",
            "series" => "g.series",
            "region" => "g.region",
            "play_mode" => "g.play_mode",
            "version" => "g.version",
            "notes" => "g.notes",
            _ => unreachable!("optional game columns are fixed by the caller"),
        })
    } else {
        Ok("NULL")
    }
}

/// Groups release families (same game across regions/versions) in a single
/// pass, records each family's distinct-title variant count on its
/// representative row, and collapses the family to that representative.
fn apply_release_families(games: &mut Vec<Game>, metadata: &mut ListMetadata) {
    let mut family_order = Vec::<String>::new();
    let mut family_members = HashMap::<String, Vec<usize>>::new();
    for (index, game) in games.iter().enumerate() {
        let base = crate::game_details::catalog_release_base(&game.title)
            .unwrap_or_else(|| game.title.trim().to_owned());
        let key = format!("{}\0{}", game.platform.to_lowercase(), base.to_lowercase());
        if !family_members.contains_key(&key) {
            family_order.push(key.clone());
        }
        family_members.entry(key).or_default().push(index);
    }

    let mut retained = Vec::with_capacity(family_order.len());
    for key in &family_order {
        let Some(members) = family_members.get(key) else {
            continue;
        };
        // The overwhelming majority of families are single releases; keep
        // them untouched without ranking or flag merging.
        if members.len() == 1 {
            retained.push(members[0]);
            continue;
        }
        let representative = members
            .iter()
            .copied()
            .min_by(|left, right| {
                representative_rank(&games[*left]).cmp(&representative_rank(&games[*right]))
            })
            .unwrap_or(members[0]);
        let local = members.iter().any(|index| games[*index].local);
        let downloadable = members.iter().any(|index| games[*index].downloadable);
        let non_retail = members.iter().all(|index| games[*index].non_retail);
        let has_non_retail_release = members
            .iter()
            .any(|index| games[*index].has_non_retail_release);
        let adult = members.iter().any(|index| games[*index].adult);
        let has_usa_release = members.iter().any(|index| games[*index].has_usa_release);
        let has_japan_release = members.iter().any(|index| games[*index].has_japan_release);
        let cooperative = if members
            .iter()
            .any(|index| games[*index].cooperative == "yes")
        {
            Some("yes")
        } else {
            None
        };
        let mut distinct_titles = HashSet::with_capacity(members.len());
        for index in members {
            distinct_titles.insert(games[*index].title.trim().to_lowercase());
        }
        let search_additions: Vec<String> = distinct_titles
            .iter()
            .filter(|title| !games[representative].search_key.contains(title.as_str()))
            .cloned()
            .collect();
        let variant_count = distinct_titles.len();
        let representative_game = &mut games[representative];
        representative_game.local = local;
        representative_game.downloadable = downloadable;
        representative_game.non_retail = non_retail;
        representative_game.has_non_retail_release = has_non_retail_release;
        representative_game.adult = adult;
        representative_game.has_usa_release = has_usa_release;
        representative_game.has_japan_release = has_japan_release;
        if let Some(cooperative) = cooperative {
            representative_game.cooperative = cooperative.to_owned();
        }
        for title in search_additions {
            representative_game.search_key.push('\n');
            representative_game.search_key.push_str(&title);
        }
        metadata.set_variant_count(representative, variant_count);
        retained.push(representative);
    }
    // Rebuild in place by moving the retained games out, avoiding a clone
    // of every game in the catalog.
    let mut collapsed = Vec::with_capacity(retained.len());
    for index in &retained {
        collapsed.push(std::mem::take(&mut games[*index]));
    }
    metadata.retain_rows(&retained);
    *games = collapsed;
}

fn representative_rank(game: &Game) -> (u8, u8, u8, String, String) {
    let base = crate::game_details::catalog_release_base(&game.title)
        .unwrap_or_else(|| game.title.trim().to_owned());
    (
        u8::from(!game.local),
        u8::from(!game.title.trim().eq_ignore_ascii_case(base.trim())),
        u8::from(game.launchbox_db_id <= 0),
        game.title.to_lowercase(),
        game.id.clone(),
    )
}

fn apply_grouped_platform_counts(games: &[Game], platforms: &mut [Platform]) {
    let mut counts = HashMap::<String, usize>::new();
    for game in games {
        *counts.entry(game.platform.to_lowercase()).or_default() += 1;
    }
    for platform in platforms {
        platform.game_count = counts
            .get(&platform.name.to_lowercase())
            .copied()
            .unwrap_or_default();
    }
}

fn cooperative_status(value: Option<i64>) -> &'static str {
    match value {
        Some(1) => "yes",
        Some(0) => "no",
        _ => "unknown",
    }
}

fn load_installed_games(path: Option<&Path>) -> Result<InstalledGames> {
    let native_state_path = crate::settings::state_database_path()?;
    load_installed_games_with_native_state(
        path,
        native_state_path
            .is_file()
            .then_some(native_state_path.as_path()),
    )
}

fn load_installed_games_with_native_state(
    path: Option<&Path>,
    native_state_path: Option<&Path>,
) -> Result<InstalledGames> {
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
    if let Some(native_state_path) = native_state_path {
        load_native_installed_games_at(&mut installed, native_state_path)?;
    }
    Ok(installed)
}

pub(crate) fn installed_game_flags(identities: &[(String, i64)]) -> Result<Vec<bool>> {
    let installed = load_installed_games(requested_user_database_path().as_deref())?;
    Ok(identities
        .iter()
        .map(|(game_uid, launchbox_db_id)| {
            installed.game_uids.contains(game_uid)
                || (*launchbox_db_id > 0 && installed.database_ids.contains(launchbox_db_id))
        })
        .collect())
}

pub(crate) fn game_availability_flags(
    identities: &[(String, i64, String)],
) -> Result<Vec<(bool, bool)>> {
    let installed = load_installed_games(requested_user_database_path().as_deref())?;
    let minerva = load_minerva_coverage(requested_minerva_database_path().as_deref())?;
    Ok(identities
        .iter()
        .map(|(game_uid, launchbox_db_id, platform)| {
            let local = installed.game_uids.contains(game_uid)
                || (*launchbox_db_id > 0 && installed.database_ids.contains(launchbox_db_id));
            let downloadable = !local
                && minerva
                    .platform_names
                    .contains(&normalize_platform_key(platform));
            (local, downloadable)
        })
        .collect())
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
            let non_retail = is_non_retail_game(&title, None);
            let adult = is_adult_game(&title, None, None);
            let (has_usa_release, has_japan_release) = release_region_membership(&title, None);
            installed.local_only_games.push(Game {
                id: format!("local-file:{id}"),
                launchbox_db_id: 0,
                search_key: format!("{}\n{}", title.to_lowercase(), platform.to_lowercase()),
                title,
                platform,
                status: "local-only".to_owned(),
                local: true,
                downloadable: false,
                non_retail,
                has_non_retail_release: non_retail,
                adult,
                has_usa_release,
                has_japan_release,
                cooperative: "unknown".to_owned(),
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

fn release_region_membership(title: &str, metadata_region: Option<&str>) -> (bool, bool) {
    let mut usa = false;
    let mut japan = false;
    let mut include = |value: &str| {
        for region in value.split([',', '/', '&', '+']).map(str::trim) {
            if ["usa", "united states", "north america"]
                .iter()
                .any(|expected| region.eq_ignore_ascii_case(expected))
            {
                usa = true;
            } else if region.eq_ignore_ascii_case("japan") {
                japan = true;
            }
        }
    };
    if let Some(region) = metadata_region.filter(|region| !region.trim().is_empty()) {
        include(region);
    }
    let (_, tags) = crate::tags::parse_title_tags(title);
    for tag in tags
        .iter()
        .filter(|tag| tag.category == crate::tags::TagCategory::Region)
    {
        include(&tag.text);
    }
    // Kana is unambiguous Japanese-language evidence. Han characters alone
    // are deliberately not treated as Japanese because the same code points
    // are also used by Chinese titles. Romanized text is likewise not guessed;
    // exact linked alternate-title region metadata is applied separately.
    if title.chars().any(is_japanese_kana) {
        japan = true;
    }
    (usa, japan)
}

fn is_japanese_kana(character: char) -> bool {
    matches!(
        character as u32,
        0x3040..=0x309f // Hiragana
            | 0x30a0..=0x30ff // Katakana
            | 0x31f0..=0x31ff // Katakana phonetic extensions
            | 0xff66..=0xff9f // Half-width Katakana
    )
}

/// Adds region evidence from alternate titles that are exactly linked through
/// LaunchBox's positive database ID. This recovers localized and romanized
/// names without inferring identity or guessing a language from Latin text.
fn apply_alternate_release_regions(connection: &Connection, games: &mut [Game]) -> Result<()> {
    if !table_exists(connection, "game_alternate_names")?
        || !column_exists(connection, "game_alternate_names", "launchbox_db_id")?
        || !column_exists(connection, "game_alternate_names", "region")?
    {
        return Ok(());
    }

    let mut usa_ids = HashSet::new();
    let mut japan_ids = HashSet::new();
    let mut statement = connection.prepare(
        "SELECT launchbox_db_id, trim(region)
         FROM game_alternate_names
         WHERE launchbox_db_id > 0 AND trim(coalesce(region, '')) <> ''",
    )?;
    let mut rows = statement.query([])?;
    while let Some(row) = rows.next()? {
        let database_id: i64 = row.get(0)?;
        let region: &str = text_column(row, 1).unwrap_or_default();
        let (usa, japan) = release_region_membership("", Some(region));
        if usa {
            usa_ids.insert(database_id);
        }
        if japan {
            japan_ids.insert(database_id);
        }
    }

    for game in games {
        if game.launchbox_db_id <= 0 {
            continue;
        }
        game.has_usa_release |= usa_ids.contains(&game.launchbox_db_id);
        game.has_japan_release |= japan_ids.contains(&game.launchbox_db_id);
    }
    Ok(())
}

pub(crate) fn is_non_retail_game(title: &str, release_type: Option<&str>) -> bool {
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

pub(crate) fn is_adult_game(title: &str, esrb: Option<&str>, genre: Option<&str>) -> bool {
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
    let selected_tag = filter.tag.trim().to_lowercase();
    let mut indices = catalog
        .games
        .iter()
        .enumerate()
        .filter(|(index, _)| {
            game_matches_filter(catalog, filter, *index, None, &search, &selected_tag)
        })
        .map(|(index, _)| index)
        .collect::<Vec<_>>();
    if let Some(column) = ListColumn::parse(&filter.sort_field) {
        let mut keyed = indices
            .into_iter()
            .map(|index| {
                let key = catalog.list_metadata.sort_key(
                    index,
                    &catalog.games[index],
                    column,
                    &filter.metadata_overrides,
                );
                (index, key)
            })
            .collect::<Vec<_>>();
        keyed.sort_by(|(left_index, left_key), (right_index, right_key)| {
            let left_game = &catalog.games[*left_index];
            let right_game = &catalog.games[*right_index];
            let ordering = left_key
                .compare(right_key)
                .then_with(|| {
                    title_sort_key(left_game, filter).cmp(&title_sort_key(right_game, filter))
                })
                .then_with(|| left_game.id.cmp(&right_game.id));
            if filter.sort_descending {
                ordering.reverse()
            } else {
                ordering
            }
        });
        return keyed.into_iter().map(|(index, _)| index).collect();
    }
    indices.sort_by(|left, right| {
        let ordering = compare_games(catalog, filter, *left, *right);
        if filter.sort_descending {
            ordering.reverse()
        } else {
            ordering
        }
    });
    indices
}

pub(crate) fn list_facet_values(
    catalog: &Catalog,
    filter: &Filter,
    column: ListColumn,
    query: &str,
    limit: usize,
) -> ListFacetResult {
    let query = query.trim().to_lowercase();
    let search = filter.search.trim().to_lowercase();
    let selected_tag = filter.tag.trim().to_lowercase();
    let mut counts = HashMap::<String, usize>::new();
    for index in 0..catalog.games.len() {
        if !game_matches_filter(catalog, filter, index, Some(column), &search, &selected_tag) {
            continue;
        }
        let value = catalog.list_metadata.filter_value(
            index,
            &catalog.games[index],
            column,
            &filter.metadata_overrides,
        );
        if !query.is_empty() && !value.to_lowercase().contains(&query) {
            continue;
        }
        let count = counts.entry(value).or_default();
        *count = count.saturating_add(1);
    }
    let total_distinct = counts.len();
    let mut values = counts
        .into_iter()
        .map(|(value, game_count)| ListFacetValue { value, game_count })
        .collect::<Vec<_>>();
    values.sort_by(|left, right| {
        left.value
            .to_lowercase()
            .cmp(&right.value.to_lowercase())
            .then_with(|| left.value.cmp(&right.value))
    });
    values.truncate(limit);
    ListFacetResult {
        values,
        total_distinct,
    }
}

fn game_matches_filter(
    catalog: &Catalog,
    filter: &Filter,
    index: usize,
    ignored_list_column: Option<ListColumn>,
    search: &str,
    selected_tag: &str,
) -> bool {
    let Some(game) = catalog.games.get(index) else {
        return false;
    };
    let release_region_matches = if filter.include_usa_releases || filter.include_japan_releases {
        (filter.include_usa_releases && game.has_usa_release)
            || (filter.include_japan_releases && game.has_japan_release)
    } else {
        true
    };
    let release_category_matches =
        if filter.include_adult_releases || filter.include_non_retail_releases {
            (filter.include_adult_releases && game.adult)
                || (filter.include_non_retail_releases && game.has_non_retail_release)
        } else {
            (!filter.hide_non_retail || !game.non_retail) && (!filter.hide_adult || !game.adult)
        };
    (search.is_empty()
        || game.search_key.contains(&search)
        || filter
            .display_titles
            .get(&game.id)
            .is_some_and(|title| title.to_lowercase().contains(&search))
        || catalog
            .list_metadata
            .matches_search(index, game, &filter.metadata_overrides, &search)
        || filter
            .game_tags
            .get(&game.id)
            .is_some_and(|tags| tags.iter().any(|tag| tag.to_lowercase().contains(&search)))
        || filter
            .game_custom_fields
            .get(&game.id)
            .is_some_and(|fields| {
                fields
                    .iter()
                    .any(|field| field.to_lowercase().contains(&search))
            }))
        && (filter.platform.is_empty() || game.platform == filter.platform)
        && (selected_tag.is_empty()
            || filter
                .game_tags
                .get(&game.id)
                .is_some_and(|tags| tags.iter().any(|tag| tag.to_lowercase() == selected_tag)))
        && release_region_matches
        && release_category_matches
        && match filter.availability.as_str() {
            "local" => game.local,
            "downloadable" => game.downloadable && !game.local,
            "favorites" => filter.favorite_game_ids.contains(&game.id),
            "recent" => filter.recent_game_order.contains_key(&game.id),
            availability if availability.starts_with("collection:") => {
                filter.collection_game_ids.contains(&game.id)
            }
            _ => true,
        }
        && filter
            .list_column_filters
            .iter()
            .all(|(column, selection)| {
                ignored_list_column == Some(*column)
                    || selection.matches(&catalog.list_metadata.filter_value(
                        index,
                        game,
                        *column,
                        &filter.metadata_overrides,
                    ))
            })
}

fn compare_games(catalog: &Catalog, filter: &Filter, left: usize, right: usize) -> Ordering {
    let left_game = &catalog.games[left];
    let right_game = &catalog.games[right];
    let identity_tie_break = || left_game.id.cmp(&right_game.id);
    match filter.sort_field.as_str() {
        _ if filter.availability == "recent" => filter
            .recent_game_order
            .get(&right_game.id)
            .cmp(&filter.recent_game_order.get(&left_game.id))
            .then_with(|| {
                title_sort_key(left_game, filter).cmp(&title_sort_key(right_game, filter))
            })
            .then_with(identity_tie_break),
        _ if filter.availability.starts_with("collection:") => filter
            .collection_game_order
            .get(&left_game.id)
            .cmp(&filter.collection_game_order.get(&right_game.id))
            .then_with(|| {
                title_sort_key(left_game, filter).cmp(&title_sort_key(right_game, filter))
            })
            .then_with(identity_tie_break),
        _ => title_sort_key(left_game, filter)
            .cmp(&title_sort_key(right_game, filter))
            .then_with(identity_tie_break),
    }
}

fn title_sort_key<'a>(game: &'a Game, filter: &'a Filter) -> Cow<'a, str> {
    if let Some(title) = filter.display_titles.get(&game.id) {
        Cow::Owned(title.to_lowercase())
    } else {
        Cow::Borrowed(
            game.search_key
                .split_once('\n')
                .map(|(title, _)| title)
                .unwrap_or(&game.search_key),
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::list_view::MetadataInput;

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
                    has_non_retail_release: false,
                    adult: false,
                    has_usa_release: false,
                    has_japan_release: false,
                    cooperative: "no".into(),
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
                    has_non_retail_release: false,
                    adult: false,
                    has_usa_release: false,
                    has_japan_release: false,
                    cooperative: "unknown".into(),
                    search_key: "outrun\narcade".into(),
                },
            ],
            ..Catalog::default()
        }
    }

    #[test]
    fn platform_search_combines_catalog_aliases_and_legacy_abbreviations() {
        let key = platform_search_key(
            "Nintendo Entertainment System",
            Some("Nintendo Family Computer,Famicom"),
        );
        for query in [
            "nintendo entertainment",
            "family computer",
            "famicom",
            "nes",
        ] {
            assert!(key.contains(query), "missing platform query {query:?}");
        }
        assert!(platform_search_key("Unknown Console", None).contains("unknown console"));
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
        assert_eq!(
            filter_indices(
                &catalog,
                &Filter {
                    availability: "recent".into(),
                    recent_game_order: Arc::new(std::collections::HashMap::from([
                        ("metroid".to_owned(), 10),
                        ("outrun".to_owned(), 20),
                    ])),
                    ..Filter::default()
                }
            ),
            vec![1, 0]
        );
    }

    #[test]
    fn exact_column_filters_compose_and_facets_ignore_their_own_selection() {
        let catalog = fixture_catalog();
        let platform_filter = ListColumnFilter {
            mode: crate::list_view::ListColumnFilterMode::Include,
            values: HashSet::from(["Arcade".to_owned()]),
        };
        let filter = Filter {
            list_column_filters: Arc::new(HashMap::from([(ListColumn::Platform, platform_filter)])),
            ..Filter::default()
        };
        assert_eq!(filter_indices(&catalog, &filter), vec![1]);

        let facets = list_facet_values(&catalog, &filter, ListColumn::Platform, "", 10);
        assert_eq!(facets.total_distinct, 2);
        assert_eq!(
            facets
                .values
                .iter()
                .map(|facet| (facet.value.as_str(), facet.game_count))
                .collect::<Vec<_>>(),
            vec![("Arcade", 1), ("Nintendo Entertainment System", 1)]
        );

        let filtered_facets =
            list_facet_values(&catalog, &filter, ListColumn::Platform, "nintendo", 10);
        assert_eq!(filtered_facets.total_distinct, 1);
        assert_eq!(filtered_facets.values[0].game_count, 1);
    }

    #[test]
    fn select_none_is_an_explicit_empty_result() {
        let catalog = fixture_catalog();
        let filter = Filter {
            list_column_filters: Arc::new(HashMap::from([(
                ListColumn::Availability,
                ListColumnFilter::none(),
            )])),
            ..Filter::default()
        };
        assert!(filter_indices(&catalog, &filter).is_empty());
    }

    #[test]
    fn display_title_overrides_are_searchable_and_sortable_without_replacing_identity() {
        let catalog = fixture_catalog();
        let display_titles = Arc::new(std::collections::HashMap::from([
            ("metroid".to_owned(), "Zebes Adventure".to_owned()),
            ("outrun".to_owned(), "Arcade Racer".to_owned()),
        ]));
        assert_eq!(
            filter_indices(
                &catalog,
                &Filter {
                    search: "zebes".into(),
                    display_titles: Arc::clone(&display_titles),
                    ..Filter::default()
                }
            ),
            vec![0]
        );
        assert_eq!(
            filter_indices(
                &catalog,
                &Filter {
                    display_titles,
                    ..Filter::default()
                }
            ),
            vec![1, 0]
        );
        assert_eq!(catalog.games[0].title, "Metroid");
        assert_eq!(catalog.games[0].id, "metroid");
    }

    #[test]
    fn explicit_library_sorting_is_deterministic_and_composes_with_filters() {
        let mut catalog = fixture_catalog();
        assert_eq!(
            filter_indices(
                &catalog,
                &Filter {
                    sort_field: "platform".into(),
                    ..Filter::default()
                }
            ),
            vec![1, 0]
        );
        assert_eq!(
            filter_indices(
                &catalog,
                &Filter {
                    sort_field: "availability".into(),
                    ..Filter::default()
                }
            ),
            vec![0, 1]
        );
        assert_eq!(
            filter_indices(
                &catalog,
                &Filter {
                    sort_field: "availability".into(),
                    sort_descending: true,
                    ..Filter::default()
                }
            ),
            vec![1, 0]
        );

        catalog.games[0].id = "z-metroid".into();
        catalog.games[0].platform = "Shared Platform".into();
        catalog.games[0].search_key = "metroid\nshared platform".into();
        catalog.games[1].id = "a-outrun".into();
        catalog.games[1].platform = "Shared Platform".into();
        catalog.games[1].search_key = "outrun\nshared platform".into();
        let mut metadata = ListMetadataBuilder::with_capacity(2);
        metadata
            .push(MetadataInput {
                publisher: Some("Zeta".into()),
                ..MetadataInput::default()
            })
            .unwrap();
        metadata
            .push(MetadataInput {
                publisher: Some("Alpha".into()),
                ..MetadataInput::default()
            })
            .unwrap();
        catalog.list_metadata = metadata.finish();
        assert_eq!(
            filter_indices(
                &catalog,
                &Filter {
                    sort_field: "platform".into(),
                    ..Filter::default()
                }
            ),
            vec![0, 1],
            "equal primary keys retain human title order before stable identity"
        );
        assert_eq!(
            filter_indices(
                &catalog,
                &Filter {
                    sort_field: "publisher".into(),
                    ..Filter::default()
                }
            ),
            vec![1, 0]
        );
    }

    #[test]
    fn user_tags_are_searchable_and_filter_by_exact_membership() {
        let catalog = fixture_catalog();
        let game_tags = Arc::new(HashMap::from([
            (
                "metroid".to_owned(),
                vec!["Couch Co-op".to_owned(), "Family".to_owned()],
            ),
            ("outrun".to_owned(), vec!["Arcade Night".to_owned()]),
        ]));
        assert_eq!(
            filter_indices(
                &catalog,
                &Filter {
                    search: "couch".into(),
                    game_tags: Arc::clone(&game_tags),
                    ..Filter::default()
                }
            ),
            vec![0]
        );
        assert_eq!(
            filter_indices(
                &catalog,
                &Filter {
                    tag: "family".into(),
                    platform: "Nintendo Entertainment System".into(),
                    game_tags: Arc::clone(&game_tags),
                    ..Filter::default()
                }
            ),
            vec![0]
        );
        assert!(
            filter_indices(
                &catalog,
                &Filter {
                    tag: "Arcade".into(),
                    game_tags,
                    ..Filter::default()
                }
            )
            .is_empty()
        );
    }

    #[test]
    fn custom_field_names_and_values_are_searchable_without_changing_identity() {
        let catalog = fixture_catalog();
        let game_custom_fields = Arc::new(HashMap::from([(
            "metroid".to_owned(),
            vec!["Cabinet".to_owned(), "Living room CRT".to_owned()],
        )]));
        for search in ["cabinet", "living room", "crt"] {
            assert_eq!(
                filter_indices(
                    &catalog,
                    &Filter {
                        search: search.into(),
                        game_custom_fields: Arc::clone(&game_custom_fields),
                        ..Filter::default()
                    }
                ),
                vec![0]
            );
        }
        assert_eq!(catalog.games[0].id, "metroid");
        assert_eq!(catalog.games[0].title, "Metroid");
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
    fn collection_filter_preserves_explicit_manual_order() {
        let catalog = fixture_catalog();
        let game_ids = HashSet::from(["metroid".to_owned(), "outrun".to_owned()]);
        let order =
            std::collections::HashMap::from([("outrun".to_owned(), 0), ("metroid".to_owned(), 1)]);
        assert_eq!(
            filter_indices(
                &catalog,
                &Filter {
                    availability: "collection:arcade-first".into(),
                    collection_game_ids: Arc::new(game_ids),
                    collection_game_order: Arc::new(order),
                    ..Filter::default()
                }
            ),
            vec![1, 0]
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
    fn release_filters_use_exact_region_and_category_facets() {
        assert_eq!(
            release_region_membership("Game (USA, Europe)", None),
            (true, false)
        );
        assert_eq!(
            release_region_membership("Game (Japan)", None),
            (false, true)
        );
        assert_eq!(
            release_region_membership("Game", Some("North America")),
            (true, false)
        );
        assert_eq!(
            release_region_membership("Japan Grand Prix", None),
            (false, false)
        );
        assert_eq!(
            release_region_membership("ゼルダの伝説", None),
            (false, true)
        );
        assert_eq!(
            release_region_membership("Zelda no Densetsu", None),
            (false, false),
            "Latin text must not be guessed as romanized Japanese"
        );

        let mut catalog = fixture_catalog();
        catalog.games[0].has_usa_release = true;
        catalog.games[0].adult = true;
        catalog.games[1].has_japan_release = true;
        catalog.games[1].non_retail = true;
        catalog.games[1].has_non_retail_release = true;

        assert_eq!(
            filter_indices(
                &catalog,
                &Filter {
                    include_usa_releases: true,
                    ..Filter::default()
                }
            ),
            vec![0]
        );
        assert_eq!(
            filter_indices(
                &catalog,
                &Filter {
                    include_usa_releases: true,
                    include_japan_releases: true,
                    ..Filter::default()
                }
            ),
            vec![0, 1]
        );
        assert_eq!(
            filter_indices(
                &catalog,
                &Filter {
                    include_adult_releases: true,
                    include_non_retail_releases: true,
                    hide_non_retail: true,
                    hide_adult: true,
                    ..Filter::default()
                }
            ),
            vec![0, 1]
        );
        assert_eq!(
            filter_indices(
                &catalog,
                &Filter {
                    include_japan_releases: true,
                    include_adult_releases: true,
                    ..Filter::default()
                }
            ),
            Vec::<usize>::new()
        );
    }

    #[test]
    fn exact_alternate_title_regions_recover_romanized_release_evidence() {
        let connection = Connection::open_in_memory().unwrap();
        connection
            .execute_batch(
                "CREATE TABLE game_alternate_names (
                   launchbox_db_id INTEGER NOT NULL,
                   alternate_name TEXT NOT NULL,
                   region TEXT
                 );
                 INSERT INTO game_alternate_names VALUES
                   (41, 'Zelda no Densetsu', 'Japan'),
                   (42, 'A North American Name', 'North America'),
                   (43, 'Unscoped Name', NULL);",
            )
            .unwrap();
        let mut games = vec![
            Game {
                launchbox_db_id: 41,
                ..Game::default()
            },
            Game {
                launchbox_db_id: 42,
                ..Game::default()
            },
            Game {
                launchbox_db_id: 43,
                ..Game::default()
            },
        ];

        apply_alternate_release_regions(&connection, &mut games).unwrap();

        assert!(games[0].has_japan_release);
        assert!(!games[0].has_usa_release);
        assert!(games[1].has_usa_release);
        assert!(!games[1].has_japan_release);
        assert!(!games[2].has_usa_release);
        assert!(!games[2].has_japan_release);
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

        let catalog = load_discovery_catalog_with_native_state(
            &canonical,
            &discovery_path,
            Some(&minerva_path),
            Some(&user_path),
            None,
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
    fn focused_preview_starts_with_the_exact_saved_game_and_platform() {
        let directory = tempfile::tempdir().unwrap();
        let canonical_path = directory.path().join("canonical.db");
        let discovery_path = directory.path().join("games.db");

        let canonical = Connection::open(&canonical_path).unwrap();
        canonical
            .execute_batch(
                "CREATE TABLE emulators (id TEXT PRIMARY KEY);
                 INSERT INTO emulators VALUES ('emu-1');",
            )
            .unwrap();

        let mut discovery = Connection::open(&discovery_path).unwrap();
        discovery
            .execute_batch(
                "CREATE TABLE platforms (id INTEGER PRIMARY KEY, name TEXT NOT NULL);
                 CREATE TABLE games (
                   id TEXT PRIMARY KEY, title TEXT NOT NULL, sort_title TEXT,
                   status TEXT, launchbox_db_id INTEGER, platform_id INTEGER NOT NULL
                 );
                 INSERT INTO platforms VALUES (1, 'Other System');
                 INSERT INTO platforms VALUES (2, 'Saved System');
                 INSERT INTO games VALUES
                   ('other-game', 'Aardvark', NULL, 'Released', 1, 1);",
            )
            .unwrap();
        let transaction = discovery.transaction().unwrap();
        for index in 0..300 {
            transaction
                .execute(
                    "INSERT INTO games VALUES (?1, ?2, NULL, 'Released', ?3, 2)",
                    params![
                        format!("saved-{index:03}"),
                        format!("Saved Game {index:03}"),
                        i64::from(index) + 10
                    ],
                )
                .unwrap();
        }
        transaction.commit().unwrap();
        drop(discovery);

        let preview = load_preview_from_sources(
            &canonical,
            &discovery_path,
            None,
            None,
            None,
            &CatalogPreviewFocus {
                platform: "Saved System".into(),
                game_uid: "saved-299".into(),
            },
        )
        .unwrap();

        assert_eq!(preview.total_game_count, 301);
        assert_eq!(preview.catalog.games.len(), 240);
        assert_eq!(preview.catalog.games[0].id, "saved-299");
        assert!(
            preview
                .catalog
                .games
                .iter()
                .all(|game| game.platform == "Saved System")
        );
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

    #[test]
    fn regional_and_revision_records_collapse_to_one_exact_release_family_card() {
        fn game(id: &str, title: &str, platform: &str, database_id: i64, local: bool) -> Game {
            let non_retail = is_non_retail_game(title, None);
            let adult = is_adult_game(title, None, None);
            let (has_usa_release, has_japan_release) = release_region_membership(title, None);
            Game {
                id: id.into(),
                launchbox_db_id: database_id,
                title: title.into(),
                platform: platform.into(),
                status: "Released".into(),
                local,
                downloadable: !local,
                non_retail,
                has_non_retail_release: non_retail,
                adult,
                has_usa_release,
                has_japan_release,
                cooperative: "unknown".into(),
                search_key: format!("{}\n{}", title.to_lowercase(), platform.to_lowercase()),
            }
        }

        let mut games = vec![
            game(
                "canonical",
                "Faxanadu",
                "Nintendo Entertainment System",
                1283,
                false,
            ),
            game(
                "europe-a",
                "Faxanadu (Europe)",
                "Nintendo Entertainment System",
                0,
                false,
            ),
            game(
                "europe-b",
                "Faxanadu (Europe)",
                "Nintendo Entertainment System",
                0,
                false,
            ),
            game(
                "usa-rev",
                "Faxanadu (USA) (Rev 1)",
                "Nintendo Entertainment System",
                0,
                false,
            ),
            game(
                "japan",
                "Faxanadu (Japan)",
                "Nintendo Entertainment System",
                0,
                false,
            ),
            game(
                "different",
                "Faxanadu: Revisioned",
                "Nintendo Entertainment System",
                462550,
                false,
            ),
            game(
                "wii",
                "Faxanadu (Europe) (NES) (Virtual Console)",
                "Nintendo Wii",
                0,
                false,
            ),
        ];
        let mut builder = ListMetadataBuilder::with_capacity(games.len());
        for _ in &games {
            builder.push_empty();
        }
        let mut metadata = builder.finish();
        apply_release_families(&mut games, &mut metadata);

        assert_eq!(games.len(), 3);
        let faxanadu = games
            .iter()
            .position(|game| game.title == "Faxanadu")
            .expect("canonical Faxanadu family card");
        assert_eq!(games[faxanadu].launchbox_db_id, 1283);
        assert!(games[faxanadu].search_key.contains("faxanadu (europe)"));
        assert!(games[faxanadu].has_usa_release);
        assert!(games[faxanadu].has_japan_release);
        assert_eq!(
            metadata.display_value(
                faxanadu,
                &games[faxanadu],
                ListColumn::Variants,
                &HashMap::new(),
            ),
            "4"
        );
        assert!(
            games
                .iter()
                .any(|game| game.title == "Faxanadu: Revisioned")
        );
        assert!(games.iter().any(|game| game.platform == "Nintendo Wii"));
    }
}
