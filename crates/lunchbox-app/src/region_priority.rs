//! Shared region ordering for catalog variants and Minerva candidates.
//!
//! The native frontend keeps the complete ordering exposed by legacy Lunchbox,
//! but normalizes aliases and duplicate entries before persisting them.

use std::collections::HashSet;

use anyhow::{Result, bail};

pub const DEFAULT_REGION_PRIORITY: &[&str] = &[
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
    "",
];

pub fn default_region_priority() -> Vec<String> {
    DEFAULT_REGION_PRIORITY
        .iter()
        .map(|region| (*region).to_owned())
        .collect()
}

pub fn normalize_custom_priority(custom_order: &[String]) -> Result<Vec<String>> {
    let mut normalized = Vec::with_capacity(custom_order.len());
    let mut seen = HashSet::new();
    for region in custom_order {
        let canonical = canonical_region(region).ok_or_else(|| {
            anyhow::anyhow!("unsupported release region {}", display_name(region))
        })?;
        let key = canonical.to_ascii_lowercase();
        if !seen.insert(key) {
            bail!(
                "release region {} occurs more than once",
                display_name(canonical)
            );
        }
        normalized.push(canonical.to_owned());
    }
    Ok(normalized)
}

pub fn effective_region_priority(custom_order: &[String]) -> Vec<String> {
    let mut order = normalize_custom_priority(custom_order).unwrap_or_default();
    for region in DEFAULT_REGION_PRIORITY {
        if !order
            .iter()
            .any(|existing| existing.eq_ignore_ascii_case(region))
        {
            order.push((*region).to_owned());
        }
    }
    order
}

pub fn priority_for_region(region: Option<&str>, custom_order: &[String]) -> usize {
    let order = effective_region_priority(custom_order);
    let Some(region) = region.map(str::trim) else {
        return order.len();
    };
    if region.is_empty() {
        return order
            .iter()
            .position(|candidate| candidate.is_empty())
            .unwrap_or(order.len());
    }

    let parts = region
        .split([',', '/', '&', '+'])
        .map(str::trim)
        .filter_map(canonical_region)
        .collect::<Vec<_>>();
    order
        .iter()
        .position(|candidate| {
            parts
                .iter()
                .any(|part| part.eq_ignore_ascii_case(candidate))
        })
        .unwrap_or(order.len())
}

pub fn display_name(region: &str) -> &str {
    if region.is_empty() {
        "No region (plain title)"
    } else {
        region
    }
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
        "" => "",
        _ => return None,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn legacy_default_starts_with_usa_japan_asia() {
        assert_eq!(
            &default_region_priority()[..3],
            &["USA".to_owned(), "Japan".to_owned(), "Asia".to_owned()]
        );
    }

    #[test]
    fn custom_order_leads_and_default_remainder_is_appended() {
        let effective = effective_region_priority(&["Japan".to_owned(), "USA".to_owned()]);
        assert_eq!(&effective[..2], &["Japan".to_owned(), "USA".to_owned()]);
        assert!(effective.iter().any(|region| region == "Europe"));
        assert_eq!(effective.last().map(String::as_str), Some(""));
    }

    #[test]
    fn aliases_are_normalized_and_duplicates_are_rejected() {
        assert_eq!(
            normalize_custom_priority(&["North America".to_owned(), "UK".to_owned()]).unwrap(),
            ["USA".to_owned(), "United Kingdom".to_owned()]
        );
        assert!(
            normalize_custom_priority(&["UK".to_owned(), "United Kingdom".to_owned()]).is_err()
        );
    }

    #[test]
    fn multi_region_candidates_use_the_best_configured_match() {
        let order = ["Europe".to_owned(), "Japan".to_owned()];
        assert_eq!(priority_for_region(Some("USA, Europe"), &order), 0);
        assert_eq!(priority_for_region(Some("Japan"), &order), 1);
        assert!(
            priority_for_region(Some("Unknown"), &order)
                < priority_for_region(Some("Unrecognized"), &order)
        );
    }
}
