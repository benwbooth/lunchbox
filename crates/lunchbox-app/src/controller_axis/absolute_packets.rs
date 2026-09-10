//! Scalar absolute report assembly, independent of hardware opening and routing.
//! Unlike relative deltas, absolute coordinates retain their last reported value.
use super::absolute_settings::AbsoluteAxisBounds;
use anyhow::{Result, ensure};
use serde::Serialize;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
pub struct AbsolutePacket {
    pub raw_x: i32,
    pub raw_y: i32,
}

/// Feed ordered evdev events from a single, identity-verified source. Construction
/// supplies no guessed center: both axes must have been observed before output.
/// Only SYN_REPORT publishes a position. Buttons, contact state, offscreen and
/// reload are separate protocols and are deliberately not inferred here.
pub struct AbsoluteReportAssembler {
    axes: [AbsoluteAxisBounds; 2],
    values: [Option<i32>; 2],
    changed: bool,
    events_in_report: usize,
    failed: bool,
}

impl AbsoluteReportAssembler {
    pub fn new(x_axis: AbsoluteAxisBounds, y_axis: AbsoluteAxisBounds) -> Result<Self> {
        x_axis.validate()?;
        y_axis.validate()?;
        ensure!(x_axis.code != y_axis.code, "Absolute axes must be distinct");
        Ok(Self {
            axes: [x_axis, y_axis],
            values: [None, None],
            changed: false,
            events_in_report: 0,
            failed: false,
        })
    }

    /// No partial report is returned. On any error the assembler is permanently
    /// invalid; the owning reader must discard this poll's accumulated batch,
    /// close the source and establish a fresh session before producing input.
    pub fn push(&mut self, kind: u16, code: u16, value: i32) -> Result<Option<AbsolutePacket>> {
        ensure!(
            !self.failed,
            "Absolute input failed; establish a new session"
        );
        let result = self.push_inner(kind, code, value);
        if result.is_err() {
            self.failed = true;
            self.values = [None, None];
            self.changed = false;
        }
        result
    }

    fn push_inner(&mut self, kind: u16, code: u16, value: i32) -> Result<Option<AbsolutePacket>> {
        ensure!(
            !(kind == 0 && code == 3),
            "Absolute input synchronization lost; establish a new session"
        );
        if kind == 0 && code == 0 {
            self.events_in_report = 0;
            let changed = std::mem::take(&mut self.changed);
            return Ok(match (changed, self.values) {
                (true, [Some(raw_x), Some(raw_y)]) => Some(AbsolutePacket { raw_x, raw_y }),
                _ => None,
            });
        }
        self.events_in_report += 1;
        ensure!(
            self.events_in_report <= 4096,
            "Absolute report exceeds event limit"
        );
        if kind == 3 {
            if let Some(index) = self.axes.iter().position(|axis| axis.code == code) {
                let axis = self.axes[index];
                ensure!(
                    (axis.minimum..=axis.maximum).contains(&value),
                    "Absolute sample exceeds recorded bounds; sentinel protocols require a separate adapter"
                );
                self.changed |= self.values[index] != Some(value);
                self.values[index] = Some(value);
            }
        }
        Ok(None)
    }
}
