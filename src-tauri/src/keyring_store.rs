//! Secure credential storage using the system keyring.
//!
//! On Linux: Uses secret-tool CLI (Secret Service) if available, falls back to keyring crate
//! On macOS: Uses Keychain via keyring crate
//! On Windows: Uses Credential Manager via keyring crate
//!
//! Falls back to database storage if keyring is unavailable.

use anyhow::Result;
use std::sync::OnceLock;

const SERVICE_NAME: &str = "lunchbox";

/// Check which keyring backend is available
#[derive(Clone, Copy, PartialEq)]
enum KeyringBackend {
    SecretTool,  // Linux: secret-tool CLI
    KeyringCrate, // macOS/Windows: keyring crate
    None,
}

fn detect_backend() -> KeyringBackend {
    static BACKEND: OnceLock<KeyringBackend> = OnceLock::new();
    *BACKEND.get_or_init(|| {
        tracing::info!("Detecting keyring backend...");

        // On Linux, prefer secret-tool if available (more reliable with KWallet)
        #[cfg(target_os = "linux")]
        {
            if let Ok(output) = std::process::Command::new("secret-tool")
                .arg("--version")
                .output()
            {
                if output.status.success() {
                    // Test that we can actually write
                    let write_result = std::process::Command::new("secret-tool")
                        .args(["store", "--label", "lunchbox-test", "service", SERVICE_NAME, "key", "_test"])
                        .stdin(std::process::Stdio::piped())
                        .spawn()
                        .and_then(|mut child| {
                            use std::io::Write;
                            if let Some(stdin) = child.stdin.as_mut() {
                                stdin.write_all(b"test")?;
                            }
                            child.wait()
                        });

                    if write_result.map(|s| s.success()).unwrap_or(false) {
                        // Clean up test entry
                        let _ = std::process::Command::new("secret-tool")
                            .args(["clear", "service", SERVICE_NAME, "key", "_test"])
                            .status();
                        tracing::info!("Using secret-tool for credential storage");
                        return KeyringBackend::SecretTool;
                    }
                }
            }
        }

        // Try keyring crate (works well on macOS/Windows)
        #[cfg(not(target_os = "linux"))]
        {
            if let Ok(entry) = keyring::Entry::new(SERVICE_NAME, "_test") {
                if entry.set_password("test").is_ok() {
                    let _ = entry.delete_credential();
                    tracing::info!("Using keyring crate for credential storage");
                    return KeyringBackend::KeyringCrate;
                }
            }
        }

        tracing::warn!("No keyring backend available. Credentials will be stored in database.");
        KeyringBackend::None
    })
}

fn keyring_available() -> bool {
    detect_backend() != KeyringBackend::None
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
    let backend = detect_backend();
    if backend == KeyringBackend::None {
        return Ok(()); // Silently skip - credentials stored in config instead
    }

    if value.is_empty() {
        delete_credential(key).ok();
        return Ok(());
    }

    tracing::debug!("Attempting to store credential: {}", key);

    match backend {
        KeyringBackend::SecretTool => {
            use std::io::Write;
            let result = std::process::Command::new("secret-tool")
                .args(["store", "--label", &format!("lunchbox:{}", key), "service", SERVICE_NAME, "key", key])
                .stdin(std::process::Stdio::piped())
                .spawn()
                .and_then(|mut child| {
                    if let Some(stdin) = child.stdin.as_mut() {
                        stdin.write_all(value.as_bytes())?;
                    }
                    child.wait()
                });

            match result {
                Ok(status) if status.success() => {
                    tracing::debug!("Stored credential in secret-tool: {}", key);
                    Ok(())
                }
                Ok(status) => {
                    tracing::warn!("secret-tool store failed with status {}", status);
                    Ok(())
                }
                Err(e) => {
                    tracing::warn!("Failed to run secret-tool: {:?}", e);
                    Ok(())
                }
            }
        }
        KeyringBackend::KeyringCrate => {
            let entry = match keyring::Entry::new(SERVICE_NAME, key) {
                Ok(e) => e,
                Err(e) => {
                    tracing::warn!("Failed to create keyring entry for {}: {:?}", key, e);
                    return Ok(());
                }
            };
            match entry.set_password(value) {
                Ok(()) => {
                    tracing::debug!("Stored credential in keyring: {}", key);
                    Ok(())
                }
                Err(e) => {
                    tracing::warn!("Failed to store credential {} in keyring: {:?}", key, e);
                    Ok(())
                }
            }
        }
        KeyringBackend::None => Ok(()),
    }
}

/// Retrieve a credential from the system keyring
pub fn get_credential(key: &str) -> Result<Option<String>> {
    let backend = detect_backend();
    if backend == KeyringBackend::None {
        return Ok(None); // Credentials come from config instead
    }

    match backend {
        KeyringBackend::SecretTool => {
            let output = std::process::Command::new("secret-tool")
                .args(["lookup", "service", SERVICE_NAME, "key", key])
                .output();

            match output {
                Ok(out) if out.status.success() => {
                    let value = String::from_utf8_lossy(&out.stdout).trim().to_string();
                    if value.is_empty() {
                        Ok(None)
                    } else {
                        Ok(Some(value))
                    }
                }
                _ => Ok(None),
            }
        }
        KeyringBackend::KeyringCrate => {
            let entry = keyring::Entry::new(SERVICE_NAME, key)?;
            match entry.get_password() {
                Ok(password) => Ok(Some(password)),
                Err(keyring::Error::NoEntry) => Ok(None),
                Err(e) => Err(anyhow::anyhow!("Failed to retrieve credential: {}", e)),
            }
        }
        KeyringBackend::None => Ok(None),
    }
}

/// Delete a credential from the system keyring
pub fn delete_credential(key: &str) -> Result<()> {
    let backend = detect_backend();
    if backend == KeyringBackend::None {
        return Ok(());
    }

    match backend {
        KeyringBackend::SecretTool => {
            let _ = std::process::Command::new("secret-tool")
                .args(["clear", "service", SERVICE_NAME, "key", key])
                .status();
            tracing::debug!("Deleted credential via secret-tool: {}", key);
            Ok(())
        }
        KeyringBackend::KeyringCrate => {
            let entry = keyring::Entry::new(SERVICE_NAME, key)?;
            match entry.delete_credential() {
                Ok(()) => {
                    tracing::debug!("Deleted credential: {}", key);
                    Ok(())
                }
                Err(keyring::Error::NoEntry) => Ok(()),
                Err(e) => Err(anyhow::anyhow!("Failed to delete credential: {}", e)),
            }
        }
        KeyringBackend::None => Ok(()),
    }
}

/// Check if keyring storage is being used
pub fn is_keyring_available() -> bool {
    keyring_available()
}

/// Get the name of the credential storage being used
pub fn get_credential_storage_name() -> &'static str {
    match detect_backend() {
        KeyringBackend::SecretTool => "Secret Service (KWallet/GNOME Keyring)",
        KeyringBackend::KeyringCrate => {
            #[cfg(target_os = "macos")]
            return "macOS Keychain";
            #[cfg(target_os = "windows")]
            return "Windows Credential Manager";
            #[cfg(not(any(target_os = "macos", target_os = "windows")))]
            return "system keyring";
        }
        KeyringBackend::None => "local database",
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
