use std::collections::{HashMap, HashSet};
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::Mutex;
use std::time::{SystemTime, UNIX_EPOCH};

use anyhow::{Context, Result, bail};
use directories::ProjectDirs;
use rusqlite::{Connection, OptionalExtension, params};
use serde::{Deserialize, Serialize};

use crate::catalog;
use crate::collections::SmartCollectionRules;
use crate::download_plan::DownloadPlan;

const KEYRING_SERVICE: &str = "com.lunchbox.Lunchbox";
const QBITTORRENT_KEYRING_ACCOUNT: &str = "qbittorrent-password";
const STEAMGRIDDB_KEYRING_ACCOUNT: &str = "steamgriddb-api-key";
const IGDB_KEYRING_ACCOUNT: &str = "igdb-twitch-credentials";
static STORE_INITIALIZATION: Mutex<()> = Mutex::new(());
pub(crate) const CONTROLLER_GAMEPAD_BUTTONS: &[&str] = &[
    "South",
    "East",
    "North",
    "West",
    "Start",
    "Select",
    "Guide",
    "QuickAccess",
    "QuickAccess2",
    "Keyboard",
    "Screenshot",
    "DPadUp",
    "DPadDown",
    "DPadLeft",
    "DPadRight",
    "LeftBumper",
    "LeftTop",
    "LeftTrigger",
    "LeftPaddle1",
    "LeftPaddle2",
    "LeftPaddle3",
    "LeftStick",
    "LeftStickTouch",
    "LeftTouchpadTouch",
    "LeftTouchpadPress",
    "RightBumper",
    "RightTop",
    "RightTrigger",
    "RightPaddle1",
    "RightPaddle2",
    "RightPaddle3",
    "RightStick",
    "RightStickTouch",
    "RightTouchpadTouch",
    "RightTouchpadPress",
];

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct AppSettings {
    pub qbittorrent_host: String,
    pub qbittorrent_port: u16,
    pub qbittorrent_use_https: bool,
    pub qbittorrent_username: String,
    pub rom_directory: PathBuf,
    pub qbittorrent_container_rom_directory: String,
    pub torrent_library_directory: PathBuf,
    pub qbittorrent_container_torrent_library_directory: String,
    pub download_entire_torrent: bool,
    pub file_link_mode: String,
    pub seeding_policy: String,
    pub preferred_region: String,
    pub region_priority: Vec<String>,
    pub version_preference: String,
    pub media_provider_priority: Vec<String>,
    pub controller_mapping: ControllerMappingSettings,
}

impl Default for AppSettings {
    fn default() -> Self {
        Self {
            qbittorrent_host: "127.0.0.1".to_owned(),
            qbittorrent_port: 8080,
            qbittorrent_use_https: false,
            qbittorrent_username: String::new(),
            rom_directory: PathBuf::new(),
            qbittorrent_container_rom_directory: String::new(),
            torrent_library_directory: PathBuf::new(),
            qbittorrent_container_torrent_library_directory: String::new(),
            download_entire_torrent: false,
            file_link_mode: "symlink".to_owned(),
            seeding_policy: "follow_client".to_owned(),
            preferred_region: "USA".to_owned(),
            region_priority: Vec::new(),
            version_preference: "latest".to_owned(),
            media_provider_priority: crate::media::default_provider_priority(),
            controller_mapping: ControllerMappingSettings::default(),
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct ControllerMappingSettings {
    #[serde(default)]
    pub enabled: bool,
    #[serde(default = "default_controller_provider")]
    pub provider: String,
    #[serde(default = "default_controller_target")]
    pub output_target: String,
    #[serde(default = "default_controller_manage_all")]
    pub manage_all: bool,
    #[serde(default)]
    pub default_profile_id: Option<String>,
    #[serde(default)]
    pub profile_controller_ids: Vec<String>,
    #[serde(default)]
    pub player_mappings: Vec<ControllerPlayerMapping>,
    #[serde(default)]
    pub platform_profile_ids: HashMap<String, String>,
    #[serde(default)]
    pub game_profile_ids: HashMap<String, String>,
    #[serde(default)]
    pub hidden_controller_ids: Vec<String>,
    #[serde(default)]
    pub custom_profiles: Vec<ControllerCustomProfile>,
}

impl Default for ControllerMappingSettings {
    fn default() -> Self {
        Self {
            enabled: false,
            provider: default_controller_provider(),
            output_target: default_controller_target(),
            manage_all: true,
            default_profile_id: None,
            profile_controller_ids: Vec::new(),
            player_mappings: Vec::new(),
            platform_profile_ids: HashMap::new(),
            game_profile_ids: HashMap::new(),
            hidden_controller_ids: Vec::new(),
            custom_profiles: Vec::new(),
        }
    }
}

#[derive(Clone, Debug, Default, Eq, PartialEq, Serialize, Deserialize)]
pub struct ControllerPlayerMapping {
    #[serde(default, alias = "controllerId")]
    pub controller_id: Option<String>,
    #[serde(default, alias = "profileId")]
    pub profile_id: Option<String>,
    #[serde(default, alias = "outputTarget")]
    pub output_target: Option<String>,
}

#[derive(Clone, Debug, Default, Eq, PartialEq, Serialize, Deserialize)]
pub struct ControllerCustomProfile {
    #[serde(default)]
    pub id: String,
    #[serde(default)]
    pub name: String,
    #[serde(default)]
    pub layout: String,
    #[serde(default)]
    pub mappings: Vec<ControllerButtonMapping>,
}

#[derive(Clone, Debug, Default, Eq, PartialEq, Serialize, Deserialize)]
pub struct ControllerButtonMapping {
    #[serde(default, alias = "sourceButton")]
    pub source_button: String,
    #[serde(default, alias = "targetButton")]
    pub target_button: String,
}

fn default_controller_provider() -> String {
    "auto".to_owned()
}

fn default_controller_target() -> String {
    "xb360".to_owned()
}

fn default_controller_manage_all() -> bool {
    true
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct LibraryPreferences {
    pub hide_non_retail: bool,
    pub hide_adult: bool,
    pub artwork_type: String,
    pub grid_zoom: i32,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SidebarPreferences {
    pub platform_search: String,
    pub width: i32,
}

#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct GameMetadata {
    pub title: String,
    pub description: String,
    pub release_date: String,
    pub developer: String,
    pub publisher: String,
    pub genre: String,
    pub players: String,
    pub rating: String,
    pub esrb: String,
    pub release_type: String,
    pub notes: String,
}

#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct GameMetadataOverride {
    pub title: Option<String>,
    pub description: Option<String>,
    pub release_date: Option<String>,
    pub developer: Option<String>,
    pub publisher: Option<String>,
    pub genre: Option<String>,
    pub players: Option<String>,
    pub rating: Option<String>,
    pub esrb: Option<String>,
    pub release_type: Option<String>,
    pub notes: Option<String>,
}

impl GameMetadataOverride {
    pub fn from_effective(canonical: &GameMetadata, effective: &GameMetadata) -> Self {
        Self {
            title: changed(&canonical.title, &effective.title),
            description: changed(&canonical.description, &effective.description),
            release_date: changed(&canonical.release_date, &effective.release_date),
            developer: changed(&canonical.developer, &effective.developer),
            publisher: changed(&canonical.publisher, &effective.publisher),
            genre: changed(&canonical.genre, &effective.genre),
            players: changed(&canonical.players, &effective.players),
            rating: changed(&canonical.rating, &effective.rating),
            esrb: changed(&canonical.esrb, &effective.esrb),
            release_type: changed(&canonical.release_type, &effective.release_type),
            notes: changed(&canonical.notes, &effective.notes),
        }
    }

    pub fn apply(&self, canonical: &GameMetadata) -> GameMetadata {
        GameMetadata {
            title: inherited(&self.title, &canonical.title),
            description: inherited(&self.description, &canonical.description),
            release_date: inherited(&self.release_date, &canonical.release_date),
            developer: inherited(&self.developer, &canonical.developer),
            publisher: inherited(&self.publisher, &canonical.publisher),
            genre: inherited(&self.genre, &canonical.genre),
            players: inherited(&self.players, &canonical.players),
            rating: inherited(&self.rating, &canonical.rating),
            esrb: inherited(&self.esrb, &canonical.esrb),
            release_type: inherited(&self.release_type, &canonical.release_type),
            notes: inherited(&self.notes, &canonical.notes),
        }
    }

    pub fn is_empty(&self) -> bool {
        self.title.is_none()
            && self.description.is_none()
            && self.release_date.is_none()
            && self.developer.is_none()
            && self.publisher.is_none()
            && self.genre.is_none()
            && self.players.is_none()
            && self.rating.is_none()
            && self.esrb.is_none()
            && self.release_type.is_none()
            && self.notes.is_none()
    }

    pub fn validate(&self) -> Result<()> {
        validate_optional_metadata("title", &self.title, 500)?;
        validate_optional_metadata("description", &self.description, 100_000)?;
        validate_optional_metadata("release date", &self.release_date, 100)?;
        validate_optional_metadata("developer", &self.developer, 1_000)?;
        validate_optional_metadata("publisher", &self.publisher, 1_000)?;
        validate_optional_metadata("genre", &self.genre, 1_000)?;
        validate_optional_metadata("players", &self.players, 100)?;
        validate_optional_metadata("rating", &self.rating, 20)?;
        validate_optional_metadata("age rating", &self.esrb, 100)?;
        validate_optional_metadata("release type", &self.release_type, 500)?;
        validate_optional_metadata("notes", &self.notes, 100_000)?;
        if self
            .title
            .as_deref()
            .is_some_and(|title| title.trim().is_empty())
        {
            bail!("title cannot be empty");
        }
        if let Some(rating) = self.rating.as_deref()
            && !rating.trim().is_empty()
        {
            let rating = rating
                .trim()
                .parse::<f32>()
                .context("rating must be a number between 0 and 5")?;
            if !(0.0..=5.0).contains(&rating) {
                bail!("rating must be between 0 and 5");
            }
        }
        Ok(())
    }
}

fn changed(canonical: &str, effective: &str) -> Option<String> {
    (canonical != effective).then(|| effective.to_owned())
}

fn inherited(value: &Option<String>, canonical: &str) -> String {
    value.clone().unwrap_or_else(|| canonical.to_owned())
}

fn validate_optional_metadata(label: &str, value: &Option<String>, maximum: usize) -> Result<()> {
    if value
        .as_deref()
        .is_some_and(|value| value.chars().count() > maximum)
    {
        bail!("{label} must be {maximum} characters or fewer");
    }
    Ok(())
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct UserCollection {
    pub id: String,
    pub name: String,
    pub description: String,
    pub kind: String,
    pub rules: Option<SmartCollectionRules>,
    pub game_count: usize,
}

#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct UserCollections {
    pub collections: Vec<UserCollection>,
    pub members: HashMap<String, Vec<String>>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct UserTag {
    pub id: String,
    pub name: String,
    pub game_count: usize,
}

#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct UserTags {
    pub tags: Vec<UserTag>,
    pub game_tags: HashMap<String, Vec<String>>,
}

pub fn parse_game_tags(input: &str) -> Result<Vec<String>> {
    if input.trim().is_empty() {
        return Ok(Vec::new());
    }
    let mut tags = Vec::new();
    let mut normalized_names = HashSet::new();
    for raw_name in input.split(',') {
        let name = normalize_tag_name(raw_name)?;
        let normalized = normalized_tag_name(&name);
        if normalized_names.insert(normalized) {
            tags.push(name);
        }
    }
    if tags.len() > 32 {
        bail!("a game can have at most 32 tags");
    }
    tags.sort_by_cached_key(|name| normalized_tag_name(name));
    Ok(tags)
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct EmulatorPreference {
    pub emulator_id: String,
    pub runtime_kind: String,
    pub core_name: String,
    pub scope: String,
}

#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct EmulatorLaunchProfile {
    pub scope_kind: String,
    pub scope_key: String,
    pub emulator_id: String,
    pub runtime_kind: String,
    pub core_name: String,
    pub extra_arguments: String,
    pub command_template: String,
    pub updated_at: i64,
}

#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct ResolvedLaunchCustomization {
    pub extra_arguments: String,
    pub command_template: String,
    pub argument_scope: String,
    pub template_scope: String,
}

#[derive(Clone, Debug, Eq, Hash, PartialEq)]
pub struct ManagedEmulatorInstall {
    pub emulator_id: String,
    pub host_system_slug: String,
    pub manager: String,
    pub package_id: String,
    pub install_path: String,
    pub installed_at: i64,
    pub updated_at: i64,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct FirmwarePackageReceipt {
    pub source_id: String,
    pub package_name: String,
    pub archive_path: String,
    pub extracted_root: String,
    pub sha256: String,
    pub file_count: i64,
    pub imported_at: i64,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct FirmwareFileReceipt {
    pub source_id: String,
    pub package_name: String,
    pub relative_path: String,
    pub store_path: String,
    pub sha256: String,
    pub file_size: i64,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct FirmwareInstallReceipt {
    pub rule_key: String,
    pub runtime_path: String,
    pub target_path: String,
    pub source_id: String,
    pub package_name: String,
    pub synced_at: i64,
}

#[derive(Clone, Debug, PartialEq)]
pub struct FirmwareDownloadReceipt {
    pub source_id: String,
    pub package_name: String,
    pub torrent_url: String,
    pub info_hash: String,
    pub file_index: u32,
    pub file_path: String,
    pub local_path: String,
    pub state: String,
    pub progress: f64,
    pub message: String,
    pub updated_at: i64,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PlayActivity {
    pub game_uid: String,
    pub launchbox_db_id: i64,
    pub title: String,
    pub platform: String,
    pub play_count: i64,
    pub total_play_time_seconds: i64,
    pub last_played_at: i64,
    pub first_played_at: i64,
    pub completion_state: String,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PlaySessionStart {
    pub session_id: String,
    pub started_at: i64,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct MediaPlaybackProgress {
    pub game_uid: String,
    pub media_key: String,
    pub position_ms: i64,
    pub duration_ms: i64,
    pub updated_at: i64,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct MediaProviderGameLink {
    pub provider: String,
    pub launchbox_db_id: i64,
    pub provider_game_id: String,
    pub provider_game_name: String,
    pub reviewed_at: i64,
}

#[derive(Clone, Debug, PartialEq)]
pub struct MediaTransfer {
    pub game_uid: String,
    pub launchbox_db_id: i64,
    pub title: String,
    pub platform: String,
    pub media_kind: String,
    pub provider: String,
    pub transport: String,
    pub source_url: String,
    pub remote_job_id: String,
    pub source_item_index: u32,
    pub source_item_path: String,
    pub local_download_path: PathBuf,
    pub local_target_path: PathBuf,
    pub state: String,
    pub progress: f64,
    pub download_speed: u64,
    pub downloaded_bytes: u64,
    pub total_bytes: u64,
    pub message: String,
    pub created_at: i64,
    pub updated_at: i64,
}

impl Default for LibraryPreferences {
    fn default() -> Self {
        Self {
            hide_non_retail: true,
            hide_adult: false,
            artwork_type: "box-front".to_owned(),
            grid_zoom: 100,
        }
    }
}

impl LibraryPreferences {
    pub fn validate(&self) -> Result<()> {
        if crate::media::ArtworkKind::parse(&self.artwork_type).is_none() {
            bail!("unsupported artwork type {}", self.artwork_type);
        }
        if !(50..=200).contains(&self.grid_zoom) {
            bail!("grid zoom must be between 50 and 200 percent");
        }
        Ok(())
    }
}

impl Default for SidebarPreferences {
    fn default() -> Self {
        Self {
            platform_search: String::new(),
            width: 254,
        }
    }
}

impl SidebarPreferences {
    pub fn validate(&self) -> Result<()> {
        if self.platform_search.chars().count() > 80 {
            bail!("platform search must be at most 80 characters");
        }
        if self.platform_search.contains('\0') {
            bail!("platform search cannot contain a null character");
        }
        if !(180..=400).contains(&self.width) {
            bail!("sidebar width must be between 180 and 400 pixels");
        }
        Ok(())
    }
}

impl AppSettings {
    pub fn validate(&self) -> Result<()> {
        if self.qbittorrent_host.trim().is_empty() {
            bail!("qBittorrent host is required");
        }
        if self.qbittorrent_port == 0 {
            bail!("qBittorrent port must be between 1 and 65535");
        }
        if !matches!(
            self.file_link_mode.as_str(),
            "symlink" | "hardlink" | "reflink" | "copy" | "leave_in_place"
        ) {
            bail!("unsupported file link mode {}", self.file_link_mode);
        }
        if !matches!(
            self.seeding_policy.as_str(),
            "follow_client" | "pause_after_import"
        ) {
            bail!("unsupported seeding policy {}", self.seeding_policy);
        }
        if !matches!(
            self.preferred_region.as_str(),
            "USA" | "Japan" | "Europe" | "World" | "Asia" | "any"
        ) {
            bail!("unsupported preferred region {}", self.preferred_region);
        }
        crate::region_priority::normalize_custom_priority(&self.region_priority)?;
        crate::media::normalize_provider_priority(&self.media_provider_priority)?;
        if !matches!(self.version_preference.as_str(), "latest" | "original") {
            bail!(
                "unsupported release version preference {}",
                self.version_preference
            );
        }
        self.controller_mapping.validate()?;
        Ok(())
    }

    pub fn qbittorrent_base_url(&self) -> String {
        let scheme = if self.qbittorrent_use_https {
            "https"
        } else {
            "http"
        };
        format!(
            "{scheme}://{}:{}",
            self.qbittorrent_host.trim(),
            self.qbittorrent_port
        )
    }
}

impl ControllerMappingSettings {
    pub fn validate(&self) -> Result<()> {
        if !matches!(self.provider.trim(), "auto" | "inputplumber") {
            bail!("unsupported controller provider {}", self.provider);
        }
        validate_controller_target(&self.output_target)?;
        if self.player_mappings.len() > 16 {
            bail!("controller player order cannot contain more than 16 entries");
        }
        let mut controller_ids = HashSet::new();
        for player in &self.player_mappings {
            let Some(controller_id) = player
                .controller_id
                .as_deref()
                .map(str::trim)
                .filter(|value| !value.is_empty())
            else {
                continue;
            };
            validate_controller_id(controller_id)?;
            if !controller_ids.insert(controller_id) {
                bail!("controller player order contains duplicate device {controller_id}");
            }
            if let Some(target) = player
                .output_target
                .as_deref()
                .map(str::trim)
                .filter(|value| !value.is_empty())
            {
                validate_controller_target(target)?;
            }
            if player
                .profile_id
                .as_deref()
                .is_some_and(|profile| profile.chars().count() > 256)
            {
                bail!("controller profile identifier is too long");
            }
        }
        if self.hidden_controller_ids.len() > 64 {
            bail!("hidden controller list cannot contain more than 64 entries");
        }
        let mut hidden_ids = HashSet::new();
        for controller_id in &self.hidden_controller_ids {
            let controller_id = controller_id.trim();
            validate_controller_id(controller_id)?;
            if !hidden_ids.insert(controller_id) {
                bail!("hidden controller list contains duplicate device {controller_id}");
            }
        }
        if self.custom_profiles.len() > 64 {
            bail!("controller profile list cannot contain more than 64 entries");
        }
        if self.platform_profile_ids.len() > 512 || self.game_profile_ids.len() > 4096 {
            bail!("controller profile override list is too large");
        }
        let mut profile_ids = HashSet::new();
        for profile in &self.custom_profiles {
            let profile_id = profile.id.trim();
            if !profile_id.starts_with("custom:") || profile_id.chars().count() > 128 {
                bail!(
                    "custom controller profile id must use the custom: prefix and contain at most 128 characters"
                );
            }
            if !profile_ids.insert(profile_id) {
                bail!("controller profile list contains duplicate id {profile_id}");
            }
            if profile.name.trim().is_empty() || profile.name.chars().count() > 100 {
                bail!("controller profile name must contain 1 to 100 characters");
            }
            if !matches!(profile.layout.trim(), "xbox" | "playstation" | "generic") {
                bail!("unsupported controller profile layout {}", profile.layout);
            }
            if profile.mappings.len() > 64 {
                bail!("controller profile contains too many button mappings");
            }
            let mut target_buttons = HashSet::new();
            for button_mapping in &profile.mappings {
                if !CONTROLLER_GAMEPAD_BUTTONS.contains(&button_mapping.source_button.as_str()) {
                    bail!(
                        "unsupported controller source button {}",
                        button_mapping.source_button
                    );
                }
                if !CONTROLLER_GAMEPAD_BUTTONS.contains(&button_mapping.target_button.as_str()) {
                    bail!(
                        "unsupported controller target button {}",
                        button_mapping.target_button
                    );
                }
                if !target_buttons.insert(button_mapping.target_button.as_str()) {
                    bail!(
                        "controller profile contains duplicate target button {}",
                        button_mapping.target_button
                    );
                }
            }
        }
        validate_controller_profile_override_map("platform", &self.platform_profile_ids, 256)?;
        validate_controller_profile_override_map("game", &self.game_profile_ids, 64)?;
        Ok(())
    }
}

fn validate_controller_profile_override_map(
    scope: &str,
    overrides: &HashMap<String, String>,
    key_limit: usize,
) -> Result<()> {
    for (key, profile_id) in overrides {
        if key.trim().is_empty()
            || key.chars().count() > key_limit
            || key.chars().any(char::is_control)
        {
            bail!("invalid {scope} controller profile override key");
        }
        if profile_id.chars().count() > 256 || profile_id.chars().any(char::is_control) {
            bail!("invalid {scope} controller profile override value");
        }
    }
    Ok(())
}

fn validate_controller_id(controller_id: &str) -> Result<()> {
    if controller_id.is_empty() || controller_id.chars().count() > 256 {
        bail!("controller device id must contain 1 to 256 characters");
    }
    if controller_id.starts_with('-') || controller_id.chars().any(char::is_control) {
        bail!("controller device id contains unsupported characters");
    }
    Ok(())
}

fn validate_controller_target(targets: &str) -> Result<()> {
    let targets = targets.split(',').map(str::trim).collect::<Vec<_>>();
    if targets.is_empty() || targets.len() > 8 || targets.iter().any(|target| target.is_empty()) {
        bail!("controller target list must contain between 1 and 8 targets");
    }
    for target in targets {
        if target.chars().count() > 64
            || target.starts_with('-')
            || !target.chars().all(|character| {
                character.is_ascii_alphanumeric() || matches!(character, '-' | '_')
            })
        {
            bail!("unsupported controller target {target}");
        }
    }
    Ok(())
}

#[derive(Clone, Debug, PartialEq)]
pub struct DownloadJob {
    pub id: String,
    pub game_id: String,
    pub launchbox_db_id: i64,
    pub title: String,
    pub platform: String,
    pub source_kind: String,
    pub torrent_url: String,
    pub torrent_file_index: Option<u32>,
    pub torrent_file_path: String,
    pub info_hash: String,
    pub client_save_path: String,
    pub local_download_path: PathBuf,
    pub local_target_path: PathBuf,
    pub state: String,
    pub progress: f64,
    pub download_speed: u64,
    pub downloaded_bytes: u64,
    pub total_bytes: u64,
    pub message: String,
    pub post_import_action: String,
    pub created_at: i64,
    pub updated_at: i64,
    pub download_plan: String,
}

pub(crate) struct NewDownloadJob {
    pub game_id: String,
    pub launchbox_db_id: i64,
    pub title: String,
    pub platform: String,
    pub source_kind: String,
    pub torrent_url: String,
    pub torrent_file_index: Option<u32>,
    pub torrent_file_path: String,
    pub info_hash: String,
    pub client_save_path: String,
    pub local_download_path: PathBuf,
    pub local_target_path: PathBuf,
    pub download_plan: String,
}

impl DownloadJob {
    pub(crate) fn queued(new_job: NewDownloadJob) -> Self {
        let now = unix_timestamp();
        Self {
            id: uuid::Uuid::new_v4().to_string(),
            game_id: new_job.game_id,
            launchbox_db_id: new_job.launchbox_db_id,
            title: new_job.title,
            platform: new_job.platform,
            source_kind: new_job.source_kind,
            torrent_url: new_job.torrent_url,
            torrent_file_index: new_job.torrent_file_index,
            torrent_file_path: new_job.torrent_file_path,
            info_hash: new_job.info_hash,
            client_save_path: new_job.client_save_path,
            local_download_path: new_job.local_download_path,
            local_target_path: new_job.local_target_path,
            state: "queued".to_owned(),
            progress: 0.0,
            download_speed: 0,
            downloaded_bytes: 0,
            total_bytes: 0,
            message: "Queued in qBittorrent".to_owned(),
            post_import_action: "none".to_owned(),
            created_at: now,
            updated_at: now,
            download_plan: new_job.download_plan,
        }
    }

    pub fn parsed_download_plan(&self) -> Result<Option<DownloadPlan>> {
        if self.download_plan.trim().is_empty() {
            return Ok(None);
        }
        let plan: DownloadPlan = serde_json::from_str(&self.download_plan)
            .context("decoding persisted download plan")?;
        plan.validate()?;
        Ok(Some(plan))
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct ManualTorrentSourceReceipt {
    pub game_uid: String,
    pub launchbox_db_id: i64,
    pub canonical_title: String,
    pub platform: String,
    pub source_file_name: String,
    pub torrent_name: String,
    pub torrent_sha256: String,
    pub info_hash: String,
    pub selected_file_index: u32,
    pub selected_file_path: String,
    pub queued_job_id: String,
    pub reviewed_at: i64,
}

#[derive(Clone, Debug)]
pub struct SettingsStore {
    path: PathBuf,
}

impl SettingsStore {
    pub fn open_default() -> Result<Self> {
        Self::at(state_database_path()?)
    }

    pub fn at(path: impl Into<PathBuf>) -> Result<Self> {
        let store = Self { path: path.into() };
        if let Some(parent) = store.path.parent()
            && !parent.as_os_str().is_empty()
        {
            fs::create_dir_all(parent)
                .with_context(|| format!("creating state directory {}", parent.display()))?;
        }
        let _initialization = STORE_INITIALIZATION
            .lock()
            .map_err(|_| anyhow::anyhow!("Lunchbox state initialization lock was poisoned"))?;
        let connection = store.connection()?;
        let journal_mode =
            connection.query_row("PRAGMA journal_mode", [], |row| row.get::<_, String>(0))?;
        if !journal_mode.eq_ignore_ascii_case("wal") {
            let enabled = connection
                .query_row("PRAGMA journal_mode=WAL", [], |row| row.get::<_, String>(0))?;
            if !enabled.eq_ignore_ascii_case("wal") {
                bail!("could not enable WAL mode for the Lunchbox state database");
            }
        }
        migrate(&connection)?;
        Ok(store)
    }

    pub fn path(&self) -> &Path {
        &self.path
    }

    pub fn media_provider_game_link(
        &self,
        provider: &str,
        launchbox_db_id: i64,
    ) -> Result<Option<MediaProviderGameLink>> {
        if provider.trim().is_empty() || launchbox_db_id <= 0 {
            return Ok(None);
        }
        self.connection()?
            .query_row(
                "SELECT provider, launchbox_db_id, provider_game_id,
                        provider_game_name, reviewed_at
                 FROM media_provider_game_links
                 WHERE provider=?1 AND launchbox_db_id=?2",
                params![provider, launchbox_db_id],
                |row| {
                    Ok(MediaProviderGameLink {
                        provider: row.get(0)?,
                        launchbox_db_id: row.get(1)?,
                        provider_game_id: row.get(2)?,
                        provider_game_name: row.get(3)?,
                        reviewed_at: row.get(4)?,
                    })
                },
            )
            .optional()
            .context("loading a reviewed media-provider game link")
    }

    pub fn save_media_provider_game_link(&self, link: &MediaProviderGameLink) -> Result<()> {
        if link.provider.trim().is_empty()
            || link.launchbox_db_id <= 0
            || link.provider_game_id.trim().is_empty()
            || link.provider_game_name.trim().is_empty()
            || link.reviewed_at < 0
        {
            bail!("reviewed media-provider game links require complete stable identifiers");
        }
        self.connection()?.execute(
            "INSERT INTO media_provider_game_links (
                 provider, launchbox_db_id, provider_game_id,
                 provider_game_name, reviewed_at
             ) VALUES (?1, ?2, ?3, ?4, ?5)
             ON CONFLICT(provider, launchbox_db_id) DO UPDATE SET
                 provider_game_id=excluded.provider_game_id,
                 provider_game_name=excluded.provider_game_name,
                 reviewed_at=excluded.reviewed_at",
            params![
                link.provider,
                link.launchbox_db_id,
                link.provider_game_id,
                link.provider_game_name,
                link.reviewed_at,
            ],
        )?;
        Ok(())
    }

    pub fn load(&self) -> Result<AppSettings> {
        let connection = self.connection()?;
        let mut settings = connection
            .query_row(
                "SELECT qbittorrent_host, qbittorrent_port, qbittorrent_use_https,
                        qbittorrent_username, rom_directory,
                        qbittorrent_container_rom_directory, torrent_library_directory,
                        qbittorrent_container_torrent_library_directory,
                        download_entire_torrent, file_link_mode, seeding_policy,
                        preferred_region, version_preference, region_priority_json,
                        controller_mapping_json, media_provider_priority_json
                 FROM app_settings WHERE id=1",
                [],
                |row| {
                    let port = row.get::<_, i64>(1)?;
                    Ok(AppSettings {
                        qbittorrent_host: row.get(0)?,
                        qbittorrent_port: u16::try_from(port).unwrap_or_default(),
                        qbittorrent_use_https: row.get(2)?,
                        qbittorrent_username: row.get(3)?,
                        rom_directory: PathBuf::from(row.get::<_, String>(4)?),
                        qbittorrent_container_rom_directory: row.get(5)?,
                        torrent_library_directory: PathBuf::from(row.get::<_, String>(6)?),
                        qbittorrent_container_torrent_library_directory: row.get(7)?,
                        download_entire_torrent: row.get(8)?,
                        file_link_mode: row.get(9)?,
                        seeding_policy: row.get(10)?,
                        preferred_region: row.get(11)?,
                        version_preference: row.get(12)?,
                        region_priority: serde_json::from_str(&row.get::<_, String>(13)?).map_err(
                            |error| {
                                rusqlite::Error::FromSqlConversionFailure(
                                    13,
                                    rusqlite::types::Type::Text,
                                    Box::new(error),
                                )
                            },
                        )?,
                        controller_mapping: serde_json::from_str(&row.get::<_, String>(14)?)
                            .map_err(|error| {
                                rusqlite::Error::FromSqlConversionFailure(
                                    14,
                                    rusqlite::types::Type::Text,
                                    Box::new(error),
                                )
                            })?,
                        media_provider_priority: serde_json::from_str(&row.get::<_, String>(15)?)
                            .map_err(|error| {
                            rusqlite::Error::FromSqlConversionFailure(
                                15,
                                rusqlite::types::Type::Text,
                                Box::new(error),
                            )
                        })?,
                    })
                },
            )
            .optional()?
            .unwrap_or_default();
        if settings.region_priority.is_empty()
            && !matches!(settings.preferred_region.as_str(), "USA" | "any")
        {
            settings
                .region_priority
                .push(settings.preferred_region.clone());
        }
        settings.media_provider_priority =
            crate::media::effective_provider_priority(&settings.media_provider_priority);
        settings.validate()?;
        Ok(settings)
    }

    pub fn save(&self, settings: &AppSettings) -> Result<()> {
        settings.validate()?;
        let mut connection = self.connection()?;
        let transaction = connection.transaction()?;
        transaction.execute(
            "INSERT INTO app_settings (
                 id, qbittorrent_host, qbittorrent_port, qbittorrent_use_https,
                 qbittorrent_username, rom_directory,
                 qbittorrent_container_rom_directory, torrent_library_directory,
                 qbittorrent_container_torrent_library_directory,
                 download_entire_torrent, file_link_mode, seeding_policy,
                 preferred_region, version_preference, region_priority_json,
                 controller_mapping_json, media_provider_priority_json
             ) VALUES (1, ?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15, ?16)
             ON CONFLICT(id) DO UPDATE SET
                 qbittorrent_host=excluded.qbittorrent_host,
                 qbittorrent_port=excluded.qbittorrent_port,
                 qbittorrent_use_https=excluded.qbittorrent_use_https,
                 qbittorrent_username=excluded.qbittorrent_username,
                 rom_directory=excluded.rom_directory,
                 qbittorrent_container_rom_directory=excluded.qbittorrent_container_rom_directory,
                 torrent_library_directory=excluded.torrent_library_directory,
                 qbittorrent_container_torrent_library_directory=excluded.qbittorrent_container_torrent_library_directory,
                 download_entire_torrent=excluded.download_entire_torrent,
                 file_link_mode=excluded.file_link_mode,
                 seeding_policy=excluded.seeding_policy,
                 preferred_region=excluded.preferred_region,
                 version_preference=excluded.version_preference,
                 region_priority_json=excluded.region_priority_json,
                 controller_mapping_json=excluded.controller_mapping_json,
                 media_provider_priority_json=excluded.media_provider_priority_json",
            params![
                settings.qbittorrent_host,
                i64::from(settings.qbittorrent_port),
                settings.qbittorrent_use_https,
                settings.qbittorrent_username,
                path_text(&settings.rom_directory),
                settings.qbittorrent_container_rom_directory,
                path_text(&settings.torrent_library_directory),
                settings.qbittorrent_container_torrent_library_directory,
                settings.download_entire_torrent,
                settings.file_link_mode,
                settings.seeding_policy,
                settings.preferred_region,
                settings.version_preference,
                serde_json::to_string(&settings.region_priority)
                    .context("encoding release region priority")?,
                serde_json::to_string(&settings.controller_mapping)
                    .context("encoding controller mapping settings")?,
                serde_json::to_string(&settings.media_provider_priority)
                    .context("encoding media provider priority")?,
            ],
        )?;
        transaction.commit()?;
        Ok(())
    }

    pub fn load_library_preferences(&self) -> Result<LibraryPreferences> {
        let connection = self.connection()?;
        let preferences = connection
            .query_row(
                "SELECT hide_non_retail, hide_adult, artwork_type, grid_zoom
                 FROM library_preferences WHERE id=1",
                [],
                |row| {
                    Ok(LibraryPreferences {
                        hide_non_retail: row.get(0)?,
                        hide_adult: row.get(1)?,
                        artwork_type: row.get(2)?,
                        grid_zoom: row.get(3)?,
                    })
                },
            )
            .optional()?
            .unwrap_or_default();
        preferences.validate()?;
        Ok(preferences)
    }

    pub fn save_library_preferences(&self, preferences: &LibraryPreferences) -> Result<()> {
        preferences.validate()?;
        let connection = self.connection()?;
        connection.execute(
            "INSERT INTO library_preferences (
                 id, hide_non_retail, hide_adult, artwork_type, grid_zoom
             ) VALUES (1, ?1, ?2, ?3, ?4)
             ON CONFLICT(id) DO UPDATE SET
                 hide_non_retail=excluded.hide_non_retail,
                 hide_adult=excluded.hide_adult,
                 artwork_type=excluded.artwork_type,
                 grid_zoom=excluded.grid_zoom",
            params![
                preferences.hide_non_retail,
                preferences.hide_adult,
                preferences.artwork_type,
                preferences.grid_zoom,
            ],
        )?;
        Ok(())
    }

    pub fn load_sidebar_preferences(&self) -> Result<SidebarPreferences> {
        let preferences = self
            .connection()?
            .query_row(
                "SELECT platform_search, width FROM sidebar_preferences WHERE id=1",
                [],
                |row| {
                    Ok(SidebarPreferences {
                        platform_search: row.get(0)?,
                        width: row.get(1)?,
                    })
                },
            )
            .optional()?
            .unwrap_or_default();
        preferences.validate()?;
        Ok(preferences)
    }

    pub fn save_sidebar_preferences(&self, preferences: &SidebarPreferences) -> Result<()> {
        preferences.validate()?;
        self.connection()?.execute(
            "INSERT INTO sidebar_preferences (id, platform_search, width)
             VALUES (1, ?1, ?2)
             ON CONFLICT(id) DO UPDATE SET
                 platform_search=excluded.platform_search,
                 width=excluded.width",
            params![preferences.platform_search, preferences.width],
        )?;
        Ok(())
    }

    pub fn game_metadata_override(&self, game_uid: &str) -> Result<GameMetadataOverride> {
        if game_uid.trim().is_empty() {
            bail!("a stable game identity is required to load metadata overrides");
        }
        Ok(self
            .connection()?
            .query_row(
                "SELECT title, description, release_date, developer, publisher,
                        genre, players, rating, esrb, release_type, notes
                 FROM game_metadata_overrides WHERE game_uid=?1",
                [game_uid],
                metadata_override_from_row,
            )
            .optional()?
            .unwrap_or_default())
    }

    pub fn all_game_metadata_overrides(&self) -> Result<HashMap<String, GameMetadataOverride>> {
        let connection = self.connection()?;
        let mut statement = connection.prepare(
            "SELECT game_uid, title, description, release_date, developer, publisher,
                    genre, players, rating, esrb, release_type, notes
             FROM game_metadata_overrides ORDER BY game_uid",
        )?;
        let rows = statement.query_map([], |row| {
            Ok((
                row.get::<_, String>(0)?,
                GameMetadataOverride {
                    title: row.get(1)?,
                    description: row.get(2)?,
                    release_date: row.get(3)?,
                    developer: row.get(4)?,
                    publisher: row.get(5)?,
                    genre: row.get(6)?,
                    players: row.get(7)?,
                    rating: row.get(8)?,
                    esrb: row.get(9)?,
                    release_type: row.get(10)?,
                    notes: row.get(11)?,
                },
            ))
        })?;
        Ok(rows.collect::<rusqlite::Result<HashMap<_, _>>>()?)
    }

    pub fn game_tags(&self, game_uid: &str) -> Result<Vec<String>> {
        validate_tag_game_uid(game_uid)?;
        let connection = self.connection()?;
        let mut statement = connection.prepare(
            "SELECT t.name
             FROM game_tags gt
             JOIN user_tags t ON t.id=gt.tag_id
             WHERE gt.game_uid=?1
             ORDER BY t.normalized_name, t.name",
        )?;
        let rows = statement.query_map([game_uid], |row| row.get::<_, String>(0))?;
        Ok(rows.collect::<rusqlite::Result<Vec<_>>>()?)
    }

    pub fn all_user_tags(&self) -> Result<UserTags> {
        let connection = self.connection()?;
        let mut tag_statement = connection.prepare(
            "SELECT t.id, t.name, COUNT(gt.game_uid)
             FROM user_tags t
             LEFT JOIN game_tags gt ON gt.tag_id=t.id
             GROUP BY t.id, t.name, t.normalized_name
             ORDER BY t.normalized_name, t.name",
        )?;
        let tag_rows = tag_statement.query_map([], |row| {
            Ok(UserTag {
                id: row.get(0)?,
                name: row.get(1)?,
                game_count: row.get::<_, i64>(2)?.max(0) as usize,
            })
        })?;
        let tags = tag_rows.collect::<rusqlite::Result<Vec<_>>>()?;

        let mut membership_statement = connection.prepare(
            "SELECT gt.game_uid, t.name
             FROM game_tags gt
             JOIN user_tags t ON t.id=gt.tag_id
             ORDER BY gt.game_uid, t.normalized_name, t.name",
        )?;
        let membership_rows = membership_statement.query_map([], |row| {
            Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?))
        })?;
        let mut game_tags = HashMap::<String, Vec<String>>::new();
        for row in membership_rows {
            let (game_uid, name) = row?;
            game_tags.entry(game_uid).or_default().push(name);
        }
        Ok(UserTags { tags, game_tags })
    }

    pub fn save_game_metadata_and_tags(
        &self,
        game_uid: &str,
        launchbox_db_id: i64,
        canonical_title: &str,
        platform: &str,
        metadata: &GameMetadataOverride,
        tag_input: &str,
    ) -> Result<Vec<String>> {
        validate_metadata_identity(game_uid, launchbox_db_id, canonical_title, platform)?;
        metadata.validate()?;
        let tags = parse_game_tags(tag_input)?;
        let mut connection = self.connection()?;
        let transaction = connection.transaction()?;
        save_game_metadata_override_on(
            &transaction,
            game_uid,
            launchbox_db_id,
            canonical_title,
            platform,
            metadata,
        )?;
        transaction.execute("DELETE FROM game_tags WHERE game_uid=?1", [game_uid])?;
        let timestamp = unix_timestamp();
        let mut stored_names = Vec::with_capacity(tags.len());
        for name in tags {
            let normalized = normalized_tag_name(&name);
            let id = stable_tag_id(&normalized);
            transaction.execute(
                "INSERT INTO user_tags (
                     id, name, normalized_name, created_at, updated_at
                 ) VALUES (?1, ?2, ?3, ?4, ?4)
                 ON CONFLICT(normalized_name) DO UPDATE SET updated_at=excluded.updated_at",
                params![id, name, normalized, timestamp],
            )?;
            let (stored_id, stored_name) = transaction.query_row(
                "SELECT id, name FROM user_tags WHERE normalized_name=?1",
                [&normalized],
                |row| Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?)),
            )?;
            transaction.execute(
                "INSERT INTO game_tags (tag_id, game_uid, added_at) VALUES (?1, ?2, ?3)",
                params![stored_id, game_uid, timestamp],
            )?;
            stored_names.push(stored_name);
        }
        transaction.execute(
            "DELETE FROM user_tags
             WHERE NOT EXISTS (
                 SELECT 1 FROM game_tags gt WHERE gt.tag_id=user_tags.id
             )",
            [],
        )?;
        transaction.commit()?;
        stored_names.sort_by_cached_key(|name| normalized_tag_name(name));
        Ok(stored_names)
    }

    pub fn favorite_game_ids(&self) -> Result<HashSet<String>> {
        let connection = self.connection()?;
        let mut statement = connection.prepare("SELECT game_uid FROM favorite_games")?;
        let rows = statement.query_map([], |row| row.get::<_, String>(0))?;
        Ok(rows.collect::<rusqlite::Result<HashSet<_>>>()?)
    }

    pub fn set_favorite(
        &self,
        game_uid: &str,
        launchbox_db_id: i64,
        title: &str,
        platform: &str,
        favorite: bool,
    ) -> Result<()> {
        if game_uid.trim().is_empty() {
            bail!("a stable game identity is required to change favorites");
        }
        let connection = self.connection()?;
        if favorite {
            connection.execute(
                "INSERT INTO favorite_games (
                     game_uid, launchbox_db_id, title, platform, added_at
                 ) VALUES (?1, ?2, ?3, ?4, ?5)
                 ON CONFLICT(game_uid) DO UPDATE SET
                     launchbox_db_id=excluded.launchbox_db_id,
                     title=excluded.title,
                     platform=excluded.platform",
                params![game_uid, launchbox_db_id, title, platform, unix_timestamp()],
            )?;
        } else {
            connection.execute(
                "DELETE FROM favorite_games WHERE game_uid=?1",
                params![game_uid],
            )?;
        }
        Ok(())
    }

    pub fn play_activity(&self, game_uid: &str) -> Result<Option<PlayActivity>> {
        if game_uid.trim().is_empty() {
            bail!("a stable game identity is required to load play activity");
        }
        let connection = self.connection()?;
        Ok(connection
            .query_row(
                "SELECT game_uid, launchbox_db_id, title, platform, play_count,
                        total_play_time_seconds, last_played_at, first_played_at,
                        completion_state
                 FROM game_activity WHERE game_uid=?1",
                [game_uid],
                play_activity_from_row,
            )
            .optional()?)
    }

    pub fn all_play_activity(&self) -> Result<Vec<PlayActivity>> {
        let connection = self.connection()?;
        let mut statement = connection.prepare(
            "SELECT game_uid, launchbox_db_id, title, platform, play_count,
                    total_play_time_seconds, last_played_at, first_played_at,
                    completion_state
             FROM game_activity
             ORDER BY game_uid",
        )?;
        let rows = statement.query_map([], play_activity_from_row)?;
        Ok(rows.collect::<rusqlite::Result<Vec<_>>>()?)
    }

    pub fn media_playback_progress(
        &self,
        game_uid: &str,
        media_key: &str,
    ) -> Result<Option<MediaPlaybackProgress>> {
        validate_media_playback_identity(game_uid, media_key)?;
        let connection = self.connection()?;
        Ok(connection
            .query_row(
                "SELECT game_uid, media_key, position_ms, duration_ms, updated_at
                 FROM media_playback_progress
                 WHERE game_uid=?1 AND media_key=?2",
                params![game_uid, media_key],
                |row| {
                    Ok(MediaPlaybackProgress {
                        game_uid: row.get(0)?,
                        media_key: row.get(1)?,
                        position_ms: row.get(2)?,
                        duration_ms: row.get(3)?,
                        updated_at: row.get(4)?,
                    })
                },
            )
            .optional()?)
    }

    pub fn save_media_playback_progress(
        &self,
        game_uid: &str,
        media_key: &str,
        position_ms: i64,
        duration_ms: i64,
    ) -> Result<MediaPlaybackProgress> {
        validate_media_playback_identity(game_uid, media_key)?;
        if position_ms < 0 || duration_ms < 0 {
            bail!("media playback position and duration cannot be negative");
        }
        let position_ms = if duration_ms > 0 {
            position_ms.min(duration_ms)
        } else {
            position_ms
        };
        let updated_at = unix_timestamp();
        let mut connection = self.connection()?;
        let transaction = connection.transaction()?;
        transaction.execute(
            "INSERT INTO media_playback_progress (
                 game_uid, media_key, position_ms, duration_ms, updated_at
             ) VALUES (?1, ?2, ?3, ?4, ?5)
             ON CONFLICT(game_uid, media_key) DO UPDATE SET
                 position_ms=excluded.position_ms,
                 duration_ms=excluded.duration_ms,
                 updated_at=excluded.updated_at",
            params![game_uid, media_key, position_ms, duration_ms, updated_at],
        )?;
        transaction.execute(
            "DELETE FROM media_playback_progress
             WHERE game_uid=?1 AND media_key<>?2",
            params![game_uid, media_key],
        )?;
        transaction.commit()?;
        Ok(MediaPlaybackProgress {
            game_uid: game_uid.to_owned(),
            media_key: media_key.to_owned(),
            position_ms,
            duration_ms,
            updated_at,
        })
    }

    pub fn media_transfer(
        &self,
        game_uid: &str,
        media_kind: &str,
    ) -> Result<Option<MediaTransfer>> {
        validate_media_transfer_identity(game_uid, media_kind)?;
        let connection = self.connection()?;
        Ok(connection
            .query_row(
                "SELECT game_uid, launchbox_db_id, title, platform, media_kind,
                        provider, transport, source_url, remote_job_id,
                        source_item_index, source_item_path,
                        local_download_path, local_target_path,
                        state, progress, download_speed, downloaded_bytes,
                        total_bytes, message, created_at, updated_at
                 FROM media_transfers WHERE game_uid=?1 AND media_kind=?2",
                params![game_uid, media_kind],
                media_transfer_from_row,
            )
            .optional()?)
    }

    pub fn media_transfers_for_remote_job(
        &self,
        provider: &str,
        remote_job_id: &str,
    ) -> Result<Vec<MediaTransfer>> {
        if provider.trim().is_empty()
            || provider.chars().count() > 128
            || remote_job_id.trim().is_empty()
            || remote_job_id.chars().count() > 512
        {
            bail!("media transfer requires bounded provider and remote job identities");
        }
        let connection = self.connection()?;
        let mut statement = connection.prepare(
            "SELECT game_uid, launchbox_db_id, title, platform, media_kind,
                    provider, transport, source_url, remote_job_id,
                    source_item_index, source_item_path,
                    local_download_path, local_target_path,
                    state, progress, download_speed, downloaded_bytes,
                    total_bytes, message, created_at, updated_at
             FROM media_transfers
             WHERE provider=?1 AND lower(remote_job_id)=lower(?2)
             ORDER BY created_at, game_uid, media_kind",
        )?;
        let rows =
            statement.query_map(params![provider, remote_job_id], media_transfer_from_row)?;
        Ok(rows.collect::<rusqlite::Result<Vec<_>>>()?)
    }

    pub fn upsert_media_transfer(&self, transfer: &MediaTransfer) -> Result<()> {
        validate_media_transfer(transfer)?;
        let connection = self.connection()?;
        connection.execute(
            "INSERT INTO media_transfers (
                 game_uid, launchbox_db_id, title, platform, media_kind,
                 provider, transport, source_url, remote_job_id,
                 source_item_index, source_item_path,
                 local_download_path, local_target_path,
                 state, progress, download_speed, downloaded_bytes,
                 total_bytes, message, created_at, updated_at
             ) VALUES (
                 ?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10,
                 ?11, ?12, ?13, ?14, ?15, ?16, ?17, ?18, ?19, ?20, ?21
             )
             ON CONFLICT(game_uid, media_kind) DO UPDATE SET
                 launchbox_db_id=excluded.launchbox_db_id,
                 title=excluded.title,
                 platform=excluded.platform,
                 provider=excluded.provider,
                 transport=excluded.transport,
                 source_url=excluded.source_url,
                 remote_job_id=excluded.remote_job_id,
                 source_item_index=excluded.source_item_index,
                 source_item_path=excluded.source_item_path,
                 local_download_path=excluded.local_download_path,
                 local_target_path=excluded.local_target_path,
                 state=excluded.state,
                 progress=excluded.progress,
                 download_speed=excluded.download_speed,
                 downloaded_bytes=excluded.downloaded_bytes,
                 total_bytes=excluded.total_bytes,
                 message=excluded.message,
                 updated_at=excluded.updated_at",
            params![
                transfer.game_uid,
                transfer.launchbox_db_id,
                transfer.title,
                transfer.platform,
                transfer.media_kind,
                transfer.provider,
                transfer.transport,
                transfer.source_url,
                transfer.remote_job_id,
                i64::from(transfer.source_item_index),
                transfer.source_item_path,
                path_text(&transfer.local_download_path),
                path_text(&transfer.local_target_path),
                transfer.state,
                transfer.progress,
                i64::try_from(transfer.download_speed).unwrap_or(i64::MAX),
                i64::try_from(transfer.downloaded_bytes).unwrap_or(i64::MAX),
                i64::try_from(transfer.total_bytes).unwrap_or(i64::MAX),
                transfer.message,
                transfer.created_at,
                transfer.updated_at,
            ],
        )?;
        Ok(())
    }

    pub fn begin_play_session(
        &self,
        game_uid: &str,
        launchbox_db_id: i64,
        title: &str,
        platform: &str,
        emulator: &str,
    ) -> Result<PlaySessionStart> {
        validate_play_identity(game_uid, title, platform)?;
        if emulator.trim().is_empty() {
            bail!("a play session requires an emulator identity");
        }
        let now = unix_timestamp();
        let session_id = uuid::Uuid::new_v4().to_string();
        let mut connection = self.connection()?;
        let transaction = connection.transaction()?;
        transaction.execute(
            "INSERT INTO game_activity (
                 game_uid, launchbox_db_id, title, platform, play_count,
                 total_play_time_seconds, last_played_at, first_played_at,
                 completion_state
             ) VALUES (?1, ?2, ?3, ?4, 1, 0, ?5, ?5, 'in_progress')
             ON CONFLICT(game_uid) DO UPDATE SET
                 launchbox_db_id=excluded.launchbox_db_id,
                 title=excluded.title,
                 platform=excluded.platform,
                 play_count=game_activity.play_count + 1,
                 last_played_at=excluded.last_played_at,
                 first_played_at=CASE
                     WHEN game_activity.play_count=0 OR game_activity.first_played_at=0
                         THEN excluded.first_played_at
                     ELSE game_activity.first_played_at
                 END,
                 completion_state=CASE
                     WHEN game_activity.completion_state='not_started' THEN 'in_progress'
                     ELSE game_activity.completion_state
                 END",
            params![game_uid, launchbox_db_id, title, platform, now],
        )?;
        transaction.execute(
            "INSERT INTO play_sessions (
                 id, game_uid, launchbox_db_id, title, platform, emulator,
                 started_at, ended_at, duration_seconds, outcome
             ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, NULL, 0, 'running')",
            params![
                session_id,
                game_uid,
                launchbox_db_id,
                title,
                platform,
                emulator,
                now
            ],
        )?;
        transaction.commit()?;
        Ok(PlaySessionStart {
            session_id,
            started_at: now,
        })
    }

    pub fn finish_play_session(
        &self,
        session_id: &str,
        duration_seconds: u64,
        outcome: &str,
    ) -> Result<bool> {
        if session_id.trim().is_empty() {
            bail!("a play session identity is required");
        }
        if !matches!(outcome, "completed" | "failed" | "terminated") {
            bail!("unsupported play session outcome {outcome}");
        }
        let duration = i64::try_from(duration_seconds).unwrap_or(i64::MAX);
        let ended_at = unix_timestamp();
        let mut connection = self.connection()?;
        let transaction = connection.transaction()?;
        let game_uid = transaction
            .query_row(
                "SELECT game_uid FROM play_sessions WHERE id=?1 AND outcome='running'",
                [session_id],
                |row| row.get::<_, String>(0),
            )
            .optional()?;
        let Some(game_uid) = game_uid else {
            transaction.rollback()?;
            return Ok(false);
        };
        let changed = transaction.execute(
            "UPDATE play_sessions
             SET ended_at=?2, duration_seconds=?3, outcome=?4
             WHERE id=?1 AND outcome='running'",
            params![session_id, ended_at, duration, outcome],
        )?;
        if changed == 1 {
            transaction.execute(
                "UPDATE game_activity
                 SET total_play_time_seconds=total_play_time_seconds + ?2
                 WHERE game_uid=?1",
                params![game_uid, duration],
            )?;
        }
        transaction.commit()?;
        Ok(changed == 1)
    }

    pub fn set_completion_state(
        &self,
        game_uid: &str,
        launchbox_db_id: i64,
        title: &str,
        platform: &str,
        completion_state: &str,
    ) -> Result<()> {
        validate_play_identity(game_uid, title, platform)?;
        validate_completion_state(completion_state)?;
        let connection = self.connection()?;
        connection.execute(
            "INSERT INTO game_activity (
                 game_uid, launchbox_db_id, title, platform, play_count,
                 total_play_time_seconds, last_played_at, first_played_at,
                 completion_state
             ) VALUES (?1, ?2, ?3, ?4, 0, 0, 0, 0, ?5)
             ON CONFLICT(game_uid) DO UPDATE SET
                 launchbox_db_id=excluded.launchbox_db_id,
                 title=excluded.title,
                 platform=excluded.platform,
                 completion_state=excluded.completion_state",
            params![game_uid, launchbox_db_id, title, platform, completion_state],
        )?;
        Ok(())
    }

    pub fn emulator_preference(
        &self,
        game_uid: &str,
        platform: &str,
    ) -> Result<Option<EmulatorPreference>> {
        let connection = self.connection()?;
        if !game_uid.trim().is_empty()
            && let Some(preference) = connection
                .query_row(
                    "SELECT emulator_id, runtime_kind, core_name
                     FROM game_emulator_preferences WHERE game_uid=?1",
                    [game_uid],
                    |row| {
                        Ok(EmulatorPreference {
                            emulator_id: row.get(0)?,
                            runtime_kind: row.get(1)?,
                            core_name: row.get(2)?,
                            scope: "game".to_owned(),
                        })
                    },
                )
                .optional()?
        {
            return Ok(Some(preference));
        }

        let platform_key = catalog::normalize_platform_key(platform);
        if platform_key.is_empty() {
            return Ok(None);
        }
        Ok(connection
            .query_row(
                "SELECT emulator_id, runtime_kind, core_name
                 FROM platform_emulator_preferences WHERE platform_key=?1",
                [platform_key],
                |row| {
                    Ok(EmulatorPreference {
                        emulator_id: row.get(0)?,
                        runtime_kind: row.get(1)?,
                        core_name: row.get(2)?,
                        scope: "platform".to_owned(),
                    })
                },
            )
            .optional()?)
    }

    pub fn managed_emulator_installs(&self) -> Result<Vec<ManagedEmulatorInstall>> {
        let connection = self.connection()?;
        let mut statement = connection.prepare(
            "SELECT emulator_id, host_system_slug, manager, package_id, install_path,
                    installed_at, updated_at
             FROM managed_emulator_installs
             ORDER BY manager, package_id",
        )?;
        let rows = statement.query_map([], |row| {
            Ok(ManagedEmulatorInstall {
                emulator_id: row.get(0)?,
                host_system_slug: row.get(1)?,
                manager: row.get(2)?,
                package_id: row.get(3)?,
                install_path: row.get(4)?,
                installed_at: row.get(5)?,
                updated_at: row.get(6)?,
            })
        })?;
        Ok(rows.collect::<rusqlite::Result<Vec<_>>>()?)
    }

    pub fn firmware_packages(&self) -> Result<Vec<FirmwarePackageReceipt>> {
        let connection = self.connection()?;
        let mut statement = connection.prepare(
            "SELECT source_id, package_name, archive_path, extracted_root,
                    sha256, file_count, imported_at
             FROM firmware_packages
             ORDER BY source_id, package_name",
        )?;
        let rows = statement.query_map([], |row| {
            Ok(FirmwarePackageReceipt {
                source_id: row.get(0)?,
                package_name: row.get(1)?,
                archive_path: row.get(2)?,
                extracted_root: row.get(3)?,
                sha256: row.get(4)?,
                file_count: row.get(5)?,
                imported_at: row.get(6)?,
            })
        })?;
        Ok(rows.collect::<rusqlite::Result<Vec<_>>>()?)
    }

    pub fn firmware_files(
        &self,
        source_id: &str,
        package_name: &str,
    ) -> Result<Vec<FirmwareFileReceipt>> {
        let connection = self.connection()?;
        let mut statement = connection.prepare(
            "SELECT source_id, package_name, relative_path, store_path,
                    sha256, file_size
             FROM firmware_files
             WHERE source_id=?1 AND package_name=?2
             ORDER BY relative_path",
        )?;
        let rows = statement.query_map(params![source_id, package_name], |row| {
            Ok(FirmwareFileReceipt {
                source_id: row.get(0)?,
                package_name: row.get(1)?,
                relative_path: row.get(2)?,
                store_path: row.get(3)?,
                sha256: row.get(4)?,
                file_size: row.get(5)?,
            })
        })?;
        Ok(rows.collect::<rusqlite::Result<Vec<_>>>()?)
    }

    pub fn firmware_installs(&self) -> Result<Vec<FirmwareInstallReceipt>> {
        let connection = self.connection()?;
        let mut statement = connection.prepare(
            "SELECT rule_key, runtime_path, target_path, source_id,
                    package_name, synced_at
             FROM firmware_installs
             ORDER BY rule_key, runtime_path",
        )?;
        let rows = statement.query_map([], |row| {
            Ok(FirmwareInstallReceipt {
                rule_key: row.get(0)?,
                runtime_path: row.get(1)?,
                target_path: row.get(2)?,
                source_id: row.get(3)?,
                package_name: row.get(4)?,
                synced_at: row.get(5)?,
            })
        })?;
        Ok(rows.collect::<rusqlite::Result<Vec<_>>>()?)
    }

    pub fn record_firmware_package(
        &self,
        receipt: &FirmwarePackageReceipt,
        files: &[FirmwareFileReceipt],
    ) -> Result<()> {
        validate_firmware_package_receipt(receipt)?;
        if usize::try_from(receipt.file_count).ok() != Some(files.len()) {
            bail!("firmware package receipt does not match its file manifest");
        }
        for file in files {
            validate_firmware_file_receipt(receipt, file)?;
        }
        let mut connection = self.connection()?;
        let transaction = connection.transaction()?;
        transaction.execute(
            "INSERT INTO firmware_packages (
                 source_id, package_name, archive_path, extracted_root,
                 sha256, file_count, imported_at
             ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)
             ON CONFLICT(source_id, package_name) DO UPDATE SET
                 archive_path=excluded.archive_path,
                 extracted_root=excluded.extracted_root,
                 sha256=excluded.sha256,
                 file_count=excluded.file_count,
                 imported_at=excluded.imported_at",
            params![
                receipt.source_id,
                receipt.package_name,
                receipt.archive_path,
                receipt.extracted_root,
                receipt.sha256,
                receipt.file_count,
                receipt.imported_at,
            ],
        )?;
        transaction.execute(
            "DELETE FROM firmware_files WHERE source_id=?1 AND package_name=?2",
            params![receipt.source_id, receipt.package_name],
        )?;
        {
            let mut insert = transaction.prepare(
                "INSERT INTO firmware_files (
                     source_id, package_name, relative_path, store_path,
                     sha256, file_size
                 ) VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
            )?;
            for file in files {
                insert.execute(params![
                    file.source_id,
                    file.package_name,
                    file.relative_path,
                    file.store_path,
                    file.sha256,
                    file.file_size,
                ])?;
            }
        }
        transaction.commit()?;
        Ok(())
    }

    pub fn record_firmware_install(&self, receipt: &FirmwareInstallReceipt) -> Result<()> {
        validate_firmware_install_receipt(receipt)?;
        let connection = self.connection()?;
        connection.execute(
            "INSERT INTO firmware_installs (
                 rule_key, runtime_path, target_path, source_id,
                 package_name, synced_at
             ) VALUES (?1, ?2, ?3, ?4, ?5, ?6)
             ON CONFLICT(rule_key, runtime_path) DO UPDATE SET
                 target_path=excluded.target_path,
                 source_id=excluded.source_id,
                 package_name=excluded.package_name,
                 synced_at=excluded.synced_at",
            params![
                receipt.rule_key,
                receipt.runtime_path,
                receipt.target_path,
                receipt.source_id,
                receipt.package_name,
                receipt.synced_at,
            ],
        )?;
        Ok(())
    }

    pub fn firmware_downloads_for_info_hash(
        &self,
        info_hash: &str,
    ) -> Result<Vec<FirmwareDownloadReceipt>> {
        let connection = self.connection()?;
        let mut statement = connection.prepare(
            "SELECT source_id, package_name, torrent_url, info_hash,
                    file_index, file_path, local_path, state, progress,
                    message, updated_at
             FROM firmware_downloads
             WHERE info_hash=?1
             ORDER BY source_id, package_name",
        )?;
        let rows = statement.query_map([info_hash], firmware_download_from_row)?;
        Ok(rows.collect::<rusqlite::Result<Vec<_>>>()?)
    }

    pub fn record_firmware_download(&self, receipt: &FirmwareDownloadReceipt) -> Result<()> {
        validate_firmware_download_receipt(receipt)?;
        let connection = self.connection()?;
        connection.execute(
            "INSERT INTO firmware_downloads (
                 source_id, package_name, torrent_url, info_hash, file_index,
                 file_path, local_path, state, progress, message, updated_at
             ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11)
             ON CONFLICT(source_id, package_name) DO UPDATE SET
                 torrent_url=excluded.torrent_url,
                 info_hash=excluded.info_hash,
                 file_index=excluded.file_index,
                 file_path=excluded.file_path,
                 local_path=excluded.local_path,
                 state=excluded.state,
                 progress=excluded.progress,
                 message=excluded.message,
                 updated_at=excluded.updated_at",
            params![
                receipt.source_id,
                receipt.package_name,
                receipt.torrent_url,
                receipt.info_hash,
                i64::from(receipt.file_index),
                receipt.file_path,
                receipt.local_path,
                receipt.state,
                receipt.progress,
                receipt.message,
                receipt.updated_at,
            ],
        )?;
        Ok(())
    }

    pub fn record_managed_emulator_install(
        &self,
        emulator_id: &str,
        host_system_slug: &str,
        manager: &str,
        package_id: &str,
        install_path: &str,
    ) -> Result<()> {
        validate_managed_emulator_install(emulator_id, host_system_slug, manager, package_id)?;
        let now = unix_timestamp();
        let connection = self.connection()?;
        connection.execute(
            "INSERT INTO managed_emulator_installs (
                 emulator_id, host_system_slug, manager, package_id, install_path,
                 installed_at, updated_at
             ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?6)
             ON CONFLICT(host_system_slug, manager, package_id) DO UPDATE SET
                 emulator_id=excluded.emulator_id,
                 install_path=excluded.install_path,
                 updated_at=excluded.updated_at",
            params![
                emulator_id,
                host_system_slug,
                manager,
                package_id,
                install_path,
                now
            ],
        )?;
        Ok(())
    }

    pub fn remove_managed_emulator_install(
        &self,
        host_system_slug: &str,
        manager: &str,
        package_id: &str,
    ) -> Result<()> {
        let connection = self.connection()?;
        connection.execute(
            "DELETE FROM managed_emulator_installs
             WHERE host_system_slug=?1 AND manager=?2 AND package_id=?3",
            params![host_system_slug, manager, package_id],
        )?;
        Ok(())
    }

    pub fn set_game_emulator_preference(
        &self,
        game_uid: &str,
        emulator_id: &str,
        runtime_kind: &str,
        core_name: &str,
    ) -> Result<()> {
        validate_emulator_preference(game_uid, emulator_id, runtime_kind, core_name)?;
        let connection = self.connection()?;
        connection.execute(
            "INSERT INTO game_emulator_preferences (
                 game_uid, emulator_id, runtime_kind, core_name, updated_at
             ) VALUES (?1, ?2, ?3, ?4, ?5)
             ON CONFLICT(game_uid) DO UPDATE SET
                 emulator_id=excluded.emulator_id,
                 runtime_kind=excluded.runtime_kind,
                 core_name=excluded.core_name,
                 updated_at=excluded.updated_at",
            params![
                game_uid,
                emulator_id,
                runtime_kind,
                core_name,
                unix_timestamp()
            ],
        )?;
        Ok(())
    }

    pub fn clear_game_emulator_preference(&self, game_uid: &str) -> Result<()> {
        if game_uid.trim().is_empty() {
            bail!("a stable game identity is required to clear an emulator preference");
        }
        self.connection()?.execute(
            "DELETE FROM game_emulator_preferences WHERE game_uid=?1",
            [game_uid],
        )?;
        Ok(())
    }

    pub fn set_platform_emulator_preference(
        &self,
        platform: &str,
        emulator_id: &str,
        runtime_kind: &str,
        core_name: &str,
    ) -> Result<()> {
        let platform_key = catalog::normalize_platform_key(platform);
        validate_emulator_preference(&platform_key, emulator_id, runtime_kind, core_name)?;
        let connection = self.connection()?;
        connection.execute(
            "INSERT INTO platform_emulator_preferences (
                 platform_key, platform_display, emulator_id,
                 runtime_kind, core_name, updated_at
             ) VALUES (?1, ?2, ?3, ?4, ?5, ?6)
             ON CONFLICT(platform_key) DO UPDATE SET
                 platform_display=excluded.platform_display,
                 emulator_id=excluded.emulator_id,
                 runtime_kind=excluded.runtime_kind,
                 core_name=excluded.core_name,
                 updated_at=excluded.updated_at",
            params![
                platform_key,
                platform.trim(),
                emulator_id,
                runtime_kind,
                core_name,
                unix_timestamp()
            ],
        )?;
        Ok(())
    }

    pub fn clear_platform_emulator_preference(&self, platform: &str) -> Result<()> {
        let platform_key = catalog::normalize_platform_key(platform);
        if platform_key.is_empty() {
            bail!("a platform is required to clear an emulator preference");
        }
        self.connection()?.execute(
            "DELETE FROM platform_emulator_preferences WHERE platform_key=?1",
            [platform_key],
        )?;
        Ok(())
    }

    pub fn emulator_launch_profile(
        &self,
        scope_kind: &str,
        scope_key: &str,
        emulator_id: &str,
        runtime_kind: &str,
        core_name: &str,
    ) -> Result<Option<EmulatorLaunchProfile>> {
        let scope_key = normalized_launch_scope_key(scope_kind, scope_key)?;
        validate_emulator_preference("launch-profile", emulator_id, runtime_kind, core_name)?;
        let connection = self.connection()?;
        connection
            .query_row(
                "SELECT scope_kind, scope_key, emulator_id, runtime_kind, core_name,
                        extra_arguments, command_template, updated_at
                 FROM emulator_launch_profiles
                 WHERE scope_kind=?1 AND scope_key=?2 AND emulator_id=?3
                   AND runtime_kind=?4 AND core_name=?5",
                params![scope_kind, scope_key, emulator_id, runtime_kind, core_name],
                |row| {
                    Ok(EmulatorLaunchProfile {
                        scope_kind: row.get(0)?,
                        scope_key: row.get(1)?,
                        emulator_id: row.get(2)?,
                        runtime_kind: row.get(3)?,
                        core_name: row.get(4)?,
                        extra_arguments: row.get(5)?,
                        command_template: row.get(6)?,
                        updated_at: row.get(7)?,
                    })
                },
            )
            .optional()
            .context("loading an emulator launch profile")
    }

    pub fn emulator_launch_profiles(&self) -> Result<Vec<EmulatorLaunchProfile>> {
        let connection = self.connection()?;
        let mut statement = connection.prepare(
            "SELECT scope_kind, scope_key, emulator_id, runtime_kind, core_name,
                    extra_arguments, command_template, updated_at
             FROM emulator_launch_profiles
             ORDER BY scope_kind, scope_key, emulator_id, runtime_kind, core_name",
        )?;
        statement
            .query_map([], |row| {
                Ok(EmulatorLaunchProfile {
                    scope_kind: row.get(0)?,
                    scope_key: row.get(1)?,
                    emulator_id: row.get(2)?,
                    runtime_kind: row.get(3)?,
                    core_name: row.get(4)?,
                    extra_arguments: row.get(5)?,
                    command_template: row.get(6)?,
                    updated_at: row.get(7)?,
                })
            })?
            .collect::<rusqlite::Result<Vec<_>>>()
            .context("listing emulator launch profiles")
    }

    pub fn resolve_launch_customization(
        &self,
        game_uid: &str,
        platform: &str,
        emulator_id: &str,
        runtime_kind: &str,
        core_name: &str,
    ) -> Result<ResolvedLaunchCustomization> {
        validate_emulator_preference("launch-profile", emulator_id, runtime_kind, core_name)?;
        let platform_key = catalog::normalize_platform_key(platform);
        let mut profiles = Vec::new();
        if !game_uid.trim().is_empty()
            && let Some(profile) = self.emulator_launch_profile(
                "game",
                game_uid,
                emulator_id,
                runtime_kind,
                core_name,
            )?
        {
            profiles.push(profile);
        }
        if !platform_key.is_empty()
            && let Some(profile) = self.emulator_launch_profile(
                "platform",
                &platform_key,
                emulator_id,
                runtime_kind,
                core_name,
            )?
        {
            profiles.push(profile);
        }
        if let Some(profile) =
            self.emulator_launch_profile("global", "", emulator_id, runtime_kind, core_name)?
        {
            profiles.push(profile);
        }

        let mut resolved = ResolvedLaunchCustomization::default();
        for profile in profiles {
            if resolved.extra_arguments.is_empty() && !profile.extra_arguments.is_empty() {
                resolved.extra_arguments = profile.extra_arguments;
                resolved.argument_scope = profile.scope_kind.clone();
            }
            if resolved.command_template.is_empty() && !profile.command_template.is_empty() {
                resolved.command_template = profile.command_template;
                resolved.template_scope = profile.scope_kind;
            }
            if !resolved.extra_arguments.is_empty() && !resolved.command_template.is_empty() {
                break;
            }
        }
        Ok(resolved)
    }

    pub fn set_emulator_launch_profile(&self, profile: &EmulatorLaunchProfile) -> Result<()> {
        let scope_key = normalized_launch_scope_key(&profile.scope_kind, &profile.scope_key)?;
        validate_emulator_preference(
            "launch-profile",
            &profile.emulator_id,
            &profile.runtime_kind,
            &profile.core_name,
        )?;
        let extra_arguments = profile.extra_arguments.trim();
        let command_template = profile.command_template.trim();
        crate::emulator::validate_launch_extra_arguments(extra_arguments)?;
        crate::emulator::validate_launch_template(command_template)?;
        if extra_arguments.is_empty() && command_template.is_empty() {
            return self.clear_emulator_launch_profile(
                &profile.scope_kind,
                &scope_key,
                &profile.emulator_id,
                &profile.runtime_kind,
                &profile.core_name,
            );
        }
        let connection = self.connection()?;
        connection.execute(
            "INSERT INTO emulator_launch_profiles (
                 scope_kind, scope_key, emulator_id, runtime_kind, core_name,
                 extra_arguments, command_template, updated_at
             ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)
             ON CONFLICT(scope_kind, scope_key, emulator_id, runtime_kind, core_name)
             DO UPDATE SET
                 extra_arguments=excluded.extra_arguments,
                 command_template=excluded.command_template,
                 updated_at=excluded.updated_at",
            params![
                profile.scope_kind,
                scope_key,
                profile.emulator_id,
                profile.runtime_kind,
                profile.core_name,
                extra_arguments,
                command_template,
                unix_timestamp()
            ],
        )?;
        Ok(())
    }

    pub fn clear_emulator_launch_profile(
        &self,
        scope_kind: &str,
        scope_key: &str,
        emulator_id: &str,
        runtime_kind: &str,
        core_name: &str,
    ) -> Result<()> {
        let scope_key = normalized_launch_scope_key(scope_kind, scope_key)?;
        validate_emulator_preference("launch-profile", emulator_id, runtime_kind, core_name)?;
        self.connection()?.execute(
            "DELETE FROM emulator_launch_profiles
             WHERE scope_kind=?1 AND scope_key=?2 AND emulator_id=?3
               AND runtime_kind=?4 AND core_name=?5",
            params![scope_kind, scope_key, emulator_id, runtime_kind, core_name],
        )?;
        Ok(())
    }

    pub fn user_collections(&self) -> Result<UserCollections> {
        let connection = self.connection()?;
        let mut collection_statement = connection.prepare(
            "SELECT id, name, description, kind, rules_json
             FROM user_collections
             ORDER BY name COLLATE NOCASE, id",
        )?;
        let collection_rows = collection_statement.query_map([], |row| {
            Ok((
                row.get::<_, String>(0)?,
                row.get::<_, String>(1)?,
                row.get::<_, String>(2)?,
                row.get::<_, String>(3)?,
                row.get::<_, String>(4)?,
            ))
        })?;
        let mut collections = Vec::new();
        for row in collection_rows {
            let (id, name, description, kind, rules_json) = row?;
            let rules = match kind.as_str() {
                "manual" if rules_json.is_empty() => None,
                "smart" => Some(
                    serde_json::from_str::<SmartCollectionRules>(&rules_json)
                        .context("decoding smart collection rules")?
                        .normalized()?,
                ),
                _ => bail!("collection {id} has an invalid kind or rule payload"),
            };
            collections.push(UserCollection {
                id,
                name,
                description,
                kind,
                rules,
                game_count: 0,
            });
        }

        let mut member_statement = connection.prepare(
            "SELECT collection_id, game_uid
             FROM user_collection_games
             ORDER BY collection_id, sort_order, game_uid",
        )?;
        let member_rows = member_statement.query_map([], |row| {
            Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?))
        })?;
        let mut members = HashMap::<String, Vec<String>>::new();
        for row in member_rows {
            let (collection_id, game_uid) = row?;
            members.entry(collection_id).or_default().push(game_uid);
        }
        for collection in &mut collections {
            collection.game_count = members
                .get(&collection.id)
                .map(Vec::len)
                .unwrap_or_default();
        }
        Ok(UserCollections {
            collections,
            members,
        })
    }

    pub fn create_collection(&self, name: &str, description: &str) -> Result<UserCollection> {
        self.create_collection_record(name, description, "manual", None)
    }

    pub fn create_smart_collection(
        &self,
        name: &str,
        description: &str,
        rules: SmartCollectionRules,
    ) -> Result<UserCollection> {
        self.create_collection_record(name, description, "smart", Some(rules.normalized()?))
    }

    fn create_collection_record(
        &self,
        name: &str,
        description: &str,
        kind: &str,
        rules: Option<SmartCollectionRules>,
    ) -> Result<UserCollection> {
        let name = validate_collection_name(name)?;
        let description = validate_collection_description(description)?;
        if !matches!((kind, &rules), ("manual", None) | ("smart", Some(_))) {
            bail!("collections require a consistent kind and rule payload");
        }
        let rules_json = rules
            .as_ref()
            .map(serde_json::to_string)
            .transpose()?
            .unwrap_or_default();
        let collection = UserCollection {
            id: uuid::Uuid::new_v4().to_string(),
            name,
            description,
            kind: kind.to_owned(),
            rules,
            game_count: 0,
        };
        let connection = self.connection()?;
        connection
            .execute(
                "INSERT INTO user_collections (
                     id, name, description, kind, rules_json, created_at, updated_at
                 ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?6)",
                params![
                    collection.id,
                    collection.name,
                    collection.description,
                    collection.kind,
                    rules_json,
                    unix_timestamp()
                ],
            )
            .with_context(|| format!("creating collection {}", collection.name))?;
        Ok(collection)
    }

    pub fn update_collection(&self, id: &str, name: &str, description: &str) -> Result<()> {
        let name = validate_collection_name(name)?;
        let description = validate_collection_description(description)?;
        let connection = self.connection()?;
        let changed = connection
            .execute(
                "UPDATE user_collections
                 SET name=?2, description=?3, updated_at=?4
                 WHERE id=?1",
                params![id, name, description, unix_timestamp()],
            )
            .with_context(|| format!("updating collection {id}"))?;
        if changed == 0 {
            bail!("collection no longer exists");
        }
        Ok(())
    }

    pub fn update_smart_collection(
        &self,
        id: &str,
        name: &str,
        description: &str,
        rules: SmartCollectionRules,
    ) -> Result<()> {
        let name = validate_collection_name(name)?;
        let description = validate_collection_description(description)?;
        let rules_json = serde_json::to_string(&rules.normalized()?)?;
        let connection = self.connection()?;
        let changed = connection.execute(
            "UPDATE user_collections
             SET name=?2, description=?3, rules_json=?4, updated_at=?5
             WHERE id=?1 AND kind='smart'",
            params![id, name, description, rules_json, unix_timestamp()],
        )?;
        if changed == 0 {
            bail!("smart collection no longer exists");
        }
        Ok(())
    }

    pub fn delete_collection(&self, id: &str) -> Result<()> {
        let connection = self.connection()?;
        let changed = connection
            .execute("DELETE FROM user_collections WHERE id=?1", params![id])
            .with_context(|| format!("deleting collection {id}"))?;
        if changed == 0 {
            bail!("collection no longer exists");
        }
        Ok(())
    }

    pub fn set_collection_membership(
        &self,
        collection_id: &str,
        game_uid: &str,
        member: bool,
    ) -> Result<()> {
        self.set_collection_memberships(collection_id, &[game_uid.to_owned()], member)?;
        Ok(())
    }

    pub fn set_collection_memberships(
        &self,
        collection_id: &str,
        game_uids: &[String],
        member: bool,
    ) -> Result<usize> {
        let game_uids = validated_collection_game_uids(game_uids)?;
        let mut connection = self.connection()?;
        let transaction = connection.transaction()?;
        let kind: String = transaction
            .query_row(
                "SELECT kind FROM user_collections WHERE id=?1",
                [collection_id],
                |row| row.get(0),
            )
            .optional()?
            .context("collection no longer exists")?;
        if kind != "manual" {
            bail!("smart collection membership is derived from its rules");
        }
        let mut changed = 0_usize;
        if member {
            let mut next_order = transaction.query_row(
                "SELECT coalesce(max(sort_order), 0) + 1
                 FROM user_collection_games WHERE collection_id=?1",
                [collection_id],
                |row| row.get::<_, i64>(0),
            )?;
            for game_uid in game_uids {
                let inserted = transaction.execute(
                    "INSERT OR IGNORE INTO user_collection_games (
                         collection_id, game_uid, sort_order, added_at
                     ) VALUES (?1, ?2, ?3, ?4)",
                    params![collection_id, game_uid, next_order, unix_timestamp()],
                )?;
                if inserted > 0 {
                    next_order = next_order.saturating_add(1);
                    changed = changed.saturating_add(1);
                }
            }
        } else {
            for game_uid in game_uids {
                changed = changed.saturating_add(transaction.execute(
                    "DELETE FROM user_collection_games
                     WHERE collection_id=?1 AND game_uid=?2",
                    params![collection_id, game_uid],
                )?);
            }
        }
        transaction.commit()?;
        Ok(changed)
    }

    pub fn replace_collection_members(
        &self,
        collection_id: &str,
        game_uids: &[String],
    ) -> Result<()> {
        let game_uids = validated_collection_game_uids(game_uids)?;
        let mut connection = self.connection()?;
        let transaction = connection.transaction()?;
        let kind: String = transaction
            .query_row(
                "SELECT kind FROM user_collections WHERE id=?1",
                [collection_id],
                |row| row.get(0),
            )
            .optional()?
            .context("collection no longer exists")?;
        if kind != "manual" {
            bail!("smart collection membership cannot be replaced");
        }
        transaction.execute(
            "DELETE FROM user_collection_games WHERE collection_id=?1",
            [collection_id],
        )?;
        for (index, game_uid) in game_uids.iter().enumerate() {
            transaction.execute(
                "INSERT INTO user_collection_games (
                     collection_id, game_uid, sort_order, added_at
                 ) VALUES (?1, ?2, ?3, ?4)",
                params![
                    collection_id,
                    game_uid,
                    i64::try_from(index).unwrap_or(i64::MAX),
                    unix_timestamp()
                ],
            )?;
        }
        transaction.commit()?;
        Ok(())
    }

    pub fn move_collection_game(
        &self,
        collection_id: &str,
        game_uid: &str,
        new_index: usize,
    ) -> Result<Vec<String>> {
        if game_uid.trim().is_empty() {
            bail!("a stable game identity is required to reorder a collection");
        }
        let collections = self.user_collections()?;
        let mut members = collections
            .members
            .get(collection_id)
            .cloned()
            .context("collection has no members")?;
        let old_index = members
            .iter()
            .position(|candidate| candidate == game_uid)
            .context("game is not in the collection")?;
        if new_index >= members.len() {
            bail!("collection order target is outside the member list");
        }
        if old_index != new_index {
            let game_uid = members.remove(old_index);
            members.insert(new_index, game_uid);
            self.replace_collection_members(collection_id, &members)?;
        }
        Ok(members)
    }

    pub fn upsert_job(&self, job: &DownloadJob) -> Result<()> {
        validate_download_source_kind(&job.source_kind)?;
        let connection = self.connection()?;
        connection.execute(
            "INSERT INTO download_jobs (
                 id, game_id, launchbox_db_id, title, platform, source_kind,
                 torrent_url, torrent_file_index,
                 torrent_file_path, info_hash, client_save_path,
                 local_download_path, local_target_path, state, progress,
                 download_speed, downloaded_bytes, total_bytes, message,
                 post_import_action, created_at, updated_at, download_plan
             ) VALUES (
                 ?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12,
                 ?13, ?14, ?15, ?16, ?17, ?18, ?19, ?20, ?21, ?22, ?23
             ) ON CONFLICT(id) DO UPDATE SET
                 state=excluded.state, progress=excluded.progress,
                 download_speed=excluded.download_speed,
                 downloaded_bytes=excluded.downloaded_bytes,
                 total_bytes=excluded.total_bytes, message=excluded.message,
                 post_import_action=excluded.post_import_action,
                 updated_at=excluded.updated_at",
            params![
                job.id,
                job.game_id,
                job.launchbox_db_id,
                job.title,
                job.platform,
                job.source_kind,
                job.torrent_url,
                job.torrent_file_index.map(i64::from),
                job.torrent_file_path,
                job.info_hash,
                job.client_save_path,
                path_text(&job.local_download_path),
                path_text(&job.local_target_path),
                job.state,
                job.progress,
                u64_to_i64(job.download_speed),
                u64_to_i64(job.downloaded_bytes),
                u64_to_i64(job.total_bytes),
                job.message,
                job.post_import_action,
                job.created_at,
                job.updated_at,
                job.download_plan,
            ],
        )?;
        Ok(())
    }

    pub(crate) fn record_manual_torrent_enqueue(
        &self,
        receipt: &ManualTorrentSourceReceipt,
        job: &DownloadJob,
    ) -> Result<()> {
        validate_manual_torrent_receipt(receipt, job)?;
        self.upsert_job(job)?;
        self.connection()?.execute(
            "INSERT INTO manual_torrent_sources (
                 game_uid, launchbox_db_id, canonical_title, platform,
                 source_file_name, torrent_name, torrent_sha256, info_hash,
                 selected_file_index, selected_file_path, queued_job_id, reviewed_at
             ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12)
             ON CONFLICT(game_uid, info_hash, selected_file_index) DO UPDATE SET
                 launchbox_db_id=excluded.launchbox_db_id,
                 canonical_title=excluded.canonical_title,
                 platform=excluded.platform,
                 source_file_name=excluded.source_file_name,
                 torrent_name=excluded.torrent_name,
                 torrent_sha256=excluded.torrent_sha256,
                 selected_file_path=excluded.selected_file_path,
                 queued_job_id=excluded.queued_job_id,
                 reviewed_at=excluded.reviewed_at",
            params![
                receipt.game_uid,
                receipt.launchbox_db_id,
                receipt.canonical_title,
                receipt.platform,
                receipt.source_file_name,
                receipt.torrent_name,
                receipt.torrent_sha256,
                receipt.info_hash,
                i64::from(receipt.selected_file_index),
                receipt.selected_file_path,
                receipt.queued_job_id,
                receipt.reviewed_at,
            ],
        )?;
        Ok(())
    }

    #[cfg(test)]
    pub(crate) fn manual_torrent_sources(&self) -> Result<Vec<ManualTorrentSourceReceipt>> {
        let connection = self.connection()?;
        let mut statement = connection.prepare(
            "SELECT game_uid, launchbox_db_id, canonical_title, platform,
                    source_file_name, torrent_name, torrent_sha256, info_hash,
                    selected_file_index, selected_file_path, queued_job_id, reviewed_at
             FROM manual_torrent_sources
             ORDER BY reviewed_at, game_uid, info_hash, selected_file_index",
        )?;
        let rows = statement.query_map([], |row| {
            let selected_file_index = row.get::<_, i64>(8)?;
            Ok(ManualTorrentSourceReceipt {
                game_uid: row.get(0)?,
                launchbox_db_id: row.get(1)?,
                canonical_title: row.get(2)?,
                platform: row.get(3)?,
                source_file_name: row.get(4)?,
                torrent_name: row.get(5)?,
                torrent_sha256: row.get(6)?,
                info_hash: row.get(7)?,
                selected_file_index: u32::try_from(selected_file_index).unwrap_or(u32::MAX),
                selected_file_path: row.get(9)?,
                queued_job_id: row.get(10)?,
                reviewed_at: row.get(11)?,
            })
        })?;
        Ok(rows.collect::<rusqlite::Result<Vec<_>>>()?)
    }

    pub fn jobs(&self) -> Result<Vec<DownloadJob>> {
        let connection = self.connection()?;
        let mut statement = connection.prepare(
            "SELECT id, game_id, launchbox_db_id, title, platform, source_kind, torrent_url,
                    torrent_file_index, torrent_file_path, info_hash,
                    client_save_path, local_download_path, local_target_path,
                    state, progress, download_speed, downloaded_bytes,
                    total_bytes, message, post_import_action, created_at, updated_at,
                    download_plan
             FROM download_jobs ORDER BY created_at DESC, id",
        )?;
        let rows = statement.query_map([], |row| {
            let file_index = row.get::<_, Option<i64>>(7)?;
            Ok(DownloadJob {
                id: row.get(0)?,
                game_id: row.get(1)?,
                launchbox_db_id: row.get(2)?,
                title: row.get(3)?,
                platform: row.get(4)?,
                source_kind: row.get(5)?,
                torrent_url: row.get(6)?,
                torrent_file_index: file_index.and_then(|value| u32::try_from(value).ok()),
                torrent_file_path: row.get(8)?,
                info_hash: row.get(9)?,
                client_save_path: row.get(10)?,
                local_download_path: PathBuf::from(row.get::<_, String>(11)?),
                local_target_path: PathBuf::from(row.get::<_, String>(12)?),
                state: row.get(13)?,
                progress: row.get(14)?,
                download_speed: i64_to_u64(row.get(15)?),
                downloaded_bytes: i64_to_u64(row.get(16)?),
                total_bytes: i64_to_u64(row.get(17)?),
                message: row.get(18)?,
                post_import_action: row.get(19)?,
                created_at: row.get(20)?,
                updated_at: row.get(21)?,
                download_plan: row.get(22)?,
            })
        })?;
        Ok(rows.collect::<rusqlite::Result<Vec<_>>>()?)
    }

    pub fn jobs_for_info_hash(&self, info_hash: &str) -> Result<Vec<DownloadJob>> {
        Ok(self
            .jobs()?
            .into_iter()
            .filter(|job| job.info_hash.eq_ignore_ascii_case(info_hash))
            .collect())
    }

    pub fn delete_job_record(&self, id: &str) -> Result<bool> {
        let connection = self.connection()?;
        let changed = connection.execute(
            "DELETE FROM download_jobs
             WHERE id=?1
               AND state NOT IN ('queued', 'downloading', 'paused', 'complete')
               AND post_import_action != 'pause_pending'",
            params![id],
        )?;
        Ok(changed > 0)
    }

    pub fn clear_finished_job_records(&self) -> Result<usize> {
        let connection = self.connection()?;
        Ok(connection.execute(
            "DELETE FROM download_jobs
             WHERE state NOT IN ('queued', 'downloading', 'paused', 'complete')
               AND post_import_action != 'pause_pending'",
            [],
        )?)
    }

    pub fn record_installed(&self, job: &DownloadJob, path: &Path) -> Result<()> {
        let connection = self.connection()?;
        connection.execute(
            "INSERT INTO installed_games (
                 game_uid, launchbox_db_id, title, platform, file_path,
                 file_size, import_source, installed_at
             ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)
             ON CONFLICT(game_uid) DO UPDATE SET
                 launchbox_db_id=excluded.launchbox_db_id,
                 title=excluded.title, platform=excluded.platform,
                 file_path=excluded.file_path, file_size=excluded.file_size,
                 import_source=excluded.import_source,
                 installed_at=excluded.installed_at",
            params![
                job.game_id,
                job.launchbox_db_id,
                job.title,
                job.platform,
                path_text(path),
                path.metadata()
                    .ok()
                    .and_then(|metadata| i64::try_from(metadata.len()).ok()),
                job.source_kind,
                unix_timestamp(),
            ],
        )?;
        Ok(())
    }

    pub(crate) fn connection(&self) -> Result<Connection> {
        let connection = Connection::open(&self.path)
            .with_context(|| format!("opening Lunchbox state {}", self.path.display()))?;
        connection.busy_timeout(std::time::Duration::from_secs(2))?;
        connection.execute_batch(
            "PRAGMA foreign_keys=ON;
             PRAGMA synchronous=NORMAL;
             PRAGMA trusted_schema=OFF;",
        )?;
        Ok(connection)
    }
}

pub fn state_database_path() -> Result<PathBuf> {
    if let Some(path) = catalog::requested_path("--state-database", "LUNCHBOX_STATE_DATABASE")
        .filter(|path| !path.as_os_str().is_empty())
    {
        return Ok(path);
    }
    ProjectDirs::from("com", "Lunchbox", "Lunchbox")
        .map(|dirs| dirs.data_local_dir().join("state.db"))
        .context("could not determine the operating system application data directory")
}

pub fn load_password() -> Result<Option<String>> {
    load_secret(QBITTORRENT_KEYRING_ACCOUNT, "qBittorrent password")
}

pub fn save_password(password: &str) -> Result<()> {
    save_secret(
        QBITTORRENT_KEYRING_ACCOUNT,
        password,
        "qBittorrent password",
    )
}

pub fn load_steamgriddb_api_key() -> Result<Option<String>> {
    load_secret(STEAMGRIDDB_KEYRING_ACCOUNT, "SteamGridDB API key")
}

pub fn save_steamgriddb_api_key(api_key: &str) -> Result<()> {
    save_secret(STEAMGRIDDB_KEYRING_ACCOUNT, api_key, "SteamGridDB API key")
}

#[derive(Deserialize, Serialize)]
struct IgdbCredentials {
    client_id: String,
    client_secret: String,
}

pub fn load_igdb_credentials() -> Result<Option<(String, String)>> {
    let Some(encoded) = load_secret(IGDB_KEYRING_ACCOUNT, "IGDB Twitch credentials")? else {
        return Ok(None);
    };
    let credentials: IgdbCredentials =
        serde_json::from_str(&encoded).context("decoding IGDB Twitch credentials")?;
    if credentials.client_id.trim().is_empty() || credentials.client_secret.trim().is_empty() {
        bail!("saved IGDB Twitch credentials are incomplete");
    }
    Ok(Some((credentials.client_id, credentials.client_secret)))
}

pub fn save_igdb_credentials(client_id: &str, client_secret: &str) -> Result<()> {
    let client_id = client_id.trim();
    let client_secret = client_secret.trim();
    if client_id.is_empty() && client_secret.is_empty() {
        return save_secret(IGDB_KEYRING_ACCOUNT, "", "IGDB Twitch credentials");
    }
    if client_id.is_empty() || client_secret.is_empty() {
        bail!("both the IGDB Twitch client ID and client secret are required");
    }
    let encoded = serde_json::to_string(&IgdbCredentials {
        client_id: client_id.to_owned(),
        client_secret: client_secret.to_owned(),
    })
    .context("encoding IGDB Twitch credentials")?;
    save_secret(IGDB_KEYRING_ACCOUNT, &encoded, "IGDB Twitch credentials")
}

fn load_secret(account: &str, label: &str) -> Result<Option<String>> {
    let entry = keyring::Entry::new(KEYRING_SERVICE, account)
        .context("opening the operating system credential store")?;
    match entry.get_password() {
        Ok(secret) => Ok(Some(secret)),
        Err(keyring::Error::NoEntry) => Ok(None),
        Err(error) => Err(error).with_context(|| format!("reading {label} from credential store")),
    }
}

fn save_secret(account: &str, secret: &str, label: &str) -> Result<()> {
    let entry = keyring::Entry::new(KEYRING_SERVICE, account)
        .context("opening the operating system credential store")?;
    if secret.is_empty() {
        match entry.delete_credential() {
            Ok(()) | Err(keyring::Error::NoEntry) => Ok(()),
            Err(error) => Err(error).with_context(|| format!("removing {label}")),
        }
    } else {
        entry
            .set_password(secret)
            .with_context(|| format!("saving {label} in credential store"))
    }
}

fn migrate(connection: &Connection) -> Result<()> {
    connection.execute_batch(
        "CREATE TABLE IF NOT EXISTS app_settings (
             id INTEGER PRIMARY KEY CHECK (id=1),
             qbittorrent_host TEXT NOT NULL,
             qbittorrent_port INTEGER NOT NULL CHECK (qbittorrent_port BETWEEN 1 AND 65535),
             qbittorrent_use_https INTEGER NOT NULL CHECK (qbittorrent_use_https IN (0, 1)),
             qbittorrent_username TEXT NOT NULL,
             rom_directory TEXT NOT NULL,
             qbittorrent_container_rom_directory TEXT NOT NULL,
             torrent_library_directory TEXT NOT NULL,
             qbittorrent_container_torrent_library_directory TEXT NOT NULL,
             download_entire_torrent INTEGER NOT NULL CHECK (download_entire_torrent IN (0, 1)),
             file_link_mode TEXT NOT NULL CHECK (
                 file_link_mode IN ('symlink', 'hardlink', 'reflink', 'copy', 'leave_in_place')
             ),
             seeding_policy TEXT NOT NULL DEFAULT 'follow_client' CHECK (
                 seeding_policy IN ('follow_client', 'pause_after_import')
             ),
             preferred_region TEXT NOT NULL DEFAULT 'USA' CHECK (
                 preferred_region IN ('USA', 'Japan', 'Europe', 'World', 'Asia', 'any')
             ),
             version_preference TEXT NOT NULL DEFAULT 'latest' CHECK (
                 version_preference IN ('latest', 'original')
             ),
             region_priority_json TEXT NOT NULL DEFAULT '[]',
             controller_mapping_json TEXT NOT NULL DEFAULT '{}',
             media_provider_priority_json TEXT NOT NULL DEFAULT '[]'
         );
         CREATE TABLE IF NOT EXISTS library_preferences (
             id INTEGER PRIMARY KEY CHECK (id=1),
             hide_non_retail INTEGER NOT NULL CHECK (hide_non_retail IN (0, 1)),
             hide_adult INTEGER NOT NULL CHECK (hide_adult IN (0, 1)),
             artwork_type TEXT NOT NULL DEFAULT 'box-front' CHECK (
                 artwork_type IN (
                     'box-front', 'box-back', 'box-3d', 'screenshot',
                     'title-screen', 'fanart', 'clear-logo'
                 )
             ),
             grid_zoom INTEGER NOT NULL DEFAULT 100 CHECK (grid_zoom BETWEEN 50 AND 200)
         );
         CREATE TABLE IF NOT EXISTS sidebar_preferences (
             id INTEGER PRIMARY KEY CHECK (id=1),
             platform_search TEXT NOT NULL DEFAULT '' CHECK (length(platform_search) <= 80),
             width INTEGER NOT NULL DEFAULT 254 CHECK (width BETWEEN 180 AND 400)
         );
         CREATE TABLE IF NOT EXISTS download_jobs (
             id TEXT PRIMARY KEY,
             game_id TEXT NOT NULL,
             launchbox_db_id INTEGER NOT NULL DEFAULT 0,
             title TEXT NOT NULL,
             platform TEXT NOT NULL,
             source_kind TEXT NOT NULL DEFAULT 'minerva' CHECK (
                 source_kind IN ('minerva', 'manual_torrent')
             ),
             torrent_url TEXT NOT NULL,
             torrent_file_index INTEGER,
             torrent_file_path TEXT NOT NULL,
             info_hash TEXT NOT NULL,
             client_save_path TEXT NOT NULL,
             local_download_path TEXT NOT NULL,
             local_target_path TEXT NOT NULL,
             state TEXT NOT NULL,
             progress REAL NOT NULL CHECK (progress BETWEEN 0.0 AND 1.0),
             download_speed INTEGER NOT NULL,
             downloaded_bytes INTEGER NOT NULL,
             total_bytes INTEGER NOT NULL,
             message TEXT NOT NULL,
             post_import_action TEXT NOT NULL DEFAULT 'none' CHECK (
                 post_import_action IN ('none', 'pause_pending', 'pause_applied')
             ),
             created_at INTEGER NOT NULL,
             updated_at INTEGER NOT NULL,
             download_plan TEXT NOT NULL DEFAULT ''
         );
         CREATE INDEX IF NOT EXISTS download_jobs_info_hash
             ON download_jobs(info_hash);
         CREATE TABLE IF NOT EXISTS manual_torrent_sources (
             game_uid TEXT NOT NULL CHECK (length(game_uid) BETWEEN 1 AND 512),
             launchbox_db_id INTEGER NOT NULL DEFAULT 0 CHECK (launchbox_db_id >= 0),
             canonical_title TEXT NOT NULL CHECK (length(canonical_title) BETWEEN 1 AND 1024),
             platform TEXT NOT NULL CHECK (length(platform) BETWEEN 1 AND 512),
             source_file_name TEXT NOT NULL CHECK (length(source_file_name) BETWEEN 1 AND 1024),
             torrent_name TEXT NOT NULL CHECK (length(torrent_name) BETWEEN 1 AND 1024),
             torrent_sha256 TEXT NOT NULL CHECK (length(torrent_sha256)=64),
             info_hash TEXT NOT NULL CHECK (length(info_hash)=40),
             selected_file_index INTEGER NOT NULL CHECK (selected_file_index >= 0),
             selected_file_path TEXT NOT NULL CHECK (length(selected_file_path) BETWEEN 1 AND 4096),
             queued_job_id TEXT NOT NULL CHECK (length(queued_job_id) > 0),
             reviewed_at INTEGER NOT NULL CHECK (reviewed_at >= 0),
             PRIMARY KEY (game_uid, info_hash, selected_file_index)
         );
         CREATE INDEX IF NOT EXISTS manual_torrent_sources_info_hash
             ON manual_torrent_sources(info_hash, game_uid);
         CREATE TABLE IF NOT EXISTS installed_games (
             game_uid TEXT PRIMARY KEY,
             launchbox_db_id INTEGER NOT NULL DEFAULT 0,
             title TEXT NOT NULL,
             platform TEXT NOT NULL,
             file_path TEXT NOT NULL,
             file_size INTEGER,
             import_source TEXT NOT NULL,
             installed_at INTEGER NOT NULL
         );
         CREATE INDEX IF NOT EXISTS installed_games_launchbox_id
             ON installed_games(launchbox_db_id) WHERE launchbox_db_id > 0;
         CREATE TABLE IF NOT EXISTS local_collection_roots (
             id TEXT PRIMARY KEY,
             path_display TEXT NOT NULL,
             path_bytes BLOB NOT NULL,
             path_encoding TEXT NOT NULL CHECK (
                 path_encoding IN ('unix_bytes', 'windows_utf16le', 'utf8')
             ),
             platform_hint TEXT NOT NULL,
             checksums_enabled INTEGER NOT NULL CHECK (checksums_enabled IN (0, 1)),
             last_scanned_at INTEGER NOT NULL
         );
         CREATE TABLE IF NOT EXISTS local_rom_files (
             id TEXT PRIMARY KEY,
             root_id TEXT NOT NULL REFERENCES local_collection_roots(id) ON DELETE CASCADE,
             path_display TEXT NOT NULL,
             path_bytes BLOB NOT NULL,
             path_encoding TEXT NOT NULL CHECK (
                 path_encoding IN ('unix_bytes', 'windows_utf16le', 'utf8')
             ),
             relative_path_display TEXT NOT NULL,
             relative_path_bytes BLOB NOT NULL,
             relative_path_encoding TEXT NOT NULL CHECK (
                 relative_path_encoding IN ('unix_bytes', 'windows_utf16le', 'utf8')
             ),
             file_name TEXT NOT NULL,
             archive_member TEXT NOT NULL DEFAULT '',
             source_archive_path_display TEXT NOT NULL DEFAULT '',
             source_archive_path_bytes BLOB NOT NULL DEFAULT X'',
             source_archive_path_encoding TEXT NOT NULL DEFAULT '' CHECK (
                 source_archive_path_encoding IN (
                     '', 'unix_bytes', 'windows_utf16le', 'utf8'
                 )
             ),
             display_title TEXT NOT NULL,
             platform TEXT NOT NULL,
             file_size INTEGER NOT NULL CHECK (file_size >= 0),
             modified_unix_ns INTEGER,
             crc32 TEXT NOT NULL,
             md5 TEXT NOT NULL,
             sha1 TEXT NOT NULL,
             game_uid TEXT,
             launchbox_db_id INTEGER NOT NULL DEFAULT 0,
             matched_title TEXT,
             match_state TEXT NOT NULL CHECK (
                 match_state IN (
                     'exact', 'reviewed', 'ambiguous', 'unmatched', 'inventory_only', 'error'
                 )
             ),
             match_method TEXT NOT NULL,
             included INTEGER NOT NULL CHECK (included IN (0, 1)),
             availability TEXT NOT NULL CHECK (availability IN ('present', 'missing')),
             imported_at INTEGER NOT NULL
         );
         CREATE UNIQUE INDEX IF NOT EXISTS local_rom_files_root_relative
             ON local_rom_files(root_id, relative_path_bytes, archive_member);
         CREATE INDEX IF NOT EXISTS local_rom_files_game_uid
             ON local_rom_files(game_uid) WHERE game_uid IS NOT NULL;
         CREATE INDEX IF NOT EXISTS local_rom_files_visible
             ON local_rom_files(included, availability);
         CREATE TABLE IF NOT EXISTS game_metadata_overrides (
             game_uid TEXT PRIMARY KEY,
             launchbox_db_id INTEGER NOT NULL DEFAULT 0 CHECK (launchbox_db_id >= 0),
             canonical_title TEXT NOT NULL,
             platform TEXT NOT NULL,
             title TEXT,
             description TEXT,
             release_date TEXT,
             developer TEXT,
             publisher TEXT,
             genre TEXT,
             players TEXT,
             rating TEXT,
             esrb TEXT,
             release_type TEXT,
             notes TEXT,
             updated_at INTEGER NOT NULL CHECK (updated_at >= 0),
             CHECK (
                 title IS NOT NULL OR description IS NOT NULL OR release_date IS NOT NULL
                 OR developer IS NOT NULL OR publisher IS NOT NULL OR genre IS NOT NULL
                 OR players IS NOT NULL OR rating IS NOT NULL OR esrb IS NOT NULL
                 OR release_type IS NOT NULL OR notes IS NOT NULL
             )
         );
         CREATE INDEX IF NOT EXISTS game_metadata_overrides_updated
             ON game_metadata_overrides(updated_at DESC, game_uid);
         CREATE TABLE IF NOT EXISTS user_tags (
             id TEXT PRIMARY KEY,
             name TEXT NOT NULL CHECK (length(name) BETWEEN 1 AND 50),
             normalized_name TEXT NOT NULL UNIQUE CHECK (length(normalized_name) > 0),
             created_at INTEGER NOT NULL CHECK (created_at >= 0),
             updated_at INTEGER NOT NULL CHECK (updated_at >= 0)
         );
         CREATE TABLE IF NOT EXISTS game_tags (
             tag_id TEXT NOT NULL REFERENCES user_tags(id) ON DELETE CASCADE,
             game_uid TEXT NOT NULL CHECK (length(game_uid) BETWEEN 1 AND 512),
             added_at INTEGER NOT NULL CHECK (added_at >= 0),
             PRIMARY KEY (tag_id, game_uid)
         );
         CREATE INDEX IF NOT EXISTS game_tags_game
             ON game_tags(game_uid, tag_id);
         CREATE TABLE IF NOT EXISTS favorite_games (
             game_uid TEXT PRIMARY KEY,
             launchbox_db_id INTEGER NOT NULL DEFAULT 0,
             title TEXT NOT NULL,
             platform TEXT NOT NULL,
             added_at INTEGER NOT NULL
         );
         CREATE INDEX IF NOT EXISTS favorite_games_launchbox_id
             ON favorite_games(launchbox_db_id) WHERE launchbox_db_id > 0;
         CREATE INDEX IF NOT EXISTS favorite_games_added_at
             ON favorite_games(added_at DESC);
         CREATE TABLE IF NOT EXISTS game_activity (
             game_uid TEXT PRIMARY KEY,
             launchbox_db_id INTEGER NOT NULL DEFAULT 0,
             title TEXT NOT NULL,
             platform TEXT NOT NULL,
             play_count INTEGER NOT NULL DEFAULT 0 CHECK (play_count >= 0),
             total_play_time_seconds INTEGER NOT NULL DEFAULT 0 CHECK (
                 total_play_time_seconds >= 0
             ),
             last_played_at INTEGER NOT NULL DEFAULT 0 CHECK (last_played_at >= 0),
             first_played_at INTEGER NOT NULL DEFAULT 0 CHECK (first_played_at >= 0),
             completion_state TEXT NOT NULL DEFAULT 'not_started' CHECK (
                 completion_state IN (
                     'not_started', 'in_progress', 'completed', 'on_hold', 'abandoned'
                 )
             )
         );
         CREATE INDEX IF NOT EXISTS game_activity_recent
             ON game_activity(last_played_at DESC, game_uid) WHERE play_count > 0;
         CREATE INDEX IF NOT EXISTS game_activity_completion
             ON game_activity(completion_state, game_uid);
         CREATE TABLE IF NOT EXISTS play_sessions (
             id TEXT PRIMARY KEY,
             game_uid TEXT NOT NULL REFERENCES game_activity(game_uid) ON DELETE CASCADE,
             launchbox_db_id INTEGER NOT NULL DEFAULT 0,
             title TEXT NOT NULL,
             platform TEXT NOT NULL,
             emulator TEXT NOT NULL,
             started_at INTEGER NOT NULL CHECK (started_at >= 0),
             ended_at INTEGER,
             duration_seconds INTEGER NOT NULL DEFAULT 0 CHECK (duration_seconds >= 0),
             outcome TEXT NOT NULL CHECK (
                 outcome IN ('running', 'completed', 'failed', 'terminated')
             ),
             CHECK (
                 (outcome='running' AND ended_at IS NULL)
                 OR (outcome<>'running' AND ended_at IS NOT NULL)
             )
         );
         CREATE INDEX IF NOT EXISTS play_sessions_game_started
             ON play_sessions(game_uid, started_at DESC);
         CREATE INDEX IF NOT EXISTS play_sessions_running
             ON play_sessions(outcome, started_at) WHERE outcome='running';
         CREATE TABLE IF NOT EXISTS media_playback_progress (
             game_uid TEXT NOT NULL,
             media_key TEXT NOT NULL,
             position_ms INTEGER NOT NULL CHECK (position_ms >= 0),
             duration_ms INTEGER NOT NULL CHECK (duration_ms >= 0),
             updated_at INTEGER NOT NULL CHECK (updated_at >= 0),
             PRIMARY KEY (game_uid, media_key)
         );
         CREATE INDEX IF NOT EXISTS media_playback_progress_updated
             ON media_playback_progress(updated_at DESC, game_uid);
         CREATE TABLE IF NOT EXISTS media_provider_game_links (
             provider TEXT NOT NULL,
             launchbox_db_id INTEGER NOT NULL CHECK (launchbox_db_id > 0),
             provider_game_id TEXT NOT NULL CHECK (length(provider_game_id) > 0),
             provider_game_name TEXT NOT NULL CHECK (length(provider_game_name) > 0),
             reviewed_at INTEGER NOT NULL CHECK (reviewed_at >= 0),
             PRIMARY KEY (provider, launchbox_db_id)
         );
         CREATE INDEX IF NOT EXISTS media_provider_game_links_provider_id
             ON media_provider_game_links(provider, provider_game_id);
         CREATE TABLE IF NOT EXISTS media_transfers (
             game_uid TEXT NOT NULL,
             launchbox_db_id INTEGER NOT NULL DEFAULT 0,
             title TEXT NOT NULL,
             platform TEXT NOT NULL,
             media_kind TEXT NOT NULL CHECK (media_kind IN ('manual', 'video')),
             provider TEXT NOT NULL CHECK (provider IN ('minerva', 'emumovies')),
             transport TEXT NOT NULL CHECK (transport IN ('torrent', 'http')),
             source_url TEXT NOT NULL,
             remote_job_id TEXT NOT NULL,
             source_item_index INTEGER NOT NULL CHECK (source_item_index >= 0),
             source_item_path TEXT NOT NULL,
             local_download_path TEXT NOT NULL,
             local_target_path TEXT NOT NULL,
             state TEXT NOT NULL CHECK (
                 state IN (
                     'queued', 'downloading', 'paused', 'complete',
                     'published', 'cancelled', 'error'
                 )
             ),
             progress REAL NOT NULL CHECK (progress BETWEEN 0.0 AND 1.0),
             download_speed INTEGER NOT NULL CHECK (download_speed >= 0),
             downloaded_bytes INTEGER NOT NULL CHECK (downloaded_bytes >= 0),
             total_bytes INTEGER NOT NULL CHECK (total_bytes >= 0),
             message TEXT NOT NULL,
             created_at INTEGER NOT NULL CHECK (created_at >= 0),
             updated_at INTEGER NOT NULL CHECK (updated_at >= 0),
             PRIMARY KEY (game_uid, media_kind)
         );
         CREATE INDEX IF NOT EXISTS media_transfers_remote_job
             ON media_transfers(provider, remote_job_id, state);
         CREATE INDEX IF NOT EXISTS media_transfers_updated
             ON media_transfers(updated_at DESC, game_uid);
         CREATE TABLE IF NOT EXISTS user_collections (
             id TEXT PRIMARY KEY,
             name TEXT NOT NULL COLLATE NOCASE UNIQUE,
             description TEXT NOT NULL,
             kind TEXT NOT NULL DEFAULT 'manual' CHECK (kind IN ('manual', 'smart')),
             rules_json TEXT NOT NULL DEFAULT '',
             created_at INTEGER NOT NULL,
             updated_at INTEGER NOT NULL,
             CHECK (
                 (kind='manual' AND rules_json='')
                 OR (kind='smart' AND length(rules_json)>0)
             )
         );
         CREATE TABLE IF NOT EXISTS user_collection_games (
             collection_id TEXT NOT NULL REFERENCES user_collections(id) ON DELETE CASCADE,
             game_uid TEXT NOT NULL,
             sort_order INTEGER NOT NULL,
             added_at INTEGER NOT NULL,
             PRIMARY KEY (collection_id, game_uid)
         );
         CREATE INDEX IF NOT EXISTS user_collection_games_order
             ON user_collection_games(collection_id, sort_order, game_uid);
         CREATE INDEX IF NOT EXISTS user_collection_games_game
             ON user_collection_games(game_uid);
         CREATE TABLE IF NOT EXISTS game_emulator_preferences (
             game_uid TEXT PRIMARY KEY,
             emulator_id TEXT NOT NULL,
             runtime_kind TEXT NOT NULL CHECK (
                 runtime_kind IN ('standalone', 'retroarch')
             ),
             core_name TEXT NOT NULL DEFAULT '',
             updated_at INTEGER NOT NULL
         );
         CREATE TABLE IF NOT EXISTS platform_emulator_preferences (
             platform_key TEXT PRIMARY KEY,
             platform_display TEXT NOT NULL,
             emulator_id TEXT NOT NULL,
             runtime_kind TEXT NOT NULL CHECK (
                 runtime_kind IN ('standalone', 'retroarch')
             ),
             core_name TEXT NOT NULL DEFAULT '',
             updated_at INTEGER NOT NULL
         );
         CREATE TABLE IF NOT EXISTS emulator_launch_profiles (
             scope_kind TEXT NOT NULL CHECK (
                 scope_kind IN ('global', 'platform', 'game')
             ),
             scope_key TEXT NOT NULL,
             emulator_id TEXT NOT NULL,
             runtime_kind TEXT NOT NULL CHECK (
                 runtime_kind IN ('standalone', 'retroarch')
             ),
             core_name TEXT NOT NULL DEFAULT '',
             extra_arguments TEXT NOT NULL DEFAULT '',
             command_template TEXT NOT NULL DEFAULT '',
             updated_at INTEGER NOT NULL,
             PRIMARY KEY (
                 scope_kind, scope_key, emulator_id, runtime_kind, core_name
             ),
             CHECK (
                 (scope_kind='global' AND scope_key='')
                 OR (scope_kind<>'global' AND length(scope_key)>0)
             ),
             CHECK (length(extra_arguments)>0 OR length(command_template)>0),
             CHECK (
                 (runtime_kind='standalone' AND core_name='')
                 OR (runtime_kind='retroarch' AND length(core_name)>0)
             )
         );
         CREATE INDEX IF NOT EXISTS emulator_launch_profiles_emulator
             ON emulator_launch_profiles(emulator_id, runtime_kind, core_name, scope_kind);
         CREATE TABLE IF NOT EXISTS managed_emulator_installs (
             emulator_id TEXT NOT NULL,
             host_system_slug TEXT NOT NULL CHECK (
                 host_system_slug IN ('linux', 'windows', 'macos')
             ),
             manager TEXT NOT NULL CHECK (
                 manager IN ('flatpak', 'appimage', 'nix', 'github', 'direct', 'winget', 'homebrew', 'libretro')
             ),
             package_id TEXT NOT NULL,
             install_path TEXT NOT NULL,
             installed_at INTEGER NOT NULL,
             updated_at INTEGER NOT NULL,
             PRIMARY KEY (host_system_slug, manager, package_id)
         );
         CREATE TABLE IF NOT EXISTS firmware_packages (
             source_id TEXT NOT NULL,
             package_name TEXT NOT NULL,
             archive_path TEXT NOT NULL,
             extracted_root TEXT NOT NULL,
             sha256 TEXT NOT NULL CHECK (length(sha256)=64),
             file_count INTEGER NOT NULL CHECK (file_count >= 0),
             imported_at INTEGER NOT NULL,
             PRIMARY KEY (source_id, package_name)
         );
         CREATE TABLE IF NOT EXISTS firmware_files (
             source_id TEXT NOT NULL,
             package_name TEXT NOT NULL,
             relative_path TEXT NOT NULL,
             store_path TEXT NOT NULL,
             sha256 TEXT NOT NULL CHECK (length(sha256)=64),
             file_size INTEGER NOT NULL CHECK (file_size >= 0),
             PRIMARY KEY (source_id, package_name, relative_path),
             FOREIGN KEY (source_id, package_name)
                 REFERENCES firmware_packages(source_id, package_name)
                 ON DELETE CASCADE
         );
         CREATE INDEX IF NOT EXISTS firmware_files_sha256
             ON firmware_files(sha256);
         CREATE TABLE IF NOT EXISTS firmware_downloads (
             source_id TEXT NOT NULL,
             package_name TEXT NOT NULL,
             torrent_url TEXT NOT NULL,
             info_hash TEXT NOT NULL,
             file_index INTEGER NOT NULL CHECK (file_index >= 0),
             file_path TEXT NOT NULL,
             local_path TEXT NOT NULL,
             state TEXT NOT NULL CHECK (
                 state IN ('queued', 'downloading', 'paused', 'complete', 'imported', 'failed')
             ),
             progress REAL NOT NULL CHECK (progress >= 0.0 AND progress <= 1.0),
             message TEXT NOT NULL,
             updated_at INTEGER NOT NULL,
             PRIMARY KEY (source_id, package_name)
         );
         CREATE INDEX IF NOT EXISTS firmware_downloads_info_hash
             ON firmware_downloads(info_hash, state);
         CREATE TABLE IF NOT EXISTS firmware_installs (
             rule_key TEXT NOT NULL,
             runtime_path TEXT NOT NULL,
             target_path TEXT NOT NULL,
             source_id TEXT NOT NULL,
             package_name TEXT NOT NULL,
             synced_at INTEGER NOT NULL,
             PRIMARY KEY (rule_key, runtime_path),
             FOREIGN KEY (source_id, package_name)
                 REFERENCES firmware_packages(source_id, package_name)
                 ON DELETE CASCADE
         );
         CREATE TABLE IF NOT EXISTS prepared_game_installs (
             game_uid TEXT PRIMARY KEY,
             launchbox_db_id INTEGER NOT NULL DEFAULT 0,
             platform TEXT NOT NULL,
             collection TEXT NOT NULL CHECK (
                 collection IN ('exodos', 'exowin3x', 'exowin9x')
             ),
             shortname TEXT NOT NULL,
             source_archive_path TEXT NOT NULL,
             companion_archive_path TEXT,
             metadata_archive_path TEXT NOT NULL,
             utility_archive_path TEXT,
             source_signature TEXT NOT NULL,
             install_root TEXT NOT NULL,
             launch_config_path TEXT NOT NULL,
             prepared_at INTEGER NOT NULL,
             last_used_at INTEGER NOT NULL
         );
         CREATE INDEX IF NOT EXISTS prepared_game_installs_launchbox_id
             ON prepared_game_installs(launchbox_db_id) WHERE launchbox_db_id > 0;",
    )?;
    let added_archive_member = !column_exists(connection, "local_rom_files", "archive_member")?;
    if added_archive_member {
        connection.execute(
            "ALTER TABLE local_rom_files ADD COLUMN archive_member TEXT NOT NULL DEFAULT ''",
            [],
        )?;
    }
    if !column_exists(connection, "local_rom_files", "source_archive_path_display")? {
        connection.execute(
            "ALTER TABLE local_rom_files ADD COLUMN source_archive_path_display TEXT NOT NULL DEFAULT ''",
            [],
        )?;
    }
    if !column_exists(connection, "local_rom_files", "source_archive_path_bytes")? {
        connection.execute(
            "ALTER TABLE local_rom_files ADD COLUMN source_archive_path_bytes BLOB NOT NULL DEFAULT X''",
            [],
        )?;
    }
    if !column_exists(
        connection,
        "local_rom_files",
        "source_archive_path_encoding",
    )? {
        connection.execute(
            "ALTER TABLE local_rom_files ADD COLUMN source_archive_path_encoding TEXT NOT NULL DEFAULT '' CHECK (source_archive_path_encoding IN ('', 'unix_bytes', 'windows_utf16le', 'utf8'))",
            [],
        )?;
    }
    if added_archive_member {
        connection.execute_batch(
            "DROP INDEX IF EXISTS local_rom_files_root_relative;
             CREATE UNIQUE INDEX local_rom_files_root_relative
                 ON local_rom_files(root_id, relative_path_bytes, archive_member);",
        )?;
    }
    if !local_rom_match_state_allows_reviewed(connection)? {
        migrate_local_rom_match_state(connection)?;
    }
    if !column_exists(connection, "download_jobs", "launchbox_db_id")? {
        connection.execute(
            "ALTER TABLE download_jobs ADD COLUMN launchbox_db_id INTEGER NOT NULL DEFAULT 0",
            [],
        )?;
    }
    if !column_exists(connection, "download_jobs", "source_kind")? {
        connection.execute(
            "ALTER TABLE download_jobs ADD COLUMN source_kind TEXT NOT NULL DEFAULT 'minerva' CHECK (source_kind IN ('minerva', 'manual_torrent'))",
            [],
        )?;
    }
    if !column_exists(connection, "download_jobs", "post_import_action")? {
        connection.execute(
            "ALTER TABLE download_jobs ADD COLUMN post_import_action TEXT NOT NULL DEFAULT 'none' CHECK (post_import_action IN ('none', 'pause_pending', 'pause_applied'))",
            [],
        )?;
    }
    if !column_exists(connection, "download_jobs", "download_plan")? {
        connection.execute(
            "ALTER TABLE download_jobs ADD COLUMN download_plan TEXT NOT NULL DEFAULT ''",
            [],
        )?;
    }
    if !column_exists(connection, "library_preferences", "artwork_type")? {
        connection.execute(
            "ALTER TABLE library_preferences ADD COLUMN artwork_type TEXT NOT NULL DEFAULT 'box-front'",
            [],
        )?;
    }
    if !column_exists(connection, "library_preferences", "grid_zoom")? {
        connection.execute(
            "ALTER TABLE library_preferences ADD COLUMN grid_zoom INTEGER NOT NULL DEFAULT 100",
            [],
        )?;
    }
    if !column_exists(connection, "app_settings", "seeding_policy")? {
        connection.execute(
            "ALTER TABLE app_settings ADD COLUMN seeding_policy TEXT NOT NULL DEFAULT 'follow_client' CHECK (seeding_policy IN ('follow_client', 'pause_after_import'))",
            [],
        )?;
    }
    if !column_exists(connection, "app_settings", "preferred_region")? {
        connection.execute(
            "ALTER TABLE app_settings ADD COLUMN preferred_region TEXT NOT NULL DEFAULT 'USA' CHECK (preferred_region IN ('USA', 'Japan', 'Europe', 'World', 'Asia', 'any'))",
            [],
        )?;
    }
    if !column_exists(connection, "app_settings", "version_preference")? {
        connection.execute(
            "ALTER TABLE app_settings ADD COLUMN version_preference TEXT NOT NULL DEFAULT 'latest' CHECK (version_preference IN ('latest', 'original'))",
            [],
        )?;
    }
    if !column_exists(connection, "app_settings", "region_priority_json")? {
        connection.execute(
            "ALTER TABLE app_settings ADD COLUMN region_priority_json TEXT NOT NULL DEFAULT '[]'",
            [],
        )?;
    }
    if !column_exists(connection, "app_settings", "controller_mapping_json")? {
        connection.execute(
            "ALTER TABLE app_settings ADD COLUMN controller_mapping_json TEXT NOT NULL DEFAULT '{}'",
            [],
        )?;
    }
    if !column_exists(connection, "app_settings", "media_provider_priority_json")? {
        connection.execute(
            "ALTER TABLE app_settings ADD COLUMN media_provider_priority_json TEXT NOT NULL DEFAULT '[]'",
            [],
        )?;
    }
    if !column_exists(connection, "user_collections", "kind")? {
        connection.execute(
            "ALTER TABLE user_collections ADD COLUMN kind TEXT NOT NULL DEFAULT 'manual' CHECK (kind IN ('manual', 'smart'))",
            [],
        )?;
    }
    if !column_exists(connection, "user_collections", "rules_json")? {
        connection.execute(
            "ALTER TABLE user_collections ADD COLUMN rules_json TEXT NOT NULL DEFAULT ''",
            [],
        )?;
    }
    Ok(())
}

fn column_exists(connection: &Connection, table: &str, column: &str) -> Result<bool> {
    let mut statement = connection.prepare(&format!("PRAGMA table_info({table})"))?;
    let names = statement.query_map([], |row| row.get::<_, String>(1))?;
    for name in names {
        if name? == column {
            return Ok(true);
        }
    }
    Ok(false)
}

fn local_rom_match_state_allows_reviewed(connection: &Connection) -> Result<bool> {
    let definition = connection.query_row(
        "SELECT sql FROM sqlite_schema WHERE type='table' AND name='local_rom_files'",
        [],
        |row| row.get::<_, String>(0),
    )?;
    Ok(definition.contains("'reviewed'"))
}

fn migrate_local_rom_match_state(connection: &Connection) -> Result<()> {
    connection.execute_batch(
        "PRAGMA foreign_keys=OFF;
         BEGIN IMMEDIATE;
         DROP INDEX IF EXISTS local_rom_files_root_relative;
         DROP INDEX IF EXISTS local_rom_files_game_uid;
         DROP INDEX IF EXISTS local_rom_files_visible;
         ALTER TABLE local_rom_files RENAME TO local_rom_files_before_reviewed;
         CREATE TABLE local_rom_files (
             id TEXT PRIMARY KEY,
             root_id TEXT NOT NULL REFERENCES local_collection_roots(id) ON DELETE CASCADE,
             path_display TEXT NOT NULL,
             path_bytes BLOB NOT NULL,
             path_encoding TEXT NOT NULL CHECK (
                 path_encoding IN ('unix_bytes', 'windows_utf16le', 'utf8')
             ),
             relative_path_display TEXT NOT NULL,
             relative_path_bytes BLOB NOT NULL,
             relative_path_encoding TEXT NOT NULL CHECK (
                 relative_path_encoding IN ('unix_bytes', 'windows_utf16le', 'utf8')
             ),
             file_name TEXT NOT NULL,
             archive_member TEXT NOT NULL DEFAULT '',
             source_archive_path_display TEXT NOT NULL DEFAULT '',
             source_archive_path_bytes BLOB NOT NULL DEFAULT X'',
             source_archive_path_encoding TEXT NOT NULL DEFAULT '' CHECK (
                 source_archive_path_encoding IN (
                     '', 'unix_bytes', 'windows_utf16le', 'utf8'
                 )
             ),
             display_title TEXT NOT NULL,
             platform TEXT NOT NULL,
             file_size INTEGER NOT NULL CHECK (file_size >= 0),
             modified_unix_ns INTEGER,
             crc32 TEXT NOT NULL,
             md5 TEXT NOT NULL,
             sha1 TEXT NOT NULL,
             game_uid TEXT,
             launchbox_db_id INTEGER NOT NULL DEFAULT 0,
             matched_title TEXT,
             match_state TEXT NOT NULL CHECK (
                 match_state IN (
                     'exact', 'reviewed', 'ambiguous', 'unmatched', 'inventory_only', 'error'
                 )
             ),
             match_method TEXT NOT NULL,
             included INTEGER NOT NULL CHECK (included IN (0, 1)),
             availability TEXT NOT NULL CHECK (availability IN ('present', 'missing')),
             imported_at INTEGER NOT NULL
         );
         INSERT INTO local_rom_files (
             id, root_id, path_display, path_bytes, path_encoding,
             relative_path_display, relative_path_bytes, relative_path_encoding,
             file_name, archive_member, source_archive_path_display,
             source_archive_path_bytes, source_archive_path_encoding, display_title,
             platform, file_size, modified_unix_ns, crc32, md5, sha1, game_uid,
             launchbox_db_id, matched_title, match_state, match_method, included,
             availability, imported_at
         )
         SELECT
             id, root_id, path_display, path_bytes, path_encoding,
             relative_path_display, relative_path_bytes, relative_path_encoding,
             file_name, archive_member, source_archive_path_display,
             source_archive_path_bytes, source_archive_path_encoding, display_title,
             platform, file_size, modified_unix_ns, crc32, md5, sha1, game_uid,
             launchbox_db_id, matched_title, match_state, match_method, included,
             availability, imported_at
         FROM local_rom_files_before_reviewed;
         DROP TABLE local_rom_files_before_reviewed;
         CREATE UNIQUE INDEX local_rom_files_root_relative
             ON local_rom_files(root_id, relative_path_bytes, archive_member);
         CREATE INDEX local_rom_files_game_uid
             ON local_rom_files(game_uid) WHERE game_uid IS NOT NULL;
         CREATE INDEX local_rom_files_visible
             ON local_rom_files(included, availability);
         COMMIT;
         PRAGMA foreign_keys=ON;",
    )?;
    Ok(())
}

fn validate_collection_name(name: &str) -> Result<String> {
    let name = name.trim();
    if name.is_empty() {
        bail!("collection name is required");
    }
    if name.chars().count() > 100 {
        bail!("collection name must be at most 100 characters");
    }
    Ok(name.to_owned())
}

fn normalize_tag_name(name: &str) -> Result<String> {
    let name = name.split_whitespace().collect::<Vec<_>>().join(" ");
    if name.is_empty() {
        bail!("tag names cannot be empty");
    }
    if name.chars().count() > 50 {
        bail!("tag names must be at most 50 characters");
    }
    if name.contains(',') {
        bail!("tag names cannot contain commas");
    }
    if name.chars().any(|character| character.is_control()) {
        bail!("tag names cannot contain control characters");
    }
    Ok(name)
}

fn normalized_tag_name(name: &str) -> String {
    name.to_lowercase()
}

fn stable_tag_id(normalized_name: &str) -> String {
    uuid::Uuid::new_v5(
        &uuid::Uuid::NAMESPACE_URL,
        format!("https://lunchbox.games/user-tag/{normalized_name}").as_bytes(),
    )
    .to_string()
}

fn validate_tag_game_uid(game_uid: &str) -> Result<()> {
    if game_uid.trim().is_empty() || game_uid.chars().count() > 512 {
        bail!("tags require a bounded stable game identity");
    }
    Ok(())
}

fn validate_metadata_identity(
    game_uid: &str,
    launchbox_db_id: i64,
    canonical_title: &str,
    platform: &str,
) -> Result<()> {
    validate_tag_game_uid(game_uid)?;
    if canonical_title.trim().is_empty() || platform.trim().is_empty() {
        bail!("canonical title and platform are required to save metadata overrides");
    }
    if launchbox_db_id < 0 {
        bail!("LaunchBox database identity cannot be negative");
    }
    Ok(())
}

fn save_game_metadata_override_on(
    connection: &Connection,
    game_uid: &str,
    launchbox_db_id: i64,
    canonical_title: &str,
    platform: &str,
    metadata: &GameMetadataOverride,
) -> Result<()> {
    validate_metadata_identity(game_uid, launchbox_db_id, canonical_title, platform)?;
    metadata.validate()?;
    if metadata.is_empty() {
        connection.execute(
            "DELETE FROM game_metadata_overrides WHERE game_uid=?1",
            [game_uid],
        )?;
        return Ok(());
    }
    connection.execute(
        "INSERT INTO game_metadata_overrides (
             game_uid, launchbox_db_id, canonical_title, platform,
             title, description, release_date, developer, publisher,
             genre, players, rating, esrb, release_type, notes, updated_at
         ) VALUES (
             ?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15, ?16
         )
         ON CONFLICT(game_uid) DO UPDATE SET
             launchbox_db_id=excluded.launchbox_db_id,
             canonical_title=excluded.canonical_title,
             platform=excluded.platform,
             title=excluded.title,
             description=excluded.description,
             release_date=excluded.release_date,
             developer=excluded.developer,
             publisher=excluded.publisher,
             genre=excluded.genre,
             players=excluded.players,
             rating=excluded.rating,
             esrb=excluded.esrb,
             release_type=excluded.release_type,
             notes=excluded.notes,
             updated_at=excluded.updated_at",
        params![
            game_uid,
            launchbox_db_id,
            canonical_title,
            platform,
            metadata.title,
            metadata.description,
            metadata.release_date,
            metadata.developer,
            metadata.publisher,
            metadata.genre,
            metadata.players,
            metadata.rating,
            metadata.esrb,
            metadata.release_type,
            metadata.notes,
            unix_timestamp(),
        ],
    )?;
    Ok(())
}

fn validated_collection_game_uids(game_uids: &[String]) -> Result<Vec<String>> {
    if game_uids.len() > 500_000 {
        bail!("collection membership operation contains too many games");
    }
    let mut unique = HashSet::with_capacity(game_uids.len());
    let mut validated = Vec::with_capacity(game_uids.len());
    for game_uid in game_uids {
        let game_uid = game_uid.trim();
        if game_uid.is_empty() || game_uid.chars().count() > 512 {
            bail!("collection membership requires bounded stable game identities");
        }
        if unique.insert(game_uid.to_owned()) {
            validated.push(game_uid.to_owned());
        }
    }
    Ok(validated)
}

fn validate_play_identity(game_uid: &str, title: &str, platform: &str) -> Result<()> {
    if game_uid.trim().is_empty() || title.trim().is_empty() || platform.trim().is_empty() {
        bail!("play activity requires a stable game identity, title, and platform");
    }
    if game_uid.chars().count() > 512
        || title.chars().count() > 1000
        || platform.chars().count() > 500
    {
        bail!("play activity identity fields exceed their safety limits");
    }
    Ok(())
}

fn validate_completion_state(completion_state: &str) -> Result<()> {
    if !matches!(
        completion_state,
        "not_started" | "in_progress" | "completed" | "on_hold" | "abandoned"
    ) {
        bail!("unsupported completion state {completion_state}");
    }
    Ok(())
}

fn validate_media_playback_identity(game_uid: &str, media_key: &str) -> Result<()> {
    if game_uid.trim().is_empty() || media_key.trim().is_empty() {
        bail!("media playback progress requires stable game and media identities");
    }
    if game_uid.chars().count() > 512 || media_key.chars().count() > 256 {
        bail!("media playback identity fields exceed their safety limits");
    }
    Ok(())
}

fn validate_media_transfer_identity(game_uid: &str, media_kind: &str) -> Result<()> {
    if game_uid.trim().is_empty() || game_uid.chars().count() > 512 {
        bail!("media transfer requires a bounded stable game identity");
    }
    if !matches!(media_kind, "manual" | "video") {
        bail!("unsupported media transfer kind {media_kind}");
    }
    Ok(())
}

fn validate_media_transfer(transfer: &MediaTransfer) -> Result<()> {
    validate_media_transfer_identity(&transfer.game_uid, &transfer.media_kind)?;
    if !matches!(transfer.provider.as_str(), "minerva" | "emumovies") {
        bail!("unsupported media transfer provider {}", transfer.provider);
    }
    if !matches!(transfer.transport.as_str(), "torrent" | "http") {
        bail!(
            "unsupported media transfer transport {}",
            transfer.transport
        );
    }
    if transfer.launchbox_db_id < 0 {
        bail!("media transfer LaunchBox ID cannot be negative");
    }
    for (label, value, limit) in [
        ("title", transfer.title.as_str(), 2_048),
        ("platform", transfer.platform.as_str(), 1_024),
        ("source URL", transfer.source_url.as_str(), 8_192),
        ("remote job identity", transfer.remote_job_id.as_str(), 512),
        (
            "source item path",
            transfer.source_item_path.as_str(),
            8_192,
        ),
        ("message", transfer.message.as_str(), 8_192),
    ] {
        if value.trim().is_empty() || value.chars().count() > limit {
            bail!("media transfer {label} is empty or exceeds its safety limit");
        }
    }
    if transfer.local_download_path.as_os_str().is_empty()
        || transfer.local_target_path.as_os_str().is_empty()
    {
        bail!("media transfer paths cannot be empty");
    }
    if !matches!(
        transfer.state.as_str(),
        "queued" | "downloading" | "paused" | "complete" | "published" | "cancelled" | "error"
    ) {
        bail!("unsupported media transfer state {}", transfer.state);
    }
    if !transfer.progress.is_finite() || !(0.0..=1.0).contains(&transfer.progress) {
        bail!("media transfer progress must be between zero and one");
    }
    if transfer.created_at < 0 || transfer.updated_at < 0 {
        bail!("media transfer timestamps cannot be negative");
    }
    Ok(())
}

fn media_transfer_from_row(row: &rusqlite::Row<'_>) -> rusqlite::Result<MediaTransfer> {
    let source_item_index = row.get::<_, i64>(9)?;
    let download_speed = row.get::<_, i64>(15)?;
    let downloaded_bytes = row.get::<_, i64>(16)?;
    let total_bytes = row.get::<_, i64>(17)?;
    Ok(MediaTransfer {
        game_uid: row.get(0)?,
        launchbox_db_id: row.get(1)?,
        title: row.get(2)?,
        platform: row.get(3)?,
        media_kind: row.get(4)?,
        provider: row.get(5)?,
        transport: row.get(6)?,
        source_url: row.get(7)?,
        remote_job_id: row.get(8)?,
        source_item_index: u32::try_from(source_item_index).unwrap_or_default(),
        source_item_path: row.get(10)?,
        local_download_path: PathBuf::from(row.get::<_, String>(11)?),
        local_target_path: PathBuf::from(row.get::<_, String>(12)?),
        state: row.get(13)?,
        progress: row.get(14)?,
        download_speed: u64::try_from(download_speed).unwrap_or_default(),
        downloaded_bytes: u64::try_from(downloaded_bytes).unwrap_or_default(),
        total_bytes: u64::try_from(total_bytes).unwrap_or_default(),
        message: row.get(18)?,
        created_at: row.get(19)?,
        updated_at: row.get(20)?,
    })
}

fn play_activity_from_row(row: &rusqlite::Row<'_>) -> rusqlite::Result<PlayActivity> {
    Ok(PlayActivity {
        game_uid: row.get(0)?,
        launchbox_db_id: row.get(1)?,
        title: row.get(2)?,
        platform: row.get(3)?,
        play_count: row.get(4)?,
        total_play_time_seconds: row.get(5)?,
        last_played_at: row.get(6)?,
        first_played_at: row.get(7)?,
        completion_state: row.get(8)?,
    })
}

fn metadata_override_from_row(row: &rusqlite::Row<'_>) -> rusqlite::Result<GameMetadataOverride> {
    Ok(GameMetadataOverride {
        title: row.get(0)?,
        description: row.get(1)?,
        release_date: row.get(2)?,
        developer: row.get(3)?,
        publisher: row.get(4)?,
        genre: row.get(5)?,
        players: row.get(6)?,
        rating: row.get(7)?,
        esrb: row.get(8)?,
        release_type: row.get(9)?,
        notes: row.get(10)?,
    })
}

fn validate_collection_description(description: &str) -> Result<String> {
    let description = description.trim();
    if description.chars().count() > 1000 {
        bail!("collection description must be at most 1000 characters");
    }
    Ok(description.to_owned())
}

fn normalized_launch_scope_key(scope_kind: &str, scope_key: &str) -> Result<String> {
    match scope_kind {
        "global" => {
            if !scope_key.trim().is_empty() {
                bail!("a global launch profile cannot include a scope key");
            }
            Ok(String::new())
        }
        "platform" => {
            let key = catalog::normalize_platform_key(scope_key);
            if key.is_empty() {
                bail!("a platform launch profile requires an exact platform identity");
            }
            Ok(key)
        }
        "game" => {
            let key = scope_key.trim();
            if key.is_empty() || key.chars().count() > 512 {
                bail!("a game launch profile requires a bounded stable game identity");
            }
            Ok(key.to_owned())
        }
        _ => bail!("unsupported launch profile scope {scope_kind}"),
    }
}

fn validate_emulator_preference(
    identity: &str,
    emulator_id: &str,
    runtime_kind: &str,
    core_name: &str,
) -> Result<()> {
    if identity.trim().is_empty() {
        bail!("a stable game or platform identity is required for an emulator preference");
    }
    if emulator_id.trim().is_empty() {
        bail!("an emulator identity is required for an emulator preference");
    }
    if !matches!(runtime_kind, "standalone" | "retroarch") {
        bail!("unsupported emulator runtime kind {runtime_kind}");
    }
    if runtime_kind == "retroarch" && core_name.trim().is_empty() {
        bail!("a RetroArch preference requires an exact core name");
    }
    if runtime_kind == "standalone" && !core_name.trim().is_empty() {
        bail!("a standalone emulator preference cannot include a RetroArch core");
    }
    Ok(())
}

fn validate_managed_emulator_install(
    emulator_id: &str,
    host_system_slug: &str,
    manager: &str,
    package_id: &str,
) -> Result<()> {
    if emulator_id.trim().is_empty() || package_id.trim().is_empty() {
        bail!("managed emulator installs require exact emulator and package identities");
    }
    if !matches!(host_system_slug, "linux" | "windows" | "macos") {
        bail!("unsupported emulator host {host_system_slug}");
    }
    if !matches!(
        manager,
        "flatpak" | "appimage" | "nix" | "github" | "direct" | "winget" | "homebrew" | "libretro"
    ) {
        bail!("unsupported emulator install manager {manager}");
    }
    Ok(())
}

fn validate_firmware_package_receipt(receipt: &FirmwarePackageReceipt) -> Result<()> {
    if receipt.source_id.trim().is_empty() || receipt.package_name.trim().is_empty() {
        bail!("firmware packages require exact source and package identities");
    }
    if receipt.archive_path.trim().is_empty() || receipt.extracted_root.trim().is_empty() {
        bail!("firmware package receipts require archive and extracted paths");
    }
    if receipt.sha256.len() != 64 || !receipt.sha256.bytes().all(|byte| byte.is_ascii_hexdigit()) {
        bail!("firmware package receipts require a SHA-256 digest");
    }
    if receipt.file_count < 0 {
        bail!("firmware package file counts cannot be negative");
    }
    Ok(())
}

fn validate_firmware_file_receipt(
    package: &FirmwarePackageReceipt,
    file: &FirmwareFileReceipt,
) -> Result<()> {
    if file.source_id != package.source_id || file.package_name != package.package_name {
        bail!("firmware file manifest identities do not match their package");
    }
    if file.relative_path.trim().is_empty()
        || file.store_path.trim().is_empty()
        || Path::new(&file.relative_path).is_absolute()
        || Path::new(&file.relative_path)
            .components()
            .any(|component| !matches!(component, std::path::Component::Normal(_)))
    {
        bail!("firmware file manifests require safe relative paths");
    }
    if file.sha256.len() != 64
        || !file.sha256.bytes().all(|byte| byte.is_ascii_hexdigit())
        || file.file_size < 0
    {
        bail!("firmware file manifests require a digest and non-negative size");
    }
    Ok(())
}

fn validate_firmware_install_receipt(receipt: &FirmwareInstallReceipt) -> Result<()> {
    if receipt.rule_key.trim().is_empty()
        || receipt.runtime_path.trim().is_empty()
        || receipt.target_path.trim().is_empty()
        || receipt.source_id.trim().is_empty()
        || receipt.package_name.trim().is_empty()
    {
        bail!("firmware install receipts require exact rule, package, and path identities");
    }
    Ok(())
}

fn firmware_download_from_row(
    row: &rusqlite::Row<'_>,
) -> rusqlite::Result<FirmwareDownloadReceipt> {
    let file_index = row.get::<_, i64>(4)?;
    Ok(FirmwareDownloadReceipt {
        source_id: row.get(0)?,
        package_name: row.get(1)?,
        torrent_url: row.get(2)?,
        info_hash: row.get(3)?,
        file_index: u32::try_from(file_index).unwrap_or_default(),
        file_path: row.get(5)?,
        local_path: row.get(6)?,
        state: row.get(7)?,
        progress: row.get(8)?,
        message: row.get(9)?,
        updated_at: row.get(10)?,
    })
}

fn validate_firmware_download_receipt(receipt: &FirmwareDownloadReceipt) -> Result<()> {
    if receipt.source_id.trim().is_empty()
        || receipt.package_name.trim().is_empty()
        || !receipt.torrent_url.starts_with("https://")
        || receipt.info_hash.trim().is_empty()
        || receipt.file_path.trim().is_empty()
        || receipt.local_path.trim().is_empty()
    {
        bail!("firmware downloads require exact source, torrent, file, and path identities");
    }
    if !matches!(
        receipt.state.as_str(),
        "queued" | "downloading" | "paused" | "complete" | "imported" | "failed"
    ) || !(0.0..=1.0).contains(&receipt.progress)
        || !receipt.progress.is_finite()
    {
        bail!("firmware download state is invalid");
    }
    Ok(())
}

pub(crate) fn unix_timestamp() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .ok()
        .and_then(|duration| i64::try_from(duration.as_secs()).ok())
        .unwrap_or_default()
}

fn path_text(path: &Path) -> String {
    path.to_string_lossy().into_owned()
}

fn validate_download_source_kind(source_kind: &str) -> Result<()> {
    if matches!(source_kind, "minerva" | "manual_torrent") {
        Ok(())
    } else {
        bail!("unsupported download source kind {source_kind}")
    }
}

fn validate_manual_torrent_receipt(
    receipt: &ManualTorrentSourceReceipt,
    job: &DownloadJob,
) -> Result<()> {
    if job.source_kind != "manual_torrent"
        || receipt.game_uid.trim().is_empty()
        || receipt.game_uid != job.game_id
        || receipt.launchbox_db_id != job.launchbox_db_id
        || receipt.canonical_title.trim().is_empty()
        || receipt.canonical_title != job.title
        || receipt.platform.trim().is_empty()
        || receipt.platform != job.platform
        || receipt.source_file_name.trim().is_empty()
        || receipt.source_file_name.chars().count() > 1024
        || receipt.torrent_name.trim().is_empty()
        || receipt.torrent_name.chars().count() > 1024
        || receipt.torrent_sha256.len() != 64
        || !receipt
            .torrent_sha256
            .bytes()
            .all(|byte| byte.is_ascii_hexdigit())
        || receipt.info_hash.len() != 40
        || !receipt
            .info_hash
            .bytes()
            .all(|byte| byte.is_ascii_hexdigit())
        || !receipt.info_hash.eq_ignore_ascii_case(&job.info_hash)
        || job
            .torrent_file_index
            .is_some_and(|index| receipt.selected_file_index != index)
        || receipt.selected_file_path.trim().is_empty()
        || receipt.selected_file_path.chars().count() > 4096
        || receipt.selected_file_path != job.torrent_file_path
        || job.torrent_url
            != format!(
                "manual-torrent:sha256:{}",
                receipt.torrent_sha256.to_ascii_lowercase()
            )
        || receipt.queued_job_id != job.id
        || receipt.reviewed_at < 0
    {
        bail!("manual torrent provenance does not match the queued download");
    }
    crate::qbittorrent::safe_torrent_relative_path(&receipt.selected_file_path)?;
    Ok(())
}

fn u64_to_i64(value: u64) -> i64 {
    i64::try_from(value).unwrap_or(i64::MAX)
}

fn i64_to_u64(value: i64) -> u64 {
    u64::try_from(value).unwrap_or_default()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn store() -> (tempfile::TempDir, SettingsStore) {
        let directory = tempfile::tempdir().unwrap();
        let store = SettingsStore::at(directory.path().join("state.db")).unwrap();
        (directory, store)
    }

    #[test]
    fn defaults_and_settings_round_trip_without_a_plaintext_password_column() {
        let (_directory, store) = store();
        assert_eq!(store.load().unwrap(), AppSettings::default());

        let expected = AppSettings {
            qbittorrent_host: "media.local".into(),
            qbittorrent_port: 9443,
            qbittorrent_use_https: true,
            qbittorrent_username: "lunchbox".into(),
            rom_directory: PathBuf::from("/native/roms"),
            qbittorrent_container_rom_directory: "/client/roms".into(),
            torrent_library_directory: PathBuf::from("/native/downloads"),
            qbittorrent_container_torrent_library_directory: "/downloads".into(),
            download_entire_torrent: true,
            file_link_mode: "hardlink".into(),
            seeding_policy: "pause_after_import".into(),
            preferred_region: "Japan".into(),
            region_priority: vec!["Japan".into(), "Europe".into(), "USA".into()],
            version_preference: "original".into(),
            media_provider_priority: vec![
                "steamgriddb".into(),
                "local".into(),
                "launchbox".into(),
                "libretro".into(),
                "igdb".into(),
                "minerva".into(),
                "emumovies".into(),
                "screenscraper".into(),
                "websearch".into(),
            ],
            controller_mapping: ControllerMappingSettings {
                enabled: true,
                output_target: "ds5".into(),
                player_mappings: vec![ControllerPlayerMapping {
                    controller_id: Some("linux:054c:0ce6:living-room".into()),
                    profile_id: Some("custom:living-room".into()),
                    output_target: Some("xb360".into()),
                }],
                custom_profiles: vec![ControllerCustomProfile {
                    id: "custom:living-room".into(),
                    name: "Living room arcade".into(),
                    layout: "playstation".into(),
                    mappings: vec![ControllerButtonMapping {
                        source_button: "East".into(),
                        target_button: "West".into(),
                    }],
                }],
                ..ControllerMappingSettings::default()
            },
        };
        store.save(&expected).unwrap();
        assert_eq!(store.load().unwrap(), expected);

        let connection = Connection::open(store.path()).unwrap();
        let mut statement = connection
            .prepare("PRAGMA table_info(app_settings)")
            .unwrap();
        let columns = statement
            .query_map([], |row| row.get::<_, String>(1))
            .unwrap()
            .collect::<rusqlite::Result<Vec<_>>>()
            .unwrap();
        assert!(!columns.iter().any(|column| column.contains("password")));
    }

    #[test]
    fn controller_settings_reject_ambiguous_custom_profiles() {
        let mapping = ControllerMappingSettings {
            custom_profiles: vec![ControllerCustomProfile {
                id: "custom:ambiguous".to_owned(),
                name: "Ambiguous".to_owned(),
                layout: "xbox".to_owned(),
                mappings: vec![
                    ControllerButtonMapping {
                        source_button: "South".to_owned(),
                        target_button: "East".to_owned(),
                    },
                    ControllerButtonMapping {
                        source_button: "North".to_owned(),
                        target_button: "East".to_owned(),
                    },
                ],
            }],
            ..ControllerMappingSettings::default()
        };
        assert!(mapping.validate().is_err());
    }

    #[test]
    fn download_queue_is_durable_and_updates_in_place() {
        let (_directory, store) = store();
        let mut job = DownloadJob::queued(NewDownloadJob {
            game_id: "game-1".into(),
            launchbox_db_id: 42,
            title: "Game".into(),
            platform: "Platform".into(),
            source_kind: "minerva".into(),
            torrent_url: "https://example.invalid/game.torrent".into(),
            torrent_file_index: Some(7),
            torrent_file_path: "Platform/Game.zip".into(),
            info_hash: "abc123".into(),
            client_save_path: "/downloads/lunchbox".into(),
            local_download_path: PathBuf::from("/native/downloads/lunchbox"),
            local_target_path: PathBuf::from("/native/roms/Platform/Game.zip"),
            download_plan: String::new(),
        });
        store.upsert_job(&job).unwrap();
        job.state = "downloading".into();
        job.progress = 0.5;
        job.download_speed = 1024;
        job.updated_at += 1;
        store.upsert_job(&job).unwrap();

        assert_eq!(store.jobs().unwrap(), vec![job]);
    }

    #[test]
    fn manual_torrent_provenance_is_exact_idempotent_and_source_aware() {
        let (directory, store) = store();
        let downloaded = directory.path().join("downloads/Sample Game.rom");
        let installed = directory.path().join("roms/Sample Game.rom");
        fs::create_dir_all(installed.parent().unwrap()).unwrap();
        fs::write(&installed, b"sample rom").unwrap();
        let job = DownloadJob::queued(NewDownloadJob {
            game_id: "game-uuid".into(),
            launchbox_db_id: 42,
            title: "Sample Game".into(),
            platform: "Sample System".into(),
            source_kind: "manual_torrent".into(),
            torrent_url: format!("manual-torrent:sha256:{}", "a".repeat(64)),
            torrent_file_index: Some(7),
            torrent_file_path: "Pack/Sample Game.rom".into(),
            info_hash: "b".repeat(40),
            client_save_path: "/downloads/lunchbox".into(),
            local_download_path: downloaded,
            local_target_path: installed.clone(),
            download_plan: String::new(),
        });
        let mut receipt = ManualTorrentSourceReceipt {
            game_uid: job.game_id.clone(),
            launchbox_db_id: job.launchbox_db_id,
            canonical_title: job.title.clone(),
            platform: job.platform.clone(),
            source_file_name: "sample.torrent".into(),
            torrent_name: "Sample Pack".into(),
            torrent_sha256: "a".repeat(64),
            info_hash: job.info_hash.clone(),
            selected_file_index: 7,
            selected_file_path: job.torrent_file_path.clone(),
            queued_job_id: job.id.clone(),
            reviewed_at: 10,
        };
        store.record_manual_torrent_enqueue(&receipt, &job).unwrap();
        receipt.reviewed_at = 11;
        store.record_manual_torrent_enqueue(&receipt, &job).unwrap();
        assert_eq!(
            store.manual_torrent_sources().unwrap(),
            vec![receipt.clone()]
        );
        assert_eq!(store.jobs().unwrap(), vec![job.clone()]);

        store.record_installed(&job, &installed).unwrap();
        let import_source: String = store
            .connection()
            .unwrap()
            .query_row(
                "SELECT import_source FROM installed_games WHERE game_uid='game-uuid'",
                [],
                |row| row.get(0),
            )
            .unwrap();
        assert_eq!(import_source, "manual_torrent");

        let mut whole_torrent_job = job.clone();
        whole_torrent_job.id = "whole-torrent-job".into();
        whole_torrent_job.torrent_file_index = None;
        receipt.queued_job_id = whole_torrent_job.id.clone();
        receipt.reviewed_at = 12;
        store
            .record_manual_torrent_enqueue(&receipt, &whole_torrent_job)
            .unwrap();
        assert_eq!(
            store.manual_torrent_sources().unwrap(),
            vec![receipt.clone()]
        );

        let mut mismatched = store.manual_torrent_sources().unwrap().remove(0);
        mismatched.selected_file_path = "Pack/Other.rom".into();
        assert!(
            store
                .record_manual_torrent_enqueue(&mismatched, &job)
                .is_err()
        );
    }

    #[test]
    fn download_history_cleanup_never_removes_active_or_pending_import_jobs() {
        let (_directory, store) = store();
        let make_job = |game_id: &str, state: &str| {
            let mut job = DownloadJob::queued(NewDownloadJob {
                game_id: game_id.into(),
                launchbox_db_id: 42,
                title: game_id.into(),
                platform: "Platform".into(),
                source_kind: "minerva".into(),
                torrent_url: "https://example.invalid/game.torrent".into(),
                torrent_file_index: Some(7),
                torrent_file_path: format!("Platform/{game_id}.zip"),
                info_hash: format!("hash-{game_id}"),
                client_save_path: "/downloads/lunchbox".into(),
                local_download_path: PathBuf::from("/native/downloads/lunchbox"),
                local_target_path: PathBuf::from(format!("/native/roms/Platform/{game_id}.zip")),
                download_plan: String::new(),
            });
            job.state = state.into();
            job
        };
        let mut pause_pending = make_job("pause-pending", "imported");
        pause_pending.post_import_action = "pause_pending".into();
        let jobs = [
            make_job("queued", "queued"),
            make_job("downloading", "downloading"),
            make_job("paused", "paused"),
            make_job("complete", "complete"),
            make_job("imported", "imported"),
            make_job("cancelled", "cancelled"),
            pause_pending,
        ];
        for job in &jobs {
            store.upsert_job(job).unwrap();
        }

        assert!(!store.delete_job_record(&jobs[0].id).unwrap());
        assert!(store.delete_job_record(&jobs[5].id).unwrap());
        assert!(!store.delete_job_record(&jobs[6].id).unwrap());
        assert_eq!(store.clear_finished_job_records().unwrap(), 1);
        assert_eq!(
            store
                .jobs()
                .unwrap()
                .into_iter()
                .map(|job| job.state)
                .collect::<HashSet<_>>(),
            HashSet::from([
                "queued".to_owned(),
                "downloading".to_owned(),
                "paused".to_owned(),
                "complete".to_owned(),
                "imported".to_owned(),
            ])
        );
    }

    #[test]
    fn library_content_filters_default_safely_and_round_trip() {
        let (_directory, store) = store();
        assert_eq!(
            store.load_library_preferences().unwrap(),
            LibraryPreferences::default()
        );

        let expected = LibraryPreferences {
            hide_non_retail: false,
            hide_adult: true,
            artwork_type: "screenshot".into(),
            grid_zoom: 135,
        };
        store.save_library_preferences(&expected).unwrap();
        assert_eq!(store.load_library_preferences().unwrap(), expected);
    }

    #[test]
    fn sidebar_search_and_width_default_safely_and_survive_restart() {
        let (_directory, store) = store();
        assert_eq!(
            store.load_sidebar_preferences().unwrap(),
            SidebarPreferences::default()
        );

        let expected = SidebarPreferences {
            platform_search: "snes".into(),
            width: 332,
        };
        store.save_sidebar_preferences(&expected).unwrap();
        let reopened = SettingsStore::at(store.path()).unwrap();
        assert_eq!(reopened.load_sidebar_preferences().unwrap(), expected);

        assert!(
            store
                .save_sidebar_preferences(&SidebarPreferences {
                    platform_search: "x".repeat(81),
                    width: 332,
                })
                .is_err()
        );
        assert!(
            store
                .save_sidebar_preferences(&SidebarPreferences {
                    platform_search: String::new(),
                    width: 401,
                })
                .is_err()
        );
    }

    #[test]
    fn metadata_overrides_preserve_canonical_identity_and_explicitly_clear_fields() {
        let (_directory, store) = store();
        let canonical = GameMetadata {
            title: "Metroid".into(),
            description: "Canonical description".into(),
            rating: "4.5".into(),
            ..GameMetadata::default()
        };
        let effective = GameMetadata {
            title: "Metroid: Personal Edition".into(),
            description: String::new(),
            rating: "5".into(),
            ..canonical.clone()
        };
        let metadata = GameMetadataOverride::from_effective(&canonical, &effective);
        assert_eq!(metadata.description.as_deref(), Some(""));
        store
            .save_game_metadata_and_tags(
                "stable-metroid-uid",
                42,
                &canonical.title,
                "Nintendo Entertainment System",
                &metadata,
                "",
            )
            .unwrap();

        let reopened = SettingsStore::at(store.path()).unwrap();
        assert_eq!(
            reopened
                .game_metadata_override("stable-metroid-uid")
                .unwrap()
                .apply(&canonical),
            effective
        );
        assert_eq!(
            store
                .all_game_metadata_overrides()
                .unwrap()
                .get("stable-metroid-uid"),
            Some(&metadata)
        );
        let identity = Connection::open(store.path())
            .unwrap()
            .query_row(
                "SELECT launchbox_db_id, canonical_title, platform
                 FROM game_metadata_overrides WHERE game_uid=?1",
                ["stable-metroid-uid"],
                |row| {
                    Ok((
                        row.get::<_, i64>(0)?,
                        row.get::<_, String>(1)?,
                        row.get::<_, String>(2)?,
                    ))
                },
            )
            .unwrap();
        assert_eq!(
            identity,
            (
                42,
                "Metroid".to_owned(),
                "Nintendo Entertainment System".to_owned()
            )
        );

        store
            .save_game_metadata_and_tags(
                "stable-metroid-uid",
                42,
                &canonical.title,
                "Nintendo Entertainment System",
                &GameMetadataOverride::default(),
                "",
            )
            .unwrap();
        assert_eq!(
            store.game_metadata_override("stable-metroid-uid").unwrap(),
            GameMetadataOverride::default()
        );
    }

    #[test]
    fn metadata_overrides_validate_user_text_and_rating() {
        let (_directory, store) = store();
        let invalid_rating = GameMetadataOverride {
            rating: Some("5.1".into()),
            ..GameMetadataOverride::default()
        };
        assert!(
            store
                .save_game_metadata_and_tags("game", 0, "Game", "Platform", &invalid_rating, "",)
                .is_err()
        );
        let invalid_title = GameMetadataOverride {
            title: Some("x".repeat(501)),
            ..GameMetadataOverride::default()
        };
        assert!(
            store
                .save_game_metadata_and_tags("game", 0, "Game", "Platform", &invalid_title, "",)
                .is_err()
        );
        assert!(
            store
                .save_game_metadata_and_tags(
                    "",
                    0,
                    "Game",
                    "Platform",
                    &GameMetadataOverride {
                        title: Some("Custom".into()),
                        ..GameMetadataOverride::default()
                    },
                    "",
                )
                .is_err()
        );
    }

    #[test]
    fn game_tags_are_normalized_durable_and_saved_atomically_with_metadata() {
        let (_directory, store) = store();
        let metadata = GameMetadataOverride {
            notes: Some("Best on the couch".into()),
            ..GameMetadataOverride::default()
        };
        let stored = store
            .save_game_metadata_and_tags(
                "stable-mario-uid",
                140,
                "Super Mario Bros.",
                "Nintendo Entertainment System",
                &metadata,
                " Family , Couch   Co-op, family ",
            )
            .unwrap();
        assert_eq!(stored, vec!["Couch Co-op", "Family"]);
        assert_eq!(store.game_tags("stable-mario-uid").unwrap(), stored);

        let reopened = SettingsStore::at(store.path()).unwrap();
        let all = reopened.all_user_tags().unwrap();
        assert_eq!(all.game_tags["stable-mario-uid"], stored);
        assert_eq!(
            all.tags
                .iter()
                .map(|tag| (&tag.name, tag.game_count))
                .collect::<Vec<_>>(),
            vec![(&"Couch Co-op".to_owned(), 1), (&"Family".to_owned(), 1)]
        );
        let first_ids = all
            .tags
            .iter()
            .map(|tag| tag.id.clone())
            .collect::<Vec<_>>();

        reopened
            .save_game_metadata_and_tags(
                "stable-mario-uid",
                140,
                "Super Mario Bros.",
                "Nintendo Entertainment System",
                &GameMetadataOverride::default(),
                "family, RPG",
            )
            .unwrap();
        assert_eq!(
            reopened.game_tags("stable-mario-uid").unwrap(),
            vec!["Family", "RPG"]
        );
        let all = reopened.all_user_tags().unwrap();
        assert_eq!(
            all.tags
                .iter()
                .map(|tag| tag.name.as_str())
                .collect::<Vec<_>>(),
            vec!["Family", "RPG"]
        );
        assert_eq!(all.tags[0].id, first_ids[1]);
        assert_eq!(
            reopened.game_metadata_override("stable-mario-uid").unwrap(),
            GameMetadataOverride::default()
        );

        let invalid = reopened.save_game_metadata_and_tags(
            "stable-mario-uid",
            140,
            "Super Mario Bros.",
            "Nintendo Entertainment System",
            &metadata,
            "valid,",
        );
        assert!(invalid.is_err());
        assert_eq!(
            reopened.game_tags("stable-mario-uid").unwrap(),
            vec!["Family", "RPG"]
        );
        assert_eq!(
            reopened.game_metadata_override("stable-mario-uid").unwrap(),
            GameMetadataOverride::default()
        );
    }

    #[test]
    fn game_tag_validation_is_bounded_and_case_insensitive() {
        assert_eq!(
            parse_game_tags("Action, action, ACTION, Role Playing").unwrap(),
            vec!["Action", "Role Playing"]
        );
        assert!(parse_game_tags("one,,two").is_err());
        assert!(parse_game_tags(&"x".repeat(51)).is_err());
        assert!(
            parse_game_tags(
                &(0..33)
                    .map(|index| format!("tag {index}"))
                    .collect::<Vec<_>>()
                    .join(",")
            )
            .is_err()
        );
    }

    #[test]
    fn favorites_use_stable_game_identity_and_round_trip() {
        let (_directory, store) = store();
        assert!(store.favorite_game_ids().unwrap().is_empty());

        store
            .set_favorite("stable-game-uid", 42, "Game", "Platform", true)
            .unwrap();
        assert_eq!(
            store.favorite_game_ids().unwrap(),
            HashSet::from(["stable-game-uid".to_owned()])
        );

        store
            .set_favorite("stable-game-uid", 99, "Updated", "Platform", true)
            .unwrap();
        let connection = Connection::open(store.path()).unwrap();
        let metadata = connection
            .query_row(
                "SELECT launchbox_db_id, title FROM favorite_games WHERE game_uid=?1",
                ["stable-game-uid"],
                |row| Ok((row.get::<_, i64>(0)?, row.get::<_, String>(1)?)),
            )
            .unwrap();
        assert_eq!(metadata, (99, "Updated".to_owned()));

        store
            .set_favorite("stable-game-uid", 99, "Updated", "Platform", false)
            .unwrap();
        assert!(store.favorite_game_ids().unwrap().is_empty());
        assert!(store.set_favorite("", 0, "", "", true).is_err());
    }

    #[test]
    fn emulator_preferences_resolve_game_before_platform_and_preserve_runtime() {
        let (_directory, store) = store();
        store
            .set_platform_emulator_preference(
                "Nintendo Entertainment System",
                "nestopia-id",
                "standalone",
                "",
            )
            .unwrap();
        assert_eq!(
            store
                .emulator_preference("game-one", "nintendo entertainment system")
                .unwrap(),
            Some(EmulatorPreference {
                emulator_id: "nestopia-id".into(),
                runtime_kind: "standalone".into(),
                core_name: String::new(),
                scope: "platform".into(),
            })
        );

        store
            .set_game_emulator_preference("game-one", "mesen-id", "retroarch", "mesen")
            .unwrap();
        assert_eq!(
            store
                .emulator_preference("game-one", "Nintendo Entertainment System")
                .unwrap(),
            Some(EmulatorPreference {
                emulator_id: "mesen-id".into(),
                runtime_kind: "retroarch".into(),
                core_name: "mesen".into(),
                scope: "game".into(),
            })
        );

        store.clear_game_emulator_preference("game-one").unwrap();
        assert_eq!(
            store
                .emulator_preference("game-one", "Nintendo Entertainment System")
                .unwrap()
                .unwrap()
                .scope,
            "platform"
        );
        store
            .clear_platform_emulator_preference("Nintendo Entertainment System")
            .unwrap();
        assert_eq!(
            store
                .emulator_preference("game-one", "Nintendo Entertainment System")
                .unwrap(),
            None
        );
        assert!(
            store
                .set_game_emulator_preference("game-one", "mesen-id", "retroarch", "")
                .is_err()
        );
    }

    #[test]
    fn launch_profiles_resolve_each_field_by_exact_scope_precedence() {
        let (_directory, store) = store();
        store
            .set_emulator_launch_profile(&EmulatorLaunchProfile {
                scope_kind: "global".into(),
                emulator_id: "mesen-id".into(),
                runtime_kind: "retroarch".into(),
                core_name: "mesen".into(),
                extra_arguments: "--verbose".into(),
                command_template: "--global %{core} %f".into(),
                ..EmulatorLaunchProfile::default()
            })
            .unwrap();
        store
            .set_emulator_launch_profile(&EmulatorLaunchProfile {
                scope_kind: "platform".into(),
                scope_key: "Nintendo Entertainment System".into(),
                emulator_id: "mesen-id".into(),
                runtime_kind: "retroarch".into(),
                core_name: "mesen".into(),
                extra_arguments: "--latency 1".into(),
                ..EmulatorLaunchProfile::default()
            })
            .unwrap();
        store
            .set_emulator_launch_profile(&EmulatorLaunchProfile {
                scope_kind: "game".into(),
                scope_key: "game-one".into(),
                emulator_id: "mesen-id".into(),
                runtime_kind: "retroarch".into(),
                core_name: "mesen".into(),
                command_template: "--game %{core} %f".into(),
                ..EmulatorLaunchProfile::default()
            })
            .unwrap();

        assert_eq!(
            store
                .resolve_launch_customization(
                    "game-one",
                    "nintendo entertainment system",
                    "mesen-id",
                    "retroarch",
                    "mesen",
                )
                .unwrap(),
            ResolvedLaunchCustomization {
                extra_arguments: "--latency 1".into(),
                command_template: "--game %{core} %f".into(),
                argument_scope: "platform".into(),
                template_scope: "game".into(),
            }
        );
        let profiles = store.emulator_launch_profiles().unwrap();
        assert_eq!(profiles.len(), 3);
        assert!(profiles.iter().any(|profile| {
            profile.scope_kind == "platform"
                && profile.scope_key == "nintendo-entertainment-system"
                && profile.extra_arguments == "--latency 1"
        }));
        assert!(
            store
                .set_emulator_launch_profile(&EmulatorLaunchProfile {
                    scope_kind: "game".into(),
                    scope_key: "game-one".into(),
                    emulator_id: "mesen-id".into(),
                    runtime_kind: "retroarch".into(),
                    core_name: "mesen".into(),
                    extra_arguments: "'unterminated".into(),
                    ..EmulatorLaunchProfile::default()
                })
                .is_err()
        );
        assert!(
            store
                .set_emulator_launch_profile(&EmulatorLaunchProfile {
                    scope_kind: "global".into(),
                    scope_key: "wrong-key".into(),
                    emulator_id: "mesen-id".into(),
                    runtime_kind: "retroarch".into(),
                    core_name: "mesen".into(),
                    extra_arguments: "--test".into(),
                    ..EmulatorLaunchProfile::default()
                })
                .is_err()
        );

        store
            .clear_emulator_launch_profile("game", "game-one", "mesen-id", "retroarch", "mesen")
            .unwrap();
        assert_eq!(
            store
                .resolve_launch_customization(
                    "game-one",
                    "Nintendo Entertainment System",
                    "mesen-id",
                    "retroarch",
                    "mesen",
                )
                .unwrap()
                .template_scope,
            "global"
        );
    }

    #[test]
    fn managed_emulator_receipts_are_exact_idempotent_and_removable() {
        let (_directory, store) = store();
        store
            .record_managed_emulator_install(
                "mesen-id",
                "linux",
                "appimage",
                "SourMesen/Mesen2",
                "/managed/mesen",
            )
            .unwrap();
        store
            .record_managed_emulator_install(
                "mesen-id",
                "linux",
                "appimage",
                "SourMesen/Mesen2",
                "/managed/mesen/current",
            )
            .unwrap();

        let installs = store.managed_emulator_installs().unwrap();
        assert_eq!(installs.len(), 1);
        assert_eq!(installs[0].package_id, "SourMesen/Mesen2");
        assert_eq!(installs[0].install_path, "/managed/mesen/current");

        store
            .remove_managed_emulator_install("linux", "appimage", "SourMesen/Mesen2")
            .unwrap();
        assert!(store.managed_emulator_installs().unwrap().is_empty());
        assert!(
            store
                .record_managed_emulator_install("", "linux", "snap", "bad", "")
                .is_err()
        );
    }

    #[test]
    fn firmware_receipts_are_exact_and_idempotent() {
        let (_directory, store) = store();
        let mut package = FirmwarePackageReceipt {
            source_id: "minerva:retroarch-system-files".into(),
            package_name: "Sony - PlayStation (SwanStation).zip".into(),
            archive_path: "/store/firmware/playstation.zip".into(),
            extracted_root: "/store/firmware/playstation".into(),
            sha256: "a".repeat(64),
            file_count: 1,
            imported_at: 10,
        };
        let file = FirmwareFileReceipt {
            source_id: package.source_id.clone(),
            package_name: package.package_name.clone(),
            relative_path: "scph5501.bin".into(),
            store_path: "/store/firmware/playstation/scph5501.bin".into(),
            sha256: "b".repeat(64),
            file_size: 512 * 1024,
        };
        store
            .record_firmware_package(&package, std::slice::from_ref(&file))
            .unwrap();
        package.imported_at = 11;
        store
            .record_firmware_package(&package, std::slice::from_ref(&file))
            .unwrap();

        let install = FirmwareInstallReceipt {
            rule_key: "duckstation:DuckStation:Sony Playstation".into(),
            runtime_path: "/runtime/bios".into(),
            target_path: "/runtime/bios".into(),
            source_id: package.source_id.clone(),
            package_name: package.package_name.clone(),
            synced_at: 12,
        };
        store.record_firmware_install(&install).unwrap();
        store.record_firmware_install(&install).unwrap();

        assert_eq!(store.firmware_packages().unwrap(), vec![package]);
        assert_eq!(
            store
                .firmware_files(
                    "minerva:retroarch-system-files",
                    "Sony - PlayStation (SwanStation).zip"
                )
                .unwrap(),
            vec![file.clone()]
        );
        assert_eq!(store.firmware_installs().unwrap(), vec![install]);
        let mut download = FirmwareDownloadReceipt {
            source_id: "minerva:retroarch-system-files".into(),
            package_name: "Sony - PlayStation (SwanStation).zip".into(),
            torrent_url: "https://example.invalid/firmware.torrent".into(),
            info_hash: "abc123".into(),
            file_index: 42,
            file_path: "firmware/playstation.zip".into(),
            local_path: "/downloads/firmware/playstation.zip".into(),
            state: "downloading".into(),
            progress: 0.5,
            message: "Downloading".into(),
            updated_at: 13,
        };
        store.record_firmware_download(&download).unwrap();
        download.state = "complete".into();
        download.progress = 1.0;
        download.updated_at = 14;
        store.record_firmware_download(&download).unwrap();
        assert_eq!(
            store.firmware_downloads_for_info_hash("abc123").unwrap(),
            vec![download]
        );
        assert!(
            store
                .record_firmware_package(
                    &FirmwarePackageReceipt {
                        sha256: "bad".into(),
                        ..store.firmware_packages().unwrap().remove(0)
                    },
                    std::slice::from_ref(&file)
                )
                .is_err()
        );
        let connection = Connection::open(store.path()).unwrap();
        assert_eq!(
            connection
                .query_row("SELECT count(*) FROM firmware_files", [], |row| {
                    row.get::<_, i64>(0)
                })
                .unwrap(),
            1
        );
    }

    #[test]
    fn play_sessions_track_real_duration_and_finalize_once() {
        let (_directory, store) = store();
        assert!(store.play_activity("game-1").unwrap().is_none());
        store
            .set_completion_state(
                "game-1",
                42,
                "Metroid",
                "Nintendo Entertainment System",
                "not_started",
            )
            .unwrap();
        let session = store
            .begin_play_session(
                "game-1",
                42,
                "Metroid",
                "Nintendo Entertainment System",
                "RetroArch · fceumm",
            )
            .unwrap();
        assert!(
            store
                .finish_play_session(&session.session_id, 93, "completed")
                .unwrap()
        );
        assert!(
            !store
                .finish_play_session(&session.session_id, 93, "completed")
                .unwrap()
        );

        let activity = store.play_activity("game-1").unwrap().unwrap();
        assert_eq!(activity.play_count, 1);
        assert_eq!(activity.total_play_time_seconds, 93);
        assert_eq!(activity.completion_state, "in_progress");
        assert!(activity.first_played_at > 0);
        assert!(activity.last_played_at >= activity.first_played_at);
        assert_eq!(store.all_play_activity().unwrap(), vec![activity]);

        assert!(
            store
                .set_completion_state(
                    "game-1",
                    42,
                    "Metroid",
                    "Nintendo Entertainment System",
                    "finished",
                )
                .is_err()
        );
        assert!(
            store
                .finish_play_session(&session.session_id, 0, "unknown")
                .is_err()
        );
    }

    #[test]
    fn media_progress_is_scoped_to_the_exact_video_and_replaces_stale_media() {
        let (_directory, store) = store();
        assert!(
            store
                .media_playback_progress("game-1", "sha256:first")
                .unwrap()
                .is_none()
        );
        let saved = store
            .save_media_playback_progress("game-1", "sha256:first", 31_500, 90_000)
            .unwrap();
        assert_eq!(saved.position_ms, 31_500);
        assert_eq!(
            store
                .media_playback_progress("game-1", "sha256:first")
                .unwrap(),
            Some(saved)
        );

        store
            .save_media_playback_progress("game-1", "sha256:replacement", 2_000, 80_000)
            .unwrap();
        assert!(
            store
                .media_playback_progress("game-1", "sha256:first")
                .unwrap()
                .is_none()
        );
        assert!(
            store
                .save_media_playback_progress("game-1", "sha256:replacement", -1, 80_000)
                .is_err()
        );
    }

    #[test]
    fn reviewed_media_provider_links_use_exact_catalog_and_provider_ids() {
        let (_directory, store) = store();
        assert!(
            store
                .media_provider_game_link("steamgriddb", 42)
                .unwrap()
                .is_none()
        );
        let first = MediaProviderGameLink {
            provider: "steamgriddb".into(),
            launchbox_db_id: 42,
            provider_game_id: "101".into(),
            provider_game_name: "Exact Game".into(),
            reviewed_at: 100,
        };
        store.save_media_provider_game_link(&first).unwrap();
        assert_eq!(
            store.media_provider_game_link("steamgriddb", 42).unwrap(),
            Some(first)
        );

        let replacement = MediaProviderGameLink {
            provider: "steamgriddb".into(),
            launchbox_db_id: 42,
            provider_game_id: "202".into(),
            provider_game_name: "Reviewed Replacement".into(),
            reviewed_at: 200,
        };
        store.save_media_provider_game_link(&replacement).unwrap();
        assert_eq!(
            store.media_provider_game_link("steamgriddb", 42).unwrap(),
            Some(replacement)
        );
        assert!(
            store
                .save_media_provider_game_link(&MediaProviderGameLink {
                    provider: "steamgriddb".into(),
                    launchbox_db_id: 0,
                    provider_game_id: String::new(),
                    provider_game_name: String::new(),
                    reviewed_at: 0,
                })
                .is_err()
        );
    }

    #[test]
    fn media_transfers_are_durable_scoped_and_share_remote_jobs() {
        let (_directory, store) = store();
        let mut first = MediaTransfer {
            game_uid: "game-one".into(),
            launchbox_db_id: 42,
            title: "First Game".into(),
            platform: "Sega Pico".into(),
            media_kind: "manual".into(),
            provider: "minerva".into(),
            transport: "torrent".into(),
            source_url: "https://example.invalid/manuals.torrent".into(),
            remote_job_id: "ABCDEF012345".into(),
            source_item_index: 3,
            source_item_path: "Manuals/First Game.zip".into(),
            local_download_path: PathBuf::from("/downloads/Manuals/First Game.zip"),
            local_target_path: PathBuf::from("/media/game-one/minerva/manual.zip"),
            state: "queued".into(),
            progress: 0.0,
            download_speed: 0,
            downloaded_bytes: 0,
            total_bytes: 1_024,
            message: "Queued".into(),
            created_at: 10,
            updated_at: 10,
        };
        let second = MediaTransfer {
            game_uid: "game-two".into(),
            source_item_index: 7,
            source_item_path: "Manuals/Second Game.zip".into(),
            local_download_path: PathBuf::from("/downloads/Manuals/Second Game.zip"),
            local_target_path: PathBuf::from("/media/game-two/minerva/manual.zip"),
            created_at: 11,
            updated_at: 11,
            ..first.clone()
        };
        store.upsert_media_transfer(&first).unwrap();
        store.upsert_media_transfer(&second).unwrap();
        assert_eq!(
            store.media_transfer("game-one", "manual").unwrap(),
            Some(first.clone())
        );
        assert_eq!(
            store
                .media_transfers_for_remote_job("minerva", "abcdef012345")
                .unwrap(),
            vec![first.clone(), second]
        );

        first.state = "downloading".into();
        first.progress = 0.5;
        first.download_speed = 2_048;
        first.downloaded_bytes = 512;
        first.message = "Downloading".into();
        first.updated_at = 12;
        store.upsert_media_transfer(&first).unwrap();
        assert_eq!(
            store.media_transfer("game-one", "manual").unwrap(),
            Some(first)
        );

        let mut invalid = store.media_transfer("game-two", "manual").unwrap().unwrap();
        invalid.progress = f64::NAN;
        assert!(store.upsert_media_transfer(&invalid).is_err());
        assert!(store.media_transfer("game-one", "soundtrack").is_err());
    }

    #[test]
    fn named_collections_preserve_membership_and_cascade_on_delete() {
        let (_directory, store) = store();
        let collection = store
            .create_collection("  Couch Co-op  ", " Games for the living room ")
            .unwrap();
        assert_eq!(collection.name, "Couch Co-op");
        assert_eq!(collection.description, "Games for the living room");

        store
            .set_collection_membership(&collection.id, "game-one", true)
            .unwrap();
        store
            .set_collection_membership(&collection.id, "game-two", true)
            .unwrap();
        store
            .set_collection_membership(&collection.id, "game-one", true)
            .unwrap();
        let loaded = store.user_collections().unwrap();
        assert_eq!(loaded.collections[0].game_count, 2);
        assert_eq!(
            loaded.members[&collection.id],
            vec!["game-one".to_owned(), "game-two".to_owned()]
        );

        store
            .update_collection(&collection.id, "Arcade Night", "Favorites")
            .unwrap();
        assert_eq!(
            store.user_collections().unwrap().collections[0].name,
            "Arcade Night"
        );
        assert!(
            store
                .create_collection("arcade night", "duplicate")
                .is_err()
        );

        store
            .set_collection_membership(&collection.id, "game-one", false)
            .unwrap();
        assert_eq!(
            store.user_collections().unwrap().collections[0].game_count,
            1
        );
        store.delete_collection(&collection.id).unwrap();
        assert_eq!(
            store.user_collections().unwrap(),
            UserCollections::default()
        );
    }

    #[test]
    fn smart_collections_and_manual_order_are_durable_and_separate() {
        let (_directory, store) = store();
        let manual = store.create_collection("Arcade order", "").unwrap();
        store
            .set_collection_memberships(
                &manual.id,
                &["game-one".into(), "game-two".into(), "game-three".into()],
                true,
            )
            .unwrap();
        let reordered = store
            .move_collection_game(&manual.id, "game-three", 0)
            .unwrap();
        assert_eq!(reordered, vec!["game-three", "game-one", "game-two"]);

        let rules = SmartCollectionRules {
            platform: "Arcade".into(),
            favorite: "favorite".into(),
            ..SmartCollectionRules::default()
        };
        let smart = store
            .create_smart_collection("Favorite arcade", "Live shelf", rules.clone())
            .unwrap();
        assert_eq!(smart.kind, "smart");
        assert_eq!(smart.rules, Some(rules.clone()));
        assert!(
            store
                .set_collection_membership(&smart.id, "game-one", true)
                .is_err()
        );
        let loaded = store.user_collections().unwrap();
        assert_eq!(loaded.members[&manual.id], reordered);
        assert_eq!(loaded.collections[1].rules, Some(rules));
    }

    #[test]
    fn legacy_collection_rows_migrate_as_ordered_manual_collections() {
        let directory = tempfile::tempdir().unwrap();
        let path = directory.path().join("legacy-state.db");
        let connection = Connection::open(&path).unwrap();
        connection
            .execute_batch(
                "CREATE TABLE user_collections (
                     id TEXT PRIMARY KEY,
                     name TEXT NOT NULL COLLATE NOCASE UNIQUE,
                     description TEXT NOT NULL,
                     created_at INTEGER NOT NULL,
                     updated_at INTEGER NOT NULL
                 );
                 CREATE TABLE user_collection_games (
                     collection_id TEXT NOT NULL REFERENCES user_collections(id) ON DELETE CASCADE,
                     game_uid TEXT NOT NULL,
                     sort_order INTEGER NOT NULL,
                     added_at INTEGER NOT NULL,
                     PRIMARY KEY (collection_id, game_uid)
                 );
                 INSERT INTO user_collections VALUES ('legacy', 'Legacy shelf', '', 1, 1);
                 INSERT INTO user_collection_games VALUES ('legacy', 'second', 20, 1);
                 INSERT INTO user_collection_games VALUES ('legacy', 'first', 10, 1);",
            )
            .unwrap();
        drop(connection);

        let collections = SettingsStore::at(&path)
            .unwrap()
            .user_collections()
            .unwrap();
        assert_eq!(collections.collections[0].kind, "manual");
        assert_eq!(collections.collections[0].rules, None);
        assert_eq!(collections.members["legacy"], vec!["first", "second"]);
    }

    #[test]
    fn collection_validation_rejects_empty_and_unbounded_fields() {
        let (_directory, store) = store();
        assert!(store.create_collection(" ", "").is_err());
        assert!(store.create_collection(&"x".repeat(101), "").is_err());
        let collection = store.create_collection("Valid", "").unwrap();
        assert!(
            store
                .update_collection(&collection.id, "Valid", &"x".repeat(1001))
                .is_err()
        );
        assert!(
            store
                .set_collection_membership(&collection.id, "", true)
                .is_err()
        );
    }

    #[test]
    fn rejects_unknown_link_modes() {
        let settings = AppSettings {
            file_link_mode: "magic".into(),
            ..AppSettings::default()
        };
        assert!(settings.validate().is_err());
    }

    #[test]
    fn rejects_unknown_seeding_policies() {
        let settings = AppSettings {
            seeding_policy: "delete_everything".into(),
            ..AppSettings::default()
        };
        assert!(settings.validate().is_err());
    }

    #[test]
    fn rejects_unknown_release_preferences() {
        let settings = AppSettings {
            preferred_region: "Moon".into(),
            ..AppSettings::default()
        };
        assert!(settings.validate().is_err());

        let settings = AppSettings {
            region_priority: vec!["Moon".into()],
            ..AppSettings::default()
        };
        assert!(settings.validate().is_err());

        let settings = AppSettings {
            region_priority: vec!["UK".into(), "United Kingdom".into()],
            ..AppSettings::default()
        };
        assert!(settings.validate().is_err());

        let settings = AppSettings {
            version_preference: "random".into(),
            ..AppSettings::default()
        };
        assert!(settings.validate().is_err());
    }

    #[test]
    fn rejects_incomplete_or_duplicate_media_provider_priority() {
        let settings = AppSettings {
            media_provider_priority: vec!["local".into()],
            ..AppSettings::default()
        };
        assert!(settings.validate().is_err());

        let mut duplicate = crate::media::default_provider_priority();
        duplicate[1] = "local".into();
        let settings = AppSettings {
            media_provider_priority: duplicate,
            ..AppSettings::default()
        };
        assert!(settings.validate().is_err());
    }

    #[test]
    fn older_state_database_migrates_to_safe_seeding_defaults() {
        let directory = tempfile::tempdir().unwrap();
        let path = directory.path().join("state.db");
        let connection = Connection::open(&path).unwrap();
        connection
            .execute_batch(
                "CREATE TABLE app_settings (
                     id INTEGER PRIMARY KEY,
                     qbittorrent_host TEXT NOT NULL,
                     qbittorrent_port INTEGER NOT NULL,
                     qbittorrent_use_https INTEGER NOT NULL,
                     qbittorrent_username TEXT NOT NULL,
                     rom_directory TEXT NOT NULL,
                     qbittorrent_container_rom_directory TEXT NOT NULL,
                     torrent_library_directory TEXT NOT NULL,
                     qbittorrent_container_torrent_library_directory TEXT NOT NULL,
                     download_entire_torrent INTEGER NOT NULL,
                     file_link_mode TEXT NOT NULL
                 );
                 INSERT INTO app_settings VALUES (
                     1, '127.0.0.1', 8080, 0, '', '', '', '', '', 0, 'symlink'
                 );
                 CREATE TABLE download_jobs (
                     id TEXT PRIMARY KEY,
                     game_id TEXT NOT NULL,
                     launchbox_db_id INTEGER NOT NULL DEFAULT 0,
                     title TEXT NOT NULL,
                     platform TEXT NOT NULL,
                     torrent_url TEXT NOT NULL,
                     torrent_file_index INTEGER,
                     torrent_file_path TEXT NOT NULL,
                     info_hash TEXT NOT NULL,
                     client_save_path TEXT NOT NULL,
                     local_download_path TEXT NOT NULL,
                     local_target_path TEXT NOT NULL,
                     state TEXT NOT NULL,
                     progress REAL NOT NULL,
                     download_speed INTEGER NOT NULL,
                     downloaded_bytes INTEGER NOT NULL,
                     total_bytes INTEGER NOT NULL,
                     message TEXT NOT NULL,
                     created_at INTEGER NOT NULL,
                     updated_at INTEGER NOT NULL
                 );",
            )
            .unwrap();
        drop(connection);

        let store = SettingsStore::at(&path).unwrap();
        let settings = store.load().unwrap();
        assert_eq!(settings.seeding_policy, "follow_client");
        assert_eq!(settings.preferred_region, "USA");
        assert!(settings.region_priority.is_empty());
        assert_eq!(settings.version_preference, "latest");
        assert_eq!(
            settings.media_provider_priority,
            crate::media::default_provider_priority()
        );
        let connection = Connection::open(&path).unwrap();
        assert!(column_exists(&connection, "download_jobs", "post_import_action").unwrap());
        assert!(column_exists(&connection, "download_jobs", "download_plan").unwrap());
        assert!(column_exists(&connection, "download_jobs", "source_kind").unwrap());
        assert!(column_exists(&connection, "app_settings", "preferred_region").unwrap());
        assert!(column_exists(&connection, "app_settings", "version_preference").unwrap());
        assert!(column_exists(&connection, "app_settings", "region_priority_json").unwrap());
        assert!(column_exists(&connection, "app_settings", "controller_mapping_json").unwrap());
        assert!(
            column_exists(&connection, "app_settings", "media_provider_priority_json").unwrap()
        );
        let prepared_table: Option<i64> = connection
            .query_row(
                "SELECT 1 FROM sqlite_master WHERE type='table' AND name='prepared_game_installs'",
                [],
                |row| row.get(0),
            )
            .optional()
            .unwrap();
        assert_eq!(prepared_table, Some(1));
        for table in ["game_activity", "play_sessions", "manual_torrent_sources"] {
            let migrated: Option<i64> = connection
                .query_row(
                    "SELECT 1 FROM sqlite_master WHERE type='table' AND name=?1",
                    [table],
                    |row| row.get(0),
                )
                .optional()
                .unwrap();
            assert_eq!(migrated, Some(1), "missing migrated table {table}");
        }
    }

    #[test]
    fn older_local_collection_schema_adds_durable_archive_member_identity() {
        let directory = tempfile::tempdir().unwrap();
        let path = directory.path().join("state.db");
        let connection = Connection::open(&path).unwrap();
        connection
            .execute_batch(
                "CREATE TABLE local_rom_files (
                     id TEXT PRIMARY KEY,
                     root_id TEXT NOT NULL,
                     path_display TEXT NOT NULL,
                     path_bytes BLOB NOT NULL,
                     path_encoding TEXT NOT NULL,
                     relative_path_display TEXT NOT NULL,
                     relative_path_bytes BLOB NOT NULL,
                     relative_path_encoding TEXT NOT NULL,
                     file_name TEXT NOT NULL,
                     display_title TEXT NOT NULL,
                     platform TEXT NOT NULL,
                     file_size INTEGER NOT NULL,
                     modified_unix_ns INTEGER,
                     crc32 TEXT NOT NULL,
                     md5 TEXT NOT NULL,
                     sha1 TEXT NOT NULL,
                     game_uid TEXT,
                     launchbox_db_id INTEGER NOT NULL DEFAULT 0,
                     matched_title TEXT,
                     match_state TEXT NOT NULL,
                     match_method TEXT NOT NULL,
                     included INTEGER NOT NULL,
                     availability TEXT NOT NULL,
                     imported_at INTEGER NOT NULL
                 );
                 CREATE UNIQUE INDEX local_rom_files_root_relative
                     ON local_rom_files(root_id, relative_path_bytes);
                 INSERT INTO local_rom_files VALUES (
                     'old-file', 'root', '/roms/Game.zip', X'01', 'unix_bytes',
                     'Game.zip', X'02', 'unix_bytes', 'Game.zip', 'Game', 'System',
                     10, NULL, '', '', '', NULL, 0, NULL, 'inventory_only',
                     'not checked', 1, 'present', 1
                 );",
            )
            .unwrap();
        drop(connection);

        SettingsStore::at(&path).unwrap();
        let connection = Connection::open(&path).unwrap();
        for column in [
            "archive_member",
            "source_archive_path_display",
            "source_archive_path_bytes",
            "source_archive_path_encoding",
        ] {
            assert!(column_exists(&connection, "local_rom_files", column).unwrap());
        }
        let migrated = connection
            .query_row(
                "SELECT archive_member, source_archive_path_display,
                        length(source_archive_path_bytes), source_archive_path_encoding
                 FROM local_rom_files WHERE id='old-file'",
                [],
                |row| {
                    Ok((
                        row.get::<_, String>(0)?,
                        row.get::<_, String>(1)?,
                        row.get::<_, i64>(2)?,
                        row.get::<_, String>(3)?,
                    ))
                },
            )
            .unwrap();
        assert_eq!(migrated, (String::new(), String::new(), 0, String::new()));
        let index_sql = connection
            .query_row(
                "SELECT sql FROM sqlite_schema
                 WHERE type='index' AND name='local_rom_files_root_relative'",
                [],
                |row| row.get::<_, String>(0),
            )
            .unwrap();
        assert!(index_sql.contains("archive_member"));
        assert!(local_rom_match_state_allows_reviewed(&connection).unwrap());
        connection
            .execute(
                "UPDATE local_rom_files SET match_state='reviewed' WHERE id='old-file'",
                [],
            )
            .unwrap();
        drop(connection);

        SettingsStore::at(&path).unwrap();
        let connection = Connection::open(path).unwrap();
        let migrated_state = connection
            .query_row(
                "SELECT match_state FROM local_rom_files WHERE id='old-file'",
                [],
                |row| row.get::<_, String>(0),
            )
            .unwrap();
        assert_eq!(migrated_state, "reviewed");
    }

    #[test]
    fn concurrent_state_initialization_does_not_race_the_schema_migration() {
        let directory = tempfile::tempdir().unwrap();
        let path = directory.path().join("state.db");
        let barrier = std::sync::Arc::new(std::sync::Barrier::new(8));
        let workers = (0..8)
            .map(|_| {
                let path = path.clone();
                let barrier = barrier.clone();
                std::thread::spawn(move || {
                    barrier.wait();
                    let store = SettingsStore::at(path).unwrap();
                    store.load().unwrap()
                })
            })
            .collect::<Vec<_>>();
        for worker in workers {
            assert_eq!(worker.join().unwrap(), AppSettings::default());
        }
    }

    #[test]
    fn igdb_credentials_round_trip_as_one_keyring_payload() {
        let encoded = serde_json::to_string(&IgdbCredentials {
            client_id: "native-client".to_owned(),
            client_secret: "native-secret".to_owned(),
        })
        .unwrap();
        let decoded: IgdbCredentials = serde_json::from_str(&encoded).unwrap();
        assert_eq!(decoded.client_id, "native-client");
        assert_eq!(decoded.client_secret, "native-secret");
        assert!(!encoded.contains("steamgriddb"));
    }
}
