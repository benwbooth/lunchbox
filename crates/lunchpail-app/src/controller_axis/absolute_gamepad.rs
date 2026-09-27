//! Session-owned calibrated absolute aim plus optional gamepad input. The
//! publisher is a joystick, never a desktop mouse or keyboard.
use super::absolute::AbsoluteReader;
use super::absolute_settings::AbsoluteDeviceSettings;
use super::{
    GamepadAxis, GamepadBridge, GamepadControl, GamepadFrame, GamepadReader, VirtualGamepad,
};
use anyhow::{Result, ensure};
use std::path::Path;
use std::sync::{
    Arc, Mutex,
    atomic::{AtomicBool, Ordering},
    mpsc,
};
use std::time::{Duration, Instant};

/// The same deterministic reservation is used by saved setup validation and
/// live construction. This reads frame metadata only, never physical devices.
pub fn output_axes(frame: Option<&GamepadFrame>) -> Result<[u16; 2]> {
    let available: Vec<_> = (0u16..=10)
        .filter(|code| frame.is_none_or(|frame| !frame.axes.contains_key(code)))
        .take(2)
        .collect();
    ensure!(
        available.len() == 2,
        "No two independent virtual axes remain for absolute aim"
    );
    Ok([available[0], available[1]])
}

pub fn start(
    settings: &AbsoluteDeviceSettings,
    pad: Option<(&Path, GamepadFrame, &Path)>,
    cancel: &AtomicBool,
) -> Result<(GamepadBridge, [u16; 2])> {
    ensure!(
        !cancel.load(Ordering::Relaxed),
        "Absolute aim startup cancelled"
    );
    settings.validate()?;
    let (buttons, mut axes) = match &pad {
        Some((_, frame, _)) => (frame.buttons.clone(), frame.axes.clone()),
        None => (Default::default(), Default::default()),
    };
    let output_axes = output_axes(pad.as_ref().map(|(_, frame, _)| frame))?;
    for code in output_axes {
        axes.insert(
            code,
            GamepadAxis::Passthrough {
                minimum: -32767,
                maximum: 32767,
                neutral: 0,
            },
        );
    }
    let frame = GamepadFrame::new(buttons, axes)?;
    let mut reader = pad
        .map(|(path, frame, identity)| GamepadReader::open(path, frame, identity))
        .transpose()?;
    let mut absolute = AbsoluteReader::open(settings)?;
    let mut publisher = VirtualGamepad::create(&frame)?;
    let system_name = publisher.system_name().to_owned();
    let settings = settings.clone();
    let (stop, stopping) = mpsc::channel();
    let (ready, readiness) = mpsc::sync_channel(1);
    let failure = Arc::new(Mutex::new(None));
    let worker_failure = failure.clone();
    let worker = std::thread::Builder::new().name("controller-absolute-aim".into()).spawn(move || {
        let mut ready = Some(ready);
        let mut state = publisher.neutral_frame();
        let mut have_aim = false;
        let result: Result<()> = (|| {
            loop {
                match stopping.try_recv() {
                    Ok(()) | Err(mpsc::TryRecvError::Disconnected) => return Ok(()),
                    Err(mpsc::TryRecvError::Empty) => {}
                }
                let mut changed = false;
                if let Some(reader) = &mut reader {
                    for report in reader.poll()? {
                        state.extend(report);
                        changed = true;
                        // Preserve short press/release transitions even when
                        // several complete pad reports arrive in one poll.
                        if have_aim && reader.frame.synchronized { publisher.publish(&state)?; }
                    }
                }
                for packet in absolute.poll_timed()? {
                    ensure!(Instant::now().saturating_duration_since(packet.captured_at) <= Duration::from_millis(500),
                        "Absolute aim event is stale");
                    let aim = settings.project_libretro_raw(packet.position.raw_x, packet.position.raw_y)?;
                    // Out-of-calibration values have no agreed reload/offscreen
                    // meaning. Never silently turn their clamped edge into aim.
                    ensure!(!aim.outside_calibrated_area,
                        "Absolute aim left its calibrated rectangle; offscreen protocol is not configured");
                    state.insert(GamepadControl::Axis(output_axes[0]), i32::from(aim.x));
                    state.insert(GamepadControl::Axis(output_axes[1]), i32::from(aim.y));
                    have_aim = true;
                    changed = true;
                    if reader.as_ref().is_none_or(|reader| reader.frame.synchronized) {
                        publisher.publish(&state)?;
                    }
                }
                let synchronized = have_aim && reader.as_ref().is_none_or(|reader| reader.frame.synchronized);
                if synchronized && changed {
                    publisher.publish(&state)?;
                    if let Some(ready) = ready.take() {
                        ready.send(Ok(())).map_err(|_| anyhow::anyhow!("Absolute aim startup cancelled"))?;
                    }
                }
                match stopping.recv_timeout(Duration::from_millis(2)) {
                    Ok(()) | Err(mpsc::RecvTimeoutError::Disconnected) => return Ok(()),
                    Err(mpsc::RecvTimeoutError::Timeout) => {}
                }
            }
        })();
        publisher.shutdown();
        let error = result.err().map(|error| format!("{error:#}"));
        if let Some(ready) = ready {
            let _ = ready.send(Err(error.clone().unwrap_or_else(|| "Absolute aim stopped before synchronization".into())));
        }
        if let Ok(mut failure) = worker_failure.lock() { *failure = error; }
    })?;
    let bridge = GamepadBridge {
        stop,
        worker: Some(worker),
        failure,
        system_name,
    };
    let deadline = Instant::now() + Duration::from_secs(10);
    loop {
        ensure!(
            !cancel.load(Ordering::Relaxed),
            "Absolute aim startup cancelled"
        );
        ensure!(
            Instant::now() < deadline,
            "Absolute aim did not synchronize: move both calibrated axes before launching"
        );
        match readiness.recv_timeout(Duration::from_millis(20)) {
            Ok(Ok(())) => break,
            Ok(Err(error)) => anyhow::bail!("Absolute aim startup failed: {error}"),
            Err(mpsc::RecvTimeoutError::Timeout) => {}
            Err(error) => return Err(error.into()),
        }
    }
    bridge.check_health()?;
    Ok((bridge, output_axes))
}
