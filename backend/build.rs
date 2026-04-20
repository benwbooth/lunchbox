use chrono::TimeZone;
use quick_xml::Reader;
use quick_xml::events::Event;
use std::fs;
use std::io::BufReader;
use std::path::Path;
use std::process::Command;

#[derive(Default)]
struct GameFields {
    database_id: Option<i64>,
    title: Option<String>,
    source: Option<String>,
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
    // Generate content hash from git
    let hash = Command::new("git")
        .args(["rev-parse", "--short", "HEAD"])
        .output()
        .map(|o| String::from_utf8_lossy(&o.stdout).trim().to_string())
        .unwrap_or_else(|_| "unknown".to_string());
    println!("cargo:rustc-env=BUILD_HASH={}", hash);

    // Generate build timestamp with timezone abbreviation (e.g., PST, EST)
    let now = chrono::Local::now();
    let tz_abbrev = iana_time_zone::get_timezone()
        .ok()
        .and_then(|tz_name| tz_name.parse::<chrono_tz::Tz>().ok())
        .map(|tz| {
            tz.from_utc_datetime(&now.naive_utc())
                .format("%Z")
                .to_string()
        })
        .unwrap_or_default();
    let timestamp = format!("{} {}", now.format("%Y-%m-%d %H:%M:%S"), tz_abbrev);
    println!("cargo:rustc-env=BUILD_TIMESTAMP={}", timestamp);

    generate_arcade_lookup();

    // Rerun if git state changes
    println!("cargo:rerun-if-changed=../.git/HEAD");
    println!("cargo:rerun-if-changed=../.git/index");
    println!("cargo:rerun-if-changed=../.git/refs/heads");
}

fn generate_arcade_lookup() {
    let source_path = Path::new("../launchbox-data/Arcade.xml");
    println!("cargo:rerun-if-changed={}", source_path.display());

    let out_dir = std::env::var("OUT_DIR").expect("OUT_DIR not set");
    let output_path = Path::new(&out_dir).join("arcade_lookup.rs");

    let Ok(file) = fs::File::open(source_path) else {
        let empty = "pub static ARCADE_LOOKUP: &[ArcadeLookupEntry] = &[];\n";
        fs::write(&output_path, empty).expect("failed to write empty arcade lookup");
        return;
    };

    let mut reader = Reader::from_reader(BufReader::new(file));
    reader.config_mut().trim_text(true);

    let mut buf = Vec::new();
    let mut current_game: Option<GameFields> = None;
    let mut current_field: Option<String> = None;
    let mut entries: Vec<GeneratedArcadeEntry> = Vec::new();

    loop {
        match reader.read_event_into(&mut buf) {
            Ok(Event::Start(ref e)) => {
                let tag_name = String::from_utf8_lossy(e.name().as_ref()).to_string();
                if tag_name == "Game" {
                    current_game = Some(GameFields::default());
                    current_field = None;
                } else if current_game.is_some() {
                    current_field = Some(tag_name);
                }
            }
            Ok(Event::End(ref e)) => {
                let tag_name = String::from_utf8_lossy(e.name().as_ref()).to_string();
                if tag_name == "Game" {
                    if let Some(game) = current_game.take() {
                        if let Some(database_id) = game.database_id {
                            let subtype = classify_arcade_subtype(game.source.as_deref());
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
                    }
                    current_field = None;
                } else if current_game.is_some() {
                    current_field = None;
                }
            }
            Ok(Event::Text(ref e)) => {
                if let (Some(game), Some(field)) = (&mut current_game, &current_field) {
                    let text = e.unescape().unwrap_or_default().to_string();
                    if text.is_empty() {
                        buf.clear();
                        continue;
                    }

                    match field.as_str() {
                        "ApplicationPath" => game.application_path = Some(text),
                        "CloneOf" => game.clone_of = Some(text),
                        "DatabaseID" => game.database_id = text.parse().ok(),
                        "Source" => game.source = Some(text),
                        "Title" => game.title = Some(text),
                        "Version" => game.version = Some(text),
                        _ => {}
                    }
                }
            }
            Ok(Event::Eof) => break,
            Err(e) => panic!("Failed to parse Arcade.xml for lookup generation: {}", e),
            _ => {}
        }

        buf.clear();
    }

    entries.sort_by(|left, right| {
        left.database_id
            .cmp(&right.database_id)
            .then_with(|| left.title.cmp(&right.title))
            .then_with(|| left.preferred_lookup.cmp(&right.preferred_lookup))
            .then_with(|| left.source.cmp(&right.source))
    });

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

fn choose_arcade_lookup(game: &impl ArcadeLookupFields) -> Option<(u8, String, String)> {
    let preferred_lookup = game
        .application_path()
        .and_then(rom_stem_from_path)
        .map(|stem| (3, stem.to_ascii_lowercase()))
        .or_else(|| {
            game.version().and_then(|version| {
                let version = version.trim();
                looks_like_romset_id(version).then(|| (2, version.to_ascii_lowercase()))
            })
        })
        .or_else(|| {
            game.clone_of().and_then(|clone_of| {
                let clone_of = clone_of.trim();
                (!clone_of.is_empty()).then(|| (1, clone_of.to_ascii_lowercase()))
            })
        })?;

    let video_lookup = game
        .clone_of()
        .map(str::trim)
        .filter(|clone_of| !clone_of.is_empty())
        .map(|clone_of| clone_of.to_ascii_lowercase())
        .unwrap_or_else(|| preferred_lookup.1.clone());

    Some((preferred_lookup.0, preferred_lookup.1, video_lookup))
}

trait ArcadeLookupFields {
    fn clone_of(&self) -> Option<&str>;
    fn application_path(&self) -> Option<&str>;
    fn version(&self) -> Option<&str>;
}

impl ArcadeLookupFields for GameFields {
    fn clone_of(&self) -> Option<&str> {
        self.clone_of.as_deref()
    }

    fn application_path(&self) -> Option<&str> {
        self.application_path.as_deref()
    }

    fn version(&self) -> Option<&str> {
        self.version.as_deref()
    }
}

fn classify_arcade_subtype(source: Option<&str>) -> GeneratedArcadeSubtype {
    let Some(source) = source.map(str::trim).filter(|value| !value.is_empty()) else {
        return GeneratedArcadeSubtype::Standard;
    };

    let normalized = source.replace('\\', "/").to_ascii_lowercase();
    if normalized.starts_with("pinball/") {
        return GeneratedArcadeSubtype::Pinball;
    }

    if matches!(
        normalized.as_str(),
        "atari/firefox.cpp"
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
    if stem.is_empty() { None } else { Some(stem) }
}

fn looks_like_romset_id(value: &str) -> bool {
    !value.is_empty()
        && value.len() <= 24
        && value
            .bytes()
            .all(|byte| byte.is_ascii_lowercase() || byte.is_ascii_digit() || byte == b'_')
}
