use std::collections::{BTreeMap, BTreeSet};
use std::path::Path;

use crate::download_plan::{
    ARCADE_DAPHNE_PLAN_KIND, ARCADE_HYPSEUS_PLAN_KIND, ARCADE_MAME_PLAN_KIND, DownloadPlan,
    DownloadPlanMember, TorrentPlanFile,
};

const PLAN_VERSION: u32 = 1;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum MediaKind {
    Framefile,
    Data,
    Video,
    Audio,
}

#[derive(Clone, Debug)]
struct MediaAsset<'a> {
    file: &'a TorrentPlanFile,
    kind: MediaKind,
    package_path: String,
    game_key: String,
    tail: String,
}

#[derive(Clone, Debug)]
struct PackageFile<'a> {
    file: &'a TorrentPlanFile,
    package_path: String,
    file_name: String,
    stem: String,
}

pub fn build_mame_laserdisc_plans(
    files: &[TorrentPlanFile],
    game_title: &str,
    romset_names: &[String],
) -> Vec<DownloadPlan> {
    let requested = romset_names
        .iter()
        .map(|name| name.to_ascii_lowercase())
        .collect::<BTreeSet<_>>();
    let mut roms = BTreeMap::new();
    let mut chds = BTreeMap::new();
    for file in files {
        let Some((romset, kind)) = parse_mame_asset(&file.filename) else {
            continue;
        };
        if !requested.contains(&romset) {
            continue;
        }
        match kind {
            "rom" => {
                roms.entry(romset).or_insert(file);
            }
            "chd" => {
                chds.entry(romset).or_insert(file);
            }
            _ => {}
        }
    }

    requested
        .into_iter()
        .filter_map(|romset| {
            let rom = *roms.get(&romset)?;
            let chd = *chds.get(&romset)?;
            let chd_name = file_name(&chd.filename)?;
            let plan = DownloadPlan {
                version: PLAN_VERSION,
                kind: ARCADE_MAME_PLAN_KIND.to_owned(),
                display_name: format!("{game_title} · MAME {romset}"),
                playlist_filename: String::new(),
                representative_index: rom.index,
                members: vec![
                    plan_member(rom, format!("MAME/roms/{romset}.zip"), "mame-rom"),
                    plan_member(chd, format!("MAME/roms/{romset}/{chd_name}"), "mame-chd"),
                ],
            };
            plan.validate().ok().map(|()| plan)
        })
        .collect()
}

pub fn build_hypseus_laserdisc_plans(
    files: &[TorrentPlanFile],
    game_title: &str,
    romset_names: &[String],
) -> Vec<DownloadPlan> {
    build_framefile_plans(files, game_title, romset_names, MachineLayout::Hypseus)
}

pub fn build_daphne_laserdisc_plans(
    files: &[TorrentPlanFile],
    game_title: &str,
    romset_names: &[String],
) -> Vec<DownloadPlan> {
    build_framefile_plans(files, game_title, romset_names, MachineLayout::Daphne)
}

#[derive(Clone, Copy)]
enum MachineLayout {
    Hypseus,
    Daphne,
}

impl MachineLayout {
    fn kind(self) -> &'static str {
        match self {
            Self::Hypseus => ARCADE_HYPSEUS_PLAN_KIND,
            Self::Daphne => ARCADE_DAPHNE_PLAN_KIND,
        }
    }

    fn role_prefix(self) -> &'static str {
        match self {
            Self::Hypseus => "hypseus",
            Self::Daphne => "daphne",
        }
    }

    fn target_prefix(self) -> &'static str {
        match self {
            Self::Hypseus => "Laserdisc Collection/Hypseus Singe",
            Self::Daphne => "Laserdisc Collection/Hypseus Singe/Daphne",
        }
    }

    fn display_name(self) -> &'static str {
        match self {
            Self::Hypseus => "Hypseus",
            Self::Daphne => "Daphne compatible",
        }
    }
}

fn build_framefile_plans(
    files: &[TorrentPlanFile],
    game_title: &str,
    romset_names: &[String],
    layout: MachineLayout,
) -> Vec<DownloadPlan> {
    let mut groups: BTreeMap<String, Vec<MediaAsset<'_>>> = BTreeMap::new();
    let mut package_roms: BTreeMap<String, Vec<PackageFile<'_>>> = BTreeMap::new();
    let mut package_ram: BTreeMap<String, Vec<PackageFile<'_>>> = BTreeMap::new();

    for file in files {
        if let Some(asset) = parse_media_asset(file, layout) {
            let group = group_key(&asset.package_path, &asset.game_key);
            groups.entry(group).or_default().push(asset);
            continue;
        }
        if let Some(package_file) = parse_package_file(file, layout, "roms", "zip") {
            package_roms
                .entry(package_file.package_path.to_ascii_lowercase())
                .or_default()
                .push(package_file);
            continue;
        }
        if matches!(layout, MachineLayout::Daphne)
            && let Some(package_file) = parse_package_file(file, layout, "ram", "gz")
        {
            package_ram
                .entry(package_file.package_path.to_ascii_lowercase())
                .or_default()
                .push(package_file);
        }
    }

    let mut plans = Vec::new();
    for assets in groups.into_values() {
        let Some(first) = assets.first() else {
            continue;
        };
        if !key_matches(&first.game_key, romset_names) {
            continue;
        }
        let package_key = first.package_path.to_ascii_lowercase();
        let Some(rom) = package_roms
            .get(&package_key)
            .and_then(|roms| select_rom(roms, &first.game_key, romset_names))
        else {
            continue;
        };
        let Some(framefile) = select_framefile(&assets) else {
            continue;
        };
        if !has_kind(&assets, MediaKind::Data)
            || !has_kind(&assets, MediaKind::Video)
            || !has_kind(&assets, MediaKind::Audio)
        {
            continue;
        }

        let package_target = normalized_target_path(&first.package_path);
        let target_prefix = format!("{}/{package_target}", layout.target_prefix());
        let mut members = Vec::new();
        members.push(plan_member(
            rom.file,
            format!(
                "{target_prefix}/roms/{}",
                sanitize_target_component(&rom.file_name)
            ),
            &format!("{}-rom", layout.role_prefix()),
        ));
        for asset in &assets {
            if asset.kind == MediaKind::Framefile && asset.file.index != framefile.file.index {
                continue;
            }
            let role = match asset.kind {
                MediaKind::Framefile => "framefile",
                MediaKind::Data => "data",
                MediaKind::Video => "video",
                MediaKind::Audio => "audio",
            };
            let mut target_tail = normalized_target_path(&asset.tail);
            if asset.file.index == framefile.file.index {
                let frame_name = format!("{}.txt", rom.stem);
                target_tail = Path::new(&target_tail)
                    .parent()
                    .map(|parent| parent.join(&frame_name))
                    .unwrap_or_else(|| frame_name.into())
                    .to_string_lossy()
                    .replace('\\', "/");
            }
            members.push(plan_member(
                asset.file,
                format!("{target_prefix}/vldp/{target_tail}"),
                &format!("{}-{role}", layout.role_prefix()),
            ));
        }
        if matches!(layout, MachineLayout::Daphne)
            && let Some(ram_files) = package_ram.get(&package_key)
        {
            for ram in ram_files
                .iter()
                .filter(|ram| stem_belongs_to_game(&ram.stem, &first.game_key))
            {
                members.push(plan_member(
                    ram.file,
                    format!(
                        "{target_prefix}/ram/{}",
                        sanitize_target_component(&ram.file_name)
                    ),
                    "daphne-ram",
                ));
            }
        }
        members.sort_by(|left, right| {
            role_priority(&left.role)
                .cmp(&role_priority(&right.role))
                .then_with(|| left.target_relative_path.cmp(&right.target_relative_path))
        });
        members.dedup_by_key(|member| member.index);
        let package_label = first
            .package_path
            .rsplit('/')
            .next()
            .unwrap_or(&first.package_path);
        let plan = DownloadPlan {
            version: PLAN_VERSION,
            kind: layout.kind().to_owned(),
            display_name: format!("{game_title} · {} {package_label}", layout.display_name()),
            playlist_filename: String::new(),
            representative_index: framefile.file.index,
            members,
        };
        if plan.validate().is_ok() {
            plans.push(plan);
        }
    }
    plans.sort_by(|left, right| {
        left.total_bytes()
            .cmp(&right.total_bytes())
            .then_with(|| left.display_name.cmp(&right.display_name))
    });
    plans
}

fn parse_mame_asset(path: &str) -> Option<(String, &'static str)> {
    let components = safe_components(path)?;
    let laserdisc = component_index(&components, "Laserdisc Collection")?;
    if !component_eq(components.get(laserdisc + 1)?, "MAME") {
        return None;
    }
    if component_eq(components.get(laserdisc + 2)?, "ROMs") && components.len() == laserdisc + 4 {
        let file_name = components.get(laserdisc + 3)?;
        if !Path::new(file_name)
            .extension()
            .and_then(|value| value.to_str())
            .is_some_and(|value| value.eq_ignore_ascii_case("zip"))
        {
            return None;
        }
        let stem = Path::new(file_name).file_stem()?.to_str()?;
        return (!stem.is_empty()).then(|| (stem.to_ascii_lowercase(), "rom"));
    }
    if component_eq(components.get(laserdisc + 2)?, "CHD") && components.len() == laserdisc + 5 {
        let romset = components.get(laserdisc + 3)?;
        let file_name = components.get(laserdisc + 4)?;
        if file_name.to_ascii_lowercase().ends_with(".chd") {
            return Some((romset.to_ascii_lowercase(), "chd"));
        }
    }
    None
}

fn parse_media_asset<'a>(
    file: &'a TorrentPlanFile,
    layout: MachineLayout,
) -> Option<MediaAsset<'a>> {
    let components = safe_components(&file.filename)?;
    let laserdisc = component_index(&components, "Laserdisc Collection")?;
    let platform_component = components.get(laserdisc + 1)?;
    match layout {
        MachineLayout::Hypseus if !component_eq(platform_component, "Hypseus Singe") => {
            return None;
        }
        MachineLayout::Daphne if !component_eq(platform_component, "Daphne") => return None,
        _ => {}
    }
    let package_start = laserdisc + 2;
    let anchor =
        components
            .iter()
            .enumerate()
            .skip(package_start)
            .find_map(|(index, component)| match layout {
                MachineLayout::Hypseus if component_eq(component, "vldp") => Some(index),
                MachineLayout::Daphne
                    if component_eq(component, "vldp_dl") || component_eq(component, "mpeg2") =>
                {
                    Some(index)
                }
                _ => None,
            })?;
    if anchor <= package_start || anchor + 2 >= components.len() {
        return None;
    }
    let package_path = components[package_start..anchor].join("/");
    let game_key = components.get(anchor + 1)?.to_ascii_lowercase();
    let tail = components[anchor + 1..].join("/");
    let extension = Path::new(components.last()?)
        .extension()?
        .to_str()?
        .to_ascii_lowercase();
    let kind = match extension.as_str() {
        "txt" => MediaKind::Framefile,
        "dat" => MediaKind::Data,
        "m2v" => MediaKind::Video,
        "ogg" | "wav" | "mp3" | "flac" => MediaKind::Audio,
        _ => return None,
    };
    Some(MediaAsset {
        file,
        kind,
        package_path,
        game_key,
        tail,
    })
}

fn parse_package_file<'a>(
    file: &'a TorrentPlanFile,
    layout: MachineLayout,
    directory: &str,
    extension: &str,
) -> Option<PackageFile<'a>> {
    let components = safe_components(&file.filename)?;
    let laserdisc = component_index(&components, "Laserdisc Collection")?;
    let platform_component = components.get(laserdisc + 1)?;
    match layout {
        MachineLayout::Hypseus if !component_eq(platform_component, "Hypseus Singe") => {
            return None;
        }
        MachineLayout::Daphne if !component_eq(platform_component, "Daphne") => return None,
        _ => {}
    }
    let directory_index = components
        .iter()
        .enumerate()
        .skip(laserdisc + 2)
        .find_map(|(index, component)| component_eq(component, directory).then_some(index))?;
    if directory_index <= laserdisc + 2 || components.len() != directory_index + 2 {
        return None;
    }
    let file_name = components.last()?.clone();
    if !Path::new(&file_name)
        .extension()
        .and_then(|value| value.to_str())
        .is_some_and(|value| value.eq_ignore_ascii_case(extension))
    {
        return None;
    }
    let stem = Path::new(&file_name).file_stem()?.to_str()?.to_owned();
    Some(PackageFile {
        file,
        package_path: components[laserdisc + 2..directory_index].join("/"),
        file_name,
        stem: stem.to_ascii_lowercase(),
    })
}

fn select_framefile<'a>(assets: &'a [MediaAsset<'a>]) -> Option<&'a MediaAsset<'a>> {
    assets.iter().find(|asset| {
        asset.kind == MediaKind::Framefile
            && Path::new(&asset.tail)
                .file_stem()
                .and_then(|value| value.to_str())
                .is_some_and(|stem| stem.eq_ignore_ascii_case(&asset.game_key))
    })
}

fn select_rom<'a>(
    roms: &'a [PackageFile<'a>],
    game_key: &str,
    aliases: &[String],
) -> Option<&'a PackageFile<'a>> {
    roms.iter()
        .min_by_key(|rom| {
            if rom.stem.eq_ignore_ascii_case(game_key) {
                (0_u8, rom.file_name.len(), rom.file_name.as_str())
            } else if aliases
                .iter()
                .any(|alias| rom.stem.eq_ignore_ascii_case(alias))
            {
                (1, rom.file_name.len(), rom.file_name.as_str())
            } else if key_matches(&rom.stem, aliases) {
                (2, rom.file_name.len(), rom.file_name.as_str())
            } else {
                (u8::MAX, rom.file_name.len(), rom.file_name.as_str())
            }
        })
        .filter(|rom| {
            rom.stem.eq_ignore_ascii_case(game_key)
                || key_matches(&rom.stem, aliases)
                || stem_belongs_to_game(&rom.stem, game_key)
        })
}

fn key_matches(key: &str, aliases: &[String]) -> bool {
    let key = key.to_ascii_lowercase();
    aliases.iter().any(|alias| {
        let alias = alias.to_ascii_lowercase();
        key == alias
            || (key.len() >= 4
                && alias.len() >= 4
                && (key.contains(&alias) || alias.contains(&key)))
    })
}

fn stem_belongs_to_game(stem: &str, game_key: &str) -> bool {
    stem == game_key || stem.starts_with(&format!("{game_key}_"))
}

fn has_kind(assets: &[MediaAsset<'_>], expected: MediaKind) -> bool {
    assets.iter().any(|asset| asset.kind == expected)
}

fn plan_member(
    file: &TorrentPlanFile,
    target_relative_path: String,
    role: &str,
) -> DownloadPlanMember {
    DownloadPlanMember {
        index: file.index,
        torrent_path: file.filename.clone(),
        target_relative_path,
        byte_size: file.byte_size,
        disc_index: None,
        playlist_entry: false,
        role: role.to_owned(),
    }
}

fn safe_components(path: &str) -> Option<Vec<String>> {
    let components = path
        .split('/')
        .filter(|component| !component.is_empty())
        .map(str::to_owned)
        .collect::<Vec<_>>();
    (!components.is_empty()
        && components
            .iter()
            .all(|component| !component.is_empty() && component != "." && component != ".."))
    .then_some(components)
}

fn component_index(components: &[String], expected: &str) -> Option<usize> {
    components
        .iter()
        .position(|component| component_eq(component, expected))
}

fn component_eq(component: &str, expected: &str) -> bool {
    component.eq_ignore_ascii_case(expected)
}

fn group_key(package_path: &str, game_key: &str) -> String {
    format!(
        "{}/{}",
        package_path.to_ascii_lowercase(),
        game_key.to_ascii_lowercase()
    )
}

fn normalized_target_path(path: &str) -> String {
    path.replace('\\', "/")
        .split('/')
        .filter(|component| !component.is_empty())
        .map(sanitize_target_component)
        .collect::<Vec<_>>()
        .join("/")
}

fn sanitize_target_component(value: &str) -> String {
    let sanitized = value
        .chars()
        .map(|character| match character {
            '<' | '>' | ':' | '"' | '/' | '\\' | '|' | '?' | '*' | '\0' => '_',
            character if character.is_control() => '_',
            character => character,
        })
        .collect::<String>();
    let sanitized = sanitized.trim().trim_end_matches(['.', ' ']);
    if sanitized.is_empty() {
        "unnamed".to_owned()
    } else {
        sanitized.to_owned()
    }
}

fn file_name(path: &str) -> Option<&str> {
    path.rsplit(['/', '\\']).next()
}

fn role_priority(role: &str) -> u8 {
    if role.ends_with("-rom") {
        0
    } else if role.ends_with("-framefile") {
        1
    } else if role.ends_with("-data") {
        2
    } else if role.ends_with("-video") {
        3
    } else if role.ends_with("-audio") {
        4
    } else {
        5
    }
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
    fn mame_plan_pairs_rom_and_chd_into_launchable_layout() {
        let files = vec![
            file(
                1,
                "Minerva_Myrient/Laserdisc Collection/MAME/ROMs/dlair.zip",
                10,
            ),
            file(
                2,
                "Minerva_Myrient/Laserdisc Collection/MAME/CHD/dlair/dlair.chd",
                20,
            ),
            file(
                3,
                "Minerva_Myrient/Laserdisc Collection/MAME/ROMs/other.zip",
                5,
            ),
        ];
        let plans = build_mame_laserdisc_plans(&files, "Dragon's Lair", &["dlair".into()]);
        assert_eq!(plans.len(), 1);
        assert_eq!(plans[0].representative_index, 1);
        assert_eq!(plans[0].total_bytes(), 30);
        assert_eq!(
            plans[0].members[0].target_relative_path,
            "MAME/roms/dlair.zip"
        );
        assert_eq!(
            plans[0].members[1].target_relative_path,
            "MAME/roms/dlair/dlair.chd"
        );
    }

    #[test]
    fn hypseus_plan_uses_real_minerva_vldp_shape() {
        let prefix = "Minerva_Myrient/Laserdisc Collection/Hypseus Singe/Singe1";
        let files = vec![
            file(1, &format!("{prefix}/roms/lair.zip"), 10),
            file(2, &format!("{prefix}/vldp/lair/lair.dat"), 20),
            file(3, &format!("{prefix}/vldp/lair/lair.m2v"), 30),
            file(4, &format!("{prefix}/vldp/lair/lair.ogg"), 40),
            file(5, &format!("{prefix}/vldp/lair/lair.txt"), 5),
            file(6, &format!("{prefix}/vldp/lair/README.txt"), 1),
        ];
        let plans = build_hypseus_laserdisc_plans(&files, "Dragon's Lair", &["dlair".into()]);
        assert_eq!(plans.len(), 1);
        assert_eq!(plans[0].representative_index, 5);
        assert_eq!(plans[0].members.len(), 5);
        let frame = plans[0].representative_member().unwrap();
        assert_eq!(
            frame.target_relative_path,
            "Laserdisc Collection/Hypseus Singe/Singe1/vldp/lair/lair.txt"
        );
    }

    #[test]
    fn daphne_plan_normalizes_vldp_dl_and_renames_frame_for_selected_rom() {
        let prefix = "Minerva_Myrient/Laserdisc Collection/Daphne/DaphneLoader";
        let files = vec![
            file(1, &format!("{prefix}/roms/lair.zip"), 10),
            file(2, &format!("{prefix}/vldp_dl/lair/lair.dat"), 20),
            file(3, &format!("{prefix}/vldp_dl/lair/lair.m2v"), 30),
            file(4, &format!("{prefix}/vldp_dl/lair/lair.ogg"), 40),
            file(5, &format!("{prefix}/vldp_dl/lair/lair.txt"), 5),
            file(6, &format!("{prefix}/ram/lair.gz"), 2),
        ];
        let plans = build_daphne_laserdisc_plans(&files, "Dragon's Lair", &["dlair".into()]);
        assert_eq!(plans.len(), 1);
        assert_eq!(plans[0].representative_index, 5);
        assert!(
            plans[0]
                .members
                .iter()
                .any(|member| member.role == "daphne-ram")
        );
        assert_eq!(
            plans[0]
                .representative_member()
                .unwrap()
                .target_relative_path,
            "Laserdisc Collection/Hypseus Singe/Daphne/DaphneLoader/vldp/lair/lair.txt"
        );
    }

    #[test]
    fn incomplete_machine_bundles_are_not_offered() {
        let prefix = "Laserdisc Collection/Hypseus Singe/Singe1";
        let files = vec![
            file(1, &format!("{prefix}/roms/lair.zip"), 10),
            file(2, &format!("{prefix}/vldp/lair/lair.m2v"), 30),
            file(3, &format!("{prefix}/vldp/lair/lair.txt"), 5),
        ];
        assert!(
            build_hypseus_laserdisc_plans(&files, "Dragon's Lair", &["dlair".into()]).is_empty()
        );
    }

    #[test]
    fn persisted_machine_plan_rejects_a_role_with_the_wrong_file_type() {
        let prefix = "Laserdisc Collection/Hypseus Singe/Singe1";
        let files = vec![
            file(1, &format!("{prefix}/roms/lair.zip"), 10),
            file(2, &format!("{prefix}/vldp/lair/lair.dat"), 20),
            file(3, &format!("{prefix}/vldp/lair/lair.m2v"), 30),
            file(4, &format!("{prefix}/vldp/lair/lair.ogg"), 40),
            file(5, &format!("{prefix}/vldp/lair/lair.txt"), 5),
        ];
        let mut plan = build_hypseus_laserdisc_plans(&files, "Dragon's Lair", &["dlair".into()])
            .pop()
            .unwrap();
        plan.members
            .iter_mut()
            .find(|member| member.role == "hypseus-audio")
            .unwrap()
            .target_relative_path =
            "Laserdisc Collection/Hypseus Singe/Singe1/vldp/lair/lair.exe".into();
        assert!(plan.validate().is_err());
    }

    #[test]
    fn source_components_preserve_legal_torrent_whitespace() {
        assert_eq!(
            safe_components("root/ package /disc.m2v ").unwrap(),
            vec!["root", " package ", "disc.m2v "]
        );
        assert_eq!(
            safe_components("root/package\\name/disc.m2v").unwrap(),
            vec!["root", "package\\name", "disc.m2v"]
        );
    }
}
