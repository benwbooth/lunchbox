//! ROM file scanner
//!
//! Discovers ROM files in specified directories and extracts metadata.

use anyhow::Result;
use rayon::prelude::*;
use serde::{Deserialize, Serialize};
use std::collections::HashSet;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;
use walkdir::WalkDir;

use super::checksum::Checksums;

/// Common ROM file extensions by platform
pub const ROM_EXTENSIONS: &[&str] = &[
    // Nintendo
    "nes", "fds", "unf", "unif",           // NES/Famicom
    "sfc", "smc", "fig", "swc", "bs",      // SNES
    "n64", "z64", "v64",                   // N64
    "gb", "gbc", "sgb",                    // Game Boy
    "gba",                                  // GBA
    "nds", "dsi",                          // DS
    "3ds", "cia",                          // 3DS
    "gcm", "gcz", "iso", "ciso", "rvz", "wbfs", "wad", // GameCube/Wii
    // Sega
    "sms", "gg",                           // Master System/Game Gear
    "md", "gen", "bin", "smd",             // Genesis/Mega Drive
    "32x",                                  // 32X
    "cue", "chd",                          // Sega CD / Saturn / Dreamcast
    // Sony
    "pbp", "cso",                          // PSP
    "pkg",                                  // PS3
    // Atari
    "a26", "a52", "a78",                   // Atari 2600/5200/7800
    "lnx",                                  // Lynx
    "jag", "j64",                          // Jaguar
    // Other
    "pce", "sgx",                          // TurboGrafx
    "ngp", "ngc",                          // Neo Geo Pocket
    "ws", "wsc",                           // WonderSwan
    "vec",                                  // Vectrex
    "col",                                  // ColecoVision
    "int",                                  // Intellivision
    // Archives
    "zip", "7z", "rar",
];

/// Information about a discovered ROM file
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RomFile {
    pub path: PathBuf,
    pub file_name: String,
    pub clean_name: String,
    pub extension: String,
    pub size: u64,
    pub region: Option<String>,
    pub version: Option<String>,
    pub checksums: Option<ChecksumsDto>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChecksumsDto {
    pub crc32: String,
    pub md5: String,
    pub sha1: String,
}

impl From<Checksums> for ChecksumsDto {
    fn from(c: Checksums) -> Self {
        ChecksumsDto {
            crc32: c.crc32,
            md5: c.md5,
            sha1: c.sha1,
        }
    }
}

/// Progress information during scanning
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScanProgress {
    pub total_files: usize,
    pub scanned_files: usize,
    pub current_file: String,
}

/// ROM file scanner
pub struct RomScanner {
    extensions: HashSet<String>,
}

impl Default for RomScanner {
    fn default() -> Self {
        Self::new()
    }
}

impl RomScanner {
    pub fn new() -> Self {
        let extensions: HashSet<String> = ROM_EXTENSIONS
            .iter()
            .map(|s| s.to_lowercase())
            .collect();

        Self { extensions }
    }

    /// Scan directories for ROM files
    pub fn scan_directories(&self, paths: &[PathBuf]) -> Vec<RomFile> {
        let files: Vec<PathBuf> = paths
            .iter()
            .flat_map(|path| self.find_rom_files(path))
            .collect();

        files
            .into_par_iter()
            .filter_map(|path| self.create_rom_file(&path).ok())
            .collect()
    }

    /// Scan directories with checksum calculation (slower but more accurate)
    pub fn scan_with_checksums(
        &self,
        paths: &[PathBuf],
        progress_callback: Option<Arc<dyn Fn(ScanProgress) + Send + Sync>>,
    ) -> Vec<RomFile> {
        let files: Vec<PathBuf> = paths
            .iter()
            .flat_map(|path| self.find_rom_files(path))
            .collect();

        let total = files.len();
        let scanned = Arc::new(AtomicUsize::new(0));

        files
            .into_par_iter()
            .filter_map(|path| {
                let result = self.create_rom_file_with_checksums(&path);

                if let Some(ref callback) = progress_callback {
                    let count = scanned.fetch_add(1, Ordering::SeqCst) + 1;
                    callback(ScanProgress {
                        total_files: total,
                        scanned_files: count,
                        current_file: path.file_name()
                            .map(|s| s.to_string_lossy().to_string())
                            .unwrap_or_default(),
                    });
                }

                result.ok()
            })
            .collect()
    }

    /// Find all ROM files in a directory
    fn find_rom_files(&self, path: &Path) -> Vec<PathBuf> {
        if !path.exists() {
            tracing::warn!("Path does not exist: {}", path.display());
            return Vec::new();
        }

        WalkDir::new(path)
            .follow_links(true)
            .into_iter()
            .filter_map(|e| e.ok())
            .filter(|e| e.file_type().is_file())
            .filter(|e| {
                e.path()
                    .extension()
                    .and_then(|ext| ext.to_str())
                    .map(|ext| self.extensions.contains(&ext.to_lowercase()))
                    .unwrap_or(false)
            })
            .map(|e| e.path().to_path_buf())
            .collect()
    }

    /// Create a RomFile from a path (without checksums)
    fn create_rom_file(&self, path: &Path) -> Result<RomFile> {
        let metadata = std::fs::metadata(path)?;
        let file_name = path
            .file_name()
            .and_then(|s| s.to_str())
            .unwrap_or("")
            .to_string();

        let extension = path
            .extension()
            .and_then(|s| s.to_str())
            .unwrap_or("")
            .to_lowercase();

        let (clean_name, region, version) = parse_rom_name(&file_name);

        Ok(RomFile {
            path: path.to_path_buf(),
            file_name,
            clean_name,
            extension,
            size: metadata.len(),
            region,
            version,
            checksums: None,
        })
    }

    /// Create a RomFile with checksums calculated
    fn create_rom_file_with_checksums(&self, path: &Path) -> Result<RomFile> {
        let mut rom = self.create_rom_file(path)?;

        // Skip checksum for very large files (> 4GB) or archives
        if rom.size < 4_000_000_000 && !["zip", "7z", "rar"].contains(&rom.extension.as_str()) {
            rom.checksums = Checksums::calculate(path).ok().map(ChecksumsDto::from);
        }

        Ok(rom)
    }
}

/// Parse ROM filename to extract clean name, region, and version
fn parse_rom_name(filename: &str) -> (String, Option<String>, Option<String>) {
    // Remove extension
    let name = filename
        .rsplit_once('.')
        .map(|(name, _)| name)
        .unwrap_or(filename);

    let mut clean_name = name.to_string();
    let mut region = None;
    let mut version = None;

    // Extract region from parentheses (USA), (Europe), (Japan), etc.
    if let Some(start) = name.find('(') {
        if let Some(end) = name[start..].find(')') {
            let tag = &name[start + 1..start + end];

            // Check if it's a region tag
            let regions = ["USA", "Europe", "Japan", "World", "En", "Fr", "De", "Es", "It",
                          "U", "E", "J", "JU", "UE", "Asia", "Korea", "China", "Brazil",
                          "Australia", "Germany", "France", "Spain", "Italy", "Netherlands"];

            if regions.iter().any(|r| tag.contains(r)) {
                region = Some(tag.to_string());
            }

            // Check for version
            if tag.starts_with('v') || tag.starts_with('V') || tag.contains("Rev") {
                version = Some(tag.to_string());
            }
        }
    }

    // Clean up the name - remove tags in parentheses and brackets
    let mut result = String::new();
    let mut depth: i32 = 0;
    let mut bracket_depth: i32 = 0;

    for c in clean_name.chars() {
        match c {
            '(' => depth += 1,
            ')' => depth = depth.saturating_sub(1),
            '[' => bracket_depth += 1,
            ']' => bracket_depth = bracket_depth.saturating_sub(1),
            _ if depth == 0 && bracket_depth == 0 => result.push(c),
            _ => {}
        }
    }

    clean_name = result.trim().to_string();

    (clean_name, region, version)
}

/// Normalize a name for database matching (similar to LaunchBox's CompareName)
pub fn normalize_for_matching(name: &str) -> String {
    name.chars()
        .filter(|c| c.is_alphanumeric())
        .flat_map(|c| c.to_uppercase())
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_rom_name() {
        let (clean, region, version) = parse_rom_name("Super Mario World (USA).sfc");
        assert_eq!(clean, "Super Mario World");
        assert_eq!(region, Some("USA".to_string()));
        assert!(version.is_none());

        let (clean, region, version) = parse_rom_name("Sonic the Hedgehog 2 (World) (Rev A).md");
        assert_eq!(clean, "Sonic the Hedgehog 2");
        assert_eq!(region, Some("World".to_string()));
        assert_eq!(version, Some("Rev A".to_string()));
    }

    #[test]
    fn test_normalize_for_matching() {
        assert_eq!(normalize_for_matching("Super Mario Bros."), "SUPERMARIOBROS");
        assert_eq!(normalize_for_matching("The Legend of Zelda: A Link to the Past"), "THELEGENDOFZELDAALINKTOTHEPAST");
    }
}
