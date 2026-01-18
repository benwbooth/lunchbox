//! Test binary for image sources
//!
//! Run with: cargo run --bin test_image_sources

use anyhow::Result;
use std::path::PathBuf;

const LAUNCHBOX_CDN_URL: &str = "https://images.launchbox-app.com";
const LIBRETRO_THUMBNAILS_URL: &str = "https://thumbnails.libretro.com";

#[tokio::main]
async fn main() -> Result<()> {
    // Test game: Super Mario Bros. on NES
    let game_title = "Super Mario Bros.";
    let launchbox_platform = "Nintendo Entertainment System";
    let libretro_platform = "Nintendo - Nintendo Entertainment System";
    let libretro_title = "Super Mario Bros. (World)";
    let image_type = "Box - Front";

    let client = reqwest::Client::builder()
        .user_agent("Lunchbox/1.0")
        .timeout(std::time::Duration::from_secs(10))
        .build()?;

    println!("Testing image sources for: {} on {}\n", game_title, launchbox_platform);

    // Test 1: LaunchBox CDN
    println!("=== Test 1: LaunchBox CDN ===");
    let lb_url = format!(
        "{}/{}/{}/{}-01.jpg",
        LAUNCHBOX_CDN_URL,
        urlencoding::encode(launchbox_platform),
        urlencoding::encode(image_type),
        urlencoding::encode(game_title)
    );
    println!("URL: {}", lb_url);
    match client.head(&lb_url).send().await {
        Ok(resp) => {
            println!("Status: {} {}", resp.status().as_u16(), if resp.status().is_success() { "✓" } else { "✗" });
            if let Some(ct) = resp.headers().get("content-type") {
                println!("Content-Type: {:?}", ct);
            }
            if let Some(cl) = resp.headers().get("content-length") {
                println!("Content-Length: {:?}", cl);
            }
        }
        Err(e) => println!("Error: {}", e),
    }
    println!();

    // Test 2: libretro-thumbnails (with libretro platform name)
    println!("=== Test 2: libretro-thumbnails ===");
    let libretro_type = "Named_Boxarts";
    // libretro uses specific character replacements
    let normalized_title = libretro_title
        .replace(':', " -")
        .replace('/', "_")
        .replace('\\', "_")
        .replace('*', "_")
        .replace('?', "_")
        .replace('"', "'")
        .replace('<', "_")
        .replace('>', "_")
        .replace('|', "_")
        .replace('&', "_");
    let lr_url = format!(
        "{}/{}/{}/{}.png",
        LIBRETRO_THUMBNAILS_URL,
        urlencoding::encode(libretro_platform),
        libretro_type,
        urlencoding::encode(&normalized_title)
    );
    println!("URL: {}", lr_url);
    match client.head(&lr_url).send().await {
        Ok(resp) => {
            println!("Status: {} {}", resp.status().as_u16(), if resp.status().is_success() { "✓" } else { "✗" });
            if let Some(ct) = resp.headers().get("content-type") {
                println!("Content-Type: {:?}", ct);
            }
        }
        Err(e) => println!("Error: {}", e),
    }
    println!();

    // Test 3: Try without region code
    println!("=== Test 3: libretro-thumbnails (no region) ===");
    let simple_title = "Super Mario Bros.";
    let lr_url2 = format!(
        "{}/{}/{}/{}.png",
        LIBRETRO_THUMBNAILS_URL,
        urlencoding::encode(libretro_platform),
        libretro_type,
        urlencoding::encode(simple_title)
    );
    println!("URL: {}", lr_url2);
    match client.head(&lr_url2).send().await {
        Ok(resp) => {
            println!("Status: {} {}", resp.status().as_u16(), if resp.status().is_success() { "✓" } else { "✗" });
        }
        Err(e) => println!("Error: {}", e),
    }
    println!();

    // Test 4: Check what libretro actually has
    println!("=== Test 4: List libretro NES boxarts (first few) ===");
    // We can't list directories, but we can try known games
    let test_titles = [
        "Super Mario Bros. (World)",
        "Super Mario Bros.",
        "Legend of Zelda, The (USA)",
        "Mega Man 2 (USA)",
        "Contra (USA)",
    ];
    for title in test_titles {
        let url = format!(
            "{}/{}/{}/{}.png",
            LIBRETRO_THUMBNAILS_URL,
            urlencoding::encode(libretro_platform),
            libretro_type,
            urlencoding::encode(title)
        );
        match client.head(&url).send().await {
            Ok(resp) => {
                let status = if resp.status().is_success() { "✓" } else { "✗" };
                println!("  {} {} - {}", status, resp.status().as_u16(), title);
            }
            Err(e) => println!("  ✗ Error: {} - {}", e, title),
        }
    }

    Ok(())
}
