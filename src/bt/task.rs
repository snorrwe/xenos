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
    pub required_bucket: Option<i16>,
    task: Rc<Fn(&mut GameState) -> ExecutionResult + 'a>,
}

impl<'a> Task<'a> {
    pub fn new<F>(task: F) -> Self
    where
        F: Fn(&mut GameState) -> ExecutionResult + 'a,
    {
        Self {
            task: Rc::new(task),
            required_bucket: None,
        }
    }

    pub fn with_required_bucket(mut self, bucket: i16) -> Self {
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
            let message = format!(
                "Task bucket requirement not met. Required: {:?}",
                required_bucket
            );
            debug!("{} State: {:?}", &message, state);
            Err(message)?;
        }
        Ok(())
    }
}

impl<'a> BtNode for Task<'a> {
    fn tick(&self, state: &mut GameState) -> ExecutionResult {
        self.assert_pre_requisites(state)?;
        (*self.task)(state).map_err(|e| {
            debug!("Task Error {:?}", e);
            e
        })
    }
}
