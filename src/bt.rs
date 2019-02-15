//! Simple Behaviour Tree implementation
//! See [Wiki](https://en.wikipedia.org/wiki/Behavior_tree_(artificial_intelligence,_robotics_and_control))
//! Notes about the way Screeps works:
//!     - There is no 'Running' state normally found in BT's
//!     - There is no explicit Task cancellation
//!
use std::rc::Rc;

/// Represents a single task in the behaviour tree
/// An executable that will be called by a Task
/// Currently passes an empty tuple as argument
/// The reason behind this is that we might want to pass
/// Some state between tasks.
/// So existing tasks using the pattern:
/// ```
/// |_| { /* task stuff */ }`
/// ```
/// will not require changes when this happens
pub type Task<'a> = Rc<Fn(()) -> ExecutionResult + 'a>;

/// Result of a task
pub type ExecutionResult = Result<(), ()>;

pub trait BtNode {
    fn tick(&self) -> ExecutionResult;
}

pub trait ControlNode {
    fn new(children: Vec<Task>) -> Self;
}

pub trait TaskNew<'a> {
    fn new<F>(task: F) -> Self
    where
        F: Fn(()) -> ExecutionResult + 'a;
}

impl<'a> BtNode for Task<'a> {
    fn tick(&self) -> ExecutionResult {
        self(())
    }
}

impl<'a> TaskNew<'a> for Task<'a> {
    fn new<F>(task: F) -> Self
    where
        F: Fn(()) -> ExecutionResult + 'a,
    {
        Rc::new(task)
    }
}

/// Control node in the Behaviour Tree
/// - Selector runs its child tasks until the first failure
/// - Sequence runs its child tasks until the first success
/// - All runs all its child tasks regardless of their result
#[derive(Clone)]
pub enum Control<'a> {
    #[allow(dead_code)]
    Selector(Vec<Task<'a>>),
    Sequence(Vec<Task<'a>>),
    All(Vec<Task<'a>>),
}

impl<'a> BtNode for Control<'a> {
    fn tick(&self) -> ExecutionResult {
        match self {
            Control::Selector(nodes) => {
                let found = nodes.iter().any(|node| node.tick().is_err());
                if !found {
                    Ok(())
                } else {
                    Err(())
                }
            }
            Control::Sequence(nodes) => {
                let found = nodes.iter().any(|node| node.tick().is_ok());
                if found {
                    Ok(())
                } else {
                    Err(())
                }
            }
            Control::All(nodes) => {
                nodes.iter().for_each(|node| {
                    node.tick().unwrap_or_else(|e| {
                        warn!("node failure in an All control {:?}", e);
                    });
                });
                Ok(())
            }
        }
    }
}
