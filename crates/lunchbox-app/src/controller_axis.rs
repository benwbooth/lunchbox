//! Measured evdev axis gestures for one-time controller calibration.
//! Keeps physical units: emulator-specific normalization belongs to the adapter.
use anyhow::{Result, ensure};
use serde::{Deserialize, Serialize};
use std::time::{Duration, Instant};

pub mod absolute_acquisition;
pub mod absolute_calibration;
pub mod absolute_packets;
pub mod absolute_settings;

#[cfg(all(target_os = "linux", target_pointer_width = "64"))]
pub mod absolute;
#[cfg(all(target_os = "linux", target_pointer_width = "64"))]
pub mod absolute_gamepad;
#[cfg(all(target_os = "linux", target_pointer_width = "64"))]
pub mod absolute_session;

#[cfg(all(target_os = "linux", target_pointer_width = "64"))]
pub mod relative;
#[cfg(all(target_os = "linux", target_pointer_width = "64"))]
pub mod relative_command;
pub mod relative_frontend;
pub mod relative_settings;
#[cfg(all(target_os = "linux", target_pointer_width = "64"))]
pub mod relative_topology;

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

/// A measured, one-sided pressure gesture normalized to libretro's analog-button
/// interval. This is deliberately independent of joydev's centered axis values:
/// a trigger released at the minimum must not lose its first half of travel.
/// Construct from the recorded evdev gesture and apply to samples from that same
/// axis after the caller has revalidated the device and its reported bounds.
#[derive(Clone, Debug)]
pub struct PressureAxis {
    measurement: AxisMeasurement,
}

impl PressureAxis {
    pub fn from_measurement(measurement: &AxisMeasurement) -> Result<Self> {
        measurement.validate()?;
        // Hat switches and similarly tiny discrete ranges are not pressure
        // controls. Bounds alone cannot establish hardware fidelity; the caller
        // must also require an analog trigger capability and a captured gesture.
        ensure!(
            (i64::from(measurement.pressed) - i64::from(measurement.released)).abs() > 2,
            "Discrete axis movement cannot supply proportional trigger pressure"
        );
        Ok(Self {
            measurement: measurement.clone(),
        })
    }

    /// Released maps to 0 and the captured full press to 32767, including
    /// reversed axes and axes whose neutral is not the midpoint. Values beyond
    /// either measured gesture endpoint saturate, but impossible device samples
    /// outside the kernel-reported bounds remain errors.
    pub fn normalize(&self, value: i32) -> Result<u16> {
        let measured = &self.measurement;
        ensure!(
            (measured.minimum..=measured.maximum).contains(&value),
            "Pressure sample is outside the calibrated device range"
        );
        let direction = i64::from(measured.direction());
        let travel = (i64::from(measured.pressed) - i64::from(measured.released)) * direction;
        let position =
            ((i64::from(value) - i64::from(measured.released)) * direction).clamp(0, travel);
        // i64 covers the full i32 input span multiplied by 32767, including
        // reversed extreme endpoints. Round to nearest, preserving both ends.
        Ok(((position * 32767 + travel / 2) / travel) as u16)
    }
}

/// Two measured halves of one physical stick, independently normalized around
/// its recorded neutral. Direction follows the selected positive gesture, not
/// the kernel axis orientation. Output is symmetric signed gamepad units.
#[derive(Clone, Debug)]
pub struct BipolarAxis {
    negative: AxisMeasurement,
    positive: AxisMeasurement,
}

impl BipolarAxis {
    pub fn from_measurements(
        negative: &AxisMeasurement,
        positive: &AxisMeasurement,
    ) -> Result<Self> {
        negative.validate()?;
        positive.validate()?;
        ensure!(
            negative.minimum == positive.minimum
                && negative.maximum == positive.maximum
                && negative.flat == positive.flat
                && negative.fuzz == positive.fuzz
                && negative.resolution == positive.resolution
                && negative.released == positive.released,
            "Stick halves have inconsistent bounds or neutral; recalibrate both directions"
        );
        ensure!(
            negative.direction() == -positive.direction(),
            "Stick gestures must be on opposite sides of neutral"
        );
        ensure!(
            [negative, positive]
                .iter()
                .all(|measurement| (i64::from(measurement.pressed)
                    - i64::from(measurement.released))
                .abs()
                    > 2),
            "Discrete axis movement cannot supply a proportional stick"
        );
        Ok(Self {
            negative: negative.clone(),
            positive: positive.clone(),
        })
    }

    pub fn normalize(&self, value: i32) -> Result<i32> {
        ensure!(
            (self.negative.minimum..=self.negative.maximum).contains(&value),
            "Stick sample is outside the calibrated device range"
        );
        let position = (i64::from(value) - i64::from(self.negative.released))
            * i64::from(self.positive.direction());
        let endpoint = if position >= 0 {
            self.positive.pressed
        } else {
            self.negative.pressed
        };
        let travel = (i64::from(endpoint) - i64::from(self.negative.released)).abs();
        let distance = position.abs().min(travel);
        // i64 safely covers the entire i32 input span times the output range.
        let magnitude = (distance * 32767 + travel / 2) / travel;
        Ok((magnitude * position.signum()) as i32)
    }
}

/// Pressure samples are committed only at evdev SYN_REPORT boundaries. A caller
/// publishing a normalized virtual controller can merge these values with its
/// other controller controls before emitting the virtual SYN_REPORT.
pub struct PressureFrame {
    axes: std::collections::BTreeMap<u16, PressureAxis>,
    pending: std::collections::BTreeMap<u16, u16>,
    synchronized: bool,
}

impl PressureFrame {
    pub fn new(measurements: impl IntoIterator<Item = (u16, AxisMeasurement)>) -> Result<Self> {
        let mut axes = std::collections::BTreeMap::new();
        for (code, measurement) in measurements {
            ensure!(code < 64, "Invalid pressure evdev axis code");
            let axis = PressureAxis::from_measurement(&measurement)?;
            ensure!(
                axes.insert(code, axis).is_none(),
                "A physical pressure axis cannot drive two independent triggers"
            );
        }
        ensure!(!axes.is_empty(), "No pressure axes were configured");
        Ok(Self {
            axes,
            pending: std::collections::BTreeMap::new(),
            synchronized: false,
        })
    }

    /// Seed or resynchronize from an authoritative EVIOCGABS snapshot, never
    /// from an assumed released position. The event reader must discard the
    /// remainder of a dropped packet before taking this snapshot.
    pub fn resynchronize(
        &mut self,
        samples: impl IntoIterator<Item = (u16, i32)>,
    ) -> Result<std::collections::BTreeMap<u16, u16>> {
        self.synchronized = false;
        self.pending.clear();
        let mut frame = std::collections::BTreeMap::new();
        for (code, value) in samples {
            let axis = self
                .axes
                .get(&code)
                .ok_or_else(|| anyhow::anyhow!("Unexpected pressure axis in resynchronization"))?;
            ensure!(
                frame.insert(code, axis.normalize(value)?).is_none(),
                "Duplicate pressure sample in resynchronization"
            );
        }
        ensure!(
            frame.len() == self.axes.len(),
            "Pressure resynchronization is missing a configured axis"
        );
        self.synchronized = true;
        Ok(frame)
    }

    /// Process EV_ABS for a configured pressure axis. Other axes belong to the
    /// caller's stick/hat handling. Invalid samples invalidate the whole pending
    /// frame; the caller must neutralize the device on any returned error.
    pub fn sample(&mut self, code: u16, value: i32) -> Result<()> {
        ensure!(
            self.synchronized,
            "Pressure input requires resynchronization"
        );
        let Some(axis) = self.axes.get(&code) else {
            return Ok(());
        };
        match axis.normalize(value) {
            Ok(value) => {
                self.pending.insert(code, value);
                Ok(())
            }
            Err(error) => {
                self.synchronized = false;
                self.pending.clear();
                Err(error)
            }
        }
    }

    /// Called at SYN_REPORT. Values are changed axes only; retain previously
    /// committed axes on the normalized device rather than releasing them.
    pub fn report(&mut self) -> Result<std::collections::BTreeMap<u16, u16>> {
        ensure!(
            self.synchronized,
            "Pressure input requires resynchronization"
        );
        Ok(std::mem::take(&mut self.pending))
    }

    /// On SYN_DROPPED, unplug, read failure or session shutdown, publish this
    /// neutral frame and stop accepting samples until a complete snapshot exists.
    pub fn invalidate(&mut self) -> std::collections::BTreeMap<u16, u16> {
        self.synchronized = false;
        self.pending.clear();
        self.axes.keys().map(|code| (*code, 0)).collect()
    }
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

    fn matches_bounds(&self, current: AxisInfo) -> bool {
        self.minimum == current.minimum
            && self.maximum == current.maximum
            && self.flat == current.flat
            && self.fuzz == current.fuzz
            && self.resolution == current.resolution
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

/// Read-only evdev pressure stream. The session owner must publish `neutral()`
/// on any error, unplug or shutdown; it must not retain the last pressed value.
/// This reader never uses EVIOCGRAB or changes physical axis calibration.
#[cfg(all(target_os = "linux", target_pointer_width = "64"))]
pub struct PressureReader {
    file: std::fs::File,
    frame: PressureFrame,
    dropping: bool,
    failed: bool,
}

#[cfg(all(target_os = "linux", target_pointer_width = "64"))]
#[repr(C)]
struct PressureInputEvent {
    time: libc::timeval,
    kind: u16,
    code: u16,
    value: i32,
}

#[cfg(all(target_os = "linux", target_pointer_width = "64"))]
impl PressureReader {
    /// The caller establishes that `path` belongs to the selected controller.
    /// Non-controller paths must never be discovered by this reader itself.
    pub fn open(
        path: &std::path::Path,
        measurements: impl IntoIterator<Item = (u16, AxisMeasurement)>,
    ) -> Result<(Self, std::collections::BTreeMap<u16, u16>)> {
        use std::os::unix::fs::OpenOptionsExt;
        let file = std::fs::OpenOptions::new()
            .read(true)
            .custom_flags(libc::O_NONBLOCK | libc::O_CLOEXEC)
            .open(path)?;
        let mut reader = Self {
            file,
            frame: PressureFrame::new(measurements)?,
            dropping: false,
            failed: false,
        };
        let initial = reader.snapshot()?;
        Ok((reader, initial))
    }

    fn snapshot(&mut self) -> Result<std::collections::BTreeMap<u16, u16>> {
        let mut samples = Vec::with_capacity(self.frame.axes.len());
        for (code, axis) in &self.frame.axes {
            let current = read_info(&self.file, *code)?;
            ensure!(
                axis.measurement.matches_bounds(current),
                "Pressure axis changed since calibration; reconnect and recalibrate"
            );
            samples.push((*code, current.value));
        }
        self.frame.resynchronize(samples)
    }

    pub fn neutral(&mut self) -> std::collections::BTreeMap<u16, u16> {
        self.failed = true;
        self.frame.invalidate()
    }

    /// Drain a bounded amount of available input without blocking. Each returned
    /// map is a separate frame; in particular, do not coalesce a loss-neutral
    /// frame with the subsequent resynchronization frame.
    pub fn poll(&mut self) -> Result<Vec<std::collections::BTreeMap<u16, u16>>> {
        ensure!(
            !self.failed,
            "Pressure stream has failed; create a new session"
        );
        let result = self.poll_inner();
        if result.is_err() {
            self.neutral();
        }
        result
    }

    fn poll_inner(&mut self) -> Result<Vec<std::collections::BTreeMap<u16, u16>>> {
        use std::io::Read;
        let mut frames = Vec::new();
        for _ in 0..256 {
            // Linux input_event on 64-bit targets has a timeval followed by
            // type/code/value. Never interpret a short read as a complete event.
            let mut event = PressureInputEvent {
                time: libc::timeval {
                    tv_sec: 0,
                    tv_usec: 0,
                },
                kind: 0,
                code: 0,
                value: 0,
            };
            let bytes = unsafe {
                std::slice::from_raw_parts_mut(
                    (&mut event as *mut PressureInputEvent).cast::<u8>(),
                    std::mem::size_of::<PressureInputEvent>(),
                )
            };
            match self.file.read(bytes) {
                Err(error) if error.kind() == std::io::ErrorKind::WouldBlock => break,
                Err(error) if error.kind() == std::io::ErrorKind::Interrupted => continue,
                Err(error) => return Err(error.into()),
                Ok(0) => anyhow::bail!("Pressure controller disconnected"),
                Ok(length) => ensure!(length == bytes.len(), "Incomplete pressure input event"),
            }
            if event.kind == 0 && event.code == 3 {
                // SYN_DROPPED
                frames.push(self.frame.invalidate());
                self.dropping = true;
                continue;
            }
            if self.dropping {
                if event.kind == 0 && event.code == 0 {
                    // SYN_REPORT
                    frames.push(self.snapshot()?);
                    self.dropping = false;
                }
                continue;
            }
            match (event.kind, event.code) {
                (3, code) => self.frame.sample(code, event.value)?, // EV_ABS
                (0, 0) => {
                    let frame = self.frame.report()?;
                    if !frame.is_empty() {
                        frames.push(frame);
                    }
                }
                _ => {} // Keyboard, button and other axis transports are separate.
            }
        }
        Ok(frames)
    }
}

/// Explicitly selected gamepad controls for a session-local virtual device.
/// This is not an arbitrary evdev relay: keyboard keys, relative pointer events,
/// switches and output events cannot enter a gamepad frame.
#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub enum GamepadControl {
    Button(u16),
    Axis(u16),
}

#[derive(Clone, Debug)]
pub enum GamepadAxis {
    /// Preserve physical units for sticks/hats. Neutral is supplied by the
    /// calibrated controller contract, never guessed from the numeric midpoint.
    Passthrough {
        minimum: i32,
        maximum: i32,
        neutral: i32,
    },
    Pressure(PressureAxis),
    Bipolar(BipolarAxis),
}

impl GamepadAxis {
    fn normalize(&self, value: i32) -> Result<i32> {
        match self {
            Self::Passthrough {
                minimum, maximum, ..
            } => {
                ensure!(
                    (*minimum..=*maximum).contains(&value),
                    "Gamepad axis sample outside declared bounds"
                );
                Ok(value)
            }
            Self::Pressure(axis) => Ok(i32::from(axis.normalize(value)?)),
            Self::Bipolar(axis) => axis.normalize(value),
        }
    }

    fn neutral(&self) -> i32 {
        match self {
            Self::Passthrough { neutral, .. } => *neutral,
            Self::Pressure(_) | Self::Bipolar(_) => 0,
        }
    }
}

/// Prepare a virtual-device contract from saved physical measurements. This
/// performs no I/O. The caller must validate the physical device before starting
/// a bridge and use the returned calibration only with that owned virtual pad.
pub fn normalized_gamepad_calibration(
    calibration: &crate::controller_catalog::Calibration,
) -> Result<(GamepadFrame, crate::controller_catalog::Calibration)> {
    use std::collections::{BTreeMap, BTreeSet};
    calibration.validate()?;
    ensure!(
        calibration.os == "linux",
        "Normalized gamepad requires Linux physical calibration"
    );
    let mut buttons = BTreeSet::new();
    let mut measurements: BTreeMap<u16, Vec<AxisMeasurement>> = BTreeMap::new();
    for input in calibration.bindings.values() {
        let native = input
            .native
            .as_ref()
            .ok_or_else(|| anyhow::anyhow!("Recalibrate to capture physical input identity"))?;
        let code = (native.code & 0xffff) as u16;
        match native.code >> 16 {
            1 => {
                buttons.insert(code);
            }
            3 => {
                let axis = input.axis.as_ref().ok_or_else(|| {
                    anyhow::anyhow!("Recalibrate every mapped axis before normalization")
                })?;
                axis.validate()?;
                ensure!(
                    native.direction == axis.direction(),
                    "Axis direction disagrees with its measurement"
                );
                measurements.entry(code).or_default().push(axis.clone());
            }
            _ => {
                anyhow::bail!("Normalized transport only accepts physical gamepad buttons and axes")
            }
        }
    }
    let mut axes = BTreeMap::new();
    for (code, gestures) in measurements {
        let first = &gestures[0];
        ensure!(
            gestures.iter().all(|axis| axis.minimum == first.minimum
                && axis.maximum == first.maximum
                && axis.flat == first.flat
                && axis.fuzz == first.fuzz
                && axis.resolution == first.resolution
                && axis.released == first.released),
            "Shared physical axis has inconsistent calibration metadata"
        );
        let negative = gestures.iter().find(|axis| axis.direction() < 0);
        let positive = gestures.iter().find(|axis| axis.direction() > 0);
        ensure!(
            gestures.iter().all(|axis| {
                let matching = if axis.direction() < 0 {
                    negative
                } else {
                    positive
                };
                matching.is_some_and(|previous| previous.pressed == axis.pressed)
            }),
            "Shared physical axis has inconsistent measured endpoints"
        );
        let axis = if (0x10..=0x17).contains(&code) {
            GamepadAxis::Passthrough {
                minimum: first.minimum,
                maximum: first.maximum,
                neutral: first.released,
            }
        } else if let (Some(negative), Some(positive)) = (negative, positive) {
            // Preserve physical numeric orientation; target inversion remains a
            // property of each semantic binding's returned measured direction.
            GamepadAxis::Bipolar(BipolarAxis::from_measurements(negative, positive)?)
        } else {
            GamepadAxis::Pressure(PressureAxis::from_measurement(first)?)
        };
        axes.insert(code, axis);
    }
    let mut translated = calibration.clone();
    for input in translated.bindings.values_mut() {
        let native = input
            .native
            .as_mut()
            .ok_or_else(|| anyhow::anyhow!("Physical input identity disappeared"))?;
        if native.code >> 16 != 3 {
            continue;
        }
        let axis = axes
            .get(&((native.code & 0xffff) as u16))
            .ok_or_else(|| anyhow::anyhow!("Normalized axis is missing"))?;
        let measured = input
            .axis
            .as_mut()
            .ok_or_else(|| anyhow::anyhow!("Axis measurement disappeared"))?;
        let released = axis.normalize(measured.released)?;
        let pressed = axis.normalize(measured.pressed)?;
        let (minimum, maximum) = match axis {
            GamepadAxis::Passthrough {
                minimum, maximum, ..
            } => (*minimum, *maximum),
            GamepadAxis::Pressure(_) | GamepadAxis::Bipolar(_) => (-32767, 32767),
        };
        *measured = AxisMeasurement {
            minimum,
            maximum,
            released,
            pressed,
            flat: 0,
            fuzz: 0,
            resolution: 0,
        };
        native.direction = measured.direction();
    }
    translated.validate()?;
    Ok((GamepadFrame::new(buttons, axes)?, translated))
}

/// A complete controller frame assembler, including the buttons and sticks that
/// must share the virtual joypad with normalized triggers. The evdev reader owns
/// snapshot/drop ordering; the publisher owns SYN_REPORT and device lifetime.
pub struct GamepadFrame {
    buttons: std::collections::BTreeSet<u16>,
    axes: std::collections::BTreeMap<u16, GamepadAxis>,
    pending: std::collections::BTreeMap<GamepadControl, i32>,
    synchronized: bool,
}

impl GamepadFrame {
    pub fn new(
        buttons: impl IntoIterator<Item = u16>,
        axes: impl IntoIterator<Item = (u16, GamepadAxis)>,
    ) -> Result<Self> {
        let mut selected_buttons = std::collections::BTreeSet::new();
        for code in buttons {
            // BTN_JOYSTICK/BTN_GAMEPAD, BTN_DPAD and BTN_TRIGGER_HAPPY only.
            // In particular, never permit KEY_* or BTN_MOUSE/BTN_DIGI.
            ensure!(
                (0x120..=0x13f).contains(&code)
                    || (0x220..=0x223).contains(&code)
                    || (0x2c0..=0x2e7).contains(&code),
                "Only gamepad button codes may be forwarded"
            );
            ensure!(selected_buttons.insert(code), "Duplicate gamepad button");
        }
        let mut selected_axes = std::collections::BTreeMap::new();
        for (code, axis) in axes {
            // Conventional joystick axes and four hats. Excludes touch,
            // multitouch, tool pressure and other non-controller ABS controls.
            ensure!(
                code <= 0x0a || (0x10..=0x17).contains(&code),
                "Unsupported gamepad axis code"
            );
            ensure!(
                !matches!(&axis, GamepadAxis::Bipolar(_)) || code <= 0x0a,
                "A normalized full stick cannot use a hat axis code"
            );
            if let GamepadAxis::Passthrough {
                minimum,
                maximum,
                neutral,
            } = &axis
            {
                ensure!(
                    minimum < maximum && (*minimum..=*maximum).contains(neutral),
                    "Invalid gamepad axis bounds or neutral"
                );
            }
            ensure!(
                selected_axes.insert(code, axis).is_none(),
                "Duplicate gamepad axis"
            );
        }
        ensure!(
            !selected_buttons.is_empty() || !selected_axes.is_empty(),
            "No gamepad controls selected"
        );
        Ok(Self {
            buttons: selected_buttons,
            axes: selected_axes,
            pending: std::collections::BTreeMap::new(),
            synchronized: false,
        })
    }

    fn normalize(&self, control: GamepadControl, value: i32) -> Result<Option<i32>> {
        match control {
            GamepadControl::Button(code) if self.buttons.contains(&code) => {
                ensure!((0..=2).contains(&value), "Invalid gamepad button value");
                // EV_KEY repeat means still held; never forward repeat as a
                // second press or as a keyboard autorepeat event.
                Ok(Some(i32::from(value != 0)))
            }
            GamepadControl::Axis(code) => self
                .axes
                .get(&code)
                .map(|axis| axis.normalize(value))
                .transpose(),
            _ => Ok(None),
        }
    }

    pub fn resynchronize(
        &mut self,
        samples: impl IntoIterator<Item = (GamepadControl, i32)>,
    ) -> Result<std::collections::BTreeMap<GamepadControl, i32>> {
        self.synchronized = false;
        self.pending.clear();
        let mut frame = std::collections::BTreeMap::new();
        for (control, value) in samples {
            let value = self
                .normalize(control, value)?
                .ok_or_else(|| anyhow::anyhow!("Unexpected control in gamepad snapshot"))?;
            ensure!(
                frame.insert(control, value).is_none(),
                "Duplicate gamepad snapshot control"
            );
        }
        ensure!(
            frame.len() == self.buttons.len() + self.axes.len(),
            "Incomplete gamepad snapshot"
        );
        self.synchronized = true;
        Ok(frame)
    }

    pub fn sample(&mut self, control: GamepadControl, value: i32) -> Result<()> {
        ensure!(
            self.synchronized,
            "Gamepad input requires resynchronization"
        );
        match self.normalize(control, value) {
            Ok(Some(value)) => {
                self.pending.insert(control, value);
            }
            Ok(None) => {}
            Err(error) => {
                self.invalidate();
                return Err(error);
            }
        }
        Ok(())
    }

    pub fn report(&mut self) -> Result<std::collections::BTreeMap<GamepadControl, i32>> {
        ensure!(
            self.synchronized,
            "Gamepad input requires resynchronization"
        );
        Ok(std::mem::take(&mut self.pending))
    }

    /// Publish this entire frame on loss or shutdown, including stick centers
    /// and released buttons, not just trigger pressure. Caller must publish it
    /// after any error before tearing down the virtual device.
    pub fn invalidate(&mut self) -> std::collections::BTreeMap<GamepadControl, i32> {
        self.synchronized = false;
        self.pending.clear();
        self.buttons
            .iter()
            .map(|code| (GamepadControl::Button(*code), 0))
            .chain(
                self.axes
                    .iter()
                    .map(|(code, axis)| (GamepadControl::Axis(*code), axis.neutral())),
            )
            .collect()
    }
}

/// Read-only input for exactly the selected gamepad controls. A session owner
/// must publish `neutral()` on errors/shutdown and retain ownership of the
/// selected device identity; this type does not discover or grab input devices.
#[cfg(all(target_os = "linux", target_pointer_width = "64"))]
pub struct GamepadReader {
    file: std::fs::File,
    frame: GamepadFrame,
    dropping: bool,
    needs_snapshot: bool,
    failed: bool,
}

#[cfg(all(target_os = "linux", target_pointer_width = "64"))]
impl GamepadReader {
    pub fn open(
        path: &std::path::Path,
        frame: GamepadFrame,
        expected_input_identity: &std::path::Path,
    ) -> Result<Self> {
        let file = open_identified_event(path, expected_input_identity)?;
        let supported = Self::key_bits(&file, 0x21)?; // EVIOCGBIT(EV_KEY)
        for code in &frame.buttons {
            ensure!(
                Self::bit(&supported, *code),
                "Selected gamepad button is not present on this device"
            );
        }
        Ok(Self {
            file,
            frame,
            dropping: false,
            needs_snapshot: true,
            failed: false,
        })
    }

    fn bit(bits: &[u8; 96], code: u16) -> bool {
        bits[usize::from(code) / 8] & (1 << (code % 8)) != 0
    }

    fn key_bits(file: &std::fs::File, operation: u32) -> Result<[u8; 96]> {
        input_bits(file, operation)
    }
}

#[cfg(all(target_os = "linux", target_pointer_width = "64"))]
fn open_identified_event(
    path: &std::path::Path,
    expected_input_identity: &std::path::Path,
) -> Result<std::fs::File> {
    use std::os::unix::fs::{FileTypeExt, MetadataExt, OpenOptionsExt};
    ensure!(
        path.parent() == Some(std::path::Path::new("/dev/input")),
        "Expected a selected controller event node"
    );
    let name = path
        .file_name()
        .and_then(|name| name.to_str())
        .ok_or_else(|| anyhow::anyhow!("Invalid controller event node name"))?;
    ensure!(
        name.strip_prefix("event").is_some_and(
            |index| !index.is_empty() && index.bytes().all(|byte| byte.is_ascii_digit())
        ),
        "Expected a selected controller event node"
    );
    let file = std::fs::OpenOptions::new()
        .read(true)
        .custom_flags(libc::O_NONBLOCK | libc::O_CLOEXEC | libc::O_NOFOLLOW)
        .open(path)?;
    let opened = file.metadata()?;
    ensure!(
        opened.file_type().is_char_device(),
        "Selected controller input is not a character device"
    );
    let device_number = format!(
        "{}:{}",
        libc::major(opened.rdev()),
        libc::minor(opened.rdev())
    );
    // Anchor identity to the opened descriptor's actual device number,
    // then also confirm that the path still names this same devtmpfs node.
    // A replaced path cannot substitute another device between discovery
    // and the start of event forwarding.
    let opened_identity = std::path::Path::new("/sys/dev/char")
        .join(device_number)
        .join("device")
        .canonicalize()?;
    ensure!(
        opened_identity == expected_input_identity,
        "Opened controller does not match the selected joystick identity"
    );
    let current = std::fs::symlink_metadata(path)?;
    ensure!(
        current.file_type().is_char_device()
            && current.rdev() == opened.rdev()
            && current.dev() == opened.dev()
            && current.ino() == opened.ino(),
        "Controller event node changed while opening normalized input"
    );
    Ok(file)
}

#[cfg(all(target_os = "linux", target_pointer_width = "64"))]
fn input_bits(file: &std::fs::File, operation: u32) -> Result<[u8; 96]> {
    use std::os::fd::AsRawFd;
    // Linux KEY_CNT is 0x300. A fixed byte array also avoids native-word
    // endianness assumptions when selecting bits from the kernel bitmap.
    let mut bits = [0u8; 96];
    let request = 0x8000_0000u32 | (96 << 16) | (0x45 << 8) | operation;
    let result = unsafe {
        libc::ioctl(
            file.as_raw_fd(),
            request as libc::c_ulong,
            bits.as_mut_ptr(),
        )
    };
    ensure!(
        result >= 0,
        "Cannot read gamepad button state: {}",
        std::io::Error::last_os_error()
    );
    Ok(bits)
}

#[cfg(all(target_os = "linux", target_pointer_width = "64"))]
impl GamepadReader {
    fn snapshot(&mut self) -> Result<std::collections::BTreeMap<GamepadControl, i32>> {
        let keys = Self::key_bits(&self.file, 0x18)?; // EVIOCGKEY
        let mut samples = Vec::with_capacity(self.frame.buttons.len() + self.frame.axes.len());
        for code in &self.frame.buttons {
            samples.push((
                GamepadControl::Button(*code),
                i32::from(Self::bit(&keys, *code)),
            ));
        }
        for (code, axis) in &self.frame.axes {
            let current = read_info(&self.file, *code)?;
            let matches = match axis {
                GamepadAxis::Passthrough {
                    minimum, maximum, ..
                } => current.minimum == *minimum && current.maximum == *maximum,
                GamepadAxis::Pressure(axis) => axis.measurement.matches_bounds(current),
                GamepadAxis::Bipolar(axis) => axis.negative.matches_bounds(current),
            };
            ensure!(
                matches,
                "Gamepad axis changed since calibration; reconnect and recalibrate"
            );
            samples.push((GamepadControl::Axis(*code), current.value));
        }
        self.frame.resynchronize(samples)
    }

    pub fn neutral(&mut self) -> std::collections::BTreeMap<GamepadControl, i32> {
        self.failed = true;
        self.frame.invalidate()
    }

    /// Return separate packets, preserving an explicit neutral packet on loss.
    /// A bounded poll may return no packets while draining a recovery backlog;
    /// the owner must poll again. No blocking waits or live-device writes occur.
    pub fn poll(&mut self) -> Result<Vec<std::collections::BTreeMap<GamepadControl, i32>>> {
        ensure!(
            !self.failed,
            "Gamepad stream has failed; create a new session"
        );
        let result = self.poll_inner();
        if result.is_err() {
            self.neutral();
        }
        result
    }

    fn poll_inner(&mut self) -> Result<Vec<std::collections::BTreeMap<GamepadControl, i32>>> {
        let mut frames = Vec::new();
        for _ in 0..256 {
            let Some(event) = read_gamepad_event(&mut self.file)? else {
                // Discard the queued pre-snapshot history, not just the rest
                // of the dropped packet. Snapshots use current kernel state,
                // so replaying that history would reintroduce stale presses.
                if self.needs_snapshot && !self.dropping {
                    frames.push(self.snapshot()?);
                    self.needs_snapshot = false;
                }
                break;
            };
            if event.kind == 0 && event.code == 3 {
                frames.push(self.frame.invalidate());
                self.dropping = true;
                self.needs_snapshot = true;
                continue;
            }
            if self.dropping {
                if event.kind == 0 && event.code == 0 {
                    self.dropping = false;
                }
                continue;
            }
            if self.needs_snapshot {
                continue;
            }
            match (event.kind, event.code) {
                (1, code) => self
                    .frame
                    .sample(GamepadControl::Button(code), event.value)?,
                (3, code) => self.frame.sample(GamepadControl::Axis(code), event.value)?,
                (0, 0) => {
                    let frame = self.frame.report()?;
                    if !frame.is_empty() {
                        frames.push(frame);
                    }
                }
                _ => {}
            }
        }
        Ok(frames)
    }
}

#[cfg(all(target_os = "linux", target_pointer_width = "64"))]
fn read_gamepad_event(file: &mut std::fs::File) -> Result<Option<PressureInputEvent>> {
    use std::io::Read;
    let mut event = PressureInputEvent {
        time: libc::timeval {
            tv_sec: 0,
            tv_usec: 0,
        },
        kind: 0,
        code: 0,
        value: 0,
    };
    let bytes = unsafe {
        std::slice::from_raw_parts_mut(
            (&mut event as *mut PressureInputEvent).cast::<u8>(),
            std::mem::size_of::<PressureInputEvent>(),
        )
    };
    // Bound signal retries as well as successful events, so shutdown cannot be
    // starved by a signal storm. Exhaustion is a failure requiring neutralization.
    for _ in 0..16 {
        match file.read(bytes) {
            Err(error) if error.kind() == std::io::ErrorKind::WouldBlock => return Ok(None),
            Err(error) if error.kind() == std::io::ErrorKind::Interrupted => continue,
            Err(error) => return Err(error.into()),
            Ok(0) => anyhow::bail!("Gamepad controller disconnected"),
            Ok(length) => ensure!(length == bytes.len(), "Incomplete gamepad input event"),
        }
        return Ok(Some(event));
    }
    anyhow::bail!("Gamepad input interrupted repeatedly")
}

/// Owned, launch-local uinput gamepad. Only controls admitted by GamepadFrame
/// are exposed. There is no physical-device grab, calibration write, keyboard
/// output, permission modification or automatic elevation in this transport.
#[cfg(all(target_os = "linux", target_pointer_width = "64"))]
pub struct VirtualGamepad {
    file: std::fs::File,
    buttons: std::collections::BTreeSet<u16>,
    axes: std::collections::BTreeMap<u16, GamepadAxis>,
    created: bool,
    system_name: String,
}

#[cfg(all(target_os = "linux", target_pointer_width = "64"))]
impl VirtualGamepad {
    pub fn create(frame: &GamepadFrame) -> Result<Self> {
        use std::os::fd::AsRawFd;
        use std::os::unix::fs::OpenOptionsExt;
        use std::sync::atomic::{AtomicU64, Ordering};
        static NEXT_DEVICE: AtomicU64 = AtomicU64::new(0);
        let file = std::fs::OpenOptions::new()
            .write(true)
            .custom_flags(libc::O_NONBLOCK | libc::O_CLOEXEC)
            .open("/dev/uinput")
            .map_err(|error| anyhow::anyhow!("Normalized controller output requires writable /dev/uinput: {error}. No permissions were changed."))?;
        let mut device = Self {
            file,
            buttons: frame.buttons.clone(),
            axes: frame.axes.clone(),
            created: false,
            system_name: String::new(),
        };
        if !device.buttons.is_empty() {
            device.capability(0x40045564, 1)?; // UI_SET_EVBIT(EV_KEY)
        }
        if !device.axes.is_empty() {
            device.capability(0x40045564, 3)?; // UI_SET_EVBIT(EV_ABS)
        }
        for code in &device.buttons {
            device.capability(0x40045565, i32::from(*code))?;
        }
        for (code, axis) in &device.axes {
            device.capability(0x40045567, i32::from(*code))?;
            let (minimum, maximum, neutral) = match axis {
                GamepadAxis::Passthrough {
                    minimum,
                    maximum,
                    neutral,
                } => (*minimum, *maximum, *neutral),
                // linuxraw joydev reads positive half-axes for analog buttons.
                // A centered declared range makes released=0/full=32767 keep
                // all of the measured physical trigger travel on that half.
                GamepadAxis::Pressure(_) | GamepadAxis::Bipolar(_) => (-32767, 32767, 0),
            };
            // uinput_abs_setup: u16 code, two padding bytes, input_absinfo.
            let mut setup = [0u8; 28];
            setup[..2].copy_from_slice(&code.to_ne_bytes());
            setup[4..8].copy_from_slice(&neutral.to_ne_bytes());
            setup[8..12].copy_from_slice(&minimum.to_ne_bytes());
            setup[12..16].copy_from_slice(&maximum.to_ne_bytes());
            // No extra virtual deadzone/fuzz: physical samples and calibrated
            // pressure normalization have already defined the signal.
            device.setup(0x401c5504, &setup)?;
        }
        let name = format!(
            "Lunchbox session gamepad {}-{}",
            std::process::id(),
            NEXT_DEVICE.fetch_add(1, Ordering::Relaxed)
        );
        ensure!(name.len() < 80, "Virtual gamepad name exceeds kernel limit");
        // uinput_setup: input_id, name[80], ff_effects_max. Do not impersonate
        // another vendor's hardware or advertise unimplemented force feedback.
        let mut setup = [0u8; 92];
        setup[..2].copy_from_slice(&0x06u16.to_ne_bytes()); // BUS_VIRTUAL
        setup[8..8 + name.len()].copy_from_slice(name.as_bytes());
        device.setup(0x405c5503, &setup)?;
        ensure!(
            unsafe { libc::ioctl(device.file.as_raw_fd(), 0x5501 as libc::c_ulong) } >= 0,
            "Cannot create normalized gamepad: {}",
            std::io::Error::last_os_error()
        );
        device.created = true;
        // UI_GET_SYSNAME returns this fd's inputN identity. Never find an owned
        // device by its display name or assume a particular js/event index.
        let mut system_name = [0u8; 128];
        let request = 0x8000_0000u32 | (128 << 16) | (0x55 << 8) | 44;
        ensure!(
            unsafe {
                libc::ioctl(
                    device.file.as_raw_fd(),
                    request as libc::c_ulong,
                    system_name.as_mut_ptr(),
                )
            } >= 0,
            "Cannot identify normalized gamepad: {}",
            std::io::Error::last_os_error()
        );
        let end = system_name
            .iter()
            .position(|byte| *byte == 0)
            .ok_or_else(|| anyhow::anyhow!("Invalid virtual gamepad identity"))?;
        let system_name = std::str::from_utf8(&system_name[..end])?;
        let suffix = system_name
            .strip_prefix("input")
            .ok_or_else(|| anyhow::anyhow!("Unexpected virtual gamepad identity"))?;
        ensure!(
            !suffix.is_empty() && suffix.bytes().all(|byte| byte.is_ascii_digit()),
            "Invalid virtual gamepad identity"
        );
        device.system_name = system_name.to_owned();
        device.publish(&device.neutral_frame())?;
        Ok(device)
    }

    fn capability(&self, request: u32, value: i32) -> Result<()> {
        use std::os::fd::AsRawFd;
        ensure!(
            unsafe {
                libc::ioctl(
                    self.file.as_raw_fd(),
                    request as libc::c_ulong,
                    value as libc::c_int,
                )
            } >= 0,
            "Cannot declare normalized gamepad capability: {}",
            std::io::Error::last_os_error()
        );
        Ok(())
    }

    fn setup(&self, request: u32, bytes: &[u8]) -> Result<()> {
        use std::os::fd::AsRawFd;
        ensure!(
            unsafe {
                libc::ioctl(
                    self.file.as_raw_fd(),
                    request as libc::c_ulong,
                    bytes.as_ptr(),
                )
            } >= 0,
            "Cannot configure normalized gamepad: {}",
            std::io::Error::last_os_error()
        );
        Ok(())
    }

    pub fn system_name(&self) -> &str {
        &self.system_name
    }

    fn neutral_frame(&self) -> std::collections::BTreeMap<GamepadControl, i32> {
        self.buttons
            .iter()
            .map(|code| (GamepadControl::Button(*code), 0))
            .chain(
                self.axes
                    .iter()
                    .map(|(code, axis)| (GamepadControl::Axis(*code), axis.neutral())),
            )
            .collect()
    }

    /// One input frame becomes one output SYN_REPORT. Validate all values
    /// before writing anything, then destroy the device on any write failure.
    pub fn publish(
        &mut self,
        frame: &std::collections::BTreeMap<GamepadControl, i32>,
    ) -> Result<()> {
        ensure!(self.created, "Normalized gamepad is no longer available");
        let result = self.publish_inner(frame);
        if result.is_err() {
            self.shutdown();
        }
        result
    }

    fn publish_inner(
        &mut self,
        frame: &std::collections::BTreeMap<GamepadControl, i32>,
    ) -> Result<()> {
        use std::io::Write;
        if frame.is_empty() {
            return Ok(());
        }
        let mut bytes = Vec::with_capacity((frame.len() + 1) * 24);
        for (control, value) in frame {
            let (kind, code) = match control {
                GamepadControl::Button(code) => {
                    ensure!(
                        self.buttons.contains(code) && (0..=1).contains(value),
                        "Invalid normalized gamepad button"
                    );
                    (1u16, *code)
                }
                GamepadControl::Axis(code) => {
                    let axis = self
                        .axes
                        .get(code)
                        .ok_or_else(|| anyhow::anyhow!("Undeclared normalized gamepad axis"))?;
                    let valid = match axis {
                        GamepadAxis::Passthrough {
                            minimum, maximum, ..
                        } => (*minimum..=*maximum).contains(value),
                        GamepadAxis::Pressure(_) => (0..=32767).contains(value),
                        GamepadAxis::Bipolar(_) => (-32767..=32767).contains(value),
                    };
                    ensure!(valid, "Normalized gamepad axis outside output bounds");
                    (3u16, *code)
                }
            };
            // 64-bit Linux input_event: zero timeval (16), type/code/value (8).
            // Explicit bytes avoid exposing struct padding to the kernel.
            bytes.extend_from_slice(&[0u8; 16]);
            bytes.extend_from_slice(&kind.to_ne_bytes());
            bytes.extend_from_slice(&code.to_ne_bytes());
            bytes.extend_from_slice(&value.to_ne_bytes());
        }
        bytes.extend_from_slice(&[0u8; 24]); // EV_SYN / SYN_REPORT / 0
        self.file.write_all(&bytes)?;
        Ok(())
    }

    pub fn shutdown(&mut self) {
        use std::os::fd::AsRawFd;
        if self.created {
            // Best effort on a broken output channel; destroying our own
            // device guarantees its lifetime cannot outlast this owner.
            let _ = self.publish_inner(&self.neutral_frame());
            unsafe {
                libc::ioctl(self.file.as_raw_fd(), 0x5502 as libc::c_ulong);
            }
            self.created = false;
        }
    }
}

#[cfg(all(target_os = "linux", target_pointer_width = "64"))]
impl Drop for VirtualGamepad {
    fn drop(&mut self) {
        self.shutdown();
    }
}

/// Owns the forwarding worker and its virtual device for one emulator session.
/// Dropping this owner requests shutdown and joins the worker; dropping the
/// worker's publisher neutralizes and destroys only the owned virtual gamepad.
#[cfg(all(target_os = "linux", target_pointer_width = "64"))]
pub struct GamepadBridge {
    stop: std::sync::mpsc::Sender<()>,
    worker: Option<std::thread::JoinHandle<()>>,
    failure: std::sync::Arc<std::sync::Mutex<Option<String>>>,
    system_name: String,
}

#[cfg(all(target_os = "linux", target_pointer_width = "64"))]
impl GamepadBridge {
    /// The caller must first establish the selected physical event-node
    /// identity and construct its measured controller-only frame contract.
    pub fn start(
        path: &std::path::Path,
        frame: GamepadFrame,
        expected_input_identity: &std::path::Path,
        cancel: &std::sync::atomic::AtomicBool,
    ) -> Result<Self> {
        ensure!(
            !cancel.load(std::sync::atomic::Ordering::Relaxed),
            "Controller bridge startup cancelled"
        );
        let reader = GamepadReader::open(path, frame, expected_input_identity)?;
        let publisher = VirtualGamepad::create(&reader.frame)?;
        let system_name = publisher.system_name().to_owned();
        let (stop, stopping) = std::sync::mpsc::channel();
        let (ready, readiness) = std::sync::mpsc::sync_channel(1);
        let failure = std::sync::Arc::new(std::sync::Mutex::new(None));
        let worker_failure = failure.clone();
        let worker = std::thread::Builder::new()
            .name("controller-pressure-bridge".to_owned())
            .spawn(move || {
                run_gamepad_bridge(reader, publisher, stopping, ready, worker_failure)
            })?;
        let bridge = Self {
            stop,
            worker: Some(worker),
            failure,
            system_name,
        };
        // No emulator may consume the initially neutral placeholder as a
        // successful synchronized session. A startup timeout drops and joins
        // this bridge, leaving no detached forwarding worker behind.
        let deadline = Instant::now() + Duration::from_secs(2);
        loop {
            ensure!(
                !cancel.load(std::sync::atomic::Ordering::Relaxed),
                "Controller bridge startup cancelled"
            );
            ensure!(
                Instant::now() < deadline,
                "Controller bridge did not synchronize within two seconds"
            );
            match readiness.recv_timeout(Duration::from_millis(20)) {
                Ok(Ok(())) => break,
                Ok(Err(error)) => anyhow::bail!("Controller bridge startup failed: {error}"),
                Err(std::sync::mpsc::RecvTimeoutError::Timeout) => continue,
                Err(error) => anyhow::bail!("Controller bridge did not synchronize: {error}"),
            }
        }
        bridge.check_health()?;
        Ok(bridge)
    }

    pub fn check_health(&self) -> Result<()> {
        let failure = self
            .failure
            .lock()
            .map_err(|_| anyhow::anyhow!("Controller bridge status unavailable"))?;
        if let Some(error) = failure.as_ref() {
            anyhow::bail!("Controller bridge failed: {error}");
        }
        ensure!(
            self.worker
                .as_ref()
                .is_some_and(|worker| !worker.is_finished()),
            "Controller bridge stopped unexpectedly"
        );
        Ok(())
    }

    /// Resolve only children of this publisher's kernel-returned inputN.
    /// None means udev/joydev has not exposed the device yet, not another pad.
    /// The launch adapter controls its own bounded readiness deadline.
    pub fn joydev_path(&self) -> Result<Option<std::path::PathBuf>> {
        self.input_node_path("js")
    }

    /// SDL's Linux evdev backend identifies controllers by event paths, not
    /// joydev ordinals. Resolve the publisher's own node without name matching.
    pub fn event_path(&self) -> Result<Option<std::path::PathBuf>> {
        self.input_node_path("event")
    }

    fn input_node_path(&self, prefix: &str) -> Result<Option<std::path::PathBuf>> {
        use std::os::unix::fs::{FileTypeExt, MetadataExt};
        self.check_health()?;
        let root = std::path::Path::new("/sys/class/input").join(&self.system_name);
        let entries = match std::fs::read_dir(&root) {
            Ok(entries) => entries,
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(None),
            Err(error) => return Err(error.into()),
        };
        let mut found = None;
        for entry in entries {
            let entry = entry?;
            let name = entry.file_name();
            let Some(name) = name.to_str() else {
                continue;
            };
            let Some(index) = name.strip_prefix(prefix) else {
                continue;
            };
            if index.is_empty() || !index.bytes().all(|byte| byte.is_ascii_digit()) {
                continue;
            }
            let path = std::path::Path::new("/dev/input").join(name);
            let file = match std::fs::File::open(&path) {
                Ok(file) => file,
                Err(error) if error.kind() == std::io::ErrorKind::NotFound => continue,
                Err(error) => return Err(error.into()),
            };
            let metadata = file.metadata()?;
            ensure!(
                metadata.file_type().is_char_device(),
                "Virtual {prefix} path is not a character device"
            );
            let expected = std::fs::read_to_string(entry.path().join("dev"))?;
            let actual = format!(
                "{}:{}",
                libc::major(metadata.rdev()),
                libc::minor(metadata.rdev())
            );
            ensure!(
                expected.trim() == actual,
                "Virtual {prefix} identity changed during discovery"
            );
            ensure!(
                found.replace(path).is_none(),
                "Virtual gamepad exposed ambiguous {prefix} nodes"
            );
        }
        self.check_health()?;
        Ok(found)
    }
}

#[cfg(all(target_os = "linux", target_pointer_width = "64"))]
fn run_gamepad_bridge(
    mut reader: GamepadReader,
    mut publisher: VirtualGamepad,
    stopping: std::sync::mpsc::Receiver<()>,
    ready: std::sync::mpsc::SyncSender<std::result::Result<(), String>>,
    failure: std::sync::Arc<std::sync::Mutex<Option<String>>>,
) {
    let mut ready = Some(ready);
    let result: Result<()> = (|| {
        loop {
            match stopping.try_recv() {
                Ok(()) | Err(std::sync::mpsc::TryRecvError::Disconnected) => return Ok(()),
                Err(std::sync::mpsc::TryRecvError::Empty) => {}
            }
            for frame in reader.poll()? {
                publisher.publish(&frame)?;
            }
            if reader.frame.synchronized {
                if let Some(ready) = ready.take() {
                    ready
                        .send(Ok(()))
                        .map_err(|_| anyhow::anyhow!("Controller bridge startup was cancelled"))?;
                }
            }
            // The channel wakes shutdown immediately; normal polling is
            // bounded to avoid a busy loop when the controller is idle.
            match stopping.recv_timeout(Duration::from_millis(2)) {
                Ok(()) | Err(std::sync::mpsc::RecvTimeoutError::Disconnected) => return Ok(()),
                Err(std::sync::mpsc::RecvTimeoutError::Timeout) => {}
            }
        }
    })();
    let neutral_result = publisher.publish(&reader.neutral());
    publisher.shutdown();
    let error = result
        .err()
        .or_else(|| neutral_result.err())
        .map(|error| format!("{error:#}"));
    if let Some(ready) = ready {
        let _ = ready.send(Err(error.clone().unwrap_or_else(|| {
            "Controller bridge stopped before synchronization".to_owned()
        })));
    }
    if let Ok(mut failure) = failure.lock() {
        *failure = error;
    }
}

#[cfg(all(target_os = "linux", target_pointer_width = "64"))]
impl Drop for GamepadBridge {
    fn drop(&mut self) {
        let _ = self.stop.send(());
        if let Some(worker) = self.worker.take() {
            let _ = worker.join();
        }
    }
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

/// Revalidate captured gestures against the current physical device mode.
/// The caller must establish this event node belongs to the selected joystick.
/// Current position is deliberately not compared with recorded rest: the user
/// may be holding a control while launching. No calibration ioctls are written.
#[cfg(target_os = "linux")]
pub fn validate_recorded_axes<'a>(
    path: &std::path::Path,
    measurements: impl IntoIterator<Item = (u32, &'a AxisMeasurement)>,
) -> Result<()> {
    let file = std::fs::File::open(path)?;
    for (encoded, measured) in measurements {
        ensure!(
            encoded >> 16 == 3 && encoded & 0xffff < 64,
            "Invalid physical axis code"
        );
        measured.validate()?;
        let current = read_info(&file, encoded as u16)?;
        ensure!(
            measured.matches_bounds(current),
            "Controller axis {} changed since calibration; check its USB mode and recalibrate",
            encoded & 0xffff
        );
        ensure!(
            (current.minimum..=current.maximum).contains(&current.value),
            "Controller axis is outside its reported range"
        );
    }
    Ok(())
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
    fn launch_bounds_check_detects_mode_changes_without_requiring_released_controls() {
        let measured = AxisMeasurement {
            minimum: -32768,
            maximum: 32767,
            flat: 128,
            fuzz: 4,
            resolution: 1,
            released: 0,
            pressed: 32767,
        };
        let baseline = AxisInfo {
            value: -24000,
            minimum: measured.minimum,
            maximum: measured.maximum,
            flat: measured.flat,
            fuzz: measured.fuzz,
            resolution: measured.resolution,
        };
        assert!(measured.matches_bounds(baseline));
        for current in [
            AxisInfo {
                minimum: 0,
                ..baseline
            },
            AxisInfo {
                maximum: 255,
                ..baseline
            },
            AxisInfo {
                flat: 0,
                ..baseline
            },
            AxisInfo {
                fuzz: 0,
                ..baseline
            },
            AxisInfo {
                resolution: 0,
                ..baseline
            },
        ] {
            assert!(!measured.matches_bounds(current));
        }
    }

    #[test]
    #[cfg(target_os = "linux")]
    #[ignore = "Read-only hardware check: set LUNCHBOX_CONTROLLER_TEST_EVENT to an evdev gamepad node"]
    fn live_axis_bounds_accept_current_mode_and_reject_changed_calibration() {
        let path = std::path::PathBuf::from(
            std::env::var_os("LUNCHBOX_CONTROLLER_TEST_EVENT").expect("Set the exact event node"),
        );
        let info = read_info(&std::fs::File::open(&path).unwrap(), 0).unwrap();
        // Synthetic endpoints exercise validation only: never save this as a
        // user's gesture or claim that hardware motion was physically tested.
        let measured = AxisMeasurement {
            minimum: info.minimum,
            maximum: info.maximum,
            flat: info.flat,
            fuzz: info.fuzz,
            resolution: info.resolution,
            released: info.minimum,
            pressed: info.maximum,
        };
        validate_recorded_axes(&path, [(0x30000, &measured)]).unwrap();
        let changed = AxisMeasurement {
            maximum: measured.maximum.checked_add(1).unwrap(),
            ..measured.clone()
        };
        assert!(
            validate_recorded_axes(&path, [(0x30000, &changed)])
                .unwrap_err()
                .to_string()
                .contains("changed since calibration")
        );
        validate_recorded_axes(&path, [(0x30000, &measured)]).unwrap();
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
