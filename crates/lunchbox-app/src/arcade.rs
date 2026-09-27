use std::borrow::Cow;
use std::collections::{HashMap, HashSet};
use std::sync::OnceLock;

// Imported libraries sometimes attach the same database ID (and even the
// same description) to unrelated video, pinball and gambling machines. Check
// their short names against the independent MAME catalog before using them.
static IDENTITIES: OnceLock<IdentityIndex> = OnceLock::new();

#[derive(Default)]
struct IdentityIndex {
    titles: HashMap<String, Vec<(String, String)>>,
    romsets: HashSet<String>,
}

impl IdentityIndex {
    fn add(&mut self, title: String, filename: String) {
        let path = std::path::Path::new(&filename);
        if !path
            .extension()
            .is_some_and(|ext| ext.eq_ignore_ascii_case("zip"))
        {
            return; // Individual ROM chips are not launchable machine names.
        }
        let Some(stem) = path.file_stem().and_then(|name| name.to_str()) else {
            return;
        };
        let key = crate::tags::normalize_title_for_matching(&title);
        let entries = self.titles.entry(key).or_default();
        let record = (stem.to_ascii_lowercase(), title);
        self.romsets.insert(record.0.clone());
        if !entries.contains(&record) {
            entries.push(record);
        }
    }

    fn entries(&self, title: &str) -> &[(String, String)] {
        self.titles
            .get(&crate::tags::normalize_title_for_matching(title))
            .map(Vec::as_slice)
            .unwrap_or_default()
    }
}

/// Load once on the catalog worker. Lookups, including artwork lookups, do no IO.
pub fn initialize(connection: &rusqlite::Connection) -> anyhow::Result<()> {
    if IDENTITIES.get().is_some() {
        return Ok(());
    }
    let mut index = IdentityIndex::default();
    let mut statement = connection.prepare(
        "SELECT DISTINCT r.title, r.file_name FROM libretro_records r
         JOIN libretro_databases d ON d.id=r.database_id
         WHERE d.source_name='MAME' AND r.title IS NOT NULL AND r.file_name IS NOT NULL",
    )?;
    for row in statement.query_map([], |row| {
        Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?))
    })? {
        let (title, filename) = row?;
        index.add(title, filename);
    }
    let _ = IDENTITIES.set(index);
    Ok(())
}

/// Exact provider spellings, including player count and revision, rather than
/// guessing console-style region suffixes for arcade thumbnail filenames.
pub fn artwork_titles(title: &str, database_id: Option<i64>) -> Vec<String> {
    let Some(index) = IDENTITIES.get() else {
        return Vec::new();
    };
    let selected = resolve_arcade_entry(title, database_id, true);
    let mut entries = index.entries(title).to_vec();
    entries.sort_by_key(|(romset, provider_title)| {
        (
            if selected.is_some_and(|entry| entry.video_lookup == romset) {
                0
            } else if selected.is_some_and(|entry| entry.preferred_lookup == romset) {
                1
            } else {
                2
            },
            provider_title.clone(),
        )
    });
    let mut titles = Vec::new();
    for (_, provider_title) in entries {
        if !titles.contains(&provider_title) {
            titles.push(provider_title);
        }
    }
    titles.truncate(8);
    titles
}

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
    pub gambling: bool,
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

    select_entry(entries, game_title, require_lookup, IDENTITIES.get())
}

fn select_entry<'a>(
    entries: &'a [ArcadeLookupEntry],
    game_title: &str,
    require_lookup: bool,
    identities: Option<&IdentityIndex>,
) -> Option<&'a ArcadeLookupEntry> {
    let verified = identities
        .map(|index| index.entries(game_title))
        .unwrap_or_default();

    let normalized_query = crate::tags::normalize_title_for_matching(game_title);
    let query_words: Vec<&str> = normalized_query.split_whitespace().collect();

    let candidates: Vec<_> = entries
        .iter()
        .filter(|entry| !require_lookup || !entry.preferred_lookup.is_empty())
        .filter(|entry| {
            verified.is_empty()
                || verified.iter().any(|(romset, _)| {
                    romset == entry.preferred_lookup
                        || (romset == entry.video_lookup
                            && identities.is_some_and(|index| {
                                !index.romsets.contains(entry.preferred_lookup)
                            }))
                })
        })
        .collect();
    let best = *candidates
        .iter()
        .max_by_key(|entry| score_entry(entry, &normalized_query, &query_words))?;
    let best_score = score_entry(best, &normalized_query, &query_words);
    // No subtype preference can prove that two identically named machines are
    // the same game. Without corroboration, leave the download unresolved.
    if candidates.iter().any(|entry| {
        score_entry(entry, &normalized_query, &query_words) == best_score
            && entry.video_lookup != best.video_lookup
    }) {
        return None;
    }
    Some(best)
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
        // Prefer the actual video game over a mechanical machine using the
        // same licensed name. Laserdisc originals outrank their machine ports.
        if entry.gambling {
            0
        } else {
            match entry.subtype {
                ArcadeSubtype::Pinball => 1,
                ArcadeSubtype::Standard => 2,
                ArcadeSubtype::Laserdisc => 3,
            }
        },
        entry.lookup_rank,
        normalized_title.len(),
    )
}

#[cfg(test)]
mod tests {
    use super::{
        ARCADE_LASERDISC_PLATFORM, ARCADE_LOOKUP, ARCADE_PINBALL_PLATFORM, ARCADE_PLATFORM,
        ArcadeSubtype, display_platform_name, resolve_download_lookup_name,
        resolve_video_lookup_name,
    };
    use super::{ArcadeLookupEntry, IdentityIndex, select_entry};

    fn has_local_lookup() -> bool {
        if let Some(path) = crate::catalog::requested_database_path()
            && let Ok(connection) = crate::catalog::open_read_only(&path, "arcade test")
        {
            super::initialize(&connection).unwrap();
        }
        !ARCADE_LOOKUP.is_empty()
    }

    fn machine(
        rom: &'static str,
        parent: &'static str,
        subtype: ArcadeSubtype,
    ) -> ArcadeLookupEntry {
        ArcadeLookupEntry {
            database_id: 7,
            title: "Shared title",
            source: "test",
            subtype,
            preferred_lookup: rom,
            video_lookup: parent,
            lookup_rank: 3,
            gambling: false,
        }
    }

    #[test]
    fn duplicate_catalog_id_requires_machine_identity_not_subtype_preference() {
        let mut index = IdentityIndex::default();
        index.add(
            "Shared title (2 Players World)".into(),
            "video2p.zip".into(),
        );
        index.add("Shared title (4 Players World)".into(), "video.zip".into());
        index.add("Shared title".into(), "pinball.rom".into());
        let mut entries = [
            machine("video2p", "video", ArcadeSubtype::Standard),
            machine("fruit", "fruit", ArcadeSubtype::Standard),
            machine("pinball", "pinball", ArcadeSubtype::Pinball),
        ];
        entries[1].gambling = true;
        for _ in 0..entries.len() {
            assert_eq!(
                select_entry(&entries, "Shared title", true, Some(&index))
                    .unwrap()
                    .preferred_lookup,
                "video2p"
            );
            assert_eq!(
                select_entry(&entries, "Shared title", true, None)
                    .unwrap()
                    .preferred_lookup,
                "video2p"
            );
            entries.rotate_left(1);
        }
    }

    #[test]
    fn equally_plausible_video_machines_are_not_chosen_arbitrarily() {
        let entries = [
            machine("video1", "video1", ArcadeSubtype::Standard),
            machine("video2", "video2", ArcadeSubtype::Standard),
        ];
        assert!(select_entry(&entries, "Shared title", true, None).is_none());
    }

    #[test]
    fn known_wrong_rom_cannot_borrow_a_correct_parent_identity() {
        let mut index = IdentityIndex::default();
        index.add("Shared title".into(), "video.zip".into());
        index.add("Another game".into(), "wrong.zip".into());
        assert!(
            select_entry(
                &[machine("wrong", "video", ArcadeSubtype::Standard)],
                "Shared title",
                true,
                Some(&index)
            )
            .is_none()
        );
    }

    #[test]
    fn known_title_rejects_all_incorrect_imported_machines() {
        let mut index = IdentityIndex::default();
        index.add("Shared title (World)".into(), "video.zip".into());
        assert!(
            select_entry(
                &[machine("pinball", "pinball", ArcadeSubtype::Pinball)],
                "Shared title",
                true,
                Some(&index)
            )
            .is_none()
        );
    }

    #[test]
    fn unrelated_names_sharing_an_id_do_not_override_the_exact_title() {
        let mut pinball = machine("pinball", "pinball", ArcadeSubtype::Pinball);
        pinball.title = "Other title";
        let entries = [machine("video", "video", ArcadeSubtype::Standard), pinball];
        assert_eq!(
            select_entry(&entries, "Other title", true, None)
                .unwrap()
                .preferred_lookup,
            "pinball"
        );
    }

    #[test]
    #[ignore = "requires the local canonical MAME catalog"]
    fn live_simpsons_identity_and_artwork_use_konami_machine() {
        assert!(has_local_lookup());
        assert_eq!(
            resolve_download_lookup_name("The Simpsons", Some(2455), false),
            "simpsons2p"
        );
        assert_eq!(
            resolve_video_lookup_name("The Simpsons", Some(2455)),
            "simpsons"
        );
        assert_eq!(
            display_platform_name("Arcade", "The Simpsons", Some(2455)),
            "Arcade"
        );
        let titles = super::artwork_titles("The Simpsons", Some(2455));
        assert_eq!(titles[0], "The Simpsons (4 Players World, set 1)");
        assert!(titles.contains(&"The Simpsons (2 Players World, set 1)".to_owned()));
        println!(
            "SIMPSONS_IDENTITY rom=simpsons2p parent=simpsons platform=Arcade artwork={titles:?}"
        );
    }

    #[test]
    fn falls_back_without_a_matching_catalog_record() {
        assert_eq!(
            resolve_download_lookup_name("Unknown Arcade Game", None, false),
            "Unknown Arcade Game"
        );
        assert_eq!(
            resolve_video_lookup_name("Unknown Arcade Game", Some(-1)),
            "Unknown Arcade Game"
        );
    }

    #[test]
    fn dragon_lair_ii_prefers_laserdisc_duplicate() {
        if !has_local_lookup() {
            return;
        }
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
        if !has_local_lookup() {
            return;
        }
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
    fn galaxy_force_ii_prefers_arcade_over_pinball_forceii() {
        if !has_local_lookup() {
            return;
        }
        assert_eq!(
            resolve_download_lookup_name("Galaxy Force II", Some(36801), false).as_ref(),
            "gforce2sd"
        );
        assert_eq!(
            resolve_video_lookup_name("Galaxy Force II", Some(36801)).as_ref(),
            "gforce2"
        );
    }

    #[test]
    fn space_ace_collision_prefers_laserdisc_entry() {
        if !has_local_lookup() {
            return;
        }
        assert_eq!(
            display_platform_name(ARCADE_PLATFORM, "Space Ace", Some(39466)).as_ref(),
            ARCADE_LASERDISC_PLATFORM
        );
    }

    #[test]
    fn note_based_laserdisc_titles_are_classified() {
        if !has_local_lookup() {
            return;
        }
        for (title, database_id) in [
            ("Crime Patrol", 28612),
            ("Mad Dog McCree", 28615),
            ("Star Rider", 448360),
        ] {
            assert_eq!(
                display_platform_name(ARCADE_PLATFORM, title, Some(database_id)).as_ref(),
                ARCADE_LASERDISC_PLATFORM
            );
        }
    }

    #[test]
    fn alg_source_titles_without_laserdisc_notes_are_classified() {
        if !has_local_lookup() {
            return;
        }
        for (title, database_id) in [
            ("Space Pirates", 39503),
            ("Mad Dog II: The Lost Gold", 37753),
            ("Who Shot Johnny Rock?", 40587),
        ] {
            assert_eq!(
                display_platform_name(ARCADE_PLATFORM, title, Some(database_id)).as_ref(),
                ARCADE_LASERDISC_PLATFORM
            );
        }
    }

    #[test]
    fn shared_laserdisc_descriptions_do_not_reclassify_gambling_machines() {
        if !has_local_lookup() {
            return;
        }
        let fruit = ARCADE_LOOKUP
            .iter()
            .find(|entry| entry.preferred_lookup == "sc4sace").unwrap();
        assert!(fruit.gambling);
        assert_eq!(fruit.subtype, ArcadeSubtype::Standard);
        let laserdisc = ARCADE_LOOKUP.iter().find(|entry| entry.preferred_lookup == "spaceace").unwrap();
        assert!(!laserdisc.gambling);
        assert_eq!(laserdisc.subtype, ArcadeSubtype::Laserdisc);
    }
}
