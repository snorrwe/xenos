//! Upgrade Controllers
//!
use super::super::bt::*;
use super::{get_energy, move_to};
use crate::game_state::GameState;
use screeps::{objects::Creep, prelude::*, ReturnCode};

pub fn run<'a>(creep: &'a Creep) -> Task<'a> {
    trace!("Running upgrader {}", creep.name());

    let tasks = vec![
        Task::new(move |state| attempt_upgrade(state, creep)),
        Task::new(move |state| get_energy(state, creep)),
        Task::new(move |state| attempt_upgrade(state, creep)),
    ];

    let tree = Control::Sequence(tasks);
    Task::new(move |state| tree.tick(state))
}

pub fn attempt_upgrade<'a>(state: &mut GameState, creep: &'a Creep) -> ExecutionResult {
    trace!("Upgrading");
    let memory = state.creep_memory_entry(creep.name());
    let loading: bool = memory
        .get("loading")
        .and_then(|x| x.as_bool())
        .unwrap_or(false);
    if loading {
        return Err("loading".into());
    }
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

