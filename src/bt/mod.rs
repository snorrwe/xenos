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

#[derive(Default, Debug, Clone)]
pub struct ExecutionError(ArrayString<[u8; 128]>);

/// Result of a task
pub type ExecutionResult = Result<(), ExecutionError>;

/// Input to a Task
pub trait TaskInput {
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

pub fn sorted_by_priority<'a, T: TaskInput>(nodes: &mut [Task<'a, T>]) {
    nodes.sort_by_key(|n| -n.priority);
}

pub trait BtNode<T>: std::fmt::Debug + std::fmt::Display {
    fn tick(&self, state: &mut T) -> ExecutionResult;
}

#[cfg(test)]
mod tests {
    use super::*;

    #[derive(Debug, Default, Clone)]
    struct TestState {
        results: String,
    }

    impl TaskInput for TestState {
        fn cpu_bucket(&self) -> Option<i16> {
            None
        }
    }

    #[test]
    fn test_priority_sorting() {
        js! {}; // Enables error messages in tests

        let mut tasks = [
            Task::new(|state: &mut TestState| {
                state.results.push('a');
                Ok(())
            })
            .with_priority(-1),
            Task::new(|state: &mut TestState| {
                state.results.push('b');
                Err("Stop here")?
            }),
            Task::new(|state: &mut TestState| {
                state.results.push('c');
                Ok(())
            })
            .with_priority(5),
            Task::new(|state: &mut TestState| {
                state.results.push('d');
                Ok(())
            })
            .with_priority(1),
        ];

        sorted_by_priority(&mut tasks);

        let mut state = TestState::default();

        selector(&mut state, tasks.iter()).expect_err("Should have failed");

        // The order by priority is cdba, but stop the execution at 'b'
        assert_eq!(state.results, "cdb");
    }
}

