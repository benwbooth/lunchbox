//! Deterministic source pads for the opt-in puNES Flatpak runtime oracle.

#[cfg(all(target_os = "linux", not(target_pointer_width = "64")))]
compile_error!("the puNES oracle pad driver supports only audited 64-bit Linux input_event");

#[cfg(target_os = "linux")]
mod linux {
    use std::{
        fs::{File, OpenOptions},
        io::{self, BufRead, Write},
        os::fd::AsRawFd,
        path::{Path, PathBuf},
        thread,
        time::{Duration, Instant},
    };

    const BUS: u16 = 0x06;
    const VENDOR: u16 = 0x1209;
    // Deliberately outside the production target range 4c51/4c52. These are
    // deterministic *source* pads; the production bridge must create and prove
    // its separate target pads rather than accidentally accepting the oracle.
    const PRODUCT_BASE: u16 = 0x4c60;
    const VERSION: u16 = 0x0001;
    const BUTTONS: [u16; 8] = [0x130, 0x131, 0x13a, 0x13b, 0x220, 0x221, 0x222, 0x223];

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

    fn guid(player: u16) -> String {
        let product = PRODUCT_BASE + player;
        format!(
            "{{{:04X}{:04X}-{:04X}-{:04X}-{:04X}-{:04X}{:04X}{:04X}}}",
            BUS.wrapping_sub(500),
            0u16.wrapping_sub(100),
            VENDOR,
            VENDOR.wrapping_sub(200),
            product,
            product.wrapping_sub(300),
            VERSION,
            VERSION.wrapping_sub(400)
        )
    }

    struct Pad {
        file: File,
        event: PathBuf,
        joystick: PathBuf,
        player: u16,
    }

    impl Pad {
        fn create(player: u16) -> io::Result<Self> {
            let file = OpenOptions::new().write(true).open("/dev/uinput")?;
            let fd = file.as_raw_fd();
            if unsafe { ioctl(fd, 0x40045564, 1) } < 0 {
                return Err(io::Error::last_os_error()); // UI_SET_EVBIT(EV_KEY)
            }
            for code in BUTTONS {
                if unsafe { ioctl(fd, 0x40045565, i32::from(code)) } < 0 {
                    return Err(io::Error::last_os_error()); // UI_SET_KEYBIT
                }
            }
            let name = format!("Lunchbox puNES oracle source P{player}");
            let mut setup = [0u8; 92];
            setup[..2].copy_from_slice(&BUS.to_ne_bytes());
            setup[2..4].copy_from_slice(&VENDOR.to_ne_bytes());
            setup[4..6].copy_from_slice(&(PRODUCT_BASE + player).to_ne_bytes());
            setup[6..8].copy_from_slice(&VERSION.to_ne_bytes());
            setup[8..8 + name.len()].copy_from_slice(name.as_bytes());
            if unsafe { ioctl(fd, 0x405c5503, setup.as_ptr()) } < 0 {
                return Err(io::Error::last_os_error()); // UI_DEV_SETUP
            }
            if unsafe { ioctl(fd, 0x5501) } < 0 {
                return Err(io::Error::last_os_error()); // UI_DEV_CREATE
            }
            let mut system_name = [0u8; 128];
            let request = 0x8000_0000usize | (128 << 16) | (0x55 << 8) | 44;
            if unsafe { ioctl(fd, request, system_name.as_mut_ptr()) } < 0 {
                return Err(io::Error::last_os_error());
            }
            let end = system_name
                .iter()
                .position(|byte| *byte == 0)
                .ok_or(io::ErrorKind::InvalidData)?;
            let system_name =
                std::str::from_utf8(&system_name[..end]).map_err(|_| io::ErrorKind::InvalidData)?;
            let deadline = Instant::now() + Duration::from_secs(10);
            let (event, joystick) = loop {
                let root = Path::new("/sys/class/input").join(system_name);
                let found = std::fs::read_dir(root).ok().map(|entries| {
                    entries
                        .filter_map(Result::ok)
                        .fold((None, None), |(event, joystick), entry| {
                            let name = entry.file_name();
                            let Some(name) = name.to_str() else {
                                return (event, joystick);
                            };
                            let event = event.or_else(|| {
                                name.strip_prefix("event")
                                    .is_some_and(|index| {
                                        !index.is_empty()
                                            && index.bytes().all(|byte| byte.is_ascii_digit())
                                    })
                                    .then(|| Path::new("/dev/input").join(name))
                            });
                            let joystick = joystick.or_else(|| {
                                name.strip_prefix("js")
                                    .is_some_and(|index| {
                                        !index.is_empty()
                                            && index.bytes().all(|byte| byte.is_ascii_digit())
                                    })
                                    .then(|| Path::new("/dev/input").join(name))
                            });
                            (event, joystick)
                        })
                });
                if let Some((Some(event), Some(joystick))) = found
                    && event.exists()
                    && joystick.exists()
                {
                    break (event, joystick);
                }
                if Instant::now() >= deadline {
                    return Err(io::Error::new(
                        io::ErrorKind::TimedOut,
                        "puNES oracle source nodes",
                    ));
                }
                thread::sleep(Duration::from_millis(25));
            };
            Ok(Self {
                file,
                event,
                joystick,
                player,
            })
        }

        fn button(&mut self, index: usize, pressed: bool) -> io::Result<()> {
            let code = *BUTTONS.get(index).ok_or(io::ErrorKind::InvalidInput)?;
            for event in [
                InputEvent {
                    time: Timeval { sec: 0, usec: 0 },
                    kind: 1,
                    code,
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
            unsafe { ioctl(self.file.as_raw_fd(), 0x5502) }; // UI_DEV_DESTROY
        }
    }

    pub(super) fn run() -> io::Result<()> {
        let mut pads = [Pad::create(1)?, Pad::create(2)?];
        for pad in &pads {
            println!(
                "P{}\t{}\t{}\t{}",
                pad.player,
                pad.joystick.display(),
                pad.event.display(),
                guid(pad.player)
            );
        }
        println!("READY");
        io::stdout().flush()?;
        for line in io::stdin().lock().lines() {
            let line = line?;
            if line == "quit" {
                break;
            }
            if line == "pulse-all" {
                for player in 1..=2 {
                    for button in 0..8 {
                        pulse(&mut pads, player, button)?;
                        println!("PULSED {player} {button}");
                        io::stdout().flush()?;
                    }
                }
                println!("PULSE-ALL-DONE");
                io::stdout().flush()?;
                continue;
            }
            let fields = line.split_whitespace().collect::<Vec<_>>();
            if fields.len() != 3 || fields[0] != "pulse" {
                return Err(io::Error::new(
                    io::ErrorKind::InvalidInput,
                    "expected pulse PLAYER BUTTON, pulse-all, or quit",
                ));
            }
            let player = fields[1]
                .parse::<usize>()
                .map_err(|_| io::ErrorKind::InvalidInput)?;
            let button = fields[2]
                .parse::<usize>()
                .map_err(|_| io::ErrorKind::InvalidInput)?;
            pulse(&mut pads, player, button)?;
            println!("PULSED {player} {button}");
            io::stdout().flush()?;
        }
        Ok(())
    }

    fn pulse(pads: &mut [Pad; 2], player: usize, button: usize) -> io::Result<()> {
        if !(1..=2).contains(&player) || button >= BUTTONS.len() {
            return Err(io::ErrorKind::InvalidInput.into());
        }
        pads[player - 1].button(button, true)?;
        thread::sleep(Duration::from_millis(150));
        pads[player - 1].button(button, false)?;
        thread::sleep(Duration::from_millis(150));
        Ok(())
    }

    #[cfg(test)]
    mod tests {
        use super::*;

        #[test]
        fn source_identity_is_distinct_from_production_targets() {
            assert_eq!(guid(1), "{FE12FF9C-1209-1141-4C61-4B350001FE71}");
            assert_eq!(guid(2), "{FE12FF9C-1209-1141-4C62-4B360001FE71}");
            assert_eq!(BUTTONS.len(), 8);
        }
    }
}

#[cfg(target_os = "linux")]
fn main() -> std::io::Result<()> {
    linux::run()
}

#[cfg(not(target_os = "linux"))]
fn main() {
    eprintln!("the puNES Flatpak oracle pad driver requires Linux");
    std::process::exit(1);
}
