use std::collections::{BTreeSet, HashMap, HashSet};
use std::fs::{self, OpenOptions};
use std::io::{Read, Write};
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex, OnceLock};
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use anyhow::{Context, Result, bail};
use directories::ProjectDirs;
use lava_torrent::torrent::v1::Torrent;
use rusqlite::OptionalExtension;
use sha1::{Digest, Sha1};
use url::{Host, Url};

use crate::arcade_download::{
    build_daphne_laserdisc_plans, build_hypseus_laserdisc_plans, build_mame_laserdisc_plans,
};
use crate::catalog;
use crate::download_plan::{
    DownloadPlan, TorrentPlanFile, build_exo_plan, build_optical_plan, exo_primary_priority,
};
use crate::exo_install::PreparedInstall;

const MAX_TORRENT_BYTES: u64 = 64 * 1024 * 1024;
const MAX_FILE_CANDIDATES: usize = 100;
const TORRENT_CACHE_MAX_AGE: Duration = Duration::from_secs(24 * 60 * 60);
type TorrentCache = Mutex<Option<(String, Arc<Vec<u8>>)>>;
type TorrentFileCache = Mutex<Option<(String, Arc<Vec<TorrentPlanFile>>)>>;
type MameRomsetCache = Mutex<HashMap<(PathBuf, u64, u64, String), Vec<String>>>;

#[derive(Clone, Debug, Default, PartialEq)]
pub struct GameDetails {
    pub id: String,
    pub title: String,
    pub platform: String,
    pub description: String,
    pub release_date: String,
    pub developer: String,
    pub publisher: String,
    pub genre: String,
    pub players: String,
    pub rating: String,
    pub rating_count: i64,
    pub esrb: String,
    pub release_type: String,
    pub sort_title: String,
    pub series: String,
    pub region: String,
    pub play_mode: String,
    pub version: String,
    pub release_status: String,
    pub cooperative: String,
    pub catalog_video_url: String,
    pub wikipedia_url: String,
    pub steam_app_id: i64,
    pub metadata_source: String,
    pub notes: String,
    pub tags: Vec<String>,
    pub custom_fields: Vec<crate::settings::GameCustomField>,
    pub canonical_metadata: crate::settings::GameMetadata,
    pub effective_metadata: crate::settings::GameMetadata,
    pub alternate_titles: Vec<AlternateTitle>,
    pub variants: Vec<GameVariant>,
    pub related_games: Vec<RelatedGame>,
    pub related_message: String,
    pub local: bool,
    pub downloadable: bool,
    pub database_id: i64,
    pub local_file_path: PathBuf,
    pub local_file_paths: Vec<PathBuf>,
    pub managed_installation: Option<crate::ingest::ManagedInstallationSummary>,
    pub prepared_install: Option<PreparedInstall>,
    pub activity: Option<crate::settings::PlayActivity>,
    pub sessions: Vec<crate::settings::PlaySession>,
    pub supplemental_media: crate::media::SupplementalMedia,
    pub video_media_key: String,
    pub video_progress: Option<crate::settings::MediaPlaybackProgress>,
    pub manual_transfer: Option<crate::settings::MediaTransfer>,
    pub bundles: Vec<MinervaBundle>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct AlternateTitle {
    pub name: String,
    pub region: String,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct GameVariant {
    pub id: String,
    pub title: String,
    pub release_label: String,
    pub region: String,
    pub version: String,
    pub launchbox_db_id: i64,
    pub local: bool,
    pub downloadable: bool,
    pub current: bool,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RelatedGame {
    pub id: String,
    pub title: String,
    pub platform: String,
    pub reason: String,
    pub launchbox_db_id: i64,
    pub local: bool,
    pub downloadable: bool,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct MinervaBundle {
    pub torrent_id: i64,
    pub torrent_url: String,
    pub source_kind: String,
    pub torrent_sha256: String,
    pub collection: String,
    pub provider_platform: String,
    pub rom_count: i64,
    pub total_size: u64,
    pub match_kind: BundleMatchKind,
}

#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub enum BundleMatchKind {
    MappedName,
    ProviderName,
    ExplicitFallback,
}

impl BundleMatchKind {
    pub fn label(self) -> &'static str {
        match self {
            Self::MappedName => "mapped platform",
            Self::ProviderName => "provider platform",
            Self::ExplicitFallback => "legacy platform rule",
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct TorrentFileCandidate {
    pub index: usize,
    pub filename: String,
    pub byte_size: u64,
    pub match_score: f64,
    pub matched_title: String,
    pub region: String,
    pub version: String,
    pub download_plan: Option<DownloadPlan>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ReleasePreferences {
    pub region_priority: Vec<String>,
    pub version_preference: String,
}

pub fn load(
    id: &str,
    title: &str,
    platform: &str,
    local: bool,
    downloadable: bool,
) -> Result<GameDetails> {
    let mut details = GameDetails {
        id: id.to_owned(),
        title: title.to_owned(),
        platform: platform.to_owned(),
        local,
        downloadable,
        ..GameDetails::default()
    };
    let settings_store = crate::settings::SettingsStore::open_default()?;
    details.activity = settings_store
        .play_activity(id)
        .context("loading play activity")?;
    details.sessions = settings_store
        .play_sessions(id, 200)
        .context("loading play-session history")?;

    if let Some(local_file_id) = id.strip_prefix("local-file:") {
        let state_path = crate::settings::state_database_path()?;
        load_local_only_details(&mut details, local_file_id, &state_path)?;
        load_prepared_state(&mut details)?;
        load_supplemental_media(&mut details)?;
        apply_metadata_override(&mut details)?;
        load_related_state(&mut details);
        return Ok(details);
    }

    if let Some(path) = catalog::requested_discovery_database_path() {
        let connection = catalog::open_read_only(&path, "Lunchbox discovery database")?;
        let row = connection
            .query_row(
                "SELECT coalesce(g.description, ''), coalesce(g.release_date, ''),
                        coalesce(g.developer, ''), coalesce(g.publisher, ''),
                        coalesce(g.genre, ''), coalesce(g.players, ''),
                        CASE WHEN g.rating IS NULL THEN '' ELSE printf('%.1f', g.rating) END,
                        coalesce(g.esrb, ''),
                        coalesce(g.release_type, ''), coalesce(g.sort_title, ''),
                        coalesce(g.series, ''), coalesce(g.region, ''),
                        coalesce(g.play_mode, ''), coalesce(g.version, ''),
                        CASE
                            WHEN lower(trim(coalesce(g.status, ''))) IN
                                 ('', 'canonical', 'deprecated', 'merged') THEN ''
                            ELSE trim(g.status)
                        END,
                        coalesce(g.notes, ''),
                        CASE WHEN g.cooperative=1 THEN 'yes'
                             WHEN g.cooperative=0 THEN 'no' ELSE 'unknown' END,
                        coalesce(g.rating_count, 0), coalesce(g.video_url, ''),
                        coalesce(g.wikipedia_url, ''), coalesce(g.steam_app_id, 0),
                        coalesce(g.metadata_source, ''),
                        coalesce(g.launchbox_db_id, 0)
                 FROM games g WHERE g.id=?1",
                [id],
                |row| {
                    Ok((
                        row.get::<_, String>(0)?,
                        row.get::<_, String>(1)?,
                        row.get::<_, String>(2)?,
                        row.get::<_, String>(3)?,
                        row.get::<_, String>(4)?,
                        row.get::<_, String>(5)?,
                        row.get::<_, String>(6)?,
                        row.get::<_, String>(7)?,
                        row.get::<_, String>(8)?,
                        row.get::<_, String>(9)?,
                        row.get::<_, String>(10)?,
                        row.get::<_, String>(11)?,
                        row.get::<_, String>(12)?,
                        row.get::<_, String>(13)?,
                        row.get::<_, String>(14)?,
                        row.get::<_, String>(15)?,
                        row.get::<_, String>(16)?,
                        row.get::<_, i64>(17)?,
                        row.get::<_, String>(18)?,
                        row.get::<_, String>(19)?,
                        row.get::<_, i64>(20)?,
                        row.get::<_, String>(21)?,
                        row.get::<_, i64>(22)?,
                    ))
                },
            )
            .optional()?;

        if let Some(row) = row {
            details.description = row.0;
            details.release_date = row.1;
            details.developer = row.2;
            details.publisher = row.3;
            details.genre = row.4;
            details.players = row.5;
            details.rating = row.6;
            details.esrb = row.7;
            details.release_type = row.8;
            details.sort_title = row.9;
            details.series = row.10;
            details.region = row.11;
            details.play_mode = row.12;
            details.version = row.13;
            details.release_status = row.14;
            details.notes = row.15;
            details.cooperative = row.16;
            details.rating_count = row.17.max(0);
            details.catalog_video_url = validated_catalog_web_url(&row.18);
            details.wikipedia_url = validated_catalog_web_url(&row.19);
            details.steam_app_id = row.20.max(0);
            details.metadata_source = row.21;
            details.database_id = row.22;
            details.alternate_titles = load_alternate_titles_from_connection(
                &connection,
                details.database_id,
                &details.title,
            )?;
            let settings = crate::settings::SettingsStore::open_default()?.load()?;
            details.variants = load_game_variants_from_connection(
                &connection,
                &details.id,
                &details.title,
                &ReleasePreferences {
                    region_priority: settings.region_priority,
                    version_preference: settings.version_preference,
                },
            )?;
        }
    }

    details.bundles = resolve_minerva_bundles(&details)?;
    details
        .bundles
        .extend(resolve_registered_torrent_bundles_for_game(
            &settings_store,
            &details.platform,
            &details.title,
            &details.alternate_titles,
        )?);
    details.downloadable = !details.local && !details.bundles.is_empty();
    let identities = details
        .variants
        .iter()
        .map(|variant| (variant.id.clone(), variant.launchbox_db_id))
        .collect::<Vec<_>>();
    let installed = catalog::installed_game_flags(&identities)?;
    let platform_downloadable = !details.bundles.is_empty();
    for (variant, local) in details.variants.iter_mut().zip(installed) {
        variant.local = local;
        variant.downloadable = !local && platform_downloadable;
    }
    load_prepared_state(&mut details)?;
    load_supplemental_media(&mut details)?;
    apply_metadata_override(&mut details)?;
    load_related_state(&mut details);
    Ok(details)
}

fn validated_catalog_web_url(value: &str) -> String {
    let value = value.trim();
    if value.is_empty() || value.len() > 4096 {
        return String::new();
    }
    let Ok(url) = Url::parse(value) else {
        return String::new();
    };
    if !matches!(url.scheme(), "http" | "https")
        || !url.username().is_empty()
        || url.password().is_some()
    {
        return String::new();
    }
    let Some(Host::Domain(host)) = url.host() else {
        return String::new();
    };
    if host.eq_ignore_ascii_case("localhost") || host.to_ascii_lowercase().ends_with(".localhost") {
        return String::new();
    }
    url.to_string()
}

fn apply_metadata_override(details: &mut GameDetails) -> Result<()> {
    let canonical = crate::settings::GameMetadata {
        title: details.title.clone(),
        sort_title: details.sort_title.clone(),
        description: details.description.clone(),
        release_date: editable_release_date(&details.release_date),
        developer: details.developer.clone(),
        publisher: details.publisher.clone(),
        genre: details.genre.clone(),
        players: details.players.clone(),
        rating: details.rating.clone(),
        esrb: details.esrb.clone(),
        release_type: details.release_type.clone(),
        series: details.series.clone(),
        region: details.region.clone(),
        play_mode: details.play_mode.clone(),
        version: details.version.clone(),
        release_status: details.release_status.clone(),
        cooperative: if details.cooperative.is_empty() {
            "unknown".to_owned()
        } else {
            details.cooperative.clone()
        },
        notes: details.notes.clone(),
    };
    let settings = crate::settings::SettingsStore::open_default()?;
    let metadata_override = settings
        .game_metadata_override(&details.id)
        .context("loading local game metadata overrides")?;
    details.tags = settings
        .game_tags(&details.id)
        .context("loading local game tags")?;
    details.custom_fields = settings
        .game_custom_fields(&details.id)
        .context("loading local game custom fields")?;
    let effective = metadata_override.apply(&canonical);
    details.title.clone_from(&effective.title);
    details.sort_title.clone_from(&effective.sort_title);
    details.description.clone_from(&effective.description);
    details.release_date.clone_from(&effective.release_date);
    details.developer.clone_from(&effective.developer);
    details.publisher.clone_from(&effective.publisher);
    details.genre.clone_from(&effective.genre);
    details.players.clone_from(&effective.players);
    details.rating.clone_from(&effective.rating);
    details.esrb.clone_from(&effective.esrb);
    details.release_type.clone_from(&effective.release_type);
    details.series.clone_from(&effective.series);
    details.region.clone_from(&effective.region);
    details.play_mode.clone_from(&effective.play_mode);
    details.version.clone_from(&effective.version);
    details.release_status.clone_from(&effective.release_status);
    details.cooperative.clone_from(&effective.cooperative);
    details.notes.clone_from(&effective.notes);
    details.canonical_metadata = canonical;
    details.effective_metadata = effective;
    Ok(())
}

const MAX_RELATED_QUERY_ROWS: i64 = 6_000;
const MAX_RELATED_GAMES: usize = 12;

#[derive(Clone, Debug)]
struct RelatedCandidate {
    id: String,
    platform: String,
    launchbox_db_id: i64,
    canonical: crate::settings::GameMetadata,
}

#[derive(Clone, Debug)]
struct RankedRelatedGame {
    game: RelatedGame,
    score: i32,
}

fn load_related_state(details: &mut GameDetails) {
    let started = std::time::Instant::now();
    match load_related_games(details) {
        Ok(related_games) => {
            details.related_message = if related_games.is_empty() {
                "No strong catalog relationships were found for this game.".to_owned()
            } else {
                "Ranked from exact catalog metadata; selecting a card keeps every game identity separate."
                    .to_owned()
            };
            details.related_games = related_games;
        }
        Err(error) => {
            details.related_games.clear();
            details.related_message = format!("Related games unavailable: {error:#}");
        }
    }
    if std::env::args_os().any(|argument| argument == "--related-games-ui-probe") {
        println!(
            "LUNCHBOX_RELATED_READY_MS={} games={} message={:?}",
            started.elapsed().as_millis(),
            details.related_games.len(),
            details.related_message
        );
    }
}

fn load_related_games(details: &GameDetails) -> Result<Vec<RelatedGame>> {
    let Some(path) = catalog::requested_discovery_database_path() else {
        return Ok(Vec::new());
    };
    let connection = catalog::open_read_only(&path, "Lunchbox discovery database")?;
    let store = crate::settings::SettingsStore::open_default()?;
    let overrides = store
        .all_game_metadata_overrides()
        .context("loading metadata overrides for related games")?;
    let preferences = store
        .load_library_preferences()
        .context("loading content preferences for related games")?;
    let mut related = load_related_games_from_connection(
        &connection,
        details,
        &overrides,
        preferences.hide_non_retail,
        preferences.hide_adult,
    )?;
    let identities = related
        .iter()
        .map(|game| (game.id.clone(), game.launchbox_db_id, game.platform.clone()))
        .collect::<Vec<_>>();
    let availability = catalog::game_availability_flags(&identities)
        .context("loading related-game availability")?;
    for (game, (local, downloadable)) in related.iter_mut().zip(availability) {
        game.local = local;
        game.downloadable = downloadable;
    }
    Ok(related)
}

fn load_related_games_from_connection(
    connection: &rusqlite::Connection,
    details: &GameDetails,
    overrides: &HashMap<String, crate::settings::GameMetadataOverride>,
    hide_non_retail: bool,
    hide_adult: bool,
) -> Result<Vec<RelatedGame>> {
    let reference = &details.effective_metadata;
    let series_key = metadata_key(&reference.series);
    let developer_key = metadata_key(&reference.developer);
    let publisher_key = metadata_key(&reference.publisher);
    let platform_key = metadata_key(&details.platform);
    if series_key.is_empty()
        && developer_key.is_empty()
        && publisher_key.is_empty()
        && platform_key.is_empty()
    {
        return Ok(Vec::new());
    }

    const COLUMNS: &str = "g.id, trim(g.title), trim(p.name), coalesce(g.launchbox_db_id, 0),
         coalesce(g.description, ''),
         CASE WHEN trim(coalesce(g.release_date, '')) <> '' THEN trim(g.release_date)
              WHEN g.release_year IS NOT NULL THEN cast(g.release_year AS TEXT)
              ELSE '' END,
         coalesce(g.developer, ''), coalesce(g.publisher, ''),
         coalesce(g.genre, ''), coalesce(g.players, ''),
         CASE WHEN g.rating IS NULL THEN '' ELSE printf('%.1f', g.rating) END,
         coalesce(g.esrb, ''), coalesce(g.release_type, ''),
         coalesce(g.sort_title, ''), coalesce(g.series, ''),
         coalesce(g.region, ''), coalesce(g.play_mode, ''),
         coalesce(g.version, ''),
         CASE WHEN lower(trim(coalesce(g.status, ''))) IN
                   ('', 'canonical', 'deprecated', 'merged') THEN ''
              ELSE trim(g.status) END,
         CASE WHEN g.cooperative=1 THEN 'yes'
              WHEN g.cooperative=0 THEN 'no' ELSE 'unknown' END,
         coalesce(g.notes, '')";
    let query = format!(
        "SELECT {COLUMNS}
         FROM games g
         JOIN platforms p ON p.id=g.platform_id
         WHERE g.id<>?1
           AND ((?2<>'' AND lower(trim(coalesce(g.series, '')))=?2)
                OR (?3<>'' AND lower(trim(coalesce(g.developer, '')))=?3)
                OR (?4<>'' AND lower(trim(coalesce(g.publisher, '')))=?4)
                OR lower(trim(p.name))=?5)
         ORDER BY
           ((?2<>'' AND lower(trim(coalesce(g.series, '')))=?2) * 8
            + (?3<>'' AND lower(trim(coalesce(g.developer, '')))=?3) * 4
            + (?4<>'' AND lower(trim(coalesce(g.publisher, '')))=?4) * 2
            + (lower(trim(p.name))=?5)) DESC,
           lower(trim(g.title)), g.id
         LIMIT ?6"
    );
    let mut statement = connection.prepare(&query)?;
    let rows = statement.query_map(
        rusqlite::params![
            details.id,
            series_key,
            developer_key,
            publisher_key,
            platform_key,
            MAX_RELATED_QUERY_ROWS,
        ],
        related_candidate_from_row,
    )?;
    let mut candidates = Vec::new();
    let mut seen = HashSet::new();
    for row in rows {
        let candidate = row?;
        seen.insert(candidate.id.clone());
        candidates.push(candidate);
    }
    drop(statement);

    let override_query = format!(
        "SELECT {COLUMNS}
         FROM games g
         JOIN platforms p ON p.id=g.platform_id
         WHERE g.id=?1"
    );
    let mut override_statement = connection.prepare(&override_query)?;
    for game_uid in overrides.keys() {
        if game_uid == &details.id || seen.contains(game_uid) {
            continue;
        }
        if let Some(candidate) = override_statement
            .query_row([game_uid], related_candidate_from_row)
            .optional()?
        {
            seen.insert(candidate.id.clone());
            candidates.push(candidate);
        }
    }

    let selected_title_key = related_title_key(&details.title);
    let mut ranked = candidates
        .into_iter()
        .filter_map(|candidate| {
            let effective = overrides
                .get(&candidate.id)
                .cloned()
                .unwrap_or_default()
                .apply(&candidate.canonical);
            if hide_non_retail
                && catalog::is_non_retail_game(&effective.title, Some(&effective.release_type))
            {
                return None;
            }
            if hide_adult
                && catalog::is_adult_game(
                    &effective.title,
                    Some(&effective.esrb),
                    Some(&effective.genre),
                )
            {
                return None;
            }
            if candidate.platform.eq_ignore_ascii_case(&details.platform)
                && related_title_key(&effective.title) == selected_title_key
            {
                return None;
            }
            let (score, reason) = related_game_score(
                reference,
                &details.platform,
                &effective,
                &candidate.platform,
            )?;
            Some(RankedRelatedGame {
                score,
                game: RelatedGame {
                    id: candidate.id,
                    title: effective.title,
                    platform: candidate.platform,
                    reason,
                    launchbox_db_id: candidate.launchbox_db_id,
                    local: false,
                    downloadable: false,
                },
            })
        })
        .collect::<Vec<_>>();
    ranked.sort_by(|left, right| {
        right
            .score
            .cmp(&left.score)
            .then_with(|| {
                left.game
                    .title
                    .to_lowercase()
                    .cmp(&right.game.title.to_lowercase())
            })
            .then_with(|| left.game.platform.cmp(&right.game.platform))
            .then_with(|| left.game.id.cmp(&right.game.id))
    });
    ranked.truncate(MAX_RELATED_GAMES);
    Ok(ranked.into_iter().map(|ranked| ranked.game).collect())
}

fn related_candidate_from_row(row: &rusqlite::Row<'_>) -> rusqlite::Result<RelatedCandidate> {
    Ok(RelatedCandidate {
        id: row.get(0)?,
        platform: row.get(2)?,
        launchbox_db_id: row.get(3)?,
        canonical: crate::settings::GameMetadata {
            title: row.get(1)?,
            description: row.get(4)?,
            release_date: row.get(5)?,
            developer: row.get(6)?,
            publisher: row.get(7)?,
            genre: row.get(8)?,
            players: row.get(9)?,
            rating: row.get(10)?,
            esrb: row.get(11)?,
            release_type: row.get(12)?,
            sort_title: row.get(13)?,
            series: row.get(14)?,
            region: row.get(15)?,
            play_mode: row.get(16)?,
            version: row.get(17)?,
            release_status: row.get(18)?,
            cooperative: row.get(19)?,
            notes: row.get(20)?,
        },
    })
}

fn related_game_score(
    reference: &crate::settings::GameMetadata,
    reference_platform: &str,
    candidate: &crate::settings::GameMetadata,
    candidate_platform: &str,
) -> Option<(i32, String)> {
    let same_series = metadata_equal(&reference.series, &candidate.series);
    let same_developer = metadata_equal(&reference.developer, &candidate.developer);
    let same_publisher = metadata_equal(&reference.publisher, &candidate.publisher);
    let same_platform = reference_platform.eq_ignore_ascii_case(candidate_platform);
    let same_play_mode = metadata_equal(&reference.play_mode, &candidate.play_mode);
    let reference_title_terms = metadata_title_terms(&reference.title);
    let candidate_title_terms = metadata_title_terms(&candidate.title);
    let shared_title_terms = reference_title_terms
        .intersection(&candidate_title_terms)
        .cloned()
        .collect::<BTreeSet<_>>();
    let reference_genres = metadata_terms(&reference.genre);
    let candidate_genres = metadata_terms(&candidate.genre);
    let shared_genres = reference_genres
        .intersection(&candidate_genres)
        .cloned()
        .collect::<BTreeSet<_>>();
    let mut score = 0;
    score += i32::from(same_series) * 140;
    score += i32::try_from(shared_title_terms.len().min(3)).unwrap_or(3) * 70;
    score += i32::from(same_developer) * 55;
    score += i32::from(same_publisher) * 30;
    score += i32::try_from(shared_genres.len().min(3)).unwrap_or(3) * 22;
    score += i32::from(same_platform) * 18;
    score += i32::from(same_play_mode) * 8;
    if let (Some(reference_year), Some(candidate_year)) = (
        metadata_year(&reference.release_date),
        metadata_year(&candidate.release_date),
    ) {
        let distance = reference_year.abs_diff(candidate_year);
        score += if distance <= 2 {
            8
        } else if distance <= 5 {
            4
        } else {
            0
        };
    }
    if score < 35 {
        return None;
    }

    let shared_genre = shared_genres
        .iter()
        .next()
        .map(|value| display_metadata_term(value))
        .unwrap_or_default();
    let reason = if same_series {
        format!("Same series · {}", reference.series.trim())
    } else if !shared_title_terms.is_empty() && same_developer {
        format!("Title match · {}", reference.developer.trim())
    } else if !shared_title_terms.is_empty() {
        "Related title".to_owned()
    } else if same_developer && !shared_genre.is_empty() {
        format!("{} · {}", reference.developer.trim(), shared_genre)
    } else if same_developer {
        format!("Same developer · {}", reference.developer.trim())
    } else if same_publisher && !shared_genre.is_empty() {
        format!("{} · {}", reference.publisher.trim(), shared_genre)
    } else if same_platform && !shared_genre.is_empty() {
        format!("{} · {}", reference_platform.trim(), shared_genre)
    } else if same_publisher {
        format!("Same publisher · {}", reference.publisher.trim())
    } else {
        "Related catalog metadata".to_owned()
    };
    Some((score, reason))
}

fn metadata_key(value: &str) -> String {
    value
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
        .to_lowercase()
}

fn metadata_equal(left: &str, right: &str) -> bool {
    let left = metadata_key(left);
    !left.is_empty() && left == metadata_key(right)
}

fn metadata_terms(value: &str) -> BTreeSet<String> {
    value
        .split([';', ',', '/', '|'])
        .map(metadata_key)
        .filter(|value| !value.is_empty())
        .collect()
}

fn metadata_title_terms(value: &str) -> BTreeSet<String> {
    const STOP_WORDS: [&str; 12] = [
        "and",
        "edition",
        "game",
        "of",
        "part",
        "remake",
        "the",
        "version",
        "with",
        "for",
        "from",
        "collection",
    ];
    value
        .split(|character: char| !character.is_alphanumeric())
        .map(str::trim)
        .filter(|term| term.chars().count() >= 3)
        .map(str::to_lowercase)
        .filter(|term| !STOP_WORDS.contains(&term.as_str()))
        .collect()
}

fn display_metadata_term(value: &str) -> String {
    let mut characters = value.chars();
    let Some(first) = characters.next() else {
        return String::new();
    };
    first.to_uppercase().chain(characters).collect()
}

fn metadata_year(value: &str) -> Option<i32> {
    value
        .trim()
        .get(..4)
        .filter(|year| year.bytes().all(|byte| byte.is_ascii_digit()))
        .and_then(|year| year.parse::<i32>().ok())
}

fn related_title_key(title: &str) -> String {
    catalog_release_base(title)
        .unwrap_or_else(|| title.trim().to_owned())
        .to_lowercase()
}

fn editable_release_date(value: &str) -> String {
    let value = value.trim();
    if value.len() >= 10
        && value.as_bytes().get(4) == Some(&b'-')
        && value.as_bytes().get(7) == Some(&b'-')
        && value.as_bytes()[..10]
            .iter()
            .enumerate()
            .all(|(index, byte)| matches!(index, 4 | 7) || byte.is_ascii_digit())
    {
        value[..10].to_owned()
    } else {
        value.to_owned()
    }
}

const MAX_GAME_VARIANTS: usize = 500;
const MAX_VARIANT_QUERY_ROWS: usize = 2_000;

#[derive(Clone, Debug)]
struct VariantCandidate {
    variant: GameVariant,
    metadata_score: usize,
}

fn load_game_variants_from_connection(
    connection: &rusqlite::Connection,
    selected_id: &str,
    selected_title: &str,
    preferences: &ReleasePreferences,
) -> Result<Vec<GameVariant>> {
    let Some(platform_id) = connection
        .query_row(
            "SELECT platform_id FROM games WHERE id=?1",
            [selected_id],
            |row| row.get::<_, i64>(0),
        )
        .optional()?
    else {
        return Ok(Vec::new());
    };
    let selected_identity = release_identity(selected_title);
    let base = selected_identity
        .as_ref()
        .map(|identity| identity.base.as_str())
        .unwrap_or(selected_title)
        .trim();
    if base.is_empty() {
        return Ok(Vec::new());
    }
    let escaped = escape_like_pattern(base);
    let parenthesized = format!("{escaped} (%");
    let bracketed = format!("{escaped} [%");
    let mut statement = connection.prepare(
        "SELECT id, trim(title), coalesce(launchbox_db_id, 0),
                (CASE WHEN trim(coalesce(description, '')) <> '' THEN 1 ELSE 0 END
                 + CASE WHEN trim(coalesce(release_date, '')) <> '' THEN 1 ELSE 0 END
                 + CASE WHEN trim(coalesce(developer, '')) <> '' THEN 1 ELSE 0 END
                 + CASE WHEN trim(coalesce(publisher, '')) <> '' THEN 1 ELSE 0 END
                 + CASE WHEN trim(coalesce(genre, '')) <> '' THEN 1 ELSE 0 END
                 + CASE WHEN trim(coalesce(notes, '')) <> '' THEN 1 ELSE 0 END)
         FROM games
         WHERE platform_id=?1
           AND (trim(title)=?2 COLLATE NOCASE
                OR trim(title) LIKE ?3 ESCAPE '\\'
                OR trim(title) LIKE ?4 ESCAPE '\\')
         ORDER BY CASE WHEN id=?6 THEN 0 ELSE 1 END, lower(trim(title)), id
         LIMIT ?5",
    )?;
    let rows = statement.query_map(
        rusqlite::params![
            platform_id,
            base,
            parenthesized,
            bracketed,
            i64::try_from(MAX_VARIANT_QUERY_ROWS).unwrap_or(i64::MAX),
            selected_id,
        ],
        |row| {
            Ok((
                row.get::<_, String>(0)?,
                row.get::<_, String>(1)?,
                row.get::<_, i64>(2)?,
                row.get::<_, usize>(3)?,
            ))
        },
    )?;

    let base_key = base.to_lowercase();
    let mut deduplicated = HashMap::<String, VariantCandidate>::new();
    for row in rows {
        let (id, title, launchbox_db_id, metadata_score) = row?;
        let identity = release_identity(&title);
        let candidate_base = identity
            .as_ref()
            .map(|identity| identity.base.as_str())
            .unwrap_or(title.as_str());
        if candidate_base.trim().to_lowercase() != base_key {
            continue;
        }
        if title.trim().to_lowercase() != base_key && identity.is_none() {
            continue;
        }
        let (region, version) = release_labels(&title);
        let release_label = identity
            .map(|identity| identity.label)
            .unwrap_or_else(|| "Original release".to_owned());
        let candidate = VariantCandidate {
            variant: GameVariant {
                current: id == selected_id,
                id,
                title: title.clone(),
                release_label,
                region,
                version,
                launchbox_db_id,
                local: false,
                downloadable: false,
            },
            metadata_score,
        };
        let key = title.trim().to_lowercase();
        match deduplicated.entry(key) {
            std::collections::hash_map::Entry::Vacant(entry) => {
                entry.insert(candidate);
            }
            std::collections::hash_map::Entry::Occupied(mut entry) => {
                if prefer_variant_representative(&candidate, entry.get()) {
                    entry.insert(candidate);
                }
            }
        }
    }
    let mut variants = deduplicated.into_values().collect::<Vec<_>>();
    if variants.len() <= 1 {
        return Ok(Vec::new());
    }
    variants.sort_by(|left, right| {
        variant_preference_order(&left.variant, &right.variant, preferences)
            .then_with(|| {
                left.variant
                    .title
                    .to_lowercase()
                    .cmp(&right.variant.title.to_lowercase())
            })
            .then_with(|| left.variant.id.cmp(&right.variant.id))
    });
    if variants.len() > MAX_GAME_VARIANTS {
        if let Some(current_index) = variants
            .iter()
            .position(|candidate| candidate.variant.current)
        {
            if current_index >= MAX_GAME_VARIANTS {
                let current = variants.remove(current_index);
                variants.truncate(MAX_GAME_VARIANTS - 1);
                variants.push(current);
            } else {
                variants.truncate(MAX_GAME_VARIANTS);
            }
        } else {
            variants.truncate(MAX_GAME_VARIANTS);
        }
    }
    Ok(variants
        .into_iter()
        .map(|candidate| candidate.variant)
        .collect())
}

fn prefer_variant_representative(left: &VariantCandidate, right: &VariantCandidate) -> bool {
    left.variant.current
        || (!right.variant.current
            && ((left.variant.launchbox_db_id > 0 && right.variant.launchbox_db_id <= 0)
                || ((left.variant.launchbox_db_id > 0) == (right.variant.launchbox_db_id > 0)
                    && (left.metadata_score > right.metadata_score
                        || (left.metadata_score == right.metadata_score
                            && left.variant.id < right.variant.id)))))
}

#[derive(Clone, Debug, Eq, PartialEq)]
struct ReleaseIdentity {
    base: String,
    label: String,
}

fn release_identity(title: &str) -> Option<ReleaseIdentity> {
    let mut base_end = title.trim_end().len();
    let mut labels = Vec::new();
    while base_end > 0 {
        let prefix = title[..base_end].trim_end();
        let (opening, closing) = match prefix.as_bytes().last().copied() {
            Some(b')') => ('(', ')'),
            Some(b']') => ('[', ']'),
            _ => break,
        };
        let Some(start) = prefix.rfind(opening) else {
            break;
        };
        let tag = prefix[start + opening.len_utf8()..prefix.len() - closing.len_utf8()].trim();
        if !is_catalog_release_tag(tag) {
            break;
        }
        labels.push(prefix[start..].to_owned());
        base_end = start;
    }
    if labels.is_empty() {
        return None;
    }
    labels.reverse();
    let base = title[..base_end].trim().to_owned();
    (!base.is_empty()).then(|| ReleaseIdentity {
        base,
        label: labels.join(" "),
    })
}

pub(crate) fn catalog_release_base(title: &str) -> Option<String> {
    release_identity(title).map(|identity| identity.base)
}

fn is_catalog_release_tag(tag: &str) -> bool {
    canonical_region_tag(tag).is_some()
        || is_version_tag(tag)
        || is_language_tag(tag)
        || matches!(
            tag.trim().to_ascii_lowercase().as_str(),
            "beta"
                | "demo"
                | "prototype"
                | "proto"
                | "sample"
                | "unlicensed"
                | "unl"
                | "aftermarket"
                | "homebrew"
                | "translation"
        )
}

fn is_language_tag(tag: &str) -> bool {
    let parts = tag
        .split([',', '/', '&', '+'])
        .map(str::trim)
        .filter(|part| !part.is_empty())
        .collect::<Vec<_>>();
    !parts.is_empty()
        && parts.iter().all(|part| {
            matches!(
                part.to_ascii_lowercase().as_str(),
                "en" | "fr"
                    | "de"
                    | "es"
                    | "it"
                    | "nl"
                    | "pt"
                    | "sv"
                    | "no"
                    | "da"
                    | "fi"
                    | "ja"
                    | "jp"
                    | "ko"
                    | "zh"
                    | "pl"
                    | "ru"
                    | "cs"
                    | "english"
                    | "french"
                    | "german"
                    | "spanish"
                    | "italian"
                    | "dutch"
                    | "portuguese"
                    | "japanese"
                    | "korean"
                    | "chinese"
            )
        })
}

fn escape_like_pattern(value: &str) -> String {
    value
        .replace('\\', "\\\\")
        .replace('%', "\\%")
        .replace('_', "\\_")
}

fn variant_preference_order(
    left: &GameVariant,
    right: &GameVariant,
    preferences: &ReleasePreferences,
) -> std::cmp::Ordering {
    crate::region_priority::priority_for_region(
        (!left.region.is_empty()).then_some(left.region.as_str()),
        &preferences.region_priority,
    )
    .cmp(&crate::region_priority::priority_for_region(
        (!right.region.is_empty()).then_some(right.region.as_str()),
        &preferences.region_priority,
    ))
    .then_with(|| {
        let left = revision_rank(&left.version);
        let right = revision_rank(&right.version);
        if preferences.version_preference == "original" {
            left.cmp(&right)
        } else {
            right.cmp(&left)
        }
    })
}

fn load_alternate_titles_from_connection(
    connection: &rusqlite::Connection,
    launchbox_db_id: i64,
    primary_title: &str,
) -> Result<Vec<AlternateTitle>> {
    if launchbox_db_id <= 0 {
        return Ok(Vec::new());
    }
    let table_count: i64 = connection.query_row(
        "SELECT count(*) FROM sqlite_schema
         WHERE type='table' AND name='game_alternate_names'",
        [],
        |row| row.get(0),
    )?;
    if table_count == 0 {
        return Ok(Vec::new());
    }

    let mut statement = connection.prepare(
        "SELECT trim(alternate_name), trim(coalesce(region, ''))
         FROM game_alternate_names
         WHERE launchbox_db_id=?1 AND trim(alternate_name) <> ''
         ORDER BY lower(trim(alternate_name)), lower(trim(coalesce(region, '')))",
    )?;
    let rows = statement.query_map([launchbox_db_id], |row| {
        Ok(AlternateTitle {
            name: row.get(0)?,
            region: row.get(1)?,
        })
    })?;
    let primary_title = primary_title.trim();
    let mut seen = HashSet::new();
    let mut titles = Vec::new();
    for title in rows {
        let title = title?;
        if title.name == primary_title {
            continue;
        }
        let key = (title.name.to_lowercase(), title.region.to_lowercase());
        if seen.insert(key) {
            titles.push(title);
        }
    }
    Ok(titles)
}

fn load_supplemental_media(details: &mut GameDetails) -> Result<()> {
    details.supplemental_media =
        crate::media::supplemental_media(&details.id, details.database_id)?;
    details.manual_transfer = crate::media_acquisition::load_manual_transfer(&details.id)?;
    let Some(video) = details.supplemental_media.video.as_ref() else {
        return Ok(());
    };
    details.video_media_key = crate::media::media_identity(&video.path)?;
    details.video_progress = crate::settings::SettingsStore::open_default()?
        .media_playback_progress(&details.id, &details.video_media_key)?;
    Ok(())
}

fn load_prepared_state(details: &mut GameDetails) -> Result<()> {
    if !details.local {
        return Ok(());
    }
    let store = crate::settings::SettingsStore::open_default()?;
    load_native_game_files(details, &store)?;
    details.managed_installation =
        crate::ingest::inspect_managed_installation(&store, &details.id)?;
    details.prepared_install = crate::exo_install::cached_install(&store, &details.id)?;
    Ok(())
}

fn load_native_game_files(
    details: &mut GameDetails,
    store: &crate::settings::SettingsStore,
) -> Result<()> {
    let connection = store.connection()?;
    let mut paths = std::mem::take(&mut details.local_file_paths);
    if !details.local_file_path.as_os_str().is_empty() {
        paths.push(details.local_file_path.clone());
    }

    let mut installed = connection.prepare(
        "SELECT r.path_display, r.path_bytes, r.path_encoding
         FROM installed_games g
         JOIN installed_game_files f ON f.game_uid=g.game_uid AND f.is_primary=1
         JOIN installed_file_receipts r
           ON r.path_encoding=f.path_encoding AND r.path_bytes=f.path_bytes
         WHERE g.game_uid=?1 OR (?2 > 0 AND g.launchbox_db_id=?2)
         ORDER BY g.installed_at DESC, r.path_display",
    )?;
    let rows = installed.query_map(rusqlite::params![details.id, details.database_id], |row| {
        Ok(crate::local_import::decode_path(
            row.get::<_, Vec<u8>>(1)?,
            &row.get::<_, String>(2)?,
            row.get::<_, String>(0)?,
        ))
    })?;
    paths.extend(rows.collect::<rusqlite::Result<Vec<_>>>()?);
    drop(installed);

    // State created before ownership receipts retained only a display path.
    // Keep it launchable, but never infer deletion authority from that row.
    let mut legacy_installed = connection.prepare(
        "SELECT g.file_path
         FROM installed_games g
         WHERE (g.game_uid=?1 OR (?2 > 0 AND g.launchbox_db_id=?2))
           AND NOT EXISTS (
               SELECT 1 FROM installed_game_files f WHERE f.game_uid=g.game_uid
           )
         ORDER BY g.installed_at DESC, g.file_path",
    )?;
    let rows = legacy_installed
        .query_map(rusqlite::params![details.id, details.database_id], |row| {
            row.get::<_, String>(0).map(PathBuf::from)
        })?;
    paths.extend(rows.collect::<rusqlite::Result<Vec<_>>>()?);
    drop(legacy_installed);

    let mut imported = connection.prepare(
        "SELECT path_display, path_bytes, path_encoding
         FROM local_rom_files
         WHERE included=1 AND availability='present'
           AND (game_uid=?1 OR (?2 > 0 AND launchbox_db_id=?2))
         ORDER BY relative_path_display COLLATE NOCASE, id",
    )?;
    let rows = imported.query_map(rusqlite::params![details.id, details.database_id], |row| {
        Ok((
            row.get::<_, String>(0)?,
            row.get::<_, Vec<u8>>(1)?,
            row.get::<_, String>(2)?,
        ))
    })?;
    for row in rows {
        let (display, bytes, encoding) = row?;
        paths.push(crate::local_import::decode_path(bytes, &encoding, display));
    }

    let mut seen = HashSet::new();
    paths.retain(|path| path.is_file() && seen.insert(path.clone()));
    if let Some(primary) = paths.first() {
        details.local_file_path = primary.clone();
    }
    details.local_file_paths = paths;
    Ok(())
}

fn load_local_only_details(details: &mut GameDetails, id: &str, state_path: &Path) -> Result<()> {
    let connection = catalog::open_read_only(state_path, "Lunchbox state database")?;
    let row = connection
        .query_row(
            "SELECT path_display, path_bytes, path_encoding, file_size,
                    match_state, match_method, sha1, md5
             FROM local_rom_files
             WHERE id=?1 AND included=1 AND availability='present'",
            [id],
            |row| {
                Ok((
                    row.get::<_, String>(0)?,
                    row.get::<_, Vec<u8>>(1)?,
                    row.get::<_, String>(2)?,
                    row.get::<_, i64>(3)?,
                    row.get::<_, String>(4)?,
                    row.get::<_, String>(5)?,
                    row.get::<_, String>(6)?,
                    row.get::<_, String>(7)?,
                ))
            },
        )
        .optional()?
        .with_context(|| format!("local collection file {id} is no longer available"))?;
    let path = crate::local_import::decode_path(row.1, &row.2, row.0);
    if !path.is_file() {
        bail!("local collection file is missing: {}", path.display());
    }
    details.description =
        "This file is in your local collection but has no proven catalog identity. Assigning metadata never changes identity unless an exact provider link or strong checksum is reviewed."
            .to_owned();
    details.release_type = "Local-only file".to_owned();
    details.notes = format!(
        "{}\n{} · {}\n{}{}",
        path.display(),
        format_bytes(u64::try_from(row.3).unwrap_or_default()),
        row.5,
        if row.6.is_empty() {
            String::new()
        } else {
            format!("SHA-1 {}", row.6)
        },
        if row.7.is_empty() {
            String::new()
        } else {
            format!(" · MD5 {}", row.7)
        },
    );
    details.local = true;
    details.downloadable = false;
    details.local_file_path = path.clone();
    details.local_file_paths = vec![path];
    Ok(())
}

pub fn resolve_minerva_bundles(details: &GameDetails) -> Result<Vec<MinervaBundle>> {
    let Some(path) = catalog::requested_minerva_database_path() else {
        return Ok(Vec::new());
    };
    resolve_minerva_bundles_from_path(details, &path)
}

fn resolve_minerva_bundles_from_path(
    details: &GameDetails,
    path: &Path,
) -> Result<Vec<MinervaBundle>> {
    let connection = catalog::open_read_only(path, "Minerva catalog")?;
    let platform_key = catalog::normalize_platform_key(&details.platform);
    let mut statement = connection.prepare(
        "SELECT t.id, t.torrent_url, coalesce(t.collection, ''),
                tp.minerva_platform, tp.rom_count, coalesce(t.total_size, 0),
                tp.lunchbox_platform_name
         FROM minerva_torrent_platforms tp
         JOIN minerva_torrents t ON t.id=tp.torrent_id",
    )?;
    let rows = statement.query_map([], |row| {
        Ok((
            row.get::<_, i64>(0)?,
            row.get::<_, String>(1)?,
            row.get::<_, String>(2)?,
            row.get::<_, String>(3)?,
            row.get::<_, i64>(4)?,
            row.get::<_, i64>(5)?,
            row.get::<_, Option<String>>(6)?,
        ))
    })?;

    let mut bundles = Vec::new();
    let mut seen = HashSet::new();
    for row in rows {
        let (
            torrent_id,
            torrent_url,
            collection,
            provider_platform,
            rom_count,
            total_size,
            mapped_platform_name,
        ) = row?;
        let provider_key = catalog::normalize_platform_key(&provider_platform);
        let match_kind = if mapped_platform_name
            .as_deref()
            .map(catalog::normalize_platform_key)
            .is_some_and(|name| name == platform_key)
        {
            Some(BundleMatchKind::MappedName)
        } else if provider_key == platform_key {
            Some(BundleMatchKind::ProviderName)
        } else if explicit_fallback_matches(&platform_key, &provider_key, &collection) {
            Some(BundleMatchKind::ExplicitFallback)
        } else {
            None
        };
        let Some(match_kind) = match_kind else {
            continue;
        };
        if !seen.insert((torrent_id, provider_platform.clone())) {
            continue;
        }
        bundles.push(MinervaBundle {
            torrent_id,
            torrent_url,
            source_kind: "minerva".to_owned(),
            torrent_sha256: String::new(),
            collection,
            provider_platform,
            rom_count,
            total_size: u64::try_from(total_size).unwrap_or_default(),
            match_kind,
        });
    }
    bundles.sort_by(|left, right| {
        bundle_source_priority(left)
            .cmp(&bundle_source_priority(right))
            .then_with(|| left.match_kind.cmp(&right.match_kind))
            .then_with(|| right.rom_count.cmp(&left.rom_count))
            .then_with(|| left.collection.cmp(&right.collection))
            .then_with(|| left.provider_platform.cmp(&right.provider_platform))
    });
    Ok(bundles)
}

fn resolve_registered_torrent_bundles_for_game(
    store: &crate::settings::SettingsStore,
    platform: &str,
    game_title: &str,
    alternate_titles: &[AlternateTitle],
) -> Result<Vec<MinervaBundle>> {
    let platform_key = catalog::normalize_platform_key(platform);
    let title_keys = registered_torrent_title_keys(game_title, alternate_titles);
    let mut bundles = Vec::new();
    for source in store.registered_torrent_sources_for_platform(&platform_key)? {
        if store
            .registered_torrent_members_for_title_keys(&source.info_hash, &title_keys)?
            .is_empty()
        {
            continue;
        }
        bundles.push(MinervaBundle {
            torrent_id: source.id,
            torrent_url: registered_torrent_locator(&source.info_hash),
            source_kind: "manual_torrent".to_owned(),
            torrent_sha256: source.torrent_sha256,
            collection: source.torrent_name,
            provider_platform: source.platform,
            rom_count: i64::from(source.file_count),
            total_size: source.total_bytes,
            match_kind: BundleMatchKind::MappedName,
        });
    }
    Ok(bundles)
}

fn bundle_source_priority(bundle: &MinervaBundle) -> u8 {
    let collection = bundle.collection.trim().to_ascii_lowercase();
    let platform = bundle.provider_platform.trim().to_ascii_lowercase();

    if collection == "no-intro" {
        if platform.contains("source code") {
            return 70;
        }
        if platform.contains("private") {
            return 60;
        }
        if platform.contains("aftermarket") {
            return 50;
        }
        if platform.contains("headerless") {
            return 5;
        }
        return 0;
    }
    match collection.as_str() {
        "redump" => 0,
        "tosec" => 10,
        "retroachievements" => 20,
        "finalburn neo" => 30,
        _ => 15,
    }
}

fn explicit_fallback_matches(platform_key: &str, provider_key: &str, collection: &str) -> bool {
    let platform_key = match platform_key {
        "arcade-laserdisc" | "arcade-pinball" => "arcade",
        other => other,
    };
    match platform_key {
        "atari-800" => provider_key == "atari" && collection == "TOSEC",
        "arcade" => {
            (collection.eq_ignore_ascii_case("MAME")
                && matches!(
                    provider_key,
                    "roms-merged" | "roms-split" | "roms-non-merged"
                ))
                || (collection.eq_ignore_ascii_case("Laserdisc Collection")
                    && matches!(provider_key, "mame" | "hypseus-singe" | "daphne"))
        }
        _ => false,
    }
}

pub fn load_torrent_files(
    bundle: &MinervaBundle,
    game_title: &str,
    alternate_titles: &[AlternateTitle],
    preferences: &ReleasePreferences,
) -> Result<Vec<TorrentFileCandidate>> {
    let plan_files = if bundle.source_kind == "manual_torrent" {
        registered_torrent_plan_files(
            &crate::settings::SettingsStore::open_default()?,
            bundle,
            game_title,
            alternate_titles,
        )?
    } else {
        indexed_torrent_files(&bundle.torrent_url)?
    };
    let files = plan_files
        .iter()
        .map(|file| TorrentFileCandidate {
            index: file.index,
            filename: file.filename.clone(),
            byte_size: file.byte_size,
            match_score: 0.0,
            matched_title: String::new(),
            region: String::new(),
            version: String::new(),
            download_plan: None,
        })
        .collect::<Vec<_>>();

    if bundle
        .collection
        .eq_ignore_ascii_case("Laserdisc Collection")
    {
        let provider_key = catalog::normalize_platform_key(&bundle.provider_platform);
        let mut candidates = Vec::new();
        for lookup_title in lookup_titles(game_title, alternate_titles) {
            let romset_names = mame_romset_names(lookup_title)?;
            let plans = match provider_key.as_str() {
                "mame" => build_mame_laserdisc_plans(&plan_files, lookup_title, &romset_names),
                "hypseus-singe" => {
                    build_hypseus_laserdisc_plans(&plan_files, lookup_title, &romset_names)
                }
                "daphne" => build_daphne_laserdisc_plans(&plan_files, lookup_title, &romset_names),
                _ => Vec::new(),
            };
            candidates.extend(machine_plan_candidates(plans, preferences, lookup_title));
        }
        let mut seen_plans = HashSet::new();
        candidates.retain(|candidate| {
            candidate.download_plan.as_ref().is_some_and(|plan| {
                seen_plans.insert(format!(
                    "{}:{}",
                    plan.kind,
                    plan.display_name.to_ascii_lowercase()
                ))
            })
        });
        candidates.sort_by(|left, right| {
            release_preference_order(left, right, preferences)
                .then_with(|| left.byte_size.cmp(&right.byte_size))
                .then_with(|| left.filename.cmp(&right.filename))
        });
        candidates.truncate(MAX_FILE_CANDIDATES);
        return Ok(candidates);
    }

    rank_file_candidates_for_platform(
        files,
        game_title,
        alternate_titles,
        preferences,
        Some(&bundle.provider_platform),
    )
}

fn registered_torrent_plan_files(
    store: &crate::settings::SettingsStore,
    bundle: &MinervaBundle,
    game_title: &str,
    alternate_titles: &[AlternateTitle],
) -> Result<Arc<Vec<TorrentPlanFile>>> {
    let info_hash = registered_torrent_info_hash(&bundle.torrent_url)?;
    let title_keys = registered_torrent_title_keys(game_title, alternate_titles);
    Ok(Arc::new(
        store
            .registered_torrent_members_for_title_keys(info_hash, &title_keys)?
            .into_iter()
            .map(|member| TorrentPlanFile {
                index: usize::try_from(member.index).unwrap_or(usize::MAX),
                filename: member.path,
                byte_size: member.byte_size,
            })
            .collect(),
    ))
}

fn registered_torrent_title_keys(
    game_title: &str,
    alternate_titles: &[AlternateTitle],
) -> Vec<String> {
    lookup_titles(game_title, alternate_titles)
        .into_iter()
        .map(torrent_member_title_key)
        .filter(|key| !key.is_empty())
        .collect::<BTreeSet<_>>()
        .into_iter()
        .collect()
}

pub(crate) fn alternate_title_probe() -> Result<String> {
    const ID: &str = "8ec34bc8-73d5-4e4a-be7b-73ed58e0dc9a";
    const TITLE: &str = "Pokémon Silver Version";
    let details = load(ID, TITLE, "Nintendo Game Boy Color", false, true)?;
    if details.alternate_titles.is_empty() {
        bail!("the exact-linked Pokémon Silver record has no alternate titles");
    }
    let bundle = details
        .bundles
        .iter()
        .find(|bundle| {
            bundle.collection.eq_ignore_ascii_case("No-Intro")
                && catalog::normalize_platform_key(&bundle.provider_platform)
                    == "nintendo-game-boy-color"
        })
        .context("the exact Nintendo Game Boy Color No-Intro bundle is unavailable")?;
    let candidates = load_torrent_files(
        bundle,
        &details.title,
        &details.alternate_titles,
        &ReleasePreferences {
            region_priority: crate::region_priority::default_region_priority(),
            version_preference: "latest".to_owned(),
        },
    )?;
    let candidate = candidates
        .iter()
        .find(|candidate| {
            !candidate.matched_title.is_empty() && candidate.matched_title != details.title
        })
        .context("no Minerva candidate matched an exact-linked alternate title")?;
    Ok(format!(
        "aliases={} matched_title={:?} score={:.2} file={:?}",
        details.alternate_titles.len(),
        candidate.matched_title,
        candidate.match_score,
        candidate.filename
    ))
}

pub(crate) fn variant_probe() -> Result<String> {
    const ID: &str = "9697a5eb-e0b4-4f24-8d43-672701414ee7";
    const TITLE: &str = "Super Mario Bros.";
    let path = catalog::requested_discovery_database_path()
        .context("--variant-probe requires a discovery games database")?;
    let connection = catalog::open_read_only(&path, "Lunchbox discovery database")?;
    let preferences = ReleasePreferences {
        region_priority: crate::region_priority::default_region_priority(),
        version_preference: "latest".to_owned(),
    };
    let variants = load_game_variants_from_connection(&connection, ID, TITLE, &preferences)?;
    if variants.len() < 3 {
        bail!(
            "expected at least three exact-platform Super Mario Bros. releases, found {}",
            variants.len()
        );
    }
    let current = variants
        .iter()
        .filter(|variant| variant.current)
        .collect::<Vec<_>>();
    if current.len() != 1 || current[0].id != ID {
        bail!("the canonical catalog UUID was not the sole current release");
    }
    let mut titles = HashSet::new();
    if !variants
        .iter()
        .all(|variant| titles.insert(variant.title.trim().to_lowercase()))
    {
        bail!("display release rows were not deduplicated by exact full title");
    }
    let target = variants
        .iter()
        .find(|variant| variant.title.contains("(Europe)"))
        .context("the preserved Europe release is unavailable")?;
    let switched =
        load_game_variants_from_connection(&connection, &target.id, &target.title, &preferences)?;
    let switched_current = switched
        .iter()
        .filter(|variant| variant.current)
        .collect::<Vec<_>>();
    if switched_current.len() != 1 || switched_current[0].id != target.id {
        bail!("switching releases did not preserve the target UUID");
    }
    Ok(format!(
        "releases={} selected_id={:?} switched_id={:?} switched_title={:?}",
        variants.len(),
        ID,
        target.id,
        target.title
    ))
}

pub(crate) fn faxanadu_candidate_probe() -> Result<String> {
    const ID: &str = "2d6cb4b2-a219-4c40-9b31-ac466a77e88c";
    const TITLE: &str = "Faxanadu";
    const PLATFORM: &str = "Nintendo Entertainment System";

    let started = std::time::Instant::now();
    let details = load(ID, TITLE, PLATFORM, false, true)?;
    let bundle = details
        .bundles
        .first()
        .context("Faxanadu has no Minerva source")?;
    if !bundle.collection.eq_ignore_ascii_case("No-Intro")
        || bundle
            .provider_platform
            .to_ascii_lowercase()
            .contains("private")
        || bundle
            .provider_platform
            .to_ascii_lowercase()
            .contains("aftermarket")
        || bundle
            .provider_platform
            .to_ascii_lowercase()
            .contains("source code")
    {
        bail!(
            "Faxanadu's primary source is not the standard No-Intro set: {} · {}",
            bundle.collection,
            bundle.provider_platform
        );
    }

    let settings = crate::settings::SettingsStore::open_default()?.load()?;
    let candidates = load_torrent_files(
        bundle,
        &details.title,
        &details.alternate_titles,
        &ReleasePreferences {
            region_priority: settings.region_priority.clone(),
            version_preference: settings.version_preference.clone(),
        },
    )?;
    let candidate = candidates
        .first()
        .context("the standard No-Intro source did not contain Faxanadu")?;
    if !candidate.filename.to_ascii_lowercase().contains("faxanadu") {
        bail!(
            "the first ranked file is not Faxanadu: {}",
            candidate.filename
        );
    }
    let store = crate::settings::SettingsStore::open_default()?;
    let preflight = crate::qbittorrent::inspect_enqueue(
        &settings,
        &store,
        &crate::qbittorrent::EnqueueRequest {
            game_id: ID.to_owned(),
            launchbox_db_id: details.database_id,
            title: TITLE.to_owned(),
            platform: PLATFORM.to_owned(),
            source_kind: "minerva".to_owned(),
            torrent_url: bundle.torrent_url.clone(),
            torrent_bytes: torrent_bytes(bundle)?,
            selected_file_index: u32::try_from(candidate.index)
                .context("Faxanadu torrent file index is too large")?,
            selected_file_path: candidate.filename.clone(),
            download_plan: candidate.download_plan.clone(),
            adopt_existing_torrent: false,
        },
    )?;
    if !preflight.can_queue {
        bail!("Faxanadu download is blocked: {}", preflight.blocked_reason);
    }

    Ok(format!(
        "elapsed_ms={} source={:?} platform={:?} score={:.2} file={:?} download_path={:?} install_path={:?}",
        started.elapsed().as_millis(),
        bundle.collection,
        bundle.provider_platform,
        candidate.match_score,
        candidate.filename,
        preflight.download_path,
        preflight.install_path
    ))
}

fn lookup_titles<'a>(
    primary_title: &'a str,
    alternate_titles: &'a [AlternateTitle],
) -> Vec<&'a str> {
    let mut seen = HashSet::new();
    std::iter::once(primary_title)
        .chain(alternate_titles.iter().map(|title| title.name.as_str()))
        .filter(|title| !title.trim().is_empty())
        .filter(|title| seen.insert(title.trim().to_lowercase()))
        .collect()
}

fn indexed_torrent_files(url: &str) -> Result<Arc<Vec<TorrentPlanFile>>> {
    static CACHE: OnceLock<TorrentFileCache> = OnceLock::new();
    let cache = CACHE.get_or_init(|| Mutex::new(None));
    if let Ok(guard) = cache.lock()
        && let Some((cached_url, files)) = guard.as_ref()
        && cached_url == url
    {
        return Ok(Arc::clone(files));
    }

    let bytes = fetch_torrent(url)?;
    let files = Arc::new(torrent_plan_files_from_bytes(bytes.as_slice())?);
    if let Ok(mut guard) = cache.lock() {
        *guard = Some((url.to_owned(), Arc::clone(&files)));
    }
    Ok(files)
}

fn torrent_plan_files_from_bytes(bytes: &[u8]) -> Result<Vec<TorrentPlanFile>> {
    let torrent = Torrent::read_from_bytes(bytes)
        .map_err(|error| anyhow::anyhow!("could not parse Minerva torrent metadata: {error}"))?;
    if let Some(torrent_files) = torrent.files {
        torrent_files
            .iter()
            .enumerate()
            .map(|(index, file)| {
                Ok(TorrentPlanFile {
                    index,
                    filename: file
                        .path
                        .components()
                        .map(|component| component.as_os_str().to_string_lossy())
                        .collect::<Vec<_>>()
                        .join("/"),
                    byte_size: u64::try_from(file.length)
                        .context("torrent file has a negative byte length")?,
                })
            })
            .collect::<Result<Vec<_>>>()
    } else {
        Ok(vec![TorrentPlanFile {
            index: 0,
            filename: torrent.name,
            byte_size: u64::try_from(torrent.length)
                .context("torrent payload has a negative byte length")?,
        }])
    }
}

fn mame_romset_names(game_title: &str) -> Result<Vec<String>> {
    let Some(path) = catalog::requested_database_path() else {
        return Ok(Vec::new());
    };
    static CACHE: OnceLock<MameRomsetCache> = OnceLock::new();
    let canonical_path = path.canonicalize().unwrap_or(path);
    let metadata = canonical_path.metadata().ok();
    let length = metadata.as_ref().map_or(0, fs::Metadata::len);
    let modified = metadata
        .and_then(|metadata| metadata.modified().ok())
        .and_then(|modified| modified.duration_since(UNIX_EPOCH).ok())
        .map_or(0, |duration| duration.as_secs());
    let title_key = normalized_words(title_without_tags(game_title));
    let key = (canonical_path.clone(), length, modified, title_key);
    let cache = CACHE.get_or_init(|| Mutex::new(HashMap::new()));
    if let Ok(guard) = cache.lock()
        && let Some(names) = guard.get(&key)
    {
        return Ok(names.clone());
    }

    let connection = catalog::open_read_only(&canonical_path, "Lunchbox canonical database")?;
    let names = mame_romset_names_from_connection(&connection, game_title)?;
    if let Ok(mut guard) = cache.lock() {
        guard.insert(key, names.clone());
    }
    Ok(names)
}

fn mame_romset_names_from_connection(
    connection: &rusqlite::Connection,
    game_title: &str,
) -> Result<Vec<String>> {
    let query = normalized_words(title_without_tags(game_title));
    let mut statement = connection.prepare(
        "SELECT r.title, r.file_name
         FROM libretro_records r
         JOIN libretro_databases d ON d.id=r.database_id
         WHERE d.source_name='MAME'
           AND r.title IS NOT NULL
           AND r.file_name IS NOT NULL",
    )?;
    let rows = statement.query_map([], |row| {
        Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?))
    })?;
    let mut names = BTreeSet::new();
    for row in rows {
        let (provider_title, file_name) = row?;
        if normalized_words(title_without_tags(&provider_title)) != query {
            continue;
        }
        let path = Path::new(&file_name);
        if !path
            .extension()
            .and_then(|extension| extension.to_str())
            .is_some_and(|extension| extension.eq_ignore_ascii_case("zip"))
        {
            continue;
        }
        if let Some(stem) = path.file_stem().and_then(|stem| stem.to_str())
            && !stem.is_empty()
        {
            names.insert(stem.to_ascii_lowercase());
        }
    }
    Ok(names.into_iter().collect())
}

fn machine_plan_candidates(
    plans: Vec<DownloadPlan>,
    preferences: &ReleasePreferences,
    matched_title: &str,
) -> Vec<TorrentFileCandidate> {
    let mut candidates = plans
        .into_iter()
        .filter_map(|plan| {
            let representative = plan.representative_member()?;
            let (region, version) = release_labels(&representative.torrent_path);
            Some(TorrentFileCandidate {
                index: representative.index,
                filename: representative.torrent_path.clone(),
                byte_size: plan.total_bytes(),
                match_score: 1.0,
                matched_title: matched_title.to_owned(),
                region,
                version,
                download_plan: Some(plan),
            })
        })
        .collect::<Vec<_>>();
    candidates.sort_by(|left, right| {
        release_preference_order(left, right, preferences)
            .then_with(|| left.byte_size.cmp(&right.byte_size))
            .then_with(|| left.filename.cmp(&right.filename))
    });
    candidates.truncate(MAX_FILE_CANDIDATES);
    candidates
}

fn fetch_torrent(url: &str) -> Result<Arc<Vec<u8>>> {
    static CACHE: OnceLock<TorrentCache> = OnceLock::new();
    static HTTP: OnceLock<ureq::Agent> = OnceLock::new();
    let cache = CACHE.get_or_init(|| Mutex::new(None));
    if let Ok(guard) = cache.lock()
        && let Some((cached_url, bytes)) = guard.as_ref()
        && cached_url == url
    {
        return Ok(Arc::clone(bytes));
    }
    if let Some(bytes) = read_cached_torrent(url) {
        let bytes = Arc::new(bytes);
        if let Ok(mut guard) = cache.lock() {
            *guard = Some((url.to_owned(), Arc::clone(&bytes)));
        }
        return Ok(bytes);
    }

    let encoded_url = url.replace(' ', "%20");
    let http = HTTP.get_or_init(|| {
        ureq::Agent::config_builder()
            .timeout_connect(Some(Duration::from_secs(4)))
            .timeout_global(Some(Duration::from_secs(30)))
            .build()
            .into()
    });
    let mut last_error = None;
    for attempt in 0..2 {
        if attempt > 0 {
            std::thread::sleep(Duration::from_millis(250 * attempt));
        }
        match http
            .get(&encoded_url)
            .header("User-Agent", "Lunchbox/0.1 Minerva metadata browser")
            .call()
        {
            Ok(mut response) => {
                if response
                    .headers()
                    .get(ureq::http::header::CONTENT_LENGTH)
                    .and_then(|value| value.to_str().ok())
                    .and_then(|value| value.parse::<u64>().ok())
                    .is_some_and(|length| length > MAX_TORRENT_BYTES)
                {
                    bail!("Minerva torrent metadata exceeds the 64 MiB safety limit");
                }
                let mut bytes = Vec::new();
                if let Err(error) = response
                    .body_mut()
                    .as_reader()
                    .take(MAX_TORRENT_BYTES + 1)
                    .read_to_end(&mut bytes)
                {
                    last_error = Some(format!("reading Minerva torrent metadata: {error}"));
                    continue;
                }
                if bytes.len() as u64 > MAX_TORRENT_BYTES {
                    bail!("Minerva torrent metadata exceeds the 64 MiB safety limit");
                }
                if Torrent::read_from_bytes(&bytes).is_ok()
                    && let Some(path) = torrent_cache_path(url)
                {
                    let _ = write_torrent_cache_file(&path, &bytes);
                }
                let bytes = Arc::new(bytes);
                if let Ok(mut guard) = cache.lock() {
                    *guard = Some((url.to_owned(), Arc::clone(&bytes)));
                }
                return Ok(bytes);
            }
            Err(error) => last_error = Some(error.to_string()),
        }
    }
    Err(anyhow::anyhow!(
        "could not fetch Minerva torrent metadata from {url}: {}",
        last_error.unwrap_or_else(|| "unknown network error".to_owned())
    ))
}

fn torrent_cache_path(url: &str) -> Option<PathBuf> {
    ProjectDirs::from("com", "Lunchbox", "Lunchbox")
        .map(|dirs| torrent_cache_path_in(dirs.cache_dir(), url))
}

fn torrent_cache_path_in(cache_root: &Path, url: &str) -> PathBuf {
    let digest = Sha1::digest(url.as_bytes());
    cache_root
        .join("minerva-torrents")
        .join(format!("{digest:x}.torrent"))
}

fn read_cached_torrent(url: &str) -> Option<Vec<u8>> {
    read_torrent_cache_file(&torrent_cache_path(url)?)
}

fn read_torrent_cache_file(path: &Path) -> Option<Vec<u8>> {
    let metadata = path.metadata().ok()?;
    if metadata.len() == 0 || metadata.len() > MAX_TORRENT_BYTES {
        return None;
    }
    let age = SystemTime::now()
        .duration_since(metadata.modified().ok()?)
        .unwrap_or_default();
    if age > TORRENT_CACHE_MAX_AGE {
        return None;
    }
    let bytes = fs::read(path).ok()?;
    if bytes.len() as u64 != metadata.len() || Torrent::read_from_bytes(&bytes).is_err() {
        return None;
    }
    Some(bytes)
}

fn write_torrent_cache_file(path: &Path, bytes: &[u8]) -> Result<()> {
    let parent = path
        .parent()
        .context("Minerva torrent cache path has no parent")?;
    fs::create_dir_all(parent)
        .with_context(|| format!("creating Minerva torrent cache {}", parent.display()))?;
    let temporary = parent.join(format!(
        ".{}.{}.tmp",
        path.file_name()
            .and_then(|name| name.to_str())
            .unwrap_or("torrent"),
        uuid::Uuid::new_v4()
    ));
    let result = (|| -> Result<()> {
        let mut file = OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&temporary)
            .with_context(|| format!("creating {}", temporary.display()))?;
        file.write_all(bytes)
            .with_context(|| format!("writing {}", temporary.display()))?;
        file.sync_all()
            .with_context(|| format!("syncing {}", temporary.display()))?;
        drop(file);
        match fs::rename(&temporary, path) {
            Ok(()) => Ok(()),
            Err(_) if path.is_file() => {
                fs::remove_file(path)
                    .with_context(|| format!("replacing stale cache {}", path.display()))?;
                fs::rename(&temporary, path).with_context(|| {
                    format!(
                        "moving Minerva torrent cache {} to {}",
                        temporary.display(),
                        path.display()
                    )
                })
            }
            Err(error) => Err(error).with_context(|| {
                format!(
                    "moving Minerva torrent cache {} to {}",
                    temporary.display(),
                    path.display()
                )
            }),
        }
    })();
    if result.is_err() {
        let _ = fs::remove_file(&temporary);
    }
    result
}

pub(crate) fn torrent_bytes(bundle: &MinervaBundle) -> Result<Vec<u8>> {
    torrent_bytes_for_url(&bundle.torrent_url)
}

pub(crate) fn torrent_bytes_for_url(url: &str) -> Result<Vec<u8>> {
    if let Ok(info_hash) = registered_torrent_info_hash(url) {
        return crate::settings::SettingsStore::open_default()?.registered_torrent_bytes(info_hash);
    }
    Ok(fetch_torrent(url)?.as_ref().clone())
}

const REGISTERED_TORRENT_LOCATOR_PREFIX: &str = "registered-torrent:";

fn registered_torrent_locator(info_hash: &str) -> String {
    format!(
        "{REGISTERED_TORRENT_LOCATOR_PREFIX}{}",
        info_hash.to_ascii_lowercase()
    )
}

fn registered_torrent_info_hash(locator: &str) -> Result<&str> {
    let info_hash = locator
        .strip_prefix(REGISTERED_TORRENT_LOCATOR_PREFIX)
        .context("torrent source is not a registered local catalog")?;
    if info_hash.len() != 40 || !info_hash.bytes().all(|byte| byte.is_ascii_hexdigit()) {
        bail!("registered torrent source has an invalid info hash");
    }
    Ok(info_hash)
}

#[cfg(test)]
pub(crate) fn rank_file_candidates(
    files: Vec<TorrentFileCandidate>,
    game_title: &str,
    alternate_titles: &[AlternateTitle],
    preferences: &ReleasePreferences,
) -> Result<Vec<TorrentFileCandidate>> {
    rank_file_candidates_for_platform(files, game_title, alternate_titles, preferences, None)
}

pub(crate) fn rank_file_candidates_for_platform(
    files: Vec<TorrentFileCandidate>,
    game_title: &str,
    alternate_titles: &[AlternateTitle],
    preferences: &ReleasePreferences,
    platform: Option<&str>,
) -> Result<Vec<TorrentFileCandidate>> {
    let queries = lookup_titles(game_title, alternate_titles)
        .into_iter()
        .filter_map(|title| {
            let query = normalized_words(title_without_tags(title));
            (!query.is_empty()).then_some((title, query))
        })
        .collect::<Vec<_>>();
    if queries.is_empty() {
        bail!("game title has no searchable characters");
    }
    let plan_files = files
        .iter()
        .map(|file| TorrentPlanFile {
            index: file.index,
            filename: file.filename.clone(),
            byte_size: file.byte_size,
        })
        .collect::<Vec<_>>();
    let mut candidates = files
        .into_iter()
        .filter_map(|mut file| {
            for (title, query) in &queries {
                let tokens = significant_tokens(query);
                let score = file_match_score(&file.filename, query, &tokens);
                if score > file.match_score {
                    file.match_score = score;
                    file.matched_title = (*title).to_owned();
                }
            }
            (file.region, file.version) = release_labels(&file.filename);
            (file.match_score >= 0.35 && platform_payload_tier(platform, &file).is_some())
                .then_some(file)
        })
        .collect::<Vec<_>>();
    candidates.sort_by(|left, right| {
        platform_payload_tier(platform, left)
            .cmp(&platform_payload_tier(platform, right))
            .then_with(|| right.match_score.total_cmp(&left.match_score))
            .then_with(|| release_preference_order(left, right, preferences))
            .then_with(|| {
                exo_primary_priority(&left.filename)
                    .unwrap_or(u8::MAX)
                    .cmp(&exo_primary_priority(&right.filename).unwrap_or(u8::MAX))
            })
            .then_with(|| left.byte_size.cmp(&right.byte_size))
            .then_with(|| left.filename.cmp(&right.filename))
    });
    let mut seen_plans = HashSet::new();
    candidates.retain_mut(|candidate| {
        let Some(plan) = build_optical_plan(&plan_files, candidate.index)
            .or_else(|| build_exo_plan(&plan_files, candidate.index))
        else {
            return true;
        };
        let plan_key = format!("{}:{}", plan.kind, plan.display_name.to_ascii_lowercase());
        if !seen_plans.insert(plan_key) {
            return false;
        }
        if let Some(representative) = plan_files
            .iter()
            .find(|file| file.index == plan.representative_index)
        {
            candidate.index = representative.index;
            candidate.filename = representative.filename.clone();
            (candidate.region, candidate.version) = release_labels(&candidate.filename);
        }
        candidate.byte_size = plan.total_bytes();
        candidate.download_plan = Some(plan);
        true
    });
    candidates.truncate(MAX_FILE_CANDIDATES);
    Ok(candidates)
}

pub(crate) fn platform_payload_tier(
    platform: Option<&str>,
    candidate: &TorrentFileCandidate,
) -> Option<u8> {
    if is_known_auxiliary_payload(candidate) {
        return None;
    }
    let Some(platform) = platform else {
        return Some(0);
    };
    let platform_key = catalog::normalize_platform_key(platform);
    if !platform_key.split('-').any(|part| part == "switch") {
        return Some(0);
    }

    // A Switch catalog association is stronger than a filename label: only formats an
    // emulator can plausibly launch may become the automatic payload. This prevents tiny
    // metadata or padding files with an exact title from beating an actual game image.
    if candidate.byte_size < 1_024 {
        return None;
    }
    let extension = Path::new(&candidate.filename)
        .extension()
        .and_then(|extension| extension.to_str())
        .unwrap_or_default()
        .to_ascii_lowercase();
    match extension.as_str() {
        "xci" | "nsp" | "nsz" | "xcz" => Some(0),
        "nro" | "nso" => Some(1),
        "zip" | "7z" | "rar" => Some(2),
        _ => None,
    }
}

fn is_known_auxiliary_payload(candidate: &TorrentFileCandidate) -> bool {
    let path = Path::new(&candidate.filename);
    let extension = path
        .extension()
        .and_then(|extension| extension.to_str())
        .unwrap_or_default()
        .to_ascii_lowercase();

    // MIG dumps place card metadata beside the image inside a directory whose
    // name itself ends in .xci/.nsp. Only the actual image is launchable. This
    // structural rule remains valid when a dump omits the usual marker names.
    let nested_below_game_image = path
        .parent()
        .and_then(Path::file_name)
        .and_then(|name| Path::new(name).extension())
        .and_then(|extension| extension.to_str())
        .map(str::to_ascii_lowercase)
        .is_some_and(|parent_extension| {
            matches!(parent_extension.as_str(), "xci" | "nsp" | "nsz" | "xcz")
                && extension != parent_extension
        });
    if nested_below_game_image {
        return true;
    }

    if extension != "bin" {
        return false;
    }

    // A 12-byte exact-title file is metadata, never a playable ROM. Keep this
    // deliberately below even the smallest historical cartridge payloads.
    if candidate.byte_size < 128 {
        return true;
    }
    if candidate.byte_size > 4 * 1024 {
        return false;
    }
    let filename = candidate.filename.to_ascii_lowercase();
    [
        "(card id set)",
        "(card uid)",
        "(certificate)",
        "(initial data)",
    ]
    .iter()
    .any(|marker| filename.contains(marker))
}

fn file_match_score(filename: &str, query: &str, query_tokens: &[&str]) -> f64 {
    let stem = Path::new(filename)
        .file_stem()
        .and_then(|name| name.to_str())
        .unwrap_or(filename);
    let candidate = normalized_words(title_without_tags(stem));
    if candidate == query {
        return 1.0;
    }
    if candidate.starts_with(query) {
        return 0.72;
    }
    if candidate.contains(query) {
        return 0.65;
    }
    if query_tokens.is_empty() {
        return 0.0;
    }
    let candidate_tokens = significant_tokens(&candidate);
    let matches = query_tokens
        .iter()
        .filter(|token| candidate_tokens.contains(token))
        .count();
    let ratio = matches as f64 / query_tokens.len() as f64;
    if matches == query_tokens.len() {
        0.55
    } else if matches >= 2 {
        0.35 + ratio * 0.2
    } else {
        0.0
    }
}

pub(crate) fn torrent_file_match_score(filename: &str, game_title: &str) -> f64 {
    let query = normalized_words(title_without_tags(game_title));
    if query.is_empty() {
        return 0.0;
    }
    let query_tokens = significant_tokens(&query);
    file_match_score(filename, &query, &query_tokens)
}

fn title_without_tags(value: &str) -> &str {
    value
        .find(['(', '['])
        .map(|index| value[..index].trim_end())
        .unwrap_or(value)
}

pub(crate) fn torrent_member_title_key(path: &str) -> String {
    let stem = Path::new(path)
        .file_stem()
        .and_then(|name| name.to_str())
        .unwrap_or(path);
    normalized_words(title_without_tags(stem))
}

fn release_labels(filename: &str) -> (String, String) {
    let mut region = String::new();
    let mut version = String::new();
    for tag in delimited_tags(filename) {
        if region.is_empty()
            && let Some(canonical) = canonical_region_tag(tag)
        {
            region = canonical;
        }
        if version.is_empty() && is_version_tag(tag) {
            version = tag.trim().to_owned();
        }
    }
    (region, version)
}

fn delimited_tags(value: &str) -> Vec<&str> {
    let mut tags = Vec::new();
    for (opening, closing) in [('(', ')'), ('[', ']')] {
        let mut remainder = value;
        while let Some(start) = remainder.find(opening) {
            let after_opening = &remainder[start + opening.len_utf8()..];
            let Some(end) = after_opening.find(closing) else {
                break;
            };
            tags.push(&after_opening[..end]);
            remainder = &after_opening[end + closing.len_utf8()..];
        }
    }
    tags
}

fn canonical_region_tag(tag: &str) -> Option<String> {
    let parts = tag
        .split([',', '/', '&', '+'])
        .map(str::trim)
        .filter(|part| !part.is_empty())
        .map(canonical_region)
        .collect::<Option<Vec<_>>>()?;
    (!parts.is_empty()).then(|| parts.join(", "))
}

fn canonical_region(region: &str) -> Option<&'static str> {
    Some(match region.trim().to_ascii_lowercase().as_str() {
        "usa" | "united states" | "north america" => "USA",
        "japan" => "Japan",
        "asia" => "Asia",
        "world" => "World",
        "europe" => "Europe",
        "australia" => "Australia",
        "canada" => "Canada",
        "brazil" => "Brazil",
        "korea" => "Korea",
        "china" => "China",
        "france" => "France",
        "germany" => "Germany",
        "italy" => "Italy",
        "spain" => "Spain",
        "united kingdom" | "uk" => "United Kingdom",
        "taiwan" => "Taiwan",
        "netherlands" => "Netherlands",
        "belgium" => "Belgium",
        "greece" => "Greece",
        "portugal" => "Portugal",
        "austria" => "Austria",
        "sweden" => "Sweden",
        "finland" => "Finland",
        "russia" => "Russia",
        "switzerland" => "Switzerland",
        "hong kong" => "Hong Kong",
        "scandinavia" => "Scandinavia",
        "denmark" => "Denmark",
        "poland" => "Poland",
        "norway" => "Norway",
        "new zealand" => "New Zealand",
        "latin america" => "Latin America",
        "unknown" => "Unknown",
        _ => return None,
    })
}

fn is_version_tag(tag: &str) -> bool {
    let lower = tag.trim().to_ascii_lowercase();
    ["rev ", "revision ", "ver ", "version "]
        .iter()
        .any(|prefix| lower.starts_with(prefix))
        || lower
            .strip_prefix('v')
            .is_some_and(|suffix| suffix.starts_with(|character: char| character.is_ascii_digit()))
}

fn release_preference_order(
    left: &TorrentFileCandidate,
    right: &TorrentFileCandidate,
    preferences: &ReleasePreferences,
) -> std::cmp::Ordering {
    crate::region_priority::priority_for_region(
        (!left.region.is_empty()).then_some(left.region.as_str()),
        &preferences.region_priority,
    )
    .cmp(&crate::region_priority::priority_for_region(
        (!right.region.is_empty()).then_some(right.region.as_str()),
        &preferences.region_priority,
    ))
    .then_with(|| {
        let left = revision_rank(&left.version);
        let right = revision_rank(&right.version);
        if preferences.version_preference == "original" {
            left.cmp(&right)
        } else {
            right.cmp(&left)
        }
    })
}

fn revision_rank(version: &str) -> (bool, [u32; 4]) {
    let mut parts = [0; 4];
    let mut index = 0;
    let mut number = String::new();
    for character in version.chars().chain(std::iter::once(' ')) {
        if character.is_ascii_digit() {
            number.push(character);
        } else if !number.is_empty() {
            if index < parts.len() {
                parts[index] = number.parse().unwrap_or(u32::MAX);
                index += 1;
            }
            number.clear();
        }
    }
    (!version.is_empty(), parts)
}

fn normalized_words(value: &str) -> String {
    let mut normalized = String::with_capacity(value.len());
    let mut separator = false;
    for character in value.chars().flat_map(char::to_lowercase) {
        if character.is_alphanumeric() {
            if separator && !normalized.is_empty() {
                normalized.push(' ');
            }
            normalized.push(character);
            separator = false;
        } else {
            separator = true;
        }
    }
    normalized
        .split_whitespace()
        .map(|token| match token {
            "ii" => "2",
            "iii" => "3",
            "iv" => "4",
            "vi" => "6",
            "vii" => "7",
            "viii" => "8",
            "ix" => "9",
            "xi" => "11",
            "xii" => "12",
            "xiii" => "13",
            "xiv" => "14",
            "xv" => "15",
            "xvi" => "16",
            "xvii" => "17",
            "xviii" => "18",
            "xix" => "19",
            "xx" => "20",
            _ => token,
        })
        .collect::<Vec<_>>()
        .join(" ")
}

fn significant_tokens(value: &str) -> Vec<&str> {
    const STOP_WORDS: &[&str] = &["a", "an", "and", "for", "in", "of", "on", "or", "the", "to"];
    value
        .split_whitespace()
        .filter(|token| token.len() > 1 && !STOP_WORDS.contains(token))
        .collect()
}

pub fn format_bytes(bytes: u64) -> String {
    const KIB: f64 = 1024.0;
    const MIB: f64 = KIB * 1024.0;
    const GIB: f64 = MIB * 1024.0;
    let bytes = bytes as f64;
    if bytes >= GIB {
        format!("{:.1} GiB", bytes / GIB)
    } else if bytes >= MIB {
        format!("{:.1} MiB", bytes / MIB)
    } else if bytes >= KIB {
        format!("{:.1} KiB", bytes / KIB)
    } else {
        format!("{} B", bytes as u64)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn preferences(region: &str, version: &str) -> ReleasePreferences {
        ReleasePreferences {
            region_priority: vec![region.into()],
            version_preference: version.into(),
        }
    }

    fn candidate(index: usize, filename: &str) -> TorrentFileCandidate {
        TorrentFileCandidate {
            index,
            filename: filename.into(),
            byte_size: 10,
            match_score: 0.0,
            matched_title: String::new(),
            region: String::new(),
            version: String::new(),
            download_plan: None,
        }
    }

    #[test]
    fn metadata_editor_uses_a_portable_date_without_creating_a_timezone_override() {
        assert_eq!(
            editable_release_date("1985-09-13T00:00:00+00:00"),
            "1985-09-13"
        );
        assert_eq!(editable_release_date("1994"), "1994");
        assert_eq!(editable_release_date("September 1985"), "September 1985");
    }

    fn related_metadata(
        title: &str,
        series: &str,
        developer: &str,
        publisher: &str,
        genre: &str,
        release_date: &str,
    ) -> crate::settings::GameMetadata {
        crate::settings::GameMetadata {
            title: title.into(),
            series: series.into(),
            developer: developer.into(),
            publisher: publisher.into(),
            genre: genre.into(),
            release_date: release_date.into(),
            play_mode: "Single Player".into(),
            ..crate::settings::GameMetadata::default()
        }
    }

    #[test]
    fn related_game_scoring_requires_meaningful_metadata_and_prioritizes_series() {
        let reference = related_metadata(
            "Star Runner",
            "Star Saga",
            "Orion Works",
            "Astra",
            "Action; Racing",
            "2000-01-01",
        );
        let series = related_metadata(
            "Star Runner II",
            "Star Saga",
            "Another Studio",
            "Another Publisher",
            "Puzzle",
            "2015-01-01",
        );
        let studio = related_metadata(
            "Orbit Racer",
            "",
            "Orion Works",
            "Another Publisher",
            "Racing",
            "2001-01-01",
        );
        let unrelated = related_metadata(
            "Platform Neighbor",
            "",
            "Another Studio",
            "Another Publisher",
            "Puzzle",
            "2015-01-01",
        );

        let (series_score, series_reason) =
            related_game_score(&reference, "Arcadia", &series, "Elsewhere").unwrap();
        let (studio_score, studio_reason) =
            related_game_score(&reference, "Arcadia", &studio, "Arcadia").unwrap();

        assert!(series_score > studio_score);
        assert_eq!(series_reason, "Same series · Star Saga");
        assert_eq!(studio_reason, "Orion Works · Racing");
        assert!(related_game_score(&reference, "Arcadia", &unrelated, "Arcadia").is_none());
    }

    #[test]
    fn related_game_query_keeps_exact_ids_filters_variants_and_honors_overrides() {
        let connection = rusqlite::Connection::open_in_memory().unwrap();
        connection
            .execute_batch(
                "CREATE TABLE platforms (
                    id INTEGER PRIMARY KEY,
                    name TEXT NOT NULL
                 );
                 CREATE TABLE games (
                    id TEXT PRIMARY KEY,
                    platform_id INTEGER NOT NULL,
                    title TEXT NOT NULL,
                    launchbox_db_id INTEGER,
                    description TEXT,
                    release_date TEXT,
                    release_year INTEGER,
                    developer TEXT,
                    publisher TEXT,
                    genre TEXT,
                    players TEXT,
                    rating REAL,
                    esrb TEXT,
                    release_type TEXT,
                    sort_title TEXT,
                    series TEXT,
                    region TEXT,
                    play_mode TEXT,
                    version TEXT,
                    status TEXT,
                    cooperative INTEGER,
                    notes TEXT
                 );
                 INSERT INTO platforms VALUES (1, 'Arcadia'), (2, 'Elsewhere');
                 INSERT INTO games VALUES
                    ('current', 1, 'Star Runner', 1, '', '2000-01-01', 2000,
                     'Orion Works', 'Astra', 'Action; Racing', '1', 4.0, 'E',
                     'Released', '', 'Star Saga', '', 'Single Player', '', '', 0, ''),
                    ('series-hit', 2, 'Star Runner II', 2, '', '2002-01-01', 2002,
                     'Other Studio', 'Other Publisher', 'Puzzle', '1', 4.0, 'E',
                     'Released', '', 'Star Saga', '', 'Single Player', '', '', 0, ''),
                    ('studio-hit', 1, 'Orbit Racer', 3, '', '2001-01-01', 2001,
                     'Orion Works', 'Other Publisher', 'Racing', '1', 4.0, 'E',
                     'Released', '', '', '', 'Single Player', '', '', 0, ''),
                    ('variant', 1, 'Star Runner (USA)', 4, '', '2000-01-01', 2000,
                     'Orion Works', 'Astra', 'Action; Racing', '1', 4.0, 'E',
                     'Released', '', 'Star Saga', 'USA', 'Single Player', '', '', 0, ''),
                    ('adult', 2, 'Star Nights', 5, '', '2003-01-01', 2003,
                     'Other Studio', 'Other Publisher', 'Action', '1', 4.0, 'AO',
                     'Released', '', 'Star Saga', '', 'Single Player', '', '', 0, ''),
                    ('homebrew', 2, 'Star Home', 6, '', '2003-01-01', 2003,
                     'Other Studio', 'Other Publisher', 'Action', '1', 4.0, 'E',
                     'Homebrew', '', 'Star Saga', '', 'Single Player', '', '', 0, ''),
                    ('override-hit', 2, 'Hidden Relation', 7, '', '2004-01-01', 2004,
                     'Different Studio', 'Different Publisher', 'Puzzle', '1', 4.0, 'E',
                     'Released', '', '', '', 'Single Player', '', '', 0, '');",
            )
            .unwrap();

        let reference = related_metadata(
            "Star Runner",
            "Star Saga",
            "Orion Works",
            "Astra",
            "Action; Racing",
            "2000-01-01",
        );
        let details = GameDetails {
            id: "current".into(),
            title: reference.title.clone(),
            platform: "Arcadia".into(),
            effective_metadata: reference,
            ..GameDetails::default()
        };
        let overrides = HashMap::from([(
            "override-hit".to_owned(),
            crate::settings::GameMetadataOverride {
                series: Some("Star Saga".into()),
                ..crate::settings::GameMetadataOverride::default()
            },
        )]);

        let related =
            load_related_games_from_connection(&connection, &details, &overrides, true, true)
                .unwrap();
        let ids = related
            .iter()
            .map(|game| game.id.as_str())
            .collect::<Vec<_>>();

        assert_eq!(ids, ["series-hit", "override-hit", "studio-hit"]);
        assert!(!ids.contains(&"variant"));
        assert!(!ids.contains(&"adult"));
        assert!(!ids.contains(&"homebrew"));
        assert!(related.len() <= MAX_RELATED_GAMES);
        assert_eq!(related[0].launchbox_db_id, 2);
        assert_eq!(related[1].reason, "Same series · Star Saga");
    }

    #[test]
    fn catalog_links_accept_only_bounded_public_web_urls() {
        assert_eq!(
            validated_catalog_web_url("https://en.wikipedia.org/wiki/Super_Mario_Bros."),
            "https://en.wikipedia.org/wiki/Super_Mario_Bros."
        );
        assert_eq!(
            validated_catalog_web_url("http://www.youtube.com/watch?v=cWOkHQXw0JQ"),
            "http://www.youtube.com/watch?v=cWOkHQXw0JQ"
        );
        assert!(validated_catalog_web_url("file:///tmp/game.html").is_empty());
        assert!(validated_catalog_web_url("https://user:secret@example.com/watch").is_empty());
        assert!(validated_catalog_web_url("http://127.0.0.1/private").is_empty());
        assert!(validated_catalog_web_url("https://[::1]/private").is_empty());
        assert!(validated_catalog_web_url("https://localhost/private").is_empty());
        assert!(validated_catalog_web_url("not a URL").is_empty());
        assert!(validated_catalog_web_url(&"x".repeat(4097)).is_empty());
    }

    #[cfg(unix)]
    #[test]
    fn managed_primary_file_loading_preserves_native_non_utf8_path_bytes() {
        use std::ffi::OsString;
        use std::os::unix::ffi::OsStringExt;

        let directory = tempfile::tempdir().unwrap();
        let store = crate::settings::SettingsStore::at(directory.path().join("state.db")).unwrap();
        let path = directory
            .path()
            .join(OsString::from_vec(b"managed-\xff.rom".to_vec()));
        fs::write(&path, b"native path fixture").unwrap();
        let job = crate::settings::DownloadJob::queued(crate::settings::NewDownloadJob {
            game_id: "native-path-game".into(),
            launchbox_db_id: 0,
            title: "Native Path".into(),
            platform: "Test".into(),
            source_kind: "minerva".into(),
            torrent_url: "https://example.invalid/native-path.torrent".into(),
            torrent_file_index: Some(0),
            torrent_file_path: "native.rom".into(),
            info_hash: "2".repeat(40),
            client_save_path: "/downloads/lunchbox".into(),
            local_download_path: directory.path().join("download/native.rom"),
            local_target_path: path.clone(),
            download_plan: String::new(),
        });
        let receipt = crate::ingest::installed_file_receipt(&path, true).unwrap();
        store.record_installed(&job, &path, &[receipt]).unwrap();
        let mut details = GameDetails {
            id: job.game_id,
            local: true,
            ..GameDetails::default()
        };

        load_native_game_files(&mut details, &store).unwrap();

        assert_eq!(details.local_file_path, path);
        assert_eq!(
            details.local_file_paths,
            vec![details.local_file_path.clone()]
        );
    }

    #[test]
    fn file_ranking_prefers_exact_title_without_accepting_identity() {
        let files = vec![
            candidate(0, "Nintendo/Game Boy/Super Mario Land (USA, Europe).zip"),
            candidate(1, "Nintendo/Game Boy/Mario Tennis (USA).zip"),
        ];
        let ranked = rank_file_candidates(
            files,
            "Super Mario Land",
            &[],
            &preferences("USA", "latest"),
        )
        .unwrap();
        assert_eq!(ranked.len(), 1);
        assert_eq!(ranked[0].index, 0);
        assert_eq!(ranked[0].match_score, 1.0);
        assert_eq!(ranked[0].matched_title, "Super Mario Land");
    }

    #[test]
    fn switch_ranking_prefers_a_launchable_image_over_a_tiny_exact_name() {
        let mut tiny_bin = candidate(0, "Super Mario Bros. RPG.bin");
        tiny_bin.byte_size = 12;
        let mut game_image = candidate(1, "Super Mario RPG [0100BC0018138000].xci");
        game_image.byte_size = 8 * 1024 * 1024 * 1024;

        let ranked = rank_file_candidates_for_platform(
            vec![tiny_bin, game_image],
            "Super Mario Bros. RPG",
            &[],
            &preferences("USA", "latest"),
            Some("Nintendo Switch"),
        )
        .unwrap();

        assert_eq!(ranked.len(), 1);
        assert_eq!(ranked[0].index, 1);
        assert!(ranked[0].filename.ends_with(".xci"));
    }

    #[test]
    fn mig_card_metadata_never_becomes_an_automatic_game_payload() {
        let root = "MIG Switch Game Collection/Super Mario RPG Legend of the Seven Stars.xci";
        let mut card_id = candidate(
            4400,
            &format!("{root}/Super Mario RPG Legend of the Seven Stars (Card ID Set).bin"),
        );
        card_id.byte_size = 12;
        let mut card_uid = candidate(
            4401,
            &format!("{root}/Super Mario RPG Legend of the Seven Stars (Card UID).bin"),
        );
        card_uid.byte_size = 64;
        let mut certificate = candidate(
            4402,
            &format!("{root}/Super Mario RPG Legend of the Seven Stars (Certificate).bin"),
        );
        certificate.byte_size = 512;
        let mut initial_data = candidate(
            4403,
            &format!("{root}/Super Mario RPG Legend of the Seven Stars (Initial Data).bin"),
        );
        initial_data.byte_size = 512;
        let mut game_image = candidate(
            4404,
            &format!("{root}/Super Mario RPG Legend of the Seven Stars.xci"),
        );
        game_image.byte_size = 7_260_093_952;

        let ranked = rank_file_candidates_for_platform(
            vec![card_id, card_uid, certificate, initial_data, game_image],
            "Super Mario RPG",
            &[],
            &preferences("USA", "latest"),
            None,
        )
        .unwrap();

        assert_eq!(ranked.first().map(|candidate| candidate.index), Some(4404));
        assert!(
            ranked
                .iter()
                .all(|candidate| candidate.byte_size > 4 * 1024)
        );
    }

    #[test]
    fn nested_mig_sidecar_without_a_known_label_is_not_a_game_payload() {
        let root = "MIG Switch/Super Mario RPG.xci";
        let mut sidecar = candidate(0, &format!("{root}/Super Mario RPG data.bin"));
        sidecar.byte_size = 512;
        let mut game_image = candidate(1, &format!("{root}/Super Mario RPG.xci"));
        game_image.byte_size = 7_260_093_952;

        let ranked = rank_file_candidates_for_platform(
            vec![sidecar, game_image],
            "Super Mario RPG",
            &[],
            &preferences("USA", "latest"),
            None,
        )
        .unwrap();

        assert_eq!(ranked.len(), 1);
        assert_eq!(ranked[0].index, 1);
    }

    #[test]
    fn switch_payload_format_never_establishes_title_identity() {
        let mut unrelated = candidate(0, "Completely Different Game.xci");
        unrelated.byte_size = 8 * 1024 * 1024 * 1024;

        let ranked = rank_file_candidates_for_platform(
            vec![unrelated],
            "Super Mario Bros. RPG",
            &[],
            &preferences("USA", "latest"),
            Some("Nintendo Switch"),
        )
        .unwrap();

        assert!(ranked.is_empty());
    }

    #[test]
    fn exact_linked_alternate_title_can_win_candidate_matching() {
        let files = vec![candidate(
            0,
            "Nintendo/Game Boy Color/Pokemon - Silver Version (USA, Europe).zip",
        )];
        let alternate_titles = vec![AlternateTitle {
            name: "Pokemon - Silver Version".into(),
            region: String::new(),
        }];
        let ranked = rank_file_candidates(
            files,
            "Pokémon Silver Version",
            &alternate_titles,
            &preferences("USA", "latest"),
        )
        .unwrap();
        assert_eq!(ranked.len(), 1);
        assert_eq!(ranked[0].match_score, 1.0);
        assert_eq!(ranked[0].matched_title, "Pokemon - Silver Version");
    }

    #[test]
    fn alternate_titles_require_an_exact_provider_link_and_deduplicate_rows() {
        let connection = rusqlite::Connection::open_in_memory().unwrap();
        connection
            .execute_batch(
                "CREATE TABLE game_alternate_names (
                    launchbox_db_id INTEGER NOT NULL,
                    alternate_name TEXT NOT NULL,
                    region TEXT
                 );
                 INSERT INTO game_alternate_names VALUES
                    (2361, 'Pokémon Silver Version', 'North America'),
                    (2361, 'Pokemon - Silver Version', NULL),
                    (2361, 'Pokemon - Silver Version', NULL),
                    (2361, 'Pocket Monsters Gin', 'Japan'),
                    (2360, 'Pokemon - Gold Version', NULL);",
            )
            .unwrap();
        let titles =
            load_alternate_titles_from_connection(&connection, 2361, "Pokémon Silver Version")
                .unwrap();
        assert_eq!(
            titles,
            vec![
                AlternateTitle {
                    name: "Pocket Monsters Gin".into(),
                    region: "Japan".into(),
                },
                AlternateTitle {
                    name: "Pokemon - Silver Version".into(),
                    region: String::new(),
                },
            ]
        );
        assert!(
            load_alternate_titles_from_connection(&connection, 0, "Anything")
                .unwrap()
                .is_empty()
        );
    }

    fn variant_fixture() -> rusqlite::Connection {
        let connection = rusqlite::Connection::open_in_memory().unwrap();
        connection
            .execute_batch(
                "CREATE TABLE games (
                    id TEXT PRIMARY KEY,
                    platform_id INTEGER NOT NULL,
                    title TEXT NOT NULL,
                    launchbox_db_id INTEGER,
                    description TEXT,
                    release_date TEXT,
                    developer TEXT,
                    publisher TEXT,
                    genre TEXT,
                    notes TEXT
                 );
                 INSERT INTO games VALUES
                    ('base', 1, 'Game', 0, '', '', '', '', '', ''),
                    ('usa-import', 1, 'Game (USA)', 0, '', '', '', '', '', ''),
                    ('usa-linked', 1, 'Game (USA)', 101, 'Rich metadata', '1990',
                     'Studio', 'Publisher', 'Action', 'Notes'),
                    ('japan', 1, 'Game (Japan) (Rev 2)', 102, '', '', '', '', '', ''),
                    ('subtitle', 1, 'Game (The Adventure)', 103, '', '', '', '', '', ''),
                    ('sequel', 1, 'Game 2 (USA)', 104, '', '', '', '', '', ''),
                    ('other-platform', 2, 'Game (Europe)', 105, '', '', '', '', '', '');",
            )
            .unwrap();
        connection
    }

    #[test]
    fn catalog_variants_are_bounded_to_exact_platform_and_recognized_release_tags() {
        let connection = variant_fixture();
        let variants = load_game_variants_from_connection(
            &connection,
            "base",
            "Game",
            &ReleasePreferences {
                region_priority: vec!["Japan".into(), "USA".into(), "Europe".into()],
                version_preference: "latest".into(),
            },
        )
        .unwrap();
        assert_eq!(
            variants
                .iter()
                .map(|variant| variant.id.as_str())
                .collect::<Vec<_>>(),
            ["japan", "usa-linked", "base"]
        );
        assert_eq!(variants[0].release_label, "(Japan) (Rev 2)");
        assert_eq!(variants[0].region, "Japan");
        assert_eq!(variants[0].version, "Rev 2");
        assert!(variants[2].current);
        assert!(!variants.iter().any(|variant| variant.id == "subtitle"));
        assert!(!variants.iter().any(|variant| variant.id == "sequel"));
        assert!(
            !variants
                .iter()
                .any(|variant| variant.id == "other-platform")
        );
    }

    #[test]
    fn selected_duplicate_keeps_its_exact_identity_without_provider_id_transfer() {
        let connection = variant_fixture();
        let variants = load_game_variants_from_connection(
            &connection,
            "usa-import",
            "Game (USA)",
            &preferences("USA", "latest"),
        )
        .unwrap();
        let selected = variants
            .iter()
            .find(|variant| variant.current)
            .expect("selected variant");
        assert_eq!(selected.id, "usa-import");
        assert_eq!(selected.launchbox_db_id, 0);
        assert_eq!(
            variants
                .iter()
                .filter(|variant| variant.title == "Game (USA)")
                .count(),
            1
        );
    }

    #[test]
    fn selected_variant_survives_query_and_display_safety_caps() {
        let mut connection = variant_fixture();
        let transaction = connection.transaction().unwrap();
        for revision in 0..=2_100 {
            transaction
                .execute(
                    "INSERT INTO games VALUES
                     (?1, 1, ?2, 0, '', '', '', '', '', '')",
                    rusqlite::params![
                        format!("revision-{revision}"),
                        format!("Game (Rev {revision})")
                    ],
                )
                .unwrap();
        }
        transaction.commit().unwrap();

        let variants = load_game_variants_from_connection(
            &connection,
            "revision-2100",
            "Game (Rev 2100)",
            &ReleasePreferences {
                region_priority: vec!["USA".into(), "Japan".into()],
                version_preference: "original".into(),
            },
        )
        .unwrap();
        assert_eq!(variants.len(), MAX_GAME_VARIANTS);
        assert_eq!(
            variants
                .iter()
                .filter(|variant| variant.current)
                .map(|variant| variant.id.as_str())
                .collect::<Vec<_>>(),
            ["revision-2100"]
        );
    }

    #[test]
    fn catalog_release_identity_does_not_strip_arbitrary_parenthetical_titles() {
        assert_eq!(
            release_identity("Game (USA) (En,Fr)"),
            Some(ReleaseIdentity {
                base: "Game".into(),
                label: "(USA) (En,Fr)".into(),
            })
        );
        assert_eq!(release_identity("Game (The Adventure)"), None);
        assert_eq!(
            release_identity("Game (The Adventure) (USA)").unwrap().base,
            "Game (The Adventure)"
        );
    }

    #[test]
    fn sequel_prefix_is_ranked_below_an_exact_region_variant() {
        let query = normalized_words("Super Mario Land");
        let tokens = significant_tokens(&query);
        assert_eq!(
            file_match_score("Super Mario Land (World).zip", &query, &tokens),
            1.0
        );
        assert_eq!(
            file_match_score("Super Mario Land 4 (Unknown).zip", &query, &tokens),
            0.72
        );
    }

    #[test]
    fn custom_region_order_and_version_rank_equal_title_matches() {
        let files = vec![
            candidate(0, "Game (USA) (Rev 1).zip"),
            candidate(1, "Game (Japan) (Rev 1).zip"),
            candidate(2, "Game (Japan) (Rev 3).zip"),
        ];
        let ranked =
            rank_file_candidates(files, "Game (USA)", &[], &preferences("Japan", "latest"))
                .unwrap();
        assert_eq!(
            ranked.iter().map(|file| file.index).collect::<Vec<_>>(),
            [2, 1, 0]
        );
        assert_eq!(ranked[0].region, "Japan");
        assert_eq!(ranked[0].version, "Rev 3");
    }

    #[test]
    fn complete_custom_region_order_is_consumed_by_candidate_ranking() {
        let files = vec![
            candidate(0, "Game (Japan).zip"),
            candidate(1, "Game (USA).zip"),
            candidate(2, "Game (Europe).zip"),
        ];
        let ranked = rank_file_candidates(
            files,
            "Game",
            &[],
            &ReleasePreferences {
                region_priority: vec!["Europe".into(), "USA".into(), "Japan".into()],
                version_preference: "latest".into(),
            },
        )
        .unwrap();
        assert_eq!(
            ranked.iter().map(|file| file.index).collect::<Vec<_>>(),
            [2, 1, 0]
        );
    }

    #[test]
    fn original_version_policy_prefers_untagged_release() {
        let files = vec![
            candidate(0, "Game (Europe) (Rev 2).zip"),
            candidate(1, "Game (Europe).zip"),
        ];
        let ranked =
            rank_file_candidates(files, "Game", &[], &preferences("Europe", "original")).unwrap();
        assert_eq!(
            ranked.iter().map(|file| file.index).collect::<Vec<_>>(),
            [1, 0]
        );
    }

    #[test]
    fn multidisc_candidates_collapse_into_one_reviewable_plan() {
        let mut files = vec![
            candidate(0, "Sony/Final Fantasy VII (USA) (Disc 1).cue"),
            candidate(1, "Sony/Final Fantasy VII (USA) (Disc 1) (Track 01).bin"),
            candidate(2, "Sony/Final Fantasy VII (USA) (Disc 2).cue"),
            candidate(3, "Sony/Final Fantasy VII (USA) (Disc 2) (Track 01).bin"),
        ];
        files[0].byte_size = 100;
        files[1].byte_size = 1_000;
        files[2].byte_size = 200;
        files[3].byte_size = 2_000;

        let ranked = rank_file_candidates(
            files,
            "Final Fantasy VII",
            &[],
            &preferences("USA", "latest"),
        )
        .unwrap();
        assert_eq!(ranked.len(), 1);
        let plan = ranked[0].download_plan.as_ref().expect("optical plan");
        assert_eq!(ranked[0].index, 0);
        assert_eq!(ranked[0].byte_size, 3_300);
        assert_eq!(plan.disc_count(), 2);
        assert_eq!(plan.members.len(), 4);
    }

    #[test]
    fn exo_candidates_collapse_dependencies_into_one_reviewable_plan() {
        let mut files = vec![
            candidate(0, "Content/GameData/eXoDOS/Prince of Persia (1990).zip"),
            candidate(1, "Content/!DOSmetadata.zip"),
            candidate(2, "eXo/eXoDOS/Prince of Persia (1990).zip"),
            candidate(3, "eXo/util/util.zip"),
            candidate(
                4,
                "Spanish Language Pack/eXo/eXoDOS/!spanish/Prince of Persia (1990).zip",
            ),
        ];
        files[0].byte_size = 10;
        files[1].byte_size = 20;
        files[2].byte_size = 30;
        files[3].byte_size = 40;
        files[4].byte_size = 1;

        let ranked = rank_file_candidates(
            files,
            "Prince of Persia",
            &[],
            &preferences("USA", "latest"),
        )
        .unwrap();
        assert_eq!(ranked.len(), 1);
        assert_eq!(ranked[0].index, 2);
        assert_eq!(ranked[0].byte_size, 100);
        let plan = ranked[0].download_plan.as_ref().expect("eXo plan");
        assert!(plan.is_exo_archive_set());
        assert_eq!(plan.members.len(), 4);
    }

    #[test]
    fn release_labels_do_not_mistake_language_lists_for_regions() {
        assert_eq!(
            release_labels("Game (USA, Europe) (En,Fr,De) (v1.2).zip"),
            ("USA, Europe".to_owned(), "v1.2".to_owned())
        );
        assert_eq!(
            release_labels("Game (En,Ja).zip"),
            (String::new(), String::new())
        );
    }

    #[test]
    fn roman_numeral_sequels_match_arabic_titles() {
        let query = normalized_words("Galaxy Force II");
        let tokens = significant_tokens(&query);
        assert_eq!(
            file_match_score("Galaxy Force 2 (USA).zip", &query, &tokens),
            1.0
        );
    }

    #[test]
    fn platform_fallbacks_are_explicit_and_bounded() {
        assert!(explicit_fallback_matches("atari-800", "atari", "TOSEC"));
        assert!(!explicit_fallback_matches(
            "atari-800",
            "atari",
            "TOSEC-ISO"
        ));
        assert!(explicit_fallback_matches(
            "arcade",
            "roms-non-merged",
            "MAME"
        ));
        assert!(!explicit_fallback_matches(
            "arcade",
            "roms-non-merged",
            "HBMAME"
        ));
        assert!(!explicit_fallback_matches("atari-2600", "atari", "TOSEC"));
    }

    #[test]
    fn byte_labels_are_binary_and_readable() {
        assert_eq!(format_bytes(1024), "1.0 KiB");
        assert_eq!(format_bytes(1024 * 1024 * 3), "3.0 MiB");
    }

    #[test]
    fn torrent_metadata_cache_is_validated_and_reusable() {
        let directory = tempfile::tempdir().unwrap();
        let url = "https://example.test/a collection.torrent";
        let path = torrent_cache_path_in(directory.path(), url);
        assert_eq!(
            path.extension().and_then(|value| value.to_str()),
            Some("torrent")
        );
        assert!(!path.to_string_lossy().contains("a collection"));

        let bytes = Torrent {
            announce: Some("https://tracker.example/announce".into()),
            announce_list: None,
            length: 4,
            files: None,
            name: "test.bin".into(),
            piece_length: 16_384,
            pieces: vec![vec![0xff; 20]],
            extra_fields: None,
            extra_info_fields: None,
        }
        .encode()
        .unwrap();
        Torrent::read_from_bytes(&bytes).unwrap();
        let files = torrent_plan_files_from_bytes(&bytes).unwrap();
        assert_eq!(files.len(), 1);
        assert_eq!(files[0].filename, "test.bin");
        write_torrent_cache_file(&path, &bytes).unwrap();
        assert_eq!(read_torrent_cache_file(&path).unwrap(), bytes);

        fs::write(&path, b"not a torrent").unwrap();
        assert!(read_torrent_cache_file(&path).is_none());
    }

    #[test]
    fn local_only_details_expose_the_real_file_without_inventing_identity() {
        let directory = tempfile::tempdir().unwrap();
        let rom = directory.path().join("Mystery Game.nes");
        std::fs::write(&rom, b"mystery").unwrap();
        let encoded = crate::local_import::encode_path(&rom);
        let state = directory.path().join("state.db");
        let connection = rusqlite::Connection::open(&state).unwrap();
        connection
            .execute_batch(
                "CREATE TABLE local_rom_files (
                     id TEXT, path_display TEXT, path_bytes BLOB, path_encoding TEXT,
                     file_size INTEGER, match_state TEXT, match_method TEXT,
                     sha1 TEXT, md5 TEXT, included INTEGER, availability TEXT
                 );",
            )
            .unwrap();
        connection
            .execute(
                "INSERT INTO local_rom_files VALUES (
                     'file-1', ?1, ?2, ?3, 7, 'unmatched', 'strong checksum',
                     'ABC', 'DEF', 1, 'present'
                 )",
                rusqlite::params![encoded.display, encoded.bytes, encoded.encoding],
            )
            .unwrap();
        drop(connection);

        let mut details = GameDetails {
            id: "local-file:file-1".into(),
            title: "Mystery Game".into(),
            platform: "Unassigned".into(),
            local: true,
            ..GameDetails::default()
        };
        load_local_only_details(&mut details, "file-1", &state).unwrap();
        assert!(details.local);
        assert!(!details.downloadable);
        assert_eq!(details.release_type, "Local-only file");
        assert!(details.notes.contains("Mystery Game.nes"));
        assert!(details.notes.contains("SHA-1 ABC · MD5 DEF"));
        assert!(details.description.contains("no proven catalog identity"));
        assert_eq!(details.local_file_path, rom);
        assert_eq!(details.local_file_paths, vec![rom]);
    }

    #[test]
    fn minerva_resolution_prefers_identity_and_bounds_legacy_rules() {
        let database = tempfile::NamedTempFile::new().unwrap();
        let connection = rusqlite::Connection::open(database.path()).unwrap();
        connection
            .execute_batch(
                "CREATE TABLE minerva_torrents (
                    id INTEGER PRIMARY KEY,
                    torrent_file TEXT NOT NULL UNIQUE,
                    torrent_url TEXT NOT NULL,
                    collection TEXT,
                    rom_count INTEGER DEFAULT 0,
                    total_size INTEGER DEFAULT 0
                );
                CREATE TABLE minerva_torrent_platforms (
                    torrent_id INTEGER NOT NULL REFERENCES minerva_torrents(id),
                    minerva_platform TEXT NOT NULL,
                    lunchbox_platform_id INTEGER,
                    lunchbox_platform_name TEXT,
                    rom_count INTEGER DEFAULT 0,
                    PRIMARY KEY (torrent_id, minerva_platform)
                );
                INSERT INTO minerva_torrents VALUES
                    (1, 'exact.torrent', 'https://example.test/exact.torrent', 'No-Intro', 1, 10),
                    (2, 'tosec.torrent', 'https://example.test/tosec.torrent', 'TOSEC', 1, 20),
                    (3, 'iso.torrent', 'https://example.test/iso.torrent', 'TOSEC-ISO', 1, 30);
                INSERT INTO minerva_torrent_platforms VALUES
                    (1, 'Nintendo Game Boy', 42, 'Nintendo Game Boy', 100),
                    (2, 'Atari', NULL, NULL, 200),
                    (3, 'Atari', NULL, NULL, 300);",
            )
            .unwrap();
        drop(connection);

        let exact = resolve_minerva_bundles_from_path(
            &GameDetails {
                platform: "Nintendo Game Boy".into(),
                ..GameDetails::default()
            },
            database.path(),
        )
        .unwrap();
        assert_eq!(exact.len(), 1);
        assert_eq!(exact[0].torrent_id, 1);
        assert_eq!(exact[0].match_kind, BundleMatchKind::MappedName);

        let atari = resolve_minerva_bundles_from_path(
            &GameDetails {
                platform: "Atari 800".into(),
                ..GameDetails::default()
            },
            database.path(),
        )
        .unwrap();
        assert_eq!(atari.len(), 1);
        assert_eq!(atari[0].torrent_id, 2);
        assert_eq!(atari[0].match_kind, BundleMatchKind::ExplicitFallback);
    }

    #[test]
    fn minerva_resolution_prefers_primary_preservation_sets() {
        let bundle = |id, collection: &str, platform: &str| MinervaBundle {
            torrent_id: id,
            torrent_url: format!("https://example.test/{id}.torrent"),
            source_kind: "minerva".to_owned(),
            torrent_sha256: String::new(),
            collection: collection.to_owned(),
            provider_platform: platform.to_owned(),
            rom_count: 1,
            total_size: 10,
            match_kind: BundleMatchKind::MappedName,
        };

        assert!(
            bundle_source_priority(&bundle(
                1,
                "No-Intro",
                "Nintendo - Nintendo Entertainment System (Headered)",
            )) < bundle_source_priority(&bundle(2, "FinalBurn Neo", "nes"))
        );
        assert!(
            bundle_source_priority(&bundle(
                1,
                "No-Intro",
                "Nintendo - Nintendo Entertainment System (Headered)",
            )) < bundle_source_priority(&bundle(
                3,
                "No-Intro",
                "Nintendo - Nintendo Entertainment System (Headered) (Private)",
            ))
        );
    }

    #[test]
    fn registered_torrent_catalog_resolves_by_exact_platform_and_title_key() {
        assert_eq!(
            torrent_member_title_key(
                "MIG Switch/Captain Toad Treasure Tracker [01009BF0072D4000][v0].xci"
            ),
            "captain toad treasure tracker"
        );
        let directory = tempfile::tempdir().unwrap();
        let store = crate::settings::SettingsStore::at(directory.path().join("state.db")).unwrap();
        let offer = crate::external_torrent::inspect_torrent_bytes(
            &crate::external_torrent::probe_fixture_torrent_bytes(),
            "switch-volume.torrent",
        )
        .unwrap();
        store
            .register_torrent_catalog(&crate::settings::RegisteredTorrentCatalog {
                platform: "Nintendo Switch".into(),
                platform_key: "nintendo-switch".into(),
                source_kind: "file".into(),
                source_label: "switch-volume.torrent".into(),
                source_file_name: "switch-volume.torrent".into(),
                torrent_name: offer.torrent_name,
                torrent_sha256: offer.torrent_sha256,
                info_hash: offer.info_hash,
                torrent_bytes: offer.torrent_bytes,
                total_bytes: offer.total_bytes,
                members: offer
                    .files
                    .into_iter()
                    .map(|file| crate::settings::RegisteredTorrentMember {
                        index: file.index,
                        title_key: torrent_member_title_key(&file.path),
                        path: file.path,
                        byte_size: file.byte_size,
                    })
                    .collect(),
                registered_at: 1,
            })
            .unwrap();

        assert!(
            resolve_registered_torrent_bundles_for_game(
                &store,
                "Nintendo GameCube",
                "Sample Game",
                &[],
            )
            .unwrap()
            .is_empty()
        );
        assert!(
            resolve_registered_torrent_bundles_for_game(
                &store,
                "Nintendo Switch",
                "Different Game",
                &[],
            )
            .unwrap()
            .is_empty()
        );
        let bundles = resolve_registered_torrent_bundles_for_game(
            &store,
            "Nintendo Switch",
            "Sample Game",
            &[],
        )
        .unwrap();
        assert_eq!(bundles.len(), 1);
        assert_eq!(bundles[0].source_kind, "manual_torrent");
        let files = registered_torrent_plan_files(&store, &bundles[0], "Sample Game", &[]).unwrap();
        assert_eq!(files.len(), 2);
        assert_eq!(files[0].filename, "Game/Sample Game.rom");
        assert_eq!(files[1].filename, "Game/Sample Game (Europe).rom");
    }

    #[test]
    fn arcade_laserdisc_source_views_share_one_exact_torrent() {
        let database = tempfile::NamedTempFile::new().unwrap();
        let connection = rusqlite::Connection::open(database.path()).unwrap();
        connection
            .execute_batch(
                "CREATE TABLE minerva_torrents (
                    id INTEGER PRIMARY KEY,
                    torrent_file TEXT NOT NULL UNIQUE,
                    torrent_url TEXT NOT NULL,
                    collection TEXT,
                    rom_count INTEGER DEFAULT 0,
                    total_size INTEGER DEFAULT 0
                );
                CREATE TABLE minerva_torrent_platforms (
                    torrent_id INTEGER NOT NULL REFERENCES minerva_torrents(id),
                    minerva_platform TEXT NOT NULL,
                    lunchbox_platform_id INTEGER,
                    lunchbox_platform_name TEXT,
                    rom_count INTEGER DEFAULT 0,
                    PRIMARY KEY (torrent_id, minerva_platform)
                );
                INSERT INTO minerva_torrents VALUES
                    (920, 'laserdisc.torrent', 'https://example.test/laserdisc.torrent',
                     'Laserdisc Collection', 35275, 4815337787382);
                INSERT INTO minerva_torrent_platforms VALUES
                    (920, 'MAME', 15, 'Arcade', 52),
                    (920, 'Hypseus Singe', NULL, NULL, 8681),
                    (920, 'Daphne', NULL, NULL, 18300);",
            )
            .unwrap();
        drop(connection);

        let bundles = resolve_minerva_bundles_from_path(
            &GameDetails {
                platform: "Arcade Laserdisc".into(),
                ..GameDetails::default()
            },
            database.path(),
        )
        .unwrap();
        assert_eq!(bundles.len(), 3);
        assert!(bundles.iter().all(|bundle| bundle.torrent_id == 920));
        assert_eq!(
            bundles
                .iter()
                .map(|bundle| bundle.provider_platform.as_str())
                .collect::<BTreeSet<_>>(),
            BTreeSet::from(["Daphne", "Hypseus Singe", "MAME"])
        );
    }

    #[test]
    fn canonical_mame_records_resolve_romsets_without_launchbox_metadata() {
        let connection = rusqlite::Connection::open_in_memory().unwrap();
        connection
            .execute_batch(
                "CREATE TABLE libretro_databases (id INTEGER PRIMARY KEY, source_name TEXT);
                 CREATE TABLE libretro_records (
                    database_id INTEGER, title TEXT, file_name TEXT
                 );
                 INSERT INTO libretro_databases VALUES (1, 'MAME'), (2, 'MAME 2003');
                 INSERT INTO libretro_records VALUES
                    (1, 'Dragon''s Lair', 'dlair.zip'),
                    (1, 'Dragon''s Lair (US Rev. F)', 'dlairf.zip'),
                    (1, 'Dragon''s Lair 2: Time Warp (US v3.19)', 'dlair2.zip'),
                    (2, 'Dragon''s Lair', 'old-dlair.zip'),
                    (1, 'Space Ace', 'spaceace.zip');",
            )
            .unwrap();
        assert_eq!(
            mame_romset_names_from_connection(&connection, "Dragon's Lair").unwrap(),
            vec!["dlair", "dlairf"]
        );
        assert_eq!(
            mame_romset_names_from_connection(&connection, "Dragon's Lair II: Time Warp").unwrap(),
            vec!["dlair2"]
        );
    }

    #[test]
    #[ignore = "requires local discovery/Minerva databases and network access"]
    fn live_minerva_metadata_matches_exact_linked_alternate_title() {
        let evidence = alternate_title_probe().unwrap();
        assert!(evidence.contains("matched_title="));
    }

    #[test]
    #[ignore = "requires local discovery/Minerva databases and network access"]
    fn live_minerva_metadata_contains_ranked_game_candidate() {
        let discovery_path = catalog::requested_discovery_database_path()
            .expect("local lunchbox-games.db is required for this probe");
        let connection = catalog::open_read_only(&discovery_path, "discovery probe").unwrap();
        let (id, platform): (String, String) = connection
            .query_row(
                "SELECT g.id, p.name FROM games g
                 JOIN platforms p ON p.id=g.platform_id
                 WHERE g.title='Super Mario Land' LIMIT 1",
                [],
                |row| Ok((row.get(0)?, row.get(1)?)),
            )
            .unwrap();
        let details = load(&id, "Super Mario Land", &platform, false, true).unwrap();
        assert!(!details.bundles.is_empty());
        let files = load_torrent_files(
            &details.bundles[0],
            &details.title,
            &details.alternate_titles,
            &preferences("USA", "latest"),
        )
        .unwrap();
        assert!(
            files
                .iter()
                .any(|file| file.filename.to_lowercase().contains("super mario land"))
        );
    }

    #[test]
    #[ignore = "requires local discovery/Minerva databases and network access"]
    fn live_minerva_metadata_builds_final_fantasy_multidisc_plan() {
        let details = load(
            "293ced29-4167-474a-8cef-91cae446cfb8",
            "Final Fantasy VII",
            "Sony Playstation",
            false,
            true,
        )
        .unwrap();
        let plan = details
            .bundles
            .iter()
            .filter_map(|bundle| {
                load_torrent_files(
                    bundle,
                    &details.title,
                    &details.alternate_titles,
                    &preferences("USA", "latest"),
                )
                .ok()
            })
            .flat_map(|files| files.into_iter())
            .find_map(|file| file.download_plan)
            .expect("live Final Fantasy VII multi-disc plan");
        assert_eq!(plan.disc_count(), 3);
        assert_eq!(plan.members.len(), 3);
        assert_eq!(plan.playlist_filename, "Final Fantasy VII (USA).m3u");
    }

    #[test]
    #[ignore = "requires local discovery/Minerva databases and network access"]
    fn live_minerva_metadata_builds_prince_of_persia_exodos_plan() {
        let details = load(
            "7b3df553-7426-475a-9818-c4589e1e9480",
            "Prince of Persia",
            "MS-DOS",
            false,
            true,
        )
        .unwrap();
        let plan = details
            .bundles
            .iter()
            .filter(|bundle| bundle.collection.eq_ignore_ascii_case("eXo"))
            .filter_map(|bundle| {
                load_torrent_files(
                    bundle,
                    &details.title,
                    &details.alternate_titles,
                    &preferences("USA", "latest"),
                )
                .ok()
            })
            .flat_map(|files| files.into_iter())
            .find_map(|file| file.download_plan)
            .expect("live Prince of Persia eXoDOS plan");
        assert!(plan.is_exo_archive_set());
        let primary_path = &plan.representative_member().unwrap().torrent_path;
        assert!(primary_path.contains("eXo/eXoDOS/Prince of Persia (1990).zip"));
        assert!(!primary_path.to_ascii_lowercase().contains("/!english/"));
        assert!(!primary_path.to_ascii_lowercase().contains("/!german/"));
        assert!(!primary_path.to_ascii_lowercase().contains("/!polish/"));
        assert!(!primary_path.to_ascii_lowercase().contains("/!spanish/"));
        assert!(plan.members.iter().any(|member| member.role == "metadata"));
    }

    #[test]
    #[ignore = "requires local discovery/Minerva/canonical databases and network access"]
    fn live_minerva_metadata_builds_dragons_lair_machine_layouts() {
        let details = load(
            "ffecdc12-bc0f-459c-a0b7-4ee856e14f9e",
            "Dragon's Lair",
            "Arcade",
            false,
            true,
        )
        .unwrap();
        let laserdisc_bundles = details
            .bundles
            .iter()
            .filter(|bundle| {
                bundle
                    .collection
                    .eq_ignore_ascii_case("Laserdisc Collection")
            })
            .collect::<Vec<_>>();
        assert_eq!(laserdisc_bundles.len(), 3);

        let mut has_mame = false;
        let mut has_hypseus = false;
        let mut has_daphne = false;
        for bundle in laserdisc_bundles {
            let files = load_torrent_files(
                bundle,
                &details.title,
                &details.alternate_titles,
                &preferences("USA", "latest"),
            )
            .unwrap();
            for plan in files
                .into_iter()
                .filter_map(|candidate| candidate.download_plan)
            {
                has_mame |= plan.is_arcade_mame_layout();
                has_hypseus |= plan.is_arcade_hypseus_layout();
                has_daphne |= plan.is_arcade_daphne_layout();
            }
        }
        assert!(has_mame);
        assert!(has_hypseus);
        assert!(has_daphne);
    }
}
