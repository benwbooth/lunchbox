//! Measured evdev axis gestures for one-time controller calibration.
//! Keeps physical units: emulator-specific normalization belongs to the adapter.
use anyhow::{Result, ensure};
use serde::{Deserialize, Serialize};
use std::time::{Duration, Instant};

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct AxisMeasurement {
    pub minimum: i32,
    pub maximum: i32,
    pub flat: i32,
    pub fuzz: i32,
    pub resolution: i32,
    pub released: i32,
    pub pressed: i32,
}

impl AxisMeasurement {
    pub fn validate(&self) -> Result<()> {
        ensure!(
            self.minimum < self.maximum && self.flat >= 0 && self.fuzz >= 0,
            "Invalid measured physical axis bounds"
        );
        ensure!(
            (self.minimum..=self.maximum).contains(&self.released)
                && (self.minimum..=self.maximum).contains(&self.pressed)
                && self.pressed != self.released,
            "Physical axis movement was not measured"
        );
        Ok(())
    }

    pub fn direction(&self) -> i8 {
        if self.pressed > self.released { 1 } else { -1 }
    }
}

// Linux input_absinfo ABI: six signed 32-bit values on both 32/64-bit hosts.
#[repr(C)]
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
struct AxisInfo {
    value: i32,
    minimum: i32,
    maximum: i32,
    fuzz: i32,
    flat: i32,
    resolution: i32,
}

struct Trace {
    bounds: AxisInfo,
    low: i32,
    high: i32,
    resting_since: Option<Instant>,
    release_started: Option<Instant>,
    rest_anchor: i32,
}

impl Trace {
    fn new(info: AxisInfo) -> Result<Self> {
        ensure!(
            info.minimum < info.maximum
                && info.flat >= 0
                && info.fuzz >= 0
                && (info.minimum..=info.maximum).contains(&info.value),
            "Invalid physical axis reading"
        );
        Ok(Self {
            bounds: info,
            low: info.value,
            high: info.value,
            resting_since: None,
            release_started: None,
            rest_anchor: info.value,
        })
    }

    fn release(&mut self, now: Instant) {
        self.release_started = Some(now);
        self.resting_since = None;
    }

    fn observe(&mut self, info: AxisInfo, now: Instant) -> Result<Option<AxisMeasurement>> {
        ensure!(
            AxisInfo {
                value: self.bounds.value,
                ..info
            } == self.bounds,
            "Physical axis calibration changed during capture; try again"
        );
        ensure!(
            (info.minimum..=info.maximum).contains(&info.value),
            "Physical axis left its reported range"
        );
        self.low = self.low.min(info.value);
        self.high = self.high.max(info.value);
        let Some(release_started) = self.release_started else {
            return Ok(None);
        };
        ensure!(
            now.duration_since(release_started) < Duration::from_secs(2),
            "Axis did not settle after release. Release the controller and try again"
        );
        // A small measured rest window tolerates sensor noise. Do not turn a
        // GilRs neutral-threshold crossing into an assumed raw center value.
        let tolerance =
            ((i64::from(info.maximum) - i64::from(info.minimum)) / 500).max(i64::from(info.fuzz));
        if self.resting_since.is_none()
            || (i64::from(info.value) - i64::from(self.rest_anchor)).abs() > tolerance
        {
            self.rest_anchor = info.value;
            self.resting_since = Some(now);
            return Ok(None);
        }
        if now.duration_since(self.resting_since.unwrap()) < Duration::from_millis(120) {
            return Ok(None);
        }
        let below = i64::from(info.value) - i64::from(self.low);
        let above = i64::from(self.high) - i64::from(info.value);
        ensure!(
            below.max(above) > tolerance.max(0),
            "No physical axis movement captured. Hold it fully before releasing"
        );
        // Opposite substantial excursions mean multiple actions were mixed into
        // one prompt (or the user pressed again before the release settled).
        ensure!(
            below.min(above) <= (below.max(above) / 5).max(tolerance),
            "Move only the highlighted direction, then let the axis settle"
        );
        let measured = AxisMeasurement {
            minimum: info.minimum,
            maximum: info.maximum,
            flat: info.flat,
            fuzz: info.fuzz,
            resolution: info.resolution,
            released: info.value,
            pressed: if above > below { self.high } else { self.low },
        };
        measured.validate()?;
        Ok(Some(measured))
    }
}

pub struct AxisCapture {
    #[cfg(target_os = "linux")]
    file: std::fs::File,
    code: u16,
    trace: Trace,
}

impl AxisCapture {
    #[cfg(target_os = "linux")]
    pub fn open(path: &std::path::Path, encoded: u32) -> Result<Self> {
        ensure!(
            encoded >> 16 == 3 && encoded & 0xffff < 64,
            "Expected an evdev axis"
        );
        let file = std::fs::File::open(path)?;
        let code = encoded as u16;
        let trace = Trace::new(read_info(&file, code)?)?;
        Ok(Self { file, code, trace })
    }

    #[cfg(not(target_os = "linux"))]
    pub fn open(_path: &std::path::Path, _encoded: u32) -> Result<Self> {
        anyhow::bail!("This physical axis backend requires Linux")
    }

    pub fn release(&mut self, now: Instant) {
        self.trace.release(now);
    }

    pub fn poll(&mut self, now: Instant) -> Result<Option<AxisMeasurement>> {
        #[cfg(target_os = "linux")]
        {
            self.trace.observe(read_info(&self.file, self.code)?, now)
        }
        #[cfg(not(target_os = "linux"))]
        {
            let _ = now;
            anyhow::bail!("This physical axis backend requires Linux")
        }
    }
}

#[cfg(target_os = "linux")]
fn read_info(file: &std::fs::File, code: u16) -> Result<AxisInfo> {
    use std::os::fd::AsRawFd;
    let mut info = AxisInfo::default();
    // EVIOCGABS(code), read-only: this neither grabs input nor changes calibration.
    let result = unsafe {
        libc::ioctl(
            file.as_raw_fd(),
            (0x80184540 + u32::from(code)) as libc::c_ulong,
            &mut info,
        )
    };
    ensure!(
        result >= 0,
        "Cannot read physical axis state: {}",
        std::io::Error::last_os_error()
    );
    Ok(info)
}

/// Read-only diagnostic evidence from the same kernel ABI used by capture.
#[cfg(target_os = "linux")]
pub fn probe(path: &std::path::Path, code: u8) -> Result<serde_json::Value> {
    ensure!(code < 64, "Invalid physical axis code");
    let info = read_info(&std::fs::File::open(path)?, u16::from(code))?;
    Ok(
        serde_json::json!({ "code": code, "value": info.value, "minimum": info.minimum,
        "maximum": info.maximum, "flat": info.flat, "fuzz": info.fuzz, "resolution": info.resolution }),
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    fn info(value: i32) -> AxisInfo {
        AxisInfo {
            value,
            minimum: -32768,
            maximum: 32767,
            ..Default::default()
        }
    }

    #[test]
    fn measures_peak_and_settled_rest_not_the_early_neutral_event() {
        let now = Instant::now();
        let mut trace = Trace::new(info(24000)).unwrap();
        trace.observe(info(32000), now).unwrap();
        trace.release(now);
        assert!(trace.observe(info(9000), now).unwrap().is_none());
        assert!(
            trace
                .observe(info(0), now + Duration::from_millis(30))
                .unwrap()
                .is_none()
        );
        assert!(
            trace
                .observe(info(0), now + Duration::from_millis(120))
                .unwrap()
                .is_none()
        );
        let measured = trace
            .observe(info(0), now + Duration::from_millis(155))
            .unwrap()
            .unwrap();
        assert_eq!(
            (measured.released, measured.pressed, measured.direction()),
            (0, 32000, 1)
        );
    }

    #[test]
    fn trigger_rest_and_reversed_axis_are_measured_not_inferred_from_labels() {
        for (rest, pressed) in [(0, 255), (255, 0), (128, 0), (128, 255)] {
            let sample = |value| AxisInfo {
                value,
                minimum: 0,
                maximum: 255,
                ..Default::default()
            };
            let now = Instant::now();
            let mut trace = Trace::new(sample(pressed)).unwrap();
            trace.release(now);
            trace.observe(sample(rest), now).unwrap();
            let measured = trace
                .observe(sample(rest), now + Duration::from_millis(125))
                .unwrap()
                .unwrap();
            assert_eq!((measured.released, measured.pressed), (rest, pressed));
            assert_eq!(measured.direction(), if pressed > rest { 1 } else { -1 });
        }
    }

    #[test]
    fn hat_axes_and_full_i32_ranges_do_not_overflow() {
        for (minimum, maximum, rest, pressed) in [
            (-1, 1, 0, -1),
            (-1, 1, 0, 1),
            (i32::MIN, i32::MAX, 0, i32::MAX),
        ] {
            let sample = |value| AxisInfo {
                value,
                minimum,
                maximum,
                ..Default::default()
            };
            let now = Instant::now();
            let mut trace = Trace::new(sample(pressed)).unwrap();
            trace.release(now);
            trace.observe(sample(rest), now).unwrap();
            assert!(
                trace
                    .observe(sample(rest), now + Duration::from_millis(125))
                    .unwrap()
                    .is_some()
            );
        }
    }

    #[test]
    fn rejects_changed_bounds_no_motion_mixed_directions_and_unsettled_input() {
        let now = Instant::now();
        let mut trace = Trace::new(info(0)).unwrap();
        assert!(
            trace
                .observe(
                    AxisInfo {
                        maximum: 100,
                        ..info(0)
                    },
                    now
                )
                .is_err()
        );
        trace.release(now);
        trace.observe(info(0), now).unwrap();
        assert!(
            trace
                .observe(info(0), now + Duration::from_millis(125))
                .is_err()
        );
        let mut trace = Trace::new(info(-32000)).unwrap();
        trace.observe(info(32000), now).unwrap();
        trace.release(now);
        trace.observe(info(0), now).unwrap();
        assert!(
            trace
                .observe(info(0), now + Duration::from_millis(125))
                .is_err()
        );
        let mut trace = Trace::new(info(32000)).unwrap();
        trace.release(now);
        assert!(
            trace
                .observe(info(0), now + Duration::from_secs(2))
                .is_err()
        );
        assert_eq!(std::mem::size_of::<AxisInfo>(), 24);
    }
}
