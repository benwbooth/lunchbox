//! Game details panel

use super::{
    Box3DViewer, LazyImage, VideoPlayer, minerva_downloads_signal,
    refresh_minerva_download_queue_now, request_minerva_download_queue_refresh,
};
use crate::backend_api::{
    self, ControllerProfileInfo, EmulatorWithStatus, Game, GameFile, GameVariant, PlayStats,
    file_to_asset_url,
};
use futures::stream::{self, StreamExt};
use leptos::prelude::*;
use leptos::task::spawn_local;
use std::collections::{BTreeMap, BTreeSet, HashMap, HashSet};
use wasm_bindgen::{JsCast, JsValue};

async fn launch_game_with_resolved_rom(
    launchbox_db_id: i64,
    platform: String,
    fallback_rom_path: Option<String>,
    emulator_name: String,
    is_retroarch_core: bool,
) -> Result<backend_api::LaunchResult, String> {
    let rom_path = if let Some(path) = fallback_rom_path.and_then(|path| {
        if path.trim().is_empty() {
            None
        } else {
            Some(path)
        }
    }) {
        path
    } else {
        match backend_api::get_game_file(launchbox_db_id).await {
            Ok(Some(file)) if !file.file_path.trim().is_empty() => file.file_path,
            _ => return Err("No ROM file path is available for this game".to_string()),
        }
    };

    backend_api::launch_game(
        emulator_name,
        Some(rom_path),
        Some(launchbox_db_id),
        Some(platform),
        is_retroarch_core,
    )
    .await
}

async fn resolve_game_file_for_display(game: &Game) -> Option<GameFile> {
    if game.database_id > 0 {
        if let Ok(Some(file)) = backend_api::get_game_file(game.database_id).await {
            return Some(file);
        }
    }

    let mut checked_ids: HashSet<i64> = HashSet::new();

    let variants = backend_api::get_game_variants(
        game.id.clone(),
        game.display_title.clone(),
        game.platform_id,
    )
    .await
    .ok()?;

    for variant in variants {
        let variant_game = match backend_api::get_game_by_uuid(variant.id).await {
            Ok(Some(g)) => g,
            _ => continue,
        };
        if variant_game.database_id <= 0 || !checked_ids.insert(variant_game.database_id) {
            continue;
        }
        if let Ok(Some(file)) = backend_api::get_game_file(variant_game.database_id).await {
            return Some(file);
        }
    }

    None
}

async fn resolve_display_game_identity(game: &Game) -> Game {
    if game.database_id > 0 {
        return game.clone();
    }

    match backend_api::get_game_by_uuid(game.id.clone()).await {
        Ok(Some(resolved)) if resolved.database_id > 0 => resolved,
        _ => game.clone(),
    }
}

async fn refresh_display_game_file_state(
    display_game: ReadSignal<Option<Game>>,
    set_display_game: WriteSignal<Option<Game>>,
    set_game_file: WriteSignal<Option<GameFile>>,
    attempts: usize,
) {
    for attempt in 0..attempts {
        let Some(game_snapshot) = display_game.get_untracked() else {
            return;
        };

        let resolved_game = resolve_display_game_identity(&game_snapshot).await;
        if resolved_game.database_id > 0 && resolved_game.database_id != game_snapshot.database_id {
            set_display_game.set(Some(resolved_game.clone()));
        }

        if let Some(file) = resolve_game_file_for_display(&resolved_game).await {
            set_game_file.set(Some(file));
            set_display_game.update(|game| {
                if let Some(game) = game.as_mut() {
                    game.has_game_file = true;
                    if game.database_id <= 0 && resolved_game.database_id > 0 {
                        *game = resolved_game.clone();
                        game.has_game_file = true;
                    }
                }
            });
            return;
        }

        if attempt + 1 < attempts {
            delay_ms(300).await;
        }
    }
}

fn pause_game_details_video() {
    let Some(window) = web_sys::window() else {
        return;
    };
    let Some(document) = window.document() else {
        return;
    };
    let Ok(Some(video_el)) = document.query_selector(".game-video") else {
        return;
    };

    let pause_value = match js_sys::Reflect::get(video_el.as_ref(), &JsValue::from_str("pause")) {
        Ok(value) => value,
        Err(_) => return,
    };
    let Some(pause_fn) = pause_value.dyn_ref::<js_sys::Function>() else {
        return;
    };
    let _ = pause_fn.call0(video_el.as_ref());
}

fn open_with_system_handler(path: String, set_manual_error: WriteSignal<Option<String>>) {
    spawn_local(async move {
        if let Err(e) = backend_api::open_local_file(path).await {
            set_manual_error.set(Some(e));
        }
    });
}

fn format_bytes(bytes: u64) -> String {
    if bytes >= 1_000_000_000 {
        format!("{:.1} GB", bytes as f64 / 1_000_000_000.0)
    } else if bytes >= 1_000_000 {
        format!("{:.1} MB", bytes as f64 / 1_000_000.0)
    } else if bytes >= 1_000 {
        format!("{:.1} KB", bytes as f64 / 1_000.0)
    } else {
        format!("{bytes} B")
    }
}

fn format_speed(bytes_per_sec: u64) -> String {
    if bytes_per_sec >= 1_000_000 {
        format!("{:.1} MB/s", bytes_per_sec as f64 / 1_000_000.0)
    } else if bytes_per_sec >= 1_000 {
        format!("{:.1} KB/s", bytes_per_sec as f64 / 1_000.0)
    } else {
        format!("{bytes_per_sec} B/s")
    }
}

async fn delay_ms(ms: i32) {
    wasm_bindgen_futures::JsFuture::from(js_sys::Promise::new(&mut |resolve, _| {
        web_sys::window()
            .unwrap()
            .set_timeout_with_callback_and_timeout_and_arguments_0(&resolve, ms)
            .unwrap();
    }))
    .await
    .unwrap();
}

#[derive(Clone, Debug)]
struct MinervaTorrentGroup {
    rom: backend_api::MinervaRom,
    items: Vec<MinervaPickerItem>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
enum MinervaDownloadSelection {
    WholeTorrent {
        torrent_url: String,
        representative_file_index: Option<usize>,
    },
    File {
        torrent_url: String,
        file_index: usize,
    },
}

#[derive(Clone, Debug)]
struct MinervaPickerItem {
    selection: MinervaDownloadSelection,
    display_name: String,
    path_detail: Option<String>,
    size: u64,
    match_score: f64,
    region: Option<String>,
    suggested_emulator: Option<String>,
    type_badge: Option<String>,
}

// Keep this fallback order in sync with backend/src/region_priority.rs.
const DEFAULT_DOWNLOAD_REGION_PRIORITY: &[&str] = &[
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

fn effective_download_region_priority(custom_order: &[String]) -> Vec<String> {
    let mut order = if custom_order.is_empty() {
        Vec::new()
    } else {
        custom_order.to_vec()
    };

    for region in DEFAULT_DOWNLOAD_REGION_PRIORITY {
        if !order
            .iter()
            .any(|existing| existing.eq_ignore_ascii_case(region))
        {
            order.push((*region).to_string());
        }
    }

    order
}

fn grouped_torrent_region_priority(region: Option<&str>, custom_order: &[String]) -> i32 {
    let order = effective_download_region_priority(custom_order);
    let Some(region) = region.map(str::trim) else {
        return order.len() as i32;
    };

    if region.is_empty() {
        return order
            .iter()
            .position(|candidate| candidate.is_empty())
            .unwrap_or(order.len()) as i32;
    }

    let region_parts: Vec<String> = region
        .split([',', '/', '&', '+'])
        .map(str::trim)
        .filter(|part| !part.is_empty())
        .map(|part| part.to_ascii_lowercase())
        .collect();

    for (priority, candidate) in order.iter().enumerate() {
        if candidate.is_empty() {
            continue;
        }

        let candidate_lower = candidate.to_ascii_lowercase();
        let matches_candidate = region_parts
            .iter()
            .any(|part| match candidate_lower.as_str() {
                "usa" => matches!(part.as_str(), "usa" | "united states" | "north america"),
                "uk" | "united kingdom" => matches!(part.as_str(), "uk" | "united kingdom"),
                other => part == other,
            });

        if matches_candidate {
            return priority as i32;
        }
    }

    order.len() as i32
}

fn minerva_collection_compatibility_priority(rom: &backend_api::MinervaRom) -> i32 {
    let haystack = format!("{} {}", rom.collection, rom.minerva_platform).to_ascii_lowercase();

    if haystack.contains("headered") && !haystack.contains("headerless") {
        0
    } else if haystack.contains("headerless") {
        2
    } else {
        1
    }
}

fn file_picker_display_name(path: &str) -> String {
    std::path::Path::new(path)
        .file_name()
        .and_then(|name| name.to_str())
        .unwrap_or(path)
        .to_string()
}

fn file_picker_path_detail(path: &str) -> Option<String> {
    let display_name = file_picker_display_name(path);
    if display_name == path {
        None
    } else {
        Some(path.to_string())
    }
}

fn torrent_file_picker_display_name(file: &backend_api::TorrentFileMatch) -> String {
    file.group_display_name
        .clone()
        .unwrap_or_else(|| file_picker_display_name(&file.filename))
}

fn torrent_file_picker_type_badge(file: &backend_api::TorrentFileMatch) -> Option<String> {
    file.group_disc_count.map(|_| "Multi-disc".to_string())
}

fn normalized_listing_path(path: &str) -> String {
    path.trim_start_matches("./")
        .replace('\\', "/")
        .to_ascii_lowercase()
}

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
struct PickerOpticalDiscKey {
    title: String,
    variant_tags: Vec<String>,
}

#[derive(Clone, Debug)]
struct PickerOpticalDiscInfo {
    key: PickerOpticalDiscKey,
    disc_index: u32,
    playlist_stem: String,
}

fn picker_file_extension(path: &str) -> Option<String> {
    std::path::Path::new(path)
        .extension()
        .and_then(|value| value.to_str())
        .map(|value| value.to_ascii_lowercase())
}

fn picker_basename_without_extension(path: &str) -> Option<String> {
    let file_name = std::path::Path::new(path).file_name()?.to_str()?;
    Some(
        file_name
            .rsplit_once('.')
            .map(|(base, _)| base)
            .unwrap_or(file_name)
            .to_string(),
    )
}

fn normalize_picker_disc_key_text(value: &str) -> String {
    value
        .to_ascii_lowercase()
        .chars()
        .map(|ch| if ch.is_ascii_alphanumeric() { ch } else { ' ' })
        .collect::<String>()
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
}

fn parse_picker_title_tags(stem: &str) -> (String, Vec<(String, String)>) {
    let mut tags = Vec::new();
    let mut base = stem.to_string();
    let mut search_start = 0usize;
    while let Some(relative_start) = base[search_start..].find('(') {
        let start = search_start + relative_start;
        let Some(relative_end) = base[start..].find(')') else {
            break;
        };
        let end = start + relative_end + 1;
        let original = base[start..end].to_string();
        let text = base[start + 1..end - 1].to_string();
        tags.push((text, original));
        search_start = end;
    }
    for (_, original) in &tags {
        base = base.replace(original, "");
    }
    (base.trim().to_string(), tags)
}

fn roman_picker_disc_number(value: &str) -> Option<u32> {
    match value.trim().to_ascii_lowercase().as_str() {
        "i" => Some(1),
        "ii" => Some(2),
        "iii" => Some(3),
        "iv" => Some(4),
        "v" => Some(5),
        "vi" => Some(6),
        _ => None,
    }
}

fn parse_picker_disc_index(value: &str) -> Option<u32> {
    let lower = value.trim().to_ascii_lowercase();
    let prefix = [
        "disc", "disk", "cd", "side", "part", "volume", "vol", "card",
    ]
    .iter()
    .find(|prefix| lower.starts_with(**prefix))?;
    let mut rest = lower[prefix.len()..]
        .trim_start_matches(|ch: char| ch.is_whitespace() || matches!(ch, '#' | '-' | '_' | ':'))
        .chars()
        .peekable();
    let mut digits = String::new();
    while let Some(ch) = rest.peek().copied() {
        if ch.is_ascii_digit() {
            digits.push(ch);
            rest.next();
        } else {
            break;
        }
    }
    if !digits.is_empty() {
        return digits.parse::<u32>().ok().filter(|value| *value > 0);
    }

    let token = rest
        .take_while(|ch| ch.is_ascii_alphabetic())
        .collect::<String>();
    if token.len() == 1 {
        let ch = token.as_bytes()[0];
        if ch.is_ascii_alphabetic() {
            return Some((ch.to_ascii_lowercase() - b'a' + 1) as u32);
        }
    }
    roman_picker_disc_number(&token)
}

fn picker_loose_disc_marker(stem: &str) -> Option<(String, u32)> {
    let lower = stem.to_ascii_lowercase();
    for marker in ["disc", "disk", "cd", "side", "part"] {
        let mut search_start = 0usize;
        while let Some(relative_pos) = lower[search_start..].find(marker) {
            let pos = search_start + relative_pos;
            let prev_ok = pos == 0
                || lower[..pos]
                    .chars()
                    .next_back()
                    .is_some_and(|ch| !ch.is_ascii_alphanumeric());
            let marker_end = pos + marker.len();
            let next_ok = lower[marker_end..].chars().next().is_some_and(|ch| {
                ch.is_ascii_digit() || ch.is_whitespace() || matches!(ch, '#' | '-' | '_' | ':')
            });
            if prev_ok && next_ok {
                if let Some(index) = parse_picker_disc_index(&stem[pos..]) {
                    let base = stem[..pos]
                        .trim_end_matches(|ch: char| {
                            ch.is_whitespace() || matches!(ch, '-' | '_' | ',' | ':')
                        })
                        .trim()
                        .to_string();
                    if !base.is_empty() {
                        return Some((base, index));
                    }
                }
            }
            search_start = marker_end;
        }
    }
    None
}

fn picker_optical_disc_info_from_component(stem: &str) -> Option<PickerOpticalDiscInfo> {
    let (base, tags) = parse_picker_title_tags(stem);
    let mut disc_index = None;
    let mut key_tags = Vec::new();
    let mut display_tags = Vec::new();

    for (tag, original) in tags {
        if let Some(index) = parse_picker_disc_index(&tag) {
            disc_index = Some(index);
            continue;
        }
        if tag.trim().to_ascii_lowercase().starts_with("track ") {
            continue;
        }
        let normalized = normalize_picker_disc_key_text(&tag);
        if !normalized.is_empty() {
            key_tags.push(normalized);
            display_tags.push(original);
        }
    }

    let (base, disc_index) = if let Some(index) = disc_index {
        (base, index)
    } else if let Some((loose_base, index)) = picker_loose_disc_marker(stem) {
        (loose_base, index)
    } else {
        return None;
    };

    let title = normalize_picker_disc_key_text(&base);
    if title.is_empty() {
        return None;
    }

    key_tags.sort();
    key_tags.dedup();

    let playlist_stem = if display_tags.is_empty() {
        base.trim().to_string()
    } else {
        format!("{} {}", base.trim(), display_tags.join(" "))
    };

    Some(PickerOpticalDiscInfo {
        key: PickerOpticalDiscKey {
            title,
            variant_tags: key_tags,
        },
        disc_index,
        playlist_stem,
    })
}

fn picker_optical_disc_info_from_path(path: &str) -> Option<PickerOpticalDiscInfo> {
    let normalized = path.trim_start_matches("./").replace('\\', "/");
    let mut components = normalized
        .split('/')
        .filter(|component| !component.is_empty())
        .map(str::to_string)
        .collect::<Vec<_>>();
    if let Some(last) = components.last_mut() {
        if let Some(stem) = picker_basename_without_extension(last) {
            *last = stem;
        }
    }

    components
        .iter()
        .rev()
        .find_map(|component| picker_optical_disc_info_from_component(component))
}

fn is_picker_optical_primary_extension(ext: &str) -> bool {
    matches!(
        ext,
        "cue"
            | "chd"
            | "ccd"
            | "mds"
            | "gdi"
            | "iso"
            | "cso"
            | "pbp"
            | "bin"
            | "img"
            | "zip"
            | "7z"
            | "rar"
    )
}

fn picker_optical_primary_priority(ext: &str) -> u8 {
    match ext {
        "chd" => 0,
        "cue" => 1,
        "ccd" => 2,
        "mds" => 3,
        "gdi" => 4,
        "pbp" => 5,
        "iso" | "cso" => 6,
        "bin" | "img" => 7,
        "zip" | "7z" | "rar" => 8,
        _ => 100,
    }
}

fn build_optical_disc_picker_items(
    rom: &backend_api::MinervaRom,
    files: &[backend_api::TorrentFileMatch],
) -> Option<Vec<MinervaPickerItem>> {
    let mut primary_groups: BTreeMap<
        PickerOpticalDiscKey,
        Vec<(backend_api::TorrentFileMatch, PickerOpticalDiscInfo, String)>,
    > = BTreeMap::new();
    let mut info_by_index = BTreeMap::new();

    for file in files {
        let Some(info) = picker_optical_disc_info_from_path(&file.filename) else {
            continue;
        };
        info_by_index.insert(file.index, info.clone());
        let Some(ext) = picker_file_extension(&file.filename) else {
            continue;
        };
        if is_picker_optical_primary_extension(&ext) {
            primary_groups
                .entry(info.key.clone())
                .or_default()
                .push((file.clone(), info, ext));
        }
    }

    let mut covered_indices = HashSet::new();
    let mut positioned_items = Vec::new();

    for (key, candidates) in primary_groups {
        let distinct_discs = candidates
            .iter()
            .map(|(_, info, _)| info.disc_index)
            .collect::<BTreeSet<_>>();
        if distinct_discs.len() < 2 {
            continue;
        }

        let mut best_by_disc: BTreeMap<
            u32,
            (backend_api::TorrentFileMatch, PickerOpticalDiscInfo, String),
        > = BTreeMap::new();
        for (file, info, ext) in candidates {
            let replace = best_by_disc.get(&info.disc_index).is_none_or(
                |(current_file, _current_info, current_ext)| {
                    picker_optical_primary_priority(&ext)
                        < picker_optical_primary_priority(current_ext)
                        || (picker_optical_primary_priority(&ext)
                            == picker_optical_primary_priority(current_ext)
                            && file.filename < current_file.filename)
                },
            );
            if replace {
                best_by_disc.insert(info.disc_index, (file, info, ext));
            }
        }
        if best_by_disc.len() < 2 {
            continue;
        }

        let playlist_stem = best_by_disc
            .values()
            .next()
            .map(|(_, info, _)| info.playlist_stem.clone())
            .unwrap_or_else(|| "Multi-disc game".to_string());
        let first_file = best_by_disc
            .values()
            .next()
            .map(|(file, _, _)| file.clone())?;
        let mut first_position = usize::MAX;
        let mut total_size = 0_u64;
        let mut match_score = 0.0_f64;
        let mut region = None;
        for (position, file) in files.iter().enumerate() {
            if info_by_index
                .get(&file.index)
                .is_some_and(|info| info.key == key)
            {
                first_position = first_position.min(position);
                covered_indices.insert(file.index);
                total_size = total_size.saturating_add(file.size);
                match_score = match_score.max(file.match_score);
                if region.is_none() {
                    region = file.region.clone();
                }
            }
        }

        positioned_items.push((
            first_position,
            MinervaPickerItem {
                selection: MinervaDownloadSelection::File {
                    torrent_url: rom.torrent_url.clone(),
                    file_index: first_file.index,
                },
                display_name: format!("{} ({} discs)", playlist_stem, best_by_disc.len()),
                path_detail: file_picker_path_detail(&first_file.filename).and_then(|path| {
                    std::path::Path::new(&path)
                        .parent()
                        .map(|p| p.display().to_string())
                }),
                size: total_size,
                match_score,
                region,
                suggested_emulator: None,
                type_badge: Some("Multi-disc".to_string()),
            },
        ));
    }

    if positioned_items.is_empty() {
        return None;
    }

    for (position, file) in files.iter().enumerate() {
        if covered_indices.contains(&file.index) {
            continue;
        }
        positioned_items.push((
            position,
            MinervaPickerItem {
                selection: MinervaDownloadSelection::File {
                    torrent_url: rom.torrent_url.clone(),
                    file_index: file.index,
                },
                display_name: torrent_file_picker_display_name(file),
                path_detail: file_picker_path_detail(&file.filename),
                size: file.size,
                match_score: file.match_score,
                region: file.region.clone(),
                suggested_emulator: None,
                type_badge: torrent_file_picker_type_badge(file),
            },
        ));
    }

    positioned_items.sort_by_key(|(position, _)| *position);
    Some(positioned_items.into_iter().map(|(_, item)| item).collect())
}

fn is_arcade_mame_merged_or_split_rom(rom: &backend_api::MinervaRom) -> bool {
    if !rom.collection.eq_ignore_ascii_case("MAME") {
        return false;
    }

    let platform = rom.minerva_platform.to_ascii_lowercase();
    (platform.contains("merged") || platform.contains("split")) && !platform.contains("non-merged")
}

fn is_laserdisc_collection_rom(rom: &backend_api::MinervaRom) -> bool {
    rom.collection.eq_ignore_ascii_case("Laserdisc Collection")
}

fn parse_mame_laserdisc_bundle(path: &str) -> Option<(String, &'static str)> {
    let normalized = normalized_listing_path(path);
    if let Some(file_name) = normalized
        .strip_prefix("laserdisc collection/mame/roms/")
        .filter(|remainder| !remainder.contains('/'))
    {
        let romset = file_name.strip_suffix(".zip")?;
        if !romset.is_empty() {
            return Some((romset.to_string(), "zip"));
        }
    }

    if let Some(remainder) = normalized.strip_prefix("laserdisc collection/mame/chd/") {
        let mut parts = remainder.split('/');
        let romset = parts.next()?;
        let file_name = parts.next()?;
        if parts.next().is_none() && file_name.ends_with(".chd") {
            return Some((romset.to_string(), "chd"));
        }
    }

    None
}

fn parse_hypseus_laserdisc_bundle(path: &str) -> Option<(String, &'static str)> {
    let normalized = normalized_listing_path(path);
    if !normalized.starts_with("laserdisc collection/") || normalized.contains("/mame/") {
        return None;
    }

    let path = std::path::Path::new(&normalized);
    let ext = path
        .extension()
        .and_then(|value| value.to_str())
        .map(|value| value.to_ascii_lowercase())?;
    let kind = match ext.as_str() {
        "zip" => "zip",
        "dat" => "dat",
        "m2v" => "m2v",
        "ogg" => "ogg",
        "txt" => "txt",
        _ => return None,
    };
    let stem = path.file_stem()?.to_str()?;
    let prefix_path = if kind == "zip" {
        let parent = path.parent()?;
        if parent
            .file_name()
            .and_then(|value| value.to_str())
            .is_some_and(|value| value.eq_ignore_ascii_case("roms"))
        {
            parent.parent()?
        } else {
            parent
        }
    } else {
        let mut parent = path.parent()?;
        if parent
            .file_name()
            .and_then(|value| value.to_str())
            .is_some_and(|value| {
                value.eq_ignore_ascii_case("video")
                    || value.eq_ignore_ascii_case("audio")
                    || value.eq_ignore_ascii_case("sound")
            })
        {
            parent = parent.parent()?;
        }
        parent
    };
    let prefix = prefix_path
        .iter()
        .filter_map(|component| component.to_str())
        .collect::<Vec<_>>()
        .join("/");
    Some((format!("{prefix}/{stem}"), kind))
}

fn laserdisc_bundle_title(bundle_key: &str) -> String {
    std::path::Path::new(bundle_key)
        .file_name()
        .and_then(|value| value.to_str())
        .unwrap_or(bundle_key)
        .to_string()
}

#[derive(Clone)]
struct ParsedDaphneLaserdiscBundle {
    package_root: String,
    game_key: Option<String>,
    stem: String,
    kind: &'static str,
}

fn daphne_bundle_title(package_root: &str, game_key: Option<&str>) -> String {
    let label = std::path::Path::new(package_root)
        .file_name()
        .and_then(|value| value.to_str())
        .unwrap_or(package_root);
    if matches!(
        label.to_ascii_lowercase().as_str(),
        "daphneloader" | "daphne"
    ) {
        if let Some(game_key) = game_key {
            return format!("{label} / {game_key}");
        }
    }
    label.to_string()
}

fn parse_daphne_laserdisc_bundle(path: &str) -> Option<ParsedDaphneLaserdiscBundle> {
    let normalized = normalized_listing_path(path);
    if !normalized.starts_with("laserdisc collection/daphne/") {
        return None;
    }

    let path = std::path::Path::new(&normalized);
    let components = path
        .iter()
        .filter_map(|component| component.to_str())
        .collect::<Vec<_>>();
    let file_name = path.file_name()?.to_str()?.to_string();
    let stem = path.file_stem()?.to_str()?.to_string();

    if let Some(anchor_idx) = components
        .iter()
        .rposition(|component| component.eq_ignore_ascii_case("roms"))
    {
        if anchor_idx + 1 != components.len() - 1 || !file_name.ends_with(".zip") {
            return None;
        }
        let package_root = components[..anchor_idx].join("/");
        if package_root.is_empty() {
            return None;
        }
        return Some(ParsedDaphneLaserdiscBundle {
            package_root,
            game_key: None,
            stem,
            kind: "zip",
        });
    }

    if let Some(anchor_idx) = components
        .iter()
        .rposition(|component| component.eq_ignore_ascii_case("ram"))
    {
        if anchor_idx + 1 != components.len() - 1 || !file_name.ends_with(".gz") {
            return None;
        }
        let package_root = components[..anchor_idx].join("/");
        if package_root.is_empty() {
            return None;
        }
        return Some(ParsedDaphneLaserdiscBundle {
            package_root,
            game_key: None,
            stem,
            kind: "ram",
        });
    }

    let extension = path
        .extension()
        .and_then(|value| value.to_str())
        .map(|value| value.to_ascii_lowercase())?;
    let kind = match extension.as_str() {
        "txt" => "txt",
        "dat" => "dat",
        "m2v" => "m2v",
        "ogg" | "wav" | "mp3" | "flac" => "audio",
        _ => return None,
    };

    for anchor_name in ["mpeg2", "vldp_dl"] {
        let Some(anchor_idx) = components
            .iter()
            .rposition(|component| component.eq_ignore_ascii_case(anchor_name))
        else {
            continue;
        };
        let package_root = components[..anchor_idx].join("/");
        let game_key = components.get(anchor_idx + 1)?.to_string();
        if package_root.is_empty() {
            return None;
        }
        return Some(ParsedDaphneLaserdiscBundle {
            package_root,
            game_key: Some(game_key),
            stem,
            kind,
        });
    }

    None
}

fn daphne_matching_game_key(
    stem: &str,
    available_game_keys: &std::collections::BTreeSet<String>,
) -> Option<String> {
    available_game_keys
        .iter()
        .filter(|game_key| {
            let game_key = game_key.as_str();
            stem == game_key || stem.starts_with(&format!("{game_key}_"))
        })
        .max_by_key(|game_key| game_key.len())
        .cloned()
}

fn build_daphne_laserdisc_bundles(
    files: Vec<backend_api::TorrentFileMatch>,
) -> Vec<(String, String, Vec<backend_api::TorrentFileMatch>)> {
    #[derive(Default)]
    struct PackageState {
        game_keys: std::collections::BTreeSet<String>,
        members: Vec<(backend_api::TorrentFileMatch, ParsedDaphneLaserdiscBundle)>,
    }

    let mut packages: std::collections::BTreeMap<String, PackageState> =
        std::collections::BTreeMap::new();
    for file in files {
        let Some(parsed) = parse_daphne_laserdisc_bundle(&file.filename) else {
            continue;
        };
        let entry = packages.entry(parsed.package_root.clone()).or_default();
        if let Some(game_key) = parsed.game_key.as_ref() {
            entry.game_keys.insert(game_key.clone());
        }
        entry.members.push((file, parsed));
    }

    let mut bundles: std::collections::BTreeMap<
        String,
        (String, Vec<backend_api::TorrentFileMatch>),
    > = std::collections::BTreeMap::new();
    for (package_root, package) in packages {
        for (file, parsed) in package.members {
            let Some(game_key) = parsed
                .game_key
                .clone()
                .or_else(|| daphne_matching_game_key(&parsed.stem, &package.game_keys))
            else {
                continue;
            };
            let bundle_key = format!("{package_root}/{game_key}");
            let bundle_name = daphne_bundle_title(&package_root, Some(&game_key));
            bundles
                .entry(bundle_key)
                .or_insert_with(|| (bundle_name, Vec::new()))
                .1
                .push(file);
        }
    }

    bundles
        .into_iter()
        .map(|(bundle_key, (bundle_name, members))| (bundle_key, bundle_name, members))
        .collect()
}

fn build_minerva_picker_items(
    rom: &backend_api::MinervaRom,
    files: Vec<backend_api::TorrentFileMatch>,
) -> Vec<MinervaPickerItem> {
    if !is_laserdisc_collection_rom(rom) {
        if let Some(items) = build_optical_disc_picker_items(rom, &files) {
            return items;
        }

        return files
            .into_iter()
            .map(|file| MinervaPickerItem {
                selection: MinervaDownloadSelection::File {
                    torrent_url: rom.torrent_url.clone(),
                    file_index: file.index,
                },
                display_name: torrent_file_picker_display_name(&file),
                path_detail: file_picker_path_detail(&file.filename),
                size: file.size,
                match_score: file.match_score,
                region: file.region.clone(),
                suggested_emulator: None,
                type_badge: torrent_file_picker_type_badge(&file),
            })
            .collect();
    }

    if rom.minerva_platform.eq_ignore_ascii_case("MAME") {
        let mut bundles: std::collections::BTreeMap<
            String,
            (
                Option<backend_api::TorrentFileMatch>,
                Option<backend_api::TorrentFileMatch>,
            ),
        > = std::collections::BTreeMap::new();
        for file in files {
            if let Some((romset, kind)) = parse_mame_laserdisc_bundle(&file.filename) {
                let entry = bundles.entry(romset).or_insert((None, None));
                match kind {
                    "zip" => entry.0 = Some(file),
                    "chd" => entry.1 = Some(file),
                    _ => {}
                }
            }
        }

        return bundles
            .into_iter()
            .filter_map(|(romset, (zip, chd))| {
                let zip = zip?;
                Some(if let Some(chd) = chd {
                    MinervaPickerItem {
                        selection: MinervaDownloadSelection::File {
                            torrent_url: rom.torrent_url.clone(),
                            file_index: zip.index,
                        },
                        display_name: format!(
                            "{romset}.zip + {}",
                            file_picker_display_name(&chd.filename)
                        ),
                        path_detail: Some("Laserdisc Collection / MAME".to_string()),
                        size: zip.size.saturating_add(chd.size),
                        match_score: zip.match_score.max(chd.match_score),
                        region: zip.region.or(chd.region),
                        suggested_emulator: Some("MAME".to_string()),
                        type_badge: Some("Bundle".to_string()),
                    }
                } else {
                    MinervaPickerItem {
                        selection: MinervaDownloadSelection::File {
                            torrent_url: rom.torrent_url.clone(),
                            file_index: zip.index,
                        },
                        display_name: file_picker_display_name(&zip.filename),
                        path_detail: Some("Laserdisc Collection / MAME".to_string()),
                        size: zip.size,
                        match_score: zip.match_score,
                        region: zip.region,
                        suggested_emulator: Some("MAME".to_string()),
                        type_badge: Some("ROM".to_string()),
                    }
                })
            })
            .collect();
    }

    if rom.minerva_platform.eq_ignore_ascii_case("Daphne") {
        return build_daphne_laserdisc_bundles(files)
            .into_iter()
            .filter_map(|(_bundle_key, bundle_name, members)| {
                let mut rom_zip = None;
                let mut framefile = None;
                let mut has_video = false;
                let mut total_size = 0_u64;
                let mut match_score = 0.0_f64;
                let mut region = None;

                for member in members {
                    let parsed = parse_daphne_laserdisc_bundle(&member.filename)?;
                    total_size = total_size.saturating_add(member.size);
                    match_score = match_score.max(member.match_score);
                    if region.is_none() {
                        region = member.region.clone();
                    }

                    match parsed.kind {
                        "zip" => rom_zip = Some(member),
                        "txt" => framefile = Some(member),
                        "m2v" => has_video = true,
                        _ => {}
                    }
                }

                let framefile = framefile?;
                let _rom_zip = rom_zip?;
                if !has_video {
                    return None;
                }

                Some(MinervaPickerItem {
                    selection: MinervaDownloadSelection::File {
                        torrent_url: rom.torrent_url.clone(),
                        file_index: framefile.index,
                    },
                    display_name: bundle_name,
                    path_detail: Some("Laserdisc Collection / Daphne".to_string()),
                    size: total_size,
                    match_score,
                    region,
                    suggested_emulator: Some("Hypseus Singe".to_string()),
                    type_badge: Some("Bundle".to_string()),
                })
            })
            .collect();
    }

    let mut bundles: std::collections::BTreeMap<String, Vec<backend_api::TorrentFileMatch>> =
        std::collections::BTreeMap::new();
    for file in files {
        if let Some((bundle_key, _)) = parse_hypseus_laserdisc_bundle(&file.filename) {
            bundles.entry(bundle_key).or_default().push(file);
        }
    }

    bundles
        .into_iter()
        .filter_map(|(bundle_key, members)| {
            let mut rom_zip = None;
            let mut data = None;
            let mut text = None;
            let mut video = None;
            let mut audio = None;
            for member in members {
                match parse_hypseus_laserdisc_bundle(&member.filename)
                    .map(|(_, kind)| kind)
                    .unwrap_or_default()
                {
                    "zip" => rom_zip = Some(member),
                    "dat" => data = Some(member),
                    "txt" => text = Some(member),
                    "m2v" => video = Some(member),
                    "ogg" => audio = Some(member),
                    _ => {}
                }
            }

            let (rom_zip, data, text, video, audio) = (rom_zip?, data?, text?, video?, audio?);
            Some(MinervaPickerItem {
                selection: MinervaDownloadSelection::File {
                    torrent_url: rom.torrent_url.clone(),
                    file_index: text.index,
                },
                display_name: format!(
                    "{}.zip + {}.m2v + {}.ogg + {}.txt",
                    laserdisc_bundle_title(&bundle_key),
                    laserdisc_bundle_title(&bundle_key),
                    laserdisc_bundle_title(&bundle_key),
                    laserdisc_bundle_title(&bundle_key)
                ),
                path_detail: std::path::Path::new(&bundle_key)
                    .parent()
                    .map(|path| path.display().to_string()),
                size: rom_zip
                    .size
                    .saturating_add(data.size)
                    .saturating_add(text.size)
                    .saturating_add(video.size)
                    .saturating_add(audio.size),
                match_score: rom_zip
                    .match_score
                    .max(text.match_score)
                    .max(video.match_score)
                    .max(audio.match_score),
                region: rom_zip
                    .region
                    .or(text.region)
                    .or(video.region)
                    .or(audio.region),
                suggested_emulator: Some("Hypseus Singe".to_string()),
                type_badge: Some("Bundle".to_string()),
            })
        })
        .collect()
}

fn format_picker_bytes(bytes: i64) -> String {
    let bytes = bytes.max(0) as u64;
    if bytes >= 1_000_000_000 {
        format!("{:.1} GB", bytes as f64 / 1_000_000_000.0)
    } else if bytes >= 1_000_000 {
        format!("{:.1} MB", bytes as f64 / 1_000_000.0)
    } else if bytes >= 1_000 {
        format!("{:.1} KB", bytes as f64 / 1_000.0)
    } else {
        format!("{} B", bytes)
    }
}

fn minerva_torrent_groups_request_key(
    launchbox_db_id: i64,
    game_title: &str,
    platform_name: &str,
    platform_id: i64,
) -> String {
    format!("{launchbox_db_id}:{platform_id}:{platform_name}:{game_title}")
}

#[cfg(test)]
mod tests {
    use super::minerva_torrent_groups_request_key;

    #[test]
    fn minerva_request_key_includes_display_platform() {
        let arcade_key =
            minerva_torrent_groups_request_key(12256, "Dragon's Lair II", "Arcade", 15);
        let laserdisc_key =
            minerva_torrent_groups_request_key(12256, "Dragon's Lair II", "Arcade Laserdisc", 15);

        assert_ne!(arcade_key, laserdisc_key);
    }
}

fn is_arcade_family_platform(platform_name: &str) -> bool {
    matches!(
        platform_name,
        "Arcade" | "Arcade Pinball" | "Arcade Laserdisc"
    )
}

fn is_arcade_laserdisc_platform(platform_name: &str) -> bool {
    platform_name == "Arcade Laserdisc"
}

fn filter_emulators_for_game(
    platform_name: &str,
    game_file: Option<&GameFile>,
    mut emulators: Vec<EmulatorWithStatus>,
) -> Vec<EmulatorWithStatus> {
    if !is_arcade_family_platform(platform_name) {
        return emulators;
    }

    let Some(game_file) = game_file else {
        return emulators;
    };

    let normalized_path = game_file.file_path.replace('\\', "/").to_ascii_lowercase();
    let required_emulator = if normalized_path.contains("/laserdisc collection/hypseus singe/")
        && normalized_path.ends_with(".txt")
    {
        Some("Hypseus Singe")
    } else if normalized_path.contains("/laserdisc collection/mame/")
        && normalized_path.ends_with(".zip")
    {
        Some("MAME")
    } else {
        None
    };

    let Some(required_emulator) = required_emulator else {
        return emulators;
    };

    let original_emulators = emulators.clone();
    emulators.retain(|emulator| {
        emulator.name.eq_ignore_ascii_case(required_emulator) && !emulator.is_retroarch_core
    });

    if emulators.is_empty() && !original_emulators.is_empty() {
        // Fail open if the expected runtime is absent from the catalog.
        return original_emulators;
    }

    emulators
}

async fn load_minerva_torrent_groups(
    launchbox_db_id: i64,
    game_title: String,
    platform_name: String,
    platform_id: i64,
    region_priority: Vec<String>,
    mut on_progress: impl FnMut(usize, usize),
) -> Result<Vec<MinervaTorrentGroup>, String> {
    let mut roms = backend_api::search_minerva(
        Some(launchbox_db_id),
        Some(game_title.clone()),
        Some(platform_id),
    )
    .await?;
    if is_arcade_family_platform(&platform_name) {
        roms.retain(|rom| !is_arcade_mame_merged_or_split_rom(rom));
    }
    let total_groups = roms.len();
    if total_groups > 0 {
        on_progress(0, total_groups);
    }
    let mut groups = Vec::new();
    let mut last_error = None;

    let mut group_results = stream::iter(roms.into_iter().map(|rom| {
        let game_title = game_title.clone();
        let platform_name = platform_name.clone();
        async move {
            let result = backend_api::list_torrent_files(
                rom.torrent_url.clone(),
                game_title,
                Some(platform_name),
                Some(launchbox_db_id),
                Some(rom.collection.clone()),
                Some(rom.minerva_platform.clone()),
            )
            .await;
            (rom, result)
        }
    }))
    .buffer_unordered(6);

    let mut completed_groups = 0usize;
    while let Some((rom, result)) = group_results.next().await {
        completed_groups += 1;
        if total_groups > 0 {
            on_progress(completed_groups, total_groups);
        }
        match result {
            Ok(files) => {
                let items = build_minerva_picker_items(&rom, files);
                if !items.is_empty() {
                    groups.push(MinervaTorrentGroup { rom, items });
                }
            }
            Err(err) => last_error = Some(err),
        }
    }

    if is_arcade_laserdisc_platform(&platform_name)
        && groups
            .iter()
            .any(|group| is_laserdisc_collection_rom(&group.rom))
    {
        groups.retain(|group| is_laserdisc_collection_rom(&group.rom));
    }

    groups.sort_by(|a, b| {
        let a_top_score = a.items.first().map(|file| file.match_score).unwrap_or(0.0);
        let b_top_score = b.items.first().map(|file| file.match_score).unwrap_or(0.0);
        minerva_collection_compatibility_priority(&a.rom)
            .cmp(&minerva_collection_compatibility_priority(&b.rom))
            .then_with(|| {
                b.rom
                    .collection
                    .eq_ignore_ascii_case("Laserdisc Collection")
                    .cmp(
                        &a.rom
                            .collection
                            .eq_ignore_ascii_case("Laserdisc Collection"),
                    )
            })
            .then_with(|| {
                b_top_score
                    .partial_cmp(&a_top_score)
                    .unwrap_or(std::cmp::Ordering::Equal)
            })
            .then_with(|| {
                grouped_torrent_region_priority(
                    a.items.first().and_then(|file| file.region.as_deref()),
                    &region_priority,
                )
                .cmp(&grouped_torrent_region_priority(
                    b.items.first().and_then(|file| file.region.as_deref()),
                    &region_priority,
                ))
            })
            .then_with(|| b.rom.rom_count.cmp(&a.rom.rom_count))
            .then_with(|| a.rom.collection.cmp(&b.rom.collection))
    });

    if groups.is_empty() {
        if let Some(err) = last_error {
            Err(err)
        } else {
            Ok(groups)
        }
    } else {
        Ok(groups)
    }
}

const CONTROLLER_PROFILE_INHERIT: &str = "__inherit";
const CONTROLLER_PROFILE_NONE: &str = "__none";
const TWO_BUTTON_CLOCKWISE_PROFILE_ID: &str = "two-button-clockwise";

fn fallback_controller_profiles() -> Vec<ControllerProfileInfo> {
    vec![ControllerProfileInfo {
        id: TWO_BUTTON_CLOCKWISE_PROFILE_ID.to_string(),
        name: "2-button clockwise diamond".to_string(),
        description:
            "Maps physical bottom/right face buttons to target left/bottom for NES-style layouts."
                .to_string(),
    }]
}

fn available_controller_profiles(
    inventory: Option<backend_api::ControllerInventory>,
) -> Vec<ControllerProfileInfo> {
    inventory
        .map(|inventory| inventory.built_in_profiles)
        .filter(|profiles| !profiles.is_empty())
        .unwrap_or_else(fallback_controller_profiles)
}

fn controller_target_options(
    inventory: Option<backend_api::ControllerInventory>,
) -> Vec<(String, String)> {
    let targets = inventory
        .map(|inventory| inventory.supported_targets)
        .unwrap_or_default();
    let filtered = targets
        .into_iter()
        .filter(|target| {
            matches!(
                target.id.as_str(),
                "xb360" | "xbox-series" | "xbox-elite" | "ds5" | "gamepad"
            )
        })
        .map(|target| (target.id, target.name))
        .collect::<Vec<_>>();

    if filtered.is_empty() {
        vec![
            ("xb360".to_string(), "Microsoft X-Box 360 pad".to_string()),
            (
                "xbox-series".to_string(),
                "Microsoft Xbox Series S|X Controller".to_string(),
            ),
            ("ds5".to_string(), "Sony DualSense".to_string()),
            ("gamepad".to_string(), "InputPlumber Gamepad".to_string()),
        ]
    } else {
        filtered
    }
}

fn controller_profile_select_value(map: &HashMap<String, String>, key: &str) -> String {
    match map.get(key).map(|value| value.trim()) {
        Some("") | Some("none") => CONTROLLER_PROFILE_NONE.to_string(),
        Some(profile_id) => profile_id.to_string(),
        None => CONTROLLER_PROFILE_INHERIT.to_string(),
    }
}

fn set_controller_profile_override(
    map: &mut HashMap<String, String>,
    key: String,
    selected_value: String,
) {
    match selected_value.as_str() {
        CONTROLLER_PROFILE_INHERIT => {
            map.remove(&key);
        }
        CONTROLLER_PROFILE_NONE => {
            map.insert(key, "none".to_string());
        }
        _ => {
            map.insert(key, selected_value);
        }
    }
}

fn save_controller_mapping_change(
    settings: RwSignal<Option<backend_api::AppSettings>>,
    set_saving: WriteSignal<bool>,
    set_error: WriteSignal<Option<String>>,
    update: impl FnOnce(&mut backend_api::AppSettings) + 'static,
) {
    let Some(mut next_settings) = settings.get_untracked() else {
        return;
    };

    update(&mut next_settings);
    settings.set(Some(next_settings.clone()));
    set_saving.set(true);
    set_error.set(None);

    spawn_local(async move {
        match backend_api::save_settings(next_settings.clone()).await {
            Ok(()) => {
                settings.set(Some(next_settings));
            }
            Err(e) => {
                set_error.set(Some(e));
            }
        }
        set_saving.set(false);
    });
}

#[component]
fn ControllerProfileDetails(
    game: ReadSignal<Option<Game>>,
    set_show_settings: Option<WriteSignal<bool>>,
) -> impl IntoView {
    let settings = RwSignal::new(None::<backend_api::AppSettings>);
    let inventory = RwSignal::new(None::<backend_api::ControllerInventory>);
    let (loading, set_loading) = signal(false);
    let (saving, set_saving) = signal(false);
    let (error, set_error) = signal::<Option<String>>(None);

    Effect::new(move || {
        if game.get().is_none() {
            return;
        }

        set_loading.set(true);
        set_error.set(None);
        spawn_local(async move {
            match backend_api::get_settings().await {
                Ok(loaded_settings) => settings.set(Some(loaded_settings)),
                Err(e) => set_error.set(Some(format!("Failed to load controller settings: {e}"))),
            }

            match backend_api::list_controllers().await {
                Ok(loaded_inventory) => inventory.set(Some(loaded_inventory)),
                Err(e) => set_error.set(Some(format!("Failed to list controllers: {e}"))),
            }

            set_loading.set(false);
        });
    });

    let system_profile_options = move || available_controller_profiles(inventory.get());
    let game_profile_options = move || available_controller_profiles(inventory.get());
    let target_options = move || controller_target_options(inventory.get());

    let mapping_enabled = move || {
        settings
            .get()
            .map(|settings| settings.controller_mapping.enabled)
            .unwrap_or(false)
    };
    let manage_all = move || {
        settings
            .get()
            .map(|settings| settings.controller_mapping.manage_all)
            .unwrap_or(false)
    };
    let output_target = move || {
        settings
            .get()
            .map(|settings| settings.controller_mapping.output_target)
            .unwrap_or_else(|| "xb360".to_string())
    };
    let system_profile_value = move || {
        let Some(settings) = settings.get() else {
            return CONTROLLER_PROFILE_INHERIT.to_string();
        };
        let Some(current_game) = game.get() else {
            return CONTROLLER_PROFILE_INHERIT.to_string();
        };
        controller_profile_select_value(
            &settings.controller_mapping.platform_profile_ids,
            &current_game.platform,
        )
    };
    let game_profile_value = move || {
        let Some(settings) = settings.get() else {
            return CONTROLLER_PROFILE_INHERIT.to_string();
        };
        let Some(current_game) = game.get() else {
            return CONTROLLER_PROFILE_INHERIT.to_string();
        };
        if current_game.database_id <= 0 {
            return CONTROLLER_PROFILE_INHERIT.to_string();
        }
        controller_profile_select_value(
            &settings.controller_mapping.game_profile_ids,
            &current_game.database_id.to_string(),
        )
    };
    let status_label = move || {
        if loading.get() {
            return "Loading".to_string();
        }
        if saving.get() {
            return "Saving".to_string();
        }
        let Some(settings) = settings.get() else {
            return "Unavailable".to_string();
        };
        if !settings.controller_mapping.enabled {
            return "Disabled".to_string();
        }
        let managed_count = inventory
            .get()
            .map(|inventory| inventory.managed_devices.len())
            .unwrap_or(0);
        format!("Enabled, {managed_count} managed")
    };
    let game_profile_disabled = move || {
        loading.get()
            || saving.get()
            || settings.get().is_none()
            || game
                .get()
                .map(|current_game| current_game.database_id <= 0)
                .unwrap_or(true)
    };
    let controls_disabled = move || loading.get() || saving.get() || settings.get().is_none();
    let settings_button = set_show_settings.map(|setter| {
        view! {
            <button
                class="controller-details-secondary-btn"
                on:click=move |_| setter.set(true)
            >
                "Advanced"
            </button>
        }
        .into_any()
    });

    view! {
        <div class="game-controller-profile">
            <div class="game-controller-profile-header">
                <div>
                    <h2>"Controller Mapping"</h2>
                    <span>{move || status_label()}</span>
                </div>
                {settings_button.unwrap_or_else(|| ().into_any())}
            </div>

            <div class="game-controller-toggles">
                <label>
                    <input
                        type="checkbox"
                        prop:checked=mapping_enabled
                        disabled=controls_disabled
                        on:change=move |ev| {
                            let checked = event_target_checked(&ev);
                            save_controller_mapping_change(
                                settings,
                                set_saving,
                                set_error,
                                move |settings| {
                                    settings.controller_mapping.enabled = checked;
                                    if checked {
                                        settings.controller_mapping.manage_all = true;
                                    }
                                },
                            );
                        }
                    />
                    <span>"Enable launch-time mapping"</span>
                </label>
                <label>
                    <input
                        type="checkbox"
                        prop:checked=manage_all
                        disabled=controls_disabled
                        on:change=move |ev| {
                            let checked = event_target_checked(&ev);
                            save_controller_mapping_change(
                                settings,
                                set_saving,
                                set_error,
                                move |settings| {
                                    settings.controller_mapping.manage_all = checked;
                                },
                            );
                        }
                    />
                    <span>"Manage supported controllers"</span>
                </label>
            </div>

            <div class="game-controller-profile-grid">
                <label class="game-controller-field">
                    <span>{move || {
                        game.get()
                            .map(|current_game| format!("System profile ({})", current_game.platform))
                            .unwrap_or_else(|| "System profile".to_string())
                    }}</span>
                    <select
                        prop:value=system_profile_value
                        disabled=controls_disabled
                        on:change=move |ev| {
                            let Some(current_game) = game.get_untracked() else {
                                return;
                            };
                            let platform = current_game.platform.clone();
                            let selected = event_target_value(&ev);
                            save_controller_mapping_change(
                                settings,
                                set_saving,
                                set_error,
                                move |settings| {
                                    if selected != CONTROLLER_PROFILE_INHERIT
                                        && selected != CONTROLLER_PROFILE_NONE
                                    {
                                        settings.controller_mapping.enabled = true;
                                        settings.controller_mapping.manage_all = true;
                                    }
                                    set_controller_profile_override(
                                        &mut settings.controller_mapping.platform_profile_ids,
                                        platform,
                                        selected,
                                    );
                                },
                            );
                        }
                    >
                        <option value=CONTROLLER_PROFILE_INHERIT>"Use default"</option>
                        <option value=CONTROLLER_PROFILE_NONE>"Off for this system"</option>
                        <For
                            each=system_profile_options
                            key=|profile| profile.id.clone()
                            children=move |profile| view! {
                                <option value=profile.id>{profile.name}</option>
                            }
                        />
                    </select>
                </label>

                <label class="game-controller-field">
                    <span>"Game profile"</span>
                    <select
                        prop:value=game_profile_value
                        disabled=game_profile_disabled
                        on:change=move |ev| {
                            let Some(current_game) = game.get_untracked() else {
                                return;
                            };
                            if current_game.database_id <= 0 {
                                return;
                            }
                            let game_key = current_game.database_id.to_string();
                            let selected = event_target_value(&ev);
                            save_controller_mapping_change(
                                settings,
                                set_saving,
                                set_error,
                                move |settings| {
                                    if selected != CONTROLLER_PROFILE_INHERIT
                                        && selected != CONTROLLER_PROFILE_NONE
                                    {
                                        settings.controller_mapping.enabled = true;
                                        settings.controller_mapping.manage_all = true;
                                    }
                                    set_controller_profile_override(
                                        &mut settings.controller_mapping.game_profile_ids,
                                        game_key,
                                        selected,
                                    );
                                },
                            );
                        }
                    >
                        <option value=CONTROLLER_PROFILE_INHERIT>"Use system/default"</option>
                        <option value=CONTROLLER_PROFILE_NONE>"Off for this game"</option>
                        <For
                            each=game_profile_options
                            key=|profile| profile.id.clone()
                            children=move |profile| view! {
                                <option value=profile.id>{profile.name}</option>
                            }
                        />
                    </select>
                </label>

                <label class="game-controller-field">
                    <span>"Virtual target"</span>
                    <select
                        prop:value=output_target
                        disabled=controls_disabled
                        on:change=move |ev| {
                            let selected = event_target_value(&ev);
                            save_controller_mapping_change(
                                settings,
                                set_saving,
                                set_error,
                                move |settings| {
                                    settings.controller_mapping.output_target = selected;
                                },
                            );
                        }
                    >
                        <For
                            each=target_options
                            key=|(id, _)| id.clone()
                            children=move |(id, name)| view! {
                                <option value=id>{name}</option>
                            }
                        />
                    </select>
                </label>
            </div>

            <Show when=move || error.get().is_some()>
                {move || error.get().map(|error| view! {
                    <div class="game-controller-error">{error}</div>
                })}
            </Show>
        </div>
    }
}

#[component]
pub fn GameDetails(
    game: ReadSignal<Option<Game>>,
    on_close: WriteSignal<Option<Game>>,
    #[prop(optional)] set_show_settings: Option<WriteSignal<bool>>,
) -> impl IntoView {
    // Local display state - allows switching variants without affecting external state
    let (display_game, set_display_game) = signal::<Option<Game>>(None);
    let (play_stats, set_play_stats) = signal::<Option<PlayStats>>(None);
    let (is_fav, set_is_fav) = signal(false);
    let (variants, set_variants) = signal::<Vec<GameVariant>>(Vec::new());
    let (selected_variant, set_selected_variant) = signal::<Option<String>>(None);
    // Track pending variant load separately from selected (to avoid infinite loops)
    let (pending_variant_load, set_pending_variant_load) = signal::<Option<String>>(None);
    // Emulator picker state
    let (show_emulator_picker, set_show_emulator_picker) = signal(false);
    let (emulators, set_emulators) = signal::<Vec<EmulatorWithStatus>>(Vec::new());
    let (emulators_loading, set_emulators_loading) = signal(false);
    // Per-game emulator preference
    let (game_emulator_pref, set_game_emulator_pref) = signal::<Option<String>>(None);
    // Import state
    let (game_file, set_game_file) = signal::<Option<GameFile>>(None);
    let (import_state_loading, set_import_state_loading) = signal(false);
    let (show_import_loading_hint, set_show_import_loading_hint) = signal(false);
    let (import_job_id, set_import_job_id) = signal::<Option<String>>(None);
    let (import_error, set_import_error) = signal::<Option<String>>(None);
    let (game_uninstalling, set_game_uninstalling) = signal(false);
    let (details_ready, set_details_ready) = signal(false);
    let (details_min_delay_elapsed, set_details_min_delay_elapsed) = signal(false);
    let (manual_path, set_manual_path) = signal::<Option<String>>(None);
    let (manual_loading, set_manual_loading) = signal(false);
    let (manual_error, set_manual_error) = signal::<Option<String>>(None);
    // Minerva download state
    let (minerva_rom, set_minerva_rom) = signal::<Option<backend_api::MinervaRom>>(None);
    let (minerva_starting, set_minerva_starting) = signal(false);
    let (minerva_job_id, set_minerva_job_id) = signal::<Option<String>>(None);
    let (minerva_downloads, _) = minerva_downloads_signal();
    // Torrent file picker state
    let (show_file_picker, set_show_file_picker) = signal(false);
    let (torrent_groups, set_torrent_groups) = signal::<Vec<MinervaTorrentGroup>>(Vec::new());
    let (selected_download, set_selected_download) =
        signal::<Option<MinervaDownloadSelection>>(None);
    let (files_loading, set_files_loading) = signal(false);
    let (files_loading_progress, set_files_loading_progress) =
        signal::<Option<(usize, usize)>>(None);
    let (torrent_groups_request_key, set_torrent_groups_request_key) =
        signal::<Option<String>>(None);

    // Initialize display_game from prop when game changes
    Effect::new(move || {
        if let Some(g) = game.get() {
            set_display_game.set(Some(g));
            set_play_stats.set(None);
            set_is_fav.set(false);
            set_variants.set(Vec::new());
            set_selected_variant.set(None);
            set_pending_variant_load.set(None);
            set_game_emulator_pref.set(None);
            set_game_file.set(None);
            set_import_job_id.set(None);
            set_import_state_loading.set(true);
            set_show_import_loading_hint.set(false);
            set_import_error.set(None);
            set_game_uninstalling.set(false);
            set_details_ready.set(false);
            set_details_min_delay_elapsed.set(false);
            set_manual_path.set(None);
            set_manual_loading.set(false);
            set_manual_error.set(None);
            set_minerva_rom.set(None);
            set_minerva_starting.set(false);
            set_minerva_job_id.set(None);
            set_files_loading.set(false);
            set_files_loading_progress.set(None);
            set_torrent_groups.set(Vec::new());
            set_selected_download.set(None);
            set_show_file_picker.set(false);
            set_torrent_groups_request_key.set(None);
        } else {
            set_display_game.set(None);
            set_play_stats.set(None);
            set_is_fav.set(false);
            set_variants.set(Vec::new());
            set_selected_variant.set(None);
            set_pending_variant_load.set(None);
            set_game_emulator_pref.set(None);
            set_game_file.set(None);
            set_import_job_id.set(None);
            set_import_state_loading.set(false);
            set_show_import_loading_hint.set(false);
            set_details_ready.set(false);
            set_details_min_delay_elapsed.set(false);
            set_manual_path.set(None);
            set_manual_loading.set(false);
            set_manual_error.set(None);
            set_minerva_starting.set(false);
            set_files_loading.set(false);
            set_files_loading_progress.set(None);
            set_torrent_groups.set(Vec::new());
            set_selected_download.set(None);
            set_show_file_picker.set(false);
            set_torrent_groups_request_key.set(None);
        }
    });

    Effect::new(move || {
        if let Some(g) = display_game.get() {
            let expected_game_id = g.id.clone();

            set_details_ready.set(false);
            set_details_min_delay_elapsed.set(false);

            spawn_local(async move {
                delay_ms(200).await;
                let still_current = display_game
                    .get_untracked()
                    .as_ref()
                    .map(|current| current.id.as_str() == expected_game_id.as_str())
                    .unwrap_or(false);
                if still_current {
                    set_details_min_delay_elapsed.set(true);
                    set_details_ready.set(true);
                }
            });
        }
    });

    Effect::new(move || {
        let Some(g) = display_game.get() else {
            return;
        };

        let expected_game_id = g.id.clone();
        let manual_title = g.display_title.clone();
        let manual_platform = g.platform.clone();
        let manual_db_id = if g.database_id > 0 {
            Some(g.database_id)
        } else {
            None
        };

        set_manual_path.set(None);
        set_manual_error.set(None);
        set_manual_loading.set(false);

        spawn_local(async move {
            if let Ok(Some(cached)) =
                backend_api::check_cached_manual(manual_title, manual_platform, manual_db_id).await
            {
                let still_current = display_game
                    .get_untracked()
                    .as_ref()
                    .map(|current| current.id.as_str() == expected_game_id.as_str())
                    .unwrap_or(false);
                if still_current {
                    set_manual_path.set(Some(cached.path));
                }
            }
        });
    });

    let request_minerva_torrent_groups =
        move |launchbox_db_id: i64,
              game_title: String,
              platform_name: String,
              platform_id: i64,
              open_picker_immediately: bool,
              surface_errors: bool| {
            let request_key = minerva_torrent_groups_request_key(
                launchbox_db_id,
                &game_title,
                &platform_name,
                platform_id,
            );
            let current_request_key = torrent_groups_request_key.get_untracked();
            let current_groups = torrent_groups.get_untracked();
            let currently_loading = files_loading.get_untracked();

            if current_request_key.as_deref() == Some(request_key.as_str()) {
                if open_picker_immediately {
                    set_show_file_picker.set(true);
                }

                if currently_loading || !current_groups.is_empty() {
                    return;
                }
            }

            set_torrent_groups_request_key.set(Some(request_key.clone()));
            set_files_loading.set(true);
            set_files_loading_progress.set(None);
            set_selected_download.set(None);

            if open_picker_immediately {
                set_show_file_picker.set(true);
                set_import_error.set(None);
            }

            let region_priority = effective_download_region_priority(&[]);
            spawn_local(async move {
                let result = load_minerva_torrent_groups(
                    launchbox_db_id,
                    game_title,
                    platform_name,
                    platform_id,
                    region_priority,
                    {
                        let request_key = request_key.clone();
                        move |completed, total| {
                            if torrent_groups_request_key.get_untracked().as_deref()
                                == Some(request_key.as_str())
                            {
                                set_files_loading_progress.set(Some((completed, total)));
                            }
                        }
                    },
                )
                .await;

                if torrent_groups_request_key.get_untracked().as_deref()
                    != Some(request_key.as_str())
                {
                    return;
                }

                match result {
                    Ok(groups) if groups.is_empty() => {
                        set_torrent_groups.set(Vec::new());
                        if surface_errors {
                            set_import_error.set(Some(
                                "No matching ROM files were found in Minerva torrents for this platform."
                                    .to_string(),
                            ));
                            set_show_file_picker.set(false);
                        }
                    }
                    Ok(groups) => {
                        set_torrent_groups.set(groups);
                    }
                    Err(e) => {
                        set_torrent_groups.set(Vec::new());
                        set_torrent_groups_request_key.set(None);
                        set_files_loading_progress.set(None);
                        if surface_errors {
                            set_import_error.set(Some(e));
                            set_show_file_picker.set(false);
                        }
                    }
                }

                set_files_loading.set(false);
                set_files_loading_progress.set(None);
            });
        };

    // Load per-game emulator preference
    Effect::new(move || {
        if let Some(g) = display_game.get() {
            let db_id = g.database_id;
            spawn_local(async move {
                // Get game-specific preference (not platform default)
                if let Ok(prefs) = backend_api::get_all_emulator_preferences().await {
                    let game_pref = prefs
                        .game_preferences
                        .into_iter()
                        .find(|p| p.launchbox_db_id == db_id)
                        .map(|p| p.emulator_name);
                    set_game_emulator_pref.set(game_pref);
                }
            });
        }
    });

    // Avoid flashing the import check hint for fast resolves.
    Effect::new(move || {
        if import_state_loading.get() {
            set_show_import_loading_hint.set(false);
            spawn_local(async move {
                wasm_bindgen_futures::JsFuture::from(js_sys::Promise::new(&mut |resolve, _| {
                    web_sys::window()
                        .unwrap()
                        .set_timeout_with_callback_and_timeout_and_arguments_0(&resolve, 250)
                        .unwrap();
                }))
                .await
                .unwrap();

                if import_state_loading.get_untracked() {
                    set_show_import_loading_hint.set(true);
                }
            });
        } else {
            set_show_import_loading_hint.set(false);
        }
    });

    // Load play stats, favorite status, and variants when display_game changes
    Effect::new(move || {
        if let Some(g) = display_game.get() {
            set_import_state_loading.set(true);
            let game_snapshot = g.clone();
            let game_id = g.id.clone();
            let expected_game_id = game_id.clone();

            // Check if we're switching variants of the same game (variants already loaded)
            // by checking if current game is in the existing variants list
            // Important: do not track `variants` reactively inside this effect.
            // This effect should follow `display_game` only; tracking variants here
            // can create a feedback loop (`set_variants` -> rerun effect -> more async loads),
            // which manifests as panel jitter/shaking.
            let current_variants = variants.get_untracked();
            let is_variant_switch = current_variants.iter().any(|v| v.id == game_id);

            spawn_local(async move {
                let is_current_game = || {
                    display_game
                        .get_untracked()
                        .as_ref()
                        .map(|current| current.id.as_str() == expected_game_id.as_str())
                        .unwrap_or(false)
                };

                let resolved_game = resolve_display_game_identity(&game_snapshot).await;
                if !is_current_game() {
                    return;
                }
                if resolved_game.database_id > 0
                    && resolved_game.database_id != game_snapshot.database_id
                {
                    set_display_game.set(Some(resolved_game.clone()));
                }

                let game_snapshot = resolved_game;
                let db_id = game_snapshot.database_id;
                let display_title = game_snapshot.display_title.clone();
                let platform_id = game_snapshot.platform_id;
                let variant_count = game_snapshot.variant_count;

                // Load play stats
                if let Ok(stats) = backend_api::get_play_stats(db_id).await {
                    if !is_current_game() {
                        return;
                    }
                    set_play_stats.set(stats);
                }
                // Check favorite status
                if let Ok(fav) = backend_api::is_favorite(db_id).await {
                    if !is_current_game() {
                        return;
                    }
                    set_is_fav.set(fav);
                }

                // Only load variants if this is a new game, not a variant switch
                web_sys::console::log_1(
                    &format!(
                        "Loading variants: is_variant_switch={}, variant_count={}",
                        is_variant_switch, variant_count
                    )
                    .into(),
                );
                if !is_variant_switch && variant_count > 1 {
                    web_sys::console::log_1(
                        &format!("Fetching variants for game_id={}", game_id).into(),
                    );
                    match backend_api::get_game_variants(
                        game_id.clone(),
                        display_title.clone(),
                        platform_id,
                    )
                    .await
                    {
                        Ok(vars) => {
                            if !is_current_game() {
                                return;
                            }
                            web_sys::console::log_1(&format!("Got {} variants", vars.len()).into());
                            set_variants.set(vars);

                            if !is_current_game() {
                                return;
                            }
                            set_selected_variant.set(Some(game_id.clone()));

                            // Only swap away from the clicked row when it has no usable DB id.
                            if db_id <= 0 {
                                let fallback_variant_id =
                                    variants.get_untracked().iter().find_map(|variant| {
                                        if variant.id != game_id {
                                            Some(variant.id.clone())
                                        } else {
                                            None
                                        }
                                    });

                                if let Some(preferred_id) = fallback_variant_id {
                                    match backend_api::get_game_by_uuid(preferred_id.clone()).await
                                    {
                                        Ok(Some(preferred_game))
                                            if preferred_game.database_id > 0 =>
                                        {
                                            if !is_current_game() {
                                                return;
                                            }
                                            set_selected_variant.set(Some(preferred_id.clone()));
                                            set_pending_variant_load.set(Some(preferred_id));
                                        }
                                        _ => {}
                                    }
                                }
                            }
                        }
                        Err(_) => {
                            if !is_current_game() {
                                return;
                            }
                            set_variants.set(Vec::new());
                        }
                    }
                } else if !is_variant_switch {
                    if !is_current_game() {
                        return;
                    }
                    set_variants.set(Vec::new());
                }

                // Check game file (with variant-aware fallback) and active import
                let resolved_file = resolve_game_file_for_display(&game_snapshot).await;
                if !is_current_game() {
                    return;
                }
                set_game_file.set(resolved_file);
                match backend_api::get_active_import(db_id).await {
                    Ok(Some(job)) => {
                        if !is_current_game() {
                            return;
                        }
                        set_import_job_id.set(Some(job.id.clone()));
                        set_minerva_job_id.set(Some(job.id));
                        request_minerva_download_queue_refresh();
                    }
                    _ => {
                        if !is_current_game() {
                            return;
                        }
                        set_import_job_id.set(None);
                    }
                }
                // Check minerva ROM availability
                {
                    if let Ok(rom) =
                        backend_api::get_minerva_rom_for_game(db_id, Some(platform_id)).await
                    {
                        if is_current_game() {
                            set_minerva_rom.set(rom);
                        }
                    }
                }

                if is_current_game() {
                    set_import_state_loading.set(false);
                }
            });
        }
    });

    let current_minerva_download = Memo::new(move |_| {
        let Some(job_id) = minerva_job_id.get() else {
            return None;
        };
        minerva_downloads
            .get()
            .into_iter()
            .find(|item| item.job_id == job_id)
    });

    Effect::new(move || {
        if minerva_job_id.get().is_some()
            && !minerva_starting.get()
            && current_minerva_download.get().is_none()
        {
            set_minerva_job_id.set(None);
            set_import_job_id.set(None);
            spawn_local(async move {
                refresh_display_game_file_state(display_game, set_display_game, set_game_file, 4)
                    .await;
            });
        }
    });

    Effect::new(move || {
        let Some(download) = current_minerva_download.get() else {
            return;
        };

        match download.status.as_str() {
            "completed" => {
                set_minerva_starting.set(false);
                set_minerva_job_id.set(None);
                set_import_job_id.set(None);
                spawn_local(async move {
                    refresh_display_game_file_state(
                        display_game,
                        set_display_game,
                        set_game_file,
                        6,
                    )
                    .await;
                });
            }
            "failed" | "cancelled" => {
                set_minerva_starting.set(false);
                set_minerva_job_id.set(None);
                set_import_job_id.set(None);
                set_import_error.set(Some(download.status_message));
            }
            _ => {
                set_minerva_starting.set(false);
            }
        }
    });

    // Preload Minerva torrent matches when the details panel resolves a Minerva source.
    Effect::new(move || {
        let Some(current_game) = display_game.get() else {
            return;
        };

        if minerva_rom.get().is_none() {
            return;
        }

        let request_key = minerva_torrent_groups_request_key(
            current_game.database_id,
            &current_game.display_title,
            &current_game.platform,
            current_game.platform_id,
        );
        if torrent_groups_request_key.get_untracked().as_deref() == Some(request_key.as_str()) {
            return;
        }

        request_minerva_torrent_groups(
            current_game.database_id,
            current_game.display_title.clone(),
            current_game.platform.clone(),
            current_game.platform_id,
            false,
            false,
        );
    });

    // Load variant game when pending_variant_load changes
    // Use untrack to avoid re-triggering when we clear the signal
    Effect::new(move || {
        let variant_id = pending_variant_load.get();
        if let Some(variant_id) = variant_id {
            let still_visible = variants.get_untracked().iter().any(|v| v.id == variant_id);
            if !still_visible {
                set_pending_variant_load.set(None);
                return;
            }
            // Update selected_variant to show visual selection
            set_selected_variant.set(Some(variant_id.clone()));
            spawn_local(async move {
                let requested_variant_id = variant_id.clone();
                if let Ok(Some(new_game)) =
                    backend_api::get_game_by_uuid(requested_variant_id.clone()).await
                {
                    if pending_variant_load.get_untracked().as_deref()
                        == Some(requested_variant_id.as_str())
                    {
                        set_display_game.set(Some(new_game));
                    }
                }
                // Clear after loading completes to prevent re-triggering during load.
                if pending_variant_load.get_untracked().as_deref()
                    == Some(requested_variant_id.as_str())
                {
                    set_pending_variant_load.set(None);
                }
            });
        }
    });

    view! {
        <Show when=move || display_game.get().is_some()>
            {move || {
                display_game.get().map(|g| {
                    let show_details_panel =
                        details_min_delay_elapsed.get() && details_ready.get();

                    if !show_details_panel {
                        return view! {
                            <div
                                class="game-details-overlay game-details-overlay-pending"
                                on:click=move |_| on_close.set(None)
                            ></div>
                        }
                        .into_any();
                    }

                    let display_title = g.display_title.clone();
                    let first_char = display_title.chars().next().unwrap_or('?').to_string();
                    let platform = g.platform.clone();
                    let description = g.description.clone().unwrap_or_else(|| "No description available.".to_string());
                    let developer = g.developer.clone();
                    let publisher = g.publisher.clone();
                    let genres = g.genres.clone();
                    let year = g.release_year;
                    let release_date = g.release_date.clone();
                    let rating = g.rating;
                    let rating_count = g.rating_count;
                    let players = g.players.clone();
                    let esrb = g.esrb.clone();
                    let cooperative = g.cooperative;
                    let video_url = g.video_url.clone();
                    let wikipedia_url = g.wikipedia_url.clone();
                    let db_id = g.database_id;

                    let title_for_fav = g.title.clone();
                    let platform_for_fav = g.platform.clone();
                    let title_for_select = g.title.clone();
                    let platform_for_select = g.platform.clone();

                    // Store game info for emulator selection
                    let stored_title = StoredValue::new(title_for_select);
                    let stored_platform = StoredValue::new(platform_for_select);
                    let stored_db_id = StoredValue::new(db_id);


                    let on_toggle_favorite = move |_| {
                        let title = title_for_fav.clone();
                        let platform = platform_for_fav.clone();
                        let currently_fav = is_fav.get();
                        spawn_local(async move {
                            if currently_fav {
                                if backend_api::remove_favorite(db_id).await.is_ok() {
                                    set_is_fav.set(false);
                                }
                            } else {
                                if backend_api::add_favorite(db_id, title, platform).await.is_ok() {
                                    set_is_fav.set(true);
                                }
                            }
                        });
                    };

                    view! {
                        <div
                            class="game-details-overlay"
                            data-nav-scope="game-details"
                            data-nav-scope-active="true"
                            data-nav-scope-priority="100"
                            on:click=move |_| on_close.set(None)
                        >
                            <div class="game-details-panel" on:click=|e| e.stop_propagation()>
                                // Title bar with game name and close button
                                <div class="game-details-titlebar">
                                    <h1 class="titlebar-title">{display_title.clone()}</h1>
                                    <button
                                        class="titlebar-close"
                                        data-nav-back="true"
                                        on:click=move |_| on_close.set(None)
                                    >
                                        "×"
                                    </button>
                                </div>

                                // Info area on its own row
                                <div class="game-details-info">
                                    <p class="game-details-platform">{platform}</p>

                                    <div class="game-details-meta">
                                            {developer.map(|d| view! {
                                                <div class="meta-item">
                                                    <span class="meta-label">"Developer"</span>
                                                    <span class="meta-value">{d}</span>
                                                </div>
                                            })}
                                            {publisher.map(|p| view! {
                                                <div class="meta-item">
                                                    <span class="meta-label">"Publisher"</span>
                                                    <span class="meta-value">{p}</span>
                                                </div>
                                            })}
                                            {year.map(|y| view! {
                                                <div class="meta-item">
                                                    <span class="meta-label">"Year"</span>
                                                    <span class="meta-value">{y}</span>
                                                </div>
                                            })}
                                            {release_date.map(|d| {
                                                let formatted = format_date(&d);
                                                view! {
                                                    <div class="meta-item">
                                                        <span class="meta-label">"Release Date"</span>
                                                        <span class="meta-value">{formatted}</span>
                                                    </div>
                                                }
                                            })}
                                            {genres.map(|g| view! {
                                                <div class="meta-item">
                                                    <span class="meta-label">"Genre"</span>
                                                    <span class="meta-value">{g}</span>
                                                </div>
                                            })}
                                            {players.map(|p| view! {
                                                <div class="meta-item">
                                                    <span class="meta-label">"Players"</span>
                                                    <span class="meta-value">{p}</span>
                                                </div>
                                            })}
                                            {esrb.map(|e| view! {
                                                <div class="meta-item">
                                                    <span class="meta-label">"ESRB"</span>
                                                    <span class="meta-value">{e}</span>
                                                </div>
                                            })}
                                            {cooperative.map(|c| view! {
                                                <div class="meta-item">
                                                    <span class="meta-label">"Co-op"</span>
                                                    <span class="meta-value">{if c { "Yes" } else { "No" }}</span>
                                                </div>
                                            })}
                                            {rating.map(|r| {
                                                let rating_str = format!("{:.1}", r);
                                                let count_str = rating_count.map(|c| format!(" ({} votes)", c)).unwrap_or_default();
                                                view! {
                                                    <div class="meta-item">
                                                        <span class="meta-label">"Rating"</span>
                                                        <span class="meta-value">{rating_str}{count_str}</span>
                                                    </div>
                                                }
                                            })}
                                        </div>
                                        // External links
                                        {(video_url.is_some() || wikipedia_url.is_some()).then(|| {
                                            let video = video_url.clone();
                                            let wiki = wikipedia_url.clone();
                                            view! {
                                                <div class="game-links">
                                                    {video.map(|url| view! {
                                                        <a href=url target="_blank" class="game-link">"Video"</a>
                                                    })}
                                                    {wiki.map(|url| view! {
                                                        <a href=url target="_blank" class="game-link">"Wikipedia"</a>
                                                    })}
                                                </div>
                                            }
                                        })}

                                        // Play statistics
                                        <Show when=move || play_stats.get().is_some()>
                                            {move || play_stats.get().map(|stats| {
                                                let play_count = stats.play_count;
                                                let last_played = stats.last_played
                                                    .map(|s| format_date(&s))
                                                    .unwrap_or_else(|| "Never".to_string());
                                                view! {
                                                    <div class="play-stats">
                                                        <span class="play-stat">
                                                            <span class="stat-value">{play_count}</span>
                                                            " plays"
                                                        </span>
                                                        <span class="play-stat">
                                                            "Last: "
                                                            <span class="stat-value">{last_played}</span>
                                                        </span>
                                                    </div>
                                                }
                                            })}
                                        </Show>

                                        // Per-game emulator preference
                                        <Show when=move || game_emulator_pref.get().is_some()>
                                            {move || game_emulator_pref.get().map(|emu_name| {
                                                view! {
                                                    <div class="game-emulator-pref">
                                                        <span class="pref-label">"Preferred emulator: "</span>
                                                        <span class="pref-value">{emu_name}</span>
                                                        <button
                                                            class="pref-reset-btn"
                                                            on:click=move |_| {
                                                                spawn_local(async move {
                                                                    if backend_api::clear_game_emulator_preference(db_id).await.is_ok() {
                                                                        set_game_emulator_pref.set(None);
                                                                    }
                                                                });
                                                            }
                                                            title="Reset to ask every time"
                                                        >
                                                            "Reset"
                                                        </button>
                                                    </div>
                                                }
                                            })}
                                        </Show>

                                        <ControllerProfileDetails
                                            game=display_game
                                            set_show_settings=set_show_settings
                                        />

                                        <div class="game-actions">

                                            // Minerva torrent download progress
                                            <Show when=move || {
                                                minerva_starting.get()
                                                    || current_minerva_download.get().is_some()
                                            }>
                                                <div class="import-section">
                                                    <div class="minerva-progress">
                                                        <div class="minerva-progress-bar">
                                                            <div class="minerva-progress-fill" style=move || {
                                                                let pct = current_minerva_download
                                                                    .get()
                                                                    .map(|download| download.progress_percent)
                                                                    .unwrap_or(0.0);
                                                                format!("width: {:.1}%", pct)
                                                            }></div>
                                                        </div>
                                                        <div class="minerva-progress-text">
                                                            {move || {
                                                                if let Some(download) = current_minerva_download.get() {
                                                                    let totals = if download.total_bytes > 0 {
                                                                        format!(
                                                                            " • {} / {}",
                                                                            format_bytes(download.downloaded_bytes),
                                                                            format_bytes(download.total_bytes)
                                                                        )
                                                                    } else {
                                                                        String::new()
                                                                    };
                                                                    let speed = if download.download_speed > 0 {
                                                                        format!(" • {}", format_speed(download.download_speed))
                                                                    } else {
                                                                        String::new()
                                                                    };
                                                                    format!("{}{}{}", download.status_message, totals, speed)
                                                                } else {
                                                                    "Starting download...".to_string()
                                                                }
                                                            }}
                                                        </div>
                                                    </div>
                                                    <div class="minerva-inline-actions">
                                                        {move || current_minerva_download.get().map(|download| {
                                                            let job_id = download.job_id.clone();
                                                            let status = download.status.clone();
                                                            let pause_job_id = job_id.clone();
                                                            let delete_job_id = job_id.clone();
                                                            view! {
                                                                <>
                                                                    {(status == "paused" || status == "downloading" || status == "fetching_torrent" || status == "extracting").then(|| view! {
                                                                        <button
                                                                            class="cancel-import-btn"
                                                                            on:click=move |_| {
                                                                                let job_id = pause_job_id.clone();
                                                                                let status = status.clone();
                                                                                spawn_local(async move {
                                                                                    let _ = if status == "paused" {
                                                                                        backend_api::resume_minerva_download(job_id.clone()).await
                                                                                    } else {
                                                                                        backend_api::pause_minerva_download(job_id.clone()).await
                                                                                    };
                                                                                    request_minerva_download_queue_refresh();
                                                                                    refresh_minerva_download_queue_now().await;
                                                                                });
                                                                            }
                                                                        >
                                                                            {if status == "paused" { "Resume" } else { "Pause" }}
                                                                        </button>
                                                                    })}
                                                                    <button
                                                                        class="cancel-import-btn"
                                                                        on:click=move |_| {
                                                                            let job_id = delete_job_id.clone();
                                                                            set_minerva_starting.set(false);
                                                                            set_minerva_job_id.set(None);
                                                                            set_import_job_id.set(None);
                                                                            spawn_local(async move {
                                                                                let _ = backend_api::delete_minerva_download(job_id).await;
                                                                                request_minerva_download_queue_refresh();
                                                                                refresh_minerva_download_queue_now().await;
                                                                            });
                                                                        }
                                                                    >
                                                                        "Delete"
                                                                    </button>
                                                                </>
                                                            }
                                                        })}
                                                    </div>
                                                </div>
                                            </Show>

                                            // Import prompt and buttons when no file and not importing
                                            <Show when=move || {
                                                import_state_loading.get()
                                                    && show_import_loading_hint.get()
                                                    && import_job_id.get().is_none()
                                                    && game_file.get().is_none()
                                                    && !display_game.get().map(|g| g.has_game_file).unwrap_or(false)
                                            }>
                                                <div class="import-status-hint">"Checking imported file…"</div>
                                            </Show>

                                            <Show when=move || {
                                                !import_state_loading.get()
                                                    && game_file.get().is_none()
                                                    && !display_game.get().map(|g| g.has_game_file).unwrap_or(false)
                                                    && import_job_id.get().is_none()
                                                    && !minerva_starting.get()
                                                    && current_minerva_download.get().is_none()
                                            }>
                                                // Download button (Minerva torrent) — opens file picker
                                                <Show when=move || !show_file_picker.get()>
                                                    <button
                                                        class="import-btn-action minerva-download-btn"
                                                        data-nav-default="true"
                                                        disabled=move || minerva_rom.get().is_none()
                                                        title=move || if minerva_rom.get().is_none() { "No minerva.db — run lunchbox-cli minerva-build first".to_string() } else { "Download ROM via torrent".to_string() }
                                                        on:click=move |_| {
                                                            if minerva_rom.get().is_some() {
                                                                if let Some(g) = display_game.get_untracked() {
                                                                    request_minerva_torrent_groups(
                                                                        g.database_id,
                                                                        g.display_title.clone(),
                                                                        g.platform.clone(),
                                                                        g.platform_id,
                                                                        true,
                                                                        true,
                                                                    );
                                                                }
                                                            }
                                                        }
                                                    >
                                                        "Download"
                                                    </button>
                                                    <button
                                                        class="import-btn-action"
                                                        title="Import a local ROM file by path"
                                                        on:click=move |_| {
                                                            if let Some(g) = display_game.get_untracked() {
                                                                let db_id = g.database_id;
                                                                let title = g.display_title.clone();
                                                                let platform = stored_platform.get_value();
                                                                // Prompt for file path in the browser/Electron shell.
                                                                let window = web_sys::window().unwrap();
                                                                if let Some(path) = window.prompt_with_message("Enter path to ROM file:").ok().flatten() {
                                                                    if !path.trim().is_empty() {
                                                                        let path = path.trim().to_string();
                                                                        spawn_local(async move {
                                                                            let entries = vec![backend_api::RomImportEntry {
                                                                                file_path: path,
                                                                                launchbox_db_id: db_id,
                                                                                game_title: title,
                                                                                platform,
                                                                                copy_to_library: false,
                                                                            }];
                                                                            match backend_api::confirm_rom_import(entries).await {
                                                                                Ok(count) => {
                                                                                    if count > 0 {
                                                                                        refresh_display_game_file_state(
                                                                                            display_game,
                                                                                            set_display_game,
                                                                                            set_game_file,
                                                                                            4,
                                                                                        )
                                                                                        .await;
                                                                                    }
                                                                                }
                                                                                Err(e) => set_import_error.set(Some(e)),
                                                                            }
                                                                        });
                                                                    }
                                                                }
                                                            }
                                                        }
                                                    >"Import"</button>
                                                </Show>

                                            </Show>

                                            // File picker dialog
                                            <Show when=move || show_file_picker.get()>
                                                <div
                                                    class="file-picker-dialog"
                                                    data-nav-scope="minerva-picker"
                                                    data-nav-scope-active="true"
                                                    data-nav-scope-priority="120"
                                                >
                                                    <div class="file-picker-header">
                                                        <h4>"Select Minerva download"</h4>
                                                        <button
                                                            class="file-picker-close"
                                                            data-nav-back="true"
                                                            on:click=move |_| set_show_file_picker.set(false)
                                                        >
                                                            "X"
                                                        </button>
                                                    </div>
                                                    <div class="import-status-hint">
                                                        "Each Minerva torrent shows a highlighted whole-torrent row first, followed by matching game files. Whole-torrent rows download the full set to your torrent library; file rows only download the selected game."
                                                    </div>
                                                    <div class="file-picker-list">
                                                        <Show when=move || files_loading.get() && torrent_groups.get().is_empty()>
                                                            <div class="file-picker-loading">
                                                                <div class="import-status-hint">
                                                                    {move || {
                                                                        files_loading_progress
                                                                            .get()
                                                                            .map(|(completed, total)| {
                                                                                let percent =
                                                                                    if total == 0 {
                                                                                        0
                                                                                    } else {
                                                                                        ((completed as f64 / total as f64)
                                                                                            * 100.0)
                                                                                            .round() as i32
                                                                                    };
                                                                                format!(
                                                                                    "Loading Minerva download options... {percent}% ({completed}/{total})"
                                                                                )
                                                                            })
                                                                            .unwrap_or_else(|| {
                                                                                "Loading Minerva download options...".to_string()
                                                                            })
                                                                    }}
                                                                </div>
                                                                <div class="download-progress">
                                                                    <div
                                                                        class="progress-bar"
                                                                        class:indeterminate=move || files_loading_progress.get().is_none()
                                                                        style:width=move || {
                                                                            files_loading_progress
                                                                                .get()
                                                                                .map(|(completed, total)| {
                                                                                    let percent = if total == 0 {
                                                                                        0.0
                                                                                    } else {
                                                                                        (completed as f64 / total as f64) * 100.0
                                                                                    };
                                                                                    format!("{percent:.1}%")
                                                                                })
                                                                                .unwrap_or_else(|| "100%".to_string())
                                                                        }
                                                                    ></div>
                                                                </div>
                                                            </div>
                                                        </Show>
                                                        {move || {
                                                            let groups = torrent_groups.get();
                                                            groups.iter().enumerate().map(|(group_index, group)| {
                                                            let group = group.clone();
                                                            let torrent_url = group.rom.torrent_url.clone();
                                                            let collection = group.rom.collection.clone();
                                                            let minerva_platform = group.rom.minerva_platform.clone();
                                                            let rom_count = group.rom.rom_count;
                                                            let total_size = group.rom.total_size;
                                                            let match_count = group.items.len();
                                                            let show_whole_torrent_row = !groups[..group_index]
                                                                .iter()
                                                                .any(|other| other.rom.torrent_url == torrent_url);
                                                            let shared_whole_torrent = groups
                                                                .iter()
                                                                .filter(|other| other.rom.torrent_url == torrent_url)
                                                                .count() > 1;
                                                            let whole_torrent_selection = MinervaDownloadSelection::WholeTorrent {
                                                                torrent_url: torrent_url.clone(),
                                                                representative_file_index: group.items.first().map(|item| match item.selection {
                                                                    MinervaDownloadSelection::File { file_index, .. } => file_index,
                                                                    MinervaDownloadSelection::WholeTorrent { representative_file_index, .. } => representative_file_index.unwrap_or(0),
                                                                }),
                                                            };
                                                            let whole_torrent_selected = {
                                                                let whole_torrent_selection = whole_torrent_selection.clone();
                                                                move || selected_download.get() == Some(whole_torrent_selection.clone())
                                                            };
                                                            let whole_torrent_click = whole_torrent_selection.clone();

                                                            view! {
                                                                <div class="file-picker-group">
                                                                    <div class="file-picker-group-label">
                                                                        {format!("{collection} / {minerva_platform}")}
                                                                    </div>
                                                                    {show_whole_torrent_row.then(|| {
                                                                        let row_ref = NodeRef::<leptos::html::Div>::new();
                                                                        view! {
                                                                            <div
                                                                                class="file-picker-row file-picker-row-torrent"
                                                                                node_ref=row_ref
                                                                                tabindex="0"
                                                                                role="button"
                                                                                data-nav="true"
                                                                                data-nav-default=if group_index == 0 { Some("true") } else { None }
                                                                                class:selected=whole_torrent_selected
                                                                                on:click=move |_| {
                                                                                    if let Some(row) = row_ref.get() {
                                                                                        let _ = row.focus();
                                                                                    }
                                                                                    set_selected_download.set(Some(whole_torrent_click.clone()))
                                                                                }
                                                                            >
                                                                                <div class="file-picker-name">
                                                                                    <span class="file-picker-type-badge">"Full Torrent"</span>
                                                                                    <span class="file-picker-type-title">
                                                                                        {if shared_whole_torrent {
                                                                                            "Download Shared Full Torrent"
                                                                                        } else {
                                                                                            "Download Full Torrent"
                                                                                        }}
                                                                                    </span>
                                                                                </div>
                                                                                <div class="file-picker-meta">
                                                                                    <span class="file-picker-size">{format!("{rom_count} ROMs total")}</span>
                                                                                    <span class="file-picker-size">{format_picker_bytes(total_size)}</span>
                                                                                    <span class="file-picker-match">
                                                                                        {if shared_whole_torrent {
                                                                                            format!("{match_count} matching file(s) in this view")
                                                                                        } else {
                                                                                            format!("{match_count} matching file(s)")
                                                                                        }}
                                                                                    </span>
                                                                                </div>
                                                                            </div>
                                                                        }
                                                                    })}
                                                                    {group.items.into_iter().map(|item| {
                                                                        let size_mb = item.size as f64 / (1024.0 * 1024.0);
                                                                        let score = item.match_score;
                                                                        let region = item.region.clone().unwrap_or_default();
                                                                        let selection = item.selection.clone();
                                                                        let is_selected = {
                                                                            let selection = selection.clone();
                                                                            move || selected_download.get() == Some(selection.clone())
                                                                        };
                                                                        let click_selection = selection.clone();
                                                                        let row_ref = NodeRef::<leptos::html::Div>::new();
                                                                        view! {
                                                                            <div
                                                                                class="file-picker-row file-picker-row-file"
                                                                                node_ref=row_ref
                                                                                tabindex="0"
                                                                                role="button"
                                                                                data-nav="true"
                                                                                data-nav-default=if group_index == 0 && !show_whole_torrent_row { Some("true") } else { None }
                                                                                class:selected=is_selected
                                                                                on:click=move |_| {
                                                                                    if let Some(row) = row_ref.get() {
                                                                                        let _ = row.focus();
                                                                                    }
                                                                                    set_selected_download.set(Some(click_selection.clone()))
                                                                                }
                                                                            >
                                                                                <div class="file-picker-name">
                                                                                    <div class="file-picker-title">
                                                                                        {item.type_badge.as_ref().map(|badge| view! {
                                                                                            <span class="file-picker-type-badge">{badge.clone()}</span>
                                                                                        })}
                                                                                        <span>{item.display_name.clone()}</span>
                                                                                    </div>
                                                                                    {item.path_detail.as_ref().map(|path| view! {
                                                                                        <div class="file-picker-path">{path.clone()}</div>
                                                                                    })}
                                                                                </div>
                                                                                <div class="file-picker-meta">
                                                                                    <span class="file-picker-size">{format!("{size_mb:.1} MB")}</span>
                                                                                    {item.suggested_emulator.as_ref().map(|emulator| view! {
                                                                                        <span class="file-picker-match">{format!("Use {}", emulator)}</span>
                                                                                    })}
                                                                                    {(!region.is_empty()).then(|| view! {
                                                                                        <span class="file-picker-region">{region}</span>
                                                                                    })}
                                                                                    {(score > 0.5).then(|| view! {
                                                                                        <span class="file-picker-match">{format!("{:.0}% match", score * 100.0)}</span>
                                                                                    })}
                                                                                </div>
                                                                            </div>
                                                                        }
                                                                    }).collect::<Vec<_>>()}
                                                                </div>
                                                            }
                                                        }).collect::<Vec<_>>()}
                                                        }
                                                    </div>
                                                    <div class="file-picker-actions">
                                                        <button
                                                            class="import-btn-action"
                                                            disabled=move || selected_download.get().is_none()
                                                            on:click=move |_| {
                                                                if let (Some(selection), Some(g)) = (selected_download.get(), display_game.get_untracked()) {
                                                                    let db_id = g.database_id;
                                                                    let title = g.display_title.clone();
                                                                    let platform = stored_platform.get_value();
                                                                    let (torrent_url, file_index, download_mode) = match selection {
                                                                        MinervaDownloadSelection::WholeTorrent { torrent_url, representative_file_index } => {
                                                                            (torrent_url, representative_file_index, backend_api::MinervaDownloadMode::FullTorrent)
                                                                        }
                                                                        MinervaDownloadSelection::File { torrent_url, file_index } => {
                                                                            (torrent_url, Some(file_index), backend_api::MinervaDownloadMode::GameOnly)
                                                                        }
                                                                    };
                                                                    set_show_file_picker.set(false);
                                                                    set_minerva_starting.set(true);
                                                                    set_import_error.set(None);
                                                                    spawn_local(async move {
                                                                        match backend_api::test_torrent_connection().await {
                                                                            Ok(result) if !result.success => {
                                                                                set_minerva_starting.set(false);
                                                                                if let Some(setter) = set_show_settings {
                                                                                    setter.set(true);
                                                                                }
                                                                                set_import_error.set(Some(format!("qBittorrent error: {}", result.message)));
                                                                                return;
                                                                            }
                                                                            Err(e) => {
                                                                                set_minerva_starting.set(false);
                                                                                if let Some(setter) = set_show_settings {
                                                                                    setter.set(true);
                                                                                }
                                                                                set_import_error.set(Some(format!("qBittorrent error: {e}")));
                                                                                return;
                                                                            }
                                                                            _ => {}
                                                                        }
                                                                        match backend_api::start_minerva_download(
                                                                            torrent_url,
                                                                            file_index,
                                                                            db_id,
                                                                            title,
                                                                            platform,
                                                                            download_mode,
                                                                        ).await {
                                                                            Ok(job) => {
                                                                                let job_id = job.id.clone();
                                                                                let job_status = job.status.clone();
                                                                                let job_message = job.status_message.clone();

                                                                                match job_status.as_str() {
                                                                                    "completed" => {
                                                                                        set_minerva_starting.set(false);
                                                                                        set_import_job_id.set(None);
                                                                                        set_minerva_job_id.set(None);
                                                                                        request_minerva_download_queue_refresh();
                                                                                        refresh_minerva_download_queue_now().await;
                                                                                        refresh_display_game_file_state(
                                                                                            display_game,
                                                                                            set_display_game,
                                                                                            set_game_file,
                                                                                            4,
                                                                                        )
                                                                                        .await;
                                                                                    }
                                                                                    "failed" | "cancelled" => {
                                                                                        set_minerva_starting.set(false);
                                                                                        set_import_job_id.set(None);
                                                                                        set_minerva_job_id.set(None);
                                                                                        set_import_error.set(Some(
                                                                                            job_message.unwrap_or_else(|| {
                                                                                                format!("Download {}.", job_status)
                                                                                            }),
                                                                                        ));
                                                                                        request_minerva_download_queue_refresh();
                                                                                        refresh_minerva_download_queue_now().await;
                                                                                    }
                                                                                    _ => {
                                                                                        set_import_job_id.set(Some(job_id.clone()));
                                                                                        set_minerva_job_id.set(Some(job_id));
                                                                                        request_minerva_download_queue_refresh();
                                                                                        refresh_minerva_download_queue_now().await;
                                                                                    }
                                                                                }
                                                                            }
                                                                            Err(e) => {
                                                                                set_minerva_starting.set(false);
                                                                                set_import_error.set(Some(e));
                                                                            }
                                                                        }
                                                                    });
                                                                }
                                                            }
                                                        >
                                                            {move || match selected_download.get() {
                                                                Some(MinervaDownloadSelection::WholeTorrent { .. }) => "Download Full Torrent",
                                                                Some(MinervaDownloadSelection::File { .. }) => "Download Selected File",
                                                                None => "Choose a Download",
                                                            }}
                                                        </button>
                                                        <button
                                                            class="cancel-import-btn"
                                                            data-nav-back="true"
                                                            on:click=move |_| set_show_file_picker.set(false)
                                                        >"Cancel"</button>
                                                    </div>
                                                </div>
                                            </Show>

                                            // Import error message
                                            <Show when=move || import_error.get().is_some()>
                                                {move || import_error.get().map(|err| view! {
                                                    <div class="import-error">
                                                        <span>{err}</span>
                                                        <button class="import-error-dismiss" on:click=move |_| set_import_error.set(None)>"Dismiss"</button>
                                                    </div>
                                                })}
                                            </Show>

                                            // Play button only when game file exists and not importing
                                            <Show when=move || {
                                                (game_file.get().is_some()
                                                    || display_game.get().map(|g| g.has_game_file).unwrap_or(false))
                                                    && import_job_id.get().is_none()
                                            }>
                                                <button
                                                    class="play-btn"
                                                    data-nav-default="true"
                                                    disabled=move || game_uninstalling.get()
                                                    on:click=move |_| {
                                                    let platform = stored_platform.get_value();
                                                    let platform_for_filter = platform.clone();
                                                    let current_game_file = game_file.get_untracked();
                                                    set_emulators_loading.set(true);
                                                    set_show_emulator_picker.set(true);
                                                    spawn_local(async move {
                                                        match backend_api::get_emulators_with_status(platform).await {
                                                            Ok(emu_list) => set_emulators.set(filter_emulators_for_game(
                                                                &platform_for_filter,
                                                                current_game_file.as_ref(),
                                                                emu_list,
                                                            )),
                                                            Err(e) => {
                                                                web_sys::console::error_1(&format!("Failed to fetch emulators: {}", e).into());
                                                                set_emulators.set(Vec::new());
                                                            }
                                                        }
                                                        set_emulators_loading.set(false);
                                                    });
                                                }
                                                >"Play"</button>
                                                <Show when=move || {
                                                    game_file
                                                        .get()
                                                        .map(|file| file.import_source == "minerva")
                                                        .unwrap_or(false)
                                                }>
                                                    <button
                                                        class="uninstall-btn"
                                                        disabled=move || game_uninstalling.get()
                                                        on:click=move |_| {
                                                            let Some(current_game) = display_game.get_untracked() else {
                                                                return;
                                                            };
                                                            let window = web_sys::window().unwrap();
                                                            let confirmed = window
                                                                .confirm_with_message(&format!(
                                                                    "Uninstall {} and remove its Lunchbox-managed files?",
                                                                    current_game.display_title
                                                                ))
                                                                .unwrap_or(false);
                                                            if !confirmed {
                                                                return;
                                                            }

                                                            set_game_uninstalling.set(true);
                                                            set_import_error.set(None);
                                                            let db_id = current_game.database_id;
                                                            spawn_local(async move {
                                                                match backend_api::uninstall_game(db_id).await {
                                                                    Ok(()) => {
                                                                        set_game_file.set(None);
                                                                        set_display_game.update(|game| {
                                                                            if let Some(game) = game.as_mut() {
                                                                                game.has_game_file = false;
                                                                            }
                                                                        });
                                                                    }
                                                                    Err(e) => {
                                                                        set_import_error.set(Some(format!(
                                                                            "Uninstall failed: {}",
                                                                            e
                                                                        )));
                                                                    }
                                                                }
                                                                set_game_uninstalling.set(false);
                                                            });
                                                        }
                                                    >
                                                        {move || if game_uninstalling.get() { "Uninstalling..." } else { "Uninstall" }}
                                                    </button>
                                                </Show>
                                                <Show when=move || minerva_rom.get().is_some()>
                                                    <button
                                                        class="select-another-file-btn"
                                                        disabled=move || game_uninstalling.get()
                                                        title="Pick a different Minerva file for this game"
                                                        on:click=move |_| {
                                                            if let Some(g) = display_game.get_untracked() {
                                                                request_minerva_torrent_groups(
                                                                    g.database_id,
                                                                    g.display_title.clone(),
                                                                    g.platform.clone(),
                                                                    g.platform_id,
                                                                    true,
                                                                    true,
                                                                );
                                                            }
                                                        }
                                                    >
                                                        "Select Another File"
                                                    </button>
                                                </Show>
                                            </Show>

                                            <button
                                                class="manual-btn"
                                                disabled=move || manual_loading.get()
                                                title=move || {
                                                    if manual_path.get().is_some() {
                                                        "Open the cached game manual"
                                                    } else {
                                                        "Download the game manual"
                                                    }
                                                }
                                                on:click=move |_| {
                                                    if let Some(path) = manual_path.get_untracked() {
                                                        set_manual_error.set(None);
                                                        open_with_system_handler(
                                                            path,
                                                            set_manual_error,
                                                        );
                                                        return;
                                                    }

                                                    let Some(current_game) = display_game.get_untracked() else {
                                                        return;
                                                    };

                                                    set_manual_loading.set(true);
                                                    set_manual_error.set(None);
                                                    let title = current_game.display_title.clone();
                                                    let platform = current_game.platform.clone();
                                                    let db_id = if current_game.database_id > 0 {
                                                        Some(current_game.database_id)
                                                    } else {
                                                        None
                                                    };

                                                    spawn_local(async move {
                                                        match backend_api::download_game_manual(
                                                            title,
                                                            platform,
                                                            db_id,
                                                        )
                                                        .await
                                                        {
                                                            Ok(path) => {
                                                                set_manual_path.set(Some(path.clone()));
                                                                if let Err(e) =
                                                                    backend_api::open_local_file(path).await
                                                                {
                                                                    set_manual_error.set(Some(e));
                                                                }
                                                            }
                                                            Err(e) => set_manual_error.set(Some(e)),
                                                        }
                                                        set_manual_loading.set(false);
                                                    });
                                                }
                                            >
                                                {move || {
                                                    if manual_loading.get() {
                                                        "Downloading Manual..."
                                                    } else if manual_path.get().is_some() {
                                                        "View Manual"
                                                    } else {
                                                        "Download Manual"
                                                    }
                                                }}
                                            </button>

                                            <button
                                                class="favorite-btn"
                                                class:is-favorite=move || is_fav.get()
                                                on:click=on_toggle_favorite
                                            >
                                                {move || if is_fav.get() { "Unfavorite" } else { "Favorite" }}
                                            </button>
                                        </div>

                                        <Show when=move || manual_error.get().is_some()>
                                            {move || manual_error.get().map(|err| view! {
                                                <div class="manual-error">
                                                    <span>{err}</span>
                                                    <button class="import-error-dismiss" on:click=move |_| set_manual_error.set(None)>"Dismiss"</button>
                                                </div>
                                            })}
                                        </Show>

                                    </div>

                                // Video player, full width
                                <VideoPlayer
                                    game_title=g.title.clone()
                                    platform=g.platform.clone()
                                    launchbox_db_id=db_id
                                />

                                // Media carousel with arrows, full width
                                <MediaCarousel
                                    launchbox_db_id=db_id
                                    game_title=g.title.clone()
                                    platform=g.platform.clone()
                                    placeholder=first_char.clone()
                                />

                                <div class="game-details-description">
                                    <h2>"Description"</h2>
                                    <p>{description}</p>
                                </div>
                            </div>

                            <Show when=move || show_emulator_picker.get()>
                                <EmulatorPickerModal
                                    emulators=emulators
                                    set_emulators=set_emulators
                                    emulators_loading=emulators_loading
                                    game_file=game_file
                                    stored_title=stored_title
                                    stored_platform=stored_platform
                                    stored_db_id=stored_db_id
                                    set_show_emulator_picker=set_show_emulator_picker
                                />
                            </Show>
                        </div>
                    }
                    .into_any()
                })
            }}
        </Show>
    }
}

/// Media types available in the carousel
const MEDIA_TYPES: &[&str] = &[
    "Box - Front",
    "Box - 3D",
    "Box - Back",
    "Screenshot - Gameplay",
    "Screenshot - Game Title",
    "Clear Logo",
    "Fanart - Background",
];

/// Media carousel with left/right navigation including 3D box view
#[component]
fn MediaCarousel(
    launchbox_db_id: i64,
    game_title: String,
    platform: String,
    placeholder: String,
) -> impl IntoView {
    let (current_index, set_current_index) = signal(0usize);
    let (available_types, _set_available_types) =
        signal::<Vec<String>>(MEDIA_TYPES.iter().map(|&s| s.to_string()).collect());
    let (box_front_url, set_box_front_url) = signal::<Option<String>>(None);
    let (box_back_url, set_box_back_url) = signal::<Option<String>>(None);

    // Store props for async use
    let title = StoredValue::new(game_title.clone());
    let plat = StoredValue::new(platform.clone());
    let db_id = launchbox_db_id;

    // Pre-load box URLs for 3D viewer in background
    Effect::new(move || {
        let title = title.get_value();
        let plat = plat.get_value();

        spawn_local(async move {
            // Pre-load box front URL for 3D viewer
            if let Ok(path) = backend_api::download_image_with_fallback(
                title.clone(),
                plat.clone(),
                "Box - Front".to_string(),
                Some(db_id),
            )
            .await
            {
                set_box_front_url.set(Some(file_to_asset_url(&path)));
            }

            // Pre-load box back URL for 3D viewer
            if let Ok(path) = backend_api::download_image_with_fallback(
                title.clone(),
                plat.clone(),
                "Box - Back".to_string(),
                Some(db_id),
            )
            .await
            {
                set_box_back_url.set(Some(file_to_asset_url(&path)));
            }
        });
    });

    let prev = move |_| {
        let types = available_types.get();
        let current = current_index.get();
        if current > 0 {
            set_current_index.set(current - 1);
        } else {
            set_current_index.set(types.len().saturating_sub(1));
        }
    };

    let next = move |_| {
        let types = available_types.get();
        let current = current_index.get();
        if current < types.len() - 1 {
            set_current_index.set(current + 1);
        } else {
            set_current_index.set(0);
        }
    };

    let game_title_for_render = game_title.clone();
    let platform_for_render = platform.clone();
    let placeholder_for_render = placeholder.clone();

    view! {
        <div class="media-carousel">
            <div class="carousel-content">
                {move || {
                    let types = available_types.get();
                    let idx = current_index.get().min(types.len().saturating_sub(1));
                    let current_type = types.get(idx).cloned().unwrap_or_else(|| "Box - Front".to_string());

                    if current_type == "Box - 3D" {
                        // Show 3D box viewer
                        let front = box_front_url.get();
                        let back = box_back_url.get();

                        if let Some(front_url) = front {
                            view! {
                                <div class="carousel-3d-container">
                                    <Box3DViewer
                                        front_url=front_url.clone()
                                        back_url=back.clone()
                                        canvas_id=format!("box3d-{}", db_id)
                                    />
                                </div>
                            }.into_any()
                        } else {
                            view! {
                                <div class="carousel-loading">
                                    <div class="loading-spinner"></div>
                                    <span>"Loading 3D view..."</span>
                                </div>
                            }.into_any()
                        }
                    } else {
                        // Show 2D image with LazyImage
                        view! {
                            <LazyImage
                                launchbox_db_id=db_id
                                game_title=game_title_for_render.clone()
                                platform=platform_for_render.clone()
                                image_type=current_type.clone()
                                alt=current_type.clone()
                                class="carousel-image".to_string()
                                placeholder=placeholder_for_render.clone()
                                render_index=0
                                in_viewport=true
                            />
                        }.into_any()
                    }
                }}

                // Overlay arrows
                <button class="carousel-arrow carousel-prev" on:click=prev title="Previous">
                    <svg viewBox="0 0 24 24" fill="currentColor">
                        <path d="M15.41 7.41L14 6l-6 6 6 6 1.41-1.41L10.83 12z"/>
                    </svg>
                </button>
                <button class="carousel-arrow carousel-next" on:click=next title="Next">
                    <svg viewBox="0 0 24 24" fill="currentColor">
                        <path d="M8.59 16.59L10 18l6-6-6-6-1.41 1.41L13.17 12z"/>
                    </svg>
                </button>

                // Media type label
                <div class="carousel-label">
                    {move || {
                        let types = available_types.get();
                        let idx = current_index.get().min(types.len().saturating_sub(1));
                        let current_type = types.get(idx).cloned().unwrap_or_default();
                        let total = types.len();
                        format!("{} ({}/{})", current_type, idx + 1, total)
                    }}
                </div>
            </div>
        </div>
    }
}

fn format_date(date_str: &str) -> String {
    use chrono::{DateTime, NaiveDate, NaiveDateTime};

    if let Ok(dt) = DateTime::parse_from_rfc3339(date_str) {
        return dt.format("%b %-d, %Y").to_string();
    }

    if let Ok(dt) = NaiveDateTime::parse_from_str(date_str, "%Y-%m-%d %H:%M:%S") {
        return dt.format("%b %-d, %Y").to_string();
    }

    if let Ok(d) = NaiveDate::parse_from_str(date_str, "%Y-%m-%d") {
        return d.format("%b %-d, %Y").to_string();
    }

    date_str.to_string()
}

fn firmware_source_label(source: &str) -> &'static str {
    if source.starts_with("manual:") {
        "Manual import"
    } else if source.starts_with("minerva:") {
        "Minerva"
    } else if source.starts_with("github:") {
        "GitHub"
    } else if source == "user-import" {
        "User import"
    } else {
        "Other"
    }
}

fn summarize_firmware_sources(statuses: &[backend_api::FirmwareStatus]) -> Option<String> {
    let sources = statuses
        .iter()
        .map(|status| firmware_source_label(&status.source))
        .collect::<std::collections::BTreeSet<_>>();

    if sources.is_empty() {
        None
    } else {
        Some(sources.into_iter().collect::<Vec<_>>().join(", "))
    }
}

fn refresh_game_launch_template_preview(
    launchbox_db_id: i64,
    platform_name: String,
    emulator_name: String,
    is_retroarch_core: bool,
    set_loading: WriteSignal<bool>,
    set_error: WriteSignal<Option<String>>,
    set_preview: WriteSignal<Option<backend_api::GameLaunchTemplatePreview>>,
    set_draft: WriteSignal<String>,
    set_override_enabled: WriteSignal<bool>,
) {
    set_loading.set(true);
    set_error.set(None);

    spawn_local(async move {
        match backend_api::get_game_launch_template_preview(
            launchbox_db_id,
            platform_name,
            emulator_name,
            is_retroarch_core,
        )
        .await
        {
            Ok(preview) => {
                let game_override = preview.game_command_template_override.clone();
                set_override_enabled.set(game_override.is_some());
                set_draft.set(game_override.unwrap_or_default());
                set_preview.set(Some(preview));
            }
            Err(e) => set_error.set(Some(format!("Failed to load command preview: {}", e))),
        }

        set_loading.set(false);
    });
}

fn game_launch_template_note(preview: &backend_api::GameLaunchTemplatePreview) -> &'static str {
    if preview.is_prepared_install {
        "Prepared installs use runtime-specific placeholders such as %{config}, %{shared_config}, %{vm_root}, %{game_path}, and %{game_id}. A game override wins over Settings > Emulator Launch Commands."
    } else if preview.runtime_kind.eq_ignore_ascii_case("retroarch") {
        "Use %f for the selected file, %{core} for the RetroArch core, and %% for a literal percent. A game override wins over Settings > Emulator Launch Commands."
    } else {
        "Use %f for the selected file and %% for a literal percent. Emulator-specific defaults may also expose placeholders like %{mame_romset} or %{hypseus_framefile}. A game override wins over Settings > Emulator Launch Commands."
    }
}

#[component]
fn GameLaunchTemplateEditor(
    launchbox_db_id: i64,
    platform_name: String,
    emulator_name: String,
    is_retroarch_core: bool,
) -> impl IntoView {
    if launchbox_db_id <= 0 {
        return view! { <></> }.into_any();
    }

    let (expanded, set_expanded) = signal(false);
    let (loading, set_loading) = signal(false);
    let (saving, set_saving) = signal(false);
    let (preview, set_preview) = signal::<Option<backend_api::GameLaunchTemplatePreview>>(None);
    let (draft_template, set_draft_template) = signal(String::new());
    let (override_enabled, set_override_enabled) = signal(false);
    let (error, set_error) = signal::<Option<String>>(None);
    let (status, set_status) = signal::<Option<String>>(None);
    let platform_name_value = StoredValue::new(platform_name);
    let emulator_name_value = StoredValue::new(emulator_name);

    view! {
        <div class="emulator-command-editor">
            <button
                class="emulator-pref-btn emulator-command-toggle"
                class:active=move || expanded.get()
                on:click=move |e: web_sys::MouseEvent| {
                    e.stop_propagation();
                    let next = !expanded.get_untracked();
                    set_expanded.set(next);
                    if next {
                        set_status.set(None);
                        refresh_game_launch_template_preview(
                            launchbox_db_id,
                            platform_name_value.get_value(),
                            emulator_name_value.get_value(),
                            is_retroarch_core,
                            set_loading,
                            set_error,
                            set_preview,
                            set_draft_template,
                            set_override_enabled,
                        );
                    }
                }
            >
                {move || if expanded.get() { "Hide Command Line" } else { "Command Line" }}
            </button>

            <Show when=move || expanded.get()>
                <div class="emulator-command-panel">
                    <Show when=move || error.get().is_some()>
                        {move || error.get().map(|message| view! {
                            <div class="emulator-error emulator-command-error">
                                <span class="error-icon">"!"</span>
                                <span>{message}</span>
                                <button class="error-dismiss" on:click=move |_| set_error.set(None)>"Dismiss"</button>
                            </div>
                        })}
                    </Show>

                    <Show when=move || status.get().is_some()>
                        {move || status.get().map(|message| view! {
                            <div class="emulator-pref-indicator">{message}</div>
                        })}
                    </Show>

                    <Show
                        when=move || !loading.get()
                        fallback=|| view! {
                            <div class="emulator-loading">
                                <div class="loading-spinner"></div>
                                <span>"Loading command preview..."</span>
                            </div>
                        }
                    >
                        {move || preview.get().map(|preview| {
                            let note = game_launch_template_note(&preview).to_string();
                            let platform_override =
                                preview.platform_command_template_override.clone();
                            let game_override = preview.game_command_template_override.clone();
                            let has_platform_override = platform_override.is_some();
                            let has_game_override = game_override.is_some();
                            let effective_template_value =
                                StoredValue::new(preview.effective_template.clone());
                            view! {
                                <div class="emulator-command-grid">
                                    <div class="emulator-command-row">
                                        <span class="emulator-command-label">"Built-In Default"</span>
                                        <code class="emulator-command-value">{preview.default_template.clone()}</code>
                                    </div>

                                    <Show when=move || has_platform_override>
                                        <div class="emulator-command-row">
                                            <span class="emulator-command-label">"Platform Override"</span>
                                            <code class="emulator-command-value">
                                                {platform_override.clone().unwrap_or_default()}
                                            </code>
                                        </div>
                                    </Show>

                                    <Show when=move || has_game_override>
                                        <div class="emulator-command-row">
                                            <span class="emulator-command-label">"Game Override"</span>
                                            <code class="emulator-command-value">
                                                {game_override.clone().unwrap_or_default()}
                                            </code>
                                        </div>
                                    </Show>

                                    <div class="emulator-command-row">
                                        <span class="emulator-command-label">"Effective Command"</span>
                                        <code class="emulator-command-value effective">
                                            {preview.effective_template.clone()}
                                        </code>
                                    </div>
                                </div>

                                <Show
                                    when=move || override_enabled.get()
                                    fallback=move || view! {
                                        <div class="emulator-command-enable">
                                            <button
                                                class="emulator-pref-btn"
                                                on:click=move |e: web_sys::MouseEvent| {
                                                    e.stop_propagation();
                                                    set_override_enabled.set(true);
                                                    set_draft_template.set(
                                                        effective_template_value.get_value(),
                                                    );
                                                    set_status.set(None);
                                                    set_error.set(None);
                                                }
                                            >
                                                "Enable Per-Game Override"
                                            </button>
                                        </div>
                                    }
                                >
                                    <label class="settings-label">
                                        "Per-Game Override"
                                        <textarea
                                            class="settings-input emulator-command-textarea"
                                            rows="3"
                                            prop:value=move || draft_template.get()
                                            on:input=move |ev| {
                                                set_draft_template.set(event_target_value(&ev));
                                                set_status.set(None);
                                                set_error.set(None);
                                            }
                                        />
                                    </label>
                                </Show>

                                <p class="settings-hint emulator-command-hint">{note}</p>

                                <Show when=move || override_enabled.get()>
                                    <div class="emulator-pref-buttons emulator-command-actions">
                                        <button
                                            class="emulator-pref-btn emulator-play-btn"
                                            disabled=move || saving.get()
                                            on:click=move |e: web_sys::MouseEvent| {
                                                e.stop_propagation();
                                                let command_template =
                                                    draft_template.get_untracked().trim().to_string();
                                                let platform_name = platform_name_value.get_value();
                                                let emulator_name = emulator_name_value.get_value();
                                                set_saving.set(true);
                                                set_status.set(None);
                                                set_error.set(None);

                                                spawn_local(async move {
                                                    match backend_api::set_game_launch_template_override(
                                                        launchbox_db_id,
                                                        emulator_name.clone(),
                                                        is_retroarch_core,
                                                        command_template.clone(),
                                                    )
                                                    .await
                                                    {
                                                        Ok(()) => {
                                                            set_status.set(Some(
                                                                "Saved the per-game command override."
                                                                    .to_string(),
                                                            ));
                                                            refresh_game_launch_template_preview(
                                                                launchbox_db_id,
                                                                platform_name.clone(),
                                                                emulator_name.clone(),
                                                                is_retroarch_core,
                                                                set_loading,
                                                                set_error,
                                                                set_preview,
                                                                set_draft_template,
                                                                set_override_enabled,
                                                            );
                                                        }
                                                        Err(e) => set_error.set(Some(format!(
                                                            "Failed to save per-game command override: {}",
                                                            e
                                                        ))),
                                                    }

                                                    set_saving.set(false);
                                                });
                                            }
                                        >
                                            {move || if saving.get() { "Saving..." } else { "Save Override" }}
                                        </button>

                                        <button
                                            class="emulator-pref-btn emulator-uninstall-btn"
                                            disabled=move || saving.get()
                                            on:click=move |e: web_sys::MouseEvent| {
                                                e.stop_propagation();
                                                if has_game_override {
                                                    let platform_name = platform_name_value.get_value();
                                                    let emulator_name = emulator_name_value.get_value();
                                                    set_saving.set(true);
                                                    set_status.set(None);
                                                    set_error.set(None);

                                                    spawn_local(async move {
                                                        match backend_api::clear_game_launch_template_override(
                                                            launchbox_db_id,
                                                            emulator_name.clone(),
                                                            is_retroarch_core,
                                                        )
                                                        .await
                                                        {
                                                            Ok(()) => {
                                                                set_status.set(Some(
                                                                    "Cleared the per-game command override."
                                                                        .to_string(),
                                                                ));
                                                                refresh_game_launch_template_preview(
                                                                    launchbox_db_id,
                                                                    platform_name.clone(),
                                                                    emulator_name.clone(),
                                                                    is_retroarch_core,
                                                                    set_loading,
                                                                    set_error,
                                                                    set_preview,
                                                                    set_draft_template,
                                                                    set_override_enabled,
                                                                );
                                                            }
                                                            Err(e) => set_error.set(Some(format!(
                                                                "Failed to clear per-game command override: {}",
                                                                e
                                                            ))),
                                                        }

                                                        set_saving.set(false);
                                                    });
                                                } else {
                                                    set_override_enabled.set(false);
                                                    set_draft_template.set(String::new());
                                                    set_status.set(None);
                                                    set_error.set(None);
                                                }
                                            }
                                        >
                                            {if has_game_override {
                                                "Clear Override"
                                            } else {
                                                "Cancel"
                                            }}
                                        </button>
                                    </div>
                                </Show>
                            }
                        })}
                    </Show>
                </div>
            </Show>
        </div>
    }
    .into_any()
}

#[component]
fn EmulatorPickerModal(
    emulators: ReadSignal<Vec<EmulatorWithStatus>>,
    set_emulators: WriteSignal<Vec<EmulatorWithStatus>>,
    emulators_loading: ReadSignal<bool>,
    game_file: ReadSignal<Option<GameFile>>,
    stored_title: StoredValue<String>,
    stored_platform: StoredValue<String>,
    stored_db_id: StoredValue<i64>,
    set_show_emulator_picker: WriteSignal<bool>,
) -> impl IntoView {
    // Track the current emulator preference
    let (current_pref, set_current_pref) = signal::<Option<String>>(None);
    // Track launching/installing state with progress message
    let (progress_state, set_progress_state) = signal::<Option<String>>(None);
    // Track error state
    let (error_state, set_error_state) = signal::<Option<String>>(None);
    // Track success state
    let (success_state, set_success_state) = signal::<Option<String>>(None);

    let show_launch_success = move |emulator_name: String| {
        set_progress_state.set(None);
        set_error_state.set(None);
        set_success_state.set(Some(format!(
            "Launched {emulator_name}. If no window surfaced, check another workspace or an existing emulator window."
        )));
        spawn_local(async move {
            wasm_bindgen_futures::JsFuture::from(js_sys::Promise::new(&mut |resolve, _| {
                web_sys::window()
                    .unwrap()
                    .set_timeout_with_callback_and_timeout_and_arguments_0(&resolve, 1800)
                    .unwrap();
            }))
            .await
            .ok();
            set_success_state.set(None);
            set_show_emulator_picker.set(false);
        });
    };

    // Load current preference when modal opens
    Effect::new(move || {
        let db_id = stored_db_id.get_value();
        let platform = stored_platform.get_value();
        spawn_local(async move {
            if let Ok(pref) = backend_api::get_emulator_preference(db_id, platform).await {
                set_current_pref.set(pref);
            }
        });
    });

    // Can close modal only when not in progress
    let can_close = move || progress_state.get().is_none();

    view! {
        <div
            class="emulator-picker-overlay"
            data-nav-scope="emulator-picker"
            data-nav-scope-active="true"
            data-nav-scope-priority="130"
            on:click=move |e| {
            e.stop_propagation();
            if can_close() {
                set_show_emulator_picker.set(false);
            }
        }>
            <div class="emulator-picker-modal" on:click=|e| e.stop_propagation()>
                <div class="emulator-picker-header">
                    <h3>"Select Emulator"</h3>
                    <button
                        class="emulator-picker-close"
                        data-nav-back="true"
                        on:click=move |_| {
                            if can_close() {
                                set_show_emulator_picker.set(false);
                            }
                        }
                        disabled=move || !can_close()
                    >"×"</button>
                </div>
                <div class="emulator-picker-content">
                    // Show progress state (installing/launching)
                    <Show when=move || progress_state.get().is_some()>
                        {move || progress_state.get().map(|msg| view! {
                            <div class="emulator-progress">
                                <div class="loading-spinner"></div>
                                <span>{msg}</span>
                            </div>
                        })}
                    </Show>

                    // Show error state
                    <Show when=move || error_state.get().is_some()>
                        {move || error_state.get().map(|err| view! {
                            <div class="emulator-error">
                                <span class="error-icon">"!"</span>
                                <span>{err}</span>
                                <button class="error-dismiss" on:click=move |_| set_error_state.set(None)>"Dismiss"</button>
                            </div>
                        })}
                    </Show>

                    <Show when=move || success_state.get().is_some()>
                        {move || success_state.get().map(|msg| view! {
                            <div class="emulator-pref-indicator">{msg}</div>
                        })}
                    </Show>

                    // Show current preference indicator (when not in progress)
                    <Show when=move || current_pref.get().is_some() && progress_state.get().is_none()>
                        {move || current_pref.get().map(|pref| view! {
                            <div class="emulator-pref-indicator">
                                "Default: " {pref}
                            </div>
                        })}
                    </Show>

                    <Show
                        when=move || !emulators_loading.get() && progress_state.get().is_none()
                        fallback=move || {
                            if emulators_loading.get() {
                                view! { <div class="emulator-loading"><div class="loading-spinner"></div>"Loading emulators..."</div> }.into_any()
                            } else {
                                view! {}.into_any()
                            }
                        }
                    >
                        <Show
                            when=move || !emulators.get().is_empty()
                            fallback=|| view! { <div class="emulator-empty">"No emulators found for this platform."</div> }
                        >
                            <ul class="emulator-list">
                                <For
                                    each=move || {
                                        emulators
                                            .get()
                                            .into_iter()
                                            .enumerate()
                                            .collect::<Vec<_>>()
                                    }
                                    key=|(index, emu)| {
                                        format!(
                                            "{}:{}:{}",
                                            index,
                                            emu.id,
                                            if emu.is_retroarch_core {
                                                "retroarch"
                                            } else {
                                                "standalone"
                                            }
                                        )
                                    }
                                    children=move |(emu_index, emu): (usize, EmulatorWithStatus)| {
                                        let name = emu.name.clone();
                                        let display_name = emu.display_name.clone();
                                        let firmware_display_name =
                                            StoredValue::new(emu.display_name.clone());
                                        let uninstall_display_name =
                                            StoredValue::new(emu.display_name.clone());
                                        let is_installed = emu.is_installed;
                                        let is_retroarch_core = emu.is_retroarch_core;
                                        let install_method = emu.install_method.clone();
                                        let uninstall_method = emu.uninstall_method.clone();
                                        let name_for_click = emu.name.clone();
                                        let name_for_game_pref = emu.name.clone();
                                        let name_for_platform_pref = emu.name.clone();
                                        let uninstall_emulator_name =
                                            StoredValue::new(emu.name.clone());
                                        let firmware_emulator_name =
                                            StoredValue::new(emu.name.clone());
                                        let homepage = emu.homepage.clone();
                                        let notes = emu.notes.clone();
                                        let required_firmware = emu
                                            .firmware_statuses
                                            .into_iter()
                                            .filter(|status| status.required)
                                            .collect::<Vec<_>>();
                                        let manual_missing_firmware = required_firmware
                                            .iter()
                                            .filter(|status| {
                                                !status.imported
                                                    && status.source.starts_with("manual:")
                                            })
                                            .cloned()
                                            .collect::<Vec<_>>();
                                        let missing_firmware = required_firmware
                                            .iter()
                                            .filter(|status| {
                                                !status.imported
                                                    && !status.source.starts_with("manual:")
                                            })
                                            .cloned()
                                            .collect::<Vec<_>>();
                                        let unsynced_firmware = required_firmware
                                            .iter()
                                            .filter(|status| {
                                                status.imported
                                                    && !status.synced
                                                    && !status.launch_scoped
                                            })
                                            .cloned()
                                            .collect::<Vec<_>>();
                                        let launch_scoped_firmware = required_firmware
                                            .iter()
                                            .filter(|status| {
                                                status.imported
                                                    && !status.synced
                                                    && status.launch_scoped
                                            })
                                            .cloned()
                                            .collect::<Vec<_>>();
                                        let missing_firmware_summary = if missing_firmware.is_empty() {
                                            None
                                        } else {
                                            Some(
                                                missing_firmware
                                                    .iter()
                                                    .map(|status| status.package_name.clone())
                                                    .collect::<Vec<_>>()
                                                    .join(", "),
                                            )
                                        };
                                        let manual_missing_firmware_summary = if manual_missing_firmware.is_empty() {
                                            None
                                        } else {
                                            Some(
                                                manual_missing_firmware
                                                    .iter()
                                                    .map(|status| status.package_name.clone())
                                                    .collect::<Vec<_>>()
                                                    .join(", "),
                                            )
                                        };
                                        let unsynced_firmware_summary = if unsynced_firmware.is_empty() {
                                            None
                                        } else {
                                            Some(
                                                unsynced_firmware
                                                    .iter()
                                                    .map(|status| status.package_name.clone())
                                                    .collect::<Vec<_>>()
                                                    .join(", "),
                                            )
                                        };

                                        let launch_scoped_firmware_summary = if launch_scoped_firmware.is_empty() {
                                            None
                                        } else {
                                            Some(
                                                launch_scoped_firmware
                                                    .iter()
                                                    .map(|status| status.package_name.clone())
                                                    .collect::<Vec<_>>()
                                                    .join(", "),
                                            )
                                        };

                                        let firmware_source_summary = summarize_firmware_sources(&required_firmware);
                                        let firmware_runtime_path = required_firmware
                                            .first()
                                            .map(|status| status.runtime_path.clone());
                                        let needs_firmware_action =
                                            !missing_firmware.is_empty() || !unsynced_firmware.is_empty();
                                        let has_repairable_firmware = is_installed
                                            && required_firmware
                                                .iter()
                                                .any(|status| {
                                                    status.imported
                                                        && !status.launch_scoped
                                                        && !status.source.starts_with("manual:")
                                                });
                                        let show_firmware_warning = needs_firmware_action
                                            || !launch_scoped_firmware.is_empty()
                                            || !manual_missing_firmware.is_empty()
                                            || has_repairable_firmware;
                                        let firmware_action_label = if !missing_firmware.is_empty() {
                                            "Install Firmware"
                                        } else if !unsynced_firmware.is_empty() {
                                            "Sync Firmware"
                                        } else {
                                            "Repair Firmware"
                                        };
                                        let can_open_firmware_folder = !manual_missing_firmware.is_empty()
                                            || (is_installed
                                                && required_firmware
                                                    .iter()
                                                    .any(|status| !status.launch_scoped));
                                        let firmware_open_label = if !manual_missing_firmware.is_empty() {
                                            "Open Import Folder"
                                        } else {
                                            "Open Firmware Folder"
                                        };

                                        // Handler for launch/install+launch
                                        let on_launch = move |_| {
                                            pause_game_details_video();
                                            let emulator_name = name_for_click.clone();
                                            let title = stored_title.get_value();
                                            let platform = stored_platform.get_value();
                                            let db_id = stored_db_id.get_value();
                                            let fallback_rom_path = game_file.get_untracked().map(|file| file.file_path);
                                            let is_ra = is_retroarch_core;

                                            if is_installed {
                                                // Just launch
                                                set_progress_state.set(Some(format!("Launching {}...", emulator_name)));
                                                spawn_local(async move {
                                                    // Record play session
                                                    let _ = backend_api::record_play_session(db_id, title, platform.clone()).await;
                                                    // Launch selected emulator with ROM path
                                                    match launch_game_with_resolved_rom(
                                                        db_id,
                                                        platform.clone(),
                                                        fallback_rom_path.clone(),
                                                        emulator_name.clone(),
                                                        is_ra,
                                                    ).await {
                                                        Ok(result) => {
                                                            if result.success {
                                                                show_launch_success(emulator_name.clone());
                                                            } else {
                                                                set_progress_state.set(None);
                                                                set_error_state.set(result.error);
                                                            }
                                                        }
                                                        Err(e) => {
                                                            set_progress_state.set(None);
                                                            set_error_state.set(Some(format!("Launch failed: {}", e)));
                                                        }
                                                    }
                                                });
                                            } else {
                                                // Install then launch
                                                set_progress_state.set(Some(format!("Installing {}...", emulator_name)));
                                                let emulator_for_install = emulator_name.clone();
                                                spawn_local(async move {
                                                    match backend_api::install_emulator(
                                                        emulator_for_install.clone(),
                                                        Some(platform.clone()),
                                                        is_ra,
                                                    )
                                                    .await
                                                    {
                                                        Ok(_path) => {
                                                            set_progress_state.set(Some(format!("Launching {}...", emulator_for_install)));
                                                            // Record play session
                                                            let _ = backend_api::record_play_session(db_id, title, platform.clone()).await;
                                                            // Launch selected emulator with ROM path
                                                            match launch_game_with_resolved_rom(
                                                                db_id,
                                                                platform.clone(),
                                                                fallback_rom_path.clone(),
                                                                emulator_for_install.clone(),
                                                                is_ra,
                                                            ).await {
                                                                Ok(result) => {
                                                                    if result.success {
                                                                        show_launch_success(emulator_for_install.clone());
                                                                    } else {
                                                                        set_progress_state.set(None);
                                                                        set_error_state.set(result.error);
                                                                    }
                                                                }
                                                                Err(e) => {
                                                                    set_progress_state.set(None);
                                                                    set_error_state.set(Some(format!("Launch failed: {}", e)));
                                                                }
                                                            }
                                                        }
                                                        Err(e) => {
                                                            set_progress_state.set(None);
                                                            set_error_state.set(Some(format!("Install failed: {}", e)));
                                                        }
                                                    }
                                                });
                                            }
                                        };

                                        let on_set_game_pref = move |e: web_sys::MouseEvent| {
                                            e.stop_propagation();
                                            pause_game_details_video();
                                            let emulator_name = name_for_game_pref.clone();
                                            let title = stored_title.get_value();
                                            let platform = stored_platform.get_value();
                                            let db_id = stored_db_id.get_value();
                                            let fallback_rom_path = game_file.get_untracked().map(|file| file.file_path);
                                            let is_ra = is_retroarch_core;

                                            if is_installed {
                                                set_progress_state.set(Some(format!("Launching {}...", emulator_name)));
                                                spawn_local(async move {
                                                    let _ = backend_api::set_game_emulator_preference(db_id, emulator_name.clone()).await;
                                                    let _ = backend_api::record_play_session(db_id, title, platform.clone()).await;
                                                    match launch_game_with_resolved_rom(
                                                        db_id,
                                                        platform.clone(),
                                                        fallback_rom_path.clone(),
                                                        emulator_name.clone(),
                                                        is_ra,
                                                    ).await {
                                                        Ok(result) => {
                                                            if result.success {
                                                                show_launch_success(emulator_name.clone());
                                                            } else {
                                                                set_progress_state.set(None);
                                                                set_error_state.set(result.error);
                                                            }
                                                        }
                                                        Err(e) => {
                                                            set_progress_state.set(None);
                                                            set_error_state.set(Some(format!("Launch failed: {}", e)));
                                                        }
                                                    }
                                                });
                                            } else {
                                                set_progress_state.set(Some(format!("Installing {}...", emulator_name)));
                                                let emu_for_install = emulator_name.clone();
                                                spawn_local(async move {
                                                    match backend_api::install_emulator(
                                                        emu_for_install.clone(),
                                                        Some(platform.clone()),
                                                        is_ra,
                                                    )
                                                    .await
                                                    {
                                                        Ok(_) => {
                                                            let _ = backend_api::set_game_emulator_preference(db_id, emu_for_install.clone()).await;
                                                            set_progress_state.set(Some(format!("Launching {}...", emu_for_install)));
                                                            let _ = backend_api::record_play_session(db_id, title, platform.clone()).await;
                                                            match launch_game_with_resolved_rom(
                                                                db_id,
                                                                platform.clone(),
                                                                fallback_rom_path.clone(),
                                                                emu_for_install.clone(),
                                                                is_ra,
                                                            ).await {
                                                                Ok(result) => {
                                                                    if result.success {
                                                                        show_launch_success(emu_for_install.clone());
                                                                    } else {
                                                                        set_progress_state.set(None);
                                                                        set_error_state.set(result.error);
                                                                    }
                                                                }
                                                                Err(e) => {
                                                                    set_progress_state.set(None);
                                                                    set_error_state.set(Some(format!("Launch failed: {}", e)));
                                                                }
                                                            }
                                                        }
                                                        Err(e) => {
                                                            set_progress_state.set(None);
                                                            set_error_state.set(Some(format!("Install failed: {}", e)));
                                                        }
                                                    }
                                                });
                                            }
                                        };

                                        let on_set_platform_pref = move |e: web_sys::MouseEvent| {
                                            e.stop_propagation();
                                            pause_game_details_video();
                                            let emulator_name = name_for_platform_pref.clone();
                                            let title = stored_title.get_value();
                                            let platform = stored_platform.get_value();
                                            let db_id = stored_db_id.get_value();
                                            let fallback_rom_path = game_file.get_untracked().map(|file| file.file_path);
                                            let is_ra = is_retroarch_core;

                                            if is_installed {
                                                set_progress_state.set(Some(format!("Launching {}...", emulator_name)));
                                                spawn_local(async move {
                                                    let _ = backend_api::set_platform_emulator_preference(platform.clone(), emulator_name.clone()).await;
                                                    let _ = backend_api::record_play_session(db_id, title, platform.clone()).await;
                                                    match launch_game_with_resolved_rom(
                                                        db_id,
                                                        platform.clone(),
                                                        fallback_rom_path.clone(),
                                                        emulator_name.clone(),
                                                        is_ra,
                                                    ).await {
                                                        Ok(result) => {
                                                            if result.success {
                                                                show_launch_success(emulator_name.clone());
                                                            } else {
                                                                set_progress_state.set(None);
                                                                set_error_state.set(result.error);
                                                            }
                                                        }
                                                        Err(e) => {
                                                            set_progress_state.set(None);
                                                            set_error_state.set(Some(format!("Launch failed: {}", e)));
                                                        }
                                                    }
                                                });
                                            } else {
                                                set_progress_state.set(Some(format!("Installing {}...", emulator_name)));
                                                let emu_for_install = emulator_name.clone();
                                                spawn_local(async move {
                                                    match backend_api::install_emulator(
                                                        emu_for_install.clone(),
                                                        Some(platform.clone()),
                                                        is_ra,
                                                    )
                                                    .await
                                                    {
                                                        Ok(_) => {
                                                            let _ = backend_api::set_platform_emulator_preference(platform.clone(), emu_for_install.clone()).await;
                                                            set_progress_state.set(Some(format!("Launching {}...", emu_for_install)));
                                                            let _ = backend_api::record_play_session(db_id, title, platform.clone()).await;
                                                            match launch_game_with_resolved_rom(
                                                                db_id,
                                                                platform.clone(),
                                                                fallback_rom_path.clone(),
                                                                emu_for_install.clone(),
                                                                is_ra,
                                                            ).await {
                                                                Ok(result) => {
                                                                    if result.success {
                                                                        show_launch_success(emu_for_install.clone());
                                                                    } else {
                                                                        set_progress_state.set(None);
                                                                        set_error_state.set(result.error);
                                                                    }
                                                                }
                                                                Err(e) => {
                                                                    set_progress_state.set(None);
                                                                    set_error_state.set(Some(format!("Launch failed: {}", e)));
                                                                }
                                                            }
                                                        }
                                                        Err(e) => {
                                                            set_progress_state.set(None);
                                                            set_error_state.set(Some(format!("Install failed: {}", e)));
                                                        }
                                                    }
                                                });
                                            }
                                        };

                                        let is_preferred = {
                                            let name_check = name.clone();
                                            move || current_pref.get().as_ref() == Some(&name_check)
                                        };

                                        let can_auto_install = install_method.is_some();
                                        let can_uninstall = uninstall_method.is_some();
                                        let action_text = if is_installed {
                                            "Play"
                                        } else if can_auto_install {
                                            "Install & Play"
                                        } else {
                                            "Manual Install Required"
                                        };
                                        let firmware_warning = if show_firmware_warning {
                                            Some(view! {
                                                <div class="emulator-firmware-warning">
                                                    <div class="emulator-firmware-title">
                                                        {if !manual_missing_firmware.is_empty() && missing_firmware.is_empty() && unsynced_firmware.is_empty() {
                                                            "Manual firmware required"
                                                        } else if !missing_firmware.is_empty() {
                                                            "Missing required firmware"
                                                        } else if !unsynced_firmware.is_empty() {
                                                            "Firmware needs sync"
                                                        } else {
                                                            "Firmware will be staged at launch"
                                                        }}
                                                    </div>
                                                    {missing_firmware_summary.clone().map(|summary| view! {
                                                        <div class="emulator-firmware-packages">
                                                            "Packages: " {summary}
                                                        </div>
                                                    })}
                                                    {manual_missing_firmware_summary.clone().map(|summary| view! {
                                                        <div class="emulator-firmware-packages">
                                                            "Manual import required: " {summary}
                                                        </div>
                                                    })}
                                                    {unsynced_firmware_summary.clone().map(|summary| view! {
                                                        <div class="emulator-firmware-packages">
                                                            "Not yet synced: " {summary}
                                                        </div>
                                                    })}
                                                    {launch_scoped_firmware_summary.clone().map(|summary| view! {
                                                        <div class="emulator-firmware-packages">
                                                            "Copied beside the ROM at launch: " {summary}
                                                        </div>
                                                    })}
                                                    {firmware_source_summary.clone().map(|summary| view! {
                                                        <div class="emulator-firmware-source">
                                                            "Source: " {summary}
                                                        </div>
                                                    })}
                                                    {firmware_runtime_path.clone().map(|path| view! {
                                                        <div class="emulator-firmware-path">
                                                            {if launch_scoped_firmware.is_empty() && missing_firmware.iter().all(|status| !status.launch_scoped) {
                                                                "Runtime path: "
                                                            } else {
                                                                "Launch path: "
                                                            }}
                                                            {path}
                                                        </div>
                                                    })}
                                                    <div class="emulator-firmware-actions">
                                                        <Show when=move || is_installed && (needs_firmware_action || has_repairable_firmware)>
                                                            <button
                                                                class="emulator-firmware-btn"
                                                                on:click=move |e: web_sys::MouseEvent| {
                                                                    e.stop_propagation();
                                                                    let emulator_name = firmware_emulator_name.get_value();
                                                                    let display_name = firmware_display_name.get_value();
                                                                    let platform = stored_platform.get_value();
                                                                    let current_game_file = game_file.get_untracked();
                                                                    let is_ra = is_retroarch_core;

                                                                    set_progress_state.set(Some(format!(
                                                                        "Installing firmware for {}...",
                                                                        display_name
                                                                    )));
                                                                    set_error_state.set(None);
                                                                    set_success_state.set(None);

                                                                    spawn_local(async move {
                                                                        match backend_api::install_firmware(
                                                                            emulator_name.clone(),
                                                                            platform.clone(),
                                                                            is_ra,
                                                                        )
                                                                        .await
                                                                        {
                                                                            Ok(_) => {
                                                                                match backend_api::get_emulators_with_status(platform.clone()).await {
                                                                                    Ok(emu_list) => {
                                                                                        set_emulators.set(filter_emulators_for_game(
                                                                                            &platform,
                                                                                            current_game_file.as_ref(),
                                                                                            emu_list,
                                                                                        ));
                                                                                        set_progress_state.set(None);
                                                                                        set_error_state.set(None);
                                                                                        set_success_state.set(Some(format!(
                                                                                            "Firmware is ready for {}.",
                                                                                            display_name
                                                                                        )));
                                                                                    }
                                                                                    Err(e) => {
                                                                                        set_progress_state.set(None);
                                                                                        set_error_state.set(Some(format!(
                                                                                            "Firmware installed, but emulator status refresh failed: {}",
                                                                                            e
                                                                                        )));
                                                                                    }
                                                                                }
                                                                            }
                                                                            Err(e) => {
                                                                                set_progress_state.set(None);
                                                                                set_error_state.set(Some(format!(
                                                                                    "Firmware install failed: {}",
                                                                                    e
                                                                                )));
                                                                            }
                                                                        }
                                                                    });
                                                                }
                                                            >
                                                                {firmware_action_label}
                                                            </button>
                                                        </Show>
                                                        <Show when=move || can_open_firmware_folder>
                                                            <button
                                                                class="emulator-firmware-btn secondary"
                                                                on:click=move |e: web_sys::MouseEvent| {
                                                                    e.stop_propagation();
                                                                    let emulator_name = firmware_emulator_name.get_value();
                                                                    let display_name = firmware_display_name.get_value();
                                                                    let platform = stored_platform.get_value();
                                                                    let is_ra = is_retroarch_core;

                                                                    set_error_state.set(None);
                                                                    set_success_state.set(None);

                                                                    spawn_local(async move {
                                                                        match backend_api::open_firmware_directory(
                                                                            emulator_name,
                                                                            platform,
                                                                            is_ra,
                                                                        )
                                                                        .await
                                                                        {
                                                                            Ok(opened_path) => {
                                                                                set_success_state.set(Some(format!(
                                                                                    "Opened firmware folder for {}: {}",
                                                                                    display_name,
                                                                                    opened_path
                                                                                )));
                                                                            }
                                                                            Err(e) => {
                                                                                set_error_state.set(Some(format!(
                                                                                    "Failed to open firmware folder: {}",
                                                                                    e
                                                                                )));
                                                                            }
                                                                        }
                                                                    });
                                                                }
                                                            >
                                                                {firmware_open_label}
                                                            </button>
                                                        </Show>
                                                    </div>
                                                </div>
                                            })
                                        } else {
                                            None
                                        };

                                        view! {
                                            <li class="emulator-item" class:is-installed=is_installed>
                                                <div class="emulator-item-header">
                                                    <span class="emulator-name">{display_name}</span>
                                                    <span class="emulator-status" class:installed=is_installed>
                                                        {if is_installed { "Installed" } else { "Not Installed" }}
                                                    </span>
                                                </div>
                                                <div class="emulator-item-meta">
                                                    {is_retroarch_core.then(|| view! {
                                                        <span class="emulator-badge retroarch">"RetroArch Core"</span>
                                                    })}
                                                    {install_method.clone().map(|method| view! {
                                                        <span class="emulator-badge install-method">{method}</span>
                                                    })}
                                                    {(!is_installed && !can_auto_install).then(|| view! {
                                                        <span class="emulator-badge install-method">"manual"</span>
                                                    })}
                                                    {homepage.clone().map(|url| view! {
                                                        <a class="emulator-homepage" href={url} target="_blank">"Website"</a>
                                                    })}
                                                </div>
                                                {notes.clone().map(|n| view! {
                                                    <div class="emulator-notes">{n}</div>
                                                })}
                                                {firmware_warning}
                                                <div class="emulator-pref-buttons">
                                                    <button
                                                        class="emulator-pref-btn emulator-play-btn"
                                                        class:install=!is_installed
                                                        data-nav-default=if emu_index == 0 { Some("true") } else { None }
                                                        disabled=move || !is_installed && !can_auto_install
                                                        on:click=on_launch
                                                    >
                                                        {action_text}
                                                    </button>
                                                    <Show when=move || is_installed && can_uninstall>
                                                        <button
                                                            class="emulator-pref-btn emulator-uninstall-btn"
                                                            on:click=move |e: web_sys::MouseEvent| {
                                                                e.stop_propagation();
                                                                let window = web_sys::window().unwrap();
                                                                let uninstall_display_name =
                                                                    uninstall_display_name.get_value();
                                                                let confirmed = window
                                                                    .confirm_with_message(&format!(
                                                                        "Uninstall {}?",
                                                                        uninstall_display_name
                                                                    ))
                                                                    .unwrap_or(false);
                                                                if !confirmed {
                                                                    return;
                                                                }

                                                                let emulator_name =
                                                                    uninstall_emulator_name.get_value();
                                                                let platform = stored_platform.get_value();
                                                                let current_game_file = game_file.get_untracked();
                                                                let display_name = uninstall_display_name;
                                                                set_progress_state.set(Some(format!(
                                                                    "Uninstalling {}...",
                                                                    display_name
                                                                )));
                                                                set_error_state.set(None);
                                                                set_success_state.set(None);

                                                                spawn_local(async move {
                                                                    match backend_api::uninstall_emulator(
                                                                        emulator_name,
                                                                        Some(platform.clone()),
                                                                        is_retroarch_core,
                                                                    )
                                                                    .await
                                                                    {
                                                                        Ok(()) => match backend_api::get_emulators_with_status(platform.clone()).await {
                                                                            Ok(emu_list) => {
                                                                                set_emulators.set(filter_emulators_for_game(
                                                                                    &platform,
                                                                                    current_game_file.as_ref(),
                                                                                    emu_list,
                                                                                ));
                                                                                set_progress_state.set(None);
                                                                                set_success_state.set(Some(format!(
                                                                                    "Uninstalled {}.",
                                                                                    display_name
                                                                                )));
                                                                            }
                                                                            Err(e) => {
                                                                                set_progress_state.set(None);
                                                                                set_error_state.set(Some(format!(
                                                                                    "Uninstalled {}, but emulator status refresh failed: {}",
                                                                                    display_name, e
                                                                                )));
                                                                            }
                                                                        },
                                                                        Err(e) => {
                                                                            set_progress_state.set(None);
                                                                            set_error_state.set(Some(format!(
                                                                                "Uninstall failed: {}",
                                                                                e
                                                                            )));
                                                                        }
                                                                    }
                                                                });
                                                            }
                                                        >
                                                            "Uninstall"
                                                        </button>
                                                    </Show>
                                                    <button
                                                        class="emulator-pref-btn"
                                                        class:active=is_preferred
                                                        on:click=on_set_game_pref
                                                    >
                                                        "Always for game"
                                                    </button>
                                                    <button
                                                        class="emulator-pref-btn"
                                                        on:click=on_set_platform_pref
                                                    >
                                                        "Always for platform"
                                                    </button>
                                                    <GameLaunchTemplateEditor
                                                        launchbox_db_id=stored_db_id.get_value()
                                                        platform_name=stored_platform.get_value()
                                                        emulator_name=emu.name.clone()
                                                        is_retroarch_core=is_retroarch_core
                                                    />
                                                </div>
                                            </li>
                                        }
                                    }
                                />
                            </ul>
                        </Show>
                    </Show>
                </div>
            </div>
        </div>
    }
}
