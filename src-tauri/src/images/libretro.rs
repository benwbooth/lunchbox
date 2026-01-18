//! libretro-thumbnails image source
//!
//! Free image source from https://thumbnails.libretro.com/
//! Structure: {Platform}/{Type}/{Game Name}.png
//! Types: Named_Boxarts, Named_Snaps, Named_Titles

use anyhow::{Context, Result};
use std::path::PathBuf;

/// libretro-thumbnails CDN base URL
pub const LIBRETRO_THUMBNAILS_URL: &str = "https://thumbnails.libretro.com";

/// Image types available from libretro-thumbnails
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LibRetroImageType {
    /// Box art / cover
    Boxart,
    /// Gameplay screenshot
    Snap,
    /// Title screen
    Title,
}

impl LibRetroImageType {
    pub fn path_segment(&self) -> &'static str {
        match self {
            LibRetroImageType::Boxart => "Named_Boxarts",
            LibRetroImageType::Snap => "Named_Snaps",
            LibRetroImageType::Title => "Named_Titles",
        }
    }

    /// Convert from LaunchBox image type
    pub fn from_launchbox_type(image_type: &str) -> Option<Self> {
        match image_type {
            "Box - Front" | "Box - Back" => Some(LibRetroImageType::Boxart),
            "Screenshot - Gameplay" | "Screenshot" => Some(LibRetroImageType::Snap),
            "Screenshot - Game Title" => Some(LibRetroImageType::Title),
            _ => None,
        }
    }
}

/// Map platform names to libretro thumbnail directory names
pub fn get_libretro_platform_name(platform: &str) -> Option<&'static str> {
    let normalized = platform.to_lowercase();

    match normalized.as_str() {
        // Nintendo
        s if s.contains("nes") && !s.contains("snes") && !s.contains("super") => {
            Some("Nintendo - Nintendo Entertainment System")
        }
        s if s.contains("snes") || s.contains("super nintendo") => {
            Some("Nintendo - Super Nintendo Entertainment System")
        }
        s if s.contains("nintendo 64") || s == "n64" => Some("Nintendo - Nintendo 64"),
        s if s.contains("game boy advance") || s == "gba" => Some("Nintendo - Game Boy Advance"),
        s if s.contains("game boy color") || s == "gbc" => Some("Nintendo - Game Boy Color"),
        s if s.contains("game boy") && !s.contains("advance") && !s.contains("color") => {
            Some("Nintendo - Game Boy")
        }
        s if s.contains("nintendo ds") || s == "nds" => Some("Nintendo - Nintendo DS"),
        s if s.contains("nintendo 3ds") || s == "3ds" => Some("Nintendo - Nintendo 3DS"),
        s if s.contains("gamecube") => Some("Nintendo - GameCube"),
        s if s.contains("wii u") => Some("Nintendo - Wii U"),
        s if s.contains("wii") && !s.contains("wii u") => Some("Nintendo - Wii"),
        s if s.contains("switch") => Some("Nintendo - Switch"),
        s if s.contains("virtual boy") => Some("Nintendo - Virtual Boy"),

        // Sega
        s if s.contains("genesis") || s.contains("mega drive") => {
            Some("Sega - Mega Drive - Genesis")
        }
        s if s.contains("master system") => Some("Sega - Master System - Mark III"),
        s if s.contains("game gear") => Some("Sega - Game Gear"),
        s if s.contains("saturn") => Some("Sega - Saturn"),
        s if s.contains("dreamcast") => Some("Sega - Dreamcast"),
        s if s.contains("sega cd") || s.contains("mega-cd") => Some("Sega - Mega-CD - Sega CD"),
        s if s.contains("32x") => Some("Sega - 32X"),

        // Sony
        s if s.contains("playstation 2") || s == "ps2" => Some("Sony - PlayStation 2"),
        s if s.contains("playstation 3") || s == "ps3" => Some("Sony - PlayStation 3"),
        s if s.contains("playstation portable") || s == "psp" => {
            Some("Sony - PlayStation Portable")
        }
        s if s.contains("ps vita") || s.contains("vita") => Some("Sony - PlayStation Vita"),
        s if s.contains("playstation") && !s.contains("2") && !s.contains("3") => {
            Some("Sony - PlayStation")
        }

        // NEC
        s if s.contains("turbografx") && s.contains("cd") => {
            Some("NEC - PC Engine CD - TurboGrafx-CD")
        }
        s if s.contains("turbografx") || s.contains("pc engine") => {
            Some("NEC - PC Engine - TurboGrafx 16")
        }
        s if s.contains("supergrafx") => Some("NEC - PC Engine SuperGrafx"),

        // SNK
        s if s.contains("neo geo pocket color") => Some("SNK - Neo Geo Pocket Color"),
        s if s.contains("neo geo pocket") => Some("SNK - Neo Geo Pocket"),
        s if s.contains("neo geo cd") => Some("SNK - Neo Geo CD"),
        s if s.contains("neo geo") => Some("SNK - Neo Geo"),

        // Atari
        s if s.contains("atari 2600") => Some("Atari - 2600"),
        s if s.contains("atari 5200") => Some("Atari - 5200"),
        s if s.contains("atari 7800") => Some("Atari - 7800"),
        s if s.contains("lynx") => Some("Atari - Lynx"),
        s if s.contains("jaguar") => Some("Atari - Jaguar"),

        // Other
        s if s.contains("colecovision") => Some("Coleco - ColecoVision"),
        s if s.contains("intellivision") => Some("Mattel - Intellivision"),
        s if s.contains("arcade") || s.contains("mame") => Some("MAME"),
        s if s.contains("dos") || s.contains("ms-dos") => Some("DOS"),

        _ => None,
    }
}

/// Normalize a game name for libretro thumbnail lookup
/// libretro uses No-Intro naming conventions
pub fn normalize_game_name(name: &str) -> String {
    // Remove common file extensions
    let name = name
        .trim_end_matches(".zip")
        .trim_end_matches(".7z")
        .trim_end_matches(".rar")
        .trim_end_matches(".rom")
        .trim_end_matches(".bin")
        .trim_end_matches(".iso")
        .trim_end_matches(".cue")
        .trim_end_matches(".chd");

    // Replace characters that are invalid in filenames
    // libretro uses underscores for certain characters
    let name = name
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

    // Clean up whitespace
    name.split_whitespace().collect::<Vec<_>>().join(" ")
}

/// Build a libretro thumbnail URL
pub fn build_thumbnail_url(platform: &str, image_type: LibRetroImageType, game_name: &str) -> Option<String> {
    let platform_dir = get_libretro_platform_name(platform)?;
    let type_dir = image_type.path_segment();
    let normalized_name = normalize_game_name(game_name);

    // URL encode the game name (but keep the structure)
    let encoded_name = urlencoding::encode(&normalized_name);

    Some(format!(
        "{}/{}/{}/{}.png",
        LIBRETRO_THUMBNAILS_URL, platform_dir, type_dir, encoded_name
    ))
}

/// libretro-thumbnails client
pub struct LibRetroThumbnailsClient {
    client: reqwest::Client,
    cache_dir: PathBuf,
}

impl LibRetroThumbnailsClient {
    /// Create a new libretro-thumbnails client
    pub fn new(cache_dir: PathBuf) -> Self {
        let client = reqwest::Client::builder()
            .user_agent("Lunchbox/1.0")
            .timeout(std::time::Duration::from_secs(30))
            .build()
            .expect("Failed to create HTTP client");

        Self { client, cache_dir }
    }

    /// Get the URL for a thumbnail (without downloading)
    pub fn get_thumbnail_url(&self, platform: &str, image_type: LibRetroImageType, game_name: &str) -> Option<String> {
        build_thumbnail_url(platform, image_type, game_name)
    }

    /// Get cache path for an image
    fn get_cache_path(&self, platform: &str, image_type: LibRetroImageType, game_name: &str) -> PathBuf {
        let platform_dir = get_libretro_platform_name(platform).unwrap_or("Unknown");
        let type_dir = image_type.path_segment();
        let normalized_name = normalize_game_name(game_name);

        self.cache_dir
            .join("libretro")
            .join(platform_dir)
            .join(type_dir)
            .join(format!("{}.png", normalized_name))
    }

    /// Check if a thumbnail exists (HEAD request)
    pub async fn check_exists(&self, platform: &str, image_type: LibRetroImageType, game_name: &str) -> bool {
        let url = match build_thumbnail_url(platform, image_type, game_name) {
            Some(u) => u,
            None => return false,
        };

        match self.client.head(&url).send().await {
            Ok(resp) => resp.status().is_success(),
            Err(_) => false,
        }
    }

    /// Download a thumbnail
    pub async fn download(
        &self,
        platform: &str,
        image_type: LibRetroImageType,
        game_name: &str,
    ) -> Result<String> {
        let url = build_thumbnail_url(platform, image_type, game_name)
            .ok_or_else(|| anyhow::anyhow!("Unknown platform: {}", platform))?;

        tracing::info!("libretro: Trying URL: {}", url);
        let cache_path = self.get_cache_path(platform, image_type, game_name);
        tracing::info!("libretro: Cache path: {}", cache_path.display());

        // Check cache first
        if cache_path.exists() {
            tracing::info!("libretro: Cache hit!");
            return Ok(cache_path.to_string_lossy().to_string());
        }

        // Download
        let response = self
            .client
            .get(&url)
            .send()
            .await
            .context("Failed to fetch from libretro-thumbnails")?;

        if !response.status().is_success() {
            tracing::info!("libretro: HTTP {} for {}", response.status(), url);
            anyhow::bail!(
                "libretro-thumbnails: HTTP {} for {}",
                response.status(),
                game_name
            );
        }

        let bytes = response.bytes().await?;
        tracing::info!("libretro: Downloaded {} bytes", bytes.len());

        // Create parent directories
        if let Some(parent) = cache_path.parent() {
            tokio::fs::create_dir_all(parent).await?;
        }

        // Write to cache
        tokio::fs::write(&cache_path, &bytes).await?;
        tracing::info!("libretro: Saved to {}", cache_path.display());

        Ok(cache_path.to_string_lossy().to_string())
    }

    /// Try to find a thumbnail with fuzzy matching
    /// Tries exact match first, then with common variations
    pub async fn find_thumbnail(
        &self,
        platform: &str,
        image_type: LibRetroImageType,
        game_name: &str,
    ) -> Option<String> {
        // Try exact match first
        if let Ok(path) = self.download(platform, image_type, game_name).await {
            return Some(path);
        }

        // Try without region codes like (USA), (Europe), etc.
        let clean_name = remove_region_codes(game_name);
        if clean_name != game_name {
            if let Ok(path) = self.download(platform, image_type, &clean_name).await {
                return Some(path);
            }
        }

        // Try with "The" moved to end (e.g., "Legend of Zelda, The")
        if let Some(modified) = move_article_to_end(game_name) {
            if let Ok(path) = self.download(platform, image_type, &modified).await {
                return Some(path);
            }
        }

        None
    }
}

/// Remove region codes like (USA), (Europe), (Japan), etc.
fn remove_region_codes(name: &str) -> String {
    let mut result = name.to_string();

    // Common region patterns
    let patterns = [
        "(USA)",
        "(Europe)",
        "(Japan)",
        "(World)",
        "(U)",
        "(E)",
        "(J)",
        "(W)",
        "(En)",
        "(Fr)",
        "(De)",
        "(Es)",
        "(It)",
        "(En,Fr,De)",
        "(En,Fr,De,Es,It)",
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
        assert_eq!(
            get_libretro_platform_name("Nintendo Entertainment System"),
            Some("Nintendo - Nintendo Entertainment System")
        );
        assert_eq!(
            get_libretro_platform_name("SNES"),
            Some("Nintendo - Super Nintendo Entertainment System")
        );
        assert_eq!(
            get_libretro_platform_name("Sega Genesis"),
            Some("Sega - Mega Drive - Genesis")
        );
    }

    #[test]
    fn test_normalize_game_name() {
        assert_eq!(normalize_game_name("Super Mario Bros."), "Super Mario Bros.");
        assert_eq!(
            normalize_game_name("Legend of Zelda: Ocarina of Time"),
            "Legend of Zelda - Ocarina of Time"
        );
    }

    #[test]
    fn test_build_url() {
        let url = build_thumbnail_url(
            "Nintendo Entertainment System",
            LibRetroImageType::Boxart,
            "Super Mario Bros.",
        );
        assert!(url.is_some());
        assert!(url.unwrap().contains("Named_Boxarts"));
    }

    #[test]
    fn test_move_article() {
        assert_eq!(
            move_article_to_end("The Legend of Zelda"),
            Some("Legend of Zelda, The".to_string())
        );
        assert_eq!(move_article_to_end("Super Mario Bros"), None);
    }
}
