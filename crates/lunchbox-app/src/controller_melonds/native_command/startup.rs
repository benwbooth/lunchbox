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
    let target = session.native_path.path().canonicalize()?;
    let child_target =
        std::path::PathBuf::from(format!("/proc/{pid}/root")).join(target.strip_prefix("/")?);
    let actual = std::fs::metadata(child_target)?;
    let generated = std::fs::metadata(session.config.path())?;
    ensure!(
        actual.dev() == generated.dev() && actual.ino() == generated.ino(),
        "melonDS child did not mount the generated configuration"
    );
    let mut opened = std::collections::BTreeSet::new();
    for (index, entry) in std::fs::read_dir(format!("/proc/{pid}/fd"))?.enumerate() {
        ensure!(index < 4096, "melonDS descriptor inventory exceeds limit");
        match std::fs::metadata(entry?.path()) {
            Ok(metadata) => {
                opened.insert((metadata.dev(), metadata.ino(), metadata.rdev()));
            }
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => continue,
            Err(error) => return Err(error.into()),
        }
    }
    for path in session.inputs.physical_paths.values() {
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
            "melonDS exited before controller handoff"
        );
        if let Some(pid) = native_pid(child.id(), &session.executable)?
            && ready(session, pid)?
        {
            session.native_path.verify()?;
            session.config.verify_source()?;
            session.inputs.verify(cancel)?;
            return Ok(());
        }
        ensure!(
            Instant::now() < deadline,
            "melonDS did not establish native controller routing before timeout"
        );
        std::thread::sleep(Duration::from_millis(25));
    }
}
