//! Simple Behaviour Tree implementation
//! See [Wiki](https://en.wikipedia.org/wiki/Behavior_tree_(artificial_intelligence,_robotics_and_control))
//! Notes about the way Screeps works:
//!     - There is no 'Running' state normally found in BT's
//!     - There is no explicit Task cancellation
//!
pub mod task;
pub use self::task::*;
pub use crate::game_state::*;
use arrayvec::ArrayVec;

/// Result of a task
pub type ExecutionResult = Result<(), String>;

pub type TaskCollection<'a> = ArrayVec<[Task<'a>; 16]>;

pub trait BtNode {
    fn tick(&self, state: &mut GameState) -> ExecutionResult;
}

/// Control node in the Behaviour Tree
#[derive(Clone)]
pub enum Control<'a> {
    #[allow(dead_code)]
    /// Runs its child tasks until the first failure
    Selector(TaskCollection<'a>),
    /// Runs its child tasks until the first success
    Sequence(TaskCollection<'a>),
}

impl<'a> BtNode for Control<'a> {
    fn tick(&self, state: &mut GameState) -> ExecutionResult {
        match self {
            Control::Selector(nodes) => {
                let found = nodes
                    .iter()
                    .map(|node| node.tick(state))
                    .find(|result| result.is_err());
                if let Some(found) = found {
                    Err(format!("Task failure in selector {:?}", found.unwrap_err()))?;
                }
                Ok(())
            }

            Control::Sequence(nodes) => {
                let found = nodes.iter().any(|node| node.tick(state).is_ok());
                if found {
                    Ok(())
                } else {
                    Err("All tasks failed in sequence".into())
                }
            }
        }
    }
}

