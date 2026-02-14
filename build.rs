use std::collections::hash_map::DefaultHasher;
use std::hash::{Hash, Hasher};
use std::path::Path;
use std::process::Command;

use chrono::TimeZone;

fn main() {
    // Generate content hash from source files (not git commit) so uncommitted
    // changes are reflected in the frontend version display.
    let hash = source_content_hash();
    println!("cargo:rustc-env=BUILD_HASH={}", hash);

    // Generate build timestamp with timezone abbreviation (e.g., PST, EST)
    let now = chrono::Local::now();
    let tz_abbrev = iana_time_zone::get_timezone()
        .ok()
        .and_then(|tz_name| tz_name.parse::<chrono_tz::Tz>().ok())
        .and_then(|tz| Some(tz.from_utc_datetime(&now.naive_utc()).format("%Z").to_string()))
        .unwrap_or_default();
    let timestamp = format!("{} {}", now.format("%Y-%m-%d %H:%M:%S"), tz_abbrev);
    println!("cargo:rustc-env=BUILD_TIMESTAMP={}", timestamp);
}

/// Hash all .rs source files and CSS to produce a short content-based build hash.
/// Falls back to git rev-parse if anything goes wrong.
fn source_content_hash() -> String {
    let dirs = ["src", "styles"];
    let mut paths = Vec::new();
    for dir in &dirs {
        collect_files(Path::new(dir), &mut paths);
    }
    paths.sort();

    if paths.is_empty() {
        return git_hash();
    }

    let mut hasher = DefaultHasher::new();
    for path in &paths {
        if let Ok(content) = std::fs::read(path) {
            path.to_string_lossy().hash(&mut hasher);
            content.hash(&mut hasher);
        }
    }
    format!("{:07x}", hasher.finish() & 0x0FFF_FFFF)
}

fn collect_files(dir: &Path, out: &mut Vec<std::path::PathBuf>) {
    let Ok(entries) = std::fs::read_dir(dir) else {
        return;
    };
    for entry in entries.flatten() {
        let path = entry.path();
        if path.is_dir() {
            collect_files(&path, out);
        } else if let Some(ext) = path.extension().and_then(|e| e.to_str()) {
            if matches!(ext, "rs" | "css" | "html" | "toml") {
                out.push(path);
            }
        }
    }
}

fn git_hash() -> String {
    Command::new("git")
        .args(["rev-parse", "--short", "HEAD"])
        .output()
        .map(|o| String::from_utf8_lossy(&o.stdout).trim().to_string())
        .unwrap_or_else(|_| "unknown".to_string())
}
