//! Behaviour Tree implementation
//! See [Wiki](https://en.wikipedia.org/wiki/Behavior_tree_(artificial_intelligence,_robotics_and_control))
//!
use std::fmt;
use std::fmt::Debug;
use std::rc::Rc;

pub struct BehaviourTree {
    root: Control,
}

impl BehaviourTree {
    pub fn new(root: Control) -> Self {
        Self { root: root }
    }
}

impl BtNode for BehaviourTree {
    fn tick(&self) -> ExecutionResult {
        self.root.tick()
    }
}

#[derive(Debug, Clone)]
pub enum Node {
    Task(Task),
    Control(Control),
}

pub type ExecutionResult = Result<(), ()>;

pub trait BtNode {
    fn tick(&self) -> ExecutionResult;
}

pub trait ControlNode {
    fn new(children: Vec<Node>) -> Self;
}

impl BtNode for Node {
    fn tick(&self) -> ExecutionResult {
        match self {
            Node::Control(node) => node.tick(),
            Node::Task(node) => node.tick(),
        }
    }
}

/// Control node in the Behaviour Tree
/// Selector runs its child tasks until the first failure
/// Sequence runs its child tasks until the first success
#[derive(Debug, Clone)]
pub enum Control {
    Selector(Vec<Node>),
    Sequence(Vec<Node>),
}

impl BtNode for Control {
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
        }
    }
}

/// Represents a single task in the behaviour tree
#[derive(Clone)]
pub struct Task {
    name: &'static str,
    task: Rc<Fn() -> ExecutionResult>,
}

impl Debug for Task {
    fn fmt(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
        write!(formatter, "Task {}", self.name)
    }
}

impl Task {
    pub fn new(name: &'static str, task: &'static Fn() -> ExecutionResult) -> Self {
        Self {
            name: name,
            task: Rc::new(task),
        }
    }
}

impl BtNode for Task {
    fn tick(&self) -> ExecutionResult {
        trace!("Executing task {:?}", self);
        let task = &*self.task;
        task()
    }
}

