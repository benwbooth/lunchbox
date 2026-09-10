//! Startup routing evidence, not a gameplay or button-response test.
use super::*;
use crate::controller_native_process::native_pid;
use std::{
    os::unix::fs::MetadataExt,
    path::Path,
    process::Child,
    time::{Duration, Instant},
};

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
    let expected = std::fs::metadata(session.inputs.configuration.config_path())?;
    ensure!(
        actual.dev() == expected.dev() && actual.ino() == expected.ino(),
        "Snes9x child does not see its private snes9x.conf"
    );
    let mut opened = std::collections::BTreeSet::new();
    for (index, entry) in std::fs::read_dir(format!("/proc/{pid}/fd"))?.enumerate() {
        ensure!(index < 4096, "Snes9x descriptor inventory exceeds limit");
        match std::fs::metadata(entry?.path()) {
            Ok(metadata) => {
                opened.insert((metadata.dev(), metadata.ino(), metadata.rdev()));
            }
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => continue,
            Err(error) => return Err(error.into()),
        }
    }
    for path in &session.inputs.runtime_paths {
        let metadata = std::fs::metadata(path)?;
        if !opened.contains(&(metadata.dev(), metadata.ino(), metadata.rdev())) {
            return Ok(false);
        }
    }
    Ok(true)
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
            "Snes9x exited before controller handoff"
        );
        if let Some(pid) = native_pid(child.id(), &session.executable)?
            && ready(session, pid)?
        {
            session.verify(cancel)?;
            return Ok(());
        }
        ensure!(
            Instant::now() < deadline,
            "Snes9x did not establish native controller routing before timeout"
        );
        std::thread::sleep(Duration::from_millis(25));
    }
}
