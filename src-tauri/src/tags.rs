//! Re-export tags module from lunchbox-core
//!
//! The canonical implementation lives in lunchbox_core::tags.
//! This re-export ensures all existing `crate::tags::*` references
//! continue to work without changes.

pub use lunchbox_core::tags::*;
