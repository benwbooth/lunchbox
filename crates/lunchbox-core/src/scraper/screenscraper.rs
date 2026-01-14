//! ScreenScraper API client
//!
//! API documentation: https://www.screenscraper.fr/webapi2.php
//! Uses checksum-based game identification for accurate matching.

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// ScreenScraper API configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScreenScraperConfig {
    /// Developer ID (required for API access)
    pub dev_id: String,
    /// Developer password
    pub dev_password: String,
    /// User's ScreenScraper account ID (optional, for higher rate limits)
    pub user_id: Option<String>,
    /// User's ScreenScraper password
    pub user_password: Option<String>,
}

impl Default for ScreenScraperConfig {
    fn default() -> Self {
        Self {
            dev_id: String::new(),
            dev_password: String::new(),
            user_id: None,
            user_password: None,
        }
    }
}

/// Game data scraped from ScreenScraper
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScrapedGame {
    pub screenscraper_id: i64,
    pub name: String,
    pub description: Option<String>,
    pub developer: Option<String>,
    pub publisher: Option<String>,
    pub release_date: Option<String>,
    pub genres: Vec<String>,
    pub players: Option<String>,
    pub rating: Option<f64>,
    pub media: ScrapedMedia,
}

/// Media URLs from ScreenScraper
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ScrapedMedia {
    pub box_front: Option<String>,
    pub box_back: Option<String>,
    pub screenshot: Option<String>,
    pub video: Option<String>,
    pub wheel: Option<String>,
    pub fanart: Option<String>,
}

/// Platform ID mapping for ScreenScraper
/// See: https://www.screenscraper.fr/webapi2.php?plateforme
pub fn get_screenscraper_platform_id(platform_name: &str) -> Option<i32> {
    let normalized = platform_name.to_lowercase();

    // Common platform mappings
    match normalized.as_str() {
        s if s.contains("nes") && !s.contains("snes") => Some(3),   // NES
        s if s.contains("snes") || s.contains("super nintendo") => Some(4), // SNES
        s if s.contains("nintendo 64") || s.contains("n64") => Some(14), // N64
        s if s.contains("game boy advance") || s.contains("gba") => Some(12), // GBA
        s if s.contains("game boy color") || s.contains("gbc") => Some(11), // GBC
        s if s.contains("game boy") && !s.contains("advance") && !s.contains("color") => Some(10), // GB
        s if s.contains("nintendo ds") || s.contains("nds") => Some(15), // NDS
        s if s.contains("nintendo 3ds") || s.contains("3ds") => Some(17), // 3DS
        s if s.contains("gamecube") || s.contains("ngc") => Some(13), // GameCube
        s if s.contains("wii u") => Some(18), // Wii U
        s if s.contains("wii") && !s.contains("wii u") => Some(16), // Wii
        s if s.contains("switch") => Some(225), // Nintendo Switch
        s if s.contains("genesis") || s.contains("mega drive") => Some(1), // Genesis
        s if s.contains("master system") => Some(2), // SMS
        s if s.contains("game gear") => Some(21), // Game Gear
        s if s.contains("saturn") => Some(22), // Saturn
        s if s.contains("dreamcast") => Some(23), // Dreamcast
        s if s.contains("sega cd") || s.contains("mega-cd") => Some(20), // Sega CD
        s if s.contains("32x") => Some(19), // 32X
        s if s.contains("playstation 2") || s.contains("ps2") => Some(58), // PS2
        s if s.contains("playstation 3") || s.contains("ps3") => Some(59), // PS3
        s if s.contains("psp") => Some(61), // PSP
        s if s.contains("ps vita") || s.contains("vita") => Some(62), // Vita
        s if s.contains("playstation") && !s.contains("2") && !s.contains("3") => Some(57), // PS1
        s if s.contains("turbografx") || s.contains("pc engine") => Some(31), // TurboGrafx
        s if s.contains("neo geo") && s.contains("pocket") => Some(82), // NGP
        s if s.contains("neo geo") => Some(142), // Neo Geo
        s if s.contains("atari 2600") => Some(26), // Atari 2600
        s if s.contains("atari 5200") => Some(40), // Atari 5200
        s if s.contains("atari 7800") => Some(41), // Atari 7800
        s if s.contains("lynx") => Some(28), // Atari Lynx
        s if s.contains("jaguar") => Some(27), // Atari Jaguar
        s if s.contains("colecovision") => Some(48), // ColecoVision
        s if s.contains("intellivision") => Some(115), // Intellivision
        s if s.contains("arcade") || s.contains("mame") => Some(75), // Arcade
        s if s.contains("dos") || s.contains("ms-dos") => Some(135), // DOS
        s if s.contains("windows") => Some(138), // Windows
        _ => None,
    }
}

/// ScreenScraper API client
pub struct ScreenScraperClient {
    config: ScreenScraperConfig,
    client: reqwest::Client,
}

impl ScreenScraperClient {
    const BASE_URL: &'static str = "https://www.screenscraper.fr/api2";
    const SOFTWARE_NAME: &'static str = "lunchbox";

    /// Create a new ScreenScraper client
    pub fn new(config: ScreenScraperConfig) -> Self {
        let client = reqwest::Client::builder()
            .user_agent("Lunchbox/0.1.0")
            .timeout(std::time::Duration::from_secs(30))
            .build()
            .expect("Failed to create HTTP client");

        Self { config, client }
    }

    /// Check if the client has valid credentials
    pub fn has_credentials(&self) -> bool {
        !self.config.dev_id.is_empty() && !self.config.dev_password.is_empty()
    }

    /// Look up a game by ROM checksums
    pub async fn lookup_by_checksum(
        &self,
        crc32: &str,
        md5: &str,
        sha1: &str,
        file_size: u64,
        file_name: &str,
        platform_id: Option<i32>,
    ) -> Result<Option<ScrapedGame>> {
        if !self.has_credentials() {
            anyhow::bail!("ScreenScraper credentials not configured");
        }

        let mut params: HashMap<&str, String> = HashMap::new();
        params.insert("devid", self.config.dev_id.clone());
        params.insert("devpassword", self.config.dev_password.clone());
        params.insert("softname", Self::SOFTWARE_NAME.to_string());
        params.insert("output", "json".to_string());
        params.insert("romtype", "rom".to_string());
        params.insert("crc", crc32.to_string());
        params.insert("md5", md5.to_string());
        params.insert("sha1", sha1.to_string());
        params.insert("romtaille", file_size.to_string());
        params.insert("romnom", file_name.to_string());

        if let Some(id) = platform_id {
            params.insert("systemeid", id.to_string());
        }

        if let Some(ref user_id) = self.config.user_id {
            params.insert("ssid", user_id.clone());
        }
        if let Some(ref user_pass) = self.config.user_password {
            params.insert("sspassword", user_pass.clone());
        }

        let url = format!("{}/jeuInfos.php", Self::BASE_URL);

        tracing::debug!("ScreenScraper lookup: {} (CRC: {})", file_name, crc32);

        let response = self
            .client
            .get(&url)
            .query(&params)
            .send()
            .await
            .context("Failed to send request to ScreenScraper")?;

        if response.status() == reqwest::StatusCode::NOT_FOUND {
            tracing::debug!("Game not found in ScreenScraper: {}", file_name);
            return Ok(None);
        }

        if !response.status().is_success() {
            let status = response.status();
            let body = response.text().await.unwrap_or_default();
            tracing::warn!("ScreenScraper error {}: {}", status, body);
            anyhow::bail!("ScreenScraper API error: {} - {}", status, body);
        }

        let body = response.text().await?;
        let data: ScreenScraperResponse = serde_json::from_str(&body)
            .context("Failed to parse ScreenScraper response")?;

        if let Some(game) = data.response.jeu {
            Ok(Some(parse_screenscraper_game(game)))
        } else {
            Ok(None)
        }
    }
}

// ScreenScraper API response types

#[derive(Debug, Deserialize)]
struct ScreenScraperResponse {
    response: ScreenScraperResponseInner,
}

#[derive(Debug, Deserialize)]
struct ScreenScraperResponseInner {
    jeu: Option<ScreenScraperGame>,
}

#[derive(Debug, Deserialize)]
struct ScreenScraperGame {
    id: String,
    #[serde(default)]
    noms: Vec<ScreenScraperName>,
    #[serde(default)]
    synopsis: Vec<ScreenScraperText>,
    #[serde(default)]
    developpeur: Option<ScreenScraperEntity>,
    #[serde(default)]
    editeur: Option<ScreenScraperEntity>,
    #[serde(default)]
    dates: Vec<ScreenScraperDate>,
    #[serde(default)]
    genres: Vec<ScreenScraperGenre>,
    #[serde(default)]
    joueurs: Option<ScreenScraperText>,
    #[serde(default)]
    note: Option<ScreenScraperRating>,
    #[serde(default)]
    medias: Vec<ScreenScraperMedia>,
}

#[derive(Debug, Deserialize)]
struct ScreenScraperName {
    region: Option<String>,
    text: String,
}

#[derive(Debug, Deserialize)]
struct ScreenScraperText {
    langue: Option<String>,
    text: String,
}

#[derive(Debug, Deserialize)]
struct ScreenScraperEntity {
    text: String,
}

#[derive(Debug, Deserialize)]
struct ScreenScraperDate {
    region: Option<String>,
    text: String,
}

#[derive(Debug, Deserialize)]
struct ScreenScraperGenre {
    #[serde(default)]
    noms: Vec<ScreenScraperName>,
}

#[derive(Debug, Deserialize)]
struct ScreenScraperRating {
    text: String,
}

#[derive(Debug, Deserialize)]
struct ScreenScraperMedia {
    #[serde(rename = "type")]
    media_type: String,
    url: String,
    region: Option<String>,
}

/// Parse ScreenScraper game data into our format
fn parse_screenscraper_game(game: ScreenScraperGame) -> ScrapedGame {
    // Get English or first available name
    let name = game.noms.iter()
        .find(|n| n.region.as_deref() == Some("us") || n.region.as_deref() == Some("wor"))
        .or_else(|| game.noms.first())
        .map(|n| n.text.clone())
        .unwrap_or_else(|| "Unknown".to_string());

    // Get English or first available description
    let description = game.synopsis.iter()
        .find(|s| s.langue.as_deref() == Some("en"))
        .or_else(|| game.synopsis.first())
        .map(|s| s.text.clone());

    let developer = game.developpeur.map(|d| d.text);
    let publisher = game.editeur.map(|e| e.text);

    // Get US or first available release date
    let release_date = game.dates.iter()
        .find(|d| d.region.as_deref() == Some("us") || d.region.as_deref() == Some("wor"))
        .or_else(|| game.dates.first())
        .map(|d| d.text.clone());

    // Extract genre names (English preferred)
    let genres: Vec<String> = game.genres.iter()
        .filter_map(|g| {
            g.noms.iter()
                .find(|n| n.region.as_deref() == Some("en"))
                .or_else(|| g.noms.first())
                .map(|n| n.text.clone())
        })
        .collect();

    let players = game.joueurs.map(|j| j.text);

    // Parse rating (ScreenScraper uses 0-20 scale)
    let rating = game.note
        .and_then(|n| n.text.parse::<f64>().ok())
        .map(|r| r / 20.0 * 10.0); // Convert to 0-10 scale

    // Extract media URLs
    let mut media = ScrapedMedia::default();

    // Build a set of media types that have US region versions
    let us_media_types: std::collections::HashSet<_> = game.medias.iter()
        .filter(|m| m.region.as_deref() == Some("us"))
        .map(|m| m.media_type.clone())
        .collect();

    for m in &game.medias {
        // Skip non-US if US version exists for this type
        if us_media_types.contains(&m.media_type) && m.region.as_deref() != Some("us") {
            continue;
        }

        match m.media_type.as_str() {
            "box-2D" | "box-2D-front" => media.box_front = Some(m.url.clone()),
            "box-2D-back" => media.box_back = Some(m.url.clone()),
            "ss" | "screenshot" => media.screenshot = Some(m.url.clone()),
            "video" | "video-normalized" => media.video = Some(m.url.clone()),
            "wheel" | "wheel-hd" => media.wheel = Some(m.url.clone()),
            "fanart" => media.fanart = Some(m.url.clone()),
            _ => {}
        }
    }

    ScrapedGame {
        screenscraper_id: game.id.parse().unwrap_or(0),
        name,
        description,
        developer,
        publisher,
        release_date,
        genres,
        players,
        rating,
        media,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_platform_id_mapping() {
        assert_eq!(get_screenscraper_platform_id("Nintendo Entertainment System"), Some(3));
        assert_eq!(get_screenscraper_platform_id("Super Nintendo Entertainment System"), Some(4));
        assert_eq!(get_screenscraper_platform_id("Nintendo 64"), Some(14));
        assert_eq!(get_screenscraper_platform_id("Sega Genesis"), Some(1));
        assert_eq!(get_screenscraper_platform_id("Sony PlayStation"), Some(57));
        assert_eq!(get_screenscraper_platform_id("Unknown Platform"), None);
    }
}
