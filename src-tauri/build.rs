use std::process::Command;

fn main() {
    tauri_build::build();

    // Generate content hash from git
    let hash = Command::new("git")
        .args(["rev-parse", "--short", "HEAD"])
        .output()
        .map(|o| String::from_utf8_lossy(&o.stdout).trim().to_string())
        .unwrap_or_else(|_| "unknown".to_string());
    println!("cargo:rustc-env=BUILD_HASH={}", hash);

    // Generate build timestamp (local timezone, space-separated)
    let timestamp = chrono::Local::now().format("%Y-%m-%d %H:%M:%S").to_string();
    println!("cargo:rustc-env=BUILD_TIMESTAMP={}", timestamp);

    // Rerun if git HEAD changes
    println!("cargo:rerun-if-changed=.git/HEAD");
}
