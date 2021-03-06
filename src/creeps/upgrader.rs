//! Upgrade Controllers
//!
use super::{move_to, sign_controller_stock_msgs, withdraw_energy, CreepState, LOADING};
use crate::prelude::*;
use screeps::{prelude::*, ReturnCode};

pub fn run<'a>(state: &mut CreepState) -> ExecutionResult {
    let tasks = [
        Task::new(|state| {
            let tasks = [
                Task::new(|state| attempt_upgrade(state)),
                Task::new(|state: &mut CreepState| sign_controller_stock_msgs(state.creep())),
            ];

            selector(state, tasks.iter())
        })
        .with_name("Attempt upgrade"),
        Task::new(|state| withdraw_energy(state)).with_name("Withdraw energy"),
        Task::new(|state| attempt_upgrade(state)).with_name("Attempt upgrade"),
    ];

    sequence(state, tasks.iter())
}

pub fn attempt_upgrade<'a>(state: &mut CreepState) -> ExecutionResult {
    let loading = state.creep_memory_bool(LOADING);
    if loading.unwrap_or(false) {
        return Err("loading")?;
    }
    if state.creep().carry_total() == 0 {
        state.creep_memory_set("loading".into(), true);
        Err("empty")?;
    }
    let controller = state.creep().room().controller().ok_or_else(|| {
        let error = "Creep has no access to a controller in the room!";
        error!("{}", error);
        error
    })?;
    let res = state.creep().upgrade_controller(&controller);
    match res {
        ReturnCode::Ok => Ok(()),
        ReturnCode::NotInRange => move_to(state.creep(), &controller),
        _ => {
            error!("Failed to upgrade controller {:?}", res);
            Err("Failed to upgrade controller")?
        }
    }
}

