use super::*;
use std::rc::Rc;

#[derive(Clone)]
pub enum Change<T> {
    Delete(TaskEntity),
    Add(ExecutionUnit<T>),
    Update(TaskEntity, ExecutionUnit<T>),
}

#[derive(Clone)]
enum ExecutionUnitInner<T> {
    Task(Task<T>),
    Selector(TaskCollection<T>),
    Sequence(TaskCollection<T>),
    All(TaskCollection<T>),
}

struct ExecutionUnit<T> {
    inner: ExecutionUnitInner<T>,
    id: TaskEntity,
}

#[derive(Clone)]
pub struct TaskGraph<Aux> {
    state: Rc<Aux>,
    tasks: Vec<ExecutionUnit<Self>>,
    pre_processors: Vec<TaskCollection<Aux>>,
}

impl<Aux> TaskGraph<Aux> {
    /// Return the number of tasks in this executor
    pub fn len(&self) -> usize {
        unimplemented!()
    }

    pub fn new(state: Rc<Aux>, pre_processors: TaskCollection<Aux>) -> Self {
        unimplemented!()
    }

    pub fn with_unit(mut self, unit: ExecutionUnit<Self>) -> Self {
        unimplemented!()
    }

    pub fn get_state(&self) -> &Aux {
        &self.state
    }

    pub fn mut_state(&mut self) -> &mut Aux {
        &mut self.state
    }

    pub fn pre_process(&mut self) {
        unimplemented!()
    }

    pub fn tick(&mut self) -> ExecutionResult {
        unimplemented!()
    }

    pub fn post_process(&mut self) {
        unimplemented!()
    }
}

