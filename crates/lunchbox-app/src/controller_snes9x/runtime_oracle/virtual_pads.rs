//! Minimal Linux/x86_64 uinput driver for the standalone Snes9x runtime oracle.
//! Each input line is `PLAYER BUTTON PRESSED`; `pulse-all` exercises buttons
//! 0 through 11 in order on player 1 and then player 2.

use std::fs::{self, File, OpenOptions};
use std::io::{self, BufRead, Write};
use std::os::fd::AsRawFd;
use std::path::{Path, PathBuf};
use std::thread;
use std::time::{Duration, Instant};

#[repr(C)]
#[derive(Clone, Copy)]
struct Timeval {
    sec: i64,
    usec: i64,
}

#[repr(C)]
#[derive(Clone, Copy)]
struct InputEvent {
    time: Timeval,
    kind: u16,
    code: u16,
    value: i32,
}

unsafe extern "C" {
    fn ioctl(fd: i32, request: usize, ...) -> i32;
}

struct Pad {
    file: File,
    name: String,
    js: PathBuf,
}

impl Pad {
    fn create(number: u16) -> io::Result<Self> {
        let file = OpenOptions::new().write(true).open("/dev/uinput")?;
        let fd = file.as_raw_fd();
        for (request, value) in [(0x40045564usize, 0), (0x40045564, 1), (0x40045564, 3)] {
            if unsafe { ioctl(fd, request, value) } < 0 {
                return Err(io::Error::last_os_error());
            }
        }
        for code in 0x130..=0x13b {
            if unsafe { ioctl(fd, 0x40045565, code) } < 0 {
                return Err(io::Error::last_os_error());
            }
        }
        for code in [0u16, 1] {
            if unsafe { ioctl(fd, 0x40045567, i32::from(code)) } < 0 {
                return Err(io::Error::last_os_error());
            }
        }

        let name = format!("Lunchbox Snes9x hardware oracle P{number}");
        let mut setup = [0u8; 92];
        setup[..2].copy_from_slice(&0x06u16.to_ne_bytes());
        setup[2..4].copy_from_slice(&0x1209u16.to_ne_bytes());
        setup[4..6].copy_from_slice(&(0x6300u16 + number).to_ne_bytes());
        setup[8..8 + name.len()].copy_from_slice(name.as_bytes());
        if unsafe { ioctl(fd, 0x405c5503, setup.as_ptr()) } < 0 {
            return Err(io::Error::last_os_error());
        }
        for code in [0u16, 1] {
            let mut abs = [0u8; 28];
            abs[..2].copy_from_slice(&code.to_ne_bytes());
            abs[8..12].copy_from_slice(&(-32767i32).to_ne_bytes());
            abs[12..16].copy_from_slice(&32767i32.to_ne_bytes());
            if unsafe { ioctl(fd, 0x401c5504, abs.as_ptr()) } < 0 {
                return Err(io::Error::last_os_error());
            }
        }
        if unsafe { ioctl(fd, 0x5501) } < 0 {
            return Err(io::Error::last_os_error());
        }

        let deadline = Instant::now() + Duration::from_secs(10);
        let js = loop {
            let mut found = None;
            for entry in fs::read_dir("/sys/class/input")? {
                let entry = entry?;
                if entry.file_name().to_string_lossy().starts_with("js")
                    && fs::read_to_string(entry.path().join("device/name"))
                        .ok()
                        .as_deref()
                        .map(str::trim)
                        == Some(&name)
                {
                    found = Some(Path::new("/dev/input").join(entry.file_name()));
                    break;
                }
            }
            if let Some(path) = found {
                break path;
            }
            if Instant::now() >= deadline {
                return Err(io::Error::new(io::ErrorKind::TimedOut, "joydev node"));
            }
            thread::sleep(Duration::from_millis(25));
        };
        Ok(Self { file, name, js })
    }

    fn button(&mut self, index: u16, pressed: bool) -> io::Result<()> {
        for event in [
            InputEvent {
                time: Timeval { sec: 0, usec: 0 },
                kind: 1,
                code: 0x130 + index,
                value: i32::from(pressed),
            },
            InputEvent {
                time: Timeval { sec: 0, usec: 0 },
                kind: 0,
                code: 0,
                value: 0,
            },
        ] {
            let bytes = unsafe {
                std::slice::from_raw_parts(
                    (&event as *const InputEvent).cast::<u8>(),
                    std::mem::size_of::<InputEvent>(),
                )
            };
            self.file.write_all(bytes)?;
        }
        Ok(())
    }
}

impl Drop for Pad {
    fn drop(&mut self) {
        unsafe {
            ioctl(self.file.as_raw_fd(), 0x5502);
        }
    }
}

fn main() -> io::Result<()> {
    let mut pads = [Pad::create(1)?, Pad::create(2)?];
    for (index, pad) in pads.iter().enumerate() {
        println!("P{}\t{}\t{}", index + 1, pad.js.display(), pad.name);
    }
    println!("READY");
    io::stdout().flush()?;

    for line in io::stdin().lock().lines() {
        let line = line?;
        if line == "quit" {
            break;
        }
        if line == "pulse-all" {
            for (pad_index, pad) in pads.iter_mut().enumerate() {
                for button in 0..12 {
                    pad.button(button, true)?;
                    thread::sleep(Duration::from_millis(120));
                    pad.button(button, false)?;
                    thread::sleep(Duration::from_millis(120));
                    println!("PULSED {} {}", pad_index + 1, button);
                    io::stdout().flush()?;
                }
            }
            println!("PULSE-ALL-DONE");
            io::stdout().flush()?;
            continue;
        }

        let parts: Vec<_> = line.split_whitespace().collect();
        if parts.len() != 3 {
            return Err(io::Error::new(
                io::ErrorKind::InvalidInput,
                "PAD INDEX STATE",
            ));
        }
        let pad: usize = parts[0].parse().map_err(|_| io::ErrorKind::InvalidInput)?;
        let button: u16 = parts[1].parse().map_err(|_| io::ErrorKind::InvalidInput)?;
        let pressed: u8 = parts[2].parse().map_err(|_| io::ErrorKind::InvalidInput)?;
        if !(1..=2).contains(&pad) || button >= 12 || pressed > 1 {
            return Err(io::ErrorKind::InvalidInput.into());
        }
        pads[pad - 1].button(button, pressed != 0)?;
        println!("OK {pad} {button} {pressed}");
        io::stdout().flush()?;
    }
    Ok(())
}
