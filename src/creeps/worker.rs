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
    Building = 1,
    PickingUpEnergy = 2,
    WithdrawingEnergy = 3,
    Harvesting = 4,
    Repairing = 5,
}

pub fn run<'a>(state: &mut CreepState) -> ExecutionResult {
    let last_task = state.creep_memory_i64(TASK).unwrap_or(0);
    let last_task: WorkerState =
        WorkerState::from_u32(last_task as u32).unwrap_or(WorkerState::Idle);

    let mut priorities = [0; 6];
    priorities[last_task as usize] += 1;

    let mut tasks = [
        Task::new(|state| attempt_build(state))
            .with_name("Attempt build")
            .with_priority(priorities[WorkerState::Building as usize])
            .with_state_save(WorkerState::Building),
        Task::new(|state: &mut CreepState| withdraw_energy(state))
            .with_name("Withdraw energy")
            .with_state_save(WorkerState::WithdrawingEnergy)
            .with_priority(priorities[WorkerState::WithdrawingEnergy as usize]),
        Task::new(|state| harvest(state))
            .with_name("Harvest")
            .with_state_save(WorkerState::Harvesting)
            .with_priority(priorities[WorkerState::Harvesting as usize]),
        // If nothing can be built
        Task::new(|state: &mut CreepState| repairer::attempt_repair(state))
            .with_required_bucket(500)
            .with_priority(priorities[WorkerState::Repairing as usize])
            .with_state_save(WorkerState::Repairing)
            .with_name("Attempt repair"),
        Task::new(|state: &mut CreepState| {
            state.creep_memory_remove(TARGET);
            Err("continue")?
        })
        .with_name("Delete target"),
        Task::new(|state| upgrader::attempt_upgrade(state)).with_name("Attempt upgrade"),
    ];

    sorted_by_priority(&mut tasks);
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

