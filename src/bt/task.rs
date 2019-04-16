use super::*;
use std::ops::Fn;
use std::rc::Rc;

/// Represents a single task in the behaviour tree
/// An executable that will be called by a Task
///
#[derive(Clone)]
pub struct Task<'a> {
    /// How much "cpu bucket" is required for the task to execute
    /// Useful for turning off tasks when the bucket falls below a threshold
    pub required_bucket: Option<i32>,

    task: Rc<Fn(&mut GameState) -> ExecutionResult + 'a>,
}

impl<'a> Task<'a> {
    pub fn with_required_bucket(mut self, bucket: i32) -> Self {
        self.required_bucket = Some(bucket);
        self
    }

    fn assert_pre_requisites(&self, state: &mut GameState) -> ExecutionResult {
        if self
            .required_bucket
            .map(|rb| state.cpu_bucket.map(|cb| cb < rb).unwrap_or(false))
            .unwrap_or(false)
        {
            let required_bucket = self.required_bucket.unwrap();
            debug!(
                "Task bucket requirement not met. Required: {:?}, State: {:?}",
                required_bucket, state
            );
            Err(format!(
                "Task bucket requirement ({:?}) not met",
                required_bucket
            ))?;
        }
        Ok(())
    }
}

impl<'a> BtNode for Task<'a> {
    fn tick(&self, state: &mut GameState) -> ExecutionResult {
        self.assert_pre_requisites(state)?;
        (*self.task)(state)
    }
}

impl<'a> TaskNew<'a> for Task<'a> {
    fn new<F>(task: F) -> Self
    where
        F: Fn(&mut GameState) -> ExecutionResult + 'a,
    {
        Self {
            task: Rc::new(task),
            required_bucket: None,
        }
    }
}

