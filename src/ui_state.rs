use serde::{Serialize, de::DeserializeOwned};

/// Load JSON state from localStorage. Returns None for missing keys or parse errors.
pub fn load_json<T: DeserializeOwned>(key: &str) -> Option<T> {
    let window = web_sys::window()?;
    let storage = window.local_storage().ok().flatten()?;
    let raw = storage.get_item(key).ok().flatten()?;
    serde_json::from_str(&raw).ok()
}

/// Save JSON state to localStorage. Errors are ignored to avoid UI disruption.
pub fn save_json<T: Serialize>(key: &str, value: &T) {
    let Some(window) = web_sys::window() else {
        return;
    };
    let Some(storage) = window.local_storage().ok().flatten() else {
        return;
    };
    let Ok(raw) = serde_json::to_string(value) else {
        return;
    };
    let _ = storage.set_item(key, &raw);
}
