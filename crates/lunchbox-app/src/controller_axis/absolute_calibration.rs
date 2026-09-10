//! Rectangular absolute-position calibration in physical device units.
//! This does not capture a gun, infer a display viewport, correct perspective,
//! or turn an out-of-area position into a hardware offscreen/reload signal.
use anyhow::{Result, ensure};
use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct AbsoluteRectangleCalibration {
    /// Raw readings at the left/right calibrated edges, after optional swap.
    /// Descending values explicitly represent a reversed physical axis.
    pub left: i32,
    pub right: i32,
    pub top: i32,
    pub bottom: i32,
    pub swap_xy: bool,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
pub struct AbsoluteProjection {
    /// Closed unsigned calibration interval. A runtime adapter must establish
    /// its own output device bounds and viewport conversion before using this.
    pub x: u16,
    pub y: u16,
    /// True when either raw reading lies beyond its calibrated edges. The
    /// coordinates are bounded, but this information is never silently lost.
    pub outside_calibrated_area: bool,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
pub struct LibretroAbsoluteProjection {
    /// Ordinary libretro pointer/gun coordinates exclude the -32768 sentinel.
    pub x: i16,
    pub y: i16,
    /// Calibration overflow is not a hardware offscreen/reload observation.
    pub outside_calibrated_area: bool,
}

impl AbsoluteRectangleCalibration {
    pub fn validate(self) -> Result<()> {
        ensure!(
            self.left != self.right,
            "Absolute X calibration has zero width"
        );
        ensure!(
            self.top != self.bottom,
            "Absolute Y calibration has zero height"
        );
        Ok(())
    }

    /// Integer nearest rounding, endpoints exact, no centered-stick dead zone
    /// or relative-velocity integration. i128 intermediates cover the full i32
    /// reading range, including reversed extreme endpoints.
    pub fn project(self, raw_x: i32, raw_y: i32) -> Result<AbsoluteProjection> {
        self.project_range(raw_x, raw_y, u16::MAX)
    }

    /// Map raw calibration directly to the libretro coordinate interval, without
    /// double-rounding through the unsigned preview. FBNeo CinpDirectCoord adds
    /// 0x7fff and treats -32768 as offscreen; an ordinary edge must not emit it.
    pub fn project_libretro(self, raw_x: i32, raw_y: i32) -> Result<LibretroAbsoluteProjection> {
        let projection = self.project_range(raw_x, raw_y, 65534)?;
        Ok(LibretroAbsoluteProjection {
            x: (i32::from(projection.x) - 32767) as i16,
            y: (i32::from(projection.y) - 32767) as i16,
            outside_calibrated_area: projection.outside_calibrated_area,
        })
    }

    fn project_range(self, raw_x: i32, raw_y: i32, maximum: u16) -> Result<AbsoluteProjection> {
        self.validate()?;
        let (x, y) = if self.swap_xy {
            (raw_y, raw_x)
        } else {
            (raw_x, raw_y)
        };
        let project_axis = |value: i32, first: i32, last: i32| {
            let span = i128::from(last) - i128::from(first);
            let offset = (i128::from(value) - i128::from(first)) * span.signum();
            let extent = span.abs();
            let outside = offset < 0 || offset > extent;
            let bounded = offset.clamp(0, extent);
            let normalized = (bounded * i128::from(maximum) + extent / 2) / extent;
            (normalized as u16, outside)
        };
        let (x, outside_x) = project_axis(x, self.left, self.right);
        let (y, outside_y) = project_axis(y, self.top, self.bottom);
        Ok(AbsoluteProjection {
            x,
            y,
            outside_calibrated_area: outside_x || outside_y,
        })
    }
}
