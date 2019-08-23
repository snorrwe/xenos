use super::*;
use std::fmt::{Debug, Display, Formatter};
use std::marker::PhantomData;
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
    pub task: fn(&mut T) -> ExecutionResult,
    pub priority: i8,
    pub name: String,
    pub required_bucket: i16,
    pub post_process: Option<Rc<dyn Fn(&mut T,ExecutionResult) -> ExecutionResult + 'a>>,

    _m: PhantomData<fn() -> &'a i8>,
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
    pub fn new(task: fn(&mut T) -> ExecutionResult) -> Self {
        Self {
            task: task,
            priority: 0,
            name: "UNNAMED_TASK".to_owned(),
            required_bucket: -1,
            post_process: None,
            _m: PhantomData,
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
    pub fn with_required_bucket(mut self, bucket: i16) -> Self {
        self.required_bucket = bucket;
        self
    }

    pub fn with_post_process<'b, F>(mut self, f: F) -> Self
    where
        F: Fn(&mut T, ExecutionResult) -> ExecutionResult + 'a,
    {
        self.post_process = Some(Rc::new(f));
        self
    }
}

impl<'a, T> BtNode<T> for Task<'a, T>
where
    T: TaskInput + 'a,
{
    fn tick(&self, state: &mut T) -> ExecutionResult {
        if state
            .cpu_bucket()
            .map(|b| b > self.required_bucket)
            .unwrap_or(true)
        {
            (self.task)(state)
        } else {
            Err(format!(
                "Bucket requirement: {} not met",
                self.required_bucket
            ))?
        }
    }
}

