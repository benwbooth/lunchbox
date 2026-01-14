//! IGDB (Internet Game Database) API client
//!
//! API documentation: https://api-docs.igdb.com/
//! Requires Twitch developer credentials for OAuth authentication.

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tokio::sync::RwLock;

/// IGDB API configuration
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct IGDBConfig {
    /// Twitch Client ID
    pub client_id: String,
    /// Twitch Client Secret
    pub client_secret: String,
}

/// Game data from IGDB
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IGDBGame {
    pub id: i64,
    pub name: String,
    pub summary: Option<String>,
    pub storyline: Option<String>,
    pub rating: Option<f64>,
    pub aggregated_rating: Option<f64>,
    pub first_release_date: Option<i64>,
    pub cover: Option<IGDBImage>,
    pub screenshots: Option<Vec<IGDBImage>>,
    pub artworks: Option<Vec<IGDBImage>>,
    pub genres: Option<Vec<IGDBGenre>>,
    pub platforms: Option<Vec<IGDBPlatform>>,
    pub involved_companies: Option<Vec<IGDBInvolvedCompany>>,
    pub websites: Option<Vec<IGDBWebsite>>,
}

/// Image reference from IGDB
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IGDBImage {
    pub id: i64,
    pub image_id: String,
    pub width: Option<i32>,
    pub height: Option<i32>,
}

impl IGDBImage {
    /// Get the full URL for this image at the specified size
    /// Sizes: cover_small, cover_big, screenshot_med, screenshot_big, screenshot_huge,
    ///        logo_med, thumb, micro, 720p, 1080p
    pub fn url(&self, size: &str) -> String {
        format!("https://images.igdb.com/igdb/image/upload/t_{}/{}.jpg", size, self.image_id)
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IGDBGenre {
    pub id: i64,
    pub name: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IGDBPlatform {
    pub id: i64,
    pub name: Option<String>,
    pub abbreviation: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IGDBInvolvedCompany {
    pub id: i64,
    pub company: Option<IGDBCompany>,
    pub developer: Option<bool>,
    pub publisher: Option<bool>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IGDBCompany {
    pub id: i64,
    pub name: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IGDBWebsite {
    pub id: i64,
    pub url: String,
    pub category: Option<i32>,
}

/// OAuth token response
#[derive(Debug, Deserialize)]
struct TokenResponse {
    access_token: String,
    expires_in: i64,
    token_type: String,
}

/// Cached token with expiry
struct CachedToken {
    token: String,
    expires_at: std::time::Instant,
}

/// IGDB API client
pub struct IGDBClient {
    config: IGDBConfig,
    client: reqwest::Client,
    token_cache: Arc<RwLock<Option<CachedToken>>>,
}

impl IGDBClient {
    const API_URL: &'static str = "https://api.igdb.com/v4";
    const TOKEN_URL: &'static str = "https://id.twitch.tv/oauth2/token";

    /// Create a new IGDB client
    pub fn new(config: IGDBConfig) -> Self {
        let client = reqwest::Client::builder()
            .user_agent("Lunchbox/0.1.0")
            .timeout(std::time::Duration::from_secs(30))
            .build()
            .expect("Failed to create HTTP client");

        Self {
            config,
            client,
            token_cache: Arc::new(RwLock::new(None)),
        }
    }

    /// Check if the client has valid credentials
    pub fn has_credentials(&self) -> bool {
        !self.config.client_id.is_empty() && !self.config.client_secret.is_empty()
    }

    /// Get or refresh OAuth token
    async fn get_token(&self) -> Result<String> {
        // Check cache first
        {
            let cache = self.token_cache.read().await;
            if let Some(ref cached) = *cache {
                if cached.expires_at > std::time::Instant::now() {
                    return Ok(cached.token.clone());
                }
            }
        }

        // Get new token
        let response = self
            .client
            .post(Self::TOKEN_URL)
            .form(&[
                ("client_id", &self.config.client_id),
                ("client_secret", &self.config.client_secret),
                ("grant_type", &"client_credentials".to_string()),
            ])
            .send()
            .await
            .context("Failed to get OAuth token from Twitch")?;

        if !response.status().is_success() {
            let status = response.status();
            let body = response.text().await.unwrap_or_default();
            anyhow::bail!("Twitch OAuth error: {} - {}", status, body);
        }

        let token_data: TokenResponse = response.json().await
            .context("Failed to parse OAuth response")?;

        // Cache token (with 60 second buffer before expiry)
        let expires_at = std::time::Instant::now() + std::time::Duration::from_secs((token_data.expires_in - 60) as u64);
        let token = token_data.access_token.clone();

        {
            let mut cache = self.token_cache.write().await;
            *cache = Some(CachedToken {
                token: token.clone(),
                expires_at,
            });
        }

        Ok(token)
    }

    /// Make an API request to IGDB
    async fn api_request(&self, endpoint: &str, body: &str) -> Result<String> {
        if !self.has_credentials() {
            anyhow::bail!("IGDB credentials not configured");
        }

        let token = self.get_token().await?;
        let url = format!("{}/{}", Self::API_URL, endpoint);

        let response = self
            .client
            .post(&url)
            .header("Client-ID", &self.config.client_id)
            .header("Authorization", format!("Bearer {}", token))
            .header("Content-Type", "text/plain")
            .body(body.to_string())
            .send()
            .await
            .context("Failed to send request to IGDB")?;

        if !response.status().is_success() {
            let status = response.status();
            let body = response.text().await.unwrap_or_default();
            anyhow::bail!("IGDB API error: {} - {}", status, body);
        }

        response.text().await.context("Failed to read IGDB response")
    }

    /// Search for games by name
    pub async fn search_games(&self, query: &str, limit: i32) -> Result<Vec<IGDBGame>> {
        let body = format!(
            r#"search "{}";
fields id, name, summary, storyline, rating, aggregated_rating, first_release_date,
       cover.image_id, cover.width, cover.height,
       screenshots.image_id, screenshots.width, screenshots.height,
       artworks.image_id, artworks.width, artworks.height,
       genres.name, platforms.name, platforms.abbreviation,
       involved_companies.company.name, involved_companies.developer, involved_companies.publisher,
       websites.url, websites.category;
limit {};"#,
            query.replace('"', "\\\""),
            limit
        );

        let response = self.api_request("games", &body).await?;
        let games: Vec<IGDBGame> = serde_json::from_str(&response)
            .context("Failed to parse IGDB games response")?;

        Ok(games)
    }

    /// Get a game by IGDB ID
    pub async fn get_game(&self, game_id: i64) -> Result<Option<IGDBGame>> {
        let body = format!(
            r#"where id = {};
fields id, name, summary, storyline, rating, aggregated_rating, first_release_date,
       cover.image_id, cover.width, cover.height,
       screenshots.image_id, screenshots.width, screenshots.height,
       artworks.image_id, artworks.width, artworks.height,
       genres.name, platforms.name, platforms.abbreviation,
       involved_companies.company.name, involved_companies.developer, involved_companies.publisher,
       websites.url, websites.category;"#,
            game_id
        );

        let response = self.api_request("games", &body).await?;
        let games: Vec<IGDBGame> = serde_json::from_str(&response)
            .context("Failed to parse IGDB game response")?;

        Ok(games.into_iter().next())
    }

    /// Get developer and publisher from involved companies
    pub fn extract_companies(game: &IGDBGame) -> (Option<String>, Option<String>) {
        let mut developer = None;
        let mut publisher = None;

        if let Some(ref companies) = game.involved_companies {
            for ic in companies {
                if let Some(ref company) = ic.company {
                    if ic.developer == Some(true) && developer.is_none() {
                        developer = Some(company.name.clone());
                    }
                    if ic.publisher == Some(true) && publisher.is_none() {
                        publisher = Some(company.name.clone());
                    }
                }
            }
        }

        (developer, publisher)
    }

    /// Test connection by searching for a well-known game
    pub async fn test_connection(&self) -> Result<String> {
        let games = self.search_games("Super Mario Bros", 1).await?;
        if games.is_empty() {
            anyhow::bail!("Connection succeeded but no results found (unexpected)");
        }
        Ok(format!("Found: {}", games[0].name))
    }
}

/// Platform ID mapping for IGDB
/// See: https://api-docs.igdb.com/#platform
pub fn get_igdb_platform_id(platform_name: &str) -> Option<i32> {
    let normalized = platform_name.to_lowercase();

    match normalized.as_str() {
        s if s.contains("nes") && !s.contains("snes") => Some(18),   // NES
        s if s.contains("snes") || s.contains("super nintendo") => Some(19), // SNES
        s if s.contains("nintendo 64") || s.contains("n64") => Some(4), // N64
        s if s.contains("game boy advance") || s.contains("gba") => Some(24), // GBA
        s if s.contains("game boy color") || s.contains("gbc") => Some(22), // GBC
        s if s.contains("game boy") && !s.contains("advance") && !s.contains("color") => Some(33), // GB
        s if s.contains("nintendo ds") || s.contains("nds") => Some(20), // NDS
        s if s.contains("nintendo 3ds") || s.contains("3ds") => Some(37), // 3DS
        s if s.contains("gamecube") || s.contains("ngc") => Some(21), // GameCube
        s if s.contains("wii u") => Some(41), // Wii U
        s if s.contains("wii") && !s.contains("wii u") => Some(5), // Wii
        s if s.contains("switch") => Some(130), // Nintendo Switch
        s if s.contains("genesis") || s.contains("mega drive") => Some(29), // Genesis
        s if s.contains("master system") => Some(64), // SMS
        s if s.contains("game gear") => Some(35), // Game Gear
        s if s.contains("saturn") => Some(32), // Saturn
        s if s.contains("dreamcast") => Some(23), // Dreamcast
        s if s.contains("sega cd") || s.contains("mega-cd") => Some(78), // Sega CD
        s if s.contains("32x") => Some(30), // 32X
        s if s.contains("playstation 2") || s.contains("ps2") => Some(8), // PS2
        s if s.contains("playstation 3") || s.contains("ps3") => Some(9), // PS3
        s if s.contains("playstation 4") || s.contains("ps4") => Some(48), // PS4
        s if s.contains("playstation 5") || s.contains("ps5") => Some(167), // PS5
        s if s.contains("psp") => Some(38), // PSP
        s if s.contains("ps vita") || s.contains("vita") => Some(46), // Vita
        s if s.contains("playstation") && !s.contains("2") && !s.contains("3") && !s.contains("4") && !s.contains("5") => Some(7), // PS1
        s if s.contains("xbox one") => Some(49), // Xbox One
        s if s.contains("xbox 360") => Some(12), // Xbox 360
        s if s.contains("xbox") && !s.contains("360") && !s.contains("one") && !s.contains("series") => Some(11), // Original Xbox
        s if s.contains("turbografx") || s.contains("pc engine") => Some(86), // TurboGrafx
        s if s.contains("neo geo") && s.contains("pocket") => Some(119), // NGP
        s if s.contains("neo geo") => Some(80), // Neo Geo
        s if s.contains("atari 2600") => Some(59), // Atari 2600
        s if s.contains("atari 5200") => Some(66), // Atari 5200
        s if s.contains("atari 7800") => Some(60), // Atari 7800
        s if s.contains("lynx") => Some(61), // Atari Lynx
        s if s.contains("jaguar") => Some(62), // Atari Jaguar
        s if s.contains("colecovision") => Some(68), // ColecoVision
        s if s.contains("intellivision") => Some(67), // Intellivision
        s if s.contains("arcade") || s.contains("mame") => Some(52), // Arcade
        s if s.contains("dos") || s.contains("ms-dos") => Some(13), // DOS
        s if s.contains("windows") || s.contains("pc") => Some(6), // Windows/PC
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_config_default() {
        let config = IGDBConfig::default();
        assert!(config.client_id.is_empty());
        assert!(config.client_secret.is_empty());
    }

    #[test]
    fn test_platform_mapping() {
        assert_eq!(get_igdb_platform_id("Nintendo Entertainment System"), Some(18));
        assert_eq!(get_igdb_platform_id("PlayStation 2"), Some(8));
        assert_eq!(get_igdb_platform_id("Unknown Platform"), None);
    }

    #[test]
    fn test_image_url() {
        let image = IGDBImage {
            id: 123,
            image_id: "abc123".to_string(),
            width: Some(1920),
            height: Some(1080),
        };
        assert_eq!(image.url("cover_big"), "https://images.igdb.com/igdb/image/upload/t_cover_big/abc123.jpg");
    }
}
