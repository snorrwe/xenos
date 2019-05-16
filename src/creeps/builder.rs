//! Build structures
//!
use super::{get_energy, harvest, move_to, repairer, upgrader, TARGET};
use crate::prelude::*;
use screeps::{
    constants::find,
    game::get_object_typed,
    objects::{ConstructionSite, Creep},
    prelude::*,
    ReturnCode,
};

pub fn run<'a>(creep: &'a Creep) -> Task<'a> {
    trace!("Running builder {}", creep.name());
    let tasks = vec![
        Task::new(move |state| attempt_build(state, creep)),
        Task::new(move |state| get_energy(state, creep)),
        Task::new(move |state| harvest(state, creep)),
        Task::new(move |state| attempt_build(state, creep)),
        // If nothing can be built
        Task::new(move |state| repairer::attempt_repair(state, creep)).with_required_bucket(500),
        Task::new(move |state| {
            state
                .creep_memory_entry(CreepName(&creep.name()))
                .remove(TARGET);
            Err("continue")?
        }),
        Task::new(move |state| upgrader::attempt_upgrade(state, creep)),
    ];

    let tree = Control::Sequence(tasks);
    Task::new(move |state| tree.tick(state))
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
            creep
                .pos()
                .find_closest_by_range(find::MY_CONSTRUCTION_SITES)
                .ok_or_else(|| debug!("Could not find a build target"))
                .map(|site| {
                    let memory = state.creep_memory_entry(CreepName(&creep.name()));
                    memory.insert(TARGET.into(), site.id().into());
                    site
                })
                .ok()
        })
}
