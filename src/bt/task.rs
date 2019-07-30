use super::*;
use std::fmt::{Debug, Display, Formatter};
use std::ops::Fn;
use std::rc::Rc;

/// Represents a single task in the behaviour tree
/// An executable that will be called by a Task
///
#[derive(Clone)]
pub struct Task<'a, T>
where
    T: TaskInput + 'a,
{
    /// Priority of the task, defaults to 0
    /// Higher value means higher priority
    pub task: Rc<dyn Fn(&mut T) -> ExecutionResult + 'a>,
    pub priority: i8,
    pub name: String,
}

impl<'a, T: 'a> Display for Task<'a, T>
where
    T: TaskInput,
{
    fn fmt(&self, f: &mut Formatter) -> Result<(), std::fmt::Error> {
        write!(f, "Task {}", self.name)
    }
}

impl<'a, T: 'a> Debug for Task<'a, T>
where
    T: TaskInput,
{
    fn fmt(&self, f: &mut Formatter) -> Result<(), std::fmt::Error> {
        write!(f, "Task {:?} priority: {:?}", self.name, self.priority)
    }
}

impl<'a, T: 'a> Task<'a, T>
where
    T: TaskInput,
{
    pub fn new<F>(task: F) -> Self
    where
        F: Fn(&mut T) -> ExecutionResult + 'a,
    {
        Self {
            task: Rc::new(task),
            priority: 0,
            name: "UNNAMED_TASK".to_owned(),
        }
    }

    pub fn with_name(mut self, name: &str) -> Self {
        self.name.clear();
        self.name.push_str(name);
        self
    }

    pub fn with_priority(mut self, priority: i8) -> Self {
        self.priority = priority;
        self
    }

    /// How much "cpu bucket" is required for the task to execute
    /// Useful for turning off tasks when the bucket falls below a threshold
    pub fn with_required_bucket(self, bucket: i16) -> Self {
        Self::new(move |state| {
            if state.cpu_bucket().map(|cb| cb < bucket).unwrap_or(false) {
                let message = format!("Task bucket requirement not met. Required: {:?}", bucket);
                Err(message)?;
            }
            self.tick(state)
        })
    }
}

impl<'a, T> BtNode<T> for Task<'a, T>
where
    T: TaskInput + 'a,
{
    fn tick(&self, state: &mut T) -> ExecutionResult {
        (*self.task)(state).map_err(|e| {
            debug!("Task Error {:?}", e);
            e
        })
    }
}

