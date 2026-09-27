//! Optional companion process. GameBuddy owns its models, guides and capture
//! permissions; Lunchpail passes only the context of a successfully started game.
use anyhow::{Context, Result, ensure};
use serde::{Deserialize, Serialize};
use std::{
    path::PathBuf,
    process::{Command, Stdio},
};

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
#[serde(default)]
pub(crate) struct Config {
    pub enabled: bool,
    /// Executable path, never a shell command. Empty means gamebuddy in PATH.
    pub executable: String,
}

fn path() -> Result<PathBuf> {
    Ok(crate::app_paths::project_dirs()
        .context("Lunchpail configuration directory is unavailable")?
        .config_dir()
        .join("gamebuddy.json"))
}

pub(crate) fn load() -> Result<Config> {
    match std::fs::read(path()?) {
        Ok(bytes) => serde_json::from_slice(&bytes).context("Reading GameBuddy launch preferences"),
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(Config::default()),
        Err(e) => Err(e.into()),
    }
}

impl Config {
    fn validate(&self) -> Result<()> {
        ensure!(
            !self.executable.contains(['\0', '\n', '\r']),
            "GameBuddy requires an executable path, not a command line"
        );
        Ok(())
    }

    pub(crate) fn save(&self) -> Result<()> {
        self.validate()?;
        let path = path()?;
        std::fs::create_dir_all(path.parent().context("Invalid configuration path")?)?;
        let mut file = tempfile::NamedTempFile::new_in(path.parent().unwrap())?;
        use std::io::Write;
        file.write_all(&serde_json::to_vec_pretty(self)?)?;
        file.as_file().sync_all()?;
        file.persist(path)
            .context("Saving GameBuddy launch preferences")?;
        Ok(())
    }

    fn command(
        &self,
        game: &str,
        game_id: &str,
        platform: &str,
        game_pid: u32,
        session: &str,
    ) -> Result<Command> {
        self.validate()?;
        let executable = std::env::var_os("GAMEBUDDY_EXECUTABLE")
            .filter(|value| !value.is_empty())
            .unwrap_or_else(|| {
                if self.executable.is_empty() {
                    "gamebuddy".into()
                } else {
                    self.executable.clone().into()
                }
            });
        let mut command = Command::new(executable);
        command.args([
            "--game",
            game,
            "--game-id",
            game_id,
            "--platform",
            platform,
            "--game-pid",
            &game_pid.to_string(),
            "--session-id",
            session,
        ]);
        command.stdin(Stdio::null());
        Ok(command)
    }

    /// Companion failure is reported as a warning, never a failed game launch.
    pub(crate) fn launch(
        &self,
        game: &str,
        game_id: &str,
        platform: &str,
        game_pid: u32,
        session: &str,
    ) -> Result<Option<u32>> {
        if !self.enabled {
            return Ok(None);
        }
        let mut child = self
            .command(game, game_id, platform, game_pid, session)?
            .spawn()
            .context("Could not start GameBuddy; set its executable path or install it in PATH")?;
        let pid = child.id();
        std::thread::spawn(move || match child.wait() {
            Ok(status) if !status.success() => {
                eprintln!("LUNCHPAIL_GAMEBUDDY_EXIT status={status}")
            }
            Err(error) => eprintln!("LUNCHPAIL_GAMEBUDDY_WAIT error={error}"),
            _ => {}
        });
        Ok(Some(pid))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn defaults_to_disabled_and_preserves_preferences() {
        let config: Config = serde_json::from_str("{}").unwrap();
        assert!(!config.enabled);
        assert!(
            config
                .launch("Faxanadu", "42", "NES", 1, "session")
                .unwrap()
                .is_none()
        );
        let config: Config = serde_json::from_str(
            r#"{"enabled":true,"executable":"C:\\Games\\GameBuddy\\gamebuddy.exe"}"#,
        )
        .unwrap();
        assert!(config.enabled);
        assert!(config.validate().is_ok());
        let bad = Config {
            enabled: true,
            executable: "bad\ncommand".into(),
        };
        assert!(bad.validate().is_err());
    }

    #[test]
    fn launch_context_is_literal_arguments_not_shell_text() {
        let command = Config::default()
            .command(
                "Faxanadu; $(touch unwanted)",
                "game 42",
                "NES",
                123,
                "session-9",
            )
            .unwrap();
        let args: Vec<_> = command
            .get_args()
            .map(|s| s.to_string_lossy().into_owned())
            .collect();
        assert_eq!(
            args,
            [
                "--game",
                "Faxanadu; $(touch unwanted)",
                "--game-id",
                "game 42",
                "--platform",
                "NES",
                "--game-pid",
                "123",
                "--session-id",
                "session-9"
            ]
        );
    }
}
