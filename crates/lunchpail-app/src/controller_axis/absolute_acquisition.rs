//! Explicit rectangular edge acquisition. Hardware timestamps and UI ownership
//! belong to the capture controller; this model never opens or saves a device.
use super::absolute_calibration::AbsoluteRectangleCalibration;
use super::absolute_packets::AbsolutePacket;
use super::absolute_settings::AbsoluteAxisBounds;
use anyhow::{Result, ensure};
use std::time::{Duration, Instant};

#[derive(Clone, Copy, Debug, serde::Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CalibrationEdge {
    Left,
    Right,
    Top,
    Bottom,
}

pub struct AbsoluteEdgeAcquisition {
    axes: [AbsoluteAxisBounds; 2],
    swap_xy: bool,
    edges: [Option<i32>; 4],
    last_capture: Option<Instant>,
    active: bool,
}

impl AbsoluteEdgeAcquisition {
    pub fn new(axes: [AbsoluteAxisBounds; 2], swap_xy: bool) -> Result<Self> {
        axes[0].validate()?;
        axes[1].validate()?;
        ensure!(
            axes[0].code != axes[1].code,
            "Absolute axes must be distinct"
        );
        Ok(Self {
            axes,
            swap_xy,
            edges: [None; 4],
            last_capture: None,
            active: true,
        })
    }

    /// The caller must supply the actual sample time on the same monotonic clock
    /// as `now`, not the dequeue time of a potentially old buffered event. A new
    /// report is required for every explicit edge confirmation. The transport
    /// must invalidate this acquisition on disconnect, bounds change or loss.
    pub fn record(
        &mut self,
        edge: CalibrationEdge,
        packet: AbsolutePacket,
        captured_at: Instant,
        now: Instant,
    ) -> Result<()> {
        ensure!(self.active, "Absolute acquisition has been invalidated");
        let age = now
            .checked_duration_since(captured_at)
            .ok_or_else(|| anyhow::anyhow!("Absolute sample timestamp is in the future"))?;
        ensure!(
            age <= Duration::from_millis(500),
            "Absolute sample is stale; acquire a fresh report"
        );
        ensure!(
            self.last_capture
                .is_none_or(|previous| captured_at > previous),
            "Each calibration edge requires a newer report"
        );
        for (axis, value) in self.axes.into_iter().zip([packet.raw_x, packet.raw_y]) {
            ensure!(
                (axis.minimum..=axis.maximum).contains(&value),
                "Absolute acquisition sample exceeds recorded hardware bounds"
            );
        }
        let (x, y) = if self.swap_xy {
            (packet.raw_y, packet.raw_x)
        } else {
            (packet.raw_x, packet.raw_y)
        };
        let (index, value) = match edge {
            CalibrationEdge::Left => (0, x),
            CalibrationEdge::Right => (1, x),
            CalibrationEdge::Top => (2, y),
            CalibrationEdge::Bottom => (3, y),
        };
        self.edges[index] = Some(value);
        self.last_capture = Some(captured_at);
        Ok(())
    }

    pub fn recorded_edges(&self) -> [Option<i32>; 4] {
        self.edges
    }

    /// Produces a calibration for review, not an automatically persisted setting.
    /// No perspective, target placement or viewport correctness is inferred.
    pub fn calibration(&self) -> Result<AbsoluteRectangleCalibration> {
        ensure!(self.active, "Absolute acquisition has been invalidated");
        let [Some(left), Some(right), Some(top), Some(bottom)] = self.edges else {
            anyhow::bail!("Record all four calibration edges before continuing")
        };
        let calibration = AbsoluteRectangleCalibration {
            left,
            right,
            top,
            bottom,
            swap_xy: self.swap_xy,
        };
        calibration.validate()?;
        Ok(calibration)
    }

    pub fn invalidate(&mut self) {
        self.active = false;
        self.edges = [None; 4];
        self.last_capture = None;
    }
}
