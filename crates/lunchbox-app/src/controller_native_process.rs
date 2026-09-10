//! Bounded native frontend helper execution; never called from settings review.
use anyhow::{Result, ensure};
use std::process::{Command, Stdio};
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::{Duration, Instant};

const OUTPUT_LIMIT: usize = 8 * 1024 * 1024;

// Bubblewrap may retain a monitor process. Resolve only its live descendants,
// never a name search across unrelated running emulator instances.
pub(crate) fn native_pid(root: u32, executable: &std::path::Path) -> Result<Option<u32>> {
    let mut pending = vec![root];
    let mut visited = std::collections::BTreeSet::new();
    let mut found = None;
    while let Some(pid) = pending.pop() {
        ensure!(
            visited.len() < 64,
            "Native frontend launch process tree exceeds limit"
        );
        if !visited.insert(pid) {
            continue;
        }
        if std::fs::canonicalize(format!("/proc/{pid}/exe"))
            .ok()
            .as_deref()
            == Some(executable)
        {
            ensure!(
                found.is_none(),
                "Multiple native frontend children in launch tree"
            );
            found = Some(pid);
        }
        match std::fs::read_to_string(format!("/proc/{pid}/task/{pid}/children")) {
            Ok(children) => {
                for child in children.split_whitespace() {
                    pending.push(child.parse::<u32>()?);
                }
            }
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
            Err(error) => return Err(error.into()),
        }
    }
    Ok(found)
}

pub(crate) fn cancelled(cancel: &AtomicBool) -> Result<()> {
    ensure!(
        !cancel.load(Ordering::Relaxed),
        "{}",
        crate::rom_launch_preparation::LAUNCH_CANCELLED_ERROR
    );
    Ok(())
}

/// Drain helpers to files so full stdout/stderr pipes cannot deadlock a worker.
/// Both elapsed time and output size are bounded; any error kills and reaps it.
pub(crate) fn capture(command: &mut Command, cancel: &AtomicBool) -> Result<(Vec<u8>, Vec<u8>)> {
    cancelled(cancel)?;
    let stdout = tempfile::NamedTempFile::new()?;
    let stderr = tempfile::NamedTempFile::new()?;
    let mut child = command
        .stdin(Stdio::null())
        .stdout(stdout.reopen()?)
        .stderr(stderr.reopen()?)
        .spawn()?;
    let result = (|| {
        let deadline = Instant::now() + Duration::from_secs(15);
        loop {
            cancelled(cancel)?;
            ensure!(
                stdout.as_file().metadata()?.len() <= OUTPUT_LIMIT as u64
                    && stderr.as_file().metadata()?.len() <= OUTPUT_LIMIT as u64,
                "Native controller helper output exceeded limit"
            );
            if let Some(status) = child.try_wait()? {
                ensure!(
                    status.success(),
                    "Native controller helper failed: {status}"
                );
                let out = std::fs::read(stdout.path())?;
                let err = std::fs::read(stderr.path())?;
                ensure!(
                    out.len() <= OUTPUT_LIMIT && err.len() <= OUTPUT_LIMIT,
                    "Native controller helper output exceeded limit"
                );
                return Ok((out, err));
            }
            ensure!(
                Instant::now() < deadline,
                "Native controller helper timed out"
            );
            std::thread::sleep(Duration::from_millis(20));
        }
    })();
    if result.is_err() {
        let _ = child.kill();
        let _ = child.wait();
    }
    result
}
