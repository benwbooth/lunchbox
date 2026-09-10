//! Owned absolute calibration capture; callers must explicitly start and poll it.
use super::absolute::{AbsoluteReader, TimedAbsolutePacket};
use super::absolute_acquisition::{AbsoluteEdgeAcquisition, CalibrationEdge};
use super::absolute_settings::AbsoluteDeviceSettings;
use anyhow::{Result, ensure};
use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};

pub struct AbsoluteCalibrationSession {
    reader: AbsoluteReader,
    acquisition: AbsoluteEdgeAcquisition,
    event_path: PathBuf,
    input_identity: PathBuf,
    latest: Option<TimedAbsolutePacket>,
    deadline: Instant,
    active: bool,
}

impl AbsoluteCalibrationSession {
    pub fn open(
        event_path: &Path,
        input_identity: &Path,
        x_code: u16,
        y_code: u16,
        swap_xy: bool,
    ) -> Result<Self> {
        let reader = AbsoluteReader::open_uncalibrated(event_path, input_identity, x_code, y_code)?;
        let acquisition = AbsoluteEdgeAcquisition::new(reader.axis_bounds(), swap_xy)?;
        Ok(Self {
            reader,
            acquisition,
            event_path: event_path.to_owned(),
            input_identity: input_identity.to_owned(),
            latest: None,
            deadline: Instant::now() + Duration::from_secs(120),
            active: true,
        })
    }

    /// Call regularly from the owning capture controller. Empty polls do not
    /// refresh sample timestamps. An error cancels the complete acquisition.
    pub fn poll(&mut self) -> Result<Option<TimedAbsolutePacket>> {
        ensure!(self.active, "Absolute calibration session is closed");
        if Instant::now() >= self.deadline {
            self.cancel();
            anyhow::bail!("Absolute calibration timed out; start a new session");
        }
        match self.reader.poll_timed() {
            Ok(packets) => {
                let newest = packets.last().copied();
                if newest.is_some() {
                    self.latest = newest;
                }
                Ok(newest)
            }
            Err(error) => {
                self.cancel();
                Err(error)
            }
        }
    }

    /// Explicit user confirmation of one edge; no automatic extremes inference.
    /// A stale or repeated sample is recoverable by moving and confirming again.
    pub fn record_edge(&mut self, edge: CalibrationEdge) -> Result<()> {
        self.poll()?;
        let sample = self.latest.ok_or_else(|| {
            anyhow::anyhow!("No complete absolute position yet; move both selected axes")
        })?;
        self.acquisition
            .record(edge, sample.position, sample.captured_at, Instant::now())
    }

    pub fn recorded_edges(&self) -> [Option<i32>; 4] {
        self.acquisition.recorded_edges()
    }

    /// Consume the session into a validated review draft, closing capture whether
    /// validation succeeds or fails. This never stages or persists user settings.
    pub fn finish(mut self) -> Result<AbsoluteDeviceSettings> {
        self.poll()?;
        let [x_axis, y_axis] = self.reader.axis_bounds();
        let settings = AbsoluteDeviceSettings {
            event_path: self.event_path.clone(),
            input_identity: self.input_identity.clone(),
            x_axis,
            y_axis,
            calibration: self.acquisition.calibration()?,
        };
        settings.validate()?;
        Ok(settings)
    }

    pub fn cancel(&mut self) {
        self.active = false;
        self.latest = None;
        self.acquisition.invalidate();
        self.reader.close();
    }
}

impl Drop for AbsoluteCalibrationSession {
    fn drop(&mut self) {
        self.cancel();
    }
}

#[derive(serde::Deserialize)]
#[serde(tag = "action", rename_all = "snake_case", deny_unknown_fields)]
enum CaptureCommand {
    Start {
        event_path: PathBuf,
        input_identity: PathBuf,
        x_code: u16,
        y_code: u16,
        swap_xy: bool,
    },
    Poll,
    Record {
        edge: CalibrationEdge,
    },
    Finish,
    Cancel,
}

/// UI command boundary. Opening is explicit; output is always a review draft,
/// never a mutation of saved settings. The owner must poll and cancel on hiding.
pub fn command(
    session: &mut Option<AbsoluteCalibrationSession>,
    request: &str,
) -> serde_json::Value {
    let result = (|| -> Result<serde_json::Value> {
        ensure!(
            request.len() <= 16 * 1024,
            "Absolute capture request exceeds size limit"
        );
        let command: CaptureCommand = serde_json::from_str(request)?;
        match command {
            CaptureCommand::Start {
                event_path,
                input_identity,
                x_code,
                y_code,
                swap_xy,
            } => {
                ensure!(
                    session.is_none(),
                    "Cancel the active absolute capture before starting another"
                );
                *session = Some(AbsoluteCalibrationSession::open(
                    &event_path,
                    &input_identity,
                    x_code,
                    y_code,
                    swap_xy,
                )?);
                Ok(serde_json::json!({"started": true}))
            }
            CaptureCommand::Cancel => {
                session.take();
                Ok(serde_json::json!({"cancelled": true}))
            }
            CaptureCommand::Finish => {
                let capture = session
                    .take()
                    .ok_or_else(|| anyhow::anyhow!("No active absolute capture"))?;
                Ok(serde_json::json!({"draft": capture.finish()?}))
            }
            CaptureCommand::Poll => {
                let capture = session
                    .as_mut()
                    .ok_or_else(|| anyhow::anyhow!("No active absolute capture"))?;
                let sample = capture.poll()?;
                Ok(serde_json::json!({
                    "sample": sample.map(|value| serde_json::json!({
                        "position": value.position,
                        "age_ms": Instant::now().saturating_duration_since(value.captured_at).as_millis()
                    })),
                    "edges": capture.recorded_edges()
                }))
            }
            CaptureCommand::Record { edge } => {
                let capture = session
                    .as_mut()
                    .ok_or_else(|| anyhow::anyhow!("No active absolute capture"))?;
                capture.record_edge(edge)?;
                Ok(serde_json::json!({"edges": capture.recorded_edges()}))
            }
        }
    })();
    if session.as_ref().is_some_and(|capture| !capture.active) {
        session.take();
    }
    let mut output = match result {
        Ok(value) => value,
        Err(error) => serde_json::json!({"error": format!("{error:#}")}),
    };
    output["active"] = serde_json::json!(session.is_some());
    output["runtime_verified"] = serde_json::json!(false);
    output
}
