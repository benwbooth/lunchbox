//! Startup routing evidence, not a gameplay or button-response test.
use super::*;
use crate::controller_native_process::native_pid;
use std::{
    os::unix::fs::MetadataExt,
    process::Child,
    time::{Duration, Instant},
};

fn ready(session: &NativeSession, pid: u32) -> Result<bool> {
    session.inputs.configuration.verify_child_mounts(pid)?;
    let mut opened = std::collections::BTreeSet::new();
    for (index, entry) in std::fs::read_dir(format!("/proc/{pid}/fd"))?.enumerate() {
        ensure!(index < 4096, "Mednafen descriptor inventory exceeds limit");
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
            "Mednafen exited before controller handoff"
        );
        if let Some(pid) = native_pid(child.id(), &session.executable)?
            && ready(session, pid)?
        {
            session.verify(cancel)?;
            return Ok(());
        }
        ensure!(
            Instant::now() < deadline,
            "Mednafen did not establish native controller routing before timeout"
        );
        std::thread::sleep(Duration::from_millis(25));
    }
}
