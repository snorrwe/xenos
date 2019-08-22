mod construction_state;
mod creep_state;
mod game_state;
mod sentinel;

pub use self::construction_state::*;
pub use self::sentinel::*;
pub use self::creep_state::*;
pub use self::game_state::*;

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

