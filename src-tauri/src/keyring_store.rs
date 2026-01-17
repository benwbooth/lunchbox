//! Secure credential storage using the system keyring.
//!
//! On Linux: Uses Secret Service (GNOME Keyring, KWallet with Secret Service enabled)
//! On macOS: Uses Keychain
//! On Windows: Uses Credential Manager
//!
//! Falls back to in-memory storage if keyring is unavailable.

use anyhow::Result;
use keyring::Entry;
use std::sync::OnceLock;

const SERVICE_NAME: &str = "lunchbox";

/// Check if keyring is available on this system
fn keyring_available() -> bool {
    static AVAILABLE: OnceLock<bool> = OnceLock::new();
    *AVAILABLE.get_or_init(|| {
        tracing::info!("Checking keyring availability...");
        match Entry::new(SERVICE_NAME, "test_availability") {
            Ok(entry) => {
                tracing::info!("Keyring entry created, testing write access...");
                // Test writing to the credential store
                match entry.set_password("test") {
                    Ok(()) => {
                        tracing::info!("Keyring write succeeded, cleaning up test entry...");
                        // Clean up the test entry
                        let _ = entry.delete_credential();
                        true
                    }
                    Err(e) => {
                        tracing::warn!("System keyring not available (write failed): {:?}. Credentials will be stored in config file.", e);
                        false
                    }
                }
            }
            Err(e) => {
                tracing::warn!("System keyring not available (entry creation failed): {:?}. Credentials will be stored in config file.", e);
                false
            }
        }
    })
}

/// Credential keys for different services
pub mod keys {
    pub const STEAMGRIDDB_API_KEY: &str = "steamgriddb_api_key";
    pub const IGDB_CLIENT_ID: &str = "igdb_client_id";
    pub const IGDB_CLIENT_SECRET: &str = "igdb_client_secret";
    pub const EMUMOVIES_USERNAME: &str = "emumovies_username";
    pub const EMUMOVIES_PASSWORD: &str = "emumovies_password";
    pub const SCREENSCRAPER_DEV_ID: &str = "screenscraper_dev_id";
    pub const SCREENSCRAPER_DEV_PASSWORD: &str = "screenscraper_dev_password";
    pub const SCREENSCRAPER_USER_ID: &str = "screenscraper_user_id";
    pub const SCREENSCRAPER_USER_PASSWORD: &str = "screenscraper_user_password";
}

/// Store a credential in the system keyring (if available)
/// Returns Ok even if keyring storage fails - the fallback is database storage
pub fn store_credential(key: &str, value: &str) -> Result<()> {
    if !keyring_available() {
        return Ok(()); // Silently skip - credentials stored in config instead
    }

    if value.is_empty() {
        delete_credential(key).ok();
        return Ok(());
    }

    tracing::debug!("Attempting to store credential: {}", key);
    let entry = match Entry::new(SERVICE_NAME, key) {
        Ok(e) => e,
        Err(e) => {
            tracing::warn!("Failed to create keyring entry for {}: {:?}", key, e);
            return Ok(()); // Don't fail, fall back to database
        }
    };
    match entry.set_password(value) {
        Ok(()) => {
            tracing::debug!("Stored credential in keyring: {}", key);
            Ok(())
        }
        Err(e) => {
            tracing::warn!("Failed to store credential {} in keyring: {:?}. Will use database fallback.", key, e);
            Ok(()) // Don't fail, fall back to database
        }
    }
}

/// Retrieve a credential from the system keyring
pub fn get_credential(key: &str) -> Result<Option<String>> {
    if !keyring_available() {
        return Ok(None); // Credentials come from config instead
    }

    let entry = Entry::new(SERVICE_NAME, key)?;
    match entry.get_password() {
        Ok(password) => Ok(Some(password)),
        Err(keyring::Error::NoEntry) => Ok(None),
        Err(e) => Err(anyhow::anyhow!("Failed to retrieve credential: {}", e)),
    }
}

/// Delete a credential from the system keyring
pub fn delete_credential(key: &str) -> Result<()> {
    if !keyring_available() {
        return Ok(());
    }

    let entry = Entry::new(SERVICE_NAME, key)?;
    match entry.delete_credential() {
        Ok(()) => {
            tracing::debug!("Deleted credential: {}", key);
            Ok(())
        }
        Err(keyring::Error::NoEntry) => Ok(()),
        Err(e) => Err(anyhow::anyhow!("Failed to delete credential: {}", e)),
    }
}

/// Check if keyring storage is being used
pub fn is_keyring_available() -> bool {
    keyring_available()
}

/// Store all image source credentials
pub fn store_image_source_credentials(
    steamgriddb_api_key: &str,
    igdb_client_id: &str,
    igdb_client_secret: &str,
    emumovies_username: &str,
    emumovies_password: &str,
    screenscraper_dev_id: &str,
    screenscraper_dev_password: &str,
    screenscraper_user_id: Option<&str>,
    screenscraper_user_password: Option<&str>,
) -> Result<()> {
    if !keyring_available() {
        return Ok(()); // Skip keyring storage
    }

    store_credential(keys::STEAMGRIDDB_API_KEY, steamgriddb_api_key)?;
    store_credential(keys::IGDB_CLIENT_ID, igdb_client_id)?;
    store_credential(keys::IGDB_CLIENT_SECRET, igdb_client_secret)?;
    store_credential(keys::EMUMOVIES_USERNAME, emumovies_username)?;
    store_credential(keys::EMUMOVIES_PASSWORD, emumovies_password)?;
    store_credential(keys::SCREENSCRAPER_DEV_ID, screenscraper_dev_id)?;
    store_credential(keys::SCREENSCRAPER_DEV_PASSWORD, screenscraper_dev_password)?;
    store_credential(keys::SCREENSCRAPER_USER_ID, screenscraper_user_id.unwrap_or(""))?;
    store_credential(keys::SCREENSCRAPER_USER_PASSWORD, screenscraper_user_password.unwrap_or(""))?;

    Ok(())
}

/// Load all image source credentials from keyring
pub fn load_image_source_credentials() -> ImageSourceCredentials {
    if !keyring_available() {
        return ImageSourceCredentials::default(); // Credentials come from config
    }

    ImageSourceCredentials {
        steamgriddb_api_key: get_credential(keys::STEAMGRIDDB_API_KEY).ok().flatten().unwrap_or_default(),
        igdb_client_id: get_credential(keys::IGDB_CLIENT_ID).ok().flatten().unwrap_or_default(),
        igdb_client_secret: get_credential(keys::IGDB_CLIENT_SECRET).ok().flatten().unwrap_or_default(),
        emumovies_username: get_credential(keys::EMUMOVIES_USERNAME).ok().flatten().unwrap_or_default(),
        emumovies_password: get_credential(keys::EMUMOVIES_PASSWORD).ok().flatten().unwrap_or_default(),
        screenscraper_dev_id: get_credential(keys::SCREENSCRAPER_DEV_ID).ok().flatten().unwrap_or_default(),
        screenscraper_dev_password: get_credential(keys::SCREENSCRAPER_DEV_PASSWORD).ok().flatten().unwrap_or_default(),
        screenscraper_user_id: get_credential(keys::SCREENSCRAPER_USER_ID).ok().flatten(),
        screenscraper_user_password: get_credential(keys::SCREENSCRAPER_USER_PASSWORD).ok().flatten(),
    }
}

/// Container for all image source credentials
#[derive(Debug, Clone, Default)]
pub struct ImageSourceCredentials {
    pub steamgriddb_api_key: String,
    pub igdb_client_id: String,
    pub igdb_client_secret: String,
    pub emumovies_username: String,
    pub emumovies_password: String,
    pub screenscraper_dev_id: String,
    pub screenscraper_dev_password: String,
    pub screenscraper_user_id: Option<String>,
    pub screenscraper_user_password: Option<String>,
}
