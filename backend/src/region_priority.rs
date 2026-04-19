//! Shared region priority helpers for browsing, variants, and Minerva downloads.

const DEFAULT_REGION_PRIORITY: &[&str] = &[
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
    "UK",
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
    "",
];

pub fn default_region_priority() -> Vec<String> {
    DEFAULT_REGION_PRIORITY
        .iter()
        .map(|region| (*region).to_string())
        .collect()
}

pub fn effective_region_priority(custom_order: &[String]) -> Vec<String> {
    let mut order = if custom_order.is_empty() {
        Vec::new()
    } else {
        custom_order.to_vec()
    };

    for region in DEFAULT_REGION_PRIORITY {
        if !order
            .iter()
            .any(|existing| existing.eq_ignore_ascii_case(region))
        {
            order.push((*region).to_string());
        }
    }

    if order.is_empty() {
        default_region_priority()
    } else {
        order
    }
}

pub fn priority_for_title(title: &str, custom_order: &[String]) -> i32 {
    let tags = crate::tags::get_region_tags(title);
    if tags.is_empty() {
        return priority_for_region(Some(""), custom_order);
    }

    tags.iter()
        .map(|tag| priority_for_region(Some(tag.as_str()), custom_order))
        .min()
        .unwrap_or_else(|| priority_for_region(None, custom_order))
}

pub fn priority_for_region(region: Option<&str>, custom_order: &[String]) -> i32 {
    let order = effective_region_priority(custom_order);
    let Some(region) = region.map(str::trim) else {
        return order.len() as i32;
    };

    if region.is_empty() {
        return order
            .iter()
            .position(|candidate| candidate.is_empty())
            .unwrap_or(order.len()) as i32;
    }

    let normalized_parts = split_region_parts(region);
    for (priority, candidate) in order.iter().enumerate() {
        if candidate.is_empty() {
            continue;
        }

        if normalized_parts
            .iter()
            .any(|part| region_part_matches(part, candidate))
        {
            return priority as i32;
        }
    }

    order.len() as i32
}

fn split_region_parts(region: &str) -> Vec<String> {
    region
        .split([',', '/', '&', '+'])
        .map(str::trim)
        .filter(|part| !part.is_empty())
        .map(|part| part.to_ascii_lowercase())
        .collect()
}

fn region_part_matches(region_part: &str, candidate: &str) -> bool {
    match candidate.to_ascii_lowercase().as_str() {
        "usa" => matches!(region_part, "usa" | "united states" | "north america"),
        "japan" => region_part == "japan",
        "asia" => region_part == "asia",
        "world" => region_part == "world",
        "uk" => matches!(region_part, "uk" | "united kingdom"),
        "united kingdom" => matches!(region_part, "united kingdom" | "uk"),
        other => region_part == other,
    }
}

#[cfg(test)]
mod tests {
    use super::{
        default_region_priority, effective_region_priority, priority_for_region, priority_for_title,
    };

    #[test]
    fn default_priority_starts_with_usa_japan_asia() {
        assert_eq!(
            default_region_priority()[..3],
            ["USA".to_string(), "Japan".to_string(), "Asia".to_string()]
        );
    }

    #[test]
    fn plain_versions_no_longer_beat_usa_by_default() {
        assert!(priority_for_title("Game (USA)", &[]) < priority_for_title("Game", &[]));
    }

    #[test]
    fn north_america_matches_usa_priority() {
        assert_eq!(
            priority_for_region(Some("North America"), &[]),
            priority_for_region(Some("USA"), &[])
        );
    }

    #[test]
    fn custom_order_is_respected_before_default_remainder() {
        let custom = vec!["Japan".to_string(), "USA".to_string()];
        let effective = effective_region_priority(&custom);
        assert_eq!(effective[0], "Japan");
        assert_eq!(effective[1], "USA");
        assert!(effective.iter().any(|region| region == "Asia"));
    }
}
