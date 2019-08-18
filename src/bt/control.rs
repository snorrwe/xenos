use super::task::*;
use super::{ExecutionError, ExecutionResult, TaskInput, MAX_TASK_PER_CONTROL};
use arrayvec::ArrayVec;
use log::Level::Debug;
use std::fmt::Write;
use std::fmt::{Display, Formatter};

pub type TaskCollection<'a, T> = ArrayVec<[Task<'a, T>; MAX_TASK_PER_CONTROL]>;

pub trait BtNode<T>: std::fmt::Debug + std::fmt::Display {
    fn tick(&self, state: &mut T) -> ExecutionResult;
}

/// Control node in the Behaviour Tree
#[derive(Clone, Debug)]
pub enum Control<'a, T>
where
    T: TaskInput,
{
    /// Runs its child tasks until the first failure
    Selector(TaskCollection<'a, T>),
    /// Runs its child tasks until the first success
    Sequence(TaskCollection<'a, T>),
}

impl<'a, T> Display for Control<'a, T>
where
    T: TaskInput,
{
    fn fmt(&self, f: &mut Formatter) -> Result<(), std::fmt::Error> {
        let tasks: ArrayVec<[&str; MAX_TASK_PER_CONTROL]> = match self {
            Control::Selector(tasks) | Control::Sequence(tasks) => {
                tasks.iter().map(|t| t.name.as_str()).collect()
            }
        };
        let name = match self {
            Control::Selector(_) => "Selector",
            Control::Sequence(_) => "Sequence",
        };
        write!(f, "Control {}, tasks: {:?}", name, tasks)
    }
}

impl<'a, T> BtNode<T> for Control<'a, T>
where
    T: TaskInput,
{
    fn tick(&self, state: &mut T) -> ExecutionResult {
        match self {
            Control::Selector(nodes) => selector(state, nodes.iter()),
            Control::Sequence(nodes) => sequence(state, nodes.iter()),
        }
    }
}

/// Run an iterator of tasks as a Selector
pub fn selector<'a, T: 'a + TaskInput, It: Iterator<Item = &'a Task<'a, T>>>(
    state: &'a mut T,
    tasks: It,
) -> ExecutionResult {
    let found = tasks
        .map(|node| (node, node.tick(state)))
        .find(|(_node, result)| result.is_err());
    if let Some(found) = found {
        if log_enabled!(Debug) {
            Err(format!(
                "Task failure in selector {:?} {:?}",
                found.1, found.0
            ))?;
        } else {
            Err(format!("A task failed in selector {:?}", found.1))?;
        }
    }
    Ok(())
}

/// Run an iterator of tasks as a Sequence
pub fn sequence<'a, T: 'a + TaskInput, It: Iterator<Item = &'a Task<'a, T>>>(
    state: &'a mut T,
    mut tasks: It,
) -> ExecutionResult {
    let mut errors: ArrayVec<[ExecutionError; MAX_TASK_PER_CONTROL]> =
        [].into_iter().cloned().collect();
    let found = tasks.any(|node| {
        let result = node.tick(state);
        debug!("Task result in sequence {:?} {:?}", node, result);
        let ok = result.is_ok();
        if let Err(err) = result {
            errors.push(err);
        }
        ok
    });
    if found {
        Ok(())
    } else {
        if log_enabled!(Debug) {
            let mut error_str = String::with_capacity(512);
            for (i, error) in errors.iter().enumerate() {
                write!(&mut error_str, "{}: {}\n", i, error).map_err(|e| {
                    error!("Failed to write to error string, aborting {:?}", e);
                    "Debug info write failure"
                })?;
            }
            Err(format!("All tasks failed in Sequence node\n{}", error_str))?
        } else {
            Err("All tasks failed in Sequence!")?
        }
    }
}

impl<'a, T> Control<'a, T>
where
    T: TaskInput,
{
    /// Sort subtasks by priority
    /// Higher priority tasks will be moved to the front
    pub fn sorted_by_priority(mut self) -> Self {
        use self::Control::*;
        match &mut self {
            Sequence(ref mut nodes) | Selector(ref mut nodes) => {
                nodes.sort_by_key(|n| -n.priority);
            }
        }
        self
    }
}

impl<'a, T: 'a + TaskInput> From<Control<'a, T>> for Task<'a, T> {
    fn from(control: Control<'a, T>) -> Task<'a, T> {
        Task::new(move |state| control.tick(state))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[derive(Debug, Default, Clone)]
    struct TestState {
        results: String,
    }

    impl TaskInput for TestState {
        fn cpu_bucket(&self) -> Option<i16> {
            None
        }
    }

    #[test]
    fn test_priority_sorting() {
        js! {}; // Enables error messages in tests

        let tasks = [
            Task::new(|state: &mut TestState| {
                state.results.push('a');
                Ok(())
            })
            .with_priority(-1),
            Task::new(|state: &mut TestState| {
                state.results.push('b');
                Err("Stop here")?
            }),
            Task::new(|state: &mut TestState| {
                state.results.push('c');
                Ok(())
            })
            .with_priority(5),
            Task::new(|state: &mut TestState| {
                state.results.push('d');
                Ok(())
            })
            .with_priority(1),
        ]
        .into_iter()
        .cloned()
        .collect();

        let mut state = TestState::default();

        let task = Control::Selector::<TestState>(tasks).sorted_by_priority();
        task.tick(&mut state).expect_err("Should have failed");

        // The order by priority is cdba, but stop the execution at 'b'
        assert_eq!(state.results, "cdb");
    }
}

