use std::fs::{self, File};
use std::io::{Read, Write};
use std::path::{Component, Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::{SystemTime, UNIX_EPOCH};

use anyhow::{Context, Result, bail};
use rusqlite::{OptionalExtension, params};
use serde::{Deserialize, Serialize};
use sha1::{Digest, Sha1};
use zip::ZipArchive;

use crate::settings::{AppSettings, SettingsStore};

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ExoCollection {
    Dos,
    Win3x,
    Win9x,
}

impl ExoCollection {
    pub fn display_name(self) -> &'static str {
        match self {
            Self::Dos => "eXoDOS",
            Self::Win3x => "eXoWin3x",
            Self::Win9x => "eXoWin9x",
        }
    }

    pub fn slug(self) -> &'static str {
        match self {
            Self::Dos => "exodos",
            Self::Win3x => "exowin3x",
            Self::Win9x => "exowin9x",
        }
    }

    fn install_root_name(self) -> &'static str {
        self.display_name()
    }

    fn primary_archive_relative_dir(self) -> &'static str {
        match self {
            Self::Dos => "eXo/eXoDOS",
            Self::Win3x => "eXo/eXoWin3x",
            Self::Win9x => "eXo/eXoWin9x",
        }
    }

    fn metadata_entry_root(self) -> &'static str {
        match self {
            Self::Dos => "eXo/eXoDOS/!dos",
            Self::Win3x => "eXo/eXoWin3x/!win3x",
            Self::Win9x => "eXo/eXoWin9x/!win9x",
        }
    }

    fn metadata_candidates(self) -> &'static [&'static str] {
        match self {
            Self::Dos if cfg!(windows) => &[
                "Content/!DOSmetadata.zip",
                "Full Release/Content/!DOSmetadata.zip",
                "Content/!DOS_linux_metadata.zip",
                "Full Release/Content/!DOS_linux_metadata.zip",
                "Linux Patches/eXoDOS/Content/!DOS_linux_metadata.zip",
                "eXo/Linux Patches/eXoDOS/Content/!DOS_linux_metadata.zip",
            ],
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
            Self::Dos if cfg!(windows) => &[
                "eXo/util/util.zip",
                "Full Release/eXo/util/util.zip",
                "eXo/util/utilDOS_linux.zip",
                "Full Release/eXo/util/utilDOS_linux.zip",
                "Linux Patches/eXoDOS/eXo/util/utilDOS_linux.zip",
                "eXo/Linux Patches/eXoDOS/eXo/util/utilDOS_linux.zip",
            ],
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
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PreparedInstall {
    pub collection: ExoCollection,
    pub install_root: PathBuf,
    pub launch_config_path: PathBuf,
    pub shortname: String,
    pub source_archive_path: PathBuf,
    pub reused: bool,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PreparationProgress {
    pub phase: String,
    pub current_file: String,
    pub completed_bytes: u64,
}

#[derive(Clone, Debug)]
struct ResolvedArchives {
    primary: PathBuf,
    companion: Option<PathBuf>,
    metadata: PathBuf,
    utilities: Option<PathBuf>,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
struct ArchiveStamp {
    role: String,
    path: String,
    bytes: u64,
    modified_unix_ns: u128,
}

struct StagingDirectory {
    path: PathBuf,
    published: bool,
}

impl StagingDirectory {
    fn new(parent: &Path) -> Result<Self> {
        fs::create_dir_all(parent)
            .with_context(|| format!("creating eXo cache directory {}", parent.display()))?;
        let path = parent.join(format!(".staging-{}", uuid::Uuid::new_v4()));
        fs::create_dir(&path)
            .with_context(|| format!("creating eXo staging directory {}", path.display()))?;
        Ok(Self {
            path,
            published: false,
        })
    }

    fn publish(mut self, destination: &Path) -> Result<PathBuf> {
        fs::rename(&self.path, destination)
            .with_context(|| format!("publishing prepared eXo cache {}", destination.display()))?;
        self.published = true;
        Ok(destination.to_path_buf())
    }
}

impl Drop for StagingDirectory {
    fn drop(&mut self) {
        if !self.published {
            let _ = fs::remove_dir_all(&self.path);
        }
    }
}

pub fn collection_for_platform(platform: &str) -> Option<ExoCollection> {
    match platform {
        "MS-DOS" => Some(ExoCollection::Dos),
        "Windows 3.X" => Some(ExoCollection::Win3x),
        "Windows" => Some(ExoCollection::Win9x),
        _ => None,
    }
}

pub fn is_preparable_archive(platform: &str, path: &Path) -> bool {
    collection_for_platform(platform)
        .is_some_and(|collection| is_primary_archive(collection, &path.to_string_lossy()))
}

pub fn cached_install(store: &SettingsStore, game_uid: &str) -> Result<Option<PreparedInstall>> {
    let connection = store.connection()?;
    let row = connection
        .query_row(
            "SELECT collection, install_root, launch_config_path, shortname, source_archive_path
             FROM prepared_game_installs WHERE game_uid=?1",
            [game_uid],
            |row| {
                Ok((
                    row.get::<_, String>(0)?,
                    row.get::<_, String>(1)?,
                    row.get::<_, String>(2)?,
                    row.get::<_, String>(3)?,
                    row.get::<_, String>(4)?,
                ))
            },
        )
        .optional()?;
    let Some((collection, install_root, launch_config_path, shortname, source_archive_path)) = row
    else {
        return Ok(None);
    };
    let collection = parse_collection(&collection)?;
    let install_root = PathBuf::from(install_root);
    let launch_config_path = PathBuf::from(launch_config_path);
    if !install_root.is_dir() || !launch_config_path.is_file() {
        return Ok(None);
    }
    Ok(Some(PreparedInstall {
        collection,
        install_root,
        launch_config_path,
        shortname,
        source_archive_path: PathBuf::from(source_archive_path),
        reused: true,
    }))
}

pub(crate) fn remove_cached_install(
    store: &SettingsStore,
    settings: &AppSettings,
    game_uid: &str,
) -> Result<bool> {
    let connection = store.connection()?;
    let row = connection
        .query_row(
            "SELECT collection, install_root
             FROM prepared_game_installs WHERE game_uid=?1",
            [game_uid],
            |row| Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?)),
        )
        .optional()?;
    let Some((collection, install_root)) = row else {
        return Ok(false);
    };
    let collection = parse_collection(&collection)?;
    let install_root = PathBuf::from(install_root);
    let expected_parent = settings
        .rom_directory
        .join(".lunchbox-pc-cache")
        .join("installs")
        .join(collection.slug())
        .join(cache_game_key(game_uid));
    if settings.rom_directory.as_os_str().is_empty()
        || !settings.rom_directory.is_absolute()
        || install_root.parent() != Some(expected_parent.as_path())
        || install_root
            .file_name()
            .is_none_or(|name| name.is_empty() || name == "." || name == "..")
    {
        bail!(
            "refusing to remove prepared cache outside its exact game boundary: {}",
            install_root.display()
        );
    }

    match fs::symlink_metadata(&install_root) {
        Ok(metadata) => {
            if metadata.file_type().is_symlink() || !metadata.is_dir() {
                bail!(
                    "refusing to remove prepared cache with an unexpected file type: {}",
                    install_root.display()
                );
            }
            let canonical_parent = expected_parent.canonicalize().with_context(|| {
                format!(
                    "resolving prepared-cache boundary {}",
                    expected_parent.display()
                )
            })?;
            let canonical_install = install_root
                .canonicalize()
                .with_context(|| format!("resolving prepared cache {}", install_root.display()))?;
            if canonical_install.parent() != Some(canonical_parent.as_path()) {
                bail!(
                    "refusing to remove prepared cache that escapes its game boundary: {}",
                    install_root.display()
                );
            }
            fs::remove_dir_all(&install_root).with_context(|| {
                format!("removing prepared game cache {}", install_root.display())
            })?;
        }
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
        Err(error) => {
            return Err(error)
                .with_context(|| format!("inspecting prepared cache {}", install_root.display()));
        }
    }

    let removed = connection.execute(
        "DELETE FROM prepared_game_installs
         WHERE game_uid=?1 AND collection=?2 AND install_root=?3",
        params![
            game_uid,
            collection.slug(),
            install_root.display().to_string()
        ],
    )?;
    if removed != 1 {
        bail!("the prepared-cache record changed during removal");
    }
    match fs::remove_dir(&expected_parent) {
        Ok(()) => {}
        Err(error)
            if matches!(
                error.kind(),
                std::io::ErrorKind::DirectoryNotEmpty | std::io::ErrorKind::NotFound
            ) => {}
        Err(error) => {
            return Err(error).with_context(|| {
                format!(
                    "pruning prepared-cache game directory {}",
                    expected_parent.display()
                )
            });
        }
    }
    Ok(true)
}

#[allow(clippy::too_many_arguments)]
pub fn prepare_install<F>(
    settings: &AppSettings,
    store: &SettingsStore,
    game_uid: &str,
    launchbox_db_id: i64,
    platform: &str,
    raw_archive_path: &Path,
    cancelled: &AtomicBool,
    mut progress: F,
) -> Result<PreparedInstall>
where
    F: FnMut(PreparationProgress),
{
    if game_uid.trim().is_empty() {
        bail!("a stable game identity is required to prepare an eXo install");
    }
    if settings.rom_directory.as_os_str().is_empty() {
        bail!("choose a native ROM directory before preparing an eXo game");
    }
    let collection = collection_for_platform(platform)
        .with_context(|| format!("{platform} does not use an eXo prepared install"))?;
    if !is_primary_archive(collection, &raw_archive_path.to_string_lossy()) {
        bail!(
            "{} is not a primary {} game archive",
            raw_archive_path.display(),
            collection.slug()
        );
    }
    check_cancelled(cancelled)?;
    progress(PreparationProgress {
        phase: "Resolving exact archives".to_owned(),
        current_file: raw_archive_path.display().to_string(),
        completed_bytes: 0,
    });

    let resolved = resolve_required_archives(collection, raw_archive_path)?;
    let shortname = archive_top_level_directory(&resolved.primary)?;
    let stamps = archive_stamps(&resolved)?;
    let signature_json =
        serde_json::to_string(&stamps).context("encoding eXo archive signature")?;
    let signature_key = digest_key(signature_json.as_bytes());

    if let Some(cached) = lookup_matching_cache(store, game_uid, &signature_json)? {
        progress(PreparationProgress {
            phase: "Prepared install is ready".to_owned(),
            current_file: cached.launch_config_path.display().to_string(),
            completed_bytes: stamps.iter().map(|stamp| stamp.bytes).sum(),
        });
        return Ok(cached);
    }

    let cache_parent = settings
        .rom_directory
        .join(".lunchbox-pc-cache")
        .join("installs")
        .join(collection.slug())
        .join(cache_game_key(game_uid));
    let canonical_destination = cache_parent.join(&signature_key);
    if canonical_destination.is_dir()
        && let Some(launch_config_path) = resolve_existing_launch_config(
            collection,
            &canonical_destination,
            &resolved.primary,
            &shortname,
            &resolved.metadata,
        )
    {
        let prepared = PreparedInstall {
            collection,
            install_root: canonical_destination,
            launch_config_path,
            shortname,
            source_archive_path: resolved.primary.clone(),
            reused: true,
        };
        record_cache(
            store,
            game_uid,
            launchbox_db_id,
            platform,
            &resolved,
            &signature_json,
            &prepared,
        )?;
        return Ok(prepared);
    }

    let staging = StagingDirectory::new(&cache_parent)?;
    let install_root = staging.path.clone();
    extract_zip_archive(
        &resolved.primary,
        &install_root.join(collection.install_root_name()),
        cancelled,
        &mut progress,
        "Extracting game archive",
    )?;

    if let Some(companion) = &resolved.companion {
        let metadata_relative =
            metadata_relative_directory(collection, &resolved.primary, &shortname);
        let prefix = format!(
            "{}/{}/",
            collection.metadata_entry_root(),
            metadata_relative
        );
        extract_zip_prefix(
            companion,
            &prefix,
            Some(collection.metadata_entry_root()),
            &install_root
                .join(collection.install_root_name())
                .join(match collection {
                    ExoCollection::Dos => "!dos",
                    ExoCollection::Win3x => "!win3x",
                    ExoCollection::Win9x => "!win9x",
                }),
            cancelled,
            &mut progress,
            "Extracting game data",
        )?;
    }

    let metadata_relative = metadata_relative_directory(collection, &resolved.primary, &shortname);
    let metadata_prefix = format!(
        "{}/{}/",
        collection.metadata_entry_root(),
        metadata_relative
    );
    extract_zip_prefix(
        &resolved.metadata,
        &metadata_prefix,
        Some(collection.primary_archive_relative_dir()),
        &install_root.join(collection.install_root_name()),
        cancelled,
        &mut progress,
        "Extracting launch metadata",
    )?;

    if let Some(shared_root) = prepare_shared_bootstrap(
        settings,
        collection,
        resolved.utilities.as_deref(),
        cancelled,
        &mut progress,
    )? {
        install_shared_bootstrap(collection, &shared_root, &install_root, cancelled)?;
    }
    check_cancelled(cancelled)?;

    let staging_config = resolve_existing_launch_config(
        collection,
        &install_root,
        &resolved.primary,
        &shortname,
        &resolved.metadata,
    )
    .with_context(|| {
        format!(
            "prepared {} install contains no supported launch config for {shortname}",
            collection.slug()
        )
    })?;
    let config_relative = staging_config
        .strip_prefix(&install_root)
        .context("prepared eXo launch config escaped its staging directory")?
        .to_path_buf();
    let destination = unique_destination(&canonical_destination);
    let install_root = staging.publish(&destination)?;
    let prepared = PreparedInstall {
        collection,
        launch_config_path: install_root.join(config_relative),
        install_root,
        shortname,
        source_archive_path: resolved.primary.clone(),
        reused: false,
    };
    record_cache(
        store,
        game_uid,
        launchbox_db_id,
        platform,
        &resolved,
        &signature_json,
        &prepared,
    )?;
    progress(PreparationProgress {
        phase: "Prepared install is ready".to_owned(),
        current_file: prepared.launch_config_path.display().to_string(),
        completed_bytes: stamps.iter().map(|stamp| stamp.bytes).sum(),
    });
    Ok(prepared)
}

fn parse_collection(value: &str) -> Result<ExoCollection> {
    match value {
        "exodos" => Ok(ExoCollection::Dos),
        "exowin3x" => Ok(ExoCollection::Win3x),
        "exowin9x" => Ok(ExoCollection::Win9x),
        _ => bail!("prepared eXo cache has unsupported collection {value}"),
    }
}

fn is_primary_archive(collection: ExoCollection, path: &str) -> bool {
    let normalized = path.replace('\\', "/");
    if !normalized.to_ascii_lowercase().ends_with(".zip") {
        return false;
    }
    let lower = normalized.to_ascii_lowercase();
    let prefix = format!(
        "/{}/",
        collection
            .primary_archive_relative_dir()
            .to_ascii_lowercase()
    );
    let rooted = format!("/{}", lower.trim_matches('/'));
    let Some((_, tail)) = rooted.rsplit_once(&prefix) else {
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

fn resolve_required_archives(
    collection: ExoCollection,
    raw_archive_path: &Path,
) -> Result<ResolvedArchives> {
    let archive_name = raw_archive_path
        .file_name()
        .and_then(|name| name.to_str())
        .with_context(|| format!("invalid eXo archive path {}", raw_archive_path.display()))?;
    let companion = if collection == ExoCollection::Dos {
        locate_relative_to_ancestors(
            raw_archive_path,
            &[
                PathBuf::from(format!("Content/GameData/eXoDOS/{archive_name}")),
                PathBuf::from(format!(
                    "Full Release/Content/GameData/eXoDOS/{archive_name}"
                )),
            ],
        )
    } else {
        None
    };
    let metadata = locate_relative_to_ancestors(
        raw_archive_path,
        &collection
            .metadata_candidates()
            .iter()
            .map(PathBuf::from)
            .collect::<Vec<_>>(),
    )
    .with_context(|| {
        format!(
            "could not locate {} metadata near {}",
            collection.slug(),
            raw_archive_path.display()
        )
    })?;
    let utilities = locate_relative_to_ancestors(
        raw_archive_path,
        &collection
            .utility_candidates()
            .iter()
            .map(PathBuf::from)
            .collect::<Vec<_>>(),
    );
    Ok(ResolvedArchives {
        primary: raw_archive_path.to_path_buf(),
        companion,
        metadata,
        utilities,
    })
}

pub(crate) fn locate_relative_to_ancestors(start: &Path, relatives: &[PathBuf]) -> Option<PathBuf> {
    let start_directory = if start.is_dir() {
        start
    } else {
        start.parent()?
    };
    for relative in relatives {
        for ancestor in start_directory.ancestors() {
            let candidate = ancestor.join(relative);
            if candidate.is_file() {
                return Some(candidate);
            }
        }
    }
    None
}

fn archive_stamps(resolved: &ResolvedArchives) -> Result<Vec<ArchiveStamp>> {
    let mut archives = vec![
        ("primary", &resolved.primary),
        ("metadata", &resolved.metadata),
    ];
    if let Some(companion) = &resolved.companion {
        archives.push(("game-data", companion));
    }
    if let Some(utilities) = &resolved.utilities {
        archives.push(("utilities", utilities));
    }
    archives
        .into_iter()
        .map(|(role, path)| archive_stamp(role, path))
        .collect()
}

fn archive_stamp(role: &str, path: &Path) -> Result<ArchiveStamp> {
    let metadata = path
        .metadata()
        .with_context(|| format!("inspecting eXo archive {}", path.display()))?;
    if !metadata.is_file() {
        bail!("eXo archive is not a regular file: {}", path.display());
    }
    let modified = metadata
        .modified()
        .unwrap_or(SystemTime::UNIX_EPOCH)
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default();
    Ok(ArchiveStamp {
        role: role.to_owned(),
        path: path.display().to_string(),
        bytes: metadata.len(),
        modified_unix_ns: modified.as_nanos(),
    })
}

fn digest_key(bytes: &[u8]) -> String {
    let digest = Sha1::digest(bytes);
    digest
        .iter()
        .take(10)
        .map(|byte| format!("{byte:02x}"))
        .collect()
}

fn cache_game_key(game_uid: &str) -> String {
    let trimmed = game_uid.trim();
    if !trimmed.is_empty()
        && trimmed.len() <= 128
        && trimmed
            .chars()
            .all(|character| character.is_ascii_alphanumeric() || matches!(character, '-' | '_'))
    {
        trimmed.to_owned()
    } else {
        digest_key(trimmed.as_bytes())
    }
}

fn unique_destination(canonical: &Path) -> PathBuf {
    if !canonical.exists() {
        canonical.to_path_buf()
    } else {
        canonical.with_file_name(format!(
            "{}-{}",
            canonical
                .file_name()
                .and_then(|name| name.to_str())
                .unwrap_or("prepared"),
            uuid::Uuid::new_v4()
        ))
    }
}

fn archive_top_level_directory(path: &Path) -> Result<String> {
    let file = File::open(path).with_context(|| format!("opening {}", path.display()))?;
    let mut archive =
        ZipArchive::new(file).with_context(|| format!("reading {}", path.display()))?;
    for index in 0..archive.len() {
        let entry = archive
            .by_index(index)
            .with_context(|| format!("reading entry in {}", path.display()))?;
        let Some(relative) = safe_relative_zip_path(entry.name()) else {
            continue;
        };
        if let Some(Component::Normal(first)) = relative.components().next()
            && let Some(first) = first.to_str()
        {
            return Ok(first.to_owned());
        }
    }
    bail!("archive {} has no safe top-level directory", path.display())
}

fn metadata_relative_directory(
    collection: ExoCollection,
    primary: &Path,
    shortname: &str,
) -> String {
    if collection != ExoCollection::Win9x {
        return shortname.to_owned();
    }
    let normalized = primary.to_string_lossy().replace('\\', "/");
    let primary_directory = format!("{}/", collection.primary_archive_relative_dir());
    let Some((_, tail)) = normalized.rsplit_once(&primary_directory) else {
        return shortname.to_owned();
    };
    let year = tail.split('/').find(|part| !part.is_empty());
    year.map_or_else(
        || shortname.to_owned(),
        |year| format!("{year}/{shortname}"),
    )
}

fn preferred_launch_config_candidates(
    collection: ExoCollection,
    install_root: &Path,
    primary: &Path,
    shortname: &str,
    metadata: &Path,
) -> Vec<PathBuf> {
    let relative = metadata_relative_directory(collection, primary, shortname);
    match collection {
        ExoCollection::Dos => {
            let base = install_root.join("eXoDOS/!dos").join(shortname);
            if metadata
                .file_name()
                .and_then(|name| name.to_str())
                .is_some_and(|name| name.eq_ignore_ascii_case("!DOS_linux_metadata.zip"))
            {
                vec![base.join("dosbox_linux.conf"), base.join("dosbox.conf")]
            } else {
                vec![base.join("dosbox.conf"), base.join("dosbox_linux.conf")]
            }
        }
        ExoCollection::Win3x => vec![
            install_root
                .join("eXoWin3x/!win3x")
                .join(relative)
                .join("dosbox.conf"),
        ],
        ExoCollection::Win9x => ["Play.conf", "Play.cfg", "Host.cfg", "Join.cfg"]
            .into_iter()
            .map(|name| {
                install_root
                    .join("eXoWin9x/!win9x")
                    .join(&relative)
                    .join(name)
            })
            .collect(),
    }
}

fn resolve_existing_launch_config(
    collection: ExoCollection,
    install_root: &Path,
    primary: &Path,
    shortname: &str,
    metadata: &Path,
) -> Option<PathBuf> {
    preferred_launch_config_candidates(collection, install_root, primary, shortname, metadata)
        .into_iter()
        .find(|path| path.is_file())
}

fn extract_zip_archive<F>(
    archive_path: &Path,
    destination: &Path,
    cancelled: &AtomicBool,
    progress: &mut F,
    phase: &str,
) -> Result<()>
where
    F: FnMut(PreparationProgress),
{
    let file = File::open(archive_path)
        .with_context(|| format!("opening eXo archive {}", archive_path.display()))?;
    let mut archive = ZipArchive::new(file)
        .with_context(|| format!("reading eXo archive {}", archive_path.display()))?;
    for index in 0..archive.len() {
        check_cancelled(cancelled)?;
        let mut entry = archive
            .by_index(index)
            .with_context(|| format!("reading entry from {}", archive_path.display()))?;
        let Some(relative) = safe_relative_zip_path(entry.name()) else {
            continue;
        };
        write_zip_entry(
            &mut entry,
            destination,
            &relative,
            cancelled,
            progress,
            phase,
        )?;
    }
    Ok(())
}

#[allow(clippy::too_many_arguments)]
fn extract_zip_prefix<F>(
    archive_path: &Path,
    prefix: &str,
    strip_prefix: Option<&str>,
    destination: &Path,
    cancelled: &AtomicBool,
    progress: &mut F,
    phase: &str,
) -> Result<()>
where
    F: FnMut(PreparationProgress),
{
    let normalized_prefix = prefix.replace('\\', "/");
    let normalized_strip = strip_prefix.map(|value| value.replace('\\', "/"));
    let file = File::open(archive_path)
        .with_context(|| format!("opening eXo archive {}", archive_path.display()))?;
    let mut archive = ZipArchive::new(file)
        .with_context(|| format!("reading eXo archive {}", archive_path.display()))?;
    for index in 0..archive.len() {
        check_cancelled(cancelled)?;
        let mut entry = archive
            .by_index(index)
            .with_context(|| format!("reading entry from {}", archive_path.display()))?;
        let entry_name = entry.name().replace('\\', "/");
        if !entry_name.starts_with(&normalized_prefix) {
            continue;
        }
        let stripped = normalized_strip
            .as_ref()
            .and_then(|strip| entry_name.strip_prefix(strip))
            .map(|value| value.trim_start_matches('/'))
            .unwrap_or(&entry_name);
        let Some(relative) = safe_relative_zip_path(stripped) else {
            continue;
        };
        write_zip_entry(
            &mut entry,
            destination,
            &relative,
            cancelled,
            progress,
            phase,
        )?;
    }
    Ok(())
}

fn write_zip_entry<R, F>(
    entry: &mut zip::read::ZipFile<'_, R>,
    destination: &Path,
    relative: &Path,
    cancelled: &AtomicBool,
    progress: &mut F,
    phase: &str,
) -> Result<()>
where
    R: Read + ?Sized,
    F: FnMut(PreparationProgress),
{
    let output = destination.join(relative);
    if entry.is_dir() {
        fs::create_dir_all(&output)
            .with_context(|| format!("creating prepared directory {}", output.display()))?;
        return Ok(());
    }
    if let Some(parent) = output.parent() {
        fs::create_dir_all(parent)
            .with_context(|| format!("creating prepared directory {}", parent.display()))?;
    }
    let mut file = File::create(&output)
        .with_context(|| format!("creating prepared file {}", output.display()))?;
    let mut buffer = [0_u8; 1024 * 1024];
    let mut completed = 0_u64;
    loop {
        check_cancelled(cancelled)?;
        let read = entry
            .read(&mut buffer)
            .with_context(|| format!("extracting prepared file {}", output.display()))?;
        if read == 0 {
            break;
        }
        file.write_all(&buffer[..read])
            .with_context(|| format!("writing prepared file {}", output.display()))?;
        completed = completed.saturating_add(u64::try_from(read).unwrap_or_default());
        progress(PreparationProgress {
            phase: phase.to_owned(),
            current_file: relative.display().to_string(),
            completed_bytes: completed,
        });
    }
    file.flush()
        .with_context(|| format!("flushing prepared file {}", output.display()))?;
    Ok(())
}

fn safe_relative_zip_path(name: &str) -> Option<PathBuf> {
    let normalized_name = name.replace('\\', "/");
    let mut normalized = PathBuf::new();
    for component in Path::new(&normalized_name).components() {
        match component {
            Component::Normal(value) => normalized.push(value),
            Component::CurDir => {}
            Component::RootDir | Component::ParentDir | Component::Prefix(_) => return None,
        }
    }
    (!normalized.as_os_str().is_empty()).then_some(normalized)
}

fn prepare_shared_bootstrap<F>(
    settings: &AppSettings,
    collection: ExoCollection,
    utilities: Option<&Path>,
    cancelled: &AtomicBool,
    progress: &mut F,
) -> Result<Option<PathBuf>>
where
    F: FnMut(PreparationProgress),
{
    let Some(utilities) = utilities else {
        return Ok(None);
    };
    let stamp = archive_stamp("utilities", utilities)?;
    let signature = digest_key(
        serde_json::to_string(&stamp)
            .context("encoding eXo utility signature")?
            .as_bytes(),
    );
    let parent = settings
        .rom_directory
        .join(".lunchbox-pc-cache/shared")
        .join(match collection {
            ExoCollection::Dos | ExoCollection::Win3x => "exodos",
            ExoCollection::Win9x => "exowin9x",
        });
    if let Some(existing) = complete_shared_cache(&parent, &signature)? {
        return Ok(Some(existing));
    }
    let staging = StagingDirectory::new(&parent)?;
    match collection {
        ExoCollection::Dos | ExoCollection::Win3x => {
            let linux = utilities
                .file_name()
                .and_then(|name| name.to_str())
                .is_some_and(|name| name.eq_ignore_ascii_case("utilDOS_linux.zip"));
            let inner = if linux {
                "EXTDOS_linux.zip"
            } else {
                "EXTDOS.zip"
            };
            extract_nested_zip_prefix(
                utilities,
                inner,
                "mt32/",
                None,
                &staging.path,
                cancelled,
                progress,
                "Preparing shared MT-32 assets",
            )?;
            let options = if linux {
                "emulators/dosbox/options_linux.conf"
            } else {
                "emulators/dosbox/options.conf"
            };
            extract_nested_zip_prefix(
                utilities,
                inner,
                options,
                None,
                &staging.path,
                cancelled,
                progress,
                "Preparing shared DOSBox options",
            )?;
        }
        ExoCollection::Win9x => {
            for prefix in [
                "emulators/dosbox/x98/parent/",
                "emulators/dosbox/options9x.conf",
                "emulators/86Box98/parent/",
                "emulators/PCBox/parent/",
            ] {
                extract_nested_zip_prefix(
                    utilities,
                    "EXTWin9x.zip",
                    prefix,
                    None,
                    &staging.path,
                    cancelled,
                    progress,
                    "Preparing shared Win9x assets",
                )?;
            }
        }
    }
    fs::write(staging.path.join(".complete"), b"1")
        .context("marking shared eXo bootstrap complete")?;
    let destination = unique_destination(&parent.join(&signature));
    staging.publish(&destination).map(Some)
}

fn complete_shared_cache(parent: &Path, signature: &str) -> Result<Option<PathBuf>> {
    let Ok(entries) = fs::read_dir(parent) else {
        return Ok(None);
    };
    for entry in entries {
        let entry = entry.with_context(|| format!("reading shared cache {}", parent.display()))?;
        let name = entry.file_name();
        let name = name.to_string_lossy();
        let path = entry.path();
        if name.starts_with(signature) && path.join(".complete").is_file() {
            return Ok(Some(path));
        }
    }
    Ok(None)
}

#[allow(clippy::too_many_arguments)]
fn extract_nested_zip_prefix<F>(
    outer: &Path,
    inner_name: &str,
    prefix: &str,
    strip_prefix: Option<&str>,
    destination: &Path,
    cancelled: &AtomicBool,
    progress: &mut F,
    phase: &str,
) -> Result<()>
where
    F: FnMut(PreparationProgress),
{
    check_cancelled(cancelled)?;
    let outer_file = File::open(outer)
        .with_context(|| format!("opening eXo utility archive {}", outer.display()))?;
    let mut outer_archive = ZipArchive::new(outer_file)
        .with_context(|| format!("reading eXo utility archive {}", outer.display()))?;
    let mut inner = outer_archive.by_name(inner_name).with_context(|| {
        format!(
            "locating {inner_name} inside eXo utility archive {}",
            outer.display()
        )
    })?;
    let temp_path = destination.join(format!(".nested-{}.zip", uuid::Uuid::new_v4()));
    {
        let mut temp = File::create(&temp_path)
            .with_context(|| format!("creating nested archive {}", temp_path.display()))?;
        copy_cancellable(&mut inner, &mut temp, cancelled)?;
        temp.flush()
            .with_context(|| format!("flushing nested archive {}", temp_path.display()))?;
    }
    let result = extract_zip_prefix(
        &temp_path,
        prefix,
        strip_prefix,
        destination,
        cancelled,
        progress,
        phase,
    );
    let _ = fs::remove_file(&temp_path);
    result
}

fn copy_cancellable<R: Read, W: Write>(
    reader: &mut R,
    writer: &mut W,
    cancelled: &AtomicBool,
) -> Result<()> {
    let mut buffer = [0_u8; 1024 * 1024];
    loop {
        check_cancelled(cancelled)?;
        let read = reader
            .read(&mut buffer)
            .context("reading nested eXo archive")?;
        if read == 0 {
            return Ok(());
        }
        writer
            .write_all(&buffer[..read])
            .context("writing nested eXo archive")?;
    }
}

fn install_shared_bootstrap(
    collection: ExoCollection,
    shared: &Path,
    install: &Path,
    cancelled: &AtomicBool,
) -> Result<()> {
    match collection {
        ExoCollection::Dos | ExoCollection::Win3x => {
            copy_directory_if_present(&shared.join("mt32"), &install.join("mt32"), cancelled)?;
            for name in ["options_linux.conf", "options.conf"] {
                copy_file_if_present(
                    &shared.join("emulators/dosbox").join(name),
                    &install.join("emulators/dosbox").join(name),
                )?;
            }
        }
        ExoCollection::Win9x => {
            copy_file_if_present(
                &shared.join("emulators/dosbox/options9x.conf"),
                &install.join("emulators/dosbox/options9x.conf"),
            )?;
            for directory in [
                "emulators/dosbox/x98/parent",
                "emulators/86Box98/parent",
                "emulators/PCBox/parent",
            ] {
                link_or_copy_directory(
                    &shared.join(directory),
                    &install.join(directory),
                    cancelled,
                )?;
            }
        }
    }
    Ok(())
}

fn copy_directory_if_present(
    source: &Path,
    destination: &Path,
    cancelled: &AtomicBool,
) -> Result<()> {
    if !source.is_dir() {
        return Ok(());
    }
    fs::create_dir_all(destination)
        .with_context(|| format!("creating {}", destination.display()))?;
    for entry in fs::read_dir(source).with_context(|| format!("reading {}", source.display()))? {
        check_cancelled(cancelled)?;
        let entry = entry?;
        let source_path = entry.path();
        let destination_path = destination.join(entry.file_name());
        if entry.file_type()?.is_dir() {
            copy_directory_if_present(&source_path, &destination_path, cancelled)?;
        } else {
            copy_file_if_present(&source_path, &destination_path)?;
        }
    }
    Ok(())
}

fn link_or_copy_directory(source: &Path, destination: &Path, cancelled: &AtomicBool) -> Result<()> {
    if !source.is_dir() {
        return Ok(());
    }
    fs::create_dir_all(destination)
        .with_context(|| format!("creating {}", destination.display()))?;
    for entry in fs::read_dir(source).with_context(|| format!("reading {}", source.display()))? {
        check_cancelled(cancelled)?;
        let entry = entry?;
        let source_path = entry.path();
        let destination_path = destination.join(entry.file_name());
        if entry.file_type()?.is_dir() {
            link_or_copy_directory(&source_path, &destination_path, cancelled)?;
        } else if !destination_path.exists()
            && fs::hard_link(&source_path, &destination_path).is_err()
        {
            fs::copy(&source_path, &destination_path).with_context(|| {
                format!(
                    "copying shared eXo asset {} to {}",
                    source_path.display(),
                    destination_path.display()
                )
            })?;
        }
    }
    Ok(())
}

fn copy_file_if_present(source: &Path, destination: &Path) -> Result<()> {
    if !source.is_file() || destination.exists() {
        return Ok(());
    }
    if let Some(parent) = destination.parent() {
        fs::create_dir_all(parent).with_context(|| format!("creating {}", parent.display()))?;
    }
    fs::copy(source, destination).with_context(|| {
        format!(
            "copying shared eXo asset {} to {}",
            source.display(),
            destination.display()
        )
    })?;
    Ok(())
}

fn check_cancelled(cancelled: &AtomicBool) -> Result<()> {
    if cancelled.load(Ordering::Relaxed) {
        bail!("eXo preparation was cancelled");
    }
    Ok(())
}

fn lookup_matching_cache(
    store: &SettingsStore,
    game_uid: &str,
    signature_json: &str,
) -> Result<Option<PreparedInstall>> {
    let connection = store.connection()?;
    let row = connection
        .query_row(
            "SELECT collection, install_root, launch_config_path, shortname, source_archive_path, source_signature
             FROM prepared_game_installs WHERE game_uid=?1",
            [game_uid],
            |row| {
                Ok((
                    row.get::<_, String>(0)?,
                    row.get::<_, String>(1)?,
                    row.get::<_, String>(2)?,
                    row.get::<_, String>(3)?,
                    row.get::<_, String>(4)?,
                    row.get::<_, String>(5)?,
                ))
            },
        )
        .optional()?;
    let Some((
        collection,
        install_root,
        launch_config_path,
        shortname,
        source_archive_path,
        cached_signature,
    )) = row
    else {
        return Ok(None);
    };
    if cached_signature != signature_json {
        return Ok(None);
    }
    let install_root = PathBuf::from(install_root);
    let launch_config_path = PathBuf::from(launch_config_path);
    if !install_root.is_dir() || !launch_config_path.is_file() {
        return Ok(None);
    }
    connection.execute(
        "UPDATE prepared_game_installs SET last_used_at=?2 WHERE game_uid=?1",
        params![game_uid, unix_timestamp()],
    )?;
    Ok(Some(PreparedInstall {
        collection: parse_collection(&collection)?,
        install_root,
        launch_config_path,
        shortname,
        source_archive_path: PathBuf::from(source_archive_path),
        reused: true,
    }))
}

#[allow(clippy::too_many_arguments)]
fn record_cache(
    store: &SettingsStore,
    game_uid: &str,
    launchbox_db_id: i64,
    platform: &str,
    resolved: &ResolvedArchives,
    signature_json: &str,
    prepared: &PreparedInstall,
) -> Result<()> {
    let connection = store.connection()?;
    let now = unix_timestamp();
    connection.execute(
        "INSERT INTO prepared_game_installs (
             game_uid, launchbox_db_id, platform, collection, shortname,
             source_archive_path, companion_archive_path, metadata_archive_path,
             utility_archive_path, source_signature, install_root,
             launch_config_path, prepared_at, last_used_at
         ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?13)
         ON CONFLICT(game_uid) DO UPDATE SET
             launchbox_db_id=excluded.launchbox_db_id,
             platform=excluded.platform,
             collection=excluded.collection,
             shortname=excluded.shortname,
             source_archive_path=excluded.source_archive_path,
             companion_archive_path=excluded.companion_archive_path,
             metadata_archive_path=excluded.metadata_archive_path,
             utility_archive_path=excluded.utility_archive_path,
             source_signature=excluded.source_signature,
             install_root=excluded.install_root,
             launch_config_path=excluded.launch_config_path,
             prepared_at=excluded.prepared_at,
             last_used_at=excluded.last_used_at",
        params![
            game_uid,
            launchbox_db_id,
            platform,
            prepared.collection.slug(),
            prepared.shortname,
            resolved.primary.display().to_string(),
            resolved
                .companion
                .as_ref()
                .map(|path| path.display().to_string()),
            resolved.metadata.display().to_string(),
            resolved
                .utilities
                .as_ref()
                .map(|path| path.display().to_string()),
            signature_json,
            prepared.install_root.display().to_string(),
            prepared.launch_config_path.display().to_string(),
            now,
        ],
    )?;
    Ok(())
}

fn unix_timestamp() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs()
        .try_into()
        .unwrap_or(i64::MAX)
}

#[cfg(test)]
mod tests {
    use std::io::Cursor;

    use super::*;
    use zip::write::SimpleFileOptions;

    fn write_zip(path: &Path, entries: &[(&str, &[u8])]) {
        fs::create_dir_all(path.parent().unwrap()).unwrap();
        let file = File::create(path).unwrap();
        let mut writer = zip::ZipWriter::new(file);
        for (name, contents) in entries {
            writer
                .start_file(*name, SimpleFileOptions::default())
                .unwrap();
            writer.write_all(contents).unwrap();
        }
        writer.finish().unwrap();
    }

    fn zip_bytes(entries: &[(&str, &[u8])]) -> Vec<u8> {
        let mut writer = zip::ZipWriter::new(Cursor::new(Vec::new()));
        for (name, contents) in entries {
            writer
                .start_file(*name, SimpleFileOptions::default())
                .unwrap();
            writer.write_all(contents).unwrap();
        }
        writer.finish().unwrap().into_inner()
    }

    #[test]
    fn primary_layout_detection_is_bounded_to_exact_collections() {
        assert!(is_preparable_archive(
            "MS-DOS",
            Path::new("bundle/eXo/eXoDOS/Game (1991).zip")
        ));
        assert!(is_preparable_archive(
            "Windows",
            Path::new("bundle/eXo/eXoWin9x/1998/Game (1998).zip")
        ));
        assert!(!is_preparable_archive(
            "Windows",
            Path::new("bundle/eXo/eXoWin9x/1998/Extras/Game (1998).zip")
        ));
        assert!(!is_preparable_archive(
            "Nintendo Switch",
            Path::new("bundle/eXo/eXoDOS/Game (1991).zip")
        ));
    }

    #[test]
    fn prepares_and_reuses_an_atomic_exodos_cache() {
        let directory = tempfile::tempdir().unwrap();
        let roms = directory.path().join("roms");
        let bundle = roms.join(".lunchbox-pc-archives/hash");
        let primary = bundle.join("eXo/eXoDOS/Prince of Persia (1990).zip");
        write_zip(&primary, &[("PRINCE/PRINCE.DAT", b"game")]);
        write_zip(
            &bundle.join("Content/GameData/eXoDOS/Prince of Persia (1990).zip"),
            &[("eXo/eXoDOS/!dos/PRINCE/exception.bat", b"rem test")],
        );
        write_zip(
            &bundle.join("Content/!DOSmetadata.zip"),
            &[("eXo/eXoDOS/!dos/PRINCE/dosbox.conf", b"[autoexec]")],
        );
        let store = SettingsStore::at(directory.path().join("state.db")).unwrap();
        let settings = AppSettings {
            rom_directory: roms,
            ..AppSettings::default()
        };
        let cancelled = AtomicBool::new(false);
        let first = prepare_install(
            &settings,
            &store,
            "game-uid",
            42,
            "MS-DOS",
            &primary,
            &cancelled,
            |_| {},
        )
        .unwrap();
        assert!(!first.reused);
        assert!(
            first
                .install_root
                .join("eXoDOS/PRINCE/PRINCE.DAT")
                .is_file()
        );
        assert!(first.launch_config_path.is_file());
        assert!(
            first
                .install_root
                .join("eXoDOS/!dos/PRINCE/exception.bat")
                .is_file()
        );

        let second = prepare_install(
            &settings,
            &store,
            "game-uid",
            42,
            "MS-DOS",
            &primary,
            &cancelled,
            |_| {},
        )
        .unwrap();
        assert!(second.reused);
        assert_eq!(second.install_root, first.install_root);
        assert_eq!(cached_install(&store, "game-uid").unwrap().unwrap(), second);
        let outside = directory.path().join("outside-cache");
        fs::create_dir(&outside).unwrap();
        fs::write(outside.join("keep.txt"), b"keep").unwrap();
        store
            .connection()
            .unwrap()
            .execute(
                "UPDATE prepared_game_installs SET install_root=?2 WHERE game_uid=?1",
                params!["game-uid", outside.display().to_string()],
            )
            .unwrap();
        assert!(remove_cached_install(&store, &settings, "game-uid").is_err());
        assert_eq!(fs::read(outside.join("keep.txt")).unwrap(), b"keep");
        store
            .connection()
            .unwrap()
            .execute(
                "UPDATE prepared_game_installs SET install_root=?2 WHERE game_uid=?1",
                params!["game-uid", first.install_root.display().to_string()],
            )
            .unwrap();
        assert!(remove_cached_install(&store, &settings, "game-uid").unwrap());
        assert!(!first.install_root.exists());
        assert!(cached_install(&store, "game-uid").unwrap().is_none());
        assert!(!remove_cached_install(&store, &settings, "game-uid").unwrap());
    }

    #[test]
    fn prepares_win9x_metadata_and_shared_nested_bootstrap() {
        let directory = tempfile::tempdir().unwrap();
        let roms = directory.path().join("roms");
        let bundle = roms.join(".lunchbox-pc-archives/hash");
        let primary = bundle.join("eXo/eXoWin9x/1998/Example Game (1998).zip");
        write_zip(&primary, &[("EXAMPLE98/disk/game.img", b"game")]);
        write_zip(
            &bundle.join("Content/!Win9Xmetadata.zip"),
            &[(
                "eXo/eXoWin9x/!win9x/1998/EXAMPLE98/Play.cfg",
                b"9xlaunchpcbox",
            )],
        );
        let utilities = zip_bytes(&[
            ("emulators/dosbox/options9x.conf", b"[dosbox]"),
            ("emulators/dosbox/x98/parent/system.vhd", b"dosbox-parent"),
            ("emulators/86Box98/parent/system.vhd", b"86box-parent"),
            ("emulators/PCBox/parent/system.vhd", b"pcbox-parent"),
        ]);
        write_zip(
            &bundle.join("eXo/util/utilWin9x.zip"),
            &[("EXTWin9x.zip", &utilities)],
        );

        let store = SettingsStore::at(directory.path().join("state.db")).unwrap();
        let settings = AppSettings {
            rom_directory: roms,
            ..AppSettings::default()
        };
        let prepared = prepare_install(
            &settings,
            &store,
            "win9x-game",
            99,
            "Windows",
            &primary,
            &AtomicBool::new(false),
            |_| {},
        )
        .unwrap();

        assert_eq!(prepared.collection, ExoCollection::Win9x);
        assert_eq!(prepared.shortname, "EXAMPLE98");
        assert_eq!(
            prepared.launch_config_path,
            prepared
                .install_root
                .join("eXoWin9x/!win9x/1998/EXAMPLE98/Play.cfg")
        );
        assert!(prepared.launch_config_path.is_file());
        assert!(
            prepared
                .install_root
                .join("emulators/dosbox/options9x.conf")
                .is_file()
        );
        for parent in [
            "emulators/dosbox/x98/parent/system.vhd",
            "emulators/86Box98/parent/system.vhd",
            "emulators/PCBox/parent/system.vhd",
        ] {
            assert!(prepared.install_root.join(parent).is_file(), "{parent}");
        }
    }

    #[test]
    fn unsafe_zip_paths_never_escape_the_owned_staging_directory() {
        let directory = tempfile::tempdir().unwrap();
        let roms = directory.path().join("roms");
        let bundle = roms.join(".lunchbox-pc-archives/hash");
        let primary = bundle.join("eXo/eXoDOS/Safe Game (1991).zip");
        write_zip(
            &primary,
            &[
                ("../../escape.txt", b"escaped"),
                ("..\\escape-windows.txt", b"escaped"),
                ("/absolute.txt", b"escaped"),
                ("SAFE/GAME.DAT", b"game"),
            ],
        );
        write_zip(
            &bundle.join("Content/!DOSmetadata.zip"),
            &[("eXo/eXoDOS/!dos/SAFE/dosbox.conf", b"config")],
        );
        let store = SettingsStore::at(directory.path().join("state.db")).unwrap();
        let settings = AppSettings {
            rom_directory: roms,
            ..AppSettings::default()
        };
        let prepared = prepare_install(
            &settings,
            &store,
            "safe-game",
            11,
            "MS-DOS",
            &primary,
            &AtomicBool::new(false),
            |_| {},
        )
        .unwrap();

        assert!(prepared.install_root.join("eXoDOS/SAFE/GAME.DAT").is_file());
        assert!(!prepared.install_root.join("escape.txt").exists());
        assert!(!prepared.install_root.join("escape-windows.txt").exists());
        assert!(!prepared.install_root.join("absolute.txt").exists());
        assert!(
            !prepared
                .install_root
                .parent()
                .unwrap()
                .join("escape.txt")
                .exists()
        );
    }

    #[test]
    fn cancellation_never_publishes_or_records_a_partial_install() {
        let directory = tempfile::tempdir().unwrap();
        let roms = directory.path().join("roms");
        let bundle = roms.join(".lunchbox-pc-archives/hash");
        let primary = bundle.join("eXo/eXoDOS/Game (1991).zip");
        write_zip(&primary, &[("GAME/GAME.DAT", &[7_u8; 2 * 1024 * 1024])]);
        write_zip(
            &bundle.join("Content/!DOSmetadata.zip"),
            &[("eXo/eXoDOS/!dos/GAME/dosbox.conf", b"config")],
        );
        let store = SettingsStore::at(directory.path().join("state.db")).unwrap();
        let settings = AppSettings {
            rom_directory: roms,
            ..AppSettings::default()
        };
        let cancelled = AtomicBool::new(true);
        assert!(
            prepare_install(
                &settings,
                &store,
                "cancelled-game",
                7,
                "MS-DOS",
                &primary,
                &cancelled,
                |_| {},
            )
            .is_err()
        );
        assert!(cached_install(&store, "cancelled-game").unwrap().is_none());
        let cache = settings
            .rom_directory
            .join(".lunchbox-pc-cache/installs/exodos/cancelled-game");
        assert!(!cache.exists() || fs::read_dir(cache).unwrap().next().is_none());
    }
}
