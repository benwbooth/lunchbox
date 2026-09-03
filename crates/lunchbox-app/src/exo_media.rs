//! Imports per-game artwork from an eXo collection metadata pack.
//!
//! The eXo projects distribute a metadata pack (for example
//! `XODOSMetadata.zip` under `Content/`) that contains the LaunchBox
//! platform XML plus one image file per game and media type. Games are
//! identified exactly — by the XML `ApplicationPath` shortname directory for
//! prepared installs, and by the XML `DatabaseID` for collection-wide
//! imports — so an import never relies on a fuzzy title match.

use std::collections::BTreeMap;
use std::collections::HashSet;
use std::fs;
use std::io::Read;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};

use anyhow::{Context, Result, bail};
use quick_xml::Reader;
use quick_xml::events::Event;
use zip::ZipArchive;

use crate::exo_install::{ExoCollection, PreparedInstall, locate_relative_to_ancestors};
use crate::media::ArtworkKind;

/// LaunchBox XML media sections mapped onto Lunchbox artwork kinds, in
/// preference order. Region subfolders are checked after the plain section.
const IMAGE_SECTIONS: &[(ArtworkKind, &[&str])] = &[
    (
        ArtworkKind::BoxFront,
        &[
            "Box - Front",
            "Box - Front - Reconstructed",
            "Box - Front/World",
            "Box - Front/North America",
            "Box - Front/Europe",
            "Box - Front/Japan",
        ],
    ),
    (
        ArtworkKind::BoxBack,
        &[
            "Box - Back",
            "Box - Back/World",
            "Box - Back/North America",
            "Box - Back/Europe",
            "Box - Back/Japan",
        ],
    ),
    (ArtworkKind::Fanart, &["Fanart - Background"]),
    (ArtworkKind::Screenshot, &["Screenshot - Gameplay"]),
    (ArtworkKind::TitleScreen, &["Screenshot - Game Title"]),
    (ArtworkKind::ClearLogo, &["Clear Logo"]),
    (
        ArtworkKind::CartFront,
        &[
            "Cart - Front",
            "Cart - Front/World",
            "Cart - Front/North America",
        ],
    ),
    (
        ArtworkKind::CartBack,
        &[
            "Cart - Back",
            "Cart - Back/World",
            "Cart - Back/North America",
        ],
    ),
    (ArtworkKind::Cart3d, &["Cart - 3D"]),
    (
        ArtworkKind::Disc,
        &["Disc", "Disc/World", "Disc/North America"],
    ),
    (ArtworkKind::Marquee, &["Marquee"]),
    (ArtworkKind::Banner, &["Banner"]),
    (ArtworkKind::Flyer, &["Advertisement Flyer - Front"]),
    (ArtworkKind::GameOverScreen, &["Screenshot - Game Over"]),
];

const IMAGE_EXTENSIONS: &[&str] = &["jpg", "png", "jpeg"];
const NAME_VARIANTS: &[&str] = &["", "-00", "-01", "-02"];

pub(crate) fn import_prepared_game_media(
    media_root: &Path,
    database_id: i64,
    prepared: &PreparedInstall,
) -> Result<Vec<ArtworkKind>> {
    let destination = media_root
        .join(format!("lb-{database_id}"))
        .join("launchbox");
    if marker_path(&destination).is_file() {
        return Ok(Vec::new());
    }
    let Some(pack_path) = locate_metadata_pack(prepared) else {
        return Ok(Vec::new());
    };

    let pack = fs::File::open(&pack_path)
        .with_context(|| format!("opening eXo metadata pack {}", pack_path.display()))?;
    let mut archive = ZipArchive::new(pack)
        .with_context(|| format!("reading eXo metadata pack {}", pack_path.display()))?;

    let member_names: Vec<String> = archive.file_names().map(str::to_owned).collect();
    let member_index: std::collections::HashSet<&str> =
        member_names.iter().map(String::as_str).collect();
    let platform_dir = resolve_platform_dir(&member_index, prepared.collection)
        .context("eXo metadata pack has no images for this collection platform")?;
    let xml_member = resolve_xml_member(&member_index, prepared.collection)
        .context("eXo metadata pack has no LaunchBox XML for this collection platform")?;
    let xml_bytes = read_zip_member(&mut archive, &xml_member)?;
    let xml = String::from_utf8_lossy(&xml_bytes);
    let game_names = launchbox_game_names(&xml, &prepared.shortname);
    if game_names.is_empty() {
        bail!("eXo metadata XML has no entry for the prepared game");
    }

    fs::create_dir_all(&destination)
        .with_context(|| format!("creating artwork directory {}", destination.display()))?;
    let mut imported = Vec::new();
    for (kind, sections) in IMAGE_SECTIONS.iter() {
        if let Some(member) = find_image_member(&member_index, &platform_dir, sections, &game_names)
        {
            let extension = member
                .rsplit('.')
                .next()
                .unwrap_or("jpg")
                .to_ascii_lowercase();
            // The media cache is indexed by these exact file stems.
            let target = destination.join(format!("{}.{}", kind.cache_file_stem(), extension));
            let bytes = read_zip_member(&mut archive, &member)?;
            fs::write(&target, &bytes)
                .with_context(|| format!("writing imported artwork {}", target.display()))?;
            imported.push(*kind);
        }
    }
    fs::write(marker_path(&destination), b"imported\n").with_context(|| {
        format!(
            "marking eXo media import complete in {}",
            destination.display()
        )
    })?;
    Ok(imported)
}

fn marker_path(destination: &Path) -> PathBuf {
    destination.join(".exo-media-imported")
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct CollectionImportSummary {
    pub collection: ExoCollection,
    pub pack_path: PathBuf,
    pub games_in_pack: usize,
    pub games_imported: usize,
    pub assets_imported: usize,
    pub cancelled: bool,
}

/// Imports artwork for every game in a collection's metadata pack into the
/// LaunchBox media cache. Each XML `<Game>` carries the LaunchBox
/// `DatabaseID`, which is exactly the media cache's `lb-<id>` key, so the
/// whole collection maps without title matching. A per-game completion
/// marker makes the run resumable, and a pack-level marker (stamped with
/// the pack's size and mtime) makes a fully-imported pack a no-op.
pub(crate) fn import_collection_media<F>(
    media_root: &Path,
    collection: ExoCollection,
    anchors: &[PathBuf],
    progress: F,
    cancelled: &AtomicBool,
) -> Result<Option<CollectionImportSummary>>
where
    F: FnMut(usize, usize),
{
    let Some(pack_path) = locate_metadata_pack_from_anchors(collection, anchors) else {
        return Ok(None);
    };
    let pack_stamp = archive_stamp(&pack_path)?;
    let pack_marker = media_root.join(format!(".{}-pack-imported", collection.slug()));
    if pack_marker.is_file() && fs::read_to_string(&pack_marker).unwrap_or_default() == pack_stamp {
        return Ok(None);
    }

    let pack = fs::File::open(&pack_path)
        .with_context(|| format!("opening eXo metadata pack {}", pack_path.display()))?;
    let mut archive = ZipArchive::new(pack)
        .with_context(|| format!("reading eXo metadata pack {}", pack_path.display()))?;
    let member_names: Vec<String> = archive.file_names().map(str::to_owned).collect();
    let member_index: HashSet<&str> = member_names.iter().map(String::as_str).collect();
    let platform_dir = resolve_platform_dir(&member_index, collection)
        .with_context(|| format!("eXo metadata pack for {collection:?} has no images"))?;
    let xml_member = resolve_xml_member(&member_index, collection)
        .context("eXo metadata pack has no LaunchBox XML for this collection platform")?;
    let xml_bytes = read_zip_member(&mut archive, &xml_member)?;
    let xml = String::from_utf8_lossy(&xml_bytes);
    let games: Vec<(i64, Vec<String>)> = parse_launchbox_xml(&xml)
        .into_iter()
        .filter(|entry| entry.database_id > 0)
        .map(|entry| (entry.database_id, entry.names))
        .collect();

    let mut summary = CollectionImportSummary {
        collection,
        pack_path: pack_path.clone(),
        games_in_pack: games.len(),
        games_imported: 0,
        assets_imported: 0,
        cancelled: false,
    };
    let mut progress = progress;
    for (position, (database_id, game_names)) in games.iter().enumerate() {
        if cancelled.load(Ordering::Relaxed) {
            summary.cancelled = true;
            break;
        }
        progress(position, games.len());
        let destination = media_root
            .join(format!("lb-{database_id}"))
            .join("launchbox");
        if marker_path(&destination).is_file() || game_names.is_empty() {
            continue;
        }
        for (kind, sections) in IMAGE_SECTIONS.iter() {
            let Some(member) =
                find_image_member(&member_index, &platform_dir, sections, game_names)
            else {
                continue;
            };
            let extension = member
                .rsplit('.')
                .next()
                .unwrap_or("jpg")
                .to_ascii_lowercase();
            // The media cache is indexed by these exact file stems.
            let target = destination.join(format!("{}.{}", kind.cache_file_stem(), extension));
            let bytes = read_zip_member(&mut archive, &member)?;
            fs::create_dir_all(&destination)
                .with_context(|| format!("creating artwork directory {}", destination.display()))?;
            fs::write(&target, &bytes)
                .with_context(|| format!("writing imported artwork {}", target.display()))?;
            summary.assets_imported += 1;
        }
        fs::create_dir_all(&destination)
            .with_context(|| format!("creating artwork directory {}", destination.display()))?;
        fs::write(marker_path(&destination), b"imported\n").with_context(|| {
            format!(
                "marking eXo media import complete in {}",
                destination.display()
            )
        })?;
        summary.games_imported += 1;
    }
    progress(games.len(), games.len());
    if !summary.cancelled {
        fs::create_dir_all(media_root)
            .with_context(|| format!("creating media directory {}", media_root.display()))?;
        fs::write(&pack_marker, pack_stamp).with_context(|| {
            format!(
                "marking the {} pack fully imported at {}",
                collection.display_name(),
                pack_marker.display()
            )
        })?;
    }
    Ok(Some(summary))
}

fn archive_stamp(path: &Path) -> Result<String> {
    let metadata = path
        .metadata()
        .with_context(|| format!("inspecting eXo metadata pack {}", path.display()))?;
    let modified = metadata
        .modified()
        .unwrap_or(std::time::UNIX_EPOCH)
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default();
    Ok(format!("{}\n{}\n", metadata.len(), modified.as_nanos()))
}

/// Collects filesystem anchors that point inside an eXo torrent download
/// tree: prepared-install source archives and completed eXo download jobs.
/// Any single anchor is enough to locate the collection's metadata pack.
pub(crate) fn collection_pack_anchors(store: &crate::settings::SettingsStore) -> Vec<PathBuf> {
    let mut anchors = Vec::new();
    let Ok(connection) = store.connection() else {
        return anchors;
    };
    let Ok(mut statement) = connection.prepare(
        "SELECT DISTINCT source_archive_path FROM prepared_game_installs
         WHERE source_archive_path <> ''",
    ) else {
        return anchors;
    };
    if let Ok(rows) = statement.query_map([], |row| row.get::<_, String>(0)) {
        for row in rows.flatten() {
            anchors.push(PathBuf::from(row));
        }
    }
    if let Ok(jobs) = store.jobs() {
        for job in jobs {
            if job.download_plan.contains("exo_archive_set")
                && !job.local_download_path.as_os_str().is_empty()
            {
                anchors.push(job.local_download_path.clone());
            }
        }
    }
    anchors.sort();
    anchors.dedup();
    anchors
}

fn locate_metadata_pack_from_anchors(
    collection: ExoCollection,
    anchors: &[PathBuf],
) -> Option<PathBuf> {
    let relatives = pack_relative_candidates(collection);
    for anchor in anchors {
        // Anchors are usually symlinks into the original torrent download
        // tree, where the metadata pack lives; resolve them first.
        let start = fs::canonicalize(anchor).unwrap_or_else(|_| anchor.clone());
        if let Some(pack) = locate_relative_to_ancestors(
            &start,
            relatives
                .iter()
                .map(PathBuf::from)
                .collect::<Vec<_>>()
                .as_slice(),
        ) {
            return Some(pack);
        }
    }
    None
}

fn pack_relative_candidates(collection: ExoCollection) -> &'static [&'static str] {
    match collection {
        ExoCollection::Dos => &[
            "Content/XODOSMetadata.zip",
            "Full Release/Content/XODOSMetadata.zip",
            "Lite Release/Content/XODOSMetadata.zip",
            // Reached from the torrent's shared eXo/ root when the only
            // anchor belongs to a different collection subtree.
            "eXoDOS/Full Release/Content/XODOSMetadata.zip",
            "eXoDOS/Lite Release/Content/XODOSMetadata.zip",
        ],
        ExoCollection::Win3x => &[
            "Content/XOWin3xMetadata.zip",
            "Full Release/Content/XOWin3xMetadata.zip",
            "Lite Release/Content/XOWin3xMetadata.zip",
            "eXoWin3x/Content/XOWin3xMetadata.zip",
        ],
        ExoCollection::Win9x => &[
            "Content/XOWin9xMetadata.zip",
            "Full Release/Content/XOWin9xMetadata.zip",
            "Lite Release/Content/XOWin9xMetadata.zip",
            "eXoWin9x/Content/XOWin9xMetadata.zip",
        ],
    }
}

fn locate_metadata_pack(prepared: &PreparedInstall) -> Option<PathBuf> {
    locate_metadata_pack_from_anchors(
        prepared.collection,
        std::slice::from_ref(&prepared.source_archive_path),
    )
}

fn platform_dir_candidates(
    collection: crate::exo_install::ExoCollection,
) -> &'static [&'static str] {
    match collection {
        crate::exo_install::ExoCollection::Dos => &["MS-DOS"],
        crate::exo_install::ExoCollection::Win3x => &["Windows 3x", "Windows 3.x", "Win3x"],
        crate::exo_install::ExoCollection::Win9x => &["Windows 9x", "Windows 9.x", "Win9x"],
    }
}

fn collection_token(collection: crate::exo_install::ExoCollection) -> &'static str {
    match collection {
        crate::exo_install::ExoCollection::Dos => "dos",
        crate::exo_install::ExoCollection::Win3x => "3x",
        crate::exo_install::ExoCollection::Win9x => "9x",
    }
}

/// Resolves the platform directory of the pack's `Images/` tree. eXoDOS uses
/// `MS-DOS`, the Windows packs `Windows 3x`/`Windows 9x`.
fn resolve_platform_dir(
    member_index: &std::collections::HashSet<&str>,
    collection: crate::exo_install::ExoCollection,
) -> Option<String> {
    for candidate in platform_dir_candidates(collection) {
        let prefix = format!("Images/{candidate}/");
        if member_index
            .iter()
            .any(|name| name.starts_with(prefix.as_str()))
        {
            return Some(candidate.to_string());
        }
    }
    let mut dirs = member_index
        .iter()
        .filter_map(|name| name.strip_prefix("Images/"))
        .filter(|rest| !rest.contains('/'))
        .filter(|dir| {
            !matches!(
                *dir,
                "Badges"
                    | "Cache-BB"
                    | "Cache-LB"
                    | "None"
                    | "Media Packs"
                    | "Platform Categories"
                    | "Platforms"
                    | "Playlists"
            )
        })
        .filter(|dir| {
            dir.to_ascii_lowercase()
                .contains(collection_token(collection))
        })
        .collect::<Vec<_>>();
    dirs.sort_unstable();
    dirs.dedup();
    if dirs.len() == 1 {
        return Some(dirs.remove(0).to_string());
    }
    None
}

/// Resolves the pack's LaunchBox platform XML member. eXoDOS nests it under
/// `xml/all/MS-DOS.xml`, Win3x keeps it at `xml/Windows 3x.xml`, and newer
/// Win9x packs name it by era without an extension (`xml/all/1994-1996.9x`).
fn resolve_xml_member(
    member_index: &std::collections::HashSet<&str>,
    collection: crate::exo_install::ExoCollection,
) -> Option<String> {
    let mut candidates = member_index
        .iter()
        .filter(|name| {
            if name.starts_with("xml/Playlists/") || name.ends_with(".bat") {
                return false;
            }
            let Some(stem) = name
                .strip_prefix("xml/all/")
                .or_else(|| name.strip_prefix("xml/"))
            else {
                return false;
            };
            // Only files directly in the platform XML directories; the
            // family/ and Playlists/ trees are not the game list.
            !stem.contains('/')
        })
        .filter(|name| {
            name.to_ascii_lowercase()
                .contains(collection_token(collection))
        })
        .copied()
        .collect::<Vec<_>>();
    candidates.sort_unstable();
    candidates.dedup();
    let first = candidates.first()?;
    Some(first.to_string())
}

fn read_zip_member(archive: &mut ZipArchive<fs::File>, member: &str) -> Result<Vec<u8>> {
    let mut entry = archive
        .by_name(member)
        .with_context(|| format!("opening member {member} in the eXo metadata pack"))?;
    let mut bytes = Vec::new();
    entry.read_to_end(&mut bytes)?;
    Ok(bytes)
}

/// One parsed `<Game>` from an eXo LaunchBox XML.
#[derive(Clone, Debug, Eq, PartialEq)]
struct XmlGameEntry {
    /// LaunchBox `DatabaseID`; this is exactly the media cache's `lb-<id>` key.
    database_id: i64,
    /// Normalized (forward-slash, lowercased) `ApplicationPath`.
    application_path: String,
    /// Name candidates — eXo mixes LaunchBox `<Name>` and `<Title>`
    /// conventions, and image files are keyed by either.
    names: Vec<String>,
}

fn parse_launchbox_xml(xml: &str) -> Vec<XmlGameEntry> {
    let mut entries = Vec::new();
    let mut reader = Reader::from_str(xml);
    reader.config_mut().trim_text(true);
    let mut in_game = false;
    let mut field: Option<String> = None;
    let mut fields: BTreeMap<String, String> = BTreeMap::new();
    let mut buffer = Vec::new();
    loop {
        buffer.clear();
        match reader.read_event_into(&mut buffer) {
            Ok(Event::Start(ref event)) => {
                let name = String::from_utf8_lossy(event.name().as_ref()).into_owned();
                if name == "Game" {
                    in_game = true;
                    fields.clear();
                } else if in_game {
                    field = Some(name);
                }
            }
            Ok(Event::Text(ref event)) => {
                if let Some(field_name) = &field
                    && in_game
                    && let Ok(text) = event.unescape()
                {
                    fields
                        .entry(field_name.clone())
                        .or_default()
                        .push_str(&text);
                }
            }
            Ok(Event::End(ref event)) => {
                let name = String::from_utf8_lossy(event.name().as_ref()).into_owned();
                if name == "Game" {
                    in_game = false;
                    let application_path = fields
                        .get("ApplicationPath")
                        .map(|path| path.replace('\\', "/").to_ascii_lowercase())
                        .unwrap_or_default();
                    let database_id = fields
                        .get("DatabaseID")
                        .and_then(|value| value.trim().parse::<i64>().ok())
                        .unwrap_or_default();
                    let mut candidates = Vec::new();
                    for key in ["Name", "Title"] {
                        if let Some(value) = fields
                            .get(key)
                            .map(|value| value.trim().to_owned())
                            .filter(|value| !value.is_empty())
                            && !candidates.contains(&value)
                        {
                            candidates.push(value);
                        }
                    }
                    entries.push(XmlGameEntry {
                        database_id,
                        application_path,
                        names: candidates,
                    });
                    fields.clear();
                } else if field.as_deref() == Some(name.as_str()) {
                    field = None;
                }
            }
            Ok(Event::Eof) => return entries,
            Err(error) => {
                tracing::warn!("could not parse the eXo LaunchBox XML: {error}");
                return entries;
            }
            _ => {}
        }
    }
}

/// Finds the LaunchBox game entry whose application path contains the eXo
/// shortname directory (for example `!dos/qmajik/`), and returns its naming
/// candidates.
fn launchbox_game_names(xml: &str, shortname: &str) -> Vec<String> {
    let needle = format!("/{}/", shortname.to_ascii_lowercase());
    parse_launchbox_xml(xml)
        .into_iter()
        .find(|entry| entry.application_path.contains(&needle))
        .map(|entry| entry.names)
        .unwrap_or_default()
}

/// LaunchBox replaces filename-unsafe characters with underscores when it
/// writes image files (for example `Quest: ...` becomes `Quest_ ...`).
fn launchbox_filename(name: &str) -> String {
    name.chars()
        .map(|character| match character {
            ':' | '/' | '\\' | '*' | '?' | '"' | '<' | '>' | '|' => '_',
            other => other,
        })
        .collect()
}

/// Locates the first image in the pack for the game, preferring the
/// collection's platform folder and falling back to eXo's unclassified
/// `None` folder (used by the Windows packs).
fn find_image_member(
    member_index: &std::collections::HashSet<&str>,
    platform_dir: &str,
    sections: &[&str],
    game_names: &[String],
) -> Option<String> {
    for platform in [platform_dir, "None"] {
        for game_name in game_names {
            for stem in [launchbox_filename(game_name), game_name.clone()] {
                for section in sections {
                    for variant in NAME_VARIANTS {
                        for extension in IMAGE_EXTENSIONS {
                            let member =
                                format!("Images/{platform}/{section}/{stem}{variant}.{extension}");
                            if member_index.contains(member.as_str()) {
                                return Some(member);
                            }
                        }
                    }
                }
            }
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    const GAME_XML: &str = "<?xml version=\"1.0\" standalone=\"yes\"?>\r\n<LaunchBox>\r\n  <Game>\r\n    <GogAppId />\r\n    <OriginAppId />\r\n    <Name>Other Game (1990)</Name>\r\n    <ApplicationPath>!dos\\other\\Other Game (1990).bat</ApplicationPath>\r\n    <Genre>Puzzle</Genre>\r\n  </Game>\r\n  <Game>\r\n    <GogAppId />\r\n    <Title>Quik Majik Adventure</Title>\r\n    <ApplicationPath>eXo\\eXoDOS\\!dos\\qmajik\\Quik Majik Adventure (1991).bat</ApplicationPath>\r\n    <Genre>Role-Playing</Genre>\r\n  </Game>\r\n</LaunchBox>\r\n";

    #[test]
    fn resolves_game_names_from_the_shortname_directory() {
        let names = launchbox_game_names(GAME_XML, "qmajik");
        assert_eq!(names, vec!["Quik Majik Adventure".to_owned()]);
    }

    #[test]
    fn prefers_the_launchbox_name_field_when_present() {
        let xml = "<LaunchBox><Game><Name>With Year (1991)</Name><Title>Quik Majik Adventure</Title><ApplicationPath>!dos\\qmajik\\game.bat</ApplicationPath></Game></LaunchBox>";
        let names = launchbox_game_names(xml, "qmajik");
        assert_eq!(
            names,
            vec![
                "With Year (1991)".to_owned(),
                "Quik Majik Adventure".to_owned()
            ]
        );
    }

    #[test]
    fn requires_the_full_shortname_directory_segment() {
        assert!(launchbox_game_names(GAME_XML, "majik").is_empty());
        assert!(launchbox_game_names(GAME_XML, "missing").is_empty());
    }

    fn pack_test_prepared(pack_path: &Path) -> PreparedInstall {
        PreparedInstall {
            collection: crate::exo_install::ExoCollection::Dos,
            install_root: pack_path.parent().unwrap().to_path_buf(),
            launch_config_path: pack_path.parent().unwrap().join("config.conf"),
            shortname: "qmajik".to_owned(),
            source_archive_path: pack_path
                .parent()
                .unwrap()
                .join("eXo/eXoDOS/Full Release/eXo/eXoDOS/game.zip"),
            reused: false,
        }
    }

    const WIN3X_XML: &str = "<?xml version=\"1.0\" standalone=\"yes\"?>\r\n<LaunchBox>\r\n  <Game>\r\n    <Title>The Suit</Title>\r\n    <ApplicationPath>eXo\\eXoWin3X\\!win3x\\TheSuit\\The Suit (1995).bat</ApplicationPath>\r\n  </Game>\r\n  <Game>\r\n    <Title>Pack 3: Deluxe</Title>\r\n    <ApplicationPath>eXo\\eXoWin3x\\!win3x\\MSEnt3\\Pack 3_ Deluxe (1991).bat</ApplicationPath>\r\n  </Game>\r\n</LaunchBox>\r\n";

    fn write_windows_pack_zip(path: &Path) {
        use zip::ZipWriter;
        use zip::write::SimpleFileOptions;

        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent).unwrap();
        }
        let file = fs::File::create(path).unwrap();
        let mut archive = ZipWriter::new(file);
        archive
            .start_file("xml/Windows 3x.xml", SimpleFileOptions::default())
            .unwrap();
        std::io::Write::write_all(&mut archive, WIN3X_XML.as_bytes()).unwrap();
        archive
            .start_file(
                "Images/Windows 3x/Box - Front/Pack 3_ Deluxe-00.jpg",
                SimpleFileOptions::default(),
            )
            .unwrap();
        std::io::Write::write_all(&mut archive, b"win3x-front-bytes").unwrap();
        archive
            .start_file(
                "Images/None/Box - Front/The Suit-01.jpg",
                SimpleFileOptions::default(),
            )
            .unwrap();
        std::io::Write::write_all(&mut archive, b"none-fallback-bytes").unwrap();
        archive.finish().unwrap();
    }

    #[test]
    fn imports_windows_pack_artwork_with_sanitized_names_and_none_fallback() {
        let temp = tempfile::tempdir().unwrap();
        let pack_path = temp.path().join("eXo/eXoWin3x/Content/XOWin3xMetadata.zip");
        write_windows_pack_zip(&pack_path);
        let prepared = PreparedInstall {
            collection: crate::exo_install::ExoCollection::Win3x,
            install_root: temp.path().to_path_buf(),
            launch_config_path: temp.path().join("config.conf"),
            shortname: "MSEnt3".to_owned(),
            source_archive_path: temp
                .path()
                .join("eXo/eXoWin3x/eXo/eXoWin3x/Pack 3 (1991).zip"),
            reused: false,
        };
        let media_root = temp.path().join("media");

        let imported = import_prepared_game_media(&media_root, 7, &prepared).unwrap();
        assert_eq!(imported, vec![ArtworkKind::BoxFront]);
        let front = media_root.join("lb-7/launchbox/box-front.jpg");
        assert_eq!(fs::read(&front).unwrap(), b"win3x-front-bytes");
        assert!(
            !media_root
                .join("lb-7/launchbox/screenshot-gameplay.png")
                .exists()
        );

        // A game whose images live in the unclassified None folder resolves too.
        let none_prepared = PreparedInstall {
            collection: crate::exo_install::ExoCollection::Win3x,
            install_root: temp.path().to_path_buf(),
            launch_config_path: temp.path().join("config2.conf"),
            shortname: "TheSuit".to_owned(),
            source_archive_path: prepared.source_archive_path.clone(),
            reused: false,
        };
        let imported = import_prepared_game_media(&media_root, 8, &none_prepared).unwrap();
        assert_eq!(imported, vec![ArtworkKind::BoxFront]);
        let none_front = media_root.join("lb-8/launchbox/box-front.jpg");
        assert_eq!(fs::read(&none_front).unwrap(), b"none-fallback-bytes");
    }

    #[test]
    fn resolves_flat_windows_platform_xml_directories() {
        let members: Vec<String> = vec![
            "xml/Windows 3x.xml".to_owned(),
            "xml/Playlists/Some Playlist.xml".to_owned(),
            "Images/Windows 3x/Box - Front/x-01.jpg".to_owned(),
        ];
        let index: std::collections::HashSet<&str> = members.iter().map(String::as_str).collect();
        assert_eq!(
            resolve_platform_dir(&index, crate::exo_install::ExoCollection::Win3x),
            Some("Windows 3x".to_owned())
        );
        assert_eq!(
            resolve_xml_member(&index, crate::exo_install::ExoCollection::Win3x),
            Some("xml/Windows 3x.xml".to_owned())
        );
        assert_eq!(
            resolve_platform_dir(&index, crate::exo_install::ExoCollection::Dos),
            None
        );
        let dos_members: Vec<String> = vec![
            "xml/MS-DOS.xml".to_owned(),
            "xml/WinFAMILY.xml".to_owned(),
            "Images/MS-DOS/Box - Front/x-01.jpg".to_owned(),
        ];
        let dos_index: std::collections::HashSet<&str> =
            dos_members.iter().map(String::as_str).collect();
        assert_eq!(
            resolve_platform_dir(&dos_index, crate::exo_install::ExoCollection::Dos),
            Some("MS-DOS".to_owned())
        );
        assert_eq!(
            resolve_xml_member(&dos_index, crate::exo_install::ExoCollection::Dos),
            Some("xml/MS-DOS.xml".to_owned())
        );
    }

    #[test]
    fn resolves_era_named_win9x_xml_and_matching_image_directory() {
        // Newer eXoWin9x packs name the platform XML by era without an
        // extension while keeping images under Images/Windows 9x/.
        let members: Vec<String> = vec![
            "xml/all/1994-1996.9x".to_owned(),
            "xml/family/1994-1996.9x".to_owned(),
            "xml/merge_9xall.bat".to_owned(),
            "xml/Playlists/Installed eXo9x Games.xml".to_owned(),
            "Images/Windows 9x/Box - Front/x-01.png".to_owned(),
            "Images/None/Box - Front/y-01.jpg".to_owned(),
            "Images/Badges/Favorite.png".to_owned(),
        ];
        let index: std::collections::HashSet<&str> = members.iter().map(String::as_str).collect();
        assert_eq!(
            resolve_xml_member(&index, crate::exo_install::ExoCollection::Win9x),
            Some("xml/all/1994-1996.9x".to_owned())
        );
        assert_eq!(
            resolve_platform_dir(&index, crate::exo_install::ExoCollection::Win9x),
            Some("Windows 9x".to_owned())
        );
    }

    const BULK_XML: &str = "<?xml version=\"1.0\" standalone=\"yes\"?>\r\n<LaunchBox>\r\n  <Game>\r\n    <Title>Alpha Game</Title>\r\n    <DatabaseID>100</DatabaseID>\r\n    <ApplicationPath>eXo\\eXoDOS\\!dos\\alpha\\Alpha Game (1990).bat</ApplicationPath>\r\n  </Game>\r\n  <Game>\r\n    <Title>Beta: The Game</Title>\r\n    <DatabaseID>101</DatabaseID>\r\n    <ApplicationPath>eXo\\eXoDOS\\!dos\\beta\\Beta Game (1991).bat</ApplicationPath>\r\n  </Game>\r\n  <Game>\r\n    <Title>No Database ID</Title>\r\n    <ApplicationPath>eXo\\eXoDOS\\!dos\\gamma\\gamma.bat</ApplicationPath>\r\n  </Game>\r\n</LaunchBox>\r\n";

    fn write_bulk_pack_zip(path: &Path) {
        use zip::ZipWriter;
        use zip::write::SimpleFileOptions;

        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent).unwrap();
        }
        let file = fs::File::create(path).unwrap();
        let mut archive = ZipWriter::new(file);
        archive
            .start_file("xml/all/MS-DOS.xml", SimpleFileOptions::default())
            .unwrap();
        std::io::Write::write_all(&mut archive, BULK_XML.as_bytes()).unwrap();
        archive
            .start_file(
                "Images/MS-DOS/Box - Front/Alpha Game-01.jpg",
                SimpleFileOptions::default(),
            )
            .unwrap();
        std::io::Write::write_all(&mut archive, b"alpha-front").unwrap();
        archive
            .start_file(
                "Images/MS-DOS/Screenshot - Gameplay/Beta_ The Game-01.png",
                SimpleFileOptions::default(),
            )
            .unwrap();
        std::io::Write::write_all(&mut archive, b"beta-shot").unwrap();
        archive.finish().unwrap();
    }

    #[test]
    fn collection_import_covers_every_pack_game_by_database_id() {
        let temp = tempfile::tempdir().unwrap();
        let pack_path = temp
            .path()
            .join("eXo/eXoDOS/Full Release/Content/XODOSMetadata.zip");
        write_bulk_pack_zip(&pack_path);
        let anchor = temp
            .path()
            .join("eXo/eXoDOS/Full Release/eXo/eXoDOS/game.zip");
        fs::create_dir_all(anchor.parent().unwrap()).unwrap();
        fs::write(&anchor, b"archive").unwrap();
        let media_root = temp.path().join("media");
        let cancelled = std::sync::atomic::AtomicBool::new(false);

        let summary = import_collection_media(
            &media_root,
            crate::exo_install::ExoCollection::Dos,
            std::slice::from_ref(&anchor),
            |_, _| {},
            &cancelled,
        )
        .unwrap()
        .expect("pack should import");
        assert_eq!(summary.games_in_pack, 2);
        assert_eq!(summary.games_imported, 2);
        assert_eq!(summary.assets_imported, 2);
        assert!(!summary.cancelled);
        assert_eq!(
            fs::read(media_root.join("lb-100/launchbox/box-front.jpg")).unwrap(),
            b"alpha-front"
        );
        assert_eq!(
            fs::read(media_root.join("lb-101/launchbox/screenshot-gameplay.png")).unwrap(),
            b"beta-shot"
        );
        // Games without a LaunchBox DatabaseID are never keyed into the cache.
        assert!(!media_root.join("lb-0").exists());
    }

    #[test]
    fn collection_import_is_idempotent_after_completion() {
        let temp = tempfile::tempdir().unwrap();
        let pack_path = temp
            .path()
            .join("eXo/eXoDOS/Full Release/Content/XODOSMetadata.zip");
        write_bulk_pack_zip(&pack_path);
        let anchor = temp
            .path()
            .join("eXo/eXoDOS/Full Release/eXo/eXoDOS/game.zip");
        fs::create_dir_all(anchor.parent().unwrap()).unwrap();
        fs::write(&anchor, b"archive").unwrap();
        let media_root = temp.path().join("media");
        let cancelled = std::sync::atomic::AtomicBool::new(false);

        import_collection_media(
            &media_root,
            crate::exo_install::ExoCollection::Dos,
            std::slice::from_ref(&anchor),
            |_, _| {},
            &cancelled,
        )
        .unwrap()
        .unwrap();
        // Removing the extracted art must not resurrect it: the pack marker
        // records that this exact pack was fully processed.
        fs::remove_file(media_root.join("lb-100/launchbox/box-front.jpg")).unwrap();
        let second = import_collection_media(
            &media_root,
            crate::exo_install::ExoCollection::Dos,
            std::slice::from_ref(&anchor),
            |_, _| {},
            &cancelled,
        )
        .unwrap();
        assert!(second.is_none());
        assert!(!media_root.join("lb-100/launchbox/box-front.jpg").exists());
    }

    #[test]
    fn cancelled_collection_import_leaves_no_pack_marker() {
        let temp = tempfile::tempdir().unwrap();
        let pack_path = temp
            .path()
            .join("eXo/eXoDOS/Full Release/Content/XODOSMetadata.zip");
        write_bulk_pack_zip(&pack_path);
        let anchor = temp
            .path()
            .join("eXo/eXoDOS/Full Release/eXo/eXoDOS/game.zip");
        fs::create_dir_all(anchor.parent().unwrap()).unwrap();
        fs::write(&anchor, b"archive").unwrap();
        let media_root = temp.path().join("media");
        let cancelled = std::sync::atomic::AtomicBool::new(true);

        let summary = import_collection_media(
            &media_root,
            crate::exo_install::ExoCollection::Dos,
            std::slice::from_ref(&anchor),
            |_, _| {},
            &cancelled,
        )
        .unwrap()
        .expect("pack should still be located");
        assert!(summary.cancelled);
        assert!(!media_root.join(".exodos-pack-imported").exists());

        cancelled.store(false, std::sync::atomic::Ordering::Relaxed);
        let resumed = import_collection_media(
            &media_root,
            crate::exo_install::ExoCollection::Dos,
            std::slice::from_ref(&anchor),
            |_, _| {},
            &cancelled,
        )
        .unwrap()
        .unwrap();
        assert_eq!(resumed.games_imported, 2);
    }

    fn write_pack_zip(path: &Path) {
        use zip::ZipWriter;
        use zip::write::SimpleFileOptions;

        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent).unwrap();
        }
        let file = fs::File::create(path).unwrap();
        let mut archive = ZipWriter::new(file);
        archive
            .start_file("xml/all/MS-DOS.xml", SimpleFileOptions::default())
            .unwrap();
        std::io::Write::write_all(&mut archive, GAME_XML.as_bytes()).unwrap();
        archive
            .start_file(
                "Images/MS-DOS/Box - Front/Quik Majik Adventure-00.jpg",
                SimpleFileOptions::default(),
            )
            .unwrap();
        std::io::Write::write_all(&mut archive, b"front-bytes").unwrap();
        archive
            .start_file(
                "Images/MS-DOS/Fanart - Background/Quik Majik Adventure-01.jpg",
                SimpleFileOptions::default(),
            )
            .unwrap();
        std::io::Write::write_all(&mut archive, b"fanart-bytes").unwrap();
        archive.finish().unwrap();
    }

    #[test]
    fn imports_available_artwork_into_the_launchbox_media_cache() {
        let temp = tempfile::tempdir().unwrap();
        let pack_path = temp
            .path()
            .join("eXo/eXoDOS/Full Release/Content/XODOSMetadata.zip");
        write_pack_zip(&pack_path);
        let prepared = pack_test_prepared(&pack_path);
        let media_root = temp.path().join("media");

        let imported = import_prepared_game_media(&media_root, 42, &prepared).unwrap();
        assert_eq!(imported, vec![ArtworkKind::BoxFront, ArtworkKind::Fanart]);
        let front = media_root.join("lb-42/launchbox/box-front.jpg");
        assert_eq!(fs::read(&front).unwrap(), b"front-bytes");
        let fanart = media_root.join("lb-42/launchbox/fanart-background.jpg");
        assert_eq!(fs::read(&fanart).unwrap(), b"fanart-bytes");
        assert!(!media_root.join("lb-42/launchbox/box-back.jpg").exists());
    }

    #[test]
    fn skips_import_once_the_completion_marker_exists() {
        let temp = tempfile::tempdir().unwrap();
        let pack_path = temp
            .path()
            .join("eXo/eXoDOS/Full Release/Content/XODOSMetadata.zip");
        write_pack_zip(&pack_path);
        let prepared = pack_test_prepared(&pack_path);
        let media_root = temp.path().join("media");

        import_prepared_game_media(&media_root, 42, &prepared).unwrap();
        let front = media_root.join("lb-42/launchbox/box-front.jpg");
        fs::remove_file(&front).unwrap();

        let imported = import_prepared_game_media(&media_root, 42, &prepared).unwrap();
        assert!(imported.is_empty());
        assert!(!front.exists());
    }

    #[test]
    fn does_nothing_when_no_metadata_pack_is_nearby() {
        let temp = tempfile::tempdir().unwrap();
        let prepared = pack_test_prepared(&temp.path().join("pack.zip"));
        let media_root = temp.path().join("media");
        let imported = import_prepared_game_media(&media_root, 42, &prepared).unwrap();
        assert!(imported.is_empty());
        assert!(!media_root.join("lb-42").exists());
    }
}
