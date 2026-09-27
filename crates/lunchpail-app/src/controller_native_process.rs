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
                if !status.success() {
                    // Surface the helper's own diagnostics: a bare status
                    // hides the cause (missing sandbox library, bad flag).
                    // Bounded to a short UTF-8-lossy tail of stderr.
                    let tail = std::fs::read(stderr.path().to_path_buf())
                        .map(|bytes| {
                            let text = String::from_utf8_lossy(&bytes);
                            let start = text.len().saturating_sub(2048);
                            text[start..].trim().to_owned()
                        })
                        .unwrap_or_default();
                    anyhow::bail!(
                        "Native controller helper failed: {status}{}",
                        if tail.is_empty() {
                            String::new()
                        } else {
                            format!(": {tail}")
                        }
                    );
                }
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

/// Directories of the shared libraries a host probe binary resolves to (its
/// loader search closure on this machine). Best-effort and host-side only:
/// empty when the loader map cannot be read, in which case callers fall back
/// to the target runtime directory alone.
pub(crate) fn host_library_dirs(program: &std::path::Path) -> Vec<std::path::PathBuf> {
    let mut ldd = crate::platform_process::host_command("ldd");
    ldd.arg(program);
    let cancel = AtomicBool::new(false);
    let Ok((out, _)) = capture(&mut ldd, &cancel) else {
        return Vec::new();
    };
    let Ok(text) = std::str::from_utf8(&out) else {
        return Vec::new();
    };
    let mut dirs = Vec::new();
    for line in text.lines() {
        let Some((_, target)) = line.trim().split_once(" => ") else {
            continue;
        };
        let path = target.split_whitespace().next().unwrap_or("");
        let path = std::path::PathBuf::from(path);
        if path.is_absolute()
            && path.is_file()
            && let Some(parent) = path.parent()
            && !dirs.contains(&parent.to_path_buf())
        {
            dirs.push(parent.to_path_buf());
        }
    }
    dirs
}

/// `LD_LIBRARY_PATH` value for sandbox probe commands. Host probe libraries
/// come first so a host-newer toolchain (Nix glibc 2.42) keeps its own
/// loader closure inside an older target runtime (Freedesktop 24.08 ships
/// glibc 2.40, which lacks private symbols the probe's libraries need);
/// the target runtime directory comes last so bare-soname loads issued from
/// inside the sandbox (sdl2-compat's SDL3) still resolve to audited files.
pub(crate) fn sandbox_library_path(
    program: &std::path::Path,
    runtime_dir: &str,
) -> std::ffi::OsString {
    let mut parts: Vec<std::ffi::OsString> = host_library_dirs(program)
        .into_iter()
        .map(std::ffi::OsString::from)
        .collect();
    parts.push(std::ffi::OsString::from(runtime_dir));
    let mut joined = std::ffi::OsString::new();
    for (index, part) in parts.iter().enumerate() {
        if index > 0 {
            joined.push(":");
        }
        joined.push(part);
    }
    joined
}
