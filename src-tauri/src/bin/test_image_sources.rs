//! Test binary for image sources
//!
//! Run with: cargo run --bin test_image_sources
//!
//! Tests all configured media sources:
//! - LaunchBox CDN (requires game_images.db)
//! - libretro-thumbnails (free, no auth)
//! - SteamGridDB (requires API key)
//! - IGDB (requires client_id + client_secret)
//! - EmuMovies (requires FTP username + password)
//! - ScreenScraper (requires dev credentials)

use anyhow::Result;
use lunchbox_lib::keyring_store;

const LIBRETRO_THUMBNAILS_URL: &str = "https://thumbnails.libretro.com";

#[tokio::main]
async fn main() -> Result<()> {
    // Test game: Super Mario Bros. on NES (launchbox_db_id 140 has box art)
    let game_title = "Super Mario Bros.";
    let platform = "Nintendo Entertainment System";
    let libretro_platform = "Nintendo - Nintendo Entertainment System";
    let libretro_title = "Super Mario Bros. (World)";
    let launchbox_db_id: i64 = 140;  // Known to have Box - Front images

    let client = reqwest::Client::builder()
        .user_agent("Lunchbox/1.0")
        .timeout(std::time::Duration::from_secs(30))
        .build()?;

    println!("╔════════════════════════════════════════════════════════════╗");
    println!("║          Testing Image Sources for Lunchbox                ║");
    println!("╚════════════════════════════════════════════════════════════╝");
    println!();
    println!("Test game: {} on {}", game_title, platform);
    println!();

    // Load credentials from keyring
    let creds = keyring_store::load_image_source_credentials();

    let mut results = Vec::new();

    // Test 1: libretro-thumbnails (always available)
    println!("━━━ Test 1: libretro-thumbnails ━━━");
    let lr_url = format!(
        "{}/{}/Named_Boxarts/{}.png",
        LIBRETRO_THUMBNAILS_URL,
        urlencoding::encode(libretro_platform),
        urlencoding::encode(libretro_title)
    );
    let lr_result = test_url(&client, &lr_url).await;
    results.push(("libretro-thumbnails", lr_result));
    println!();

    // Test 2: SteamGridDB
    println!("━━━ Test 2: SteamGridDB ━━━");
    if creds.steamgriddb_api_key.is_empty() {
        println!("⚠ No API key configured");
        results.push(("SteamGridDB", TestResult::NotConfigured));
    } else {
        println!("API key: {}...", &creds.steamgriddb_api_key[..8.min(creds.steamgriddb_api_key.len())]);
        let sgdb_result = test_steamgriddb(&client, &creds.steamgriddb_api_key, game_title).await;
        results.push(("SteamGridDB", sgdb_result));
    }
    println!();

    // Test 3: IGDB
    println!("━━━ Test 3: IGDB ━━━");
    if creds.igdb_client_id.is_empty() || creds.igdb_client_secret.is_empty() {
        println!("⚠ No client_id/client_secret configured");
        results.push(("IGDB", TestResult::NotConfigured));
    } else {
        println!("Client ID: {}...", &creds.igdb_client_id[..8.min(creds.igdb_client_id.len())]);
        let igdb_result = test_igdb(&client, &creds.igdb_client_id, &creds.igdb_client_secret, game_title).await;
        results.push(("IGDB", igdb_result));
    }
    println!();

    // Test 4: EmuMovies
    println!("━━━ Test 4: EmuMovies ━━━");
    if creds.emumovies_username.is_empty() || creds.emumovies_password.is_empty() {
        println!("⚠ No username/password configured");
        results.push(("EmuMovies", TestResult::NotConfigured));
    } else {
        println!("Username: {}", creds.emumovies_username);
        let emumovies_result = test_emumovies(&creds.emumovies_username, &creds.emumovies_password, platform).await;
        results.push(("EmuMovies", emumovies_result));
    }
    println!();

    // Test 5: ScreenScraper
    println!("━━━ Test 5: ScreenScraper ━━━");
    if creds.screenscraper_dev_id.is_empty() || creds.screenscraper_dev_password.is_empty() {
        println!("⚠ No dev credentials configured");
        results.push(("ScreenScraper", TestResult::NotConfigured));
    } else {
        println!("Dev ID: {}", creds.screenscraper_dev_id);
        let ss_result = test_screenscraper(
            &client,
            &creds.screenscraper_dev_id,
            &creds.screenscraper_dev_password,
            creds.screenscraper_user_id.as_deref(),
            creds.screenscraper_user_password.as_deref(),
            game_title,
        ).await;
        results.push(("ScreenScraper", ss_result));
    }
    println!();

    // Test 6: LaunchBox CDN (requires game_images.db)
    println!("━━━ Test 6: LaunchBox CDN ━━━");
    let lb_result = test_launchbox_cdn(&client, launchbox_db_id).await;
    results.push(("LaunchBox CDN", lb_result));
    println!();

    // Summary
    println!("╔════════════════════════════════════════════════════════════╗");
    println!("║                        Summary                             ║");
    println!("╠════════════════════════════════════════════════════════════╣");
    for (name, result) in &results {
        let status = match result {
            TestResult::Success(msg) => format!("✓ {}", msg),
            TestResult::Failed(msg) => format!("✗ {}", msg),
            TestResult::NotConfigured => "⚠ Not configured".to_string(),
        };
        println!("║ {:20} {:37} ║", name, status);
    }
    println!("╚════════════════════════════════════════════════════════════╝");

    Ok(())
}

#[derive(Debug)]
enum TestResult {
    Success(String),
    Failed(String),
    NotConfigured,
}

async fn test_url(client: &reqwest::Client, url: &str) -> TestResult {
    println!("URL: {}", url);
    match client.head(url).send().await {
        Ok(resp) => {
            if resp.status().is_success() {
                println!("Status: {} ✓", resp.status().as_u16());
                TestResult::Success(format!("{}", resp.status().as_u16()))
            } else {
                println!("Status: {} ✗", resp.status().as_u16());
                TestResult::Failed(format!("{}", resp.status().as_u16()))
            }
        }
        Err(e) => {
            println!("Error: {}", e);
            TestResult::Failed(e.to_string())
        }
    }
}

async fn test_steamgriddb(client: &reqwest::Client, api_key: &str, game_title: &str) -> TestResult {
    // Search for game
    let search_url = format!(
        "https://www.steamgriddb.com/api/v2/search/autocomplete/{}",
        urlencoding::encode(game_title)
    );
    println!("Searching for game...");

    let resp = match client
        .get(&search_url)
        .header("Authorization", format!("Bearer {}", api_key))
        .send()
        .await
    {
        Ok(r) => r,
        Err(e) => return TestResult::Failed(format!("Request failed: {}", e)),
    };

    if !resp.status().is_success() {
        return TestResult::Failed(format!("Search failed: {}", resp.status()));
    }

    let body: serde_json::Value = match resp.json().await {
        Ok(v) => v,
        Err(e) => return TestResult::Failed(format!("JSON parse failed: {}", e)),
    };

    let games = body["data"].as_array();
    if games.map(|g| g.is_empty()).unwrap_or(true) {
        return TestResult::Failed("No games found".to_string());
    }

    let game_id = body["data"][0]["id"].as_i64().unwrap_or(0);
    let game_name = body["data"][0]["name"].as_str().unwrap_or("Unknown");
    println!("Found: {} (ID: {})", game_name, game_id);

    // Get grids for game
    let grids_url = format!(
        "https://www.steamgriddb.com/api/v2/grids/game/{}",
        game_id
    );

    let resp = match client
        .get(&grids_url)
        .header("Authorization", format!("Bearer {}", api_key))
        .send()
        .await
    {
        Ok(r) => r,
        Err(e) => return TestResult::Failed(format!("Grids request failed: {}", e)),
    };

    if !resp.status().is_success() {
        return TestResult::Failed(format!("Grids failed: {}", resp.status()));
    }

    let body: serde_json::Value = match resp.json().await {
        Ok(v) => v,
        Err(e) => return TestResult::Failed(format!("JSON parse failed: {}", e)),
    };

    let grids = body["data"].as_array();
    let count = grids.map(|g| g.len()).unwrap_or(0);
    println!("Found {} grid images", count);

    if count > 0 {
        if let Some(url) = body["data"][0]["url"].as_str() {
            println!("First image: {}", url);
        }
        TestResult::Success(format!("{} images", count))
    } else {
        TestResult::Failed("No images found".to_string())
    }
}

async fn test_igdb(client: &reqwest::Client, client_id: &str, client_secret: &str, game_title: &str) -> TestResult {
    // Get OAuth token
    println!("Getting OAuth token...");
    let token_url = format!(
        "https://id.twitch.tv/oauth2/token?client_id={}&client_secret={}&grant_type=client_credentials",
        client_id, client_secret
    );

    let resp = match client.post(&token_url).send().await {
        Ok(r) => r,
        Err(e) => return TestResult::Failed(format!("Token request failed: {}", e)),
    };

    if !resp.status().is_success() {
        let status = resp.status();
        let body = resp.text().await.unwrap_or_default();
        return TestResult::Failed(format!("Token failed: {} - {}", status, body));
    }

    let body: serde_json::Value = match resp.json().await {
        Ok(v) => v,
        Err(e) => return TestResult::Failed(format!("JSON parse failed: {}", e)),
    };

    let access_token = match body["access_token"].as_str() {
        Some(t) => t,
        None => return TestResult::Failed("No access_token in response".to_string()),
    };
    println!("Got token: {}...", &access_token[..8.min(access_token.len())]);

    // Search for game
    println!("Searching for game...");
    let search_body = format!(
        "search \"{}\"; fields name,cover.url; limit 1;",
        game_title
    );

    let resp = match client
        .post("https://api.igdb.com/v4/games")
        .header("Client-ID", client_id)
        .header("Authorization", format!("Bearer {}", access_token))
        .body(search_body)
        .send()
        .await
    {
        Ok(r) => r,
        Err(e) => return TestResult::Failed(format!("Search failed: {}", e)),
    };

    if !resp.status().is_success() {
        return TestResult::Failed(format!("Search failed: {}", resp.status()));
    }

    let body: serde_json::Value = match resp.json().await {
        Ok(v) => v,
        Err(e) => return TestResult::Failed(format!("JSON parse failed: {}", e)),
    };

    let games = body.as_array();
    if games.map(|g| g.is_empty()).unwrap_or(true) {
        return TestResult::Failed("No games found".to_string());
    }

    let game_name = body[0]["name"].as_str().unwrap_or("Unknown");
    println!("Found: {}", game_name);

    if let Some(cover_url) = body[0]["cover"]["url"].as_str() {
        // Convert to full URL
        let full_url = if cover_url.starts_with("//") {
            format!("https:{}", cover_url)
        } else {
            cover_url.to_string()
        };
        println!("Cover: {}", full_url);
        TestResult::Success("Has cover".to_string())
    } else {
        TestResult::Failed("No cover found".to_string())
    }
}

async fn test_emumovies(username: &str, password: &str, platform: &str) -> TestResult {
    println!("Connecting to FTP...");

    // Use suppaftp for FTP connection
    let mut ftp = match suppaftp::FtpStream::connect("ftp.emumovies.com:21") {
        Ok(f) => f,
        Err(e) => return TestResult::Failed(format!("FTP connect failed: {}", e)),
    };

    if let Err(e) = ftp.login(username, password) {
        return TestResult::Failed(format!("FTP login failed: {}", e));
    }
    println!("Logged in successfully");

    // Try to list the platform directory
    let platform_dir = format!("/{}", platform);
    println!("Checking directory: {}", platform_dir);

    match ftp.cwd(&platform_dir) {
        Ok(_) => {
            println!("Directory exists");
            // Try to list Box subdirectory
            match ftp.cwd("Box") {
                Ok(_) => {
                    match ftp.nlst(None) {
                        Ok(files) => {
                            let count = files.len();
                            println!("Found {} files in Box directory", count);
                            if count > 0 {
                                println!("Sample: {}", files[0]);
                            }
                            let _ = ftp.quit();
                            TestResult::Success(format!("{} box images", count))
                        }
                        Err(e) => {
                            let _ = ftp.quit();
                            TestResult::Failed(format!("List failed: {}", e))
                        }
                    }
                }
                Err(_) => {
                    let _ = ftp.quit();
                    TestResult::Failed("No Box subdirectory".to_string())
                }
            }
        }
        Err(e) => {
            let _ = ftp.quit();
            TestResult::Failed(format!("Platform dir not found: {}", e))
        }
    }
}

async fn test_screenscraper(
    client: &reqwest::Client,
    dev_id: &str,
    dev_password: &str,
    user_id: Option<&str>,
    user_password: Option<&str>,
    game_title: &str,
) -> TestResult {
    // Build URL with credentials
    let mut url = format!(
        "https://api.screenscraper.fr/api2/jeuInfos.php?devid={}&devpassword={}&softname=lunchbox&output=json&romnom={}",
        urlencoding::encode(dev_id),
        urlencoding::encode(dev_password),
        urlencoding::encode(&format!("{}.nes", game_title))
    );

    if let (Some(uid), Some(upwd)) = (user_id, user_password) {
        url.push_str(&format!("&ssid={}&sspassword={}", urlencoding::encode(uid), urlencoding::encode(upwd)));
    }

    println!("Querying API...");

    let resp = match client.get(&url).send().await {
        Ok(r) => r,
        Err(e) => return TestResult::Failed(format!("Request failed: {}", e)),
    };

    if !resp.status().is_success() {
        return TestResult::Failed(format!("API error: {}", resp.status()));
    }

    let body: serde_json::Value = match resp.json().await {
        Ok(v) => v,
        Err(e) => return TestResult::Failed(format!("JSON parse failed: {}", e)),
    };

    // Check for API error
    if let Some(error) = body["response"].get("error") {
        return TestResult::Failed(format!("API error: {}", error));
    }

    // Check for game data
    if let Some(game) = body["response"]["jeu"].as_object() {
        let name = game.get("noms")
            .and_then(|n| n.as_array())
            .and_then(|arr| arr.first())
            .and_then(|n| n["text"].as_str())
            .unwrap_or("Unknown");
        println!("Found: {}", name);

        // Check for media
        let media_count = game.get("medias")
            .and_then(|m| m.as_array())
            .map(|arr| arr.len())
            .unwrap_or(0);
        println!("Media items: {}", media_count);

        if media_count > 0 {
            TestResult::Success(format!("{} media items", media_count))
        } else {
            TestResult::Failed("No media found".to_string())
        }
    } else {
        TestResult::Failed("No game data in response".to_string())
    }
}

async fn test_launchbox_cdn(client: &reqwest::Client, launchbox_db_id: i64) -> TestResult {
    use sqlx::sqlite::{SqliteConnectOptions, SqlitePoolOptions};
    use std::str::FromStr;

    // Find game_images.db
    let possible_paths = [
        "../db/game_images.db",
        "./db/game_images.db",
        &format!("{}/.local/share/lunchbox/game_images.db", std::env::var("HOME").unwrap_or_default()),
    ];

    let mut db_path = None;
    for path in &possible_paths {
        if std::path::Path::new(path).exists() {
            db_path = Some(path.to_string());
            break;
        }
    }

    let db_path = match db_path {
        Some(p) => p,
        None => return TestResult::Failed("game_images.db not found".to_string()),
    };

    println!("Using database: {}", db_path);
    println!("LaunchBox DB ID: {}", launchbox_db_id);

    // Connect to database
    let db_url = format!("sqlite:{}?mode=ro", db_path);
    let pool = match SqlitePoolOptions::new()
        .max_connections(1)
        .connect_with(SqliteConnectOptions::from_str(&db_url).unwrap().read_only(true))
        .await
    {
        Ok(p) => p,
        Err(e) => return TestResult::Failed(format!("DB connect failed: {}", e)),
    };

    // Look up image in game_images
    let image_row: Option<(String,)> = sqlx::query_as(
        "SELECT filename FROM game_images WHERE launchbox_db_id = ? AND image_type = 'Box - Front' LIMIT 1"
    )
    .bind(launchbox_db_id)
    .fetch_optional(&pool)
    .await
    .ok()
    .flatten();

    let filename = match image_row {
        Some((f,)) => f,
        None => return TestResult::Failed("No Box - Front image in database".to_string()),
    };

    // Build CDN URL
    let cdn_url = format!("https://images.launchbox-app.com/{}", filename);
    println!("CDN URL: {}", cdn_url);

    // Test the URL
    match client.head(&cdn_url).send().await {
        Ok(resp) => {
            if resp.status().is_success() {
                println!("Status: {} ✓", resp.status().as_u16());
                TestResult::Success("Image found".to_string())
            } else {
                println!("Status: {} ✗", resp.status().as_u16());
                TestResult::Failed(format!("{}", resp.status()))
            }
        }
        Err(e) => TestResult::Failed(format!("Request failed: {}", e)),
    }
}
