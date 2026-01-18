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
        // Nintendo - exact matches first
        "nintendo entertainment system" | "nes" => Some("Nintendo - Nintendo Entertainment System"),
        "super nintendo entertainment system" | "super nintendo" | "snes" => {
            Some("Nintendo - Super Nintendo Entertainment System")
        }
        "nintendo 64" | "n64" => Some("Nintendo - Nintendo 64"),
        "nintendo game boy advance" | "game boy advance" | "gba" => Some("Nintendo - Game Boy Advance"),
        "nintendo game boy color" | "game boy color" | "gbc" => Some("Nintendo - Game Boy Color"),
        "nintendo game boy" | "game boy" | "gb" => Some("Nintendo - Game Boy"),
        "nintendo ds" | "nds" => Some("Nintendo - Nintendo DS"),
        "nintendo 3ds" | "3ds" => Some("Nintendo - Nintendo 3DS"),
        "nintendo gamecube" | "gamecube" | "gc" => Some("Nintendo - GameCube"),
        "nintendo wii u" | "wii u" | "wiiu" => Some("Nintendo - Wii U"),
        "nintendo wii" | "wii" => Some("Nintendo - Wii"),
        "nintendo switch" | "switch" => Some("Nintendo - Switch"),
        "nintendo virtual boy" | "virtual boy" => Some("Nintendo - Virtual Boy"),
        "nintendo famicom disk system" | "famicom disk system" | "fds" => Some("Nintendo - Famicom Disk System"),

        // Sega
        "sega genesis" | "sega mega drive" | "genesis" | "mega drive" => Some("Sega - Mega Drive - Genesis"),
        "sega master system" | "master system" => Some("Sega - Master System - Mark III"),
        "sega game gear" | "game gear" => Some("Sega - Game Gear"),
        "sega saturn" | "saturn" => Some("Sega - Saturn"),
        "sega dreamcast" | "dreamcast" => Some("Sega - Dreamcast"),
        "sega cd" | "mega-cd" | "mega cd" => Some("Sega - Mega-CD - Sega CD"),
        "sega 32x" | "32x" => Some("Sega - 32X"),

        // Sony
        "sony playstation 2" | "playstation 2" | "ps2" => Some("Sony - PlayStation 2"),
        "sony playstation 3" | "playstation 3" | "ps3" => Some("Sony - PlayStation 3"),
        "sony playstation portable" | "playstation portable" | "psp" => Some("Sony - PlayStation Portable"),
        "sony playstation vita" | "playstation vita" | "ps vita" | "psvita" => Some("Sony - PlayStation Vita"),
        "sony playstation" | "playstation" | "ps1" | "psx" => Some("Sony - PlayStation"),

        // NEC
        "nec turbografx-cd" | "turbografx-cd" | "pc engine cd" => Some("NEC - PC Engine CD - TurboGrafx-CD"),
        "nec turbografx-16" | "turbografx-16" | "pc engine" => Some("NEC - PC Engine - TurboGrafx 16"),
        "nec pc engine supergrafx" | "supergrafx" => Some("NEC - PC Engine SuperGrafx"),

        // SNK
        "snk neo geo pocket color" | "neo geo pocket color" | "ngpc" => Some("SNK - Neo Geo Pocket Color"),
        "snk neo geo pocket" | "neo geo pocket" | "ngp" => Some("SNK - Neo Geo Pocket"),
        "snk neo geo cd" | "neo geo cd" => Some("SNK - Neo Geo CD"),
        "snk neo geo" | "neo geo" | "neogeo" => Some("SNK - Neo Geo"),

        // Atari
        "atari 2600" | "atari2600" => Some("Atari - 2600"),
        "atari 5200" | "atari5200" => Some("Atari - 5200"),
        "atari 7800" | "atari7800" => Some("Atari - 7800"),
        "atari lynx" | "lynx" => Some("Atari - Lynx"),
        "atari jaguar" | "jaguar" => Some("Atari - Jaguar"),
        "atari st" => Some("Atari - ST"),

        // Other
        "colecovision" => Some("Coleco - ColecoVision"),
        "mattel intellivision" | "intellivision" => Some("Mattel - Intellivision"),
        "arcade" | "mame" => Some("MAME"),
        "dos" | "ms-dos" => Some("DOS"),
        "commodore 64" | "c64" => Some("Commodore - 64"),
        "commodore amiga" | "amiga" => Some("Commodore - Amiga"),
        "zx spectrum" | "sinclair zx spectrum" => Some("Sinclair - ZX Spectrum"),
        "msx" => Some("Microsoft - MSX"),
        "msx2" => Some("Microsoft - MSX2"),

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
    // If platform already looks like a libretro platform name (contains " - "), use it directly
    // Otherwise, try to map from display name
    let platform_dir = if platform.contains(" - ") || platform == "MAME" || platform == "DOS" {
        platform
    } else {
        get_libretro_platform_name(platform)?
    };

    let type_dir = image_type.path_segment();
    let normalized_name = normalize_game_name(game_name);

    // URL encode the platform and game name for the URL
    let encoded_platform = urlencoding::encode(platform_dir);
    let encoded_name = urlencoding::encode(&normalized_name);

    Some(format!(
        "{}/{}/{}/{}.png",
        LIBRETRO_THUMBNAILS_URL, encoded_platform, type_dir, encoded_name
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
