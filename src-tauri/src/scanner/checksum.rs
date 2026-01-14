//! Checksum calculation for ROM files
//!
//! Calculates CRC32, MD5, and SHA1 checksums for ROM identification.

use anyhow::Result;
use md5::{Digest, Md5};
use sha1::Sha1;
use std::fs::File;
use std::io::{BufReader, Read};
use std::path::Path;

/// Checksums for a ROM file
#[derive(Debug, Clone, Default)]
pub struct Checksums {
    pub crc32: String,
    pub md5: String,
    pub sha1: String,
    pub size: u64,
}

impl Checksums {
    /// Calculate all checksums for a file
    pub fn calculate(path: &Path) -> Result<Self> {
        let file = File::open(path)?;
        let metadata = file.metadata()?;
        let size = metadata.len();

        let mut reader = BufReader::new(file);
        let mut buffer = [0u8; 65536]; // 64KB buffer

        let mut crc32_hasher = crc32fast::Hasher::new();
        let mut md5_hasher = Md5::new();
        let mut sha1_hasher = Sha1::new();

        loop {
            let bytes_read = reader.read(&mut buffer)?;
            if bytes_read == 0 {
                break;
            }

            let chunk = &buffer[..bytes_read];
            crc32_hasher.update(chunk);
            md5_hasher.update(chunk);
            sha1_hasher.update(chunk);
        }

        let crc32 = format!("{:08X}", crc32_hasher.finalize());
        let md5 = format!("{:032x}", md5_hasher.finalize());
        let sha1 = format!("{:040x}", sha1_hasher.finalize());

        Ok(Checksums {
            crc32,
            md5,
            sha1,
            size,
        })
    }

    /// Calculate only MD5 (faster, for RetroAchievements matching)
    pub fn calculate_md5(path: &Path) -> Result<String> {
        let file = File::open(path)?;
        let mut reader = BufReader::new(file);
        let mut buffer = [0u8; 65536];
        let mut hasher = Md5::new();

        loop {
            let bytes_read = reader.read(&mut buffer)?;
            if bytes_read == 0 {
                break;
            }
            hasher.update(&buffer[..bytes_read]);
        }

        Ok(format!("{:032x}", hasher.finalize()))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;
    use tempfile::NamedTempFile;

    #[test]
    fn test_checksum_calculation() {
        let mut file = NamedTempFile::new().unwrap();
        file.write_all(b"Hello, World!").unwrap();
        file.flush().unwrap();

        let checksums = Checksums::calculate(file.path()).unwrap();

        assert_eq!(checksums.size, 13);
        assert!(!checksums.crc32.is_empty());
        assert!(!checksums.md5.is_empty());
        assert!(!checksums.sha1.is_empty());
    }
}
