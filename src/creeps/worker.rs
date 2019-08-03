//! Build structures
//!
use super::{harvest, move_to, repairer, upgrader, withdraw_energy, TARGET, TASK};
use crate::prelude::*;
use num::FromPrimitive;
use screeps::{
    constants::find,
    game::get_object_typed,
    objects::{ConstructionSite, Creep},
    prelude::*,
    ReturnCode,
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

pub fn run<'a>(creep: &'a Creep) -> Task<'a, GameState> {
    trace!("Running builder {}", creep.name());

    Task::new(move |state| {
        let task = prepare_task(creep, state);
        task.tick(state)
    })
    .with_name("Worker")
}

fn prepare_task<'a>(creep: &'a Creep, state: &GameState) -> Task<'a, GameState> {
    let name = creep.name();
    let last_task = state
        .creep_memory_i64(CreepName(name.as_str()), TASK)
        .unwrap_or(0);
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
        Task::new(move |state| attempt_build(state, creep))
            .with_name("Attempt build")
            .with_priority(priorities[0]),
        Task::new(move |state| withdraw_energy(state, creep))
            .with_name("Withdraw energy")
            .with_priority(priorities[2]),
        Task::new(move |state| harvest(state, creep))
            .with_name("Harvest")
            .with_priority(priorities[3]),
        // If nothing can be built
        Task::new(move |state| {
            if creep
                .room()
                .controller()
                .map(|c| c.level() >= 3)
                .unwrap_or(false)
            {
                repairer::attempt_repair(state, creep)
            } else {
                Err("Skip repairing until the controller is level 3")?
            }
        })
        .with_required_bucket(500)
        .with_priority(priorities[4])
        .with_name("Attempt repair"),
        Task::new(move |state: &mut GameState| {
            state
                .creep_memory_entry(CreepName(&creep.name()))
                .remove(TARGET);
            Err("continue")?
        })
        .with_name("Delete target"),
        Task::new(move |state| upgrader::attempt_upgrade(state, creep))
            .with_name("Attempt upgrade"),
    ]
    .into_iter()
    .cloned()
    .collect();
    let tree = Control::Sequence(tasks);
    tree.sorted_by_priority().into()
}

pub fn attempt_build<'a>(state: &mut GameState, creep: &'a Creep) -> ExecutionResult {
    trace!("Building");

    let name = creep.name();
    {
        let loading = state.creep_memory_bool(CreepName(&name), "loading");
        if loading {
            Err("loading")?;
        }
        let memory = state.creep_memory_entry(CreepName(&name));
        if creep.carry_total() == 0 {
            memory.insert("loading".into(), true.into());
            Err("empty")?
        }
    }
    let target =
        get_build_target(state, creep).ok_or_else(|| format!("Failed to find build target"))?;
    let res = creep.build(&target);
    match res {
        ReturnCode::Ok => Ok(()),
        ReturnCode::NotInRange => move_to(creep, &target),
        _ => {
            let error = format!("Failed to build target {:?} {:?}", res, target.id());
            error!("{}", error);
            let memory = state.creep_memory_entry(CreepName(&name));
            memory.remove(TARGET);
            Err(error)
        }
    }
}

fn get_build_target(state: &mut GameState, creep: &Creep) -> Option<ConstructionSite> {
    state
        .creep_memory_string(CreepName(&creep.name()), TARGET)
        .and_then(|id| get_object_typed(id).unwrap_or(None))
        .or_else(|| {
            let sites = creep.room().find(find::MY_CONSTRUCTION_SITES);
            sites
                .into_iter()
                .min_by_key(|s| s.progress_total() - s.progress())
                .ok_or_else(|| debug!("Could not find a build target"))
                .map(|site| {
                    let memory = state.creep_memory_entry(CreepName(&creep.name()));
                    memory.insert(TARGET.into(), site.id().into());
                    site
                })
                .ok()
        })
}

