use cxx_qt_build::{CxxQtBuilder, QmlModule};
use quick_xml::Reader;
use quick_xml::events::Event;
use std::fmt::Write as _;
use std::fs;
use std::io::BufReader;
use std::path::{Path, PathBuf};

#[derive(Default)]
struct GameFields {
    database_id: Option<i64>,
    title: Option<String>,
    source: Option<String>,
    notes: Option<String>,
    genre: Option<String>,
    clone_of: Option<String>,
    application_path: Option<String>,
    version: Option<String>,
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum GeneratedArcadeSubtype {
    Standard,
    Pinball,
    Laserdisc,
}

impl GeneratedArcadeSubtype {
    fn rust_variant_name(self) -> &'static str {
        match self {
            Self::Standard => "Standard",
            Self::Pinball => "Pinball",
            Self::Laserdisc => "Laserdisc",
        }
    }
}

struct GeneratedArcadeEntry {
    database_id: i64,
    title: String,
    source: String,
    subtype: GeneratedArcadeSubtype,
    preferred_lookup: String,
    video_lookup: String,
    lookup_rank: u8,
}

fn main() {
    generate_arcade_lookup();
    let platform_resources = generate_platform_resources();

    CxxQtBuilder::new_qml_module(
        QmlModule::new("Lunchbox")
            .depend("QtQuick.Dialogs")
            .depend("QtQuick.Layouts")
            .depend("QtMultimedia")
            .qml_files([
                "qml/AcceleratedWheelHandler.qml",
                "qml/AlphabetRail.qml",
                "qml/ArtworkMat.qml",
                "qml/AuditMetric.qml",
                "qml/CatalogLinkButton.qml",
                "qml/ClearableSearchField.qml",
                "qml/CollectionMetric.qml",
                "qml/GameFileIdentityDialog.qml",
                "qml/CollectionMemberPresentationDialog.qml",
                "qml/CouchAudioSettings.qml",
                "qml/CouchBackgroundMusic.qml",
                "qml/CouchDownloadScreen.qml",
                "qml/CouchGameShelf.qml",
                "qml/CouchLaunchScreen.qml",
                "qml/CouchModeView.qml",
                "qml/Box3DViewer.qml",
                "qml/BulkMetadataEditor.qml",
                "qml/DownloadReviewDialog.qml",
                "qml/EmuMoviesTransferStatus.qml",
                "qml/FilterToggle.qml",
                "qml/FirmwareAuditView.qml",
                "qml/FirstRunSetup.qml",
                "qml/GameDownloadStatus.qml",
                "qml/GameDetailsStatusCard.qml",
                "qml/GameFilesCard.qml",
                "qml/GamePlayHero.qml",
                "qml/GameSoundtrackCard.qml",
                "qml/HeaderButton.qml",
                "qml/HorizontalWheelHandler.qml",
                "qml/HoverPreviewPresentation.qml",
                "qml/InlineProgressBar.qml",
                "qml/LaunchCommandPreview.qml",
                "qml/LibraryAuditHistory.qml",
                "qml/LoadedTorrentPicker.qml",
                "qml/LocalProviderManifests.qml",
                "qml/MetadataField.qml",
                "qml/MediaDownloadStatus.qml",
                "qml/MediaRepairRecoveryBanner.qml",
                "qml/MediaRetryController.qml",
                "qml/NativeTextArea.qml",
                "qml/RomDownloadStatus.qml",
                "qml/RomScanScheduleCard.qml",
                "qml/RetryingMediaPlayer.qml",
                "qml/ScreenScraperSettings.qml",
                "qml/SecretField.qml",
                "qml/SidebarNavButton.qml",
                "qml/SettingsNavButton.qml",
                "qml/StatusPill.qml",
                "qml/TorrentPayloadPicker.qml",
                "qml/ViewportCardGeometry.qml",
                "qml/WatchedTorrentInbox.qml",
                "qml/Main.qml",
            ]),
    )
    .qrc(platform_resources)
    .qt_module("Quick")
    .qt_module("QuickControls2")
    .qt_module("Quick3D")
    .qt_module("Multimedia")
    .file("src/collection_identity_model.rs")
    .file("src/download_queue_model.rs")
    .file("src/emumovies_model.rs")
    .file("src/emulator_manager_model.rs")
    .file("src/emulator_update_model.rs")
    .file("src/external_torrent_model.rs")
    .file("src/firmware_audit_model.rs")
    .file("src/game_details_model.rs")
    .file("src/gamepad_input.rs")
    .file("src/igdb_model.rs")
    .file("src/launch_profile_manager_model.rs")
    .file("src/library_audit_model.rs")
    .file("src/library_model.rs")
    .file("src/media_audit_model.rs")
    .file("src/local_import_model.rs")
    .file("src/local_provider_manifest_model.rs")
    .file("src/screenscraper_model.rs")
    .file("src/settings_model.rs")
    .file("src/steamgriddb_model.rs")
    .file("src/web_artwork_model.rs")
    .file("src/watched_torrent_model.rs")
    .build();
}

fn generate_platform_resources() -> PathBuf {
    let manifest_directory =
        PathBuf::from(std::env::var_os("CARGO_MANIFEST_DIR").expect("CARGO_MANIFEST_DIR not set"));
    let icon_directory = manifest_directory.join("../../assets/platforms");
    let search_icon = manifest_directory.join("qml/icons/search.svg");
    println!("cargo:rerun-if-changed={}", icon_directory.display());
    println!("cargo:rerun-if-changed={}", search_icon.display());

    let mut icons = fs::read_dir(&icon_directory)
        .unwrap_or_else(|error| {
            panic!(
                "failed to read platform icons at {}: {error}",
                icon_directory.display()
            )
        })
        .filter_map(Result::ok)
        .map(|entry| entry.path())
        .filter(|path| path.extension().is_some_and(|extension| extension == "png"))
        .collect::<Vec<_>>();
    icons.sort();

    let mut qrc = String::from("<RCC>\n  <qresource prefix=\"/platforms\">\n");
    for path in icons {
        println!("cargo:rerun-if-changed={}", path.display());
        let name = path
            .file_name()
            .and_then(|name| name.to_str())
            .expect("platform icon filename is not UTF-8");
        writeln!(qrc, "    <file alias=\"{name}\">{}</file>", path.display())
            .expect("writing platform resource manifest");
    }
    qrc.push_str("  </qresource>\n");
    writeln!(
        qrc,
        "  <qresource prefix=\"/qt/qml/Lunchbox/qml/icons\">\n    <file alias=\"search.svg\">{}</file>\n  </qresource>",
        search_icon.display()
    )
    .expect("writing search icon resource manifest");
    qrc.push_str("</RCC>\n");

    let output = PathBuf::from(std::env::var_os("OUT_DIR").expect("OUT_DIR not set"))
        .join("platform_icons.qrc");
    fs::write(&output, qrc).expect("failed to write platform icon resource manifest");
    output
}

fn arcade_source_path() -> PathBuf {
    std::env::var_os("LUNCHBOX_ARCADE_XML")
        .map(PathBuf::from)
        .unwrap_or_else(|| {
            Path::new(env!("CARGO_MANIFEST_DIR"))
                .join("../..")
                .join("launchbox-data")
                .join("Arcade.xml")
        })
}

fn generate_arcade_lookup() {
    println!("cargo:rerun-if-env-changed=LUNCHBOX_ARCADE_XML");
    let source_path = arcade_source_path();
    println!("cargo:rerun-if-changed={}", source_path.display());

    let out_dir = std::env::var("OUT_DIR").expect("OUT_DIR not set");
    let output_path = Path::new(&out_dir).join("arcade_lookup.rs");

    let Ok(file) = fs::File::open(&source_path) else {
        fs::write(
            &output_path,
            "pub static ARCADE_LOOKUP: &[ArcadeLookupEntry] = &[];\n",
        )
        .expect("failed to write empty arcade lookup");
        return;
    };

    let mut reader = Reader::from_reader(BufReader::new(file));
    reader.config_mut().trim_text(true);

    let mut buffer = Vec::new();
    let mut current_game: Option<GameFields> = None;
    let mut current_field: Option<String> = None;
    let mut entries = Vec::new();

    loop {
        match reader.read_event_into(&mut buffer) {
            Ok(Event::Start(ref event)) => {
                let tag_name = String::from_utf8_lossy(event.name().as_ref()).to_string();
                if tag_name == "Game" {
                    current_game = Some(GameFields::default());
                    current_field = None;
                } else if current_game.is_some() {
                    current_field = Some(tag_name);
                }
            }
            Ok(Event::End(ref event)) => {
                let tag_name = String::from_utf8_lossy(event.name().as_ref()).to_string();
                if tag_name == "Game" {
                    if let Some(game) = current_game.take()
                        && let Some(database_id) = game.database_id
                    {
                        let subtype = classify_arcade_subtype(&game);
                        let lookup = choose_arcade_lookup(&game);
                        if subtype != GeneratedArcadeSubtype::Standard || lookup.is_some() {
                            let (lookup_rank, preferred_lookup, video_lookup) =
                                lookup.unwrap_or_else(|| (0, String::new(), String::new()));
                            entries.push(GeneratedArcadeEntry {
                                database_id,
                                title: game.title.unwrap_or_default(),
                                source: game.source.unwrap_or_default(),
                                subtype,
                                preferred_lookup,
                                video_lookup,
                                lookup_rank,
                            });
                        }
                    }
                    current_field = None;
                } else if current_game.is_some() {
                    current_field = None;
                }
            }
            Ok(Event::Text(ref event)) => {
                if let (Some(game), Some(field)) = (&mut current_game, &current_field) {
                    let text = event.unescape().unwrap_or_default().to_string();
                    if !text.is_empty() {
                        match field.as_str() {
                            "ApplicationPath" => game.application_path = Some(text),
                            "CloneOf" => game.clone_of = Some(text),
                            "DatabaseID" => game.database_id = text.parse().ok(),
                            "Genre" => game.genre = Some(text),
                            "Notes" => game.notes = Some(text),
                            "Source" => game.source = Some(text),
                            "Title" => game.title = Some(text),
                            "Version" => game.version = Some(text),
                            _ => {}
                        }
                    }
                }
            }
            Ok(Event::Eof) => break,
            Err(error) => panic!(
                "failed to parse arcade lookup source {}: {error}",
                source_path.display()
            ),
            _ => {}
        }
        buffer.clear();
    }

    entries.sort_by(
        |left: &GeneratedArcadeEntry, right: &GeneratedArcadeEntry| {
            left.database_id
                .cmp(&right.database_id)
                .then_with(|| left.title.cmp(&right.title))
                .then_with(|| left.preferred_lookup.cmp(&right.preferred_lookup))
                .then_with(|| left.source.cmp(&right.source))
        },
    );

    let mut generated = String::from("pub static ARCADE_LOOKUP: &[ArcadeLookupEntry] = &[\n");
    for entry in entries {
        generated.push_str(&format!(
            "    ArcadeLookupEntry {{ database_id: {}, title: {:?}, source: {:?}, subtype: ArcadeSubtype::{}, preferred_lookup: {:?}, video_lookup: {:?}, lookup_rank: {} }},\n",
            entry.database_id,
            entry.title,
            entry.source,
            entry.subtype.rust_variant_name(),
            entry.preferred_lookup,
            entry.video_lookup,
            entry.lookup_rank
        ));
    }
    generated.push_str("];\n");
    fs::write(&output_path, generated).expect("failed to write arcade lookup");
}

fn choose_arcade_lookup(game: &GameFields) -> Option<(u8, String, String)> {
    let preferred_lookup = game
        .application_path
        .as_deref()
        .and_then(rom_stem_from_path)
        .map(|stem| (3, stem.to_ascii_lowercase()))
        .or_else(|| {
            game.version.as_deref().and_then(|version| {
                let version = version.trim();
                looks_like_romset_id(version).then(|| (2, version.to_ascii_lowercase()))
            })
        })
        .or_else(|| {
            game.clone_of.as_deref().and_then(|clone_of| {
                let clone_of = clone_of.trim();
                (!clone_of.is_empty()).then(|| (1, clone_of.to_ascii_lowercase()))
            })
        })?;

    let video_lookup = game
        .clone_of
        .as_deref()
        .map(str::trim)
        .filter(|clone_of| !clone_of.is_empty())
        .map(str::to_ascii_lowercase)
        .unwrap_or_else(|| preferred_lookup.1.clone());

    Some((preferred_lookup.0, preferred_lookup.1, video_lookup))
}

fn classify_arcade_subtype(game: &GameFields) -> GeneratedArcadeSubtype {
    let normalized = game
        .source
        .as_deref()
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(|value| value.replace('\\', "/").to_ascii_lowercase())
        .unwrap_or_default();
    if normalized.starts_with("pinball/") {
        return GeneratedArcadeSubtype::Pinball;
    }

    if matches!(
        normalized.as_str(),
        "amiga/alg.cpp"
            | "atari/firefox.cpp"
            | "cinematronics/dlair.cpp"
            | "cinematronics/dlair2.cpp"
            | "dataeast/deco_ld.cpp"
            | "misc/cubeqst.cpp"
            | "misc/istellar.cpp"
            | "misc/thayers.cpp"
            | "sega/gpworld.cpp"
            | "sega/segald.cpp"
            | "sega/timetrv.cpp"
            | "stern/cliffhgr.cpp"
            | "universal/superdq.cpp"
    ) {
        return GeneratedArcadeSubtype::Laserdisc;
    }

    let genre = game
        .genre
        .as_deref()
        .map(str::trim)
        .unwrap_or_default()
        .to_ascii_lowercase();
    if genre.contains("laserdisc") {
        return GeneratedArcadeSubtype::Laserdisc;
    }

    let notes = game
        .notes
        .as_deref()
        .map(str::trim)
        .unwrap_or_default()
        .to_ascii_lowercase();
    if [
        "arcade laserdisc game",
        "game on laserdisc",
        "laserdisc fmv arcade game",
        "laserdisc game",
        "laserdisc video game",
        "laserdisc-based",
        "laserdisc based",
        "laserdisc-streamed",
        "laserdisc streamed",
        "live-action laserdisc",
        "only laserdisc game",
    ]
    .iter()
    .any(|phrase| notes.contains(phrase))
    {
        return GeneratedArcadeSubtype::Laserdisc;
    }

    let version = game
        .version
        .as_deref()
        .map(str::trim)
        .unwrap_or_default()
        .to_ascii_lowercase();
    if [
        "pioneer ld",
        "sony ld",
        "data east ld",
        "amld",
        "(dld)",
        " laserdisc",
    ]
    .iter()
    .any(|phrase| version.contains(phrase))
    {
        return GeneratedArcadeSubtype::Laserdisc;
    }

    GeneratedArcadeSubtype::Standard
}

fn rom_stem_from_path(path: &str) -> Option<&str> {
    let file_name = path
        .rsplit(['\\', '/'])
        .next()
        .filter(|segment| !segment.is_empty())?;
    let stem = file_name
        .rsplit_once('.')
        .map(|(stem, _)| stem)
        .unwrap_or(file_name);
    (!stem.is_empty()).then_some(stem)
}

fn looks_like_romset_id(value: &str) -> bool {
    !value.is_empty()
        && value.len() <= 24
        && value
            .bytes()
            .all(|byte| byte.is_ascii_lowercase() || byte.is_ascii_digit() || byte == b'_')
}
