//! The emulator can outlive Lunchbox (notably when dev.sh restarts the UI).
//! Keep the launch claim outside the game-details selection and verify the
//! process birth time before treating a persisted PID as a live session.

use std::fs::{self, OpenOptions};
use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};

use anyhow::{Context, Result, anyhow, ensure};
use fs2::FileExt;
use serde::{Deserialize, Serialize};
#[cfg(not(target_os = "windows"))]
use sysinfo::Signal;
use sysinfo::{Pid, ProcessRefreshKind, ProcessStatus, ProcessesToUpdate, System, UpdateKind};

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct ProcessIdentity {
    pid: u32,
    started: u64,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct Session {
    version: u32,
    pub token: String,
    pub game_id: String,
    pub title: String,
    pub emulator: String,
    owner: ProcessIdentity,
    pub process: Option<ProcessIdentity>,
}

impl Session {
    pub fn preparing(&self) -> bool {
        self.process.is_none()
    }
}

fn session_path() -> Result<PathBuf> {
    Ok(
        directories::ProjectDirs::from("com", "Lunchbox", "Lunchbox")
            .context("Lunchbox data directory is unavailable")?
            .data_local_dir()
            .join("emulator-session.json"),
    )
}

fn with_lock<T>(action: impl FnOnce(&Path) -> Result<T>) -> Result<T> {
    let path = session_path()?;
    let directory = path.parent().context("session path has no parent")?;
    fs::create_dir_all(directory).context("creating Lunchbox session directory")?;
    let lock = OpenOptions::new()
        .create(true)
        .read(true)
        .write(true)
        .open(directory.join("emulator-session.lock"))?;
    lock.lock_exclusive().context("locking emulator session")?;
    let result = action(&path);
    let _ = FileExt::unlock(&lock);
    result
}

fn identity(pid: u32) -> Option<ProcessIdentity> {
    if !is_process_leader(pid) {
        return None;
    }
    let mut system = System::new();
    let key = Pid::from_u32(pid);
    system.refresh_processes(ProcessesToUpdate::Some(&[key]), true);
    system.process(key).and_then(|process| {
        (!matches!(
            process.status(),
            ProcessStatus::Zombie | ProcessStatus::Dead
        ) && process.start_time() > 0)
            .then_some(ProcessIdentity {
                pid,
                started: process.start_time(),
            })
    })
}

#[cfg(target_os = "linux")]
fn is_process_leader(pid: u32) -> bool {
    let Ok(status) = fs::read_to_string(format!("/proc/{pid}/status")) else {
        return false;
    };
    status.lines().find_map(|line| {
        line.strip_prefix("Tgid:")
            .and_then(|value| value.trim().parse::<u32>().ok())
    }) == Some(pid)
}

#[cfg(not(target_os = "linux"))]
fn is_process_leader(_pid: u32) -> bool {
    true
}

fn alive(process: &ProcessIdentity) -> bool {
    identity(process.pid).as_ref() == Some(process)
}

fn read(path: &Path) -> Result<Option<Session>> {
    match fs::read(path) {
        Ok(bytes) => Ok(Some(
            serde_json::from_slice::<Session>(&bytes).context("reading emulator session")?,
        )),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(None),
        Err(error) => Err(error).context("reading emulator session"),
    }
}

fn write(path: &Path, session: &Session) -> Result<()> {
    use std::io::Write;
    let parent = path.parent().context("session path has no parent")?;
    let mut temporary = tempfile::NamedTempFile::new_in(parent)?;
    serde_json::to_writer(&mut temporary, session)?;
    temporary.flush()?;
    temporary.as_file().sync_all()?;
    temporary
        .persist(path)
        .context("publishing emulator session")?;
    Ok(())
}

fn owned_retroarch_orphan() -> Option<Session> {
    // Migration for games started before this record existed. The generated
    // Lunchbox config/content path distinguishes our session from a user's
    // independently launched RetroArch process.
    // Refresh processes once. `new_all()` already refreshes everything, so
    // pairing it with `refresh_processes` doubled the cost of every poll.
    let mut system = System::new();
    system.refresh_processes_specifics(
        ProcessesToUpdate::All,
        true,
        ProcessRefreshKind::nothing().with_cmd(UpdateKind::Always),
    );
    let process = system.processes().values().find(|process| {
        is_process_leader(process.pid().as_u32())
            && !matches!(
                process.status(),
                ProcessStatus::Zombie | ProcessStatus::Dead
            )
            && process
                .name()
                .to_string_lossy()
                .eq_ignore_ascii_case("retroarch")
            && process.cmd().iter().any(|arg| {
                let value = arg.to_string_lossy();
                value.contains("/lunchbox/launch-display/retroarch-")
                    || value.contains("/lunchbox/launch-preparation/session-")
            })
    })?;
    let pid = process.pid().as_u32();
    let title = process
        .cmd()
        .last()
        .and_then(|value| Path::new(value).file_stem())
        .map(|value| value.to_string_lossy().into_owned())
        .unwrap_or_else(|| "Running game".to_owned());
    Some(Session {
        version: 1,
        token: uuid::Uuid::new_v4().to_string(),
        game_id: String::new(),
        title,
        emulator: "RetroArch".to_owned(),
        owner: ProcessIdentity {
            pid,
            started: process.start_time(),
        },
        process: Some(ProcessIdentity {
            pid,
            started: process.start_time(),
        }),
    })
}

fn active_locked(path: &Path) -> Result<Option<Session>> {
    if let Some(session) = read(path)? {
        let valid = session.version == 1
            && match &session.process {
                Some(process) => alive(process),
                None => alive(&session.owner),
            };
        if valid {
            return Ok(Some(session));
        }
        fs::remove_file(path).context("removing expired emulator session")?;
    }
    if let Some(orphan) = owned_retroarch_orphan() {
        write(path, &orphan)?;
        return Ok(Some(orphan));
    }
    Ok(None)
}

pub fn active() -> Result<Option<Session>> {
    with_lock(active_locked)
}

pub fn reserve(game_id: &str, title: &str) -> Result<Session> {
    with_lock(|path| {
        ensure!(
            active_locked(path)?.is_none(),
            "another emulator session is already active"
        );
        let owner = identity(std::process::id()).context("Lunchbox process is not available")?;
        let session = Session {
            version: 1,
            token: uuid::Uuid::new_v4().to_string(),
            game_id: game_id.to_owned(),
            title: title.to_owned(),
            emulator: String::new(),
            owner,
            process: None,
        };
        write(path, &session)?;
        Ok(session)
    })
}

pub fn mark_running(token: &str, pid: u32, emulator: &str) -> Result<()> {
    with_lock(|path| {
        let mut session = read(path)?.context("launch reservation is missing")?;
        ensure!(session.token == token, "launch reservation was replaced");
        let process =
            identity(pid).context("emulator process exited before it could be tracked")?;
        session.process = Some(process);
        session.emulator = emulator.to_owned();
        write(path, &session)
    })
}

pub fn clear(token: &str) -> Result<()> {
    with_lock(|path| {
        if read(path)?.is_some_and(|session| session.token == token) {
            fs::remove_file(path).context("clearing emulator session")?;
        }
        Ok(())
    })
}

fn request_clean_exit(process: &sysinfo::Process) -> Result<()> {
    #[cfg(target_os = "windows")]
    {
        // taskkill without /F requests a window close; /F discards saves.
        let status = std::process::Command::new("taskkill.exe")
            .arg("/PID")
            .arg(process.pid().as_u32().to_string())
            .status()
            .context("requesting emulator window close")?;
        ensure!(
            status.success(),
            "Windows could not close the emulator window"
        );
    }
    #[cfg(not(target_os = "windows"))]
    ensure!(
        process.kill_with(Signal::Term) == Some(true),
        "the emulator did not accept a clean exit request"
    );
    Ok(())
}

pub fn stop(session: &Session) -> Result<()> {
    let root = session
        .process
        .as_ref()
        .context("emulator is still preparing")?;
    ensure!(alive(root), "the tracked emulator has already exited");
    let root_pid = Pid::from_u32(root.pid);
    let mut system = System::new_all();
    system.refresh_processes(ProcessesToUpdate::All, true);
    ensure!(
        system
            .process(root_pid)
            .is_some_and(|process| process.start_time() == root.started),
        "the emulator PID now belongs to another process"
    );

    // Ask the actual emulator (the leaf below Flatpak/bwrap, when present)
    // to quit first so it can flush SRAM and automatic save state. Never
    // signal Lunchbox's process group: dev.sh and the UI share that group.
    let mut descendants = Vec::new();
    let mut frontier = vec![root_pid];
    while let Some(parent) = frontier.pop() {
        for (pid, process) in system.processes() {
            if process.parent() == Some(parent) && is_process_leader(pid.as_u32()) {
                descendants.push(*pid);
                frontier.push(*pid);
            }
        }
    }
    let leaves = descendants
        .iter()
        .copied()
        .filter(|pid| {
            !descendants.iter().any(|other| {
                system
                    .process(*other)
                    .is_some_and(|process| process.parent() == Some(*pid))
            })
        })
        .collect::<Vec<_>>();
    let targets = if leaves.is_empty() {
        vec![root_pid]
    } else {
        leaves
    };
    for pid in &targets {
        if let Some(process) = system.process(*pid)
            && identity(pid.as_u32())
                .as_ref()
                .is_some_and(|identity| identity.started == process.start_time())
        {
            request_clean_exit(process)?;
        }
    }
    let deadline = Instant::now() + Duration::from_secs(5);
    while Instant::now() < deadline {
        if !alive(root) {
            return Ok(());
        }
        std::thread::sleep(Duration::from_millis(100));
    }
    // Never tear down a wrapper while its game is still alive: the wrapper
    // could force-close it before SRAM and auto-state have been written.
    if targets.iter().any(|pid| identity(pid.as_u32()).is_some()) {
        return Err(anyhow!(
            "emulator ignored the clean stop request; quit from its menu to preserve saves"
        ));
    }
    if alive(root) {
        let mut current = System::new();
        current.refresh_processes(ProcessesToUpdate::Some(&[root_pid]), true);
        if let Some(process) = current.process(root_pid)
            && process.start_time() == root.started
        {
            request_clean_exit(process)?;
        }
    }
    let deadline = Instant::now() + Duration::from_secs(3);
    while Instant::now() < deadline {
        if !alive(root) {
            return Ok(());
        }
        std::thread::sleep(Duration::from_millis(100));
    }
    Err(anyhow!(
        "emulator did not exit after the stop request; close its window to preserve saves"
    ))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn current_process_identity_is_stable_and_wrong_birth_time_is_rejected() {
        let current = identity(std::process::id()).unwrap();
        assert!(alive(&current));
        assert!(!alive(&ProcessIdentity {
            started: current.started + 1,
            ..current
        }));
    }

    #[test]
    fn unrelated_process_is_not_adopted_as_a_game() {
        let current = identity(std::process::id()).unwrap();
        let session = Session {
            version: 1,
            token: "test".into(),
            game_id: "test".into(),
            title: "test".into(),
            emulator: String::new(),
            owner: current,
            process: None,
        };
        assert!(session.preparing());
    }

    #[test]
    fn session_record_round_trips_without_using_the_live_registry() {
        let directory = tempfile::tempdir().unwrap();
        let path = directory.path().join("session.json");
        let owner = identity(std::process::id()).unwrap();
        let session = Session {
            version: 1,
            token: "test-token".into(),
            game_id: "game".into(),
            title: "Test Game".into(),
            emulator: "RetroArch".into(),
            owner: owner.clone(),
            process: Some(owner),
        };
        write(&path, &session).unwrap();
        assert_eq!(read(&path).unwrap(), Some(session));
    }

    #[test]
    fn stop_rejects_reused_pid_without_signalling_it() {
        let current = identity(std::process::id()).unwrap();
        let invalid = ProcessIdentity {
            started: current.started + 1,
            ..current.clone()
        };
        let session = Session {
            version: 1,
            token: "test".into(),
            game_id: "test".into(),
            title: "test".into(),
            emulator: String::new(),
            owner: current,
            process: Some(invalid),
        };
        assert!(stop(&session).is_err());
    }

    #[cfg(unix)]
    #[test]
    fn stop_ends_only_the_tracked_test_process() {
        let mut child = std::process::Command::new("sleep")
            .arg("30")
            .spawn()
            .unwrap();
        let process = identity(child.id()).unwrap();
        let session = Session {
            version: 1,
            token: "test".into(),
            game_id: "test".into(),
            title: "test".into(),
            emulator: "sleep".into(),
            owner: identity(std::process::id()).unwrap(),
            process: Some(process),
        };
        let result = stop(&session);
        if result.is_err() {
            let _ = child.kill();
        }
        let _ = child.wait();
        result.unwrap();
    }

    #[cfg(target_os = "linux")]
    #[test]
    fn retroarch_worker_threads_cannot_be_adopted_as_sessions() {
        let (tid_sender, tid_receiver) = std::sync::mpsc::channel();
        let (done_sender, done_receiver) = std::sync::mpsc::channel();
        let worker = std::thread::spawn(move || {
            let tid = unsafe { libc::syscall(libc::SYS_gettid) as u32 };
            tid_sender.send(tid).unwrap();
            done_receiver.recv().unwrap();
        });
        let tid = tid_receiver.recv().unwrap();
        assert_ne!(tid, std::process::id());
        assert!(!is_process_leader(tid));
        assert!(identity(tid).is_none());
        done_sender.send(()).unwrap();
        worker.join().unwrap();
    }
}
