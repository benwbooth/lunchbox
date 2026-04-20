use crate::state::AppSettings;
use serde::{Deserialize, Serialize};
use sqlx::SqlitePool;
use std::fs::File;
use std::io::Write;
use std::path::{Component, Path, PathBuf};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum ExoCollection {
    Dos,
    Win3x,
    Win9x,
}

impl ExoCollection {
    fn table_name(self) -> &'static str {
        match self {
            Self::Dos => "exodos",
            Self::Win3x => "exowin3x",
            Self::Win9x => "exowin9x",
        }
    }

    fn install_root_name(self) -> &'static str {
        match self {
            Self::Dos => "eXoDOS",
            Self::Win3x => "eXoWin3x",
            Self::Win9x => "eXoWin9x",
        }
    }

    fn metadata_archive_relative_path(self) -> &'static str {
        match self {
            Self::Dos => "Content/!DOSmetadata.zip",
            Self::Win3x => "Content/!Win3Xmetadata.zip",
            Self::Win9x => "Content/!Win9Xmetadata.zip",
        }
    }

    fn primary_archive_relative_dir(self) -> &'static str {
        match self {
            Self::Dos => "eXo/eXoDOS",
            Self::Win3x => "eXo/eXoWin3x",
            Self::Win9x => "eXo/eXoWin9x",
        }
    }

    fn companion_archive_relative_dir(self) -> Option<&'static str> {
        match self {
            Self::Dos => Some("Content/GameData/eXoDOS"),
            Self::Win3x | Self::Win9x => None,
        }
    }

    fn metadata_entry_root(self) -> &'static str {
        match self {
            Self::Dos => "eXo/eXoDOS/!dos",
            Self::Win3x => "eXo/eXoWin3x/!win3x",
            Self::Win9x => "eXo/eXoWin9x/!win9x",
        }
    }

    fn shared_util_archive_candidates(self) -> &'static [&'static str] {
        match self {
            Self::Dos => &[
                "eXo/util/utilDOS_linux.zip",
                "Full Release/eXo/util/utilDOS_linux.zip",
                "eXo/util/util.zip",
                "Full Release/eXo/util/util.zip",
                "Linux Patches/eXoDOS/eXo/util/utilDOS_linux.zip",
                "eXo/Linux Patches/eXoDOS/eXo/util/utilDOS_linux.zip",
            ],
            Self::Win3x => &[
                "eXoDOS/eXo/util/util.zip",
                "eXo/eXoDOS/Full Release/eXo/util/util.zip",
            ],
            Self::Win9x => &[
                "eXo/util/utilWin9x.zip",
                "Full Release/eXo/util/utilWin9x.zip",
            ],
        }
    }

    fn shared_bootstrap_name(self) -> &'static str {
        match self {
            Self::Dos | Self::Win3x => "exodos",
            Self::Win9x => "exowin9x",
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PreparedInstall {
    pub collection: ExoCollection,
    pub install_root: PathBuf,
    pub launch_config_path: PathBuf,
    pub shortname: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DosboxExceptionPlan {
    pub copy_mt32_roms: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LinuxDosboxExceptionPlan {
    pub launch_config_name: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LinuxScummvmExceptionPlan {
    pub config_path: String,
    pub game_path: String,
    pub game_id: String,
    pub extra_args: Vec<String>,
}

#[derive(Debug, Clone)]
pub struct DownloadSelectionPlan {
    pub requested_indices: Vec<usize>,
    pub representative_index: usize,
}

#[derive(Debug, Clone)]
struct ResolvedArchives {
    primary_archive_path: PathBuf,
    companion_archive_path: Option<PathBuf>,
    metadata_archive_path: PathBuf,
    shared_util_archive_path: Option<PathBuf>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum CachedInstallLookup {
    Match,
    Missing,
    Mismatch,
}

const EXODOS_LANGUAGE_BUCKETS: &[(&str, &str, i32)] = &[
    ("!english", "English", 1),
    ("!german", "German", 2),
    ("!polish", "Polish", 3),
    ("!spanish", "Spanish", 4),
];

pub fn collection_for_platform(platform: &str) -> Option<ExoCollection> {
    match platform {
        "MS-DOS" => Some(ExoCollection::Dos),
        "Windows 3.X" => Some(ExoCollection::Win3x),
        "Windows" => Some(ExoCollection::Win9x),
        _ => None,
    }
}

pub fn requires_prepared_install(platform: &str) -> bool {
    matches!(
        collection_for_platform(platform),
        Some(ExoCollection::Dos | ExoCollection::Win3x)
    )
}

pub fn should_use_prepared_install(platform: &str, raw_archive_path: &Path) -> bool {
    let Some(collection) = collection_for_platform(platform) else {
        return false;
    };

    is_primary_download_candidate_for_collection(collection, &raw_archive_path.to_string_lossy())
}

pub fn is_primary_download_candidate(platform: &str, path: &str) -> bool {
    let Some(collection) = collection_for_platform(platform) else {
        return true;
    };

    is_primary_download_candidate_for_collection(collection, path)
}

pub fn primary_download_priority(platform: &str, path: &str) -> i32 {
    let Some(collection) = collection_for_platform(platform) else {
        return 0;
    };

    primary_download_priority_for_collection(collection, path)
}

fn is_primary_download_candidate_for_collection(collection: ExoCollection, path: &str) -> bool {
    let normalized = path.replace('\\', "/");
    let Some(file_name) = Path::new(&normalized)
        .file_name()
        .and_then(|name| name.to_str())
    else {
        return false;
    };

    if !file_name.to_ascii_lowercase().ends_with(".zip") {
        return false;
    }

    let primary_dir = format!("{}/", collection.primary_archive_relative_dir());
    let Some((_, tail)) = normalized.rsplit_once(&primary_dir) else {
        return false;
    };

    match collection {
        ExoCollection::Dos => {
            let parts: Vec<&str> = tail.split('/').filter(|part| !part.is_empty()).collect();
            match parts.as_slice() {
                [file_name] => file_name.to_ascii_lowercase().ends_with(".zip"),
                [language_bucket, file_name] => {
                    exodos_language_bucket(language_bucket).is_some()
                        && file_name.to_ascii_lowercase().ends_with(".zip")
                }
                _ => false,
            }
        }
        ExoCollection::Win3x => !tail.contains('/'),
        ExoCollection::Win9x => {
            let mut parts = tail.split('/').filter(|part| !part.is_empty());
            let Some(_year_dir) = parts.next() else {
                return false;
            };
            let Some(file_name) = parts.next() else {
                return false;
            };
            parts.next().is_none() && file_name.to_ascii_lowercase().ends_with(".zip")
        }
    }
}

fn primary_download_priority_for_collection(collection: ExoCollection, path: &str) -> i32 {
    let normalized = path.replace('\\', "/");
    let primary_dir = format!("{}/", collection.primary_archive_relative_dir());
    let Some((_, tail)) = normalized.rsplit_once(&primary_dir) else {
        return 100;
    };

    match collection {
        ExoCollection::Dos => {
            let parts: Vec<&str> = tail.split('/').filter(|part| !part.is_empty()).collect();
            match parts.as_slice() {
                [_file_name] => 0,
                [language_bucket, _file_name] => exodos_language_bucket(language_bucket)
                    .map(|(_, _, priority)| priority)
                    .unwrap_or(100),
                _ => 100,
            }
        }
        ExoCollection::Win3x | ExoCollection::Win9x => 0,
    }
}

fn exodos_language_bucket(segment: &str) -> Option<(&'static str, &'static str, i32)> {
    EXODOS_LANGUAGE_BUCKETS
        .iter()
        .copied()
        .find(|(bucket, _, _)| segment.eq_ignore_ascii_case(bucket))
}

fn metadata_archive_candidate_paths(collection: ExoCollection) -> Vec<PathBuf> {
    let metadata_relative = PathBuf::from(collection.metadata_archive_relative_path());
    match collection {
        ExoCollection::Dos => vec![
            PathBuf::from("Content/!DOS_linux_metadata.zip"),
            PathBuf::from("Full Release/Content/!DOS_linux_metadata.zip"),
            PathBuf::from("Linux Patches/eXoDOS/Content/!DOS_linux_metadata.zip"),
            PathBuf::from("eXo/Linux Patches/eXoDOS/Content/!DOS_linux_metadata.zip"),
            metadata_relative.clone(),
            PathBuf::from("Full Release").join(metadata_relative),
        ],
        ExoCollection::Win3x | ExoCollection::Win9x => vec![metadata_relative],
    }
}

fn companion_archive_candidate_paths(
    collection: ExoCollection,
    archive_name: &str,
) -> Vec<PathBuf> {
    let Some(dir) = collection.companion_archive_relative_dir() else {
        return Vec::new();
    };

    let relative = PathBuf::from(dir).join(archive_name);
    match collection {
        ExoCollection::Dos => vec![
            relative.clone(),
            PathBuf::from("Full Release").join(relative),
        ],
        ExoCollection::Win3x | ExoCollection::Win9x => vec![relative],
    }
}

fn install_config_candidates(
    collection: ExoCollection,
    install_root: &Path,
    primary_archive_path: &Path,
    shortname: &str,
    metadata_archive_path: &Path,
) -> Vec<PathBuf> {
    let metadata_relative_dir =
        metadata_relative_dir_for_archive(collection, primary_archive_path, shortname);
    match collection {
        ExoCollection::Dos => {
            let base = install_root.join("eXoDOS").join("!dos").join(shortname);
            if is_linux_dos_metadata_archive(metadata_archive_path) {
                vec![base.join("dosbox_linux.conf"), base.join("dosbox.conf")]
            } else {
                vec![base.join("dosbox.conf"), base.join("dosbox_linux.conf")]
            }
        }
        ExoCollection::Win3x => vec![
            install_root
                .join("eXoWin3x")
                .join("!win3x")
                .join(&metadata_relative_dir)
                .join("dosbox.conf"),
        ],
        ExoCollection::Win9x => vec![
            install_root
                .join("eXoWin9x")
                .join("!win9x")
                .join(&metadata_relative_dir)
                .join("Play.conf"),
            install_root
                .join("eXoWin9x")
                .join("!win9x")
                .join(&metadata_relative_dir)
                .join("Play.cfg"),
            install_root
                .join("eXoWin9x")
                .join("!win9x")
                .join(&metadata_relative_dir)
                .join("Host.cfg"),
            install_root
                .join("eXoWin9x")
                .join("!win9x")
                .join(&metadata_relative_dir)
                .join("Join.cfg"),
        ],
    }
}

fn preferred_install_config_path(
    collection: ExoCollection,
    install_root: &Path,
    primary_archive_path: &Path,
    shortname: &str,
    metadata_archive_path: &Path,
) -> PathBuf {
    if collection == ExoCollection::Win9x {
        return preferred_win9x_install_config_path(
            install_root,
            primary_archive_path,
            shortname,
            metadata_archive_path,
        );
    }

    install_config_candidates(
        collection,
        install_root,
        primary_archive_path,
        shortname,
        metadata_archive_path,
    )
    .into_iter()
    .next()
    .expect("install config candidates")
}

fn resolve_existing_install_config_path(
    collection: ExoCollection,
    install_root: &Path,
    primary_archive_path: &Path,
    shortname: &str,
    metadata_archive_path: &Path,
) -> Result<PathBuf, String> {
    install_config_candidates(
        collection,
        install_root,
        primary_archive_path,
        shortname,
        metadata_archive_path,
    )
    .into_iter()
    .find(|candidate| candidate.exists())
    .ok_or_else(|| {
        format!(
            "Prepared eXo install is missing a launch config under {}",
            install_root.display()
        )
    })
}

fn metadata_relative_dir_for_archive(
    collection: ExoCollection,
    primary_archive_path: &Path,
    shortname: &str,
) -> PathBuf {
    if collection != ExoCollection::Win9x {
        return PathBuf::from(shortname);
    }

    let normalized = primary_archive_path.to_string_lossy().replace('\\', "/");
    let primary_dir = format!("{}/", collection.primary_archive_relative_dir());
    let Some((_, tail)) = normalized.rsplit_once(&primary_dir) else {
        return PathBuf::from(shortname);
    };

    let mut parts = tail.split('/').filter(|part| !part.is_empty());
    let Some(year_dir) = parts.next() else {
        return PathBuf::from(shortname);
    };

    PathBuf::from(year_dir).join(shortname)
}

fn preferred_win9x_install_config_path(
    install_root: &Path,
    primary_archive_path: &Path,
    shortname: &str,
    metadata_archive_path: &Path,
) -> PathBuf {
    let metadata_relative_dir =
        metadata_relative_dir_for_archive(ExoCollection::Win9x, primary_archive_path, shortname);
    for config_name in ["Play.conf", "Play.cfg", "Host.cfg", "Join.cfg"] {
        let archive_entry = format!(
            "{}/{}/{}",
            ExoCollection::Win9x.metadata_entry_root(),
            metadata_relative_dir.to_string_lossy().replace('\\', "/"),
            config_name
        );
        if zip_archive_contains_entry(metadata_archive_path, &archive_entry) {
            return install_root
                .join("eXoWin9x")
                .join("!win9x")
                .join(&metadata_relative_dir)
                .join(config_name);
        }
    }

    install_root
        .join("eXoWin9x")
        .join("!win9x")
        .join(&metadata_relative_dir)
        .join("Play.conf")
}

fn zip_archive_contains_entry(archive_path: &Path, entry_name: &str) -> bool {
    let Ok(file) = File::open(archive_path) else {
        return false;
    };
    let Ok(mut archive) = zip::ZipArchive::new(file) else {
        return false;
    };
    let exists = archive.by_name(entry_name).is_ok();
    exists
}

fn is_linux_dos_metadata_archive(path: &Path) -> bool {
    path.file_name()
        .and_then(|name| name.to_str())
        .map(|name| name.eq_ignore_ascii_case("!DOS_linux_metadata.zip"))
        .unwrap_or(false)
}

fn is_linux_dos_util_archive(path: &Path) -> bool {
    path.file_name()
        .and_then(|name| name.to_str())
        .map(|name| name.eq_ignore_ascii_case("utilDOS_linux.zip"))
        .unwrap_or(false)
}

pub fn parse_dosbox_exception_script(contents: &str) -> Result<DosboxExceptionPlan, String> {
    let normalized = contents.replace("\r\n", "\n").replace('\r', "\n");
    let lines: Vec<&str> = normalized.lines().collect();
    let branch_lines = extract_dosbox_branch_lines(&lines);
    let dosbox_marker = r#".\emulators\dosbox\%dosbox%"#;

    if !branch_lines
        .iter()
        .any(|line| line.to_ascii_lowercase().contains(dosbox_marker))
    {
        return Err("exception launcher does not contain a supported DOSBox branch".to_string());
    }

    let mut plan = DosboxExceptionPlan {
        copy_mt32_roms: false,
    };

    for original_line in branch_lines {
        let line = original_line.trim();
        let lower = line.to_ascii_lowercase();

        if lower.is_empty()
            || lower == "echo off"
            || lower == "cls"
            || lower.starts_with("echo.")
            || lower.starts_with("echo ")
            || lower.starts_with("cd ")
            || lower.starts_with("goto ")
            || lower.starts_with(':')
            || lower.contains("setconsole.exe")
            || lower == "del stdout.txt"
            || lower == "del stderr.txt"
            || (lower.starts_with("if exist ") && lower.contains(" del "))
        {
            continue;
        }

        if lower == r#"copy .\mt32\*.rom .\"# {
            plan.copy_mt32_roms = true;
            continue;
        }

        if lower == "del *.rom" {
            continue;
        }

        if lower.contains(dosbox_marker) {
            let normalized_line = lower.trim_start_matches('"');
            if !normalized_line.starts_with(dosbox_marker) {
                return Err(format!(
                    "exception launcher uses an unsupported DOSBox wrapper command: {}",
                    line
                ));
            }
            continue;
        }

        if lower.starts_with("start ") || lower.starts_with("taskkill ") {
            return Err(format!(
                "exception launcher requires helper process management that Lunchbox does not support yet: {}",
                line
            ));
        }

        return Err(format!(
            "exception launcher contains an unsupported command in its DOSBox path: {}",
            line
        ));
    }

    Ok(plan)
}

pub fn load_dosbox_exception_plan(
    exception_script_path: &Path,
) -> Result<DosboxExceptionPlan, String> {
    let contents = std::fs::read_to_string(exception_script_path).map_err(|e| {
        format!(
            "Failed to read eXo exception launcher {}: {}",
            exception_script_path.display(),
            e
        )
    })?;
    parse_dosbox_exception_script(&contents)
}

pub fn parse_linux_dosbox_exception_script(
    contents: &str,
) -> Result<LinuxDosboxExceptionPlan, String> {
    let normalized = contents.replace("\r\n", "\n").replace('\r', "\n");
    let mut config_names = Vec::new();

    for line in normalized.lines() {
        let lower = line.trim().to_ascii_lowercase();
        if !lower.contains("options_linux.conf") {
            continue;
        }

        let line_config_names = extract_linux_config_names(line);
        if line_config_names.is_empty() {
            continue;
        }

        for name in line_config_names {
            if name.eq_ignore_ascii_case("options_linux.conf") {
                continue;
            }
            if !config_names
                .iter()
                .any(|existing: &String| existing.eq_ignore_ascii_case(&name))
            {
                config_names.push(name);
            }
        }
    }

    if config_names.is_empty() {
        return Err(
            "exception launcher does not contain a supported Linux DOSBox branch".to_string(),
        );
    }

    let selected = config_names
        .iter()
        .find(|name| name.eq_ignore_ascii_case("dosbox_linux.conf"))
        .cloned()
        .unwrap_or_else(|| config_names[0].clone());

    Ok(LinuxDosboxExceptionPlan {
        launch_config_name: selected,
    })
}

pub fn load_linux_dosbox_exception_plan(
    exception_script_path: &Path,
) -> Result<LinuxDosboxExceptionPlan, String> {
    let contents = std::fs::read_to_string(exception_script_path).map_err(|e| {
        format!(
            "Failed to read Linux eXo exception launcher {}: {}",
            exception_script_path.display(),
            e
        )
    })?;
    parse_linux_dosbox_exception_script(&contents)
}

pub fn parse_linux_scummvm_exception_script(
    contents: &str,
) -> Result<LinuxScummvmExceptionPlan, String> {
    let normalized = contents.replace("\r\n", "\n").replace('\r', "\n");

    for line in normalized.lines() {
        let trimmed = line.trim();
        if trimmed.is_empty() {
            continue;
        }

        let lower = trimmed.to_ascii_lowercase();
        if !lower.contains("scummvm") || !lower.contains("-p") {
            continue;
        }

        let tokens: Vec<String> = trimmed
            .split_whitespace()
            .map(normalize_shellish_token)
            .filter(|token| !token.is_empty())
            .collect();
        if tokens.is_empty() {
            continue;
        }

        let args = if tokens.first().map(|token| token.as_str()) == Some("flatpak")
            && tokens.get(1).map(|token| token.as_str()) == Some("run")
        {
            if tokens.len() < 4 || !tokens[2].to_ascii_lowercase().contains("scummvm") {
                continue;
            }
            &tokens[3..]
        } else {
            if !tokens[0].to_ascii_lowercase().contains("scummvm") {
                continue;
            }
            &tokens[1..]
        };

        let mut config_path = None;
        let mut game_path = None;
        let mut extra_args = Vec::new();
        let mut positionals = Vec::new();
        let mut idx = 0usize;

        while idx < args.len() {
            let token = &args[idx];
            if let Some(value) = token.strip_prefix("--config=") {
                config_path = Some(value.to_string());
                idx += 1;
                continue;
            }
            if token == "--config" {
                let value = args.get(idx + 1).ok_or_else(|| {
                    "ScummVM exception launcher is missing the config path".to_string()
                })?;
                config_path = Some(value.clone());
                idx += 2;
                continue;
            }
            if let Some(value) = token.strip_prefix("-p") {
                if !value.is_empty() {
                    game_path = Some(value.to_string());
                    idx += 1;
                    continue;
                }
                let value = args.get(idx + 1).ok_or_else(|| {
                    "ScummVM exception launcher is missing the game path".to_string()
                })?;
                game_path = Some(value.clone());
                idx += 2;
                continue;
            }
            if token.starts_with('-') {
                extra_args.push(token.clone());
            } else {
                positionals.push(token.clone());
            }
            idx += 1;
        }

        let config_path = config_path
            .ok_or_else(|| "ScummVM exception launcher is missing --config".to_string())?;
        let game_path =
            game_path.ok_or_else(|| "ScummVM exception launcher is missing -p".to_string())?;
        let game_id = positionals
            .last()
            .cloned()
            .ok_or_else(|| "ScummVM exception launcher is missing the game id".to_string())?;

        return Ok(LinuxScummvmExceptionPlan {
            config_path,
            game_path,
            game_id,
            extra_args,
        });
    }

    Err("exception launcher does not contain a supported Linux ScummVM branch".to_string())
}

pub fn load_linux_scummvm_exception_plan(
    exception_script_path: &Path,
) -> Result<LinuxScummvmExceptionPlan, String> {
    let contents = std::fs::read_to_string(exception_script_path).map_err(|e| {
        format!(
            "Failed to read Linux eXo exception launcher {}: {}",
            exception_script_path.display(),
            e
        )
    })?;
    parse_linux_scummvm_exception_script(&contents)
}

fn normalize_shellish_token(token: &str) -> String {
    token
        .trim()
        .trim_matches('"')
        .trim_matches('\'')
        .replace("\\!", "!")
}

fn extract_linux_config_names(line: &str) -> Vec<String> {
    let mut names = Vec::new();
    let mut search_start = 0usize;
    const SUFFIX: &str = "_linux.conf";

    while let Some(relative_end) = line[search_start..].find(SUFFIX) {
        let end = search_start + relative_end + SUFFIX.len();
        let mut start = search_start + relative_end;
        while start > 0 {
            let ch = line.as_bytes()[start - 1] as char;
            if ch.is_ascii_alphanumeric() || matches!(ch, '_' | '-' | '.') {
                start -= 1;
            } else {
                break;
            }
        }

        if start < end {
            names.push(line[start..end].to_string());
        }
        search_start = end;
    }

    names
}

fn extract_dosbox_branch_lines<'a>(lines: &'a [&'a str]) -> Vec<&'a str> {
    let mut start = 0usize;
    if let Some(idx) = lines
        .iter()
        .position(|line| line.trim().eq_ignore_ascii_case(":dosbox"))
    {
        start = idx + 1;
    }

    let mut branch = Vec::new();
    for line in &lines[start..] {
        let trimmed = line.trim();
        if !branch.is_empty() && trimmed.starts_with(':') {
            break;
        }
        branch.push(*line);
    }
    branch
}

pub fn plan_related_downloads(
    platform: &str,
    selected_index: usize,
    files: &[crate::torrent::TorrentFileInfo],
) -> Option<DownloadSelectionPlan> {
    let collection = collection_for_platform(platform)?;
    let selected = files.iter().find(|file| file.index == selected_index)?;
    let selected_name = selected.filename.replace('\\', "/");
    let archive_name = Path::new(&selected_name)
        .file_name()
        .and_then(|name| name.to_str())?;

    let companion_relatives = companion_archive_candidate_paths(collection, archive_name);
    let metadata_relatives: Vec<String> = metadata_archive_candidate_paths(collection)
        .into_iter()
        .map(|path| path.to_string_lossy().replace('\\', "/"))
        .collect();
    let util_relatives: Vec<String> = collection
        .shared_util_archive_candidates()
        .iter()
        .map(|value| value.replace('\\', "/"))
        .collect();

    let primary = if is_primary_download_candidate(platform, &selected_name) {
        selected
    } else {
        match collection {
            ExoCollection::Win9x => return None,
            ExoCollection::Dos | ExoCollection::Win3x => files.iter().find(|file| {
                file.filename.replace('\\', "/").ends_with(&format!(
                    "{}/{}",
                    collection.primary_archive_relative_dir(),
                    archive_name
                ))
            })?,
        }
    };

    let mut requested_indices = vec![primary.index];

    if let Some(companion) = files.iter().find(|file| {
        let filename = file.filename.replace('\\', "/");
        companion_relatives
            .iter()
            .any(|relative| filename.ends_with(&relative.to_string_lossy().replace('\\', "/")))
    }) {
        requested_indices.push(companion.index);
    }

    if let Some(metadata) = files.iter().find(|file| {
        let filename = file.filename.replace('\\', "/");
        metadata_relatives
            .iter()
            .any(|relative| filename.ends_with(relative))
    }) {
        requested_indices.push(metadata.index);
    }

    if matches!(collection, ExoCollection::Dos | ExoCollection::Win9x) {
        if let Some(util) = files.iter().find(|file| {
            let filename = file.filename.replace('\\', "/");
            util_relatives
                .iter()
                .any(|relative| filename.ends_with(relative))
        }) {
            requested_indices.push(util.index);
        }
    }

    requested_indices.sort_unstable();
    requested_indices.dedup();

    Some(DownloadSelectionPlan {
        requested_indices,
        representative_index: primary.index,
    })
}

pub async fn prepare_install_for_game(
    settings: &AppSettings,
    db_pool: &SqlitePool,
    launchbox_db_id: i64,
    platform: &str,
    raw_archive_path: &Path,
) -> Result<PreparedInstall, String> {
    let collection = collection_for_platform(platform)
        .ok_or_else(|| format!("{platform} does not use an eXo prepared install"))?;

    let resolved = resolve_required_archives(collection, raw_archive_path)?;
    let shortname = archive_top_level_directory(&resolved.primary_archive_path)?;
    let install_root = settings
        .get_rom_directory()
        .join(".lunchbox-pc-cache")
        .join(collection.table_name())
        .join(launchbox_db_id.to_string());
    let launch_config_path = preferred_install_config_path(
        collection,
        &install_root,
        &resolved.primary_archive_path,
        &shortname,
        &resolved.metadata_archive_path,
    );

    match lookup_cached_install(
        db_pool,
        launchbox_db_id,
        &resolved.primary_archive_path,
        &resolved.metadata_archive_path,
        &launch_config_path,
        &install_root,
    )
    .await?
    {
        CachedInstallLookup::Match => {
            return Ok(PreparedInstall {
                collection,
                install_root,
                launch_config_path,
                shortname,
            });
        }
        CachedInstallLookup::Missing => {
            if install_root.exists() && launch_config_path.exists() {
                record_cached_install(
                    db_pool,
                    launchbox_db_id,
                    platform,
                    collection,
                    &shortname,
                    &resolved,
                    &install_root,
                    &launch_config_path,
                )
                .await?;

                return Ok(PreparedInstall {
                    collection,
                    install_root,
                    launch_config_path,
                    shortname,
                });
            }
        }
        CachedInstallLookup::Mismatch => {}
    }

    if install_root.exists() {
        std::fs::remove_dir_all(&install_root).map_err(|e| {
            format!(
                "Failed to clear stale eXo install cache {}: {}",
                install_root.display(),
                e
            )
        })?;
    }

    std::fs::create_dir_all(&install_root).map_err(|e| {
        format!(
            "Failed to create eXo install cache {}: {}",
            install_root.display(),
            e
        )
    })?;

    extract_zip_archive(
        &resolved.primary_archive_path,
        &install_root.join(collection.install_root_name()),
    )?;

    if let Some(ref companion_archive_path) = resolved.companion_archive_path {
        let metadata_relative_dir = metadata_relative_dir_for_archive(
            collection,
            &resolved.primary_archive_path,
            &shortname,
        );
        let prefix = format!(
            "{}/{}/",
            collection.metadata_entry_root(),
            metadata_relative_dir.to_string_lossy().replace('\\', "/")
        );
        extract_zip_prefix(
            companion_archive_path,
            &prefix,
            Some(collection.metadata_entry_root()),
            &install_root
                .join(collection.install_root_name())
                .join("!dos"),
        )?;
    }

    let metadata_relative_dir =
        metadata_relative_dir_for_archive(collection, &resolved.primary_archive_path, &shortname);
    let metadata_prefix = format!(
        "{}/{}/",
        collection.metadata_entry_root(),
        metadata_relative_dir.to_string_lossy().replace('\\', "/")
    );
    let strip_prefix = match collection {
        ExoCollection::Dos => "eXo/eXoDOS",
        ExoCollection::Win3x => "eXo/eXoWin3x",
        ExoCollection::Win9x => "eXo/eXoWin9x",
    };
    extract_zip_prefix(
        &resolved.metadata_archive_path,
        &metadata_prefix,
        Some(strip_prefix),
        &install_root.join(collection.install_root_name()),
    )?;

    ensure_shared_bootstrap_assets(settings, collection, &resolved, &install_root)?;

    let launch_config_path = resolve_existing_install_config_path(
        collection,
        &install_root,
        &resolved.primary_archive_path,
        &shortname,
        &resolved.metadata_archive_path,
    )?;

    record_cached_install(
        db_pool,
        launchbox_db_id,
        platform,
        collection,
        &shortname,
        &resolved,
        &install_root,
        &launch_config_path,
    )
    .await?;

    Ok(PreparedInstall {
        collection,
        install_root,
        launch_config_path,
        shortname,
    })
}

async fn lookup_cached_install(
    db_pool: &SqlitePool,
    launchbox_db_id: i64,
    primary_archive_path: &Path,
    metadata_archive_path: &Path,
    expected_launch_config_path: &Path,
    expected_install_root: &Path,
) -> Result<CachedInstallLookup, String> {
    let row: Option<(String, String, String, String)> = sqlx::query_as(
        "SELECT shortname, source_archive_path, metadata_archive_path, install_root
         FROM pc_game_installs
         WHERE launchbox_db_id = ?",
    )
    .bind(launchbox_db_id)
    .fetch_optional(db_pool)
    .await
    .map_err(|e| e.to_string())?;

    let Some((_shortname, source_archive_path, cached_metadata_path, install_root)) = row else {
        return Ok(CachedInstallLookup::Missing);
    };

    if source_archive_path != primary_archive_path.display().to_string()
        || cached_metadata_path != metadata_archive_path.display().to_string()
        || install_root != expected_install_root.display().to_string()
    {
        return Ok(CachedInstallLookup::Mismatch);
    }

    if !expected_launch_config_path.exists() || !expected_install_root.exists() {
        return Ok(CachedInstallLookup::Mismatch);
    }

    let _ = sqlx::query(
        "UPDATE pc_game_installs SET last_used_at = CURRENT_TIMESTAMP WHERE launchbox_db_id = ?",
    )
    .bind(launchbox_db_id)
    .execute(db_pool)
    .await;

    Ok(CachedInstallLookup::Match)
}

async fn record_cached_install(
    db_pool: &SqlitePool,
    launchbox_db_id: i64,
    platform: &str,
    collection: ExoCollection,
    shortname: &str,
    resolved: &ResolvedArchives,
    install_root: &Path,
    launch_config_path: &Path,
) -> Result<(), String> {
    sqlx::query(
        "INSERT INTO pc_game_installs (
            launchbox_db_id,
            platform,
            collection,
            shortname,
            source_archive_path,
            companion_archive_path,
            metadata_archive_path,
            install_root,
            launch_config_path,
            prepared_at,
            last_used_at
        ) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, CURRENT_TIMESTAMP, CURRENT_TIMESTAMP)
        ON CONFLICT(launchbox_db_id) DO UPDATE SET
            platform = excluded.platform,
            collection = excluded.collection,
            shortname = excluded.shortname,
            source_archive_path = excluded.source_archive_path,
            companion_archive_path = excluded.companion_archive_path,
            metadata_archive_path = excluded.metadata_archive_path,
            install_root = excluded.install_root,
            launch_config_path = excluded.launch_config_path,
            prepared_at = CURRENT_TIMESTAMP,
            last_used_at = CURRENT_TIMESTAMP",
    )
    .bind(launchbox_db_id)
    .bind(platform)
    .bind(collection.table_name())
    .bind(shortname)
    .bind(resolved.primary_archive_path.display().to_string())
    .bind(
        resolved
            .companion_archive_path
            .as_ref()
            .map(|path| path.display().to_string()),
    )
    .bind(resolved.metadata_archive_path.display().to_string())
    .bind(install_root.display().to_string())
    .bind(launch_config_path.display().to_string())
    .execute(db_pool)
    .await
    .map_err(|e| e.to_string())?;

    Ok(())
}

fn resolve_required_archives(
    collection: ExoCollection,
    raw_archive_path: &Path,
) -> Result<ResolvedArchives, String> {
    let archive_name = raw_archive_path
        .file_name()
        .and_then(|name| name.to_str())
        .ok_or_else(|| format!("Invalid eXo archive path {}", raw_archive_path.display()))?;

    let primary_archive_path = if is_primary_download_candidate_for_collection(
        collection,
        &raw_archive_path.to_string_lossy(),
    ) {
        raw_archive_path.to_path_buf()
    } else {
        let primary_relative =
            PathBuf::from(collection.primary_archive_relative_dir()).join(archive_name);
        locate_relative_to_ancestors(raw_archive_path, &[primary_relative]).ok_or_else(|| {
            format!(
                "Could not locate the primary eXo archive for {} near {}",
                archive_name,
                raw_archive_path.display()
            )
        })?
    };

    let companion_candidates = companion_archive_candidate_paths(collection, archive_name);
    let companion_archive_path = if companion_candidates.is_empty() {
        None
    } else {
        locate_relative_to_ancestors(&primary_archive_path, &companion_candidates)
    };

    let metadata_archive_path = locate_relative_to_ancestors(
        &primary_archive_path,
        &metadata_archive_candidate_paths(collection),
    )
    .ok_or_else(|| {
        format!(
            "Could not locate {} near {}",
            collection.metadata_archive_relative_path(),
            primary_archive_path.display()
        )
    })?;
    let shared_util_archive_path = locate_relative_to_ancestors(
        &primary_archive_path,
        &collection
            .shared_util_archive_candidates()
            .iter()
            .map(PathBuf::from)
            .collect::<Vec<_>>(),
    );

    Ok(ResolvedArchives {
        primary_archive_path,
        companion_archive_path,
        metadata_archive_path,
        shared_util_archive_path,
    })
}

fn locate_relative_to_ancestors(start: &Path, relatives: &[PathBuf]) -> Option<PathBuf> {
    let start_dir = if start.is_dir() {
        start
    } else {
        start.parent()?
    };

    for relative in relatives {
        for ancestor in start_dir.ancestors() {
            let candidate = ancestor.join(relative);
            if candidate.exists() {
                return Some(candidate);
            }
        }
    }

    None
}

fn archive_top_level_directory(archive_path: &Path) -> Result<String, String> {
    let file = File::open(archive_path)
        .map_err(|e| format!("Failed to open {}: {}", archive_path.display(), e))?;
    let mut archive = zip::ZipArchive::new(file)
        .map_err(|e| format!("Failed to read {}: {}", archive_path.display(), e))?;

    for idx in 0..archive.len() {
        let entry = archive
            .by_index(idx)
            .map_err(|e| format!("Failed to read {}: {}", archive_path.display(), e))?;
        let Some(relative_path) = safe_relative_zip_path(entry.name()) else {
            continue;
        };
        let Some(first) = relative_path
            .components()
            .find_map(|component| match component {
                Component::Normal(value) => value.to_str().map(|value| value.to_string()),
                _ => None,
            })
        else {
            continue;
        };
        return Ok(first);
    }

    Err(format!(
        "Archive {} does not contain a top-level directory",
        archive_path.display()
    ))
}

fn extract_zip_archive(archive_path: &Path, dest_root: &Path) -> Result<(), String> {
    let file = File::open(archive_path)
        .map_err(|e| format!("Failed to open {}: {}", archive_path.display(), e))?;
    let mut archive = zip::ZipArchive::new(file)
        .map_err(|e| format!("Failed to read {}: {}", archive_path.display(), e))?;

    for idx in 0..archive.len() {
        let mut entry = archive
            .by_index(idx)
            .map_err(|e| format!("Failed to read {}: {}", archive_path.display(), e))?;
        let Some(relative_path) = safe_relative_zip_path(entry.name()) else {
            continue;
        };
        write_zip_entry(&mut entry, dest_root, &relative_path)?;
    }

    Ok(())
}

fn extract_zip_prefix(
    archive_path: &Path,
    prefix: &str,
    strip_prefix: Option<&str>,
    dest_root: &Path,
) -> Result<(), String> {
    let normalized_prefix = prefix.replace('\\', "/");
    let normalized_strip_prefix = strip_prefix.map(|value| value.replace('\\', "/"));
    let file = File::open(archive_path)
        .map_err(|e| format!("Failed to open {}: {}", archive_path.display(), e))?;
    let mut archive = zip::ZipArchive::new(file)
        .map_err(|e| format!("Failed to read {}: {}", archive_path.display(), e))?;

    for idx in 0..archive.len() {
        let mut entry = archive
            .by_index(idx)
            .map_err(|e| format!("Failed to read {}: {}", archive_path.display(), e))?;
        let entry_name = entry.name().replace('\\', "/");
        if !entry_name.starts_with(&normalized_prefix) {
            continue;
        }

        let stripped_name = if let Some(ref strip_prefix) = normalized_strip_prefix {
            entry_name
                .strip_prefix(strip_prefix)
                .and_then(|value| value.strip_prefix('/'))
                .unwrap_or(&entry_name)
                .to_string()
        } else {
            entry_name
        };

        let Some(relative_path) = safe_relative_zip_path(&stripped_name) else {
            continue;
        };
        write_zip_entry(&mut entry, dest_root, &relative_path)?;
    }

    Ok(())
}

fn ensure_shared_bootstrap_assets(
    settings: &AppSettings,
    collection: ExoCollection,
    resolved: &ResolvedArchives,
    install_root: &Path,
) -> Result<(), String> {
    let bootstrap_root = settings
        .get_rom_directory()
        .join(".lunchbox-pc-cache")
        .join("shared")
        .join(collection.shared_bootstrap_name());

    if let Some(util_archive_path) = resolved.shared_util_archive_path.as_ref() {
        match collection {
            ExoCollection::Dos | ExoCollection::Win3x => {
                ensure_mt32_bootstrap(util_archive_path, &bootstrap_root)?;
                ensure_dosbox_options_bootstrap(util_archive_path, &bootstrap_root)?;
            }
            ExoCollection::Win9x => {
                ensure_win9x_parent_bootstrap(util_archive_path, &bootstrap_root)?;
                ensure_win9x_options_bootstrap(util_archive_path, &bootstrap_root)?;
                ensure_win9x_86box_parent_bootstrap(util_archive_path, &bootstrap_root)?;
                ensure_win9x_pcbox_parent_bootstrap(util_archive_path, &bootstrap_root)?;
            }
        }
    }

    if matches!(collection, ExoCollection::Dos | ExoCollection::Win3x) {
        let shared_mt32 = bootstrap_root.join("mt32");
        if shared_mt32.exists() {
            let install_mt32 = install_root.join("mt32");
            if !install_mt32.exists() {
                copy_directory_recursive(&shared_mt32, &install_mt32)?;
            }
        }
        for options_name in ["options_linux.conf", "options.conf"] {
            let shared_options = bootstrap_root.join("emulators/dosbox").join(options_name);
            if shared_options.exists() {
                let install_options = install_root.join("emulators/dosbox").join(options_name);
                if !install_options.exists() {
                    if let Some(parent) = install_options.parent() {
                        std::fs::create_dir_all(parent).map_err(|e| {
                            format!("Failed to create directory {}: {}", parent.display(), e)
                        })?;
                    }
                    std::fs::copy(&shared_options, &install_options).map_err(|e| {
                        format!(
                            "Failed to copy {} to {}: {}",
                            shared_options.display(),
                            install_options.display(),
                            e
                        )
                    })?;
                }
            }
        }
    } else {
        let shared_options = bootstrap_root.join("emulators/dosbox/options9x.conf");
        if shared_options.exists() {
            let install_options = install_root.join("emulators/dosbox/options9x.conf");
            if !install_options.exists() {
                if let Some(parent) = install_options.parent() {
                    std::fs::create_dir_all(parent).map_err(|e| {
                        format!("Failed to create directory {}: {}", parent.display(), e)
                    })?;
                }
                std::fs::copy(&shared_options, &install_options).map_err(|e| {
                    format!(
                        "Failed to copy {} to {}: {}",
                        shared_options.display(),
                        install_options.display(),
                        e
                    )
                })?;
            }
        }

        let shared_parent_root = bootstrap_root.join("emulators/dosbox/x98/parent");
        if shared_parent_root.exists() {
            let install_parent_root = install_root.join("emulators/dosbox/x98/parent");
            link_or_copy_directory_files(&shared_parent_root, &install_parent_root)?;
        }

        let shared_86box_parent_root = bootstrap_root.join("emulators/86Box98/parent");
        if shared_86box_parent_root.exists() {
            let install_86box_parent_root = install_root.join("emulators/86Box98/parent");
            link_or_copy_directory_files(&shared_86box_parent_root, &install_86box_parent_root)?;
        }

        let shared_pcbox_parent_root = bootstrap_root.join("emulators/PCBox/parent");
        if shared_pcbox_parent_root.exists() {
            let install_pcbox_parent_root = install_root.join("emulators/PCBox/parent");
            link_or_copy_directory_files(&shared_pcbox_parent_root, &install_pcbox_parent_root)?;
        }
    }

    Ok(())
}

fn ensure_mt32_bootstrap(util_archive_path: &Path, bootstrap_root: &Path) -> Result<(), String> {
    let soundfont_path = bootstrap_root.join("mt32").join("SoundCanvas.sf2");
    if soundfont_path.exists() {
        return Ok(());
    }

    std::fs::create_dir_all(bootstrap_root).map_err(|e| {
        format!(
            "Failed to create shared eXo bootstrap directory {}: {}",
            bootstrap_root.display(),
            e
        )
    })?;

    let inner_zip_name = if is_linux_dos_util_archive(util_archive_path) {
        "EXTDOS_linux.zip"
    } else {
        "EXTDOS.zip"
    };

    extract_nested_zip_prefix(
        util_archive_path,
        inner_zip_name,
        "mt32/",
        None,
        bootstrap_root,
    )
}

fn ensure_dosbox_options_bootstrap(
    util_archive_path: &Path,
    bootstrap_root: &Path,
) -> Result<(), String> {
    let (inner_zip_name, options_relative) = if is_linux_dos_util_archive(util_archive_path) {
        ("EXTDOS_linux.zip", "emulators/dosbox/options_linux.conf")
    } else {
        ("EXTDOS.zip", "emulators/dosbox/options.conf")
    };
    let options_path = bootstrap_root.join(options_relative);
    if options_path.exists() {
        return Ok(());
    }

    extract_nested_zip_prefix(
        util_archive_path,
        inner_zip_name,
        options_relative,
        None,
        bootstrap_root,
    )
}

fn ensure_win9x_parent_bootstrap(
    util_archive_path: &Path,
    bootstrap_root: &Path,
) -> Result<(), String> {
    let parent_root = bootstrap_root.join("emulators/dosbox/x98/parent");
    if parent_root.exists()
        && std::fs::read_dir(&parent_root)
            .map(|mut entries| entries.next().is_some())
            .unwrap_or(false)
    {
        return Ok(());
    }

    extract_nested_zip_prefix(
        util_archive_path,
        "EXTWin9x.zip",
        "emulators/dosbox/x98/parent/",
        None,
        bootstrap_root,
    )
}

fn ensure_win9x_options_bootstrap(
    util_archive_path: &Path,
    bootstrap_root: &Path,
) -> Result<(), String> {
    let options_path = bootstrap_root.join("emulators/dosbox/options9x.conf");
    if options_path.exists() {
        return Ok(());
    }

    extract_nested_zip_prefix(
        util_archive_path,
        "EXTWin9x.zip",
        "emulators/dosbox/options9x.conf",
        None,
        bootstrap_root,
    )
}

fn ensure_win9x_86box_parent_bootstrap(
    util_archive_path: &Path,
    bootstrap_root: &Path,
) -> Result<(), String> {
    let parent_root = bootstrap_root.join("emulators/86Box98/parent");
    if parent_root.exists()
        && std::fs::read_dir(&parent_root)
            .map(|mut entries| entries.next().is_some())
            .unwrap_or(false)
    {
        return Ok(());
    }

    extract_nested_zip_prefix(
        util_archive_path,
        "EXTWin9x.zip",
        "emulators/86Box98/parent/",
        None,
        bootstrap_root,
    )
}

fn ensure_win9x_pcbox_parent_bootstrap(
    util_archive_path: &Path,
    bootstrap_root: &Path,
) -> Result<(), String> {
    let parent_root = bootstrap_root.join("emulators/PCBox/parent");
    if parent_root.exists()
        && std::fs::read_dir(&parent_root)
            .map(|mut entries| entries.next().is_some())
            .unwrap_or(false)
    {
        return Ok(());
    }

    extract_nested_zip_prefix(
        util_archive_path,
        "EXTWin9x.zip",
        "emulators/PCBox/parent/",
        None,
        bootstrap_root,
    )
}

fn extract_nested_zip_prefix(
    outer_archive_path: &Path,
    inner_zip_name: &str,
    prefix: &str,
    strip_prefix: Option<&str>,
    dest_root: &Path,
) -> Result<(), String> {
    let outer_file = File::open(outer_archive_path)
        .map_err(|e| format!("Failed to open {}: {}", outer_archive_path.display(), e))?;
    let mut outer_archive = zip::ZipArchive::new(outer_file)
        .map_err(|e| format!("Failed to read {}: {}", outer_archive_path.display(), e))?;
    let mut inner_entry = outer_archive.by_name(inner_zip_name).map_err(|e| {
        format!(
            "Failed to locate {} inside {}: {}",
            inner_zip_name,
            outer_archive_path.display(),
            e
        )
    })?;

    let temp_path = std::env::temp_dir().join(format!(
        "lunchbox-{}-{inner_zip_name}",
        uuid::Uuid::new_v4()
    ));
    {
        let mut temp_file = File::create(&temp_path)
            .map_err(|e| format!("Failed to create {}: {}", temp_path.display(), e))?;
        std::io::copy(&mut inner_entry, &mut temp_file).map_err(|e| {
            format!(
                "Failed to extract {} from {}: {}",
                inner_zip_name,
                outer_archive_path.display(),
                e
            )
        })?;
        temp_file
            .flush()
            .map_err(|e| format!("Failed to flush {}: {}", temp_path.display(), e))?;
    }

    let result = extract_zip_prefix(&temp_path, prefix, strip_prefix, dest_root);
    let _ = std::fs::remove_file(&temp_path);
    result
}

fn write_zip_entry(
    entry: &mut zip::read::ZipFile<'_>,
    dest_root: &Path,
    relative_path: &Path,
) -> Result<(), String> {
    let output_path = dest_root.join(relative_path);
    if entry.is_dir() {
        std::fs::create_dir_all(&output_path).map_err(|e| {
            format!(
                "Failed to create directory {}: {}",
                output_path.display(),
                e
            )
        })?;
        return Ok(());
    }

    if let Some(parent) = output_path.parent() {
        std::fs::create_dir_all(parent)
            .map_err(|e| format!("Failed to create directory {}: {}", parent.display(), e))?;
    }

    let mut output_file = File::create(&output_path)
        .map_err(|e| format!("Failed to create {}: {}", output_path.display(), e))?;
    std::io::copy(entry, &mut output_file)
        .map_err(|e| format!("Failed to extract {}: {}", output_path.display(), e))?;
    output_file
        .flush()
        .map_err(|e| format!("Failed to flush {}: {}", output_path.display(), e))?;
    Ok(())
}

fn copy_directory_recursive(src: &Path, dest: &Path) -> Result<(), String> {
    std::fs::create_dir_all(dest)
        .map_err(|e| format!("Failed to create directory {}: {}", dest.display(), e))?;

    for entry in std::fs::read_dir(src)
        .map_err(|e| format!("Failed to read directory {}: {}", src.display(), e))?
    {
        let entry = entry.map_err(|e| format!("Failed to read directory entry: {}", e))?;
        let source_path = entry.path();
        let dest_path = dest.join(entry.file_name());
        let file_type = entry
            .file_type()
            .map_err(|e| format!("Failed to inspect {}: {}", source_path.display(), e))?;

        if file_type.is_dir() {
            copy_directory_recursive(&source_path, &dest_path)?;
        } else {
            if let Some(parent) = dest_path.parent() {
                std::fs::create_dir_all(parent).map_err(|e| {
                    format!("Failed to create directory {}: {}", parent.display(), e)
                })?;
            }
            std::fs::copy(&source_path, &dest_path).map_err(|e| {
                format!(
                    "Failed to copy {} to {}: {}",
                    source_path.display(),
                    dest_path.display(),
                    e
                )
            })?;
        }
    }

    Ok(())
}

fn link_or_copy_directory_files(src: &Path, dest: &Path) -> Result<(), String> {
    std::fs::create_dir_all(dest)
        .map_err(|e| format!("Failed to create directory {}: {}", dest.display(), e))?;

    for entry in std::fs::read_dir(src)
        .map_err(|e| format!("Failed to read directory {}: {}", src.display(), e))?
    {
        let entry = entry.map_err(|e| format!("Failed to read directory entry: {}", e))?;
        let source_path = entry.path();
        let dest_path = dest.join(entry.file_name());
        let file_type = entry
            .file_type()
            .map_err(|e| format!("Failed to inspect {}: {}", source_path.display(), e))?;

        if file_type.is_dir() {
            link_or_copy_directory_files(&source_path, &dest_path)?;
            continue;
        }

        if dest_path.exists() {
            continue;
        }

        match std::fs::hard_link(&source_path, &dest_path) {
            Ok(()) => {}
            Err(_) => {
                std::fs::copy(&source_path, &dest_path).map_err(|e| {
                    format!(
                        "Failed to copy {} to {}: {}",
                        source_path.display(),
                        dest_path.display(),
                        e
                    )
                })?;
            }
        }
    }

    Ok(())
}

fn safe_relative_zip_path(name: &str) -> Option<PathBuf> {
    let path = Path::new(name);
    let mut normalized = PathBuf::new();

    for component in path.components() {
        match component {
            Component::Normal(value) => normalized.push(value),
            Component::CurDir => {}
            Component::RootDir | Component::ParentDir | Component::Prefix(_) => return None,
        }
    }

    if normalized.as_os_str().is_empty() {
        None
    } else {
        Some(normalized)
    }
}

#[cfg(test)]
mod tests {
    use super::{
        DosboxExceptionPlan, LinuxDosboxExceptionPlan, LinuxScummvmExceptionPlan,
        is_primary_download_candidate, parse_dosbox_exception_script,
        parse_linux_dosbox_exception_script, parse_linux_scummvm_exception_script,
        plan_related_downloads, prepare_install_for_game, primary_download_priority,
        should_use_prepared_install,
    };
    use crate::state::AppSettings;
    use crate::torrent::TorrentFileInfo;
    use std::fs::File;
    use std::io::Write;

    #[test]
    fn filters_exodos_download_picker_to_primary_archives() {
        assert!(is_primary_download_candidate(
            "MS-DOS",
            "eXo/eXoDOS/Prince of Persia (1990).zip"
        ));
        assert!(!is_primary_download_candidate(
            "MS-DOS",
            "Content/GameData/eXoDOS/Prince of Persia (1990).zip"
        ));
        assert!(!is_primary_download_candidate(
            "MS-DOS",
            "Content/!DOSmetadata.zip"
        ));
        assert!(is_primary_download_candidate(
            "MS-DOS",
            "eXo/eXoDOS/!spanish/Prince of Persia (1990).zip"
        ));
    }

    #[test]
    fn prioritizes_default_exodos_archives_before_language_variants() {
        assert_eq!(
            primary_download_priority("MS-DOS", "eXo/eXoDOS/Prince of Persia (1990).zip"),
            0
        );
        assert_eq!(
            primary_download_priority("MS-DOS", "eXo/eXoDOS/!english/Prince of Persia (1990).zip"),
            1
        );
        assert_eq!(
            primary_download_priority("MS-DOS", "eXo/eXoDOS/!german/Prince of Persia (1990).zip"),
            2
        );
    }

    #[test]
    fn plans_related_exodos_downloads() {
        let files = vec![
            TorrentFileInfo {
                index: 1,
                filename: "Content/GameData/eXoDOS/Prince of Persia (1990).zip".to_string(),
                size: 10,
            },
            TorrentFileInfo {
                index: 2,
                filename: "Content/!DOSmetadata.zip".to_string(),
                size: 20,
            },
            TorrentFileInfo {
                index: 3,
                filename: "eXo/eXoDOS/Prince of Persia (1990).zip".to_string(),
                size: 30,
            },
            TorrentFileInfo {
                index: 4,
                filename: "eXo/util/util.zip".to_string(),
                size: 40,
            },
        ];

        let plan = plan_related_downloads("MS-DOS", 3, &files).expect("plan");
        assert_eq!(plan.representative_index, 3);
        assert_eq!(plan.requested_indices, vec![1, 2, 3, 4]);
    }

    #[test]
    fn plans_related_exodos_language_downloads() {
        let files = vec![
            TorrentFileInfo {
                index: 1,
                filename: "Full Release/Content/GameData/eXoDOS/11th Hour, The (1995).zip"
                    .to_string(),
                size: 10,
            },
            TorrentFileInfo {
                index: 2,
                filename: "Full Release/Content/!DOSmetadata.zip".to_string(),
                size: 20,
            },
            TorrentFileInfo {
                index: 4,
                filename: "Full Release/eXo/util/util.zip".to_string(),
                size: 40,
            },
            TorrentFileInfo {
                index: 5,
                filename: "German Language Pack/eXo/eXoDOS/!german/11th Hour, The (1995).zip"
                    .to_string(),
                size: 30,
            },
        ];

        let plan = plan_related_downloads("MS-DOS", 5, &files).expect("plan");
        assert_eq!(plan.representative_index, 5);
        assert_eq!(plan.requested_indices, vec![1, 2, 4, 5]);
    }

    #[test]
    fn plans_related_exowin3x_downloads() {
        let files = vec![
            TorrentFileInfo {
                index: 4,
                filename: "Content/!Win3Xmetadata.zip".to_string(),
                size: 20,
            },
            TorrentFileInfo {
                index: 7,
                filename: "eXo/eXoWin3x/3-D Ultra Pinball - Creep Night (1996).zip".to_string(),
                size: 30,
            },
        ];

        let plan = plan_related_downloads("Windows 3.X", 7, &files).expect("plan");
        assert_eq!(plan.representative_index, 7);
        assert_eq!(plan.requested_indices, vec![4, 7]);
    }

    #[test]
    fn filters_exowin9x_download_picker_to_year_bucket_archives() {
        assert!(is_primary_download_candidate(
            "Windows",
            "eXo/eXoWin9x/1995/3-D Ultra Pinball (1995).zip"
        ));
        assert!(!is_primary_download_candidate(
            "Windows",
            "Content/!Win9Xmetadata.zip"
        ));
        assert!(!is_primary_download_candidate(
            "Windows",
            "eXo/eXoWin9x/1995/Extras/3-D Ultra Pinball (1995).zip"
        ));
    }

    #[test]
    fn plans_related_exowin9x_downloads() {
        let files = vec![
            TorrentFileInfo {
                index: 10,
                filename: "Content/!Win9Xmetadata.zip".to_string(),
                size: 20,
            },
            TorrentFileInfo {
                index: 11,
                filename: "eXo/eXoWin9x/1995/3-D Ultra Pinball (1995).zip".to_string(),
                size: 30,
            },
            TorrentFileInfo {
                index: 12,
                filename: "eXo/util/utilWin9x.zip".to_string(),
                size: 40,
            },
        ];

        let plan = plan_related_downloads("Windows", 11, &files).expect("plan");
        assert_eq!(plan.representative_index, 11);
        assert_eq!(plan.requested_indices, vec![10, 11, 12]);
    }

    #[test]
    fn only_uses_prepared_install_for_exo_layouts() {
        assert!(should_use_prepared_install(
            "Windows",
            std::path::Path::new("eXo/eXoWin9x/1995/3-D Ultra Pinball (1995).zip")
        ));
        assert!(!should_use_prepared_install(
            "Windows",
            std::path::Path::new("/roms/Windows/Game.exe")
        ));
    }

    #[test]
    fn parses_menu_based_exception_script_as_supported_dosbox_path() {
        let script = r#"
echo off
cd %VAR%
..\..\..\util\setconsole.exe /reset
echo.
..\..\..\util\choice /C:12 /N Please Choose:

if errorlevel = 2 goto scummvm
if errorlevel = 1 goto dosbox

:dosbox
..\..\..\util\setconsole.exe /minimize
cd ..
cd ..
cd ..
".\emulators\dosbox\%dosbox%" -conf "%var%\dosbox.conf" -conf ".\emulators\dosbox\options.conf" -conf %conf% -noconsole -exit
del stdout.txt
del stderr.txt
if exist glide.* del glide.*
if exist .\eXoDOS\CWSDPMI.swp del .\eXoDOS\CWSDPMI.swp
goto end

:scummvm
".\emulators\scummvm\scummvm.exe" --no-console foo
goto end

:end
"#;

        assert_eq!(
            parse_dosbox_exception_script(script).unwrap(),
            DosboxExceptionPlan {
                copy_mt32_roms: false,
            }
        );
    }

    #[test]
    fn parses_mt32_copy_exception_script() {
        let script = r#"
echo off
cls

:dosbox
copy .\mt32\*.rom .\
".\emulators\dosbox\%dosbox%" -conf "%var%\dosbox.conf" -conf ".\emulators\dosbox\options.conf" -conf %conf% -noconsole -exit -nomenu
del *.rom
goto end

:end
"#;

        assert_eq!(
            parse_dosbox_exception_script(script).unwrap(),
            DosboxExceptionPlan {
                copy_mt32_roms: true,
            }
        );
    }

    #[test]
    fn rejects_helper_process_exception_script() {
        let script = r#"
echo off
start .\eXoDOS\120Deg\sciAudio\sciAudio.exe
".\emulators\dosbox\%dosbox%" -conf "%var%\dosbox.conf" -conf ".\emulators\dosbox\options.conf" -conf %conf% -noconsole -exit -nomenu
TASKKILL /F /IM sciAudio.exe
goto end

:end
"#;

        let err = parse_dosbox_exception_script(script).unwrap_err();
        assert!(err.contains("helper process management"));
    }

    #[test]
    fn rejects_wrapped_dosbox_exception_script() {
        let script = r#"
echo off
:dosbox
.\eXoDOS\dune2\Dune2MouseHelper.exe ".\emulators\dosbox\%dosbox%" -c cls ".\eXoDOS\dune2\DUNE2.exe" -conf ".\eXoDOS\!dos\dune2\dosbox.conf" -conf ".\emulators\dosbox\options.conf" -conf %conf% -noconsole -exit -nomenu
goto end

:end
"#;

        let err = parse_dosbox_exception_script(script).unwrap_err();
        assert!(err.contains("unsupported DOSBox wrapper"));
    }

    #[test]
    fn parses_linux_exception_script_and_prefers_generic_dosbox_config() {
        let script = r#"
: tandy
eval "$(echo "${dosbox}" | sed -e "s/\$/\\$/g")" -conf \"$(echo "${var}" | sed -e "s/\$/\\$/g")/dosbox_tandy_linux.conf\" -conf \"./emulators/dosbox/options_linux.conf\" -conf $(echo "${conf}" | sed -e "s/\$/\\$/g") -noconsole -exit -nomenu

: pc
eval "$(echo "${dosbox}" | sed -e "s/\$/\\$/g")" -conf \"$(echo "${var}" | sed -e "s/\$/\\$/g")/dosbox_linux.conf\" -conf \"./emulators/dosbox/options_linux.conf\" -conf $(echo "${conf}" | sed -e "s/\$/\\$/g") -noconsole -exit -nomenu
"#;

        assert_eq!(
            parse_linux_dosbox_exception_script(script).unwrap(),
            LinuxDosboxExceptionPlan {
                launch_config_name: "dosbox_linux.conf".to_string(),
            }
        );
    }

    #[test]
    fn parses_linux_exception_script_with_generic_and_wine_side_branch() {
        let script = r#"
: orig
eval "$(echo "${dosbox}" | sed -e "s/\$/\\$/g")" -conf \"$(echo "${var}" | sed -e "s/\$/\\$/g")/dosbox_linux.conf\" -conf \"./emulators/dosbox/options_linux.conf\" -conf $(echo "${conf}" | sed -e "s/\$/\\$/g") -noconsole -exit -nomenu

: sspcfg
flatpak run com.retro_exo.wine SSP.exe
"#;

        assert_eq!(
            parse_linux_dosbox_exception_script(script).unwrap(),
            LinuxDosboxExceptionPlan {
                launch_config_name: "dosbox_linux.conf".to_string(),
            }
        );
    }

    #[test]
    fn parses_linux_exception_script_wrapped_through_wine_helper() {
        let script = r#"
: dosbox
flatpak run com.retro_exo.wine ./eXoDOS/dune2/Dune2MouseHelper.exe "./emulators/dosbox/ece4230/DOSBox.exe" -conf "./eXoDOS/"\!"dos/dune2/dosbox_linux.conf" -conf "./emulators/dosbox/options_linux.conf" -conf "${conf}" -noconsole -exit -nomenu
"#;

        assert_eq!(
            parse_linux_dosbox_exception_script(script).unwrap(),
            LinuxDosboxExceptionPlan {
                launch_config_name: "dosbox_linux.conf".to_string(),
            }
        );
    }

    #[test]
    fn rejects_linux_exception_script_without_supported_dosbox_branch() {
        let script = r#"
flatpak run com.retro_exo.scummvm-2-3-0-git15811-gf97bfb7ce1 --config=./emulators/scummvm/svn/scummvm.ini -F -g3x --aspect-ratio -peXoDOS/120Deg sci-fanmade
"#;

        let err = parse_linux_dosbox_exception_script(script).unwrap_err();
        assert!(err.contains("supported Linux DOSBox branch"));
    }

    #[test]
    fn parses_linux_scummvm_exception_script_with_flatpak_runner() {
        let script = r#"
flatpak run com.retro_exo.scummvm-2-3-0-git15811-gf97bfb7ce1 --config=./emulators/scummvm/svn/scummvm.ini -F -g3x --aspect-ratio -peXoDOS/120Deg sci-fanmade
"#;

        assert_eq!(
            parse_linux_scummvm_exception_script(script).unwrap(),
            LinuxScummvmExceptionPlan {
                config_path: "./emulators/scummvm/svn/scummvm.ini".to_string(),
                game_path: "eXoDOS/120Deg".to_string(),
                game_id: "sci-fanmade".to_string(),
                extra_args: vec![
                    "-F".to_string(),
                    "-g3x".to_string(),
                    "--aspect-ratio".to_string(),
                ],
            }
        );
    }

    #[test]
    fn parses_linux_scummvm_exception_script_with_direct_game_path() {
        let script = r#"
flatpak run com.retro_exo.scummvm-2-2-0 --config=./emulators/scummvm/scummvm_linux.ini -F -g3x --aspect-ratio -p./eXoDOS/gnomer sci-fanmade
"#;

        assert_eq!(
            parse_linux_scummvm_exception_script(script).unwrap(),
            LinuxScummvmExceptionPlan {
                config_path: "./emulators/scummvm/scummvm_linux.ini".to_string(),
                game_path: "./eXoDOS/gnomer".to_string(),
                game_id: "sci-fanmade".to_string(),
                extra_args: vec![
                    "-F".to_string(),
                    "-g3x".to_string(),
                    "--aspect-ratio".to_string(),
                ],
            }
        );
    }

    #[test]
    fn rejects_linux_scummvm_exception_script_without_supported_branch() {
        let script = r#"
eval "$(echo "${dosbox}" | sed -e "s/\$/\\$/g")" -conf \"$(echo "${var}" | sed -e "s/\$/\\$/g")/dosbox_linux.conf\" -conf \"./emulators/dosbox/options_linux.conf\" -conf $(echo "${conf}" | sed -e "s/\$/\\$/g") -noconsole -exit -nomenu
"#;

        let err = parse_linux_scummvm_exception_script(script).unwrap_err();
        assert!(err.contains("supported Linux ScummVM branch"));
    }

    fn create_zip(archive_path: &std::path::Path, entries: &[(&str, &[u8])]) {
        let archive_file = File::create(archive_path).unwrap();
        let mut writer = zip::ZipWriter::new(archive_file);
        for (name, contents) in entries {
            writer
                .start_file::<_, ()>(*name, zip::write::FileOptions::default())
                .unwrap();
            writer.write_all(contents).unwrap();
        }
        writer.finish().unwrap();
    }

    fn create_nested_zip_bytes(entries: &[(&str, &[u8])]) -> Vec<u8> {
        let cursor = std::io::Cursor::new(Vec::new());
        let mut writer = zip::ZipWriter::new(cursor);
        for (name, contents) in entries {
            writer
                .start_file::<_, ()>(*name, zip::write::FileOptions::default())
                .unwrap();
            writer.write_all(contents).unwrap();
        }
        writer.finish().unwrap().into_inner()
    }

    #[tokio::test]
    async fn prepares_and_reuses_exodos_install_cache() {
        let temp_dir = tempfile::tempdir().unwrap();
        let release_root = temp_dir.path().join("release");
        let primary_archive = release_root
            .join("eXo/eXoDOS")
            .join("Prince of Persia (1990).zip");
        let metadata_archive = release_root.join("Content/!DOSmetadata.zip");
        let companion_archive = release_root
            .join("Content/GameData/eXoDOS")
            .join("Prince of Persia (1990).zip");
        let util_archive = release_root.join("eXo/util/util.zip");

        std::fs::create_dir_all(primary_archive.parent().unwrap()).unwrap();
        std::fs::create_dir_all(metadata_archive.parent().unwrap()).unwrap();
        std::fs::create_dir_all(companion_archive.parent().unwrap()).unwrap();
        std::fs::create_dir_all(util_archive.parent().unwrap()).unwrap();

        create_zip(
            &primary_archive,
            &[
                ("Ppersia/run.bat", b"echo run"),
                ("Ppersia/POP.EXE", b"binary"),
            ],
        );
        create_zip(
            &metadata_archive,
            &[(
                "eXo/eXoDOS/!dos/Ppersia/dosbox.conf",
                b"[autoexec]\nmount c .\\eXoDOS\\Ppersia\n",
            )],
        );
        create_zip(
            &companion_archive,
            &[("eXo/eXoDOS/!dos/Ppersia/Extras/manual.txt", b"manual")],
        );
        create_zip(
            &util_archive,
            &[(
                "EXTDOS.zip",
                &create_nested_zip_bytes(&[
                    ("mt32/SoundCanvas.sf2", b"sf2"),
                    (
                        "emulators/dosbox/options.conf",
                        b"[sdl]\nfullscreen=false\n",
                    ),
                ]),
            )],
        );

        let db_path = temp_dir.path().join("lunchbox.db");
        let pool = crate::db::init_pool(&db_path).await.unwrap();

        let mut settings = AppSettings::default();
        settings.torrent.rom_directory = Some(temp_dir.path().join("roms"));

        let prepared = prepare_install_for_game(&settings, &pool, 42, "MS-DOS", &primary_archive)
            .await
            .unwrap();

        assert_eq!(
            prepared.launch_config_path,
            temp_dir
                .path()
                .join("roms/.lunchbox-pc-cache/exodos/42/eXoDOS/!dos/Ppersia/dosbox.conf")
        );
        assert!(
            temp_dir
                .path()
                .join("roms/.lunchbox-pc-cache/exodos/42/eXoDOS/Ppersia/POP.EXE")
                .exists()
        );
        assert!(
            temp_dir
                .path()
                .join("roms/.lunchbox-pc-cache/exodos/42/eXoDOS/!dos/Ppersia/Extras/manual.txt")
                .exists()
        );
        assert_eq!(
            std::fs::read(
                temp_dir
                    .path()
                    .join("roms/.lunchbox-pc-cache/exodos/42/mt32/SoundCanvas.sf2")
            )
            .unwrap(),
            b"sf2"
        );
        assert_eq!(
            std::fs::read(
                temp_dir
                    .path()
                    .join("roms/.lunchbox-pc-cache/exodos/42/emulators/dosbox/options.conf")
            )
            .unwrap(),
            b"[sdl]\nfullscreen=false\n"
        );

        let sentinel_path = temp_dir
            .path()
            .join("roms/.lunchbox-pc-cache/exodos/42/eXoDOS/Ppersia/SAVEGAME.SAV");
        std::fs::write(&sentinel_path, b"save").unwrap();
        sqlx::query("DELETE FROM pc_game_installs WHERE launchbox_db_id = 42")
            .execute(&pool)
            .await
            .unwrap();

        let prepared_again =
            prepare_install_for_game(&settings, &pool, 42, "MS-DOS", &primary_archive)
                .await
                .unwrap();

        assert_eq!(prepared_again.install_root, prepared.install_root);
        assert_eq!(std::fs::read(&sentinel_path).unwrap(), b"save");
    }

    #[tokio::test]
    async fn reinstalls_when_switching_to_exodos_language_variant() {
        let temp_dir = tempfile::tempdir().unwrap();
        let release_root = temp_dir.path().join("release");
        let primary_archive = release_root
            .join("Full Release/eXo/eXoDOS")
            .join("11th Hour, The (1995).zip");
        let german_archive = release_root
            .join("German Language Pack/eXo/eXoDOS/!german")
            .join("11th Hour, The (1995).zip");
        let metadata_archive = release_root.join("Full Release/Content/!DOSmetadata.zip");
        let companion_archive = release_root
            .join("Full Release/Content/GameData/eXoDOS")
            .join("11th Hour, The (1995).zip");
        let util_archive = release_root.join("Full Release/eXo/util/util.zip");

        std::fs::create_dir_all(primary_archive.parent().unwrap()).unwrap();
        std::fs::create_dir_all(german_archive.parent().unwrap()).unwrap();
        std::fs::create_dir_all(metadata_archive.parent().unwrap()).unwrap();
        std::fs::create_dir_all(companion_archive.parent().unwrap()).unwrap();
        std::fs::create_dir_all(util_archive.parent().unwrap()).unwrap();

        create_zip(
            &primary_archive,
            &[
                ("11thHour/GAME.DAT", b"english"),
                ("11thHour/run.bat", b"run"),
            ],
        );
        create_zip(
            &german_archive,
            &[
                ("11thHour/GAME.DAT", b"german"),
                ("11thHour/run.bat", b"run"),
            ],
        );
        create_zip(
            &metadata_archive,
            &[(
                "eXo/eXoDOS/!dos/11thHour/dosbox.conf",
                b"[autoexec]\nmount c .\\eXoDOS\\11thHour\n",
            )],
        );
        create_zip(
            &companion_archive,
            &[("eXo/eXoDOS/!dos/11thHour/Extras/readme.txt", b"readme")],
        );
        create_zip(
            &util_archive,
            &[(
                "EXTDOS.zip",
                &create_nested_zip_bytes(&[
                    ("mt32/SoundCanvas.sf2", b"sf2"),
                    (
                        "emulators/dosbox/options.conf",
                        b"[sdl]\nfullscreen=false\n",
                    ),
                ]),
            )],
        );

        let db_path = temp_dir.path().join("lunchbox.db");
        let pool = crate::db::init_pool(&db_path).await.unwrap();

        let mut settings = AppSettings::default();
        settings.torrent.rom_directory = Some(temp_dir.path().join("roms"));

        let prepared = prepare_install_for_game(&settings, &pool, 77, "MS-DOS", &primary_archive)
            .await
            .unwrap();
        let installed_file = prepared.install_root.join("eXoDOS/11thHour/GAME.DAT");
        assert_eq!(std::fs::read(&installed_file).unwrap(), b"english");

        let prepared_german =
            prepare_install_for_game(&settings, &pool, 77, "MS-DOS", &german_archive)
                .await
                .unwrap();

        assert_eq!(prepared_german.install_root, prepared.install_root);
        assert_eq!(std::fs::read(&installed_file).unwrap(), b"german");
    }

    #[tokio::test]
    async fn prepares_exowin9x_install_cache_with_shared_parent_images() {
        let temp_dir = tempfile::tempdir().unwrap();
        let release_root = temp_dir.path().join("release");
        let primary_archive = release_root
            .join("eXo/eXoWin9x/1995")
            .join("3D Ultra Pinball (1995).zip");
        let metadata_archive = release_root.join("Content/!Win9Xmetadata.zip");
        let util_archive = release_root.join("eXo/util/utilWin9x.zip");

        std::fs::create_dir_all(primary_archive.parent().unwrap()).unwrap();
        std::fs::create_dir_all(metadata_archive.parent().unwrap()).unwrap();
        std::fs::create_dir_all(util_archive.parent().unwrap()).unwrap();

        create_zip(
            &primary_archive,
            &[
                (
                    "3D Ultra Pinball (1995)/3D Ultra Pinball (1995).vhd",
                    b"gamevhd",
                ),
                (
                    "3D Ultra Pinball (1995)/3D Ultra Pinball (1995).cue",
                    b"FILE \"3D Ultra Pinball (1995).bin\" BINARY",
                ),
            ],
        );
        create_zip(
            &metadata_archive,
            &[
                (
                    "eXo/eXoWin9x/!win9x/1995/3D Ultra Pinball (1995)/Play.conf",
                    b"[autoexec]\nvhdmake -f -l .\\emulators\\dosbox\\x98\\parent/W98-C.vhd .\\emulators\\dosbox\\x98\\W98-C.vhd\nBOOT -l c\n",
                ),
                (
                    "eXo/eXoWin9x/!win9x/1995/3D Ultra Pinball (1995)/3D Ultra Pinball (1995).bat",
                    b"@echo off\r\n.\\util\\9xlaunch.bat\r\n",
                ),
            ],
        );
        create_zip(
            &util_archive,
            &[(
                "EXTWin9x.zip",
                &create_nested_zip_bytes(&[
                    (
                        "emulators/dosbox/options9x.conf",
                        b"[sdl]\nfullscreen=false\n",
                    ),
                    ("emulators/dosbox/x98/parent/W98-C.vhd", b"parentvhd"),
                ]),
            )],
        );

        let db_path = temp_dir.path().join("lunchbox.db");
        let pool = crate::db::init_pool(&db_path).await.unwrap();

        let mut settings = AppSettings::default();
        settings.torrent.rom_directory = Some(temp_dir.path().join("roms"));

        let prepared = prepare_install_for_game(&settings, &pool, 144, "Windows", &primary_archive)
            .await
            .unwrap();

        assert_eq!(prepared.collection, super::ExoCollection::Win9x);
        assert_eq!(
            prepared.launch_config_path,
            temp_dir
                .path()
                .join("roms/.lunchbox-pc-cache/exowin9x/144/eXoWin9x/!win9x/1995/3D Ultra Pinball (1995)/Play.conf")
        );
        assert_eq!(
            std::fs::read(
                temp_dir
                    .path()
                    .join("roms/.lunchbox-pc-cache/exowin9x/144/emulators/dosbox/options9x.conf")
            )
            .unwrap(),
            b"[sdl]\nfullscreen=false\n"
        );
        assert_eq!(
            std::fs::read(temp_dir.path().join(
                "roms/.lunchbox-pc-cache/exowin9x/144/emulators/dosbox/x98/parent/W98-C.vhd"
            ))
            .unwrap(),
            b"parentvhd"
        );
    }

    #[tokio::test]
    async fn prepares_exowin9x_install_cache_with_86box_metadata_cfg() {
        let temp_dir = tempfile::tempdir().unwrap();
        let release_root = temp_dir.path().join("release");
        let primary_archive = release_root
            .join("eXo/eXoWin9x/1996")
            .join("Daytona USA (1996).zip");
        let metadata_archive = release_root.join("Content/!Win9Xmetadata.zip");
        let util_archive = release_root.join("eXo/util/utilWin9x.zip");

        std::fs::create_dir_all(primary_archive.parent().unwrap()).unwrap();
        std::fs::create_dir_all(metadata_archive.parent().unwrap()).unwrap();
        std::fs::create_dir_all(util_archive.parent().unwrap()).unwrap();

        create_zip(
            &primary_archive,
            &[("Daytona USA (1996)/Daytona USA (1996).vhd", b"gamevhd")],
        );
        create_zip(
            &metadata_archive,
            &[
                (
                    "eXo/eXoWin9x/!win9x/1996/Daytona USA (1996)/Play.cfg",
                    b"[Hard disks]\nhdd_01_fn = W98-C.vhd\n",
                ),
                (
                    "eXo/eXoWin9x/!win9x/1996/Daytona USA (1996)/Daytona USA (1996).bat",
                    b"@echo off\r\n.\\util\\9xlaunch86Box.bat\r\n",
                ),
            ],
        );
        create_zip(
            &util_archive,
            &[(
                "EXTWin9x.zip",
                &create_nested_zip_bytes(&[
                    (
                        "emulators/dosbox/options9x.conf",
                        b"[sdl]\nfullscreen=false\n",
                    ),
                    ("emulators/86Box98/parent/W98-P.vhd", b"w98parent"),
                ]),
            )],
        );

        let db_path = temp_dir.path().join("lunchbox.db");
        let pool = crate::db::init_pool(&db_path).await.unwrap();

        let mut settings = AppSettings::default();
        settings.torrent.rom_directory = Some(temp_dir.path().join("roms"));

        let prepared = prepare_install_for_game(&settings, &pool, 145, "Windows", &primary_archive)
            .await
            .unwrap();

        assert_eq!(
            prepared.launch_config_path,
            temp_dir
                .path()
                .join("roms/.lunchbox-pc-cache/exowin9x/145/eXoWin9x/!win9x/1996/Daytona USA (1996)/Play.cfg")
        );
        assert_eq!(
            std::fs::read(
                temp_dir.path().join(
                    "roms/.lunchbox-pc-cache/exowin9x/145/emulators/86Box98/parent/W98-P.vhd"
                )
            )
            .unwrap(),
            b"w98parent"
        );
    }

    #[tokio::test]
    async fn prepares_exowin9x_install_cache_with_pcbox_metadata_cfg() {
        let temp_dir = tempfile::tempdir().unwrap();
        let release_root = temp_dir.path().join("release");
        let primary_archive = release_root
            .join("eXo/eXoWin9x/1997")
            .join("Need for Speed II SE (1997).zip");
        let metadata_archive = release_root.join("Content/!Win9Xmetadata.zip");
        let util_archive = release_root.join("eXo/util/utilWin9x.zip");

        std::fs::create_dir_all(primary_archive.parent().unwrap()).unwrap();
        std::fs::create_dir_all(metadata_archive.parent().unwrap()).unwrap();
        std::fs::create_dir_all(util_archive.parent().unwrap()).unwrap();

        create_zip(
            &primary_archive,
            &[(
                "Need for Speed II SE (1997)/Need for Speed II SE (1997).vhd",
                b"gamevhd",
            )],
        );
        create_zip(
            &metadata_archive,
            &[
                (
                    "eXo/eXoWin9x/!win9x/1997/Need for Speed II SE (1997)/Play.cfg",
                    b"[Hard disks]\nhdd_01_fn = W98-C.vhd\n",
                ),
                (
                    "eXo/eXoWin9x/!win9x/1997/Need for Speed II SE (1997)/Need for Speed II SE (1997).bat",
                    b"@echo off\r\n.\\util\\9xlaunchPCBox.bat\r\n",
                ),
            ],
        );
        create_zip(
            &util_archive,
            &[(
                "EXTWin9x.zip",
                &create_nested_zip_bytes(&[
                    (
                        "emulators/dosbox/options9x.conf",
                        b"[sdl]\nfullscreen=false\n",
                    ),
                    ("emulators/PCBox/parent/W98-P.vhd", b"pcboxparent"),
                ]),
            )],
        );

        let db_path = temp_dir.path().join("lunchbox.db");
        let pool = crate::db::init_pool(&db_path).await.unwrap();

        let mut settings = AppSettings::default();
        settings.torrent.rom_directory = Some(temp_dir.path().join("roms"));

        let prepared = prepare_install_for_game(&settings, &pool, 146, "Windows", &primary_archive)
            .await
            .unwrap();

        assert_eq!(
            prepared.launch_config_path,
            temp_dir
                .path()
                .join("roms/.lunchbox-pc-cache/exowin9x/146/eXoWin9x/!win9x/1997/Need for Speed II SE (1997)/Play.cfg")
        );
        assert_eq!(
            std::fs::read(
                temp_dir
                    .path()
                    .join("roms/.lunchbox-pc-cache/exowin9x/146/emulators/PCBox/parent/W98-P.vhd")
            )
            .unwrap(),
            b"pcboxparent"
        );
    }

    #[tokio::test]
    async fn prefers_linux_exodos_metadata_and_options_when_available() {
        let temp_dir = tempfile::tempdir().unwrap();
        let release_root = temp_dir.path().join("release");
        let primary_archive = release_root
            .join("eXo/eXoDOS/Full Release/eXo/eXoDOS")
            .join("Dune 2 - The Building of a Dynasty (1992).zip");
        let windows_metadata_archive =
            release_root.join("eXo/eXoDOS/Full Release/Content/!DOSmetadata.zip");
        let linux_metadata_archive =
            release_root.join("eXo/Linux Patches/eXoDOS/Content/!DOS_linux_metadata.zip");
        let companion_archive = release_root
            .join("eXo/eXoDOS/Full Release/Content/GameData/eXoDOS")
            .join("Dune 2 - The Building of a Dynasty (1992).zip");
        let linux_util_archive =
            release_root.join("eXo/Linux Patches/eXoDOS/eXo/util/utilDOS_linux.zip");

        std::fs::create_dir_all(primary_archive.parent().unwrap()).unwrap();
        std::fs::create_dir_all(windows_metadata_archive.parent().unwrap()).unwrap();
        std::fs::create_dir_all(linux_metadata_archive.parent().unwrap()).unwrap();
        std::fs::create_dir_all(companion_archive.parent().unwrap()).unwrap();
        std::fs::create_dir_all(linux_util_archive.parent().unwrap()).unwrap();

        create_zip(
            &primary_archive,
            &[("dune2/DUNE2.EXE", b"binary"), ("dune2/run.bat", b"run")],
        );
        create_zip(
            &windows_metadata_archive,
            &[(
                "eXo/eXoDOS/!dos/dune2/dosbox.conf",
                b"[autoexec]\nmount c .\\eXoDOS\\dune2\n",
            )],
        );
        create_zip(
            &linux_metadata_archive,
            &[(
                "eXo/eXoDOS/!dos/dune2/dosbox_linux.conf",
                b"[autoexec]\nmount c ./eXoDOS/dune2\n",
            )],
        );
        create_zip(
            &companion_archive,
            &[("eXo/eXoDOS/!dos/dune2/cd/game.cue", b"CUE")],
        );
        create_zip(
            &linux_util_archive,
            &[(
                "EXTDOS_linux.zip",
                &create_nested_zip_bytes(&[
                    (
                        "emulators/dosbox/options_linux.conf",
                        b"[sdl]\nfullscreen=false\n",
                    ),
                    ("mt32/SoundCanvas.sf2", b"sf2"),
                ]),
            )],
        );

        let db_path = temp_dir.path().join("lunchbox.db");
        let pool = crate::db::init_pool(&db_path).await.unwrap();

        let mut settings = AppSettings::default();
        settings.torrent.rom_directory = Some(temp_dir.path().join("roms"));

        let prepared = prepare_install_for_game(&settings, &pool, 91, "MS-DOS", &primary_archive)
            .await
            .unwrap();

        assert_eq!(
            prepared.launch_config_path,
            temp_dir
                .path()
                .join("roms/.lunchbox-pc-cache/exodos/91/eXoDOS/!dos/dune2/dosbox_linux.conf")
        );
        assert_eq!(
            std::fs::read(
                temp_dir
                    .path()
                    .join("roms/.lunchbox-pc-cache/exodos/91/emulators/dosbox/options_linux.conf")
            )
            .unwrap(),
            b"[sdl]\nfullscreen=false\n"
        );
    }
}
