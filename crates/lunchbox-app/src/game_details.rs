use std::collections::HashSet;
use std::io::Read;
use std::path::Path;
use std::sync::{Arc, Mutex, OnceLock};
use std::time::Duration;

use anyhow::{Context, Result, bail};
use lava_torrent::torrent::v1::Torrent;
use rusqlite::OptionalExtension;

use crate::catalog;
use crate::download_plan::{DownloadPlan, TorrentPlanFile, build_optical_plan};

const MAX_TORRENT_BYTES: u64 = 64 * 1024 * 1024;
const MAX_FILE_CANDIDATES: usize = 100;
type TorrentCache = Mutex<Option<(String, Arc<Vec<u8>>)>>;

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
    pub esrb: String,
    pub release_type: String,
    pub notes: String,
    pub local: bool,
    pub downloadable: bool,
    pub database_id: i64,
    pub bundles: Vec<MinervaBundle>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct MinervaBundle {
    pub torrent_id: i64,
    pub torrent_url: String,
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
    pub region: String,
    pub version: String,
    pub download_plan: Option<DownloadPlan>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ReleasePreferences {
    pub preferred_region: String,
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

    if let Some(local_file_id) = id.strip_prefix("local-file:") {
        let state_path = crate::settings::state_database_path()?;
        load_local_only_details(&mut details, local_file_id, &state_path)?;
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
                        coalesce(g.release_type, ''), coalesce(g.notes, ''),
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
                        row.get::<_, i64>(10)?,
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
            details.notes = row.9;
            details.database_id = row.10;
        }
    }

    details.bundles = resolve_minerva_bundles(&details)?;
    details.downloadable = !details.local && !details.bundles.is_empty();
    Ok(details)
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
            collection,
            provider_platform,
            rom_count,
            total_size: u64::try_from(total_size).unwrap_or_default(),
            match_kind,
        });
    }
    bundles.sort_by(|left, right| {
        left.match_kind
            .cmp(&right.match_kind)
            .then_with(|| right.rom_count.cmp(&left.rom_count))
            .then_with(|| left.collection.cmp(&right.collection))
            .then_with(|| left.provider_platform.cmp(&right.provider_platform))
    });
    Ok(bundles)
}

fn explicit_fallback_matches(platform_key: &str, provider_key: &str, collection: &str) -> bool {
    match platform_key {
        "atari-800" => provider_key == "atari" && collection == "TOSEC",
        "arcade" => {
            collection == "MAME"
                && matches!(
                    provider_key,
                    "roms-merged" | "roms-split" | "roms-non-merged"
                )
        }
        _ => false,
    }
}

pub fn load_torrent_files(
    bundle: &MinervaBundle,
    game_title: &str,
    preferences: &ReleasePreferences,
) -> Result<Vec<TorrentFileCandidate>> {
    let bytes = fetch_torrent(&bundle.torrent_url)?;
    let torrent = Torrent::read_from_bytes(bytes.as_slice())
        .map_err(|error| anyhow::anyhow!("could not parse Minerva torrent metadata: {error}"))?;
    let files = if let Some(torrent_files) = torrent.files {
        torrent_files
            .iter()
            .enumerate()
            .map(|(index, file)| {
                Ok(TorrentFileCandidate {
                    index,
                    filename: file
                        .path
                        .components()
                        .map(|component| component.as_os_str().to_string_lossy())
                        .collect::<Vec<_>>()
                        .join("/"),
                    byte_size: u64::try_from(file.length)
                        .context("torrent file has a negative byte length")?,
                    match_score: 0.0,
                    region: String::new(),
                    version: String::new(),
                    download_plan: None,
                })
            })
            .collect::<Result<Vec<_>>>()?
    } else {
        vec![TorrentFileCandidate {
            index: 0,
            filename: torrent.name,
            byte_size: u64::try_from(torrent.length)
                .context("torrent payload has a negative byte length")?,
            match_score: 0.0,
            region: String::new(),
            version: String::new(),
            download_plan: None,
        }]
    };

    rank_file_candidates(files, game_title, preferences)
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

pub(crate) fn torrent_bytes(bundle: &MinervaBundle) -> Result<Vec<u8>> {
    Ok(fetch_torrent(&bundle.torrent_url)?.as_ref().clone())
}

fn rank_file_candidates(
    files: Vec<TorrentFileCandidate>,
    game_title: &str,
    preferences: &ReleasePreferences,
) -> Result<Vec<TorrentFileCandidate>> {
    let query = normalized_words(title_without_tags(game_title));
    if query.is_empty() {
        bail!("game title has no searchable characters");
    }

    let query_tokens = significant_tokens(&query);
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
            file.match_score = file_match_score(&file.filename, &query, &query_tokens);
            (file.region, file.version) = release_labels(&file.filename);
            (file.match_score >= 0.35).then_some(file)
        })
        .collect::<Vec<_>>();
    candidates.sort_by(|left, right| {
        right
            .match_score
            .total_cmp(&left.match_score)
            .then_with(|| release_preference_order(left, right, preferences))
            .then_with(|| left.byte_size.cmp(&right.byte_size))
            .then_with(|| left.filename.cmp(&right.filename))
    });
    let mut seen_plans = HashSet::new();
    candidates.retain_mut(|candidate| {
        let Some(plan) = build_optical_plan(&plan_files, candidate.index) else {
            return true;
        };
        let plan_key = plan.display_name.to_ascii_lowercase();
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

fn title_without_tags(value: &str) -> &str {
    value
        .find(['(', '['])
        .map(|index| value[..index].trim_end())
        .unwrap_or(value)
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
    region_priority(&left.region, &preferences.preferred_region)
        .cmp(&region_priority(
            &right.region,
            &preferences.preferred_region,
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

fn region_priority(region: &str, preferred_region: &str) -> usize {
    const DEFAULT_ORDER: &[&str] = &[
        "USA",
        "Japan",
        "Asia",
        "World",
        "Europe",
        "Australia",
        "Canada",
        "Brazil",
        "Korea",
        "China",
        "France",
        "Germany",
        "Italy",
        "Spain",
        "United Kingdom",
        "Taiwan",
        "Netherlands",
        "Belgium",
        "Greece",
        "Portugal",
        "Austria",
        "Sweden",
        "Finland",
        "Russia",
        "Switzerland",
        "Hong Kong",
        "Scandinavia",
        "Denmark",
        "Poland",
        "Norway",
        "New Zealand",
        "Latin America",
        "Unknown",
    ];
    if preferred_region == "any" {
        return 0;
    }
    let regions = region.split(", ").collect::<Vec<_>>();
    if regions
        .iter()
        .any(|region| region.eq_ignore_ascii_case(preferred_region))
    {
        return 0;
    }
    DEFAULT_ORDER
        .iter()
        .filter(|candidate| !candidate.eq_ignore_ascii_case(preferred_region))
        .position(|candidate| {
            regions
                .iter()
                .any(|region| region.eq_ignore_ascii_case(candidate))
        })
        .map(|position| position + 1)
        .unwrap_or(usize::MAX)
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
            preferred_region: region.into(),
            version_preference: version.into(),
        }
    }

    fn candidate(index: usize, filename: &str) -> TorrentFileCandidate {
        TorrentFileCandidate {
            index,
            filename: filename.into(),
            byte_size: 10,
            match_score: 0.0,
            region: String::new(),
            version: String::new(),
            download_plan: None,
        }
    }

    #[test]
    fn file_ranking_prefers_exact_title_without_accepting_identity() {
        let files = vec![
            candidate(0, "Nintendo/Game Boy/Super Mario Land (USA, Europe).zip"),
            candidate(1, "Nintendo/Game Boy/Mario Tennis (USA).zip"),
        ];
        let ranked =
            rank_file_candidates(files, "Super Mario Land", &preferences("USA", "latest")).unwrap();
        assert_eq!(ranked.len(), 1);
        assert_eq!(ranked[0].index, 0);
        assert_eq!(ranked[0].match_score, 1.0);
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
    fn preferred_region_and_version_rank_equal_title_matches() {
        let files = vec![
            candidate(0, "Game (USA) (Rev 1).zip"),
            candidate(1, "Game (Japan) (Rev 1).zip"),
            candidate(2, "Game (Japan) (Rev 3).zip"),
        ];
        let ranked =
            rank_file_candidates(files, "Game (USA)", &preferences("Japan", "latest")).unwrap();
        assert_eq!(
            ranked.iter().map(|file| file.index).collect::<Vec<_>>(),
            [2, 1, 0]
        );
        assert_eq!(ranked[0].region, "Japan");
        assert_eq!(ranked[0].version, "Rev 3");
    }

    #[test]
    fn original_version_policy_prefers_untagged_release() {
        let files = vec![
            candidate(0, "Game (Europe) (Rev 2).zip"),
            candidate(1, "Game (Europe).zip"),
        ];
        let ranked =
            rank_file_candidates(files, "Game", &preferences("Europe", "original")).unwrap();
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

        let ranked =
            rank_file_candidates(files, "Final Fantasy VII", &preferences("USA", "latest"))
                .unwrap();
        assert_eq!(ranked.len(), 1);
        let plan = ranked[0].download_plan.as_ref().expect("optical plan");
        assert_eq!(ranked[0].index, 0);
        assert_eq!(ranked[0].byte_size, 3_300);
        assert_eq!(plan.disc_count(), 2);
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
            "6119074e-d813-446d-a7ff-556f75e52320",
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
                load_torrent_files(bundle, &details.title, &preferences("USA", "latest")).ok()
            })
            .flat_map(|files| files.into_iter())
            .find_map(|file| file.download_plan)
            .expect("live Final Fantasy VII multi-disc plan");
        assert_eq!(plan.disc_count(), 3);
        assert_eq!(plan.members.len(), 3);
        assert_eq!(plan.playlist_filename, "Final Fantasy VII (USA).m3u");
    }
}
