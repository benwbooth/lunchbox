//! Confirm the owned native child, not a helper's predicted startup state.
use super::*;
use std::os::unix::fs::MetadataExt;
use std::process::Child;
use std::sync::{Arc, Mutex, atomic::Ordering};
use std::time::{Duration, Instant};

const LIMIT: usize = 8 * 1024 * 1024;

struct Output {
    streams: Arc<Mutex<[Vec<u8>; 2]>>,
    collecting: Arc<AtomicBool>,
    overflow: Arc<AtomicBool>,
}

impl Drop for Output {
    fn drop(&mut self) {
        // Readers continue draining while the child lives, without retaining
        // gameplay logs or filling a pipe after startup confirmation returns.
        self.collecting.store(false, Ordering::Relaxed);
    }
}

impl Output {
    fn attach(child: &mut Child) -> Result<Self> {
        let output = Self {
            streams: Arc::new(Mutex::new([Vec::new(), Vec::new()])),
            collecting: Arc::new(AtomicBool::new(true)),
            overflow: Arc::new(AtomicBool::new(false)),
        };
        let readers: [Box<dyn Read + Send>; 2] = [
            Box::new(child.stdout.take().context("PPSSPP stdout is absent")?),
            Box::new(child.stderr.take().context("PPSSPP stderr is absent")?),
        ];
        for (stream, mut reader) in readers.into_iter().enumerate() {
            let bytes = output.streams.clone();
            let collecting = output.collecting.clone();
            let overflow = output.overflow.clone();
            std::thread::Builder::new()
                .name("ppsspp-startup-output".into())
                .spawn(move || {
                    let mut buffer = [0_u8; 4096];
                    loop {
                        let count = match reader.read(&mut buffer) {
                            Ok(0) => break,
                            Ok(count) => count,
                            Err(error) if error.kind() == std::io::ErrorKind::Interrupted => {
                                continue;
                            }
                            Err(_) => break,
                        };
                        if collecting.load(Ordering::Relaxed) {
                            let Ok(mut bytes) = bytes.lock() else {
                                break;
                            };
                            if bytes[stream].len() + count > LIMIT {
                                overflow.store(true, Ordering::Relaxed);
                            } else {
                                bytes[stream].extend_from_slice(&buffer[..count]);
                            }
                        }
                    }
                })?;
        }
        Ok(output)
    }

    fn confirms(&self, mappings: &[String]) -> Result<bool> {
        ensure!(
            !self.overflow.load(Ordering::Relaxed),
            "PPSSPP startup output exceeded limit"
        );
        let streams = self
            .streams
            .lock()
            .map_err(|_| anyhow::anyhow!("PPSSPP log reader failed"))?;
        for bytes in streams.iter() {
            let log = String::from_utf8_lossy(bytes);
            let mut found = Vec::new();
            for line in log
                .split_inclusive('\n')
                .filter(|line| line.ends_with('\n'))
            {
                let line = without_colors(line);
                if line.contains("loading control pad mappings from gamecontrollerdb.txt:") {
                    found.clear();
                } else if let Some((_, mapping)) = line.split_once("SUCCESS, mapping is: ") {
                    found.push(mapping.trim().to_owned());
                } else if line.contains("pad 1 has been assigned to control pad:") {
                    // In the pinned SDL frontend this marker follows the entire
                    // ordered setUpControllers loop, not one selected pad.
                    ensure!(
                        found == mappings,
                        "PPSSPP child controller mapping/order differs from the captured runtime"
                    );
                    return Ok(true);
                }
            }
        }
        Ok(false)
    }
}

fn without_colors(line: &str) -> String {
    let mut result = String::new();
    let mut chars = line.chars().peekable();
    while let Some(ch) = chars.next() {
        if ch == '\u{1b}' && chars.peek() == Some(&'[') {
            chars.next();
            for code in chars.by_ref() {
                if ('@'..='~').contains(&code) {
                    break;
                }
            }
        } else {
            result.push(ch);
        }
    }
    result
}

use crate::controller_native_process::native_pid;

fn verify_child(session: &PreparedLaunch, pid: u32) -> Result<()> {
    let maps = std::fs::read_to_string(format!("/proc/{pid}/maps"))?;
    let expected_sdl = session.setup.sdl_library.canonicalize()?;
    ensure!(
        maps.lines().any(|line| {
            let path = line
                .split_whitespace()
                .skip(5)
                .collect::<Vec<_>>()
                .join(" ")
                .replace("\\040", " ");
            Path::new(&path) == expected_sdl
        }),
        "PPSSPP child did not load the inspected SDL2 library"
    );
    let mounted = PathBuf::from(format!("/proc/{pid}/root")).join(
        session
            .setup
            .source_system
            .canonicalize()?
            .strip_prefix("/")?,
    );
    let actual = std::fs::metadata(mounted)?;
    let expected = std::fs::metadata(session.input.configuration.system_directory())?;
    ensure!(
        actual.dev() == expected.dev() && actual.ino() == expected.ino(),
        "PPSSPP child does not see the private SYSTEM overlay"
    );
    session.verify()
}

pub(super) fn confirm(
    session: &PreparedLaunch,
    child: &mut Child,
    cancel: &AtomicBool,
) -> Result<()> {
    let output = Output::attach(child)?;
    let deadline = Instant::now() + Duration::from_secs(20);
    loop {
        cancelled(cancel)?;
        ensure!(
            child.try_wait()?.is_none(),
            "PPSSPP exited before controller startup confirmation"
        );
        if output.confirms(&session.mappings)?
            && let Some(pid) = native_pid(child.id(), &session.executable)?
        {
            verify_child(session, pid)?;
            return Ok(());
        }
        ensure!(
            Instant::now() < deadline,
            "PPSSPP did not confirm controller startup before timeout"
        );
        std::thread::sleep(Duration::from_millis(25));
    }
}
