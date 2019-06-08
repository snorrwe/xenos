//! Upgrade Controllers
//!
use super::{get_energy, move_to};
use crate::game_state::GameState;
use crate::prelude::*;
use screeps::{objects::Creep, prelude::*, ReturnCode};

pub fn run<'a>(creep: &'a Creep) -> Task<'a, GameState> {
    trace!("Running upgrader {}", creep.name());

    let tasks = [
        Task::new(move |state| attempt_upgrade(state, creep)),
        Task::new(move |state| get_energy(state, creep)),
        Task::new(move |state| attempt_upgrade(state, creep)),
    ]
    .into_iter()
    .cloned()
    .collect();

    let tree = Control::Sequence(tasks);
    Task::new(move |state| tree.tick(state))
}

pub fn attempt_upgrade<'a>(state: &mut GameState, creep: &'a Creep) -> ExecutionResult {
    trace!("Upgrading");
    let loading = state.creep_memory_bool(CreepName(&creep.name()), "loading");
    if loading {
        return Err("loading".into());
    }
    let memory = state.creep_memory_entry(CreepName(&creep.name()));
    if creep.carry_total() == 0 {
        memory.insert("loading".into(), true.into());
        Err("empty".to_string())?;
    }
    let controller = creep.room().controller().ok_or_else(|| {
        let error = format!("Creep has no access to a controller in the room!");
        error!("{}", error);
        error
    })?;
    let res = creep.upgrade_controller(&controller);
    match res {
        ReturnCode::Ok => Ok(()),
        ReturnCode::NotInRange => move_to(creep, &controller),
        _ => {
            let error = format!("Failed to upgrade controller {:?}", res);
            error!("{}", error);
            Err(error)
        }
    }
}

