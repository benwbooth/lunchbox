//! EmuMovies FTP client
//!
//! FTP access to EmuMovies media library.
//! Host: files.emumovies.com (or files2.emumovies.com for Europe)
//! Port: 21
//! Uses forum username/password for authentication.

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use suppaftp::FtpStream;

/// EmuMovies FTP configuration
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct EmuMoviesConfig {
    /// EmuMovies forum username
    pub username: String,
    /// EmuMovies forum password
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
    pub fn folder_name(&self) -> &'static str {
        match self {
            EmuMoviesMediaType::BoxFront => "Box",
            EmuMoviesMediaType::BoxBack => "BoxBack",
            EmuMoviesMediaType::Box3D => "Box3D",
            EmuMoviesMediaType::Screenshot => "Snap",
            EmuMoviesMediaType::TitleScreen => "Title",
            EmuMoviesMediaType::CartFront => "Cart",
            EmuMoviesMediaType::CartBack => "CartBack",
            EmuMoviesMediaType::Video => "Video",
            EmuMoviesMediaType::Manual => "Manual",
            EmuMoviesMediaType::Fanart => "Fanart",
            EmuMoviesMediaType::ClearLogo => "Logos",
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

/// Map platform names to EmuMovies FTP folder names
pub fn get_emumovies_system_folder(platform: &str) -> Option<&'static str> {
    let normalized = platform.to_lowercase();

    match normalized.as_str() {
        // Nintendo
        s if s.contains("nes") && !s.contains("snes") && !s.contains("super") => Some("Nintendo NES"),
        s if s.contains("snes") || s.contains("super nintendo") => Some("Nintendo SNES"),
        s if s.contains("nintendo 64") || s == "n64" => Some("Nintendo 64"),
        s if s.contains("game boy advance") || s == "gba" => Some("Nintendo Game Boy Advance"),
        s if s.contains("game boy color") || s == "gbc" => Some("Nintendo Game Boy Color"),
        s if s.contains("game boy") && !s.contains("advance") && !s.contains("color") => Some("Nintendo Game Boy"),
        s if s.contains("nintendo ds") || s == "nds" => Some("Nintendo DS"),
        s if s.contains("nintendo 3ds") || s == "3ds" => Some("Nintendo 3DS"),
        s if s.contains("gamecube") => Some("Nintendo GameCube"),
        s if s.contains("wii u") => Some("Nintendo Wii U"),
        s if s.contains("wii") && !s.contains("wii u") => Some("Nintendo Wii"),
        s if s.contains("switch") => Some("Nintendo Switch"),
        s if s.contains("virtual boy") => Some("Nintendo Virtual Boy"),

        // Sega
        s if s.contains("genesis") || s.contains("mega drive") => Some("Sega Genesis"),
        s if s.contains("master system") => Some("Sega Master System"),
        s if s.contains("game gear") => Some("Sega Game Gear"),
        s if s.contains("saturn") => Some("Sega Saturn"),
        s if s.contains("dreamcast") => Some("Sega Dreamcast"),
        s if s.contains("sega cd") || s.contains("mega-cd") => Some("Sega CD"),
        s if s.contains("32x") => Some("Sega 32X"),

        // Sony
        s if s.contains("playstation 2") || s == "ps2" => Some("Sony Playstation 2"),
        s if s.contains("playstation 3") || s == "ps3" => Some("Sony Playstation 3"),
        s if s.contains("playstation portable") || s == "psp" => Some("Sony PSP"),
        s if s.contains("ps vita") || s.contains("vita") => Some("Sony Playstation Vita"),
        s if s.contains("playstation") && !s.contains("2") && !s.contains("3") => Some("Sony Playstation"),

        // NEC
        s if s.contains("turbografx") && s.contains("cd") => Some("NEC TurboGrafx-CD"),
        s if s.contains("turbografx") || s.contains("pc engine") => Some("NEC TurboGrafx-16"),
        s if s.contains("supergrafx") => Some("NEC SuperGrafx"),

        // SNK
        s if s.contains("neo geo pocket color") => Some("SNK Neo Geo Pocket Color"),
        s if s.contains("neo geo pocket") => Some("SNK Neo Geo Pocket"),
        s if s.contains("neo geo cd") => Some("SNK Neo Geo CD"),
        s if s.contains("neo geo") => Some("SNK Neo Geo"),

        // Atari
        s if s.contains("atari 2600") => Some("Atari 2600"),
        s if s.contains("atari 5200") => Some("Atari 5200"),
        s if s.contains("atari 7800") => Some("Atari 7800"),
        s if s.contains("lynx") => Some("Atari Lynx"),
        s if s.contains("jaguar") => Some("Atari Jaguar"),

        // Other
        s if s.contains("colecovision") => Some("Coleco ColecoVision"),
        s if s.contains("intellivision") => Some("Mattel Intellivision"),
        s if s.contains("arcade") || s.contains("mame") => Some("MAME"),
        s if s.contains("dos") || s.contains("ms-dos") => Some("Microsoft DOS"),

        _ => None,
    }
}

/// EmuMovies FTP client
#[derive(Clone)]
pub struct EmuMoviesClient {
    config: EmuMoviesConfig,
    cache_dir: PathBuf,
}

const FTP_HOST: &str = "files.emumovies.com";
const FTP_PORT: u16 = 21;

impl EmuMoviesClient {
    /// Create a new EmuMovies client
    pub fn new(config: EmuMoviesConfig, cache_dir: PathBuf) -> Self {
        Self { config, cache_dir }
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

    /// Connect to FTP server
    fn connect(&self) -> Result<FtpStream> {
        let addr = format!("{}:{}", FTP_HOST, FTP_PORT);
        let mut ftp = FtpStream::connect(&addr)
            .context("Failed to connect to EmuMovies FTP server")?;

        ftp.login(&self.config.username, &self.config.password)
            .context("FTP login failed - check username/password")?;

        Ok(ftp)
    }

    /// List files in a directory
    pub fn list_files(&self, path: &str) -> Result<Vec<String>> {
        let mut ftp = self.connect()?;
        let files = ftp.nlst(Some(path)).context("Failed to list directory")?;
        let _ = ftp.quit();
        Ok(files)
    }

    /// Download a file from FTP
    pub fn download_file(&self, remote_path: &str, local_path: &PathBuf) -> Result<()> {
        let mut ftp = self.connect()?;

        // Set binary mode for file transfer
        ftp.transfer_type(suppaftp::types::FileType::Binary)?;

        // Get the file
        let data = ftp.retr_as_buffer(remote_path)
            .context(format!("Failed to download: {}", remote_path))?;

        let _ = ftp.quit();

        // Create parent directories
        if let Some(parent) = local_path.parent() {
            std::fs::create_dir_all(parent)?;
        }

        // Write to local file
        std::fs::write(local_path, data.into_inner())?;

        Ok(())
    }

    /// Search for media by game name
    pub fn get_media(
        &self,
        platform: &str,
        media_type: EmuMoviesMediaType,
        game_name: &str,
    ) -> Result<Option<String>> {
        if !self.has_credentials() {
            anyhow::bail!("EmuMovies credentials not configured");
        }

        let system_folder = get_emumovies_system_folder(platform)
            .ok_or_else(|| anyhow::anyhow!("Unknown platform: {}", platform))?;

        // Build FTP path
        // Format: /{System}/{MediaType}/{GameName}.png (or .jpg, etc.)
        let media_folder = media_type.folder_name();
        let base_path = format!("/{}/{}", system_folder, media_folder);

        // Try to find the file with various extensions
        let extensions = ["png", "jpg", "jpeg"];

        let mut ftp = self.connect()?;

        for ext in extensions {
            let remote_path = format!("{}/{}.{}", base_path, game_name, ext);

            // Check if file exists by trying to get its size
            if ftp.size(&remote_path).is_ok() {
                let _ = ftp.quit();
                return Ok(Some(remote_path));
            }
        }

        let _ = ftp.quit();
        Ok(None)
    }

    /// Download media and cache it locally
    pub fn download_media(
        &self,
        platform: &str,
        media_type: EmuMoviesMediaType,
        game_name: &str,
    ) -> Result<String> {
        let system_folder = get_emumovies_system_folder(platform)
            .ok_or_else(|| anyhow::anyhow!("Unknown platform: {}", platform))?;

        // Check cache first
        let cache_path = self.get_cache_path(system_folder, media_type.folder_name(), game_name, "png");
        if cache_path.exists() {
            return Ok(cache_path.to_string_lossy().to_string());
        }

        // Find and download the file
        let remote_path = self
            .get_media(platform, media_type, game_name)?
            .ok_or_else(|| anyhow::anyhow!("No media found for: {} - {}", game_name, platform))?;

        // Determine extension from remote path
        let ext = remote_path
            .rsplit('.')
            .next()
            .filter(|e| ["png", "jpg", "jpeg", "webp", "gif"].contains(e))
            .unwrap_or("png");

        let cache_path = self.get_cache_path(system_folder, media_type.folder_name(), game_name, ext);

        // Download the file
        self.download_file(&remote_path, &cache_path)?;

        Ok(cache_path.to_string_lossy().to_string())
    }

    /// Find media with fuzzy matching (tries variations of the game name)
    pub fn find_media(
        &self,
        platform: &str,
        media_type: EmuMoviesMediaType,
        game_name: &str,
    ) -> Option<String> {
        // Try exact match first
        if let Ok(path) = self.download_media(platform, media_type, game_name) {
            return Some(path);
        }

        // Try without region codes
        let clean_name = remove_region_codes(game_name);
        if clean_name != game_name {
            if let Ok(path) = self.download_media(platform, media_type, &clean_name) {
                return Some(path);
            }
        }

        // Try with "The" moved to end
        if let Some(modified) = move_article_to_end(game_name) {
            if let Ok(path) = self.download_media(platform, media_type, &modified) {
                return Some(path);
            }
        }

        None
    }

    /// Test connection with credentials
    pub fn test_connection(&self) -> Result<()> {
        if !self.has_credentials() {
            anyhow::bail!("EmuMovies credentials not configured");
        }

        let mut ftp = self.connect()?;

        // Try to list root directory to verify access
        let _ = ftp.nlst(Some("/"))
            .context("Failed to list directory - access denied")?;

        let _ = ftp.quit();

        Ok(())
    }
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
        assert_eq!(get_emumovies_system_folder("Nintendo Entertainment System"), Some("Nintendo NES"));
        assert_eq!(get_emumovies_system_folder("SNES"), Some("Nintendo SNES"));
        assert_eq!(get_emumovies_system_folder("Sega Genesis"), Some("Sega Genesis"));
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
