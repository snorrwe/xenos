//! Simple Behaviour Tree implementation
//! See [Wiki](https://en.wikipedia.org/wiki/Behavior_tree_(artificial_intelligence,_robotics_and_control))
//! Notes about the way Screeps works:
//!     - There is no 'Running' state normally found in BT's
//!     - There is no explicit Task cancellation
//!
pub mod task;
pub mod control;
pub use self::task::*;
pub use self::control::*;

pub const MAX_TASK_PER_CONTROL: usize = 16;

/// Result of a task
pub type ExecutionResult = Result<(), String>;
