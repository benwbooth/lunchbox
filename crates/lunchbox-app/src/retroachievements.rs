//! Account tokens stay in the OS vault. RetroArch owns hashing, achievement
//! evaluation and submissions; Lunchbox never impersonates an unpatched ROM.
use crate::{
    emulator::{EmulatorExecutable, EmulatorRuntimeKind, LaunchPlan, RomEmulatorOption},
    settings::SettingsStore,
};
use anyhow::{Context, Result, ensure};
use rusqlite::OptionalExtension;
use serde::{Deserialize, Serialize};
use std::{fs, io::Write, path::Path, time::Duration};

const ACCOUNT: &str = "retroachievements-login-token";
const GLOBAL: &str = "global";

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Mode {
    #[default]
    Emulator,
    Off,
    Casual,
    Hardcore,
}
impl Mode {
    pub fn parse(value: &str) -> Result<Self> {
        match value {
            "emulator" => Ok(Self::Emulator),
            "off" => Ok(Self::Off),
            "casual" => Ok(Self::Casual),
            "hardcore" => Ok(Self::Hardcore),
            _ => anyhow::bail!("Unknown achievement mode"),
        }
    }
    pub fn key(self) -> &'static str {
        match self {
            Self::Emulator => "emulator",
            Self::Off => "off",
            Self::Casual => "casual",
            Self::Hardcore => "hardcore",
        }
    }
}
fn scope(game: &str) -> String {
    if game.is_empty() {
        GLOBAL.into()
    } else {
        format!("game:{game}")
    }
}
pub fn preference(store: &SettingsStore, game: &str) -> Result<Option<Mode>> {
    let value = store
        .connection()?
        .query_row(
            "SELECT mode FROM achievement_preferences WHERE scope=?1",
            [scope(game)],
            |r| r.get::<_, String>(0),
        )
        .optional()?;
    value.map(|value| Mode::parse(&value)).transpose()
}
pub fn save_preference(store: &SettingsStore, game: &str, value: &str) -> Result<()> {
    if value == "inherit" && !game.is_empty() {
        store.connection()?.execute(
            "DELETE FROM achievement_preferences WHERE scope=?1",
            [scope(game)],
        )?;
    } else {
        let mode = Mode::parse(value)?;
        store.connection()?.execute("INSERT INTO achievement_preferences(scope,mode) VALUES(?1,?2) ON CONFLICT(scope) DO UPDATE SET mode=excluded.mode", [scope(game), mode.key().to_owned()])?;
    }
    Ok(())
}

// Deliberately not Debug: authentication material must not appear in logs.
#[derive(Serialize, Deserialize)]
pub struct Credentials {
    pub username: String,
    token: String,
}
impl Credentials {
    fn validate(&self) -> Result<()> {
        for value in [&self.username, &self.token] {
            ensure!(
                !value.is_empty()
                    && value.len() <= 256
                    && !value
                        .chars()
                        .any(|c| c.is_control() || c == '"' || c == '\\'),
                "Invalid RetroAchievements credentials; please sign in again"
            );
        }
        Ok(())
    }
}
pub fn credentials() -> Result<Option<Credentials>> {
    crate::settings::load_secret(ACCOUNT, "RetroAchievements token")?
        .map(|value| {
            let credentials: Credentials =
                serde_json::from_str(&value).context("Reading RetroAchievements account")?;
            credentials.validate()?;
            Ok(credentials)
        })
        .transpose()
}
pub fn sign_out() -> Result<()> {
    crate::settings::save_secret(ACCOUNT, "", "RetroAchievements token")
}
pub fn sign_in(username: &str, password: &str) -> Result<String> {
    ensure!(
        !username.trim().is_empty() && !password.is_empty(),
        "Enter your RetroAchievements username and password"
    );
    // Official rcheevos login2 protocol, HTTPS POST only. No redirects may
    // forward the password to another origin, and no response body is logged.
    let agent: ureq::Agent = ureq::Agent::config_builder()
        .timeout_connect(Some(Duration::from_secs(8)))
        .timeout_global(Some(Duration::from_secs(20)))
        .max_redirects(0)
        .build()
        .into();
    let mut response = agent
        .post("https://retroachievements.org/dorequest.php")
        .header(
            "User-Agent",
            concat!("Lunchbox/", env!("CARGO_PKG_VERSION")),
        )
        .send_form([("r", "login2"), ("u", username.trim()), ("p", password)])
        .map_err(|_| {
            anyhow::anyhow!(
                "Could not contact RetroAchievements securely. Check your connection and try again."
            )
        })?;
    ensure!(
        response.status().is_success(),
        "RetroAchievements sign-in failed; please try again later"
    );
    let body = response
        .body_mut()
        .with_config()
        .limit(64 * 1024)
        .read_to_string()
        .context("Reading RetroAchievements sign-in response")?;
    let credentials = parse_login(&body)?;
    crate::settings::save_secret(
        ACCOUNT,
        &serde_json::to_string(&credentials)?,
        "RetroAchievements token",
    )?;
    Ok(credentials.username)
}
fn parse_login(body: &str) -> Result<Credentials> {
    let value: serde_json::Value =
        serde_json::from_str(body).context("Invalid RetroAchievements sign-in response")?;
    ensure!(
        value["Success"].as_bool() == Some(true),
        "Sign-in was rejected. Check your username/password and account status."
    );
    let credentials = Credentials {
        username: value["User"].as_str().unwrap_or_default().to_owned(),
        token: value["Token"].as_str().unwrap_or_default().to_owned(),
    };
    credentials.validate()?;
    Ok(credentials)
}

pub struct LaunchPolicy {
    pub mode: Mode,
    pub hardcore: bool,
    credentials: Option<Credentials>,
}
impl LaunchPolicy {
    /// Resolve before expensive launch preparation. Explicit per-game requests
    /// fail on unsupported runtimes; a global preference only targets RetroArch.
    pub fn load(
        store: &SettingsStore,
        game: &str,
        option: Option<&RomEmulatorOption>,
        mods: &crate::game_mods::Profile,
    ) -> Result<Option<Self>> {
        let game_mode = preference(store, game)?;
        let mode = game_mode.unwrap_or(preference(store, "")?.unwrap_or_default());
        let Some(option) =
            option.filter(|option| option.runtime_kind == EmulatorRuntimeKind::RetroArch)
        else {
            ensure!(
                !matches!(game_mode, Some(Mode::Casual | Mode::Hardcore)),
                "This game's achievement mode needs RetroArch. Select a RetroArch core or use Emulator settings for this game."
            );
            return Ok(None);
        };
        let read_bool = |key, default| {
            crate::display_setup::retroarch_config_value(&option.executable, key)
                .map(|value| value == "true")
                .unwrap_or(default)
        };
        let hardcore = mode == Mode::Hardcore
            || (mode == Mode::Emulator
                && read_bool("cheevos_enable", false)
                && read_bool("cheevos_hardcore_mode_enable", true));
        validate_cheats(hardcore, mods)?;
        let credentials = if matches!(mode, Mode::Casual | Mode::Hardcore) {
            Some(credentials()?.context("Sign in to RetroAchievements in Settings, or choose Off / Emulator settings for this game")?)
        } else {
            None
        };
        Ok(Some(Self {
            mode,
            hardcore,
            credentials,
        }))
    }
    /// Kept alive by the launch worker until the emulator exits. The directory
    /// is private, is never a settings backup, and contains no password.
    pub fn attach(
        &self,
        plan: &mut LaunchPlan,
        executable: &EmulatorExecutable,
    ) -> Result<Option<tempfile::TempDir>> {
        let Some(config) = self.config()? else {
            return Ok(None);
        };
        let root = directories::ProjectDirs::from("com", "Lunchbox", "Lunchbox")
            .context("Finding achievement session directory")?
            .data_local_dir()
            .join("achievement-sessions");
        fs::create_dir_all(&root)?;
        let directory = tempfile::Builder::new()
            .prefix("session-")
            .tempdir_in(root)?;
        let path = directory.path().join("achievements.cfg");
        write_private(&path, &config)?;
        crate::controller_launch::attach_config(plan, executable, &path)?;
        Ok(Some(directory))
    }
    fn config(&self) -> Result<Option<String>> {
        if self.mode == Mode::Emulator && !self.hardcore {
            return Ok(None);
        }
        let mut config = String::from(
            // RetroArch applies per-core/game override files AFTER appendconfig.
            // They must not undo an explicit Off/Hardcore choice or auto-resume
            // protection. This affects only this launch, never the saved config.
            "# Lunchbox private achievement session\nconfig_save_on_exit = \"false\"\nauto_overrides_enable = \"false\"\n",
        );
        if self.mode != Mode::Emulator {
            config.push_str(&format!(
                "cheevos_enable = \"{}\"\ncheevos_hardcore_mode_enable = \"{}\"\n",
                self.mode != Mode::Off,
                self.hardcore
            ));
        }
        if let Some(credentials) = &self.credentials {
            credentials.validate()?;
            config.push_str(&format!("cheevos_username = \"{}\"\ncheevos_token = \"{}\"\ncheevos_password = \"\"\ncheevos_custom_host = \"\"\ncheevos_verbose_enable = \"true\"\ncheevos_badges_enable = \"true\"\ncheevos_start_active = \"false\"\ncheevos_test_unofficial = \"false\"\n", credentials.username, credentials.token));
            config.push_str("cheevos_visibility_unlock = \"true\"\ncheevos_visibility_summary = \"1\"\ncheevos_visibility_mastery = \"true\"\n");
        }
        if self.hardcore {
            // SRAM and creating auto-states remain available. RetroArch itself
            // enforces load-state/slow-motion restrictions and core eligibility.
            config.push_str("savestate_auto_load = \"false\"\nrewind_enable = \"false\"\napply_cheats_after_load = \"false\"\napply_cheats_after_toggle = \"false\"\n");
        }
        Ok(Some(config))
    }
}
fn validate_cheats(hardcore: bool, mods: &crate::game_mods::Profile) -> Result<()> {
    ensure!(
        !(hardcore && mods.cheats_enabled && mods.cheats.iter().any(|c| c.enabled)),
        "Hardcore achievements and enabled cheats cannot be used together. Disable cheats or select Casual / Off in this game's RetroAchievements settings."
    );
    Ok(())
}
fn write_private(path: &Path, text: &str) -> Result<()> {
    let mut options = fs::OpenOptions::new();
    options.write(true).create_new(true);
    #[cfg(unix)]
    {
        use std::os::unix::fs::OpenOptionsExt;
        options.mode(0o600);
    }
    options.open(path)?.write_all(text.as_bytes())?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn preferences_round_trip_and_inherit() {
        let temp = tempfile::tempdir().unwrap();
        let store = SettingsStore::at(temp.path().join("settings.sqlite")).unwrap();
        assert_eq!(preference(&store, "").unwrap(), None);
        save_preference(&store, "", "casual").unwrap();
        save_preference(&store, "game", "off").unwrap();
        assert_eq!(preference(&store, "").unwrap(), Some(Mode::Casual));
        assert_eq!(preference(&store, "game").unwrap(), Some(Mode::Off));
        save_preference(&store, "game", "inherit").unwrap();
        assert_eq!(preference(&store, "game").unwrap(), None);
        assert!(save_preference(&store, "", "inherit").is_err());
    }
    #[test]
    fn login_response_is_validated_without_echoing_secrets() {
        let credentials =
            parse_login(r#"{"Success":true,"User":"Player","Token":"private-token"}"#).unwrap();
        assert_eq!(credentials.username, "Player");
        assert!(parse_login(r#"{"Success":true,"User":"Player","Token":"bad\nvalue"}"#).is_err());
        let error = parse_login(r#"{"Success":false,"Error":"private-password"}"#)
            .err()
            .unwrap();
        assert!(!error.to_string().contains("private-password"));
    }
    #[test]
    fn hardcore_blocks_cheats_and_preserves_sram_and_state_saving() {
        let mut mods = crate::game_mods::Profile::default();
        mods.cheats_enabled = true;
        mods.cheats.push(crate::game_mods::Cheat {
            enabled: true,
            ..Default::default()
        });
        assert!(validate_cheats(true, &mods).is_err());
        assert!(validate_cheats(false, &mods).is_ok());
        let policy = LaunchPolicy {
            mode: Mode::Hardcore,
            hardcore: true,
            credentials: Some(Credentials {
                username: "Player".into(),
                token: "test-token".into(),
            }),
        };
        let config = policy.config().unwrap().unwrap();
        assert!(config.contains("savestate_auto_load = \"false\""));
        assert!(config.contains("rewind_enable = \"false\""));
        assert!(!config.contains("savestate_auto_save"));
        assert!(!config.contains("savefile_directory"));
        assert!(config.contains("cheevos_password = \"\""));
    }
    #[test]
    fn default_does_not_override_existing_emulator_account() {
        let policy = LaunchPolicy {
            mode: Mode::Emulator,
            hardcore: false,
            credentials: None,
        };
        assert!(policy.config().unwrap().is_none());
        let off = LaunchPolicy {
            mode: Mode::Off,
            hardcore: false,
            credentials: None,
        }
        .config()
        .unwrap()
        .unwrap();
        assert!(off.contains("cheevos_enable = \"false\""));
        assert!(!off.contains("cheevos_token"));
    }
    #[test]
    fn explicit_modes_require_a_supported_runtime() {
        let temp = tempfile::tempdir().unwrap();
        let store = SettingsStore::at(temp.path().join("settings.sqlite")).unwrap();
        let mods = crate::game_mods::Profile::default();
        save_preference(&store, "", "hardcore").unwrap();
        assert!(
            LaunchPolicy::load(&store, "game", None, &mods)
                .unwrap()
                .is_none()
        );
        save_preference(&store, "game", "casual").unwrap();
        assert!(LaunchPolicy::load(&store, "game", None, &mods).is_err());
        save_preference(&store, "game", "emulator").unwrap();
        assert!(
            LaunchPolicy::load(&store, "game", None, &mods)
                .unwrap()
                .is_none()
        );
    }
    #[test]
    fn launch_config_attaches_last_without_credentials_in_arguments() {
        let policy = LaunchPolicy {
            mode: Mode::Casual,
            hardcore: false,
            credentials: Some(Credentials {
                username: "Player".into(),
                token: "fake-secret-token".into(),
            }),
        };
        let mut plan = LaunchPlan {
            emulator_name: "RetroArch".into(),
            program: "retroarch".into(),
            arguments: vec!["--appendconfig".into(), "/tmp/display.cfg".into()],
            current_directory: std::env::temp_dir(),
            environment: vec![],
            cleanup_paths: vec![],
            retroarch_content: None,
        };
        let session = policy
            .attach(&mut plan, &EmulatorExecutable::Native("retroarch".into()))
            .unwrap()
            .unwrap();
        let path = session.path().join("achievements.cfg");
        assert!(
            plan.command_summary()
                .ends_with(&path.to_string_lossy().to_string())
        );
        assert!(!plan.command_summary().contains("fake-secret-token"));
        let config = fs::read_to_string(&path).unwrap();
        assert!(config.contains("cheevos_enable = \"true\""));
        assert!(config.contains("cheevos_hardcore_mode_enable = \"false\""));
        assert!(config.contains("auto_overrides_enable = \"false\""));
        drop(session);
        assert!(!path.exists());
    }
    #[test]
    fn private_config_is_removed_with_session() {
        let directory = tempfile::tempdir().unwrap();
        let path = directory.path().join("session.cfg");
        write_private(&path, "secret").unwrap();
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            assert_eq!(
                fs::metadata(&path).unwrap().permissions().mode() & 0o777,
                0o600
            );
        }
        drop(directory);
        assert!(!path.exists());
    }
}
