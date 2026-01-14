//! SteamGridDB API client
//!
//! API documentation: https://www.steamgriddb.com/api/v2
//! Provides custom artwork (grids, heroes, logos, icons) for games.

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};

/// SteamGridDB API configuration
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct SteamGridDBConfig {
    /// API key (get from steamgriddb.com/profile/preferences/api)
    pub api_key: String,
}

/// Artwork types available from SteamGridDB
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ArtworkType {
    Grid,   // Game cover/box art (600x900 or 920x430)
    Hero,   // Banner image (1920x620)
    Logo,   // Game logo with transparency
    Icon,   // Square icon
}

/// Artwork result from SteamGridDB
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SteamGridArtwork {
    pub id: i64,
    pub url: String,
    pub thumb: String,
    pub width: i32,
    pub height: i32,
    pub style: Option<String>,
}

/// Game search result from SteamGridDB
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SteamGridGame {
    pub id: i64,
    pub name: String,
    pub release_date: Option<i64>,
    pub types: Vec<String>,
}

/// All artwork for a game
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct GameArtwork {
    pub grids: Vec<SteamGridArtwork>,
    pub heroes: Vec<SteamGridArtwork>,
    pub logos: Vec<SteamGridArtwork>,
    pub icons: Vec<SteamGridArtwork>,
}

/// SteamGridDB API client
pub struct SteamGridDBClient {
    config: SteamGridDBConfig,
    client: reqwest::Client,
}

impl SteamGridDBClient {
    const BASE_URL: &'static str = "https://www.steamgriddb.com/api/v2";

    /// Create a new SteamGridDB client
    pub fn new(config: SteamGridDBConfig) -> Self {
        let client = reqwest::Client::builder()
            .user_agent("Lunchbox/0.1.0")
            .timeout(std::time::Duration::from_secs(30))
            .build()
            .expect("Failed to create HTTP client");

        Self { config, client }
    }

    /// Check if the client has a valid API key
    pub fn has_credentials(&self) -> bool {
        !self.config.api_key.is_empty()
    }

    /// Search for a game by name
    pub async fn search_game(&self, query: &str) -> Result<Vec<SteamGridGame>> {
        if !self.has_credentials() {
            anyhow::bail!("SteamGridDB API key not configured");
        }

        let url = format!("{}/search/autocomplete/{}", Self::BASE_URL, urlencoding::encode(query));

        let response = self
            .client
            .get(&url)
            .header("Authorization", format!("Bearer {}", self.config.api_key))
            .send()
            .await
            .context("Failed to send request to SteamGridDB")?;

        if !response.status().is_success() {
            let status = response.status();
            let body = response.text().await.unwrap_or_default();
            anyhow::bail!("SteamGridDB API error: {} - {}", status, body);
        }

        let data: SteamGridDBResponse<Vec<SteamGridGame>> = response.json().await
            .context("Failed to parse SteamGridDB response")?;

        if data.success {
            Ok(data.data.unwrap_or_default())
        } else {
            Ok(vec![])
        }
    }

    /// Get artwork for a game by SteamGridDB game ID
    pub async fn get_artwork(&self, game_id: i64, artwork_type: ArtworkType) -> Result<Vec<SteamGridArtwork>> {
        if !self.has_credentials() {
            anyhow::bail!("SteamGridDB API key not configured");
        }

        let endpoint = match artwork_type {
            ArtworkType::Grid => "grids",
            ArtworkType::Hero => "heroes",
            ArtworkType::Logo => "logos",
            ArtworkType::Icon => "icons",
        };

        let url = format!("{}/{}/game/{}", Self::BASE_URL, endpoint, game_id);

        let response = self
            .client
            .get(&url)
            .header("Authorization", format!("Bearer {}", self.config.api_key))
            .send()
            .await
            .context("Failed to send request to SteamGridDB")?;

        if response.status() == reqwest::StatusCode::NOT_FOUND {
            return Ok(vec![]);
        }

        if !response.status().is_success() {
            let status = response.status();
            let body = response.text().await.unwrap_or_default();
            anyhow::bail!("SteamGridDB API error: {} - {}", status, body);
        }

        let data: SteamGridDBResponse<Vec<SteamGridArtwork>> = response.json().await
            .context("Failed to parse SteamGridDB response")?;

        if data.success {
            Ok(data.data.unwrap_or_default())
        } else {
            Ok(vec![])
        }
    }

    /// Get all artwork types for a game
    pub async fn get_all_artwork(&self, game_id: i64) -> Result<GameArtwork> {
        let grids = self.get_artwork(game_id, ArtworkType::Grid).await.unwrap_or_default();
        let heroes = self.get_artwork(game_id, ArtworkType::Hero).await.unwrap_or_default();
        let logos = self.get_artwork(game_id, ArtworkType::Logo).await.unwrap_or_default();
        let icons = self.get_artwork(game_id, ArtworkType::Icon).await.unwrap_or_default();

        Ok(GameArtwork { grids, heroes, logos, icons })
    }

    /// Search for a game and get its best artwork
    pub async fn search_and_get_artwork(&self, game_name: &str) -> Result<Option<(SteamGridGame, GameArtwork)>> {
        let games = self.search_game(game_name).await?;

        if let Some(game) = games.into_iter().next() {
            let artwork = self.get_all_artwork(game.id).await?;
            Ok(Some((game, artwork)))
        } else {
            Ok(None)
        }
    }

    /// Test connection by searching for a well-known game
    pub async fn test_connection(&self) -> Result<()> {
        let games = self.search_game("Super Mario Bros").await?;
        if games.is_empty() {
            anyhow::bail!("Connection succeeded but no results found (unexpected)");
        }
        Ok(())
    }
}

// API response wrapper
#[derive(Debug, Deserialize)]
struct SteamGridDBResponse<T> {
    success: bool,
    data: Option<T>,
    #[serde(default)]
    errors: Vec<String>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_config_default() {
        let config = SteamGridDBConfig::default();
        assert!(config.api_key.is_empty());
    }
}
