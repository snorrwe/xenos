//! Simple Behaviour Tree implementation
//! See [Wiki](https://en.wikipedia.org/wiki/Behavior_tree_(artificial_intelligence,_robotics_and_control))
//! Notes about the way Screeps works:
//!     - There is no 'Running' state normally found in BT's
//!     - There is no explicit Task cancellation
//!
pub mod task;
pub use self::task::*;

#[derive(Debug, Clone, Default)]
pub struct GameState {
    /// CPU bucket available this tick
    pub cpu_bucket: Option<i32>,
    /// Lazily countable global conqueror creep count
    pub conqueror_count: Option<i8>,
}

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
