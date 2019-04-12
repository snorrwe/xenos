//! Simple Behaviour Tree implementation
//! See [Wiki](https://en.wikipedia.org/wiki/Behavior_tree_(artificial_intelligence,_robotics_and_control))
//! Notes about the way Screeps works:
//!     - There is no 'Running' state normally found in BT's
//!     - There is no explicit Task cancellation
//!
use std::ops::Fn;
use std::rc::Rc;

/// Represents a single task in the behaviour tree
/// An executable that will be called by a Task
pub type Task<'a> = Rc<Fn(&mut GameState) -> ExecutionResult + 'a>;

#[derive(Debug, Clone)]
pub struct GameState {}

/// Result of a task
pub type ExecutionResult = Result<(), String>;

pub trait BtNode {
    fn tick(&self, state: &mut GameState) -> ExecutionResult;
}

pub trait ControlNode {
    fn new(children: Vec<Task>) -> Self;
}

pub trait TaskNew<'a> {
    fn new<F>(task: F) -> Self
    where
        F: Fn(&mut GameState) -> ExecutionResult + 'a;
}

impl<'a> BtNode for Task<'a> {
    fn tick(&self, state: &mut GameState) -> ExecutionResult {
        self(state)
    }
}

impl<'a> TaskNew<'a> for Task<'a> {
    fn new<F>(task: F) -> Self
    where
        F: Fn(&mut GameState) -> ExecutionResult + 'a,
    {
        Rc::new(task)
    }
}

/// Control node in the Behaviour Tree
/// - Selector runs its child tasks until the first failure
/// - Sequence runs its child tasks until the first success
#[derive(Clone)]
pub enum Control<'a> {
    #[allow(dead_code)]
    Selector(Vec<Task<'a>>),
    Sequence(Vec<Task<'a>>),
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
                    let error = found.unwrap_err();
                    debug!("Failure in selector {:?}", error);
                    Err(error)
                } else {
                    Ok(())
                }
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
