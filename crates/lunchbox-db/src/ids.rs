use std::fs::File;
use std::io::{Read, Result};
use std::path::Path;

use sha2::{Digest, Sha256};
use uuid::Uuid;

const LUNCHBOX_NAMESPACE: Uuid = Uuid::from_u128(0x8af5_0745_69ec_5c2f_9de6_109e_b56f_223a);

pub fn stable_id(kind: &str, key: &str) -> String {
    let name = format!("{kind}\0{key}");
    Uuid::new_v5(&LUNCHBOX_NAMESPACE, name.as_bytes()).to_string()
}

pub fn normalize_key(value: &str) -> String {
    let mut result = String::with_capacity(value.len());
    let mut separator_pending = false;

    for character in value.trim().chars() {
        if character == '+' {
            if !result.is_empty() && !result.ends_with('-') {
                result.push('-');
            }
            result.push_str("plus");
            separator_pending = true;
        } else if character.is_alphanumeric() {
            if separator_pending && !result.is_empty() && !result.ends_with('-') {
                result.push('-');
            }
            for lower in character.to_lowercase() {
                result.push(lower);
            }
            separator_pending = false;
        } else {
            separator_pending = true;
        }
    }

    result.trim_matches('-').to_owned()
}

pub fn sha256_bytes(bytes: &[u8]) -> String {
    hex::encode(Sha256::digest(bytes))
}

pub fn sha256_file(path: &Path) -> Result<String> {
    let mut file = File::open(path)?;
    let mut hasher = Sha256::new();
    let mut buffer = [0_u8; 128 * 1024];
    loop {
        let read = file.read(&mut buffer)?;
        if read == 0 {
            break;
        }
        hasher.update(&buffer[..read]);
    }
    Ok(hex::encode(hasher.finalize()))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn normalization_is_stable_and_preserves_plus() {
        assert_eq!(normalize_key("  Nintendo  3DS  "), "nintendo-3ds");
        assert_eq!(normalize_key("MSX2+"), "msx2-plus");
        assert_ne!(normalize_key("MSX2+"), normalize_key("MSX2"));
    }

    #[test]
    fn stable_ids_are_repeatable_and_namespaced() {
        assert_eq!(stable_id("game", "x"), stable_id("game", "x"));
        assert_ne!(stable_id("game", "x"), stable_id("release", "x"));
    }
}
