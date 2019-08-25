//! Build structures
//!
use super::{
    harvest, move_to, repairer, upgrader, withdraw_energy, CreepState, LOADING, TARGET, TASK,
};
use crate::prelude::*;
use num::FromPrimitive;
use screeps::{
    constants::find, game::get_object_typed, objects::ConstructionSite, prelude::*, ReturnCode,
};

#[derive(Debug, Clone, Copy, FromPrimitive, ToPrimitive)]
#[repr(u8)]
enum WorkerState {
    Idle = 0,
    Building,
    PickingUpEnergy,
    WithdrawingEnergy,
    Harvesting,
    Repairing,
}

pub fn run<'a>(state: &mut CreepState) -> ExecutionResult {
    let last_task = state.creep_memory_i64(TASK).unwrap_or(0);
    let last_task: WorkerState =
        WorkerState::from_u32(last_task as u32).unwrap_or(WorkerState::Idle);

    let mut priorities = [0; 5];

    match last_task {
        WorkerState::Building => priorities[0] += 1,
        WorkerState::PickingUpEnergy => priorities[1] += 1,
        WorkerState::WithdrawingEnergy => priorities[2] += 1,
        WorkerState::Harvesting => priorities[3] += 1,
        WorkerState::Repairing => priorities[4] += 1,
        _ => {}
    }
    let tasks = [
        Task::new(|state| attempt_build(state))
            .with_name("Attempt build")
            .with_priority(priorities[0]),
        Task::new(|state: &mut CreepState| withdraw_energy(state))
            .with_name("Withdraw energy")
            .with_priority(priorities[2]),
        Task::new(|state| harvest(state))
            .with_name("Harvest")
            .with_priority(priorities[3]),
        // If nothing can be built
        Task::new(|state: &mut CreepState| repairer::attempt_repair(state))
            .with_required_bucket(500)
            .with_priority(priorities[4])
            .with_name("Attempt repair"),
        Task::new(|state: &mut CreepState| {
            state.creep_memory_remove(TARGET);
            Err("continue")?
        })
        .with_name("Delete target"),
        Task::new(|state| upgrader::attempt_upgrade(state)).with_name("Attempt upgrade"),
    ];

    sequence(state, tasks.iter())
}

pub fn attempt_build<'a>(state: &mut CreepState) -> ExecutionResult {
    trace!("Building");

    let loading = state.creep_memory_bool(LOADING);
    if loading.unwrap_or(false) {
        Err("loading")?;
    }

    if state.creep().carry_total() == 0 {
        state.creep_memory_set(LOADING.into(), true);
        Err("empty")?
    }
    let target = get_build_target(state).ok_or_else(|| format!("Failed to find build target"))?;
    let res = state.creep().build(&target);
    match res {
        ReturnCode::Ok => Ok(()),
        ReturnCode::NotInRange => move_to(state.creep(), &target),
        _ => {
            error!("Failed to build target {:?} {:?}", res, target.id());
            state.creep_memory_remove(TARGET);
            Err("Failed to build target")?
        }
    }
}

fn get_build_target<'a>(state: &mut CreepState) -> Option<ConstructionSite> {
    state
        .creep_memory_string(TARGET)
        .and_then(|id| get_object_typed(id).unwrap_or(None))
        .or_else(|| {
            let sites = state.creep().room().find(find::MY_CONSTRUCTION_SITES);
            sites
                .into_iter()
                .min_by_key(|s| s.progress_total() - s.progress())
                .ok_or_else(|| debug!("Could not find a build target"))
                .map(|site| {
                    state.creep_memory_set(TARGET.into(), site.id());
                    site
                })
                .ok()
        })
}

