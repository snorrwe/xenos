//! Build structures
//!
use super::super::bt::*;
use super::{get_energy, harvest, move_to, repairer, upgrader};
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
        Task::new(move |_| repairer::attempt_repair(creep)).with_required_bucket(500),
        Task::new(move |state| {
            state.creep_memory_entry(creep.name()).remove("target");
            Err("continue")?
        }),
        Task::new(move |state| upgrader::attempt_upgrade(state, creep)),
    ];

    let tree = Control::Sequence(tasks);
    Task::new(move |state| tree.tick(state))
}

pub fn attempt_build<'a>(state: &mut GameState, creep: &'a Creep) -> ExecutionResult {
    trace!("Building");

    {
        let loading = state.creep_memory_bool("loading".to_string(), &creep.name());
        if loading {
            Err("loading")?;
        }
        let memory = state.creep_memory_entry(creep.name());
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
            let memory = state.creep_memory_entry(creep.name());
            memory.remove("target");
            Err(error)
        }
    }
}

fn get_build_target(state: &mut GameState, creep: &Creep) -> Option<ConstructionSite> {
    let memory = state.creep_memory_entry(creep.name());
    memory
        .get("target")?
        .as_str()
        .iter()
        .find_map(|id| get_object_typed(id).unwrap_or(None))
        .or_else(|| {
            creep
                .pos()
                .find_closest_by_range(find::MY_CONSTRUCTION_SITES)
                .ok_or_else(|| debug!("Could not find a build target"))
                .map(|site| {
                    memory.insert("target".into(), site.id().into());
                    site
                })
                .ok()
        })
}

