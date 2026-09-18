//! Single-instance guard for the desktop application.
//!
//! Lunchbox owns one library, one settings store and one set of running
//! emulators, so a second GUI process must not start. The first process takes
//! an advisory lock on `instance.lock` next to the state database and listens
//! on a loopback socket; a later launch connects, asks the owner to raise its
//! window, and exits.
//!
//! `fs2` file locking and `std::net` are used instead of Qt's `QLocalServer`
//! so the guard works identically on Linux, macOS and Windows without another
//! C++ bridge. The socket is loopback-only and its port is published inside the
//! user's own data directory.

use anyhow::{Context, Result};
use fs2::FileExt;
use std::io::{BufRead, BufReader, Write};
use std::net::{TcpListener, TcpStream};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::Duration;

/// Set by the listener thread and consumed by the Qt bridge on the GUI thread.
static RAISE_REQUESTED: AtomicBool = AtomicBool::new(false);

/// Keeps the advisory lock and listener alive for the process lifetime.
pub(crate) struct InstanceGuard {
    _lock: std::fs::File,
    port_path: PathBuf,
}

impl Drop for InstanceGuard {
    fn drop(&mut self) {
        // Remove only the published port; the lock file itself stays so the
        // inode is stable across processes.
        let _ = std::fs::remove_file(&self.port_path);
    }
}

fn paths() -> Result<(PathBuf, PathBuf)> {
    let directory = crate::settings::state_database_path()?
        .parent()
        .map(Path::to_path_buf)
        .context("state database path has no parent directory")?;
    std::fs::create_dir_all(&directory)
        .with_context(|| format!("creating {}", directory.display()))?;
    Ok((
        directory.join("instance.lock"),
        directory.join("instance.port"),
    ))
}

/// Take ownership of the instance slot, or ask the running owner to raise.
/// `Ok(None)` means another instance already owns the slot and was notified.
pub(crate) fn request_or_own() -> Result<Option<InstanceGuard>> {
    let (lock_path, port_path) = paths()?;
    request_paths(&lock_path, &port_path)
}

fn request_paths(lock_path: &Path, port_path: &Path) -> Result<Option<InstanceGuard>> {
    let lock = std::fs::OpenOptions::new()
        .create(true)
        .read(true)
        .write(true)
        .truncate(false)
        .open(lock_path)
        .with_context(|| format!("opening {}", lock_path.display()))?;
    match lock.try_lock_exclusive() {
        Ok(()) => {
            let listener = TcpListener::bind(("127.0.0.1", 0))
                .context("binding the Lunchbox instance socket")?;
            let port = listener
                .local_addr()
                .context("reading the Lunchbox instance socket address")?
                .port();
            // Publish the port atomically so a racing second launch never
            // reads a half-written number.
            let temporary = port_path.with_extension("port.tmp");
            std::fs::write(&temporary, port.to_string())
                .with_context(|| format!("writing {}", temporary.display()))?;
            std::fs::rename(&temporary, &port_path)
                .with_context(|| format!("publishing {}", port_path.display()))?;
            std::thread::Builder::new()
                .name("lunchbox-instance".into())
                .spawn(move || {
                    for stream in listener.incoming().flatten() {
                        accept(stream);
                    }
                })
                .context("spawning the Lunchbox instance listener")?;
            Ok(Some(InstanceGuard {
                _lock: lock,
                port_path: port_path.to_path_buf(),
            }))
        }
        Err(error) if error.kind() == std::io::ErrorKind::WouldBlock => {
            raise_existing(&port_path);
            Ok(None)
        }
        Err(error) => Err(error).context("locking the Lunchbox instance slot"),
    }
}

fn accept(mut stream: TcpStream) {
    let mut request = String::new();
    if let Ok(clone) = stream.try_clone() {
        let _ = BufReader::new(clone).read_line(&mut request);
    }
    if request.trim() == "raise" {
        RAISE_REQUESTED.store(true, Ordering::SeqCst);
    }
    let _ = stream.write_all(b"ok\n");
    let _ = stream.flush();
}

fn raise_existing(port_path: &Path) {
    // The owner publishes the port just after locking, so a simultaneous
    // launch retries briefly before giving up; the lock still prevents a
    // second library from starting either way.
    for _ in 0..20 {
        if let Ok(text) = std::fs::read_to_string(port_path)
            && let Ok(port) = text.trim().parse::<u16>()
            && let Ok(mut stream) = TcpStream::connect(("127.0.0.1", port))
        {
            let _ = stream.write_all(b"raise\n");
            let _ = stream.flush();
            return;
        }
        std::thread::sleep(Duration::from_millis(25));
    }
}

/// Consume a pending raise request. Called from the Qt thread.
pub(crate) fn take_raise_signal() -> bool {
    RAISE_REQUESTED.swap(false, Ordering::SeqCst)
}

#[cxx_qt::bridge]
pub mod qobject {
    unsafe extern "RustQt" {
        #[qobject]
        #[qml_element]
        type SingleInstance = super::SingleInstanceRust;

        /// True once per request from a later launch that this window should be
        /// raised and focused.
        #[qinvokable]
        fn take_raise_request(self: &SingleInstance) -> bool;
    }
}

#[derive(Default)]
pub struct SingleInstanceRust;

impl qobject::SingleInstance {
    pub fn take_raise_request(&self) -> bool {
        take_raise_signal()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn second_request_raises_the_owner_and_takes_no_new_slot() {
        let directory = tempfile::tempdir().unwrap();
        let lock = directory.path().join("instance.lock");
        let port = directory.path().join("instance.port");
        RAISE_REQUESTED.store(false, Ordering::SeqCst);
        let owner = request_paths(&lock, &port).unwrap();
        assert!(owner.is_some(), "the first request owns the slot");
        // The owner is listening once request_paths returns, so a second
        // request connects and asks it to raise without taking a new slot.
        let second = request_paths(&lock, &port).unwrap();
        assert!(second.is_none(), "the second request must not start a copy");
        let mut raised = false;
        for _ in 0..100 {
            if take_raise_signal() {
                raised = true;
                break;
            }
            std::thread::sleep(Duration::from_millis(10));
        }
        assert!(raised, "the owner must receive the raise request");
        assert!(!take_raise_signal(), "the raise signal is consumed once");
    }
}
