use crate::creeps::{Role, TASK};
use crate::prelude::*;
use num::ToPrimitive;
use screeps::Creep;
use screeps::RoomObjectProperties;
use serde_json::Value;
use std::fmt::{Debug, Formatter};

pub struct CreepState {
    creep: Creep,
    creep_name: String,
    world_position: WorldPosition,
    memory: *mut CreepMemoryEntry,
    game_state: *mut GameState,
}

impl Clone for CreepState {
    fn clone(&self) -> Self {
        panic!("Do not clone CreepState objects, the trait impl is provided so the Tasks are cloneable");
    }
}

impl CreepState {
    /// Contract: GameState must live for the lifetime of this object
    pub fn new(creep: Creep, game_state: &mut GameState) -> Self {
        let creep_name = creep.name();
        let memory = game_state.creep_memory_entry(CreepName(creep_name.as_str())) as *mut _;
        let game_state = game_state as *mut _;
        Self {
            world_position: WorldPosition::from(creep.room()),
            creep,
            creep_name,
            game_state,
            memory,
        }
    }

    pub fn current_room(&self) -> WorldPosition {
        self.world_position
    }

    pub fn creep_memory_remove(&mut self, key: &str) {
        let memory = unsafe { &mut *self.memory };
        memory.remove(key);
    }

    pub fn creep_memory_set<T: Into<Value>>(&mut self, key: &str, value: T) {
        let val: Value = value.into();
        let memory = unsafe { &mut *self.memory };
        memory.insert(key.to_owned(), val);
    }

    #[allow(unused)]
    pub fn creep_memory_mut<'a, T>(&'a mut self, key: &str) -> Option<&'a mut T>
    where
        &'a mut T: From<&'a mut Value>,
    {
        let memory = unsafe { &mut *self.memory };
        memory.get_mut(key).map(|value| value.into())
    }

    #[allow(unused)]
    pub fn creep_memory_get<'a, T>(&'a self, key: &str) -> Option<&'a T>
    where
        &'a T: From<&'a Value>,
    {
        let memory = unsafe { &*self.memory };
        memory.get(key).map(|value| value.into())
    }

    pub fn get_game_state<'a>(&'a self) -> &'a GameState {
        unsafe { &*self.game_state }
    }

    pub fn mut_game_state<'a>(&'a mut self) -> *mut GameState {
        self.game_state
    }

    pub fn creep_name<'a>(&'a self) -> CreepName<'a> {
        CreepName(self.creep_name.as_str())
    }

    pub fn creep<'a>(&'a self) -> &'a Creep {
        &self.creep
    }

    pub fn creep_memory_role(&self, key: &str) -> Option<Role> {
        let memory = unsafe { &mut *self.memory };
        memory
            .get(key)
            .and_then(|value| value.as_i64())
            .map(|x: i64| Role::from(x as u8))
    }

    pub fn creep_memory_string<'a>(&'a self, key: &str) -> Option<&'a str> {
        let memory = unsafe { &mut *self.memory };
        memory.get(key).and_then(|value| value.as_str())
    }

    pub fn creep_memory_bool(&self, key: &str) -> Option<bool> {
        let memory = unsafe { &mut *self.memory };
        memory.get(key).and_then(|value| value.as_bool())
    }

    pub fn creep_memory_i64(&self, key: &str) -> Option<i64> {
        let memory = unsafe { &mut *self.memory };
        memory.get(key).and_then(|value| value.as_i64())
    }
}

impl TaskInput for CreepState {
    fn cpu_bucket(&self) -> Option<i16> {
        self.get_game_state().cpu_bucket()
    }
}

impl Debug for CreepState {
    fn fmt(&self, f: &mut Formatter) -> std::fmt::Result {
        write!(f, "CreepState for creep {}", self.creep_name)?;
        Ok(())
    }
}

pub trait WithStateSave<'a> {
    type State: TaskInput;

    fn with_state_save<T: ToPrimitive + 'a>(self, task_id: T) -> Task<'a, Self::State>;
}

impl<'a> WithStateSave<'a> for Task<'a, CreepState> {
    type State = CreepState;

    fn with_state_save<T: 'a + ToPrimitive>(self, task_id: T) -> Task<'a, CreepState> {
        let tasks = [
            self,
            Task::new(move |state: &mut CreepState| {
                state.creep_memory_set(TASK, task_id.to_u32().unwrap_or(0) as i32);
                Ok(())
            }),
        ]
        .into_iter()
        .cloned()
        .collect();

        // Only save the state if the task succeeded
        let selector = Control::Selector(tasks);
        selector.into()
    }
}

