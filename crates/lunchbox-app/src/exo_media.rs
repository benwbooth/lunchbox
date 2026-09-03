//! Imports per-game artwork from an eXo collection metadata pack.
//!
//! The eXo projects distribute a metadata pack (for example
//! `XODOSMetadata.zip` under `Content/`) that contains the LaunchBox
//! platform XML plus one image file per game and media type. The game is
//! identified by its exact eXo shortname via the XML `ApplicationPath`, so
//! an import never relies on a fuzzy title match.

use std::collections::BTreeMap;
use std::fs;
use std::io::Read;
use std::path::{Path, PathBuf};

use anyhow::{Context, Result, bail};
use quick_xml::Reader;
use quick_xml::events::Event;
use zip::ZipArchive;

use crate::exo_install::{PreparedInstall, locate_relative_to_ancestors};
use crate::media::ArtworkKind;

/// LaunchBox XML media sections mapped onto Lunchbox artwork kinds, in
/// preference order. Region subfolders are checked after the plain section.
const IMAGE_SECTIONS: &[(ArtworkKind, &[&str])] = &[
    (
        ArtworkKind::BoxFront,
        &[
            "Box - Front",
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
    let platform_dir = resolve_platform_dir(&member_names, prepared.collection)
        .context("eXo metadata pack has no XML for this collection platform")?;
    let xml_member = format!("xml/all/{platform_dir}.xml");
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
        if let Some(member) = find_image_member(&member_names, &platform_dir, sections, &game_names)
        {
            let extension = member
                .rsplit('.')
                .next()
                .unwrap_or("jpg")
                .to_ascii_lowercase();
            let target = destination.join(format!("{}.{}", kind.key(), extension));
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

fn locate_metadata_pack(prepared: &PreparedInstall) -> Option<PathBuf> {
    let relatives: &[&str] = match prepared.collection {
        crate::exo_install::ExoCollection::Dos => &[
            "Content/XODOSMetadata.zip",
            "Full Release/Content/XODOSMetadata.zip",
            "Lite Release/Content/XODOSMetadata.zip",
        ],
        crate::exo_install::ExoCollection::Win3x => &[
            "Content/XOWin3xMetadata.zip",
            "Full Release/Content/XOWin3xMetadata.zip",
            "Lite Release/Content/XOWin3xMetadata.zip",
        ],
        crate::exo_install::ExoCollection::Win9x => &[
            "Content/XOWin9xMetadata.zip",
            "Full Release/Content/XOWin9xMetadata.zip",
            "Lite Release/Content/XOWin9xMetadata.zip",
        ],
    };
    // Prepared library files are often symlinks into the original torrent
    // download tree, where the metadata pack lives; resolve them first.
    let source = fs::canonicalize(&prepared.source_archive_path)
        .unwrap_or_else(|_| prepared.source_archive_path.clone());
    locate_relative_to_ancestors(
        &source,
        relatives
            .iter()
            .map(PathBuf::from)
            .collect::<Vec<_>>()
            .as_slice(),
    )
}

fn platform_dir_candidates(
    collection: crate::exo_install::ExoCollection,
) -> &'static [&'static str] {
    match collection {
        crate::exo_install::ExoCollection::Dos => &["MS-DOS"],
        crate::exo_install::ExoCollection::Win3x => &["Windows 3.x", "Win3x"],
        crate::exo_install::ExoCollection::Win9x => &["Windows 9.x", "Win9x", "Windows 9X"],
    }
}

fn resolve_platform_dir(
    member_names: &[String],
    collection: crate::exo_install::ExoCollection,
) -> Option<String> {
    for candidate in platform_dir_candidates(collection) {
        let member = format!("xml/all/{candidate}.xml");
        if member_names.iter().any(|name| name == &member) {
            return Some(candidate.to_string());
        }
    }
    let all: Vec<&String> = member_names
        .iter()
        .filter(|name| {
            let Some(stem) = name.strip_prefix("xml/all/") else {
                return false;
            };
            stem.ends_with(".xml")
        })
        .collect();
    if all.len() == 1 {
        return all[0]
            .strip_prefix("xml/all/")
            .map(|stem| stem.trim_end_matches(".xml").to_owned());
    }
    None
}

fn read_zip_member(archive: &mut ZipArchive<fs::File>, member: &str) -> Result<Vec<u8>> {
    let mut entry = archive
        .by_name(member)
        .with_context(|| format!("opening member {member} in the eXo metadata pack"))?;
    let mut bytes = Vec::new();
    entry.read_to_end(&mut bytes)?;
    Ok(bytes)
}

/// Finds the LaunchBox game entry whose application path contains the eXo
/// shortname directory (for example `!dos/qmajik/`), and returns its naming
/// candidates — eXo mixes LaunchBox `<Name>` and `<Title>` conventions, and
/// image files are keyed by either.
fn launchbox_game_names(xml: &str, shortname: &str) -> Vec<String> {
    let needle = format!("/{}/", shortname.to_ascii_lowercase());
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
                    if application_path.contains(&needle) {
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
                        return candidates;
                    }
                    fields.clear();
                } else if field.as_deref() == Some(name.as_str()) {
                    field = None;
                }
            }
            Ok(Event::Eof) => return Vec::new(),
            Err(error) => {
                tracing::warn!("could not parse the eXo LaunchBox XML: {error}");
                return Vec::new();
            }
            _ => {}
        }
    }
}

fn find_image_member(
    member_names: &[String],
    platform_dir: &str,
    sections: &[&str],
    game_names: &[String],
) -> Option<String> {
    for game_name in game_names {
        for section in sections {
            for variant in NAME_VARIANTS {
                for extension in IMAGE_EXTENSIONS {
                    let member =
                        format!("Images/{platform_dir}/{section}/{game_name}{variant}.{extension}");
                    if member_names.iter().any(|name| name == &member) {
                        return Some(member);
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
        let fanart = media_root.join("lb-42/launchbox/fanart.jpg");
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
