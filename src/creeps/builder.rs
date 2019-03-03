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
        Task::new(move |_| attempt_build(creep)),
        Task::new(move |state| get_energy(state, creep)),
        Task::new(move |_| harvest(creep)),
        Task::new(move |_| attempt_build(creep)),
        // If nothing can be built
        Task::new(move |_| repairer::attempt_repair(creep)),
        Task::new(move |_| {
            creep.memory().del("target");
            Err("continue".into())
        }),
        Task::new(move |_| upgrader::attempt_upgrade(creep)),
    ];

    let tree = Control::Sequence(tasks);
    Task::new(move |state| tree.tick(state))
}

pub fn attempt_build<'a>(creep: &'a Creep) -> ExecutionResult {
    trace!("Building");

    let loading: bool = creep.memory().bool("loading");
    if loading {
        return Err("loading".into());
    }
    if creep.carry_total() == 0 {
        creep.memory().set("loading", true);
        Err("empty".into())
    } else {
        let target =
            get_build_target(creep).ok_or_else(|| format!("Failed to find build target"))?;
        let res = creep.build(&target);
        match res {
            ReturnCode::Ok => Ok(()),
            ReturnCode::NotInRange => move_to(creep, &target),
            _ => {
                let error = format!("Failed to build target {:?} {:?}", res, target.id());
                error!("{}", error);
                creep.memory().del("target");
                Err(error)
            }
        }
    }
}

fn get_build_target(creep: &Creep) -> Option<ConstructionSite> {
    let target = creep
        .memory()
        .string("target")
        .map_err(|e| {
            error!("Failed to read creep target {:?}", e);
            creep.memory().del("target");
        })
        .ok()?
        .map(|id| get_object_typed(id.as_str()))
        .ok_or_else(|| {
            creep
                .pos()
                .find_closest_by_range(find::MY_CONSTRUCTION_SITES)
                .ok_or_else(|| debug!("Could not find a build target"))
                .map(|site| {
                    creep.memory().set("target", site.id());
                    site
                })
        })
        .ok()?;
    target.ok()?
}

