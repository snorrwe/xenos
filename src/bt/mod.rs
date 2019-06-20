//! Simple Behaviour Tree implementation
//! See [Wiki](https://en.wikipedia.org/wiki/Behavior_tree_(artificial_intelligence,_robotics_and_control))
//! Notes about the way Screeps works:
//!     - There is no 'Running' state normally found in BT's
//!     - There is no explicit Task cancellation
//!
pub mod task;
pub use self::task::*;
use arrayvec::ArrayVec;
use std::fmt::{Display, Formatter};

pub const MAX_TASK_PER_CONTROL: usize = 16;

/// Result of a task
pub type ExecutionResult = Result<(), String>;

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
    #[allow(dead_code)]
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
        let tasks: ArrayVec<[&'a str; MAX_TASK_PER_CONTROL]> = match self {
            Control::Selector(tasks) | Control::Sequence(tasks) => {
                tasks.iter().map(|t| t.name).collect()
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
            Control::Selector(nodes) => {
                let found = nodes
                    .iter()
                    .map(|node| (node, node.tick(state)))
                    .find(|(_node, result)| result.is_err());
                if let Some(found) = found {
                    Err(format!(
                        "Task failure in selector {:?} {:?}",
                        found.1, found.0
                    ))?;
                }
                Ok(())
            }

            Control::Sequence(nodes) => {
                let found = nodes.iter().any(|node| {
                    let result = node.tick(state);
                    debug!("Task result in sequence {:?} {:?}", node, result);
                    result.is_ok()
                });
                if found {
                    Ok(())
                } else {
                    Err(format!("All tasks failed in {}", &self))
                }
            }
        }
    }
}

impl<'a, T> Control<'a, T>
where
    T: TaskInput,
{
    #[allow(dead_code)]
    /// Sort subtasks by priority
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
        let tasks = [
            Task::new(|state: &mut TestState| {
                state.results.push('a');
                Ok(())
            })
            .with_priority(-1),
            Task::new(|state: &mut TestState| {
                state.results.push('b');
                Ok(())
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
        task.tick(&mut state).unwrap();

        assert_eq!(state.results, "cdba");
    }
}

