use std::process::Command;
use chrono::TimeZone;

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
        .and_then(|tz| Some(tz.from_utc_datetime(&now.naive_utc()).format("%Z").to_string()))
        .unwrap_or_default();
    let timestamp = format!("{} {}", now.format("%Y-%m-%d %H:%M:%S"), tz_abbrev);
    println!("cargo:rustc-env=BUILD_TIMESTAMP={}", timestamp);

    // Rerun if git state changes
    println!("cargo:rerun-if-changed=.git/HEAD");
    println!("cargo:rerun-if-changed=.git/index");
    println!("cargo:rerun-if-changed=.git/refs/heads");
}
