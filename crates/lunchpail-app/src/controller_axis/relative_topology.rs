//! Detect input hotplug events across a frontend startup and transport lifetime.
//! Subscribe after virtual endpoints exist, before starting frontend discovery.
//! This is a polling guard, not atomic coordination with frontend reindexing.
use anyhow::{Result, ensure};
use std::os::fd::{AsRawFd, FromRawFd, OwnedFd};

pub struct RelativeTopologyGuard {
    socket: OwnedFd,
    invalidated: bool,
}

impl RelativeTopologyGuard {
    pub fn subscribe() -> Result<Self> {
        let fd = unsafe {
            libc::socket(
                libc::AF_NETLINK,
                libc::SOCK_DGRAM | libc::SOCK_NONBLOCK | libc::SOCK_CLOEXEC,
                libc::NETLINK_KOBJECT_UEVENT,
            )
        };
        ensure!(
            fd >= 0,
            "Cannot monitor input topology: {}",
            std::io::Error::last_os_error()
        );
        let socket = unsafe { OwnedFd::from_raw_fd(fd) };
        let mut address: libc::sockaddr_nl = unsafe { std::mem::zeroed() };
        address.nl_family = libc::AF_NETLINK as libc::sa_family_t;
        address.nl_groups = 1; // Kernel uevents, not userspace udev multicast.
        ensure!(
            unsafe {
                libc::bind(
                    socket.as_raw_fd(),
                    (&address as *const libc::sockaddr_nl).cast(),
                    std::mem::size_of_val(&address) as libc::socklen_t,
                )
            } == 0,
            "Cannot subscribe to input topology: {}",
            std::io::Error::last_os_error()
        );
        Ok(Self {
            socket,
            invalidated: false,
        })
    }

    /// Any input event invalidates startup numbering permanently. Queue loss,
    /// malformed messages and an excessive backlog also stop the session; never
    /// drain and then treat a potentially changed table as trustworthy again.
    pub fn verify(&mut self) -> Result<()> {
        ensure!(!self.invalidated, "Input topology guard is invalidated");
        let result = self.drain();
        if result.is_err() {
            self.invalidated = true;
        }
        result
    }

    fn drain(&self) -> Result<()> {
        let mut buffer = [0u8; 65536];
        for _ in 0..64 {
            let mut sender: libc::sockaddr_nl = unsafe { std::mem::zeroed() };
            let mut length = std::mem::size_of_val(&sender) as libc::socklen_t;
            let received = unsafe {
                libc::recvfrom(
                    self.socket.as_raw_fd(),
                    buffer.as_mut_ptr().cast(),
                    buffer.len(),
                    libc::MSG_DONTWAIT | libc::MSG_TRUNC,
                    (&mut sender as *mut libc::sockaddr_nl).cast(),
                    &mut length,
                )
            };
            if received < 0 {
                let error = std::io::Error::last_os_error();
                if error.kind() == std::io::ErrorKind::WouldBlock {
                    return Ok(());
                }
                if error.kind() == std::io::ErrorKind::Interrupted {
                    continue;
                }
                return Err(error.into()); // Includes ENOBUFS: events were lost.
            }
            ensure!(
                length as usize == std::mem::size_of_val(&sender)
                    && sender.nl_family == libc::AF_NETLINK as libc::sa_family_t
                    && sender.nl_pid == 0,
                "Unexpected input topology event sender"
            );
            ensure!(
                received > 0 && received as usize <= buffer.len(),
                "Truncated input topology event"
            );
            let message = &buffer[..received as usize];
            ensure!(
                message.last() == Some(&0),
                "Unterminated input topology event"
            );
            let mut subsystem = None;
            for field in message.split(|byte| *byte == 0) {
                if let Some(value) = field.strip_prefix(b"SUBSYSTEM=") {
                    ensure!(
                        subsystem.replace(value).is_none(),
                        "Ambiguous input topology event"
                    );
                }
            }
            let subsystem =
                subsystem.ok_or_else(|| anyhow::anyhow!("Missing topology event subsystem"))?;
            ensure!(
                subsystem != b"input",
                "Input devices changed; frontend mouse routing must be resolved again"
            );
        }
        anyhow::bail!("Input topology event backlog exceeds the polling bound")
    }
}
