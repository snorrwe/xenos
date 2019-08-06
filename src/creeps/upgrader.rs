//! Upgrade Controllers
//!
use super::{move_to, withdraw_energy, CreepState, LOADING};
use crate::prelude::*;
use screeps::{prelude::*, ReturnCode};

pub fn task<'a>(state: &'a CreepState) -> Task<'a, CreepState> {
    let tasks = [
        Task::new(move |state| attempt_upgrade(state)).with_name("Attempt upgrade"),
        Task::new(move |state| withdraw_energy(state)).with_name("Withdraw energy"),
        Task::new(move |state| attempt_upgrade(state)).with_name("Attempt upgrade"),
    ]
    .into_iter()
    .cloned()
    .collect();

    let tree = Control::Sequence(tasks);
    Task::from(tree).with_name("Upgrader")
}

pub fn attempt_upgrade<'a>(state: &mut CreepState) -> ExecutionResult {
    trace!("Upgrading");
    let loading = state.creep_memory_bool(LOADING);
    if loading.unwrap_or(false) {
        return Err("loading".into());
    }
    let creep = state.creep();
    if creep.carry_total() == 0 {
        state.creep_memory_set("loading".into(), true.into());
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

