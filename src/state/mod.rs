pub mod construction_state;
pub mod creep_state;
pub mod game_state;
pub mod sentinel;

pub use self::sentinel::*;

use crate::prelude::*;
use num::ToPrimitive;

pub trait WithStateSave<'a> {
    type State: TaskInput;

    fn with_state_save<T: ToPrimitive + 'a>(
        self,
        creep: String,
        task_id: T,
    ) -> Task<'a, Self::State>;
}

