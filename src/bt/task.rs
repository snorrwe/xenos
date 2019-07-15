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
    T: TaskInput,
{
    /// How much "cpu bucket" is required for the task to execute
    /// Useful for turning off tasks when the bucket falls below a threshold
    pub required_bucket: i16,
    /// Priority of the task, defaults to 0
    /// Higher value means higher priority
    pub priority: i8,

    task: Rc<dyn Fn(&mut T) -> ExecutionResult + 'a>,

    pub name: String,
}

impl<'a, T> Display for Task<'a, T>
where
    T: TaskInput,
{
    fn fmt(&self, f: &mut Formatter) -> Result<(), std::fmt::Error> {
        write!(f, "Task {}", self.name)
    }
}

impl<'a, T> Debug for Task<'a, T>
where
    T: TaskInput,
{
    fn fmt(&self, f: &mut Formatter) -> Result<(), std::fmt::Error> {
        write!(
            f,
            "Task {:?} required_bucket: {:?} priority: {:?}",
            self.name, self.required_bucket, self.priority
        )
    }
}

impl<'a, T> Task<'a, T>
where
    T: TaskInput,
{
    pub fn new<F>(task: F) -> Self
    where
        F: Fn(&mut T) -> ExecutionResult + 'a,
    {
        Self {
            task: Rc::new(task),
            required_bucket: -1,
            priority: 0,
            name: "UNNAMED_TASK".to_owned(),
        }
    }

    pub fn with_name(mut self, name: &str) -> Self {
        self.name.clear();
        self.name.push_str(name);
        self
    }

    #[allow(dead_code)]
    pub fn with_priority(mut self, priority: i8) -> Self {
        self.priority = priority;
        self
    }

    pub fn with_required_bucket(mut self, bucket: i16) -> Self {
        self.required_bucket = bucket;
        self
    }

    fn assert_pre_requisites(&self, state: &mut T) -> ExecutionResult {
        if state
            .cpu_bucket()
            .map(|cb| cb < self.required_bucket)
            .unwrap_or(false)
        {
            let message = format!(
                "Task bucket requirement not met. Required: {:?}",
                self.required_bucket
            );
            Err(message)?;
        }
        Ok(())
    }
}

impl<'a, T> BtNode<T> for Task<'a, T>
where
    T: TaskInput,
{
    fn tick(&self, state: &mut T) -> ExecutionResult {
        self.assert_pre_requisites(state)?;
        (*self.task)(state).map_err(|e| {
            debug!("Task Error {:?}", e);
            e
        })
    }
}

