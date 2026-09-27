//! Identity-bound, non-grabbing scalar absolute evdev capture.
use super::absolute_packets::{AbsolutePacket, AbsoluteReportAssembler};
use super::absolute_settings::{AbsoluteAxisBounds, AbsoluteDeviceSettings};
use super::{GamepadReader, input_bits, open_identified_event, read_gamepad_event, read_info};
use anyhow::{Result, ensure};
use std::os::fd::AsRawFd;
use std::time::{Duration, Instant};

#[derive(Clone, Copy, Debug)]
pub struct TimedAbsolutePacket {
    pub position: AbsolutePacket,
    pub captured_at: Instant,
}

struct EventClock {
    monotonic: Duration,
    instant: Instant,
}

impl EventClock {
    fn configure(file: &std::fs::File) -> Result<Self> {
        // Linux v6.12 include/uapi/linux/input.h: EVIOCSCLOCKID = _IOW('E', 0xa0, int).
        // This changes only our evdev client's timestamp clock.
        let clock: libc::c_int = libc::CLOCK_MONOTONIC;
        ensure!(
            unsafe { libc::ioctl(file.as_raw_fd(), 0x400445a0 as libc::c_ulong, &clock) } == 0,
            "Cannot select monotonic absolute event timestamps: {}",
            std::io::Error::last_os_error()
        );
        // Anchor before sampling the native clock: any scheduling gap makes the
        // resulting timestamp conservatively older, never fresher than its event.
        let instant = Instant::now();
        let mut now = libc::timespec {
            tv_sec: 0,
            tv_nsec: 0,
        };
        ensure!(
            unsafe { libc::clock_gettime(libc::CLOCK_MONOTONIC, &mut now) } == 0,
            "Cannot read monotonic clock: {}",
            std::io::Error::last_os_error()
        );
        ensure!(
            now.tv_sec >= 0 && (0..1_000_000_000).contains(&now.tv_nsec),
            "Invalid monotonic clock value"
        );
        Ok(Self {
            monotonic: Duration::new(now.tv_sec as u64, now.tv_nsec as u32),
            instant,
        })
    }

    fn timestamp(&self, time: libc::timeval) -> Result<Instant> {
        ensure!(
            time.tv_sec >= 0 && (0..1_000_000).contains(&time.tv_usec),
            "Invalid absolute event timestamp"
        );
        let event = Duration::new(time.tv_sec as u64, time.tv_usec as u32 * 1000);
        let result = if event >= self.monotonic {
            self.instant.checked_add(event - self.monotonic)
        } else {
            self.instant.checked_sub(self.monotonic - event)
        }
        .ok_or_else(|| anyhow::anyhow!("Absolute event timestamp is out of range"))?;
        ensure!(
            result <= Instant::now(),
            "Absolute event timestamp is in the future"
        );
        Ok(result)
    }
}

/// Position-only transport. No button/contact/offscreen interpretation, virtual
/// device, frontend routing, or calibration writes are performed here.
pub struct AbsoluteReader {
    file: Option<std::fs::File>,
    axes: [AbsoluteAxisBounds; 2],
    reports: AbsoluteReportAssembler,
    waiting_for_boundary: bool,
    clock: EventClock,
    last_report_time: Option<Instant>,
}

impl AbsoluteReader {
    pub fn open(settings: &AbsoluteDeviceSettings) -> Result<Self> {
        settings.validate()?;
        let reader = Self::open_uncalibrated(
            &settings.event_path,
            &settings.input_identity,
            settings.x_axis.code,
            settings.y_axis.code,
        )?;
        ensure!(
            reader.axes == [settings.x_axis, settings.y_axis],
            "Absolute axis bounds changed since calibration; reconnect and recalibrate"
        );
        Ok(reader)
    }

    /// Read physical samples for first-time calibration without requiring made-up
    /// calibrated edges. The caller supplies an explicitly selected device and
    /// physical axis codes; no discovery by display name or axis-role guess occurs.
    /// Returned bounds are observations, not measured screen calibration.
    pub fn open_uncalibrated(
        event_path: &std::path::Path,
        input_identity: &std::path::Path,
        x_code: u16,
        y_code: u16,
    ) -> Result<Self> {
        ensure!(x_code != y_code, "Absolute axes must be distinct");
        ensure!(
            x_code <= 0x28 && y_code <= 0x28,
            "Absolute scalar capture does not support multitouch slots"
        );
        let file = open_identified_event(event_path, input_identity)?;
        let clock = EventClock::configure(&file)?;
        let supported = input_bits(&file, 0x23)?; // EVIOCGBIT(EV_ABS)
        let read_axis = |code| -> Result<AbsoluteAxisBounds> {
            ensure!(
                GamepadReader::bit(&supported, code),
                "Selected absolute axis is absent on this device"
            );
            let info = read_info(&file, code)?;
            let axis = AbsoluteAxisBounds {
                code,
                minimum: info.minimum,
                maximum: info.maximum,
            };
            axis.validate()?;
            Ok(axis)
        };
        let axes = [read_axis(x_code)?, read_axis(y_code)?];
        let reader = Self {
            file: Some(file),
            axes,
            reports: AbsoluteReportAssembler::new(axes[0], axes[1])?,
            waiting_for_boundary: true,
            clock,
            last_report_time: None,
        };
        reader.verify_bounds()?;
        Ok(reader)
    }

    /// Recorded physical X/Y bounds in the explicitly selected order. Polling
    /// revalidates these; this accessor alone is not a live device-health check.
    pub fn axis_bounds(&self) -> [AbsoluteAxisBounds; 2] {
        self.axes
    }

    fn verify_bounds(&self) -> Result<()> {
        let file = self
            .file
            .as_ref()
            .ok_or_else(|| anyhow::anyhow!("Absolute source is closed"))?;
        for axis in self.axes {
            let current = read_info(file, axis.code)?;
            ensure!(
                current.minimum == axis.minimum && current.maximum == axis.maximum,
                "Absolute axis bounds changed; reconnect and recalibrate"
            );
        }
        Ok(())
    }

    /// Start at a report boundary, then wait until both axes have appeared in
    /// the event stream. Separate EVIOCGABS queries are not an atomic X/Y sample,
    /// so their current values are deliberately not injected into reports.
    /// A stationary axis may therefore delay the first position. Initial-state
    /// acquisition remains a separate requirement for the capture workflow.
    ///
    /// Work is bounded per call. Any error discards the entire local batch and
    /// closes the source; owners must invalidate their last output immediately.
    pub fn poll(&mut self) -> Result<Vec<AbsolutePacket>> {
        Ok(self
            .poll_timed()?
            .into_iter()
            .map(|packet| packet.position)
            .collect())
    }

    /// Retains kernel report timestamps for acquisition freshness checks. These
    /// are not polling timestamps; queued old reports remain old.
    pub fn poll_timed(&mut self) -> Result<Vec<TimedAbsolutePacket>> {
        let result = self.poll_inner();
        if result.is_err() {
            self.close();
        }
        result
    }

    fn poll_inner(&mut self) -> Result<Vec<TimedAbsolutePacket>> {
        self.verify_bounds()?;
        let file = self
            .file
            .as_mut()
            .ok_or_else(|| anyhow::anyhow!("Absolute source is closed"))?;
        let mut packets = Vec::new();
        for _ in 0..256 {
            let Some(event) = read_gamepad_event(file)? else {
                break;
            };
            ensure!(
                !(event.kind == 0 && event.code == 3),
                "Absolute input synchronization lost; establish a new session"
            );
            if self.waiting_for_boundary {
                if event.kind == 0 && event.code == 0 {
                    self.waiting_for_boundary = false;
                }
                continue;
            }
            if let Some(packet) = self.reports.push(event.kind, event.code, event.value)? {
                let captured_at = self.clock.timestamp(event.time)?;
                ensure!(
                    self.last_report_time
                        .is_none_or(|previous| captured_at >= previous),
                    "Absolute report timestamps moved backwards"
                );
                self.last_report_time = Some(captured_at);
                packets.push(TimedAbsolutePacket {
                    position: packet,
                    captured_at,
                });
            }
        }
        self.verify_bounds()?;
        Ok(packets)
    }

    /// Close rather than inventing a neutral absolute position or offscreen shot.
    pub fn close(&mut self) {
        self.file.take();
    }
}
