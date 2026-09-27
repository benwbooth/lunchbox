//! Optional companion process. GameBuddy owns its models, guides and capture
//! permissions. On Linux, prepare its compositor before starting the emulator;
//! keep the actual emulator Child here so controller pipes and cancellation work.
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
        game_pid: Option<u32>,
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
            "--session-id",
            session,
        ]);
        if let Some(pid) = game_pid {
            command.args(["--game-pid", &pid.to_string()]);
        }
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
            .command(game, game_id, platform, Some(game_pid), session)?
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

/// The compositor is scoped to one emulator session. No shell wrapping of the
/// emulator means its PID, stdin/stdout and calibrated controller transport stay intact.
pub(crate) struct Session {
    #[cfg(target_os = "linux")]
    child: std::process::Child,
    #[cfg(target_os = "linux")]
    directory: tempfile::TempDir,
    #[cfg(target_os = "linux")]
    display: String,
}
impl Config {
    pub(crate) fn prepare(
        &self,
        game: &str,
        game_id: &str,
        platform: &str,
        session: &str,
        cancelled: &std::sync::atomic::AtomicBool,
    ) -> Result<Option<Session>> {
        if !self.enabled {
            return Ok(None);
        }
        #[cfg(target_os = "linux")]
        {
            use std::{
                sync::atomic::Ordering,
                time::{Duration, Instant},
            };
            let directory = tempfile::Builder::new()
                .prefix("lunchpail-gamebuddy-")
                .tempdir()?;
            use std::os::unix::fs::PermissionsExt;
            std::fs::set_permissions(directory.path(), std::fs::Permissions::from_mode(0o700))?;
            let log_path = directory.path().join("compositor.log");
            let log = std::fs::File::create(&log_path)?;
            let child = self
                .command(game, game_id, platform, None, session)?
                .arg("--gamescope-bridge")
                .arg(directory.path())
                .stdout(log.try_clone()?)
                .stderr(log)
                .spawn()
                .context("Could not start the GameBuddy Gamescope launcher")?;
            let mut prepared = Session {
                child,
                directory,
                display: String::new(),
            };
            let deadline = Instant::now() + Duration::from_secs(45);
            loop {
                anyhow::ensure!(
                    !cancelled.load(Ordering::Relaxed),
                    "GameBuddy startup cancelled"
                );
                if let Ok(bytes) = std::fs::read(prepared.directory.path().join("ready.json")) {
                    let ready: Ready = serde_json::from_slice(&bytes)
                        .context("Invalid GameBuddy session handshake")?;
                    ready.validate()?;
                    prepared.display = ready.display;
                    return Ok(Some(prepared));
                }
                if let Some(status) = prepared.child.try_wait()? {
                    let log = std::fs::read_to_string(&log_path).unwrap_or_default();
                    anyhow::bail!(
                        "Gamescope launcher exited with {status}: {}",
                        log.lines()
                            .rev()
                            .take(5)
                            .collect::<Vec<_>>()
                            .into_iter()
                            .rev()
                            .collect::<Vec<_>>()
                            .join(" | ")
                    );
                }
                anyhow::ensure!(
                    Instant::now() < deadline,
                    "Timed out waiting for Gamescope to create the game display"
                );
                std::thread::sleep(Duration::from_millis(25));
            }
        }
        #[cfg(not(target_os = "linux"))]
        {
            let _ = (game, game_id, platform, session, cancelled);
            Ok(None)
        }
    }
}
#[cfg(target_os = "linux")]
#[derive(Deserialize)]
struct Ready {
    version: u32,
    display: String,
}
#[cfg(target_os = "linux")]
impl Ready {
    fn validate(&self) -> Result<()> {
        ensure!(self.version == 1, "Unsupported GameBuddy session protocol");
        ensure!(
            self.display.strip_prefix(':').is_some_and(
                |s| !s.is_empty() && s.bytes().all(|c| c.is_ascii_digit() || c == b'.')
            ),
            "GameBuddy returned an invalid local X display"
        );
        Ok(())
    }
}
impl Session {
    pub(crate) fn apply(&self, plan: &mut crate::emulator::LaunchPlan) {
        #[cfg(target_os = "linux")]
        for (key, value) in [
            ("DISPLAY", self.display.as_str()),
            ("WAYLAND_DISPLAY", ""),
            ("SDL_VIDEODRIVER", "x11"),
            ("QT_QPA_PLATFORM", "xcb"),
            ("GDK_BACKEND", "x11"),
        ] {
            plan.environment.retain(|(name, _)| name != key);
            plan.environment.push((key.into(), value.into()));
        }
        // fallback-x11 is suppressed when Flatpak sees the host Wayland
        // socket. Explicitly use this session's Xwayland for surface capture.
        #[cfg(target_os = "linux")]
        if plan
            .program
            .file_name()
            .is_some_and(|name| name == "flatpak")
            && plan.arguments.first().is_some_and(|arg| arg == "run")
        {
            plan.arguments
                .splice(1..1, ["--socket=x11".into(), "--nosocket=wayland".into()]);
        }
        #[cfg(not(target_os = "linux"))]
        let _ = plan;
    }
    pub(crate) fn attach(&self, game_pid: u32) -> Result<()> {
        #[cfg(target_os = "linux")]
        {
            let path = self.directory.path().join("game.json");
            let temp = self.directory.path().join("game.tmp");
            std::fs::write(
                &temp,
                serde_json::to_vec(&serde_json::json!({"game_pid": game_pid}))?,
            )?;
            std::fs::rename(temp, path)?;
        }
        #[cfg(not(target_os = "linux"))]
        let _ = game_pid;
        Ok(())
    }
}
impl Drop for Session {
    fn drop(&mut self) {
        #[cfg(target_os = "linux")]
        {
            let _ = std::fs::write(self.directory.path().join("stop"), b"");
            for _ in 0..40 {
                if matches!(self.child.try_wait(), Ok(Some(_))) {
                    return;
                }
                std::thread::sleep(std::time::Duration::from_millis(25));
            }
            let _ = self.child.kill();
            let _ = self.child.wait();
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[cfg(target_os = "linux")]
    #[test]
    fn validates_local_versioned_compositor_handshake() {
        assert!(
            Ready {
                version: 1,
                display: ":5".into()
            }
            .validate()
            .is_ok()
        );
        for (version, display) in [(2, ":5"), (1, "remote:5"), (1, ":"), (1, ":5;bad")] {
            assert!(
                Ready {
                    version,
                    display: display.into()
                }
                .validate()
                .is_err()
            );
        }
    }
    /// Opt-in runtime test: starts a private compositor and the real GameBuddy
    /// UI, then launches the input/capture fixture through Lunchpail's pipe path.
    #[cfg(target_os = "linux")]
    #[test]
    #[ignore = "requires a desktop, Gamescope and built GameBuddy compositor_probe"]
    fn live_gamescope_preserves_emulator_process_and_pipes() -> Result<()> {
        use std::{
            io::Read,
            sync::atomic::AtomicBool,
            time::{Duration, Instant},
        };
        let config = Config {
            enabled: true,
            executable: std::env::var("GAMEBUDDY_TEST_EXECUTABLE")?,
        };
        let session = config
            .prepare(
                "Faxanadu",
                "fixture",
                "NES",
                "integration-test",
                &AtomicBool::new(false),
            )?
            .context("no compositor")?;
        let mut plan = crate::emulator::LaunchPlan {
            emulator_name: "GameBuddy test fixture".into(),
            program: std::env::var("GAMEBUDDY_TEST_FIXTURE")?.into(),
            arguments: vec![std::env::var("GAMEBUDDY_TEST_OUTPUT")?.into()],
            current_directory: std::env::current_dir()?,
            environment: vec![],
            cleanup_paths: vec![],
            retroarch_content: None,
        };
        let program = plan.program.clone();
        session.apply(&mut plan);
        assert_eq!(plan.program, program);
        let mut child = crate::emulator::spawn_launch_plan_with_controller_pipes(&plan)?;
        assert!(child.stdin.is_some() && child.stdout.is_some() && child.stderr.is_some());
        session.attach(child.id())?;
        let deadline = Instant::now() + Duration::from_secs(45);
        let status = loop {
            if let Some(status) = child.try_wait()? {
                break status;
            }
            if Instant::now() >= deadline {
                let _ = child.kill();
                let _ = child.wait();
                anyhow::bail!("compositor fixture timed out");
            }
            std::thread::sleep(Duration::from_millis(50));
        };
        let mut stdout = String::new();
        let mut stderr = String::new();
        child.stdout.take().unwrap().read_to_string(&mut stdout)?;
        child.stderr.take().unwrap().read_to_string(&mut stderr)?;
        ensure!(status.success(), "fixture failed: {stderr}");
        ensure!(
            stdout.contains("GAMEBUDDY_COMPOSITOR_TEST_PASS"),
            "fixture did not finish: {stdout} {stderr}"
        );
        let display = session.display.clone();
        let compositor_pid = session.child.id();
        drop(session);
        assert!(!std::path::Path::new(&format!("/proc/{compositor_pid}")).exists());
        eprintln!(
            "LUNCHPAIL_GAMEBUDDY_LIVE_PASS display={display} actual_game_pid={} pipes=true cleanup=true",
            child.id()
        );
        Ok(())
    }

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
                Some(123),
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
                "--session-id",
                "session-9",
                "--game-pid",
                "123"
            ]
        );
    }
}
