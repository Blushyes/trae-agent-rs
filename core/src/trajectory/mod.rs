//! Execution trajectory recording and replay

pub mod recorder;
pub mod entry;

pub use recorder::TrajectoryRecorder;
pub use entry::{TrajectoryEntry, EntryType};
