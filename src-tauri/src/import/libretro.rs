//! LibRetro database DAT file parser
//!
//! Parses clrmamepro format DAT files used by the libretro-database project.
//! These files contain game metadata and ROM checksums from No-Intro, Redump, TOSEC, etc.
//!
//! The DAT format uses nested parentheses:
//! ```text
//! clrmamepro (
//!     name "System Name"
//!     description "System Description"
//! )
//!
//! game (
//!     name "Game Name (Region)"
//!     description "Game Name (Region)"
//!     rom ( name "file.ext" size 1234 crc DEADBEEF md5 ... sha1 ... )
//! )
//! ```

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::Path;
use crate::tags;

/// Header information from a DAT file
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct DatHeader {
    pub name: String,
    pub description: Option<String>,
    pub version: Option<String>,
    pub author: Option<String>,
    pub homepage: Option<String>,
    pub comment: Option<String>,
}

/// ROM information within a game entry
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DatRom {
    pub name: String,
    pub size: Option<u64>,
    pub crc: Option<String>,
    pub md5: Option<String>,
    pub sha1: Option<String>,
}

/// Game entry from a DAT file
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DatGame {
    pub name: String,
    pub description: Option<String>,
    pub region: Option<String>,
    pub release_year: Option<u32>,
    pub release_month: Option<u32>,
    pub serial: Option<String>,
    pub developer: Option<String>,
    pub publisher: Option<String>,
    pub genre: Option<String>,
    pub franchise: Option<String>,
    pub roms: Vec<DatRom>,
}

impl DatGame {
    /// Extract region from game name if not explicitly set
    /// e.g., "Super Mario Bros. (USA)" -> "USA"
    /// Uses centralized tags module for parsing.
    pub fn infer_region(&self) -> Option<String> {
        if self.region.is_some() {
            return self.region.clone();
        }

        // Use centralized tags module to extract region
        let regions = tags::get_region_tags(&self.name);
        regions.into_iter().next()
    }
}

/// Parsed DAT file contents
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DatFile {
    pub header: DatHeader,
    pub games: Vec<DatGame>,
}

/// Token types for the DAT parser
#[derive(Debug, Clone, PartialEq)]
enum Token {
    Word(String),
    QuotedString(String),
    OpenParen,
    CloseParen,
}

/// Tokenizer for DAT files
struct Tokenizer<'a> {
    input: &'a str,
    pos: usize,
}

impl<'a> Tokenizer<'a> {
    fn new(input: &'a str) -> Self {
        Self { input, pos: 0 }
    }

    fn peek_char(&self) -> Option<char> {
        self.input[self.pos..].chars().next()
    }

    fn skip_whitespace(&mut self) {
        while let Some(c) = self.peek_char() {
            if c.is_whitespace() {
                self.pos += c.len_utf8();
            } else {
                break;
            }
        }
    }

    fn next_token(&mut self) -> Option<Token> {
        self.skip_whitespace();

        let c = self.peek_char()?;

        match c {
            '(' => {
                self.pos += 1;
                Some(Token::OpenParen)
            }
            ')' => {
                self.pos += 1;
                Some(Token::CloseParen)
            }
            '"' => {
                self.pos += 1; // Skip opening quote
                let start = self.pos;
                let mut escaped = false;

                while let Some(c) = self.peek_char() {
                    if escaped {
                        escaped = false;
                        self.pos += c.len_utf8();
                    } else if c == '\\' {
                        escaped = true;
                        self.pos += 1;
                    } else if c == '"' {
                        let value = self.input[start..self.pos].replace("\\\"", "\"");
                        self.pos += 1; // Skip closing quote
                        return Some(Token::QuotedString(value));
                    } else {
                        self.pos += c.len_utf8();
                    }
                }
                // Unterminated string - return what we have
                Some(Token::QuotedString(self.input[start..].to_string()))
            }
            _ => {
                // Word token - read until whitespace or paren
                let start = self.pos;
                while let Some(c) = self.peek_char() {
                    if c.is_whitespace() || c == '(' || c == ')' {
                        break;
                    }
                    self.pos += c.len_utf8();
                }
                Some(Token::Word(self.input[start..self.pos].to_string()))
            }
        }
    }
}

impl Iterator for Tokenizer<'_> {
    type Item = Token;

    fn next(&mut self) -> Option<Self::Item> {
        self.next_token()
    }
}

/// Parse a block of key-value pairs within parentheses
fn parse_block(tokens: &mut std::iter::Peekable<Tokenizer>) -> HashMap<String, Vec<String>> {
    let mut result: HashMap<String, Vec<String>> = HashMap::new();

    while let Some(token) = tokens.peek() {
        match token {
            Token::CloseParen => {
                tokens.next(); // consume the close paren
                break;
            }
            Token::Word(key) => {
                let key = key.clone();
                tokens.next(); // consume the key

                // Check if value is a block or a single value
                if let Some(Token::OpenParen) = tokens.peek() {
                    tokens.next(); // consume open paren
                    // Recursively parse nested block, but flatten into a string for now
                    let nested = parse_block(tokens);
                    // For ROM blocks, we want to preserve the structure
                    // Store as a special format: "nested:key=value;key=value"
                    let nested_str = nested
                        .iter()
                        .map(|(k, v)| format!("{}={}", k, v.join(",")))
                        .collect::<Vec<_>>()
                        .join(";");
                    result
                        .entry(key)
                        .or_default()
                        .push(format!("nested:{}", nested_str));
                } else if let Some(value_token) = tokens.next() {
                    let value = match value_token {
                        Token::Word(w) => w,
                        Token::QuotedString(s) => s,
                        _ => continue,
                    };
                    result.entry(key).or_default().push(value);
                }
            }
            Token::QuotedString(s) => {
                // Standalone quoted string - treat as unnamed value
                result
                    .entry("_value".to_string())
                    .or_default()
                    .push(s.clone());
                tokens.next();
            }
            Token::OpenParen => {
                // Unexpected open paren - skip
                tokens.next();
            }
        }
    }

    result
}

/// Parse a ROM entry from nested block data
fn parse_rom(nested_str: &str) -> Option<DatRom> {
    // Parse "nested:name=file.ext;size=1234;crc=DEADBEEF;..."
    if !nested_str.starts_with("nested:") {
        return None;
    }

    let data = &nested_str[7..];
    let mut rom = DatRom {
        name: String::new(),
        size: None,
        crc: None,
        md5: None,
        sha1: None,
    };

    for pair in data.split(';') {
        if let Some((key, value)) = pair.split_once('=') {
            match key {
                "name" => rom.name = value.to_string(),
                "size" => rom.size = value.parse().ok(),
                "crc" | "crc32" => rom.crc = Some(value.to_uppercase()),
                "md5" => rom.md5 = Some(value.to_uppercase()),
                "sha1" => rom.sha1 = Some(value.to_uppercase()),
                _ => {}
            }
        }
    }

    if rom.name.is_empty() {
        None
    } else {
        Some(rom)
    }
}

/// Parse a DAT file from a string
pub fn parse_dat(content: &str) -> Result<DatFile> {
    let mut tokenizer = Tokenizer::new(content).peekable();
    let mut header = DatHeader::default();
    let mut games = Vec::new();

    while let Some(token) = tokenizer.next() {
        match token {
            Token::Word(block_type) => {
                // Expect an open paren
                if let Some(Token::OpenParen) = tokenizer.next() {
                    let block = parse_block(&mut tokenizer);

                    match block_type.as_str() {
                        "clrmamepro" | "header" => {
                            header.name = block
                                .get("name")
                                .and_then(|v| v.first())
                                .cloned()
                                .unwrap_or_default();
                            header.description =
                                block.get("description").and_then(|v| v.first()).cloned();
                            header.version = block.get("version").and_then(|v| v.first()).cloned();
                            header.author = block.get("author").and_then(|v| v.first()).cloned();
                            header.homepage =
                                block.get("homepage").and_then(|v| v.first()).cloned();
                            header.comment = block.get("comment").and_then(|v| v.first()).cloned();
                        }
                        "game" | "machine" => {
                            let name = block
                                .get("name")
                                .and_then(|v| v.first())
                                .cloned()
                                .unwrap_or_default();

                            let roms: Vec<DatRom> = block
                                .get("rom")
                                .map(|rom_strs| {
                                    rom_strs.iter().filter_map(|s| parse_rom(s)).collect()
                                })
                                .unwrap_or_default();

                            let game = DatGame {
                                name,
                                description: block
                                    .get("description")
                                    .and_then(|v| v.first())
                                    .cloned(),
                                region: block.get("region").and_then(|v| v.first()).cloned(),
                                release_year: block
                                    .get("releaseyear")
                                    .and_then(|v| v.first())
                                    .and_then(|s| s.parse().ok()),
                                release_month: block
                                    .get("releasemonth")
                                    .and_then(|v| v.first())
                                    .and_then(|s| s.parse().ok()),
                                serial: block.get("serial").and_then(|v| v.first()).cloned(),
                                developer: block.get("developer").and_then(|v| v.first()).cloned(),
                                publisher: block.get("publisher").and_then(|v| v.first()).cloned(),
                                genre: block.get("genre").and_then(|v| v.first()).cloned(),
                                franchise: block.get("franchise").and_then(|v| v.first()).cloned(),
                                roms,
                            };

                            games.push(game);
                        }
                        _ => {
                            // Unknown block type, skip
                        }
                    }
                }
            }
            _ => {
                // Skip unexpected tokens
            }
        }
    }

    Ok(DatFile { header, games })
}

/// Parse a DAT file from a file path
pub fn parse_dat_file(path: &Path) -> Result<DatFile> {
    let content = std::fs::read_to_string(path)
        .with_context(|| format!("Failed to read DAT file: {}", path.display()))?;
    parse_dat(&content)
}

/// Merge multiple DAT files by CRC
/// This is used to combine base game data with supplementary metadata (developer, publisher, genre)
pub fn merge_dat_files(base: DatFile, supplements: Vec<DatFile>) -> DatFile {
    // Build a CRC -> game index for fast lookup
    let mut crc_to_index: HashMap<String, usize> = HashMap::new();
    let mut games = base.games;

    for (idx, game) in games.iter().enumerate() {
        for rom in &game.roms {
            if let Some(crc) = &rom.crc {
                crc_to_index.insert(crc.clone(), idx);
            }
        }
    }

    // Merge supplementary data
    for supplement in supplements {
        for supp_game in supplement.games {
            // Find matching game by CRC
            for rom in &supp_game.roms {
                if let Some(crc) = &rom.crc {
                    if let Some(&idx) = crc_to_index.get(crc) {
                        let game = &mut games[idx];
                        // Merge non-None fields from supplement
                        if game.developer.is_none() && supp_game.developer.is_some() {
                            game.developer = supp_game.developer.clone();
                        }
                        if game.publisher.is_none() && supp_game.publisher.is_some() {
                            game.publisher = supp_game.publisher.clone();
                        }
                        if game.genre.is_none() && supp_game.genre.is_some() {
                            game.genre = supp_game.genre.clone();
                        }
                        if game.franchise.is_none() && supp_game.franchise.is_some() {
                            game.franchise = supp_game.franchise.clone();
                        }
                        if game.release_year.is_none() && supp_game.release_year.is_some() {
                            game.release_year = supp_game.release_year;
                        }
                        if game.release_month.is_none() && supp_game.release_month.is_some() {
                            game.release_month = supp_game.release_month;
                        }
                        if game.serial.is_none() && supp_game.serial.is_some() {
                            game.serial = supp_game.serial.clone();
                        }
                    }
                }
            }
        }
    }

    DatFile {
        header: base.header,
        games,
    }
}

/// Platform mapping from DAT file names to our internal platform names
pub fn get_platform_from_dat_name(dat_name: &str) -> String {
    // The DAT files are named like "Nintendo - Game Boy" or "Atari - 2600"
    dat_name.to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_simple_dat() {
        let content = r#"
clrmamepro (
    name "Test System"
    description "Test System Description"
    version "1.0"
)

game (
    name "Test Game (USA)"
    description "Test Game (USA)"
    rom ( name "test.rom" size 1024 crc ABCD1234 md5 1234567890ABCDEF sha1 DEADBEEFCAFE )
)
"#;

        let dat = parse_dat(content).unwrap();

        assert_eq!(dat.header.name, "Test System");
        assert_eq!(dat.header.version, Some("1.0".to_string()));
        assert_eq!(dat.games.len(), 1);
        assert_eq!(dat.games[0].name, "Test Game (USA)");
        assert_eq!(dat.games[0].roms.len(), 1);
        assert_eq!(dat.games[0].roms[0].crc, Some("ABCD1234".to_string()));
    }

    #[test]
    fn test_infer_region() {
        let game = DatGame {
            name: "Super Mario Bros. (USA)".to_string(),
            description: None,
            region: None,
            release_year: None,
            release_month: None,
            serial: None,
            developer: None,
            publisher: None,
            genre: None,
            franchise: None,
            roms: vec![],
        };

        assert_eq!(game.infer_region(), Some("USA".to_string()));
    }

    #[test]
    fn test_merge_dat_files() {
        let base = DatFile {
            header: DatHeader {
                name: "Base".to_string(),
                ..Default::default()
            },
            games: vec![DatGame {
                name: "Game 1".to_string(),
                description: None,
                region: None,
                release_year: None,
                release_month: None,
                serial: None,
                developer: None,
                publisher: None,
                genre: None,
                franchise: None,
                roms: vec![DatRom {
                    name: "game1.rom".to_string(),
                    size: Some(1024),
                    crc: Some("AAAA1111".to_string()),
                    md5: None,
                    sha1: None,
                }],
            }],
        };

        let supplement = DatFile {
            header: DatHeader {
                name: "Developer Data".to_string(),
                ..Default::default()
            },
            games: vec![DatGame {
                name: "Game 1".to_string(),
                description: None,
                region: None,
                release_year: None,
                release_month: None,
                serial: None,
                developer: Some("Nintendo".to_string()),
                publisher: None,
                genre: None,
                franchise: None,
                roms: vec![DatRom {
                    name: "game1.rom".to_string(),
                    size: Some(1024),
                    crc: Some("AAAA1111".to_string()),
                    md5: None,
                    sha1: None,
                }],
            }],
        };

        let merged = merge_dat_files(base, vec![supplement]);
        assert_eq!(merged.games[0].developer, Some("Nintendo".to_string()));
    }
}
