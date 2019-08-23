mod construction_state;
mod creep_state;
mod game_state;
mod sentinel;

pub use self::construction_state::*;
pub use self::creep_state::*;
pub use self::game_state::*;
pub use self::sentinel::*;

use crate::bt::TaskInput;

pub struct WrappedState<'a, T, S: TaskInput> {
    pub item: T,
    pub state: &'a mut S,
}

impl<'a, T, S: TaskInput> TaskInput for WrappedState<'a, T, S> {
    fn cpu_bucket(&self) -> Option<i16> {
        self.state.cpu_bucket()
    }
}

impl<'a, T, S: TaskInput> WrappedState<'a, T, S> {
    pub fn new(item: T, game_state: &'a mut S) -> Self {
        Self {
            item,
            state: game_state,
        }
    }
}

