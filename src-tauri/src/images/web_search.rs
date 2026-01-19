//! Web image search fallback using DuckDuckGo
//!
//! When all other sources fail, this module searches the web for game images
//! and returns the first result's thumbnail.

use anyhow::{Context, Result};
use serde::Deserialize;
use std::path::Path;

/// DuckDuckGo image search client
pub struct WebImageSearch {
    client: reqwest::Client,
}

#[derive(Debug, Deserialize)]
struct DdgImageResult {
    /// Thumbnail URL
    thumbnail: String,
    /// Full image URL
    image: String,
    /// Image title
    #[allow(dead_code)]
    title: String,
}

#[derive(Debug, Deserialize)]
struct DdgResponse {
    results: Vec<DdgImageResult>,
}

impl WebImageSearch {
    pub fn new() -> Self {
        let client = reqwest::Client::builder()
            .user_agent("Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/120.0.0.0 Safari/537.36")
            .timeout(std::time::Duration::from_secs(15))
            .build()
            .expect("Failed to create HTTP client");

        Self { client }
    }

    /// Search for an image and return the first result's URL
    pub async fn search_image(&self, query: &str) -> Result<Option<String>> {
        // Step 1: Get the vqd token from DuckDuckGo
        let search_url = format!(
            "https://duckduckgo.com/?q={}&iax=images&ia=images",
            urlencoding::encode(query)
        );

        let html = self.client
            .get(&search_url)
            .send()
            .await
            .context("Failed to fetch DuckDuckGo search page")?
            .text()
            .await
            .context("Failed to read search page response")?;

        // Extract vqd token from the HTML
        let vqd = extract_vqd(&html)
            .ok_or_else(|| anyhow::anyhow!("Failed to extract vqd token from DuckDuckGo"))?;

        // Step 2: Fetch image results using the token
        let images_url = format!(
            "https://duckduckgo.com/i.js?l=us-en&o=json&q={}&vqd={}&f=,,,,,&p=1",
            urlencoding::encode(query),
            vqd
        );

        let response = self.client
            .get(&images_url)
            .header("Referer", "https://duckduckgo.com/")
            .send()
            .await
            .context("Failed to fetch image results")?;

        if !response.status().is_success() {
            anyhow::bail!("DuckDuckGo returned status {}", response.status());
        }

        let text = response.text().await?;

        // Parse the JSON response
        let data: DdgResponse = serde_json::from_str(&text)
            .context("Failed to parse DuckDuckGo response")?;

        // Return the first thumbnail URL
        if let Some(first) = data.results.first() {
            // Prefer thumbnail as it's smaller and faster to download
            Ok(Some(first.thumbnail.clone()))
        } else {
            Ok(None)
        }
    }

    /// Search for a game image and download it to the cache
    pub async fn search_and_download(
        &self,
        game_title: &str,
        platform: &str,
        image_type: &str,
        cache_path: &Path,
    ) -> Result<String> {
        // Build search query
        let search_term = match image_type {
            "Box - Front" | "BoxFront" => "box art",
            "Box - Back" | "BoxBack" => "box back",
            "Screenshot - Gameplay" | "Screenshot" => "screenshot gameplay",
            "Screenshot - Game Title" | "TitleScreen" => "title screen",
            "Clear Logo" | "ClearLogo" => "logo transparent",
            "Banner" => "banner",
            "Fanart - Background" | "Fanart" => "fanart wallpaper",
            _ => "box art",
        };

        let query = format!("{} {} {}", game_title, platform, search_term);
        tracing::info!("Web image search: '{}'", query);

        let image_url = self.search_image(&query)
            .await?
            .ok_or_else(|| anyhow::anyhow!("No image results found for: {}", query))?;

        tracing::info!("Found image: {}", image_url);

        // Download the image
        let response = self.client
            .get(&image_url)
            .send()
            .await
            .context("Failed to download image")?;

        if !response.status().is_success() {
            anyhow::bail!("Failed to download image: HTTP {}", response.status());
        }

        // Determine file extension from content type or URL
        let content_type = response
            .headers()
            .get("content-type")
            .and_then(|v| v.to_str().ok())
            .unwrap_or("image/jpeg");

        let extension = match content_type {
            "image/png" => "png",
            "image/gif" => "gif",
            "image/webp" => "webp",
            _ => "jpg",
        };

        let bytes = response.bytes().await?;

        // Create cache directory if needed
        if let Some(parent) = cache_path.parent() {
            tokio::fs::create_dir_all(parent).await?;
        }

        // Write with correct extension
        let final_path = cache_path.with_extension(extension);
        tokio::fs::write(&final_path, &bytes).await?;

        Ok(final_path.to_string_lossy().to_string())
    }
}

/// Extract the vqd token from DuckDuckGo's HTML response
fn extract_vqd(html: &str) -> Option<String> {
    // Look for vqd in various forms:
    // vqd='...' or vqd="..." or vqd=...&

    // Try pattern: vqd='...'
    if let Some(start) = html.find("vqd='") {
        let start = start + 5;
        if let Some(end) = html[start..].find('\'') {
            return Some(html[start..start + end].to_string());
        }
    }

    // Try pattern: vqd="..."
    if let Some(start) = html.find("vqd=\"") {
        let start = start + 5;
        if let Some(end) = html[start..].find('"') {
            return Some(html[start..start + end].to_string());
        }
    }

    // Try pattern: vqd=...& (in URLs)
    if let Some(start) = html.find("vqd=") {
        let start = start + 4;
        let remaining = &html[start..];
        let end = remaining.find(|c: char| c == '&' || c == '"' || c == '\'' || c.is_whitespace())
            .unwrap_or(remaining.len().min(50));
        let token = &remaining[..end];
        if !token.is_empty() && token.chars().all(|c| c.is_alphanumeric() || c == '-' || c == '_') {
            return Some(token.to_string());
        }
    }

    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_extract_vqd() {
        let html1 = r#"vqd='4-123456789'"#;
        assert_eq!(extract_vqd(html1), Some("4-123456789".to_string()));

        let html2 = r#"vqd="4-987654321""#;
        assert_eq!(extract_vqd(html2), Some("4-987654321".to_string()));

        let html3 = r#"&vqd=4-abcdef123&q=test"#;
        assert_eq!(extract_vqd(html3), Some("4-abcdef123".to_string()));
    }
}
