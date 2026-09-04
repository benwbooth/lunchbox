use std::collections::{HashMap, HashSet};
use std::fs;
use std::io::{Read, Write};
use std::path::{Path, PathBuf};

use anyhow::{Context, Result, bail};
use serde::{Deserialize, Serialize};

use crate::catalog::Game;

const PORTABLE_FORMAT: &str = "lunchbox-collection";
const PORTABLE_VERSION: u32 = 1;
const MAX_PORTABLE_BYTES: u64 = 128 * 1024 * 1024;
const MAX_PORTABLE_GAMES: usize = 500_000;

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct SmartCollectionRules {
    pub title_contains: String,
    pub platform: String,
    pub tag: String,
    pub availability: String,
    pub favorite: String,
    pub completion_state: String,
    pub content: String,
    pub cooperative: String,
}

impl Default for SmartCollectionRules {
    fn default() -> Self {
        Self {
            title_contains: String::new(),
            platform: String::new(),
            tag: String::new(),
            availability: "any".to_owned(),
            favorite: "any".to_owned(),
            completion_state: "any".to_owned(),
            content: "any".to_owned(),
            cooperative: "any".to_owned(),
        }
    }
}

impl SmartCollectionRules {
    pub fn normalized(mut self) -> Result<Self> {
        self.title_contains = self.title_contains.trim().to_owned();
        self.platform = self.platform.trim().to_owned();
        self.tag = self.tag.trim().to_owned();
        self.availability = self.availability.trim().to_owned();
        self.favorite = self.favorite.trim().to_owned();
        self.completion_state = self.completion_state.trim().to_owned();
        self.content = self.content.trim().to_owned();
        self.cooperative = self.cooperative.trim().to_owned();
        self.validate()?;
        Ok(self)
    }

    pub fn validate(&self) -> Result<()> {
        if self.title_contains.chars().count() > 200 || self.platform.chars().count() > 200 {
            bail!("smart collection text rules must be 200 characters or fewer");
        }
        if self.tag.chars().count() > 50 {
            bail!("smart collection tags must be 50 characters or fewer");
        }
        if !matches!(
            self.availability.as_str(),
            "any" | "installed" | "downloadable" | "catalog"
        ) {
            bail!("unsupported smart collection availability rule");
        }
        if !matches!(self.favorite.as_str(), "any" | "favorite" | "not_favorite") {
            bail!("unsupported smart collection favorite rule");
        }
        if !matches!(
            self.completion_state.as_str(),
            "any" | "not_started" | "in_progress" | "completed" | "on_hold" | "abandoned"
        ) {
            bail!("unsupported smart collection completion rule");
        }
        if !matches!(
            self.content.as_str(),
            "any" | "retail" | "non_retail" | "adult"
        ) {
            bail!("unsupported smart collection content rule");
        }
        if !matches!(self.cooperative.as_str(), "any" | "yes" | "no" | "unknown") {
            bail!("unsupported smart collection cooperative-play rule");
        }
        if self == &Self::default() {
            bail!("a smart collection needs at least one rule");
        }
        Ok(())
    }

    #[cfg(test)]
    pub fn matches(
        &self,
        game: &Game,
        favorites: &HashSet<String>,
        completion_states: &HashMap<String, String>,
    ) -> bool {
        let title_needle = self.title_contains.to_lowercase();
        self.matches_with_title_needle(game, favorites, completion_states, &title_needle)
    }

    #[cfg(test)]
    pub(crate) fn matches_with_title_needle(
        &self,
        game: &Game,
        favorites: &HashSet<String>,
        completion_states: &HashMap<String, String>,
        title_needle: &str,
    ) -> bool {
        self.matches_with_display_title(
            game,
            favorites,
            completion_states,
            title_needle,
            &game.title,
            &[],
            &game.cooperative,
        )
    }

    pub(crate) fn matches_with_display_title(
        &self,
        game: &Game,
        favorites: &HashSet<String>,
        completion_states: &HashMap<String, String>,
        title_needle: &str,
        display_title: &str,
        tags: &[String],
        cooperative: &str,
    ) -> bool {
        let canonical_title = game
            .search_key
            .split_once('\n')
            .map(|(title, _)| title)
            .unwrap_or(&game.search_key);
        let title_matches = title_needle.is_empty()
            || canonical_title.contains(title_needle)
            || display_title.to_lowercase().contains(title_needle);
        let platform_matches = self.platform.is_empty() || game.platform == self.platform;
        let normalized_rule_tag = self.tag.to_lowercase();
        let tag_matches = self.tag.is_empty()
            || tags
                .iter()
                .any(|tag| tag.to_lowercase() == normalized_rule_tag);
        let availability_matches = match self.availability.as_str() {
            "installed" => game.local,
            "downloadable" => game.downloadable && !game.local,
            "catalog" => !game.local && !game.downloadable,
            _ => true,
        };
        let favorite_matches = match self.favorite.as_str() {
            "favorite" => favorites.contains(&game.id),
            "not_favorite" => !favorites.contains(&game.id),
            _ => true,
        };
        let completion_matches = self.completion_state == "any"
            || completion_states
                .get(&game.id)
                .map(String::as_str)
                .unwrap_or("not_started")
                == self.completion_state;
        let content_matches = match self.content.as_str() {
            "retail" => !game.non_retail && !game.adult,
            "non_retail" => game.non_retail,
            "adult" => game.adult,
            _ => true,
        };
        let cooperative_matches = self.cooperative == "any" || self.cooperative == cooperative;
        title_matches
            && platform_matches
            && tag_matches
            && availability_matches
            && favorite_matches
            && completion_matches
            && content_matches
            && cooperative_matches
    }

    pub fn summary(&self) -> String {
        let mut rules = Vec::new();
        if !self.title_contains.is_empty() {
            rules.push(format!("title contains ‘{}’", self.title_contains));
        }
        if !self.platform.is_empty() {
            rules.push(self.platform.clone());
        }
        if !self.tag.is_empty() {
            rules.push(format!("tagged {}", self.tag));
        }
        match self.availability.as_str() {
            "installed" => rules.push("installed".to_owned()),
            "downloadable" => rules.push("available from Minerva".to_owned()),
            "catalog" => rules.push("catalog only".to_owned()),
            _ => {}
        }
        match self.favorite.as_str() {
            "favorite" => rules.push("favorites".to_owned()),
            "not_favorite" => rules.push("not favorites".to_owned()),
            _ => {}
        }
        if self.completion_state != "any" {
            rules.push(self.completion_state.replace('_', " "));
        }
        match self.content.as_str() {
            "retail" => rules.push("retail".to_owned()),
            "non_retail" => rules.push("non-retail".to_owned()),
            "adult" => rules.push("adult".to_owned()),
            _ => {}
        }
        match self.cooperative.as_str() {
            "yes" => rules.push("co-op supported".to_owned()),
            "no" => rules.push("no co-op".to_owned()),
            "unknown" => rules.push("co-op unspecified".to_owned()),
            _ => {}
        }
        rules.join(" · ")
    }
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct PortableGameReference {
    pub game_uid: String,
    pub launchbox_db_id: i64,
    pub title: String,
    pub platform: String,
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub playlist_title: String,
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub playlist_notes: String,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ResolvedPortableGameReference {
    pub game_uid: String,
    pub playlist_title: String,
    pub playlist_notes: String,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct PortableCollection {
    pub format: String,
    pub version: u32,
    pub name: String,
    pub description: String,
    pub kind: String,
    pub rules: Option<SmartCollectionRules>,
    pub games: Vec<PortableGameReference>,
}

impl PortableCollection {
    pub fn new(
        name: String,
        description: String,
        kind: String,
        rules: Option<SmartCollectionRules>,
        games: Vec<PortableGameReference>,
    ) -> Result<Self> {
        let portable = Self {
            format: PORTABLE_FORMAT.to_owned(),
            version: PORTABLE_VERSION,
            name,
            description,
            kind,
            rules,
            games,
        };
        portable.validate()?;
        Ok(portable)
    }

    pub fn validate(&self) -> Result<()> {
        if self.format != PORTABLE_FORMAT || self.version != PORTABLE_VERSION {
            bail!("unsupported Lunchbox collection file format or version");
        }
        validate_portable_text("name", &self.name, 100, false)?;
        validate_portable_text("description", &self.description, 1_000, true)?;
        if !matches!(self.kind.as_str(), "manual" | "smart") {
            bail!("portable collection kind must be manual or smart");
        }
        match (&self.kind[..], &self.rules) {
            ("manual", None) => {}
            ("smart", Some(rules)) => rules.validate()?,
            ("manual", Some(_)) => bail!("manual collection files cannot carry smart rules"),
            ("smart", None) => bail!("smart collection files require rules"),
            _ => unreachable!(),
        }
        if self.games.len() > MAX_PORTABLE_GAMES {
            bail!("portable collection contains too many games");
        }
        let mut identities = HashSet::new();
        for game in &self.games {
            validate_portable_text("game title", &game.title, 500, false)?;
            validate_portable_text("game platform", &game.platform, 300, true)?;
            validate_portable_text("playlist title", &game.playlist_title, 500, true)?;
            validate_portable_text("playlist notes", &game.playlist_notes, 4_000, true)?;
            if game.game_uid.trim().is_empty() && game.launchbox_db_id <= 0 {
                bail!("portable games require a stable game or LaunchBox database ID");
            }
            if game.game_uid.trim() != game.game_uid || game.game_uid.chars().count() > 512 {
                bail!("portable game identities must be trimmed and at most 512 characters");
            }
            let identity = if game.game_uid.trim().is_empty() {
                format!("lb:{}", game.launchbox_db_id)
            } else {
                format!("uid:{}", game.game_uid)
            };
            if !identities.insert(identity) {
                bail!("portable collection contains a duplicate game identity");
            }
        }
        Ok(())
    }
}

pub fn save_portable_collection(path: &Path, portable: &PortableCollection) -> Result<PathBuf> {
    portable.validate()?;
    if path.as_os_str().is_empty() {
        bail!("choose a collection export file");
    }
    let parent = path.parent().unwrap_or_else(|| Path::new("."));
    fs::create_dir_all(parent)
        .with_context(|| format!("creating collection export directory {}", parent.display()))?;
    let file_name = path
        .file_name()
        .and_then(|name| name.to_str())
        .context("collection export filename is not valid UTF-8")?;
    let temporary = parent.join(format!(".{file_name}.{}.tmp", std::process::id()));
    let bytes = serde_json::to_vec_pretty(portable).context("encoding collection export")?;
    if bytes.len() as u64 > MAX_PORTABLE_BYTES {
        bail!("collection export exceeds the 128 MiB safety limit");
    }
    let mut file = fs::File::create(&temporary).with_context(|| {
        format!(
            "creating temporary collection export {}",
            temporary.display()
        )
    })?;
    file.write_all(&bytes)
        .with_context(|| format!("writing collection export {}", temporary.display()))?;
    file.write_all(b"\n")?;
    file.sync_all()?;
    drop(file);
    replace_atomically(&temporary, path)?;
    Ok(path.to_path_buf())
}

pub fn load_portable_collection(path: &Path) -> Result<PortableCollection> {
    let metadata = fs::metadata(path)
        .with_context(|| format!("reading collection file metadata {}", path.display()))?;
    if !metadata.is_file() || metadata.len() > MAX_PORTABLE_BYTES {
        bail!("collection file must be a regular file no larger than 128 MiB");
    }
    let mut bytes = Vec::with_capacity(usize::try_from(metadata.len()).unwrap_or_default());
    fs::File::open(path)
        .with_context(|| format!("opening collection file {}", path.display()))?
        .take(MAX_PORTABLE_BYTES + 1)
        .read_to_end(&mut bytes)?;
    if bytes.len() as u64 > MAX_PORTABLE_BYTES {
        bail!("collection file exceeds the 128 MiB safety limit");
    }
    let portable: PortableCollection =
        serde_json::from_slice(&bytes).context("decoding Lunchbox collection file")?;
    portable.validate()?;
    Ok(portable)
}

pub fn resolve_portable_game_presentations(
    references: &[PortableGameReference],
    catalog_games: &[Game],
) -> (Vec<ResolvedPortableGameReference>, usize) {
    let by_uid = catalog_games
        .iter()
        .map(|game| (game.id.as_str(), game.id.clone()))
        .collect::<HashMap<_, _>>();
    let mut by_database_id = HashMap::<i64, Option<String>>::new();
    for game in catalog_games.iter().filter(|game| game.launchbox_db_id > 0) {
        by_database_id
            .entry(game.launchbox_db_id)
            .and_modify(|identity| *identity = None)
            .or_insert_with(|| Some(game.id.clone()));
    }

    let mut resolved = Vec::new();
    let mut seen = HashSet::new();
    let mut unavailable = 0_usize;
    for reference in references {
        let matched = by_uid
            .get(reference.game_uid.as_str())
            .cloned()
            .or_else(|| {
                by_database_id
                    .get(&reference.launchbox_db_id)
                    .and_then(Clone::clone)
            });
        if let Some(game_uid) = matched {
            if seen.insert(game_uid.clone()) {
                resolved.push(ResolvedPortableGameReference {
                    game_uid,
                    playlist_title: reference.playlist_title.clone(),
                    playlist_notes: reference.playlist_notes.clone(),
                });
            }
        } else {
            unavailable = unavailable.saturating_add(1);
        }
    }
    (resolved, unavailable)
}

fn validate_portable_text(label: &str, value: &str, maximum: usize, empty: bool) -> Result<()> {
    if (!empty && value.trim().is_empty()) || value.chars().count() > maximum {
        bail!("portable collection {label} is empty or too long");
    }
    Ok(())
}

fn replace_atomically(temporary: &Path, output: &Path) -> Result<()> {
    if output.is_file() {
        let backup = output.with_extension("previous");
        fs::remove_file(&backup).ok();
        fs::rename(output, &backup)?;
        match fs::rename(temporary, output) {
            Ok(()) => {
                fs::remove_file(backup).ok();
                Ok(())
            }
            Err(error) => {
                let _ = fs::rename(&backup, output);
                Err(error).context("publishing collection export")
            }
        }
    } else {
        fs::rename(temporary, output).context("publishing collection export")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn game(id: &str, title: &str, platform: &str) -> Game {
        Game {
            id: id.to_owned(),
            launchbox_db_id: 1,
            media_id: 1,
            title: title.to_owned(),
            platform: platform.to_owned(),
            status: "canonical".to_owned(),
            local: false,
            downloadable: true,
            non_retail: false,
            has_non_retail_release: false,
            adult: false,
            release_regions: 0,
            cooperative: "unknown".to_owned(),
            search_key: String::new(),
        }
    }

    #[test]
    fn smart_rules_compose_without_establishing_identity() {
        let rules = SmartCollectionRules {
            platform: "Nintendo Entertainment System".to_owned(),
            availability: "downloadable".to_owned(),
            favorite: "favorite".to_owned(),
            ..SmartCollectionRules::default()
        }
        .normalized()
        .unwrap();
        let metroid = game("metroid", "Metroid", "Nintendo Entertainment System");
        let favorites = HashSet::from(["metroid".to_owned()]);
        assert!(rules.matches(&metroid, &favorites, &HashMap::new()));
        assert!(!rules.matches(&metroid, &HashSet::new(), &HashMap::new()));
        assert!(rules.summary().contains("Nintendo Entertainment System"));
    }

    #[test]
    fn smart_rules_match_exact_user_tags() {
        let rules = SmartCollectionRules {
            tag: "Family".to_owned(),
            ..SmartCollectionRules::default()
        }
        .normalized()
        .unwrap();
        let metroid = game("metroid", "Metroid", "Nintendo Entertainment System");
        assert!(rules.matches_with_display_title(
            &metroid,
            &HashSet::new(),
            &HashMap::new(),
            "",
            "Metroid",
            &["family".to_owned(), "Couch Co-op".to_owned()],
            "unknown",
        ));
        assert!(!rules.matches_with_display_title(
            &metroid,
            &HashSet::new(),
            &HashMap::new(),
            "",
            "Metroid",
            &["Family Friendly".to_owned()],
            "unknown",
        ));
        assert_eq!(rules.summary(), "tagged Family");
    }

    #[test]
    fn smart_rules_match_effective_cooperative_metadata() {
        let rules = SmartCollectionRules {
            cooperative: "yes".to_owned(),
            ..SmartCollectionRules::default()
        }
        .normalized()
        .unwrap();
        let game = game("mario-bros", "Mario Bros.", "Arcade");

        assert!(rules.matches_with_display_title(
            &game,
            &HashSet::new(),
            &HashMap::new(),
            "",
            "Mario Bros.",
            &[],
            "yes",
        ));
        assert!(!rules.matches_with_display_title(
            &game,
            &HashSet::new(),
            &HashMap::new(),
            "",
            "Mario Bros.",
            &[],
            "no",
        ));
        assert_eq!(rules.summary(), "co-op supported");
    }

    #[test]
    fn portable_collection_round_trip_is_bounded_and_exact() {
        let directory = tempfile::tempdir().unwrap();
        let path = directory.path().join("favorites.lunchbox-collection.json");
        let portable = PortableCollection::new(
            "Favorites".to_owned(),
            "Portable shelf".to_owned(),
            "manual".to_owned(),
            None,
            vec![PortableGameReference {
                game_uid: "stable-game".to_owned(),
                launchbox_db_id: 42,
                title: "Exact Game".to_owned(),
                platform: "Arcade".to_owned(),
                playlist_title: "Friday-night edition".to_owned(),
                playlist_notes: "Use the two-player cabinet.".to_owned(),
            }],
        )
        .unwrap();
        save_portable_collection(&path, &portable).unwrap();
        assert_eq!(load_portable_collection(&path).unwrap(), portable);
    }

    #[test]
    fn portable_resolution_uses_only_stable_exact_identifiers() {
        let mut metroid = game("metroid", "Metroid", "NES");
        metroid.launchbox_db_id = 42;
        let mut ambiguous_a = game("arcade-a", "Same", "Arcade");
        ambiguous_a.launchbox_db_id = 99;
        let mut ambiguous_b = game("arcade-b", "Same", "Arcade");
        ambiguous_b.launchbox_db_id = 99;
        let references = vec![
            PortableGameReference {
                game_uid: "metroid".to_owned(),
                launchbox_db_id: 999,
                title: "A different display title".to_owned(),
                platform: "A different display platform".to_owned(),
                playlist_title: "Collection alias".to_owned(),
                playlist_notes: "Collection notes".to_owned(),
            },
            PortableGameReference {
                game_uid: "old-metroid-id".to_owned(),
                launchbox_db_id: 42,
                title: "Metroid".to_owned(),
                platform: "NES".to_owned(),
                playlist_title: String::new(),
                playlist_notes: String::new(),
            },
            PortableGameReference {
                game_uid: "missing".to_owned(),
                launchbox_db_id: 99,
                title: "Same".to_owned(),
                platform: "Arcade".to_owned(),
                playlist_title: String::new(),
                playlist_notes: String::new(),
            },
        ];
        let (resolved, unavailable) =
            resolve_portable_game_presentations(&references, &[metroid, ambiguous_a, ambiguous_b]);
        assert_eq!(
            resolved
                .iter()
                .map(|reference| reference.game_uid.as_str())
                .collect::<Vec<_>>(),
            vec!["metroid"]
        );
        assert_eq!(unavailable, 1);
    }

    #[test]
    fn portable_collection_accepts_legacy_entries_without_playlist_fields() {
        let portable: PortableCollection = serde_json::from_str(
            r#"{
                "format":"lunchbox-collection",
                "version":1,
                "name":"Legacy",
                "description":"",
                "kind":"manual",
                "rules":null,
                "games":[{
                    "game_uid":"stable-game",
                    "launchbox_db_id":42,
                    "title":"Exact Game",
                    "platform":"Arcade"
                }]
            }"#,
        )
        .unwrap();
        portable.validate().unwrap();
        assert!(portable.games[0].playlist_title.is_empty());
        assert!(portable.games[0].playlist_notes.is_empty());
    }

    #[test]
    fn portable_resolution_preserves_playlist_presentation_by_exact_identity() {
        let mut exact = game("metroid", "Metroid", "NES");
        exact.launchbox_db_id = 42;
        let references = [PortableGameReference {
            game_uid: "metroid".to_owned(),
            launchbox_db_id: 42,
            title: "Metroid".to_owned(),
            platform: "NES".to_owned(),
            playlist_title: "Metroid night".to_owned(),
            playlist_notes: "Continue the shared save.".to_owned(),
        }];
        let (resolved, unavailable) = resolve_portable_game_presentations(&references, &[exact]);
        assert_eq!(unavailable, 0);
        assert_eq!(
            resolved,
            vec![ResolvedPortableGameReference {
                game_uid: "metroid".to_owned(),
                playlist_title: "Metroid night".to_owned(),
                playlist_notes: "Continue the shared save.".to_owned(),
            }]
        );
    }
}
