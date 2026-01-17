//! Secure credential storage using the system keyring.
//!
//! On Linux: Uses Secret Service (GNOME Keyring, KWallet)
//! On macOS: Uses Keychain
//! On Windows: Uses Credential Manager

use anyhow::{Context, Result};
use keyring::Entry;

const SERVICE_NAME: &str = "lunchbox";

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

/// Store a credential in the system keyring
pub fn store_credential(key: &str, value: &str) -> Result<()> {
    if value.is_empty() {
        // Delete the credential if the value is empty
        delete_credential(key).ok();
        return Ok(());
    }

    let entry = Entry::new(SERVICE_NAME, key)
        .context("Failed to create keyring entry")?;

    entry.set_password(value)
        .context("Failed to store credential in keyring")?;

    tracing::debug!("Stored credential: {}", key);
    Ok(())
}

/// Retrieve a credential from the system keyring
pub fn get_credential(key: &str) -> Result<Option<String>> {
    let entry = Entry::new(SERVICE_NAME, key)
        .context("Failed to create keyring entry")?;

    match entry.get_password() {
        Ok(password) => Ok(Some(password)),
        Err(keyring::Error::NoEntry) => Ok(None),
        Err(e) => Err(anyhow::anyhow!("Failed to retrieve credential: {}", e)),
    }
}

/// Delete a credential from the system keyring
pub fn delete_credential(key: &str) -> Result<()> {
    let entry = Entry::new(SERVICE_NAME, key)
        .context("Failed to create keyring entry")?;

    match entry.delete_credential() {
        Ok(()) => {
            tracing::debug!("Deleted credential: {}", key);
            Ok(())
        }
        Err(keyring::Error::NoEntry) => Ok(()), // Already deleted
        Err(e) => Err(anyhow::anyhow!("Failed to delete credential: {}", e)),
    }
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

/// Load all image source credentials
pub fn load_image_source_credentials() -> ImageSourceCredentials {
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
