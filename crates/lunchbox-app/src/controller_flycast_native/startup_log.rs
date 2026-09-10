//! Bounded startup console capture. Readers keep draining after the snapshot
//! fills so an emulator cannot block on a full stdout/stderr pipe.
use anyhow::{Result, ensure};
use std::{
    io::Read,
    sync::{Arc, Mutex},
};

const LIMIT: usize = 4 * 1024 * 1024;
#[derive(Default)]
struct Buffer {
    bytes: Vec<u8>,
    overflow: bool,
    failed: bool,
}

pub(crate) struct StartupLog {
    streams: Vec<Arc<Mutex<Buffer>>>,
}

impl StartupLog {
    pub(crate) fn attach(child: &mut std::process::Child) -> Result<Self> {
        let stdout = child
            .stdout
            .take()
            .ok_or_else(|| anyhow::anyhow!("Flycast stdout pipe is missing"))?;
        let stderr = child
            .stderr
            .take()
            .ok_or_else(|| anyhow::anyhow!("Flycast stderr pipe is missing"))?;
        // Flycast does not use a controller protocol on stdin.
        drop(child.stdin.take());
        let mut streams = Vec::new();
        for mut reader in [Box::new(stdout) as Box<dyn Read + Send>, Box::new(stderr)] {
            let buffer = Arc::new(Mutex::new(Buffer::default()));
            let target = buffer.clone();
            std::thread::Builder::new()
                .name("flycast-console".into())
                .spawn(move || {
                    let mut chunk = [0u8; 8192];
                    loop {
                        match reader.read(&mut chunk) {
                            Ok(0) => break,
                            Ok(count) => {
                                let Ok(mut buffer) = target.lock() else {
                                    break;
                                };
                                let keep = count.min(LIMIT.saturating_sub(buffer.bytes.len()));
                                buffer.bytes.extend_from_slice(&chunk[..keep]);
                                buffer.overflow |= keep != count;
                            }
                            Err(error) if error.kind() == std::io::ErrorKind::Interrupted => {
                                continue;
                            }
                            Err(_) => {
                                if let Ok(mut buffer) = target.lock() {
                                    buffer.failed = true;
                                }
                                break;
                            }
                        }
                    }
                })?;
            streams.push(buffer);
        }
        Ok(Self { streams })
    }

    pub(crate) fn opened(
        &self,
    ) -> Result<std::collections::BTreeMap<i32, super::routing::OpenedJoystick>> {
        let mut all = std::collections::BTreeMap::new();
        for stream in &self.streams {
            let buffer = stream
                .lock()
                .map_err(|_| anyhow::anyhow!("Flycast console reader failed"))?;
            ensure!(
                !buffer.failed && !buffer.overflow,
                "Flycast startup console failed or exceeded limit"
            );
            // Only decode complete lines; the final UTF-8 character may be split.
            let end = buffer
                .bytes
                .iter()
                .rposition(|byte| *byte == b'\n')
                .map_or(0, |index| index + 1);
            let text = std::str::from_utf8(&buffer.bytes[..end])?;
            for (id, record) in super::routing::opened_joysticks(text)? {
                if let Some(previous) = all.insert(id, record.clone()) {
                    ensure!(
                        previous == record,
                        "Flycast console streams disagree about joystick identity"
                    );
                }
            }
        }
        Ok(all)
    }
}
