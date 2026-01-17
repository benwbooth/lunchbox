//! EmuMovies API client
//!
//! API documentation: https://api.emumovies.com/
//! Provides box art, screenshots, videos, and other media for retro games.
//! Requires lifetime or subscription account for API access.

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

/// EmuMovies API configuration
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct EmuMoviesConfig {
    /// EmuMovies username
    pub username: String,
    /// EmuMovies password or API key
    pub password: String,
}

/// Media types available from EmuMovies
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EmuMoviesMediaType {
    BoxFront,
    BoxBack,
    Box3D,
    Screenshot,
    TitleScreen,
    CartFront,
    CartBack,
    Video,
    Manual,
    Fanart,
    ClearLogo,
    Banner,
}

impl EmuMoviesMediaType {
    pub fn api_name(&self) -> &'static str {
        match self {
            EmuMoviesMediaType::BoxFront => "Box_Front",
            EmuMoviesMediaType::BoxBack => "Box_Back",
            EmuMoviesMediaType::Box3D => "Box_3D",
            EmuMoviesMediaType::Screenshot => "Screenshot",
            EmuMoviesMediaType::TitleScreen => "Title_Screen",
            EmuMoviesMediaType::CartFront => "Cart_Front",
            EmuMoviesMediaType::CartBack => "Cart_Back",
            EmuMoviesMediaType::Video => "Video",
            EmuMoviesMediaType::Manual => "Manual",
            EmuMoviesMediaType::Fanart => "Fanart",
            EmuMoviesMediaType::ClearLogo => "Clear_Logo",
            EmuMoviesMediaType::Banner => "Banner",
        }
    }

    /// Convert from LaunchBox image type
    pub fn from_launchbox_type(image_type: &str) -> Option<Self> {
        match image_type {
            "Box - Front" => Some(EmuMoviesMediaType::BoxFront),
            "Box - Back" => Some(EmuMoviesMediaType::BoxBack),
            "Box - 3D" => Some(EmuMoviesMediaType::Box3D),
            "Screenshot - Gameplay" | "Screenshot" => Some(EmuMoviesMediaType::Screenshot),
            "Screenshot - Game Title" => Some(EmuMoviesMediaType::TitleScreen),
            "Cart - Front" => Some(EmuMoviesMediaType::CartFront),
            "Cart - Back" => Some(EmuMoviesMediaType::CartBack),
            "Fanart - Background" => Some(EmuMoviesMediaType::Fanart),
            "Clear Logo" => Some(EmuMoviesMediaType::ClearLogo),
            "Banner" => Some(EmuMoviesMediaType::Banner),
            _ => None,
        }
    }
}

/// Map platform names to EmuMovies system names
pub fn get_emumovies_system_name(platform: &str) -> Option<&'static str> {
    let normalized = platform.to_lowercase();

    match normalized.as_str() {
        // Nintendo
        s if s.contains("nes") && !s.contains("snes") && !s.contains("super") => Some("Nintendo_NES"),
        s if s.contains("snes") || s.contains("super nintendo") => Some("Nintendo_SNES"),
        s if s.contains("nintendo 64") || s == "n64" => Some("Nintendo_N64"),
        s if s.contains("game boy advance") || s == "gba" => Some("Nintendo_GBA"),
        s if s.contains("game boy color") || s == "gbc" => Some("Nintendo_GBC"),
        s if s.contains("game boy") && !s.contains("advance") && !s.contains("color") => Some("Nintendo_GB"),
        s if s.contains("nintendo ds") || s == "nds" => Some("Nintendo_NDS"),
        s if s.contains("nintendo 3ds") || s == "3ds" => Some("Nintendo_3DS"),
        s if s.contains("gamecube") => Some("Nintendo_GCN"),
        s if s.contains("wii u") => Some("Nintendo_WiiU"),
        s if s.contains("wii") && !s.contains("wii u") => Some("Nintendo_Wii"),
        s if s.contains("switch") => Some("Nintendo_Switch"),
        s if s.contains("virtual boy") => Some("Nintendo_VB"),

        // Sega
        s if s.contains("genesis") || s.contains("mega drive") => Some("Sega_Genesis"),
        s if s.contains("master system") => Some("Sega_SMS"),
        s if s.contains("game gear") => Some("Sega_GG"),
        s if s.contains("saturn") => Some("Sega_Saturn"),
        s if s.contains("dreamcast") => Some("Sega_Dreamcast"),
        s if s.contains("sega cd") || s.contains("mega-cd") => Some("Sega_CD"),
        s if s.contains("32x") => Some("Sega_32X"),

        // Sony
        s if s.contains("playstation 2") || s == "ps2" => Some("Sony_PS2"),
        s if s.contains("playstation 3") || s == "ps3" => Some("Sony_PS3"),
        s if s.contains("playstation portable") || s == "psp" => Some("Sony_PSP"),
        s if s.contains("ps vita") || s.contains("vita") => Some("Sony_PSVita"),
        s if s.contains("playstation") && !s.contains("2") && !s.contains("3") => Some("Sony_PSX"),

        // NEC
        s if s.contains("turbografx") && s.contains("cd") => Some("NEC_TGCD"),
        s if s.contains("turbografx") || s.contains("pc engine") => Some("NEC_TG16"),
        s if s.contains("supergrafx") => Some("NEC_SGX"),

        // SNK
        s if s.contains("neo geo pocket color") => Some("SNK_NGPC"),
        s if s.contains("neo geo pocket") => Some("SNK_NGP"),
        s if s.contains("neo geo cd") => Some("SNK_NEOCD"),
        s if s.contains("neo geo") => Some("SNK_NeoGeo"),

        // Atari
        s if s.contains("atari 2600") => Some("Atari_2600"),
        s if s.contains("atari 5200") => Some("Atari_5200"),
        s if s.contains("atari 7800") => Some("Atari_7800"),
        s if s.contains("lynx") => Some("Atari_Lynx"),
        s if s.contains("jaguar") => Some("Atari_Jaguar"),

        // Other
        s if s.contains("colecovision") => Some("Coleco_Vision"),
        s if s.contains("intellivision") => Some("Mattel_INTV"),
        s if s.contains("arcade") || s.contains("mame") => Some("Arcade"),
        s if s.contains("dos") || s.contains("ms-dos") => Some("Microsoft_DOS"),

        _ => None,
    }
}

/// EmuMovies API client
pub struct EmuMoviesClient {
    config: EmuMoviesConfig,
    client: reqwest::Client,
    cache_dir: PathBuf,
}

impl EmuMoviesClient {
    const API_URL: &'static str = "https://api.emumovies.com/api";

    /// Create a new EmuMovies client
    pub fn new(config: EmuMoviesConfig, cache_dir: PathBuf) -> Self {
        let client = reqwest::Client::builder()
            .user_agent("Lunchbox/1.0")
            .timeout(std::time::Duration::from_secs(30))
            .build()
            .expect("Failed to create HTTP client");

        Self {
            config,
            client,
            cache_dir,
        }
    }

    /// Check if the client has valid credentials
    pub fn has_credentials(&self) -> bool {
        !self.config.username.is_empty() && !self.config.password.is_empty()
    }

    /// Get cache path for an image
    fn get_cache_path(&self, system: &str, media_type: &str, game_name: &str, ext: &str) -> PathBuf {
        let safe_name = game_name
            .chars()
            .map(|c| if c.is_alphanumeric() || c == ' ' || c == '-' { c } else { '_' })
            .collect::<String>();

        self.cache_dir
            .join("emumovies")
            .join(system)
            .join(media_type)
            .join(format!("{}.{}", safe_name, ext))
    }

    /// Search for media by game name
    pub async fn get_media(
        &self,
        platform: &str,
        media_type: EmuMoviesMediaType,
        game_name: &str,
    ) -> Result<Option<String>> {
        if !self.has_credentials() {
            anyhow::bail!("EmuMovies credentials not configured");
        }

        let system = get_emumovies_system_name(platform)
            .ok_or_else(|| anyhow::anyhow!("Unknown platform: {}", platform))?;

        // Build API URL for media search
        // EmuMovies API format: /api/Media/{System}/{MediaType}/{GameName}
        let url = format!(
            "{}/Media/{}/{}/{}",
            Self::API_URL,
            system,
            media_type.api_name(),
            urlencoding::encode(game_name)
        );

        let response = self
            .client
            .get(&url)
            .basic_auth(&self.config.username, Some(&self.config.password))
            .send()
            .await
            .context("Failed to send request to EmuMovies")?;

        if response.status() == reqwest::StatusCode::NOT_FOUND {
            return Ok(None);
        }

        if !response.status().is_success() {
            let status = response.status();
            let body = response.text().await.unwrap_or_default();
            anyhow::bail!("EmuMovies API error: {} - {}", status, body);
        }

        // Parse response - EmuMovies returns a URL to the media file
        let data: EmuMoviesResponse = response
            .json()
            .await
            .context("Failed to parse EmuMovies response")?;

        if data.success && !data.url.is_empty() {
            Ok(Some(data.url))
        } else {
            Ok(None)
        }
    }

    /// Download media and cache it locally
    pub async fn download_media(
        &self,
        platform: &str,
        media_type: EmuMoviesMediaType,
        game_name: &str,
    ) -> Result<String> {
        let system = get_emumovies_system_name(platform)
            .ok_or_else(|| anyhow::anyhow!("Unknown platform: {}", platform))?;

        // Check cache first
        let cache_path = self.get_cache_path(system, media_type.api_name(), game_name, "png");
        if cache_path.exists() {
            return Ok(cache_path.to_string_lossy().to_string());
        }

        // Get media URL
        let url = self
            .get_media(platform, media_type, game_name)
            .await?
            .ok_or_else(|| anyhow::anyhow!("No media found for: {} - {}", game_name, platform))?;

        // Determine extension from URL
        let ext = url
            .rsplit('.')
            .next()
            .filter(|e| ["png", "jpg", "jpeg", "webp", "gif"].contains(e))
            .unwrap_or("png");

        let cache_path = self.get_cache_path(system, media_type.api_name(), game_name, ext);

        // Download the file
        let response = self
            .client
            .get(&url)
            .basic_auth(&self.config.username, Some(&self.config.password))
            .send()
            .await
            .context("Failed to download media from EmuMovies")?;

        if !response.status().is_success() {
            anyhow::bail!("HTTP {}: {}", response.status(), url);
        }

        let bytes = response.bytes().await?;

        // Create parent directories
        if let Some(parent) = cache_path.parent() {
            tokio::fs::create_dir_all(parent).await?;
        }

        // Write to cache
        tokio::fs::write(&cache_path, &bytes).await?;

        Ok(cache_path.to_string_lossy().to_string())
    }

    /// Find media with fuzzy matching (tries variations of the game name)
    pub async fn find_media(
        &self,
        platform: &str,
        media_type: EmuMoviesMediaType,
        game_name: &str,
    ) -> Option<String> {
        // Try exact match first
        if let Ok(path) = self.download_media(platform, media_type, game_name).await {
            return Some(path);
        }

        // Try without region codes
        let clean_name = remove_region_codes(game_name);
        if clean_name != game_name {
            if let Ok(path) = self.download_media(platform, media_type, &clean_name).await {
                return Some(path);
            }
        }

        // Try with "The" moved to end
        if let Some(modified) = move_article_to_end(game_name) {
            if let Ok(path) = self.download_media(platform, media_type, &modified).await {
                return Some(path);
            }
        }

        None
    }

    /// Test connection
    pub async fn test_connection(&self) -> Result<()> {
        if !self.has_credentials() {
            anyhow::bail!("EmuMovies credentials not configured");
        }

        // Try to get a known game's media to test auth
        let response = self
            .client
            .get(format!("{}/Systems", Self::API_URL))
            .basic_auth(&self.config.username, Some(&self.config.password))
            .send()
            .await
            .context("Failed to connect to EmuMovies")?;

        if !response.status().is_success() {
            let status = response.status();
            anyhow::bail!("EmuMovies auth failed: HTTP {}", status);
        }

        Ok(())
    }
}

/// EmuMovies API response
#[derive(Debug, Deserialize)]
struct EmuMoviesResponse {
    #[serde(default)]
    success: bool,
    #[serde(default)]
    url: String,
}

/// Remove region codes like (USA), (Europe), etc.
fn remove_region_codes(name: &str) -> String {
    let mut result = name.to_string();

    let patterns = [
        "(USA)", "(Europe)", "(Japan)", "(World)", "(U)", "(E)", "(J)", "(W)",
        "(En)", "(Fr)", "(De)", "(Es)", "(It)", "(En,Fr,De)", "(En,Fr,De,Es,It)",
    ];

    for pattern in patterns {
        result = result.replace(pattern, "");
    }

    result.trim().to_string()
}

/// Move leading articles to end: "The Legend of Zelda" -> "Legend of Zelda, The"
fn move_article_to_end(name: &str) -> Option<String> {
    let articles = ["The ", "A ", "An "];

    for article in articles {
        if name.starts_with(article) {
            let rest = &name[article.len()..];
            return Some(format!("{}, {}", rest, article.trim()));
        }
    }

    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_platform_mapping() {
        assert_eq!(get_emumovies_system_name("Nintendo Entertainment System"), Some("Nintendo_NES"));
        assert_eq!(get_emumovies_system_name("SNES"), Some("Nintendo_SNES"));
        assert_eq!(get_emumovies_system_name("Sega Genesis"), Some("Sega_Genesis"));
    }

    #[test]
    fn test_media_type_from_launchbox() {
        assert_eq!(
            EmuMoviesMediaType::from_launchbox_type("Box - Front"),
            Some(EmuMoviesMediaType::BoxFront)
        );
        assert_eq!(
            EmuMoviesMediaType::from_launchbox_type("Screenshot - Gameplay"),
            Some(EmuMoviesMediaType::Screenshot)
        );
    }
}
