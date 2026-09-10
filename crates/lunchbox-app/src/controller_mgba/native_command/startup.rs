//! Child-side mount/runtime checks for the pinned SDL native frontend.
//! This establishes launch routing, not a gameplay or button-response test.
use super::*;
use crate::controller_native_process::native_pid;
use std::os::unix::fs::MetadataExt;
use std::time::{Duration, Instant};

fn ready(session: &NativeSession, pid: u32) -> Result<bool> {
    let maps = std::fs::read_to_string(format!("/proc/{pid}/maps"))?;
    let sdl = session.setup.sdl_library.canonicalize()?;
    if !maps.lines().any(|line| {
        let path = line
            .split_whitespace()
            .skip(5)
            .collect::<Vec<_>>()
            .join(" ")
            .replace("\\040", " ");
        Path::new(&path) == sdl
    }) {
        return Ok(false);
    }
    let mounted = PathBuf::from(format!("/proc/{pid}/root")).join(
        session
            .setup
            .source_config
            .canonicalize()?
            .strip_prefix("/")?,
    );
    let actual = std::fs::metadata(mounted)?;
    let private = std::fs::metadata(session.configuration.config_path())?;
    ensure!(
        actual.dev() == private.dev() && actual.ino() == private.ino(),
        "mGBA child does not see the private native config.ini"
    );
    // GUID preference is unique in the retained inventory. Also require that
    // the owned native process actually opened the selected runtime node.
    let selected = std::fs::metadata(&session.runtime_path)?;
    let mut opened = false;
    for (index, entry) in std::fs::read_dir(format!("/proc/{pid}/fd"))?.enumerate() {
        ensure!(
            index < 4096,
            "mGBA child descriptor inventory exceeds limit"
        );
        let entry = entry?;
        let metadata = match std::fs::metadata(entry.path()) {
            Ok(metadata) => metadata,
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => continue,
            Err(error) => return Err(error.into()),
        };
        if metadata.dev() == selected.dev()
            && metadata.ino() == selected.ino()
            && metadata.rdev() == selected.rdev()
        {
            opened = true;
            break;
        }
    }
    Ok(opened)
}

pub(super) fn confirm(
    session: &NativeSession,
    child: &mut Child,
    cancel: &AtomicBool,
) -> Result<()> {
    let deadline = Instant::now() + Duration::from_secs(20);
    loop {
        cancelled(cancel)?;
        ensure!(
            child.try_wait()?.is_none(),
            "mGBA exited before controller handoff"
        );
        if let Some(pid) = native_pid(child.id(), &session.executable)?
            && ready(session, pid)?
        {
            session.verify()?;
            return Ok(());
        }
        ensure!(
            Instant::now() < deadline,
            "mGBA did not establish native controller routing before timeout"
        );
        std::thread::sleep(Duration::from_millis(25));
    }
}
