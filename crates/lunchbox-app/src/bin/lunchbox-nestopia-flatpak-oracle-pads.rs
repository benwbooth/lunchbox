//! Two deterministic Linux joydev pads for the opt-in Nestopia Flatpak oracle.

#[cfg(target_os = "linux")]
mod linux {
    use std::{
        fs::{self, File, OpenOptions},
        io::{self, BufRead, Write},
        os::fd::AsRawFd,
        path::{Path, PathBuf},
        thread,
        time::{Duration, Instant},
    };

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
            for event_kind in [1, 3] {
                if unsafe { ioctl(fd, 0x40045564, event_kind) } < 0 {
                    // UI_SET_EVBIT EV_KEY / EV_ABS
                    return Err(io::Error::last_os_error());
                }
            }
            for code in 0x130..=0x137 {
                if unsafe { ioctl(fd, 0x40045565, code) } < 0 {
                    // UI_SET_KEYBIT
                    return Err(io::Error::last_os_error());
                }
            }
            for code in [0u16, 1] {
                if unsafe { ioctl(fd, 0x40045567, i32::from(code)) } < 0 {
                    // UI_SET_ABSBIT
                    return Err(io::Error::last_os_error());
                }
            }
            let name = format!("Lunchbox Nestopia hardware oracle P{number}");
            let mut setup = [0u8; 92];
            setup[..2].copy_from_slice(&0x06u16.to_ne_bytes());
            setup[2..4].copy_from_slice(&0x1209u16.to_ne_bytes());
            setup[4..6].copy_from_slice(&(0x6e00u16 + number).to_ne_bytes());
            setup[8..8 + name.len()].copy_from_slice(name.as_bytes());
            if unsafe { ioctl(fd, 0x405c5503, setup.as_ptr()) } < 0 {
                // UI_DEV_SETUP
                return Err(io::Error::last_os_error());
            }
            for code in [0u16, 1] {
                let mut abs = [0u8; 28];
                abs[..2].copy_from_slice(&code.to_ne_bytes());
                abs[8..12].copy_from_slice(&(-32767i32).to_ne_bytes());
                abs[12..16].copy_from_slice(&32767i32.to_ne_bytes());
                if unsafe { ioctl(fd, 0x401c5504, abs.as_ptr()) } < 0 {
                    // UI_ABS_SETUP
                    return Err(io::Error::last_os_error());
                }
            }
            if unsafe { ioctl(fd, 0x5501) } < 0 {
                // UI_DEV_CREATE
                return Err(io::Error::last_os_error());
            }
            let deadline = Instant::now() + Duration::from_secs(10);
            let js = loop {
                let found = fs::read_dir("/sys/class/input")?.find_map(|entry| {
                    let entry = entry.ok()?;
                    if !entry.file_name().to_string_lossy().starts_with("js") {
                        return None;
                    }
                    (fs::read_to_string(entry.path().join("device/name"))
                        .ok()?
                        .trim()
                        == name)
                        .then(|| Path::new("/dev/input").join(entry.file_name()))
                });
                if let Some(path) = found {
                    break path;
                }
                if Instant::now() >= deadline {
                    return Err(io::Error::new(
                        io::ErrorKind::TimedOut,
                        "Nestopia oracle joydev node",
                    ));
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
            } // UI_DEV_DESTROY
        }
    }

    pub(super) fn run() -> io::Result<()> {
        let mut pads = [Pad::create(1)?, Pad::create(2)?];
        for (index, pad) in pads.iter().enumerate() {
            println!("P{}\t{}\t{}", index + 1, pad.js.display(), pad.name);
        }
        println!("READY");
        io::stdout().flush()?;
        for line in io::stdin().lock().lines() {
            match line?.as_str() {
                "quit" => break,
                "pulse-all" => {
                    for (pad_index, pad) in pads.iter_mut().enumerate() {
                        for button in 0..8 {
                            pad.button(button, true)?;
                            thread::sleep(Duration::from_millis(150));
                            pad.button(button, false)?;
                            thread::sleep(Duration::from_millis(150));
                            println!("PULSED {} {}", pad_index + 1, button);
                            io::stdout().flush()?;
                        }
                    }
                    println!("PULSE-ALL-DONE");
                    io::stdout().flush()?;
                }
                command => {
                    let fields: Vec<_> = command.split_whitespace().collect();
                    if fields.len() != 3 || fields[0] != "pulse" {
                        return Err(io::Error::new(
                            io::ErrorKind::InvalidInput,
                            "expected pulse PLAYER BUTTON, pulse-all, or quit",
                        ));
                    }
                    let player: usize =
                        fields[1].parse().map_err(|_| io::ErrorKind::InvalidInput)?;
                    let button: u16 = fields[2].parse().map_err(|_| io::ErrorKind::InvalidInput)?;
                    if !(1..=2).contains(&player) || button >= 8 {
                        return Err(io::ErrorKind::InvalidInput.into());
                    }
                    pads[player - 1].button(button, true)?;
                    thread::sleep(Duration::from_millis(150));
                    pads[player - 1].button(button, false)?;
                    thread::sleep(Duration::from_millis(150));
                    println!("PULSED {player} {button}");
                    io::stdout().flush()?;
                }
            }
        }
        Ok(())
    }
}

#[cfg(target_os = "linux")]
fn main() -> std::io::Result<()> {
    linux::run()
}

#[cfg(not(target_os = "linux"))]
fn main() {
    eprintln!("the Nestopia Flatpak oracle pad driver requires Linux");
    std::process::exit(1);
}
