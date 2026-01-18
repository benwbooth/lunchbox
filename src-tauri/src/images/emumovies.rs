//! EmuMovies FTP client with archive and video support
//!
//! FTP access to EmuMovies media library.
//! Host: files.emumovies.com (or files2.emumovies.com for Europe)
//! Port: 21
//! Uses forum username/password for authentication.
//!
//! EmuMovies distributes artwork as archive packs (zip files) that must be
//! downloaded whole, then individual images extracted on demand.
//!
//! Videos are distributed as individual mp4 files and can be downloaded directly.
//!
//! FTP Structure:
//!   /Official/Artwork/{Platform}/{Platform} (Type)(Source)(Version).zip
//!   /Official/Video Snaps (HQ)/{Platform} (Video Snaps)(HQ)(...)/game.mp4

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::{Path, PathBuf};
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
    /// Get the folder name pattern used in archive filenames
    pub fn archive_pattern(&self) -> &'static str {
        match self {
            EmuMoviesMediaType::BoxFront => "Boxes-2D",
            EmuMoviesMediaType::BoxBack => "Boxes-Back",
            EmuMoviesMediaType::Box3D => "Boxes-3D",
            EmuMoviesMediaType::Screenshot => "Snaps",
            EmuMoviesMediaType::TitleScreen => "Titles",
            EmuMoviesMediaType::CartFront => "Carts",
            EmuMoviesMediaType::CartBack => "Carts-Back",
            EmuMoviesMediaType::Video => "Video",
            EmuMoviesMediaType::Manual => "Manuals",
            EmuMoviesMediaType::Fanart => "Fanart",
            EmuMoviesMediaType::ClearLogo => "Logos",
            EmuMoviesMediaType::Banner => "Banners",
        }
    }

    /// Get the normalized filename for local cache
    pub fn cache_filename(&self) -> &'static str {
        match self {
            EmuMoviesMediaType::BoxFront => "box-front",
            EmuMoviesMediaType::BoxBack => "box-back",
            EmuMoviesMediaType::Box3D => "box-3d",
            EmuMoviesMediaType::Screenshot => "screenshot",
            EmuMoviesMediaType::TitleScreen => "title-screen",
            EmuMoviesMediaType::CartFront => "cart-front",
            EmuMoviesMediaType::CartBack => "cart-back",
            EmuMoviesMediaType::Video => "video",
            EmuMoviesMediaType::Manual => "manual",
            EmuMoviesMediaType::Fanart => "fanart",
            EmuMoviesMediaType::ClearLogo => "clear-logo",
            EmuMoviesMediaType::Banner => "banner",
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

    /// Check if this is a video type
    pub fn is_video(&self) -> bool {
        matches!(self, EmuMoviesMediaType::Video)
    }
}

/// Map platform names to EmuMovies FTP folder names
pub fn get_emumovies_system_folder(platform: &str) -> Option<&'static str> {
    let normalized = platform.to_lowercase();

    match normalized.as_str() {
        // Nintendo
        s if s.contains("nintendo entertainment system") => Some("Nintendo Entertainment System"),
        s if s.contains("nes") && !s.contains("snes") && !s.contains("super") => Some("Nintendo Entertainment System"),
        s if s.contains("super nintendo") || (s.contains("snes") && !s.contains("msu")) => Some("Super Nintendo Entertainment System"),
        s if s.contains("nintendo 64") || s == "n64" => Some("Nintendo 64"),
        s if s.contains("game boy advance") || s == "gba" => Some("Nintendo Game Boy Advance"),
        s if s.contains("game boy color") || s == "gbc" || s.contains("gameboy color") => Some("Nintendo Gameboy Color"),
        s if s.contains("game boy") && !s.contains("advance") && !s.contains("color") => Some("Nintendo Game Boy"),
        s if s.contains("nintendo ds") || s == "nds" => Some("Nintendo DS"),
        s if s.contains("nintendo 3ds") || s == "3ds" => Some("Nintendo 3DS"),
        s if s.contains("gamecube") => Some("Nintendo GameCube"),
        s if s.contains("wii u") => Some("Nintendo Wii U"),
        s if s.contains("wiiware") => Some("Nintendo WiiWare"),
        s if s.contains("wii") && !s.contains("wii u") && !s.contains("wiiware") => Some("Nintendo Wii"),
        s if s.contains("switch") => Some("Nintendo Switch"),
        s if s.contains("virtual boy") => Some("Nintendo Virtual Boy"),
        s if s.contains("famicom disk") => Some("Nintendo Famicom Disk System"),
        s if s.contains("famicom") => Some("Nintendo Famicom"),

        // Sega
        s if s.contains("genesis") || s.contains("mega drive") => Some("Sega Genesis - Mega Drive"),
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
        s if s.contains("colecovision") => Some("ColecoVision"),
        s if s.contains("intellivision") => Some("Mattel Intellivision"),
        s if s.contains("arcade") || s.contains("mame") => Some("MAME"),
        s if s.contains("dos") || s.contains("ms-dos") => Some("Microsoft DOS"),
        s if s.contains("3do") => Some("3DO Interactive Multiplayer"),

        _ => None,
    }
}

/// Archive index entry - maps filenames in archive to their paths
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ArchiveIndex {
    /// Archive filename (e.g., "nes-boxes-2d.zip")
    pub archive_name: String,
    /// Map of normalized game names to archive entry paths
    pub entries: HashMap<String, String>,
    /// When the index was created
    pub created_at: String,
}

impl ArchiveIndex {
    /// Create a new archive index
    pub fn new(archive_name: String) -> Self {
        Self {
            archive_name,
            entries: HashMap::new(),
            created_at: chrono::Utc::now().to_rfc3339(),
        }
    }

    /// Find a matching entry for a game name
    pub fn find_entry(&self, game_name: &str) -> Option<&String> {
        let normalized = normalize_game_name(game_name);

        // Try exact normalized match first
        if let Some(entry) = self.entries.get(&normalized) {
            return Some(entry);
        }

        // Try without region codes
        let no_region = remove_region_codes(&normalized);
        if no_region != normalized {
            if let Some(entry) = self.entries.get(&no_region) {
                return Some(entry);
            }
        }

        // Try fuzzy matching
        for (key, entry) in &self.entries {
            let key_no_region = remove_region_codes(key);
            if key_no_region == no_region {
                return Some(entry);
            }
        }

        None
    }
}

/// Progress callback type for downloads
pub type ProgressCallback = Box<dyn Fn(f32) + Send + Sync>;

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

    /// Get the archives directory
    fn archives_dir(&self) -> PathBuf {
        self.cache_dir.join("emumovies-archives")
    }

    /// Get the archive path for a platform and media type
    fn get_archive_path(&self, platform: &str, media_type: EmuMoviesMediaType) -> PathBuf {
        let safe_platform = platform
            .to_lowercase()
            .chars()
            .map(|c| if c.is_alphanumeric() { c } else { '-' })
            .collect::<String>();

        let filename = format!("{}-{}.zip", safe_platform, media_type.archive_pattern().to_lowercase());
        self.archives_dir().join(filename)
    }

    /// Get the index path for an archive
    fn get_index_path(&self, archive_path: &Path) -> PathBuf {
        archive_path.with_extension("json")
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

    /// Find the archive file for a platform and media type on the FTP server
    pub fn find_archive(&self, platform: &str, media_type: EmuMoviesMediaType) -> Result<Option<String>> {
        let system_folder = get_emumovies_system_folder(platform)
            .ok_or_else(|| anyhow::anyhow!("Unknown platform: {}", platform))?;

        let artwork_path = format!("/Official/Artwork/{}", system_folder);
        let pattern = media_type.archive_pattern();

        tracing::info!("Searching for {} archives in {}", pattern, artwork_path);

        let files = self.list_files(&artwork_path)?;

        // Find an archive containing the pattern
        for file in &files {
            let filename = file.rsplit('/').next().unwrap_or(file);
            if filename.contains(pattern) && filename.ends_with(".zip") {
                tracing::info!("Found archive: {}", file);
                return Ok(Some(file.clone()));
            }
        }

        tracing::info!("No archive found matching pattern {}", pattern);
        Ok(None)
    }

    /// Download an archive from FTP with progress callback
    pub fn download_archive(
        &self,
        remote_path: &str,
        local_path: &Path,
        progress: Option<&ProgressCallback>,
    ) -> Result<()> {
        if local_path.exists() {
            tracing::info!("Archive already exists: {}", local_path.display());
            return Ok(());
        }

        // Create parent directories
        if let Some(parent) = local_path.parent() {
            std::fs::create_dir_all(parent)?;
        }

        tracing::info!("Downloading archive: {} -> {}", remote_path, local_path.display());

        let mut ftp = self.connect()?;
        ftp.transfer_type(suppaftp::types::FileType::Binary)?;

        // Get file size for progress reporting
        let _file_size = ftp.size(remote_path).ok();

        // Create a temp file path
        let temp_path = local_path.with_extension("tmp");

        // Download with progress tracking
        let data = ftp.retr_as_buffer(remote_path)
            .context(format!("Failed to download: {}", remote_path))?;

        let _ = ftp.quit();

        let bytes = data.into_inner();

        if let Some(progress_fn) = progress {
            progress_fn(1.0);
        }

        tracing::info!("Downloaded {} bytes", bytes.len());

        // Write to temp file then rename
        std::fs::write(&temp_path, &bytes)?;
        std::fs::rename(&temp_path, local_path)?;

        Ok(())
    }

    /// Build or load an archive index
    pub fn get_or_build_index(&self, archive_path: &Path) -> Result<ArchiveIndex> {
        let index_path = self.get_index_path(archive_path);

        // Try to load existing index
        if index_path.exists() {
            let content = std::fs::read_to_string(&index_path)?;
            if let Ok(index) = serde_json::from_str::<ArchiveIndex>(&content) {
                return Ok(index);
            }
        }

        // Build new index from archive
        tracing::info!("Building index for archive: {}", archive_path.display());

        let file = std::fs::File::open(archive_path)?;
        let mut archive = zip::ZipArchive::new(file)?;

        let archive_name = archive_path
            .file_name()
            .map(|n| n.to_string_lossy().to_string())
            .unwrap_or_default();

        let mut index = ArchiveIndex::new(archive_name);

        for i in 0..archive.len() {
            let entry = archive.by_index(i)?;
            let entry_name = entry.name().to_string();

            // Skip directories
            if entry_name.ends_with('/') {
                continue;
            }

            // Extract just the filename for matching
            let filename = entry_name.rsplit('/').next().unwrap_or(&entry_name);

            // Remove extension and normalize
            let base_name = filename
                .rsplit_once('.')
                .map(|(name, _)| name)
                .unwrap_or(filename);

            let normalized = normalize_game_name(base_name);
            index.entries.insert(normalized, entry_name);
        }

        // Save index
        let json = serde_json::to_string_pretty(&index)?;
        std::fs::write(&index_path, json)?;

        tracing::info!("Built index with {} entries", index.entries.len());

        Ok(index)
    }

    /// Extract a specific file from an archive
    pub fn extract_from_archive(
        &self,
        archive_path: &Path,
        entry_path: &str,
        output_path: &Path,
    ) -> Result<()> {
        if output_path.exists() {
            return Ok(());
        }

        let file = std::fs::File::open(archive_path)?;
        let mut archive = zip::ZipArchive::new(file)?;

        let mut entry = archive.by_name(entry_path)
            .context(format!("Entry not found in archive: {}", entry_path))?;

        // Create parent directories
        if let Some(parent) = output_path.parent() {
            std::fs::create_dir_all(parent)?;
        }

        let mut output = std::fs::File::create(output_path)?;
        std::io::copy(&mut entry, &mut output)?;

        tracing::info!("Extracted {} to {}", entry_path, output_path.display());

        Ok(())
    }

    /// Get media from archives - downloads archive if needed, extracts requested file
    pub fn get_media_from_archive(
        &self,
        platform: &str,
        media_type: EmuMoviesMediaType,
        game_name: &str,
        game_cache_dir: &Path,
        progress: Option<&ProgressCallback>,
    ) -> Result<PathBuf> {
        // Don't use archives for video
        if media_type.is_video() {
            anyhow::bail!("Use get_video() for video content");
        }

        let archive_path = self.get_archive_path(platform, media_type);

        // Check if we need to download the archive
        if !archive_path.exists() {
            // Find the archive on FTP
            let remote_path = self.find_archive(platform, media_type)?
                .ok_or_else(|| anyhow::anyhow!(
                    "No {} archive found for platform {}",
                    media_type.archive_pattern(),
                    platform
                ))?;

            // Download it
            self.download_archive(&remote_path, &archive_path, progress)?;
        }

        // Get or build the index
        let index = self.get_or_build_index(&archive_path)?;

        // Find the entry for this game
        let entry_path = index.find_entry(game_name)
            .ok_or_else(|| anyhow::anyhow!(
                "No entry found for game '{}' in {} archive",
                game_name,
                media_type.archive_pattern()
            ))?;

        // Determine output path
        let ext = entry_path.rsplit('.').next().unwrap_or("png");
        let output_path = game_cache_dir
            .join("emumovies")
            .join(format!("{}.{}", media_type.cache_filename(), ext));

        // Extract the file
        self.extract_from_archive(&archive_path, entry_path, &output_path)?;

        Ok(output_path)
    }

    /// Find the video folder for a platform
    pub fn find_video_folder(&self, platform: &str) -> Result<Option<String>> {
        let system_folder = get_emumovies_system_folder(platform)
            .ok_or_else(|| anyhow::anyhow!("Unknown platform: {}", platform))?;

        let video_base = "/Official/Video Snaps (HQ)";

        tracing::info!("Searching for video folder for {} in {}", system_folder, video_base);

        let folders = self.list_files(video_base)?;

        // Find a folder containing the platform name
        for folder in &folders {
            let folder_name = folder.rsplit('/').next().unwrap_or(folder);
            if folder_name.contains(system_folder) ||
               folder_name.to_lowercase().contains(&system_folder.to_lowercase()) {
                tracing::info!("Found video folder: {}", folder);
                return Ok(Some(folder.clone()));
            }
        }

        tracing::info!("No video folder found for {}", system_folder);
        Ok(None)
    }

    /// Download a video for a game
    pub fn get_video(
        &self,
        platform: &str,
        game_name: &str,
        game_cache_dir: &Path,
        progress: Option<&ProgressCallback>,
    ) -> Result<PathBuf> {
        let output_path = game_cache_dir
            .join("emumovies")
            .join("video.mp4");

        // Check cache first
        if output_path.exists() {
            return Ok(output_path);
        }

        // Find the video folder
        let video_folder = self.find_video_folder(platform)?
            .ok_or_else(|| anyhow::anyhow!("No video folder found for platform {}", platform))?;

        // List videos in the folder
        let videos = self.list_files(&video_folder)?;

        // Find a matching video
        let game_normalized = normalize_game_name(game_name);
        let game_no_region = remove_region_codes(&game_normalized);

        let mut best_match: Option<&String> = None;

        for video in &videos {
            let filename = video.rsplit('/').next().unwrap_or(video);
            if !filename.ends_with(".mp4") {
                continue;
            }

            let video_name = filename.strip_suffix(".mp4").unwrap_or(filename);
            let video_normalized = normalize_game_name(video_name);
            let video_no_region = remove_region_codes(&video_normalized);

            // Exact match (with region)
            if video_normalized == game_normalized {
                best_match = Some(video);
                break;
            }

            // Match without region
            if video_no_region == game_no_region {
                best_match = Some(video);
            }
        }

        let video_path = best_match
            .ok_or_else(|| anyhow::anyhow!("No video found for game '{}'", game_name))?;

        tracing::info!("Downloading video: {}", video_path);

        // Create parent directories
        if let Some(parent) = output_path.parent() {
            std::fs::create_dir_all(parent)?;
        }

        // Download the video
        let mut ftp = self.connect()?;
        ftp.transfer_type(suppaftp::types::FileType::Binary)?;

        let file_size = ftp.size(video_path).ok();
        tracing::info!("Video size: {:?} bytes", file_size);

        let data = ftp.retr_as_buffer(video_path)
            .context(format!("Failed to download: {}", video_path))?;

        let _ = ftp.quit();

        let bytes = data.into_inner();

        if let Some(progress_fn) = progress {
            progress_fn(1.0);
        }

        // Write to file
        let temp_path = output_path.with_extension("tmp");
        std::fs::write(&temp_path, &bytes)?;
        std::fs::rename(&temp_path, &output_path)?;

        tracing::info!("Downloaded video to {}", output_path.display());

        Ok(output_path)
    }

    /// Download media to the unified cache structure
    pub fn download_to_path(
        &self,
        platform: &str,
        media_type: EmuMoviesMediaType,
        game_name: &str,
        cache_dir: &Path,
        game_id: &str,
        _image_type: &str,
    ) -> Result<String> {
        let game_cache_dir = cache_dir.join(game_id);

        // Check cache first
        let expected_ext = if media_type.is_video() { "mp4" } else { "png" };
        let cache_path = game_cache_dir
            .join("emumovies")
            .join(format!("{}.{}", media_type.cache_filename(), expected_ext));

        if cache_path.exists() {
            return Ok(cache_path.to_string_lossy().to_string());
        }

        // Route to appropriate download method
        let result_path = if media_type.is_video() {
            self.get_video(platform, game_name, &game_cache_dir, None)?
        } else {
            self.get_media_from_archive(platform, media_type, game_name, &game_cache_dir, None)?
        };

        Ok(result_path.to_string_lossy().to_string())
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

/// Normalize a game name for matching
fn normalize_game_name(name: &str) -> String {
    name.to_lowercase()
        .chars()
        .filter(|c| c.is_alphanumeric() || c.is_whitespace() || *c == '-')
        .collect::<String>()
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
}

/// Remove region codes like (USA), (Europe), etc.
fn remove_region_codes(name: &str) -> String {
    let mut result = name.to_string();

    let patterns = [
        "(usa)", "(europe)", "(japan)", "(world)", "(u)", "(e)", "(j)", "(w)",
        "(en)", "(fr)", "(de)", "(es)", "(it)", "(en,fr,de)", "(en,fr,de,es,it)",
        "(usa, europe)", "(japan, usa)", "(rev a)", "(rev b)", "(v1.0)", "(v1.1)",
    ];

    for pattern in patterns {
        result = result.replace(pattern, "");
    }

    result.trim().to_string()
}

/// Move leading articles to end: "The Legend of Zelda" -> "Legend of Zelda, The"
#[allow(dead_code)]
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
        assert_eq!(get_emumovies_system_folder("Nintendo Entertainment System"), Some("Nintendo Entertainment System"));
        assert_eq!(get_emumovies_system_folder("NES"), Some("Nintendo Entertainment System"));
        assert_eq!(get_emumovies_system_folder("Sega Genesis"), Some("Sega Genesis - Mega Drive"));
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

    #[test]
    fn test_normalize_game_name() {
        assert_eq!(normalize_game_name("Super Mario Bros."), "super mario bros");
        assert_eq!(normalize_game_name("The Legend of Zelda"), "the legend of zelda");
    }

    #[test]
    fn test_remove_region_codes() {
        assert_eq!(remove_region_codes("super mario bros (usa)"), "super mario bros");
        assert_eq!(remove_region_codes("zelda (japan, usa)"), "zelda");
    }
}
