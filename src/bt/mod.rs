//! Simple Behaviour Tree implementation
//! See [Wiki](https://en.wikipedia.org/wiki/Behavior_tree_(artificial_intelligence,_robotics_and_control))
//! Notes about the way Screeps works:
//!     - There is no 'Running' state normally found in BT's
//!     - There is no explicit Task cancellation
//!
pub mod control;
pub mod graph;
use self::graph::*;
use arrayvec::ArrayVec;
use std::ops::Deref;

pub const MAX_TASK_PER_CONTROL: usize = 16;
pub const MAX_CHANGE_PER_TASK: usize = 16;
pub const MAX_DATA_SIZE_PER_TASK_IN_BYTES: usize = 32;

/// Result of a task
pub type ExecutionResult = Result<(), String>;
pub type ChangeSet<T> = ArrayVec<[Change<T>; MAX_CHANGE_PER_TASK]>;
pub type TaskCollection<T> = ArrayVec<[Task<T>; MAX_TASK_PER_CONTROL]>;
pub type ChildNodes<T> = Vec<TaskGraph<T>>;
pub type TaskFnPtr<T> = fn(TaskEntity, &mut TaskGraph<T>) -> ExecutionResult;

#[derive(Clone)]
pub struct Task<T>(pub TaskEntity, pub TaskFnPtr<T>);

impl<T> Task<T> {
    pub fn tick(&self, graph: &mut TaskGraph<T>) -> ExecutionResult {
        self.1(self.0, graph)
    }
}

#[derive(Debug, Clone, Copy)]
pub struct TaskEntity(usize);

impl TaskEntity {
    pub fn next() -> Self {
        static mut i: usize = 0;
        i += 1;
        Self(i)
    }

    pub fn into_inner(self) -> usize {
        self.0
    }
}

impl Deref for TaskEntity {
    type Target = usize;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

#[cfg(test)]
mod tests {
    // use super::*;

    #[test]
    fn test_single_task_creation() {}
}

