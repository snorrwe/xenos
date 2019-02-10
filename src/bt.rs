//! Simple Behaviour Tree implementation
//! See [Wiki](https://en.wikipedia.org/wiki/Behavior_tree_(artificial_intelligence,_robotics_and_control))
//! Notes:
//!     - Because of the way Screeps works we will not use the 'Running' state normally found in BT's
//!     - For the above reason we have no Task cancellation
//!
use std::fmt::{self, Debug};
use std::rc::Rc;

pub struct BehaviourTree<'a> {
    root: Control<'a>,
}

impl<'a> BehaviourTree<'a> {
    pub fn new(root: Control<'a>) -> Self {
        Self { root: root }
    }
}

impl<'a> BtNode for BehaviourTree<'a> {
    fn tick(&self) -> ExecutionResult {
        self.root.tick()
    }
}

#[derive(Debug, Clone)]
pub enum Node<'a> {
    Task(Task<'a>),
    Control(Control<'a>),
}

pub type ExecutionResult = Result<(), ()>;

pub trait BtNode {
    fn tick(&self) -> ExecutionResult;
}

pub trait ControlNode {
    fn new(children: Vec<Node>) -> Self;
}

impl<'a> BtNode for Node<'a> {
    fn tick(&self) -> ExecutionResult {
        match self {
            Node::Control(node) => node.tick(),
            Node::Task(node) => node.tick(),
        }
    }
}

/// Control node in the Behaviour Tree
/// - Selector runs its child tasks until the first failure
/// - Sequence runs its child tasks until the first success
/// - All runs all its child tasks regardless of their result
#[derive(Debug, Clone)]
pub enum Control<'a> {
    Selector(Vec<Node<'a>>),
    Sequence(Vec<Node<'a>>),
    All(Vec<Node<'a>>),
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
                        error!("node failure in an All control {:?}", e);
                    });
                });
                Ok(())
            }
        }
    }
}

/// Represents a single task in the behaviour tree
#[derive(Clone)]
pub struct Task<'a> {
    name: &'a str,
    task: Rc<Fn() -> ExecutionResult + 'a>,
}

impl<'a> Debug for Task<'a> {
    fn fmt(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
        write!(formatter, "Task {}", self.name)
    }
}

impl<'a> Task<'a> {
    pub fn new<F>(name: &'a str, task: F) -> Self
    where
        F: Fn() -> ExecutionResult + 'a,
    {
        Self {
            name: name,
            task: Rc::new(task),
        }
    }
}

impl<'a> BtNode for Task<'a> {
    fn tick(&self) -> ExecutionResult {
        trace!("Executing task {:?}", self);
        let task = &*self.task;
        task()
    }
}
