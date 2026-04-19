use chrono::TimeZone;
use quick_xml::events::Event;
use quick_xml::Reader;
use std::collections::HashMap;
use std::fs;
use std::io::BufReader;
use std::path::Path;
use std::process::Command;

#[derive(Default)]
struct GameFields {
    database_id: Option<i64>,
    clone_of: Option<String>,
    application_path: Option<String>,
    version: Option<String>,
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

    generate_arcade_video_lookup();

    // Rerun if git state changes
    println!("cargo:rerun-if-changed=../.git/HEAD");
    println!("cargo:rerun-if-changed=../.git/index");
    println!("cargo:rerun-if-changed=../.git/refs/heads");
}

fn generate_arcade_video_lookup() {
    let source_path = Path::new("../launchbox-data/Arcade.xml");
    println!("cargo:rerun-if-changed={}", source_path.display());

    let out_dir = std::env::var("OUT_DIR").expect("OUT_DIR not set");
    let output_path = Path::new(&out_dir).join("arcade_video_lookup.rs");

    let Ok(file) = fs::File::open(source_path) else {
        let empty = "pub static ARCADE_LOOKUP: &[(i64, &str, &str)] = &[];\n";
        fs::write(&output_path, empty).expect("failed to write empty arcade lookup");
        return;
    };

    let mut reader = Reader::from_reader(BufReader::new(file));
    reader.config_mut().trim_text(true);

    let mut buf = Vec::new();
    let mut current_game: Option<GameFields> = None;
    let mut current_field: Option<String> = None;
    let mut entries: HashMap<i64, (u8, String, String)> = HashMap::new();

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
                            if let Some((rank, preferred_lookup, video_lookup)) =
                                choose_arcade_lookup(&game)
                            {
                                match entries.get(&database_id) {
                                    Some((existing_rank, _, _)) if *existing_rank >= rank => {}
                                    _ => {
                                        entries.insert(
                                            database_id,
                                            (rank, preferred_lookup, video_lookup),
                                        );
                                    }
                                }
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
                        "DatabaseID" => game.database_id = text.parse().ok(),
                        "CloneOf" => game.clone_of = Some(text),
                        "ApplicationPath" => game.application_path = Some(text),
                        "Version" => game.version = Some(text),
                        _ => {}
                    }
                }
            }
            Ok(Event::Eof) => break,
            Err(e) => panic!(
                "Failed to parse Arcade.xml for video lookup generation: {}",
                e
            ),
            _ => {}
        }

        buf.clear();
    }

    let mut rows: Vec<(i64, String, String)> = entries
        .into_iter()
        .map(|(database_id, (_, preferred_lookup, video_lookup))| {
            (database_id, preferred_lookup, video_lookup)
        })
        .collect();
    rows.sort_by_key(|(database_id, _, _)| *database_id);

    let mut generated = String::from("pub static ARCADE_LOOKUP: &[(i64, &str, &str)] = &[\n");
    for (database_id, preferred_lookup, video_lookup) in rows {
        generated.push_str(&format!(
            "    ({}, {:?}, {:?}),\n",
            database_id, preferred_lookup, video_lookup
        ));
    }
    generated.push_str("];\n");

    fs::write(&output_path, generated).expect("failed to write arcade video lookup");
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

fn rom_stem_from_path(path: &str) -> Option<&str> {
    let file_name = path
        .rsplit(['\\', '/'])
        .next()
        .filter(|segment| !segment.is_empty())?;
    let stem = file_name
        .rsplit_once('.')
        .map(|(stem, _)| stem)
        .unwrap_or(file_name);
    if stem.is_empty() {
        None
    } else {
        Some(stem)
    }
}

fn looks_like_romset_id(value: &str) -> bool {
    !value.is_empty()
        && value.len() <= 24
        && value
            .bytes()
            .all(|byte| byte.is_ascii_lowercase() || byte.is_ascii_digit() || byte == b'_')
}
