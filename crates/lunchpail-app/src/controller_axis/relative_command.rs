//! Owned child stdio transport for the opt-in frontend routing query and log.
//! Does not spawn a process, enable commands, open devices or start forwarding.
use super::relative_frontend::FrontendMouseObservation;
use anyhow::{Context, Result, ensure};
use std::io::{ErrorKind, Read, Write};
use std::os::fd::AsRawFd;
use std::process::{Child, ChildStderr, ChildStdin, ChildStdout};
use std::time::{Duration, Instant};

const QUERY: &[u8] = b"GET_CONFIG_PARAM lunchbox_mouse_routes_v1\n";
const MARKER: &str = "GET_CONFIG_PARAM lunchbox_mouse_routes_v1";

/// The launch owner retains the Child and must terminate it on channel failure.
/// Only use freshly spawned, identity-checked frontend pipes; no other writer
/// may issue commands. Stderr is drained and retained until startup is sealed.
pub struct RelativeCommandChannel {
    input: ChildStdin,
    output: ChildStdout,
    errors: ChildStderr,
    startup_log: Vec<u8>,
    log_partial: Vec<u8>,
    startup_sealed: bool,
    buffer: Vec<u8>,
    deadline: Option<Instant>,
    failed: bool,
}

fn nonblocking(fd: &impl AsRawFd) -> Result<()> {
    let raw = fd.as_raw_fd();
    // Borrowed live descriptor; preserve all existing file-status flags.
    let flags = unsafe { libc::fcntl(raw, libc::F_GETFL) };
    if flags < 0 || unsafe { libc::fcntl(raw, libc::F_SETFL, flags | libc::O_NONBLOCK) } < 0 {
        return Err(std::io::Error::last_os_error()).context("Configuring frontend command pipe");
    }
    Ok(())
}

impl RelativeCommandChannel {
    pub fn take_from_child(child: &mut Child) -> Result<Self> {
        ensure!(child.try_wait()?.is_none(), "Frontend has already exited");
        let input = child
            .stdin
            .as_ref()
            .context("Frontend stdin was not piped")?;
        let output = child
            .stdout
            .as_ref()
            .context("Frontend stdout was not piped")?;
        let errors = child
            .stderr
            .as_ref()
            .context("Frontend stderr was not piped")?;
        nonblocking(input)?;
        nonblocking(output)?;
        nonblocking(errors)?;
        Ok(Self {
            input: child.stdin.take().context("Frontend stdin disappeared")?,
            output: child.stdout.take().context("Frontend stdout disappeared")?,
            errors: child.stderr.take().context("Frontend stderr disappeared")?,
            startup_log: Vec::new(),
            log_partial: Vec::new(),
            startup_sealed: false,
            buffer: Vec::new(),
            deadline: None,
            failed: false,
        })
    }

    fn drain_stderr(&mut self) -> Result<()> {
        for _ in 0..64 {
            let mut bytes = [0u8; 4096];
            let count = match self.errors.read(&mut bytes) {
                Ok(0) => anyhow::bail!("Frontend stderr closed"),
                Ok(count) => count,
                Err(error) if error.kind() == ErrorKind::WouldBlock => return Ok(()),
                Err(error) if error.kind() == ErrorKind::Interrupted => continue,
                Err(error) => return Err(error.into()),
            };
            if !self.startup_sealed {
                ensure!(
                    self.startup_log.len() + count <= 8 * 1024 * 1024,
                    "Frontend startup log exceeds bounds"
                );
                self.startup_log.extend_from_slice(&bytes[..count]);
            }
            self.log_partial.extend_from_slice(&bytes[..count]);
            while let Some(end) = self.log_partial.iter().position(|byte| *byte == b'\n') {
                let line: Vec<_> = self.log_partial.drain(..=end).collect();
                ensure!(line.len() <= 8192, "Frontend stderr record exceeds bounds");
                if self.startup_sealed {
                    let marker = b"[udev] Mouse/Touch #";
                    ensure!(
                        !line.windows(marker.len()).any(|window| window == marker),
                        "Frontend mouse enumeration changed after startup"
                    );
                }
            }
            ensure!(
                self.log_partial.len() <= 8192,
                "Unterminated frontend stderr record exceeds bounds"
            );
        }
        anyhow::bail!("Frontend stderr exceeds bounded drain budget")
    }

    /// Consume this child's startup log after the initial command reply. Later
    /// stderr is drained without retaining an unbounded game-session log.
    pub fn seal_startup_log(&mut self) -> Result<String> {
        let result = (|| {
            ensure!(
                !self.failed && !self.startup_sealed && self.deadline.is_none(),
                "Frontend startup log cannot be sealed in the current query state"
            );
            self.drain_stderr()?;
            let log = String::from_utf8(std::mem::take(&mut self.startup_log))
                .context("Frontend startup log is not UTF-8")?;
            self.startup_sealed = true;
            Ok(log)
        })();
        if result.is_err() {
            self.failed = true;
        }
        result
    }

    /// Drain unsolicited output first, then issue one bounded atomic pipe write.
    /// The caller polls regularly and treats any error as a session failure.
    pub fn begin_query(&mut self, timeout: Duration) -> Result<()> {
        self.begin_command(QUERY, timeout)
    }

    pub fn begin_route_assignment(
        &mut self,
        routes: &std::collections::BTreeMap<u8, u32>,
        timeout: Duration,
    ) -> Result<()> {
        let command = super::relative_frontend::relative_route_command(routes)?;
        self.begin_command(command.as_bytes(), timeout)
    }

    fn begin_command(&mut self, command: &[u8], timeout: Duration) -> Result<()> {
        let result = (|| {
            ensure!(!self.failed, "Frontend command channel has failed");
            ensure!(
                self.deadline.is_none(),
                "Frontend routing query already pending"
            );
            ensure!(
                (Duration::from_millis(10)..=Duration::from_secs(30)).contains(&timeout),
                "Frontend routing query timeout is outside bounds"
            );
            self.poll()?;
            // Do not join a new response to a partial prior stdout record.
            ensure!(
                self.buffer.is_empty(),
                "Frontend stdout has an incomplete prior record"
            );
            let written = self.input.write(command)?;
            ensure!(
                written == command.len(),
                "Incomplete frontend routing query write"
            );
            self.deadline = Some(Instant::now() + timeout);
            Ok(())
        })();
        if result.is_err() {
            self.failed = true;
        }
        result
    }

    /// One bounded nonblocking drain. Non-query stdout lines are discarded;
    /// startup evidence is retained from this channel's owned stderr stream.
    /// A complete reply is returned only after the pipe reaches WouldBlock,
    /// so duplicate replies in the same drain cannot be mistaken for success.
    pub fn poll(&mut self) -> Result<Option<FrontendMouseObservation>> {
        let result = (|| {
            ensure!(!self.failed, "Frontend command channel has failed");
            self.drain_stderr()?;
            let mut state = None;
            for _ in 0..64 {
                ensure!(
                    self.deadline
                        .is_none_or(|deadline| Instant::now() < deadline),
                    "Frontend routing query timed out"
                );
                let mut bytes = [0u8; 4096];
                match self.output.read(&mut bytes) {
                    Ok(0) => anyhow::bail!("Frontend command output closed"),
                    Ok(count) => self.buffer.extend_from_slice(&bytes[..count]),
                    Err(error) if error.kind() == ErrorKind::WouldBlock => {
                        if state.is_some() {
                            self.deadline = None;
                        }
                        return Ok(state);
                    }
                    Err(error) if error.kind() == ErrorKind::Interrupted => continue,
                    Err(error) => return Err(error.into()),
                }
                while let Some(end) = self.buffer.iter().position(|byte| *byte == b'\n') {
                    let line: Vec<_> = self.buffer.drain(..=end).collect();
                    ensure!(line.len() <= 8192, "Frontend stdout record exceeds bounds");
                    let text =
                        std::str::from_utf8(&line).context("Frontend stdout is not UTF-8")?;
                    if text.contains(MARKER) {
                        ensure!(
                            self.deadline.is_some() && state.is_none(),
                            "Unsolicited or duplicate frontend routing reply"
                        );
                        state = Some(FrontendMouseObservation::from_command_reply(text)?);
                    }
                }
                ensure!(
                    self.buffer.len() <= 8192,
                    "Unterminated frontend stdout record exceeds bounds"
                );
            }
            anyhow::bail!("Frontend stdout exceeds bounded command-drain budget")
        })();
        if result.is_err() {
            self.failed = true;
        }
        result
    }
}
