//! Simple Behaviour Tree implementation
//! See [Wiki](https://en.wikipedia.org/wiki/Behavior_tree_(artificial_intelligence,_robotics_and_control))
//! Notes about the way Screeps works:
//!     - There is no 'Running' state normally found in BT's
//!     - There is no explicit Task cancellation
//!
pub mod control;
pub mod task;
pub use self::control::*;
pub use self::task::*;
use arrayvec::ArrayString;
use std::fmt::{self, Display, Formatter};

pub const MAX_TASK_PER_CONTROL: usize = 16;

#[derive(Default, Debug, Clone)]
pub struct ExecutionError(ArrayString<[u8; 64]>);

/// Result of a task
pub type ExecutionResult = Result<(), ExecutionError>;

/// Input to a Task
pub trait TaskInput: std::fmt::Debug {
    fn cpu_bucket(&self) -> Option<i16>;
}

impl TaskInput for () {
    fn cpu_bucket(&self) -> Option<i16> {
        None
    }
}

impl<'a> From<&'a str> for ExecutionError {
    fn from(s: &'a str) -> Self {
        let mut result = Self::default();
        result.0.push_str(s);
        result
    }
}

impl From<String> for ExecutionError {
    fn from(s: String) -> Self {
        Self::from(s.as_str())
    }
}

impl Display for ExecutionError {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0.as_str())
    }
}

