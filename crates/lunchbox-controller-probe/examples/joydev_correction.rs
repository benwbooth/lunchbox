//! Read-only kernel oracle: compare corrected evdev values with real joydev
//! startup events on an exact jsN/eventN pair. No synthetic input or calibration
//! changes. The controller must stay still across the short observation window.
#[cfg(not(target_os = "linux"))]
fn main() -> anyhow::Result<()> {
    anyhow::bail!("This oracle requires Linux")
}

#[cfg(target_os = "linux")]
fn main() -> anyhow::Result<()> {
    use anyhow::{Context, ensure};
    use lunchbox_controller_probe::linux_classic;
    use std::{
        collections::BTreeMap,
        fs,
        io::Read,
        os::{fd::AsRawFd, unix::fs::OpenOptionsExt},
        path::{Path, PathBuf},
        time::{Duration, Instant},
    };
    let args: Vec<_> = std::env::args_os().skip(1).collect();
    ensure!(
        args.len() == 2,
        "Pass /dev/input/jsN and its exact /dev/input/eventN"
    );
    let joystick = PathBuf::from(&args[0]);
    let event = PathBuf::from(&args[1]);
    for (path, prefix) in [(&joystick, "js"), (&event, "event")] {
        ensure!(
            path.parent() == Some(Path::new("/dev/input"))
                && path
                    .file_name()
                    .and_then(|s| s.to_str())
                    .and_then(|s| s.strip_prefix(prefix))
                    .is_some_and(|s| !s.is_empty() && s.bytes().all(|b| b.is_ascii_digit())),
            "Expected an exact input device path"
        );
    }
    let sys = |path: &Path| {
        Path::new("/sys/class/input")
            .join(path.file_name().unwrap())
            .join("device")
            .canonicalize()
    };
    ensure!(
        sys(&joystick)? == sys(&event)?,
        "Joystick and evdev nodes are not the same physical input device"
    );
    let map = linux_classic::read(&joystick)?;
    ensure!(!map.axis_corrections.is_empty(), "No axes to verify");
    let evdev = fs::File::open(&event)?;
    let sample = || -> anyhow::Result<BTreeMap<u8, i32>> {
        map.axis_corrections
            .keys()
            .map(|&code| {
                let mut info = [0i32; 6];
                ensure!(
                    unsafe {
                        libc::ioctl(
                            evdev.as_raw_fd(),
                            (0x80184540 + u32::from(code)) as libc::c_ulong,
                            info.as_mut_ptr(),
                        )
                    } >= 0,
                    "Cannot read evdev axis {code}"
                );
                Ok((code, info[0]))
            })
            .collect()
    };
    let before = sample()?;
    let mut stream = fs::OpenOptions::new()
        .read(true)
        .custom_flags(libc::O_NONBLOCK)
        .open(&joystick)?;
    let mut axis_map = [0u8; 64];
    ensure!(
        unsafe {
            libc::ioctl(
                stream.as_raw_fd(),
                0x80406a32 as libc::c_ulong,
                axis_map.as_mut_ptr(),
            )
        } >= 0,
        "Cannot read joystick axis map"
    );
    let started = Instant::now();
    let mut observed = BTreeMap::new();
    while observed.len() < map.axis_corrections.len() {
        ensure!(
            started.elapsed() < Duration::from_millis(250),
            "Missing joystick startup axis events"
        );
        let mut bytes = [0u8; 512];
        match stream.read(&mut bytes) {
            Ok(count) => {
                ensure!(count > 0 && count % 8 == 0, "Invalid joystick event frame");
                for frame in bytes[..count].chunks_exact(8) {
                    if frame[6] == 0x82 {
                        // JS_EVENT_INIT | JS_EVENT_AXIS
                        let code = *axis_map
                            .get(usize::from(frame[7]))
                            .context("Invalid joystick axis index")?;
                        ensure!(
                            map.axis_corrections.contains_key(&code),
                            "Unknown physical startup axis"
                        );
                        ensure!(
                            observed
                                .insert(code, i16::from_ne_bytes([frame[4], frame[5]]))
                                .is_none(),
                            "Duplicate startup axis"
                        );
                    }
                }
            }
            Err(error) if error.kind() == std::io::ErrorKind::WouldBlock => {
                std::thread::sleep(Duration::from_millis(1))
            }
            Err(error) => return Err(error.into()),
        }
    }
    ensure!(
        before == sample()?,
        "Controller moved during observation; retry while it is still"
    );
    ensure!(
        map == linux_classic::read(&joystick)?,
        "Kernel numbering or correction changed during observation"
    );
    let mut evidence = Vec::new();
    for (code, raw) in before {
        let predicted = map.axis_corrections[&code].apply(raw)?;
        ensure!(
            observed[&code] == predicted,
            "Kernel axis {code} differs: predicted {predicted}, observed {}",
            observed[&code]
        );
        evidence.push(serde_json::json!({ "physical_code": code, "evdev_value": raw, "predicted": predicted, "observed": observed[&code] }));
    }
    println!(
        "{}",
        serde_json::to_string_pretty(
            &serde_json::json!({"joystick":joystick,"evdev":event,"verified_axes":evidence})
        )?
    );
    Ok(())
}
