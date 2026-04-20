use std::borrow::Cow;

pub const ARCADE_PLATFORM: &str = "Arcade";
pub const ARCADE_PINBALL_PLATFORM: &str = "Arcade Pinball";
pub const ARCADE_LASERDISC_PLATFORM: &str = "Arcade Laserdisc";

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ArcadeSubtype {
    Standard,
    Pinball,
    Laserdisc,
}

#[derive(Debug, Clone, Copy)]
pub struct ArcadeLookupEntry {
    pub database_id: i64,
    pub title: &'static str,
    pub source: &'static str,
    pub subtype: ArcadeSubtype,
    pub preferred_lookup: &'static str,
    pub video_lookup: &'static str,
    pub lookup_rank: u8,
}

include!(concat!(env!("OUT_DIR"), "/arcade_lookup.rs"));

pub fn canonicalize_platform_name(name: &str) -> &str {
    match name.trim() {
        ARCADE_PINBALL_PLATFORM | ARCADE_LASERDISC_PLATFORM => ARCADE_PLATFORM,
        other => other,
    }
}

pub fn is_arcade_family_platform(name: &str) -> bool {
    matches!(
        name.trim(),
        ARCADE_PLATFORM | ARCADE_PINBALL_PLATFORM | ARCADE_LASERDISC_PLATFORM
    )
}

pub fn is_arcade_derived_platform(name: &str) -> bool {
    matches!(
        name.trim(),
        ARCADE_PINBALL_PLATFORM | ARCADE_LASERDISC_PLATFORM
    )
}

pub fn display_platform_name<'a>(
    platform_name: &'a str,
    game_title: &str,
    launchbox_db_id: Option<i64>,
) -> Cow<'a, str> {
    let trimmed = platform_name.trim();
    if !is_arcade_family_platform(trimmed) {
        return Cow::Borrowed(trimmed);
    }

    match resolve_arcade_subtype(game_title, launchbox_db_id) {
        ArcadeSubtype::Pinball => Cow::Borrowed(ARCADE_PINBALL_PLATFORM),
        ArcadeSubtype::Laserdisc => Cow::Borrowed(ARCADE_LASERDISC_PLATFORM),
        ArcadeSubtype::Standard => match trimmed {
            ARCADE_PINBALL_PLATFORM | ARCADE_LASERDISC_PLATFORM => Cow::Borrowed(trimmed),
            _ => Cow::Borrowed(ARCADE_PLATFORM),
        },
    }
}

pub fn resolve_download_lookup_name<'a>(
    game_title: &'a str,
    launchbox_db_id: Option<i64>,
    use_parent_lookup: bool,
) -> Cow<'a, str> {
    let Some(entry) = resolve_arcade_entry(game_title, launchbox_db_id, true) else {
        return Cow::Borrowed(game_title);
    };

    let lookup = if use_parent_lookup {
        entry.video_lookup
    } else {
        entry.preferred_lookup
    };
    if lookup.is_empty() {
        Cow::Borrowed(game_title)
    } else {
        Cow::Borrowed(lookup)
    }
}

pub fn resolve_video_lookup_name<'a>(
    game_title: &'a str,
    launchbox_db_id: Option<i64>,
) -> Cow<'a, str> {
    let Some(entry) = resolve_arcade_entry(game_title, launchbox_db_id, true) else {
        return Cow::Borrowed(game_title);
    };

    if entry.video_lookup.is_empty() {
        Cow::Borrowed(game_title)
    } else {
        Cow::Borrowed(entry.video_lookup)
    }
}

fn resolve_arcade_subtype(game_title: &str, launchbox_db_id: Option<i64>) -> ArcadeSubtype {
    resolve_arcade_entry(game_title, launchbox_db_id, false)
        .map(|entry| entry.subtype)
        .unwrap_or(ArcadeSubtype::Standard)
}

fn resolve_arcade_entry(
    game_title: &str,
    launchbox_db_id: Option<i64>,
    require_lookup: bool,
) -> Option<&'static ArcadeLookupEntry> {
    let launchbox_db_id = launchbox_db_id?;
    let entries = entries_for_db_id(launchbox_db_id);
    if entries.is_empty() {
        return None;
    }

    let normalized_query = crate::tags::normalize_title_for_matching(game_title);
    let query_words: Vec<&str> = normalized_query.split_whitespace().collect();

    entries
        .iter()
        .filter(|entry| !require_lookup || !entry.preferred_lookup.is_empty())
        .max_by_key(|entry| score_entry(entry, &normalized_query, &query_words))
}

fn entries_for_db_id(database_id: i64) -> &'static [ArcadeLookupEntry] {
    let start = ARCADE_LOOKUP.partition_point(|entry| entry.database_id < database_id);
    let end = ARCADE_LOOKUP.partition_point(|entry| entry.database_id <= database_id);
    &ARCADE_LOOKUP[start..end]
}

fn score_entry(
    entry: &ArcadeLookupEntry,
    normalized_query: &str,
    query_words: &[&str],
) -> (u8, u8, usize, u8, u8, u8, usize) {
    let normalized_title = crate::tags::normalize_title_for_matching(entry.title);
    let title_words: Vec<&str> = normalized_title.split_whitespace().collect();
    let common_words = query_words
        .iter()
        .filter(|word| title_words.contains(word))
        .count();
    let exact_match = (!normalized_query.is_empty() && normalized_title == normalized_query) as u8;
    let all_query_words_match =
        (!query_words.is_empty() && common_words == query_words.len()) as u8;
    let contains_match = (!normalized_query.is_empty()
        && (normalized_title.contains(normalized_query)
            || normalized_query.contains(&normalized_title))) as u8;

    (
        exact_match,
        all_query_words_match,
        common_words,
        contains_match,
        entry.lookup_rank,
        subtype_priority(entry.subtype),
        normalized_title.len(),
    )
}

fn subtype_priority(subtype: ArcadeSubtype) -> u8 {
    match subtype {
        ArcadeSubtype::Standard => 0,
        ArcadeSubtype::Pinball => 1,
        ArcadeSubtype::Laserdisc => 2,
    }
}

#[cfg(test)]
mod tests {
    use super::{
        ARCADE_LASERDISC_PLATFORM, ARCADE_PINBALL_PLATFORM, ARCADE_PLATFORM, display_platform_name,
        resolve_download_lookup_name, resolve_video_lookup_name,
    };

    #[test]
    fn dragon_lair_ii_prefers_laserdisc_duplicate() {
        assert_eq!(
            display_platform_name(ARCADE_PLATFORM, "Dragon's Lair II: Time Warp", Some(12256))
                .as_ref(),
            ARCADE_LASERDISC_PLATFORM
        );
        assert_eq!(
            resolve_video_lookup_name("Dragon's Lair II: Time Warp", Some(12256)).as_ref(),
            "dlair2"
        );
    }

    #[test]
    fn time_warp_prefers_pinball_duplicate() {
        assert_eq!(
            display_platform_name(ARCADE_PLATFORM, "Time Warp", Some(12256)).as_ref(),
            ARCADE_PINBALL_PLATFORM
        );
        assert_eq!(
            resolve_download_lookup_name("Time Warp", Some(12256), false).as_ref(),
            "tmwrp_l3"
        );
    }

    #[test]
    fn space_ace_collision_prefers_laserdisc_entry() {
        assert_eq!(
            display_platform_name(ARCADE_PLATFORM, "Space Ace", Some(39466)).as_ref(),
            ARCADE_LASERDISC_PLATFORM
        );
    }
}
