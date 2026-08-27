use std::collections::{BTreeMap, BTreeSet, HashSet};
use std::path::{Component, Path};

use anyhow::{Result, bail};
use serde::{Deserialize, Serialize};

const PLAN_VERSION: u32 = 1;
const OPTICAL_PLAN_KIND: &str = "optical_multidisc";
const EXO_PLAN_KIND: &str = "exo_archive_set";

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct TorrentPlanFile {
    pub index: usize,
    pub filename: String,
    pub byte_size: u64,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct DownloadPlanMember {
    pub index: usize,
    pub torrent_path: String,
    pub target_relative_path: String,
    pub byte_size: u64,
    pub disc_index: Option<u32>,
    pub playlist_entry: bool,
    #[serde(default)]
    pub role: String,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct DownloadPlan {
    pub version: u32,
    pub kind: String,
    pub display_name: String,
    pub playlist_filename: String,
    pub representative_index: usize,
    pub members: Vec<DownloadPlanMember>,
}

impl DownloadPlan {
    pub fn validate(&self) -> Result<()> {
        if self.version != PLAN_VERSION {
            bail!("unsupported download plan version {}", self.version);
        }
        if !matches!(self.kind.as_str(), OPTICAL_PLAN_KIND | EXO_PLAN_KIND) {
            bail!("unsupported download plan kind {}", self.kind);
        }
        if self.display_name.trim().is_empty() {
            bail!("download plan display name is empty");
        }
        if self.kind == OPTICAL_PLAN_KIND
            && (!safe_relative_path(&self.playlist_filename)
                || Path::new(&self.playlist_filename)
                    .extension()
                    .and_then(|extension| extension.to_str())
                    .is_none_or(|extension| !extension.eq_ignore_ascii_case("m3u")))
        {
            bail!("download plan playlist path is not a safe .m3u filename");
        }
        if self.kind == EXO_PLAN_KIND && !self.playlist_filename.is_empty() {
            bail!("eXo archive plan must not contain a playlist filename");
        }
        if self.members.is_empty() {
            bail!("download plan has no members");
        }

        let mut indices = HashSet::new();
        let mut targets = HashSet::new();
        let mut disc_indices = BTreeSet::new();
        let mut representative_is_disc = false;
        for member in &self.members {
            if !indices.insert(member.index) {
                bail!("download plan repeats torrent file index {}", member.index);
            }
            if member.torrent_path.trim().is_empty() || !safe_relative_path(&member.torrent_path) {
                bail!("download plan contains an unsafe torrent path");
            }
            if !safe_relative_path(&member.target_relative_path) {
                bail!("download plan contains an unsafe target path");
            }
            if !targets.insert(member.target_relative_path.to_ascii_lowercase()) {
                bail!("download plan contains colliding target paths");
            }
            if member.playlist_entry {
                let disc_index = member
                    .disc_index
                    .ok_or_else(|| anyhow::anyhow!("playlist entry has no disc index"))?;
                disc_indices.insert(disc_index);
                representative_is_disc |= member.index == self.representative_index;
            }
        }
        if self.kind == OPTICAL_PLAN_KIND {
            if disc_indices.len() < 2 {
                bail!("optical download plan must contain at least two discs");
            }
            if !representative_is_disc {
                bail!("download plan representative is not a playlist entry");
            }
        } else {
            if self.members.len() < 2 {
                bail!("eXo archive plan must contain a primary and metadata archive");
            }
            let mut role_counts = BTreeMap::new();
            for member in &self.members {
                if !matches!(
                    member.role.as_str(),
                    "primary" | "game-data" | "metadata" | "utilities"
                ) {
                    bail!("eXo archive plan contains an unsupported member role");
                }
                *role_counts.entry(member.role.as_str()).or_insert(0_usize) += 1;
            }
            if role_counts.get("primary") != Some(&1) {
                bail!("eXo archive plan must contain exactly one primary archive");
            }
            if role_counts.get("metadata") != Some(&1) {
                bail!("eXo archive plan must contain exactly one metadata archive");
            }
            if role_counts.get("game-data").is_some_and(|count| *count > 1)
                || role_counts.get("utilities").is_some_and(|count| *count > 1)
            {
                bail!("eXo archive plan repeats a dependency role");
            }
            let representative = self
                .members
                .iter()
                .find(|member| member.index == self.representative_index)
                .ok_or_else(|| anyhow::anyhow!("download plan representative is missing"))?;
            if representative.role != "primary" {
                bail!("eXo archive plan representative is not the primary archive");
            }
            if self
                .members
                .iter()
                .any(|member| member.playlist_entry || member.disc_index.is_some())
            {
                bail!("eXo archive plan contains optical-disc metadata");
            }
        }
        Ok(())
    }

    pub fn is_optical_multidisc(&self) -> bool {
        self.kind == OPTICAL_PLAN_KIND
    }

    pub fn is_exo_archive_set(&self) -> bool {
        self.kind == EXO_PLAN_KIND
    }

    pub fn representative_member(&self) -> Option<&DownloadPlanMember> {
        self.members
            .iter()
            .find(|member| member.index == self.representative_index)
    }

    pub fn disc_count(&self) -> usize {
        self.members
            .iter()
            .filter_map(|member| member.playlist_entry.then_some(member.disc_index).flatten())
            .collect::<BTreeSet<_>>()
            .len()
    }

    pub fn total_bytes(&self) -> u64 {
        self.members.iter().fold(0_u64, |total, member| {
            total.saturating_add(member.byte_size)
        })
    }

    pub fn selection(&self) -> Vec<(usize, String)> {
        self.members
            .iter()
            .map(|member| (member.index, member.torrent_path.clone()))
            .collect()
    }

    pub fn playlist_members(&self) -> Vec<&DownloadPlanMember> {
        let mut members = self
            .members
            .iter()
            .filter(|member| member.playlist_entry)
            .collect::<Vec<_>>();
        members.sort_by_key(|member| member.disc_index);
        members
    }
}

#[derive(Clone, Debug, Eq, Ord, PartialEq, PartialOrd)]
struct DiscGroupKey {
    title: String,
    variant_tags: Vec<String>,
}

#[derive(Clone, Debug)]
struct DiscInfo {
    key: DiscGroupKey,
    disc_index: u32,
    playlist_stem: String,
}

pub fn build_optical_plan(
    files: &[TorrentPlanFile],
    selected_index: usize,
) -> Option<DownloadPlan> {
    let selected = files.iter().find(|file| file.index == selected_index)?;
    let selected_extension = file_extension(&selected.filename)?;
    if !is_optical_primary_extension(&selected_extension) {
        return None;
    }
    let selected_info = disc_info_from_torrent_path(&selected.filename)?;

    let all_candidates = files
        .iter()
        .filter_map(|file| {
            let extension = file_extension(&file.filename)?;
            if !is_optical_primary_extension(&extension) {
                return None;
            }
            let info = disc_info_from_torrent_path(&file.filename)?;
            (info.key == selected_info.key).then_some((file, info, extension))
        })
        .collect::<Vec<_>>();

    let selected_extension_candidates = all_candidates
        .iter()
        .filter(|(_, _, extension)| extension == &selected_extension)
        .cloned()
        .collect::<Vec<_>>();
    let selected_extension_disc_count = selected_extension_candidates
        .iter()
        .map(|(_, info, _)| info.disc_index)
        .collect::<BTreeSet<_>>()
        .len();
    let selected_primary_extension = if is_preferred_optical_primary_extension(&selected_extension)
        && selected_extension_disc_count >= 2
    {
        selected_extension.as_str()
    } else {
        ""
    };
    let candidate_pool = if !selected_primary_extension.is_empty() {
        selected_extension_candidates
    } else {
        all_candidates
    };

    let mut best_by_disc: BTreeMap<u32, (&TorrentPlanFile, DiscInfo, String)> = BTreeMap::new();
    for (file, info, extension) in candidate_pool {
        let replace = best_by_disc.get(&info.disc_index).is_none_or(
            |(current_file, _, current_extension)| {
                primary_extension_priority(&extension, selected_primary_extension)
                    < primary_extension_priority(current_extension, selected_primary_extension)
                    || (primary_extension_priority(&extension, selected_primary_extension)
                        == primary_extension_priority(
                            current_extension,
                            selected_primary_extension,
                        )
                        && file.filename < current_file.filename)
            },
        );
        if replace {
            best_by_disc.insert(info.disc_index, (file, info, extension));
        }
    }
    if best_by_disc.len() < 2 {
        return None;
    }

    let representative_index = best_by_disc.values().next()?.0.index;
    let display_name = best_by_disc
        .values()
        .next()
        .map(|(_, info, _)| info.playlist_stem.clone())
        .filter(|name| !name.trim().is_empty())
        .unwrap_or_else(|| "Multi-disc game".to_owned());

    let mut requested = BTreeSet::new();
    let mut disc_indices = BTreeMap::new();
    for (disc_index, (file, info, _)) in &best_by_disc {
        requested.insert(file.index);
        disc_indices.insert(file.index, *disc_index);
        requested.extend(sidecar_indices(files, file, info));
    }
    let requested_files = requested
        .iter()
        .filter_map(|index| files.iter().find(|file| file.index == *index))
        .collect::<Vec<_>>();
    let relative_targets = relative_targets(&requested_files)?;

    let mut members = requested_files
        .into_iter()
        .map(|file| DownloadPlanMember {
            index: file.index,
            torrent_path: normalized_listing_path(&file.filename),
            target_relative_path: relative_targets
                .get(&file.index)
                .cloned()
                .unwrap_or_default(),
            byte_size: file.byte_size,
            disc_index: disc_indices.get(&file.index).copied().or_else(|| {
                disc_info_from_torrent_path(&file.filename).map(|info| info.disc_index)
            }),
            playlist_entry: disc_indices.contains_key(&file.index),
            role: if disc_indices.contains_key(&file.index) {
                "disc".to_owned()
            } else {
                "companion".to_owned()
            },
        })
        .collect::<Vec<_>>();
    members.sort_by(|left, right| {
        left.disc_index
            .cmp(&right.disc_index)
            .then_with(|| right.playlist_entry.cmp(&left.playlist_entry))
            .then_with(|| left.torrent_path.cmp(&right.torrent_path))
    });

    let plan = DownloadPlan {
        version: PLAN_VERSION,
        kind: OPTICAL_PLAN_KIND.to_owned(),
        display_name: display_name.clone(),
        playlist_filename: format!("{}.m3u", safe_filename(&display_name)),
        representative_index,
        members,
    };
    plan.validate().ok()?;
    Some(plan)
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum ExoCollection {
    Dos,
    Win3x,
    Win9x,
}

impl ExoCollection {
    fn primary_directory(self) -> &'static str {
        match self {
            Self::Dos => "eXo/eXoDOS",
            Self::Win3x => "eXo/eXoWin3x",
            Self::Win9x => "eXo/eXoWin9x",
        }
    }

    fn metadata_candidates(self) -> &'static [&'static str] {
        match self {
            Self::Dos => &[
                "Content/!DOS_linux_metadata.zip",
                "Full Release/Content/!DOS_linux_metadata.zip",
                "Linux Patches/eXoDOS/Content/!DOS_linux_metadata.zip",
                "eXo/Linux Patches/eXoDOS/Content/!DOS_linux_metadata.zip",
                "Content/!DOSmetadata.zip",
                "Full Release/Content/!DOSmetadata.zip",
            ],
            Self::Win3x => &["Content/!Win3Xmetadata.zip"],
            Self::Win9x => &["Content/!Win9Xmetadata.zip"],
        }
    }

    fn utility_candidates(self) -> &'static [&'static str] {
        match self {
            Self::Dos => &[
                "eXo/util/utilDOS_linux.zip",
                "Full Release/eXo/util/utilDOS_linux.zip",
                "eXo/util/util.zip",
                "Full Release/eXo/util/util.zip",
                "Linux Patches/eXoDOS/eXo/util/utilDOS_linux.zip",
                "eXo/Linux Patches/eXoDOS/eXo/util/utilDOS_linux.zip",
            ],
            Self::Win3x => &[],
            Self::Win9x => &[
                "eXo/util/utilWin9x.zip",
                "Full Release/eXo/util/utilWin9x.zip",
            ],
        }
    }
}

pub fn build_exo_plan(files: &[TorrentPlanFile], selected_index: usize) -> Option<DownloadPlan> {
    let selected = files.iter().find(|file| file.index == selected_index)?;
    let selected_path = normalized_listing_path(&selected.filename);
    let collection = exo_collection_from_path(&selected_path)?;
    let archive_name = Path::new(&selected_path).file_name()?.to_str()?;
    if !archive_name.to_ascii_lowercase().ends_with(".zip") {
        return None;
    }

    let primary = if is_exo_primary(collection, &selected_path) {
        selected
    } else if collection != ExoCollection::Win9x {
        files.iter().find(|file| {
            is_exo_primary(collection, &file.filename)
                && file_name(&file.filename)
                    .is_some_and(|name| name.eq_ignore_ascii_case(archive_name))
        })?
    } else {
        return None;
    };

    let mut roles = BTreeMap::from([(primary.index, "primary")]);
    if collection == ExoCollection::Dos {
        let companion_suffixes = [
            format!("Content/GameData/eXoDOS/{archive_name}"),
            format!("Full Release/Content/GameData/eXoDOS/{archive_name}"),
        ];
        if let Some(companion) = files.iter().find(|file| {
            companion_suffixes
                .iter()
                .any(|suffix| path_ends_with(&file.filename, suffix))
        }) {
            roles.insert(companion.index, "game-data");
        }
    }
    let metadata = files.iter().find(|file| {
        collection
            .metadata_candidates()
            .iter()
            .any(|suffix| path_ends_with(&file.filename, suffix))
    })?;
    roles.insert(metadata.index, "metadata");
    if matches!(collection, ExoCollection::Dos | ExoCollection::Win9x)
        && let Some(utilities) = files.iter().find(|file| {
            collection
                .utility_candidates()
                .iter()
                .any(|suffix| path_ends_with(&file.filename, suffix))
        })
    {
        roles.insert(utilities.index, "utilities");
    }

    let requested_files = roles
        .keys()
        .filter_map(|index| files.iter().find(|file| file.index == *index))
        .collect::<Vec<_>>();
    let relative_targets = relative_targets(&requested_files)?;
    let mut members = requested_files
        .into_iter()
        .map(|file| {
            Some(DownloadPlanMember {
                index: file.index,
                torrent_path: normalized_listing_path(&file.filename),
                target_relative_path: relative_targets.get(&file.index)?.clone(),
                byte_size: file.byte_size,
                disc_index: None,
                playlist_entry: false,
                role: roles.get(&file.index)?.to_string(),
            })
        })
        .collect::<Option<Vec<_>>>()?;
    members.sort_by_key(|member| exo_role_priority(&member.role));
    let display_name = path_stem(&primary.filename)?;
    let plan = DownloadPlan {
        version: PLAN_VERSION,
        kind: EXO_PLAN_KIND.to_owned(),
        display_name,
        playlist_filename: String::new(),
        representative_index: primary.index,
        members,
    };
    plan.validate().ok()?;
    Some(plan)
}

pub fn exo_primary_priority(path: &str) -> Option<u8> {
    let collection = exo_collection_from_path(path)?;
    if !is_exo_primary(collection, path) {
        return None;
    }
    if collection != ExoCollection::Dos {
        return Some(0);
    }
    let normalized =
        format!("/{}", normalized_listing_path(path).trim_matches('/')).to_ascii_lowercase();
    let prefix = format!("/{}/", collection.primary_directory().to_ascii_lowercase());
    let (_, tail) = normalized.rsplit_once(&prefix)?;
    let parts = tail
        .split('/')
        .filter(|part| !part.is_empty())
        .collect::<Vec<_>>();
    match parts.as_slice() {
        [_archive] => Some(0),
        ["!english", _archive] => Some(1),
        ["!german", _archive] => Some(2),
        ["!polish", _archive] => Some(3),
        ["!spanish", _archive] => Some(4),
        _ => None,
    }
}

fn exo_collection_from_path(path: &str) -> Option<ExoCollection> {
    let normalized =
        format!("/{}/", normalized_listing_path(path).trim_matches('/')).to_ascii_lowercase();
    if normalized.contains("/exo/exodos/") || normalized.contains("/content/gamedata/exodos/") {
        Some(ExoCollection::Dos)
    } else if normalized.contains("/exo/exowin3x/") {
        Some(ExoCollection::Win3x)
    } else if normalized.contains("/exo/exowin9x/") {
        Some(ExoCollection::Win9x)
    } else {
        None
    }
}

fn is_exo_primary(collection: ExoCollection, path: &str) -> bool {
    let normalized = normalized_listing_path(path);
    let normalized_lower = format!("/{}", normalized.trim_matches('/')).to_ascii_lowercase();
    let primary_prefix = format!("/{}/", collection.primary_directory().to_ascii_lowercase());
    let Some((_, tail)) = normalized_lower.rsplit_once(&primary_prefix) else {
        return false;
    };
    let parts = tail
        .split('/')
        .filter(|part| !part.is_empty())
        .collect::<Vec<_>>();
    match collection {
        ExoCollection::Dos => match parts.as_slice() {
            [archive] => archive.ends_with(".zip"),
            [language, archive] => {
                matches!(*language, "!english" | "!german" | "!polish" | "!spanish")
                    && archive.ends_with(".zip")
            }
            _ => false,
        },
        ExoCollection::Win3x => matches!(parts.as_slice(), [archive] if archive.ends_with(".zip")),
        ExoCollection::Win9x => {
            matches!(parts.as_slice(), [_year, archive] if archive.ends_with(".zip"))
        }
    }
}

fn path_ends_with(path: &str, suffix: &str) -> bool {
    let path = normalized_listing_path(path).to_ascii_lowercase();
    let suffix = suffix.replace('\\', "/").to_ascii_lowercase();
    path == suffix || path.ends_with(&format!("/{suffix}"))
}

fn file_name(path: &str) -> Option<&str> {
    path.rsplit(['/', '\\']).next()
}

fn exo_role_priority(role: &str) -> u8 {
    match role {
        "primary" => 0,
        "game-data" => 1,
        "metadata" => 2,
        "utilities" => 3,
        _ => 100,
    }
}

fn relative_targets(files: &[&TorrentPlanFile]) -> Option<BTreeMap<usize, String>> {
    let paths = files
        .iter()
        .map(|file| safe_components(&file.filename))
        .collect::<Option<Vec<_>>>()?;
    let mut common_parent_len = paths.first()?.len().saturating_sub(1);
    for path in paths.iter().skip(1) {
        common_parent_len = (0..common_parent_len.min(path.len().saturating_sub(1)))
            .take_while(|index| paths[0][*index].eq_ignore_ascii_case(&path[*index]))
            .count();
    }
    let targets = files
        .iter()
        .zip(paths)
        .map(|(file, components)| {
            let relative = components[common_parent_len..].join("/");
            (file.index, relative)
        })
        .collect::<BTreeMap<_, _>>();
    let unique = targets
        .values()
        .map(|target| target.to_ascii_lowercase())
        .collect::<HashSet<_>>();
    (unique.len() == targets.len()).then_some(targets)
}

fn safe_components(value: &str) -> Option<Vec<String>> {
    let normalized = value.trim_start_matches("./").replace('\\', "/");
    let mut components = Vec::new();
    for component in Path::new(&normalized).components() {
        match component {
            Component::Normal(component) => {
                let component = component.to_str()?.trim();
                if component.is_empty() {
                    return None;
                }
                components.push(component.to_owned());
            }
            Component::CurDir => {}
            Component::ParentDir | Component::RootDir | Component::Prefix(_) => return None,
        }
    }
    (!components.is_empty()).then_some(components)
}

fn safe_relative_path(value: &str) -> bool {
    safe_components(value).is_some()
}

fn normalized_listing_path(value: &str) -> String {
    value.trim_start_matches("./").replace('\\', "/")
}

fn normalized_key_text(value: &str) -> String {
    value
        .to_ascii_lowercase()
        .chars()
        .map(|character| {
            if character.is_ascii_alphanumeric() {
                character
            } else {
                ' '
            }
        })
        .collect::<String>()
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
}

fn safe_filename(value: &str) -> String {
    let cleaned = value
        .chars()
        .map(|character| match character {
            '/' | '\\' | ':' | '*' | '?' | '"' | '<' | '>' | '|' | '\0' => '_',
            character if character.is_control() => '_',
            character => character,
        })
        .collect::<String>();
    let cleaned = cleaned.trim().trim_matches('.');
    if cleaned.is_empty() {
        "Multi-disc game".to_owned()
    } else {
        cleaned.to_owned()
    }
}

fn file_extension(path: &str) -> Option<String> {
    Path::new(path)
        .extension()
        .and_then(|extension| extension.to_str())
        .map(str::to_ascii_lowercase)
}

fn path_stem(path: &str) -> Option<String> {
    let name = Path::new(path).file_name()?.to_str()?;
    Some(
        name.rsplit_once('.')
            .map(|(stem, _)| stem)
            .unwrap_or(name)
            .to_owned(),
    )
}

fn tags(value: &str) -> Vec<(String, String)> {
    let mut tags = Vec::new();
    for (opening, closing) in [('(', ')'), ('[', ']')] {
        let mut remainder = value;
        while let Some(start) = remainder.find(opening) {
            let after_opening = &remainder[start + opening.len_utf8()..];
            let Some(end) = after_opening.find(closing) else {
                break;
            };
            tags.push((
                after_opening[..end].to_owned(),
                format!("{opening}{}{closing}", &after_opening[..end]),
            ));
            remainder = &after_opening[end + closing.len_utf8()..];
        }
    }
    tags
}

fn disc_info_from_component(component_stem: &str) -> Option<DiscInfo> {
    let base_before_tag = component_stem
        .find(['(', '['])
        .map(|index| component_stem[..index].trim())
        .unwrap_or(component_stem.trim());
    let parsed_tags = tags(component_stem);
    let mut disc_index = None;
    let mut display_tags = Vec::new();
    let mut key_tags = Vec::new();
    for (text, original) in parsed_tags {
        if let Some(index) = parse_disc_index(&text) {
            disc_index = Some(index);
            continue;
        }
        if text.trim().to_ascii_lowercase().starts_with("track ") {
            continue;
        }
        let normalized = normalized_key_text(&text);
        if !normalized.is_empty() {
            key_tags.push(normalized);
            display_tags.push(original);
        }
    }

    let (base_title, disc_index) = if let Some(index) = disc_index {
        (base_before_tag.to_owned(), index)
    } else {
        find_loose_disc_marker(component_stem)?
    };
    let normalized_title = normalized_key_text(&base_title);
    if normalized_title.is_empty() {
        return None;
    }
    key_tags.sort();
    key_tags.dedup();
    let playlist_stem = if display_tags.is_empty() {
        base_title.trim().to_owned()
    } else {
        format!("{} {}", base_title.trim(), display_tags.join(" "))
    };
    Some(DiscInfo {
        key: DiscGroupKey {
            title: normalized_title,
            variant_tags: key_tags,
        },
        disc_index,
        playlist_stem,
    })
}

fn disc_info_from_torrent_path(path: &str) -> Option<DiscInfo> {
    let mut components = safe_components(path)?;
    if let Some(last) = components.last_mut() {
        *last = path_stem(last)?;
    }
    for (index, component) in components.iter().enumerate().rev() {
        if let Some(info) = disc_info_from_component(component) {
            return Some(info);
        }
        if index > 0 && parse_disc_index(component).is_some() {
            let combined = format!("{} ({component})", components[index - 1]);
            if let Some(info) = disc_info_from_component(&combined) {
                return Some(info);
            }
        }
    }
    None
}

fn parse_disc_index(value: &str) -> Option<u32> {
    let lower = value.trim().to_ascii_lowercase();
    let prefix = [
        "disc", "disk", "cd", "side", "part", "volume", "vol", "card",
    ]
    .iter()
    .find(|prefix| lower.starts_with(**prefix))?;
    let remainder = lower[prefix.len()..].trim_start_matches(|character: char| {
        character.is_whitespace() || matches!(character, '#' | '-' | '_' | ':')
    });
    let digits = remainder
        .chars()
        .take_while(char::is_ascii_digit)
        .collect::<String>();
    if !digits.is_empty() {
        return digits.parse::<u32>().ok().filter(|index| *index > 0);
    }
    let token = remainder
        .chars()
        .take_while(char::is_ascii_alphabetic)
        .collect::<String>();
    let roman = match token.as_str() {
        "i" => Some(1),
        "ii" => Some(2),
        "iii" => Some(3),
        "iv" => Some(4),
        "v" => Some(5),
        "vi" => Some(6),
        _ => None,
    };
    if roman.is_some() {
        return roman;
    }
    if token.len() == 1 {
        return Some(u32::from(
            token.as_bytes()[0].to_ascii_lowercase() - b'a' + 1,
        ));
    }
    None
}

fn find_loose_disc_marker(stem: &str) -> Option<(String, u32)> {
    let lower = stem.to_ascii_lowercase();
    for marker in ["disc", "disk", "cd", "side", "part"] {
        let mut search_start = 0;
        while let Some(relative_position) = lower[search_start..].find(marker) {
            let position = search_start + relative_position;
            let previous_is_boundary = position == 0
                || lower[..position]
                    .chars()
                    .next_back()
                    .is_some_and(|character| !character.is_ascii_alphanumeric());
            let marker_end = position + marker.len();
            let next_is_valid = lower[marker_end..].chars().next().is_some_and(|character| {
                character.is_ascii_digit()
                    || character.is_whitespace()
                    || matches!(character, '#' | '-' | '_' | ':')
            });
            if previous_is_boundary
                && next_is_valid
                && let Some(disc_index) = parse_disc_index(&stem[position..])
            {
                let base = stem[..position]
                    .trim_end_matches(|character: char| {
                        character.is_whitespace() || matches!(character, '-' | '_' | ',' | ':')
                    })
                    .trim();
                if !base.is_empty() {
                    return Some((base.to_owned(), disc_index));
                }
            }
            search_start = marker_end;
        }
    }
    None
}

fn is_optical_primary_extension(extension: &str) -> bool {
    matches!(
        extension,
        "cue"
            | "chd"
            | "ccd"
            | "mds"
            | "gdi"
            | "pbp"
            | "iso"
            | "cso"
            | "bin"
            | "img"
            | "zip"
            | "7z"
            | "rar"
    )
}

fn is_preferred_optical_primary_extension(extension: &str) -> bool {
    matches!(
        extension,
        "cue" | "chd" | "ccd" | "mds" | "gdi" | "pbp" | "iso" | "cso"
    )
}

fn primary_extension_priority(extension: &str, selected_extension: &str) -> u8 {
    if extension == selected_extension {
        return 0;
    }
    match extension {
        "chd" => 1,
        "cue" => 2,
        "ccd" => 3,
        "mds" => 4,
        "gdi" => 5,
        "pbp" => 6,
        "iso" | "cso" => 7,
        "bin" | "img" => 8,
        "zip" | "7z" | "rar" => 9,
        _ => 100,
    }
}

fn sidecar_extensions(primary_extension: &str) -> &'static [&'static str] {
    match primary_extension {
        "cue" => &["bin", "wav", "flac", "ape", "ogg", "mp3", "aif", "aiff"],
        "ccd" => &["img", "sub", "cue"],
        "mds" => &["mdf"],
        "gdi" => &["bin", "raw"],
        _ => &[],
    }
}

fn path_without_extension(path: &str) -> String {
    let normalized = normalized_listing_path(path).to_ascii_lowercase();
    normalized
        .rsplit_once('.')
        .map(|(base, _)| base.to_owned())
        .unwrap_or(normalized)
}

fn sidecar_indices(
    files: &[TorrentPlanFile],
    primary: &TorrentPlanFile,
    primary_info: &DiscInfo,
) -> Vec<usize> {
    let Some(primary_extension) = file_extension(&primary.filename) else {
        return Vec::new();
    };
    let extensions = sidecar_extensions(&primary_extension);
    if extensions.is_empty() {
        return Vec::new();
    }
    let primary_stem = path_without_extension(&primary.filename);
    files
        .iter()
        .filter(|file| file.index != primary.index)
        .filter(|file| {
            file_extension(&file.filename)
                .is_some_and(|extension| extensions.contains(&extension.as_str()))
        })
        .filter(|file| {
            let candidate_stem = path_without_extension(&file.filename);
            if matches!(primary_extension.as_str(), "ccd" | "mds") && candidate_stem == primary_stem
            {
                return true;
            }
            disc_info_from_torrent_path(&file.filename).is_some_and(|candidate| {
                candidate.key == primary_info.key && candidate.disc_index == primary_info.disc_index
            })
        })
        .map(|file| file.index)
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn file(index: usize, filename: &str, byte_size: u64) -> TorrentPlanFile {
        TorrentPlanFile {
            index,
            filename: filename.to_owned(),
            byte_size,
        }
    }

    #[test]
    fn plans_a_multidisc_cue_set_with_every_track_sidecar() {
        let files = vec![
            file(10, "Sony/Final Fantasy VII (USA) (Disc 1).cue", 100),
            file(
                11,
                "Sony/Final Fantasy VII (USA) (Disc 1) (Track 01).bin",
                1_000,
            ),
            file(12, "Sony/Final Fantasy VII (USA) (Disc 2).cue", 200),
            file(
                13,
                "Sony/Final Fantasy VII (USA) (Disc 2) (Track 01).bin",
                2_000,
            ),
            file(14, "Sony/Final Fantasy VIII (USA) (Disc 1).cue", 300),
        ];
        let plan = build_optical_plan(&files, 10).expect("multi-disc plan");
        assert_eq!(plan.display_name, "Final Fantasy VII (USA)");
        assert_eq!(plan.playlist_filename, "Final Fantasy VII (USA).m3u");
        assert_eq!(plan.disc_count(), 2);
        assert_eq!(plan.total_bytes(), 3_300);
        assert_eq!(
            plan.members
                .iter()
                .map(|member| member.index)
                .collect::<BTreeSet<_>>(),
            BTreeSet::from([10, 11, 12, 13])
        );
        assert_eq!(
            plan.playlist_members()
                .iter()
                .map(|member| member.index)
                .collect::<Vec<_>>(),
            [10, 12]
        );
        assert_eq!(
            plan.members[0].target_relative_path,
            "Final Fantasy VII (USA) (Disc 1).cue"
        );
        plan.validate().unwrap();
    }

    #[test]
    fn prefers_the_selected_format_across_all_discs() {
        let files = vec![
            file(1, "Game (Disc 1).chd", 10),
            file(2, "Game (Disc 2).chd", 20),
            file(3, "Game (Disc 1).cue", 30),
            file(4, "Game (Disc 2).cue", 40),
            file(5, "Game (Disc 1).bin", 50),
            file(6, "Game (Disc 2).bin", 60),
        ];
        let plan = build_optical_plan(&files, 3).expect("cue plan");
        assert_eq!(
            plan.playlist_members()
                .iter()
                .map(|member| member.index)
                .collect::<Vec<_>>(),
            [3, 4]
        );
        assert_eq!(
            plan.members
                .iter()
                .map(|member| member.index)
                .collect::<BTreeSet<_>>(),
            BTreeSet::from([3, 4, 5, 6])
        );
    }

    #[test]
    fn preserves_nested_disc_layout_below_the_common_source_parent() {
        let files = vec![
            file(1, "Collection/Game/Disc 1/game.cue", 10),
            file(2, "Collection/Game/Disc 1/track.bin", 20),
            file(3, "Collection/Game/Disc 2/game.cue", 30),
            file(4, "Collection/Game/Disc 2/track.bin", 40),
        ];
        let plan = build_optical_plan(&files, 1).expect("nested plan");
        assert_eq!(
            plan.members
                .iter()
                .map(|member| member.target_relative_path.as_str())
                .collect::<BTreeSet<_>>(),
            BTreeSet::from([
                "Disc 1/game.cue",
                "Disc 1/track.bin",
                "Disc 2/game.cue",
                "Disc 2/track.bin",
            ])
        );
    }

    #[test]
    fn rejects_single_disc_and_unrelated_variants() {
        let files = vec![
            file(1, "Game (USA) (Disc 1).chd", 10),
            file(2, "Game (Japan) (Disc 2).chd", 20),
        ];
        assert!(build_optical_plan(&files, 1).is_none());
    }

    #[test]
    fn parses_loose_and_roman_disc_markers() {
        assert_eq!(parse_disc_index("Disc I"), Some(1));
        assert_eq!(
            disc_info_from_component("Game - CD II").map(|info| info.disc_index),
            Some(2)
        );
        assert_eq!(parse_disc_index("Volume V"), Some(5));
        assert_eq!(
            disc_info_from_component("Game [Side B]").map(|info| info.disc_index),
            Some(2)
        );
    }

    #[test]
    fn plans_exact_exodos_archives_and_shared_dependencies() {
        let files = vec![
            file(1, "Content/GameData/eXoDOS/Prince of Persia (1990).zip", 10),
            file(2, "Content/!DOSmetadata.zip", 20),
            file(3, "eXo/eXoDOS/Prince of Persia (1990).zip", 30),
            file(4, "eXo/util/util.zip", 40),
        ];
        let plan = build_exo_plan(&files, 3).expect("eXoDOS plan");
        assert!(plan.is_exo_archive_set());
        assert_eq!(plan.representative_index, 3);
        assert_eq!(plan.display_name, "Prince of Persia (1990)");
        assert_eq!(plan.total_bytes(), 100);
        assert_eq!(
            plan.members
                .iter()
                .map(|member| (member.index, member.role.as_str()))
                .collect::<Vec<_>>(),
            vec![
                (3, "primary"),
                (1, "game-data"),
                (2, "metadata"),
                (4, "utilities"),
            ]
        );
        let mut invalid = plan.clone();
        invalid.members[1].role = "primary".to_owned();
        assert!(invalid.validate().is_err());
    }

    #[test]
    fn plans_exodos_language_archive_with_full_release_dependencies() {
        let files = vec![
            file(
                1,
                "Full Release/Content/GameData/eXoDOS/11th Hour, The (1995).zip",
                10,
            ),
            file(2, "Full Release/Content/!DOSmetadata.zip", 20),
            file(4, "Full Release/eXo/util/util.zip", 40),
            file(
                5,
                "German Language Pack/eXo/eXoDOS/!german/11th Hour, The (1995).zip",
                30,
            ),
        ];
        let plan = build_exo_plan(&files, 5).expect("language plan");
        assert_eq!(plan.representative_index, 5);
        assert_eq!(exo_primary_priority(&files[3].filename), Some(2));
        assert_eq!(
            plan.members
                .iter()
                .map(|member| member.index)
                .collect::<BTreeSet<_>>(),
            BTreeSet::from([1, 2, 4, 5])
        );
    }

    #[test]
    fn plans_exowin3x_and_exowin9x_dependencies() {
        let win3x = vec![
            file(4, "Content/!Win3Xmetadata.zip", 20),
            file(
                7,
                "eXo/eXoWin3x/3-D Ultra Pinball - Creep Night (1996).zip",
                30,
            ),
        ];
        let win3x_plan = build_exo_plan(&win3x, 7).expect("Win3x plan");
        assert_eq!(win3x_plan.selection().len(), 2);

        let win9x = vec![
            file(10, "Content/!Win9Xmetadata.zip", 20),
            file(11, "eXo/eXoWin9x/1995/3-D Ultra Pinball (1995).zip", 30),
            file(12, "eXo/util/utilWin9x.zip", 40),
        ];
        let win9x_plan = build_exo_plan(&win9x, 11).expect("Win9x plan");
        assert_eq!(exo_primary_priority(&win9x[1].filename), Some(0));
        assert_eq!(
            win9x_plan
                .members
                .iter()
                .map(|member| member.role.as_str())
                .collect::<Vec<_>>(),
            vec!["primary", "metadata", "utilities"]
        );
    }

    #[test]
    fn rejects_exo_extras_and_incomplete_archive_sets() {
        let files = vec![
            file(
                1,
                "eXo/eXoWin9x/1995/Extras/3-D Ultra Pinball (1995).zip",
                10,
            ),
            file(2, "Content/!Win9Xmetadata.zip", 20),
        ];
        assert!(build_exo_plan(&files, 1).is_none());

        let missing_metadata = vec![file(3, "eXo/eXoDOS/Prince of Persia (1990).zip", 30)];
        assert!(build_exo_plan(&missing_metadata, 3).is_none());
    }

    #[test]
    fn persisted_optical_version_one_plans_without_roles_remain_valid() {
        let encoded = r#"{
            "version":1,
            "kind":"optical_multidisc",
            "display_name":"Game",
            "playlist_filename":"Game.m3u",
            "representative_index":1,
            "members":[
                {"index":1,"torrent_path":"Game (Disc 1).chd","target_relative_path":"Game (Disc 1).chd","byte_size":10,"disc_index":1,"playlist_entry":true},
                {"index":2,"torrent_path":"Game (Disc 2).chd","target_relative_path":"Game (Disc 2).chd","byte_size":20,"disc_index":2,"playlist_entry":true}
            ]
        }"#;
        let plan: DownloadPlan = serde_json::from_str(encoded).unwrap();
        assert!(plan.members.iter().all(|member| member.role.is_empty()));
        plan.validate().unwrap();
    }
}
