//! Game title tag parsing and normalization
//!
//! Handles parsing of parenthetical tags in game titles like:
//! - Regions: (USA), (Japan), (Europe), etc.
//! - Languages: (En), (En,Fr,De), etc.
//! - Revisions: (Rev 1), (v1.0), etc.
//! - Status: (Beta), (Proto), (Demo), etc.
//! - Disc info: (Disc 1), (Side A), etc.
//! - Platform/Distribution: (Virtual Console), (PSN), etc.


/// Tag category for classification
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum TagCategory {
    /// Geographic region: USA, Japan, Europe, World, etc.
    Region,
    /// Language codes: En, Ja, Fr, De, etc.
    Language,
    /// Version/revision: Rev 1, v1.0, v2.01, etc.
    Revision,
    /// Development status: Beta, Proto, Demo, Sample, etc.
    Status,
    /// Disc/media info: Disc 1, Side A, Card 1, etc.
    DiscInfo,
    /// Platform or distribution: Virtual Console, PSN, eShop, etc.
    Platform,
    /// Content type: Addon, DLC, Update, etc.
    ContentType,
    /// Special editions: Limited Edition, Collector's Edition, etc.
    Edition,
    /// License status: Unl, Pirate, Aftermarket, etc.
    License,
    /// Hardware features: GB Compatible, SGB Enhanced, etc.
    Hardware,
    /// Unknown/other tags
    Other,
}

/// A parsed tag from a game title
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ParsedTag {
    /// The tag text without parentheses
    pub text: String,
    /// The category of this tag
    pub category: TagCategory,
    /// Original text with parentheses
    pub original: String,
}

// ============================================================================
// Region tags - geographic locations
// ============================================================================

/// Primary single-country/region tags
const REGIONS_PRIMARY: &[&str] = &[
    "USA", "Japan", "Europe", "World", "Korea", "Germany", "France", "Spain",
    "Italy", "Asia", "China", "Taiwan", "UK", "Netherlands", "Russia",
    "Australia", "Brazil", "Sweden", "Canada", "Poland", "Portugal", "Denmark",
    "Norway", "Finland", "Belgium", "Austria", "Switzerland", "Greece",
    "Hong Kong", "Latin America", "Scandinavia", "United Kingdom", "Unknown",
];

/// Multi-region combined tags (order matters for matching)
const REGIONS_COMBINED: &[&str] = &[
    "USA, Europe", "USA, Europe, Asia", "USA, Europe, Brazil", "USA, Europe, Korea",
    "USA, Asia", "USA, Japan", "USA, Korea", "USA, Brazil", "USA, Canada",
    "USA, Australia",
    "Europe, Australia", "Europe, Asia", "Europe, Brazil", "Europe, USA",
    "Japan, Europe", "Japan, USA", "Japan, Korea", "Japan, Asia",
    "Japan, Europe, Australia, New Zealand", "Japan, Australia",
    "Japan, USA, Brazil", "Japan, USA, Korea", "Japan, Europe, Korea",
    "Asia, Korea",
    "UK, Australia", "Australia, New Zealand",
    "Austria, Switzerland", "Belgium, Netherlands",
];

// ============================================================================
// Language tags - ISO 639-1 codes and combinations
// ============================================================================

/// Two-letter language codes
const LANGUAGE_CODES: &[&str] = &[
    "En", "Ja", "Fr", "De", "Es", "It", "Nl", "Pt", "Ru", "Zh", "Ko",
    "Sv", "Da", "Fi", "No", "Pl", "Ar", "El", "Tr", "Cs", "Hu", "He",
    "Hi", "Th", "Vi", "Id", "Ms", "Ca", "Hr", "Sl", "Ro", "Bg", "Uk",
    "Sr", "Lt", "Lv", "Et", "Is", "Ga", "Mt", "Sk", "Mk",
];

// ============================================================================
// Status tags - development/release status
// ============================================================================

/// Beta/prototype/demo status tags
const STATUS_TAGS: &[&str] = &[
    "Beta", "Proto", "Prototype", "Demo", "Sample", "Promo", "Alt",
    "Debug", "Test", "Kiosk", "Trade Demo", "Possible Proto", "Tech Demo",
    "Preview", "Pre-Release", "Early", "WIP",
];

/// Numbered status tags (patterns)
const STATUS_NUMBERED_PREFIXES: &[&str] = &[
    "Beta ", "Proto ", "Demo ", "Alt ", "Sample ",
];

// ============================================================================
// Revision tags - version numbers
// ============================================================================

/// Revision prefixes
const REVISION_PREFIXES: &[&str] = &[
    "Rev ", "v", "Ver ", "Version ",
];

// ============================================================================
// Disc/media tags
// ============================================================================

const DISC_PREFIXES: &[&str] = &[
    "Disc ", "Disk ", "Side ", "Card ", "Volume ", "Vol ", "Part ",
];

// ============================================================================
// Platform/distribution tags
// ============================================================================

const PLATFORM_TAGS: &[&str] = &[
    // Digital distribution
    "Virtual Console", "PSN", "eShop", "WiiWare", "XBLA", "XBLIG", "Steam",
    "GOD", "minis", "Switch Online", "DSiWare",
    // Console-specific
    "NES", "SNES", "N64", "GameCube", "Wii", "Wii U", "Switch",
    "GB", "GBC", "GBA", "DS", "3DS",
    "PS1", "PS2", "PS3", "PS4", "PS5", "PSP", "Vita",
    "Xbox", "Xbox 360", "Xbox One",
    "Genesis", "Mega Drive", "Saturn", "Dreamcast",
    "TurboGrafx-16", "PC Engine", "Neo Geo",
    // Virtual console variants
    "Wii Virtual Console", "Wii U Virtual Console", "3DS Virtual Console",
    // Broadcast/channel
    "Channel", "Wii Broadcast", "Nintendo Channel",
    // Mini consoles
    "Classic Mini", "Mega Drive Mini", "Genesis Mini",
    // Collections
    "Evercade", "Arcade",
];

// ============================================================================
// Content type tags
// ============================================================================

const CONTENT_TAGS: &[&str] = &[
    "Addon", "DLC", "Update", "Patch", "Expansion", "Data", "Save Data",
    "Title Update", "Content", "Bonus", "Bonus Disc", "Collection",
    "Compilation", "Video", "Album", "Manual", "Menu", "System",
    "Download Station", "Kiosk Demo",
];

// ============================================================================
// License tags
// ============================================================================

const LICENSE_TAGS: &[&str] = &[
    "Unl", "Pirate", "Aftermarket", "Bootleg", "Hack", "Homebrew",
    "Budget", "Rerelease",
];

// ============================================================================
// Hardware feature tags
// ============================================================================

const HARDWARE_TAGS: &[&str] = &[
    "GB Compatible", "SGB Enhanced", "NDSi Enhanced", "Rumble Version",
    "Color", "Greyscale", "PAL", "NTSC", "Enhancement Chip",
    "FamicomBox", "PlayChoice-10", "VS. System",
];

// ============================================================================
// Edition tags (patterns to match)
// ============================================================================

const EDITION_SUFFIXES: &[&str] = &[
    "Edition", "Box", "Pack", "Bundle", "Set", "Collection",
    "Limited Edition", "Collector's Edition", "Special Edition",
    "Premium Box", "Deluxe Pack", "Game of the Year",
];

// ============================================================================
// Public API
// ============================================================================

/// Parse all tags from a game title
/// Returns the base title (without tags) and a list of parsed tags
pub fn parse_title_tags(title: &str) -> (String, Vec<ParsedTag>) {
    let mut tags = Vec::new();
    let mut base_title = title.to_string();

    // Find all parenthetical expressions
    let mut i = 0;
    while let Some(start) = base_title[i..].find('(') {
        let abs_start = i + start;
        if let Some(end) = base_title[abs_start..].find(')') {
            let abs_end = abs_start + end + 1;
            let tag_with_parens = &base_title[abs_start..abs_end];
            let tag_text = &base_title[abs_start + 1..abs_end - 1];

            let category = categorize_tag(tag_text);
            tags.push(ParsedTag {
                text: tag_text.to_string(),
                category,
                original: tag_with_parens.to_string(),
            });
            i = abs_end;
        } else {
            break;
        }
    }

    // Remove all tags from base title
    for tag in &tags {
        base_title = base_title.replace(&tag.original, "");
    }

    // Clean up base title
    base_title = base_title.trim().to_string();
    // Remove trailing punctuation that might be left
    while base_title.ends_with(" -") || base_title.ends_with(",") {
        base_title = base_title.trim_end_matches(" -").trim_end_matches(",").trim().to_string();
    }

    (base_title, tags)
}

/// Categorize a single tag (without parentheses)
pub fn categorize_tag(tag: &str) -> TagCategory {
    let tag_lower = tag.to_lowercase();
    let tag_trimmed = tag.trim();

    // Check regions first (most common)
    for region in REGIONS_PRIMARY {
        if tag_trimmed.eq_ignore_ascii_case(region) {
            return TagCategory::Region;
        }
    }
    for region in REGIONS_COMBINED {
        if tag_trimmed.eq_ignore_ascii_case(region) {
            return TagCategory::Region;
        }
    }

    // Check if it's a language tag (comma-separated 2-letter codes)
    if is_language_tag(tag_trimmed) {
        return TagCategory::Language;
    }

    // Check revision tags
    for prefix in REVISION_PREFIXES {
        if tag_lower.starts_with(&prefix.to_lowercase()) {
            return TagCategory::Revision;
        }
    }

    // Check status tags
    for status in STATUS_TAGS {
        if tag_trimmed.eq_ignore_ascii_case(status) {
            return TagCategory::Status;
        }
    }
    for prefix in STATUS_NUMBERED_PREFIXES {
        if tag_lower.starts_with(&prefix.to_lowercase()) {
            return TagCategory::Status;
        }
    }

    // Check disc/media tags
    for prefix in DISC_PREFIXES {
        if tag_lower.starts_with(&prefix.to_lowercase()) {
            return TagCategory::DiscInfo;
        }
    }

    // Check platform tags
    for platform in PLATFORM_TAGS {
        if tag_trimmed.eq_ignore_ascii_case(platform) {
            return TagCategory::Platform;
        }
    }

    // Check content tags
    for content in CONTENT_TAGS {
        if tag_trimmed.eq_ignore_ascii_case(content) {
            return TagCategory::ContentType;
        }
    }

    // Check license tags
    for license in LICENSE_TAGS {
        if tag_trimmed.eq_ignore_ascii_case(license) {
            return TagCategory::License;
        }
    }

    // Check hardware tags
    for hw in HARDWARE_TAGS {
        if tag_trimmed.eq_ignore_ascii_case(hw) {
            return TagCategory::Hardware;
        }
    }

    // Check edition suffixes
    for suffix in EDITION_SUFFIXES {
        if tag_lower.contains(&suffix.to_lowercase()) {
            return TagCategory::Edition;
        }
    }

    TagCategory::Other
}

/// Check if a tag is a language tag (e.g., "En", "En,Fr,De")
fn is_language_tag(tag: &str) -> bool {
    // Split by comma and check each part
    let parts: Vec<&str> = tag.split(',').collect();
    if parts.is_empty() {
        return false;
    }

    for part in parts {
        let trimmed = part.trim();
        // Must be 2-3 chars and match a language code
        if trimmed.len() < 2 || trimmed.len() > 3 {
            return false;
        }
        let is_lang = LANGUAGE_CODES.iter().any(|code| code.eq_ignore_ascii_case(trimmed));
        if !is_lang {
            return false;
        }
    }
    true
}

/// Remove region and language tags from a title, returning a normalized base name
pub fn strip_region_and_language_tags(title: &str) -> String {
    let (base, _tags) = parse_title_tags(title);
    base.trim().to_string()
}

/// Normalize a game title for matching purposes
/// - Lowercase
/// - Remove all tags
/// - Remove special characters except alphanumeric and spaces
/// - Collapse multiple spaces
pub fn normalize_title_for_matching(title: &str) -> String {
    let (base, _tags) = parse_title_tags(title);

    base.to_lowercase()
        .chars()
        .filter(|c| c.is_alphanumeric() || c.is_whitespace())
        .collect::<String>()
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
}

/// Get all region tags from a title
pub fn get_region_tags(title: &str) -> Vec<String> {
    let (_base, tags) = parse_title_tags(title);
    tags.into_iter()
        .filter(|t| t.category == TagCategory::Region)
        .map(|t| t.text)
        .collect()
}

/// Get all language tags from a title
pub fn get_language_tags(title: &str) -> Vec<String> {
    let (_base, tags) = parse_title_tags(title);
    tags.into_iter()
        .filter(|t| t.category == TagCategory::Language)
        .map(|t| t.text)
        .collect()
}

/// Check if a title has a specific region
pub fn has_region(title: &str, region: &str) -> bool {
    let regions = get_region_tags(title);
    regions.iter().any(|r| {
        r.eq_ignore_ascii_case(region) ||
        r.to_lowercase().contains(&region.to_lowercase())
    })
}

/// Get region priority for sorting (lower = preferred)
/// USA/World are typically preferred, followed by Europe, then others
pub fn region_priority(region: &str) -> i32 {
    match region.to_lowercase().as_str() {
        "usa" | "north america" | "united states" => 0,
        "world" => 1,
        "japan" => 2,
        "europe" => 3,
        "australia" => 4,
        "asia" => 10,
        "korea" => 11,
        "china" => 12,
        "taiwan" => 13,
        "brazil" => 20,
        "canada" => 21,
        "france" => 22,
        "germany" => 23,
        "italy" => 24,
        "spain" => 25,
        "uk" | "united kingdom" => 26,
        _ => 100,
    }
}

/// Sort regions by priority (returns sorted list)
pub fn sort_regions_by_priority(regions: &[String]) -> Vec<String> {
    let mut sorted = regions.to_vec();
    sorted.sort_by_key(|r| region_priority(r));
    sorted
}

/// Check if a tag indicates the title is not a main release
/// (demo, beta, proto, pirate, etc.)
pub fn is_non_release_tag(tag: &ParsedTag) -> bool {
    matches!(tag.category,
        TagCategory::Status |
        TagCategory::License
    ) && !tag.text.eq_ignore_ascii_case("Aftermarket")
}

/// Filter tags to only include those useful for display
pub fn filter_display_tags(tags: &[ParsedTag]) -> Vec<&ParsedTag> {
    tags.iter()
        .filter(|t| matches!(t.category,
            TagCategory::Region |
            TagCategory::Revision |
            TagCategory::Status |
            TagCategory::DiscInfo
        ))
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_title_tags() {
        let (base, tags) = parse_title_tags("Super Mario Bros. (USA) (Rev 1)");
        assert_eq!(base, "Super Mario Bros.");
        assert_eq!(tags.len(), 2);
        assert_eq!(tags[0].text, "USA");
        assert_eq!(tags[0].category, TagCategory::Region);
        assert_eq!(tags[1].text, "Rev 1");
        assert_eq!(tags[1].category, TagCategory::Revision);
    }

    #[test]
    fn test_parse_language_tags() {
        let (base, tags) = parse_title_tags("Game (En,Fr,De)");
        assert_eq!(base, "Game");
        assert_eq!(tags.len(), 1);
        assert_eq!(tags[0].category, TagCategory::Language);
    }

    #[test]
    fn test_categorize_status() {
        assert_eq!(categorize_tag("Beta"), TagCategory::Status);
        assert_eq!(categorize_tag("Beta 2"), TagCategory::Status);
        assert_eq!(categorize_tag("Proto"), TagCategory::Status);
        assert_eq!(categorize_tag("Demo"), TagCategory::Status);
    }

    #[test]
    fn test_normalize_title() {
        let normalized = normalize_title_for_matching("Super Mario Bros. (USA) (Rev 1)");
        assert_eq!(normalized, "super mario bros");
    }

    #[test]
    fn test_region_priority() {
        assert!(region_priority("USA") < region_priority("Europe"));
        assert!(region_priority("World") < region_priority("Japan"));
    }

    #[test]
    fn test_combined_regions() {
        let (_, tags) = parse_title_tags("Game (USA, Europe)");
        assert_eq!(tags[0].category, TagCategory::Region);
    }
}
