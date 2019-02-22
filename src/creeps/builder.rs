//! Build structures
//!
use super::super::bt::*;
use super::{get_energy, harvest, move_to, repairer, upgrader};
use screeps::{constants::find, objects::Creep, prelude::*, ReturnCode};

pub fn run<'a>(creep: &'a Creep) -> Task<'a> {
    trace!("Running builder {}", creep.name());
    let tasks = vec![
        Task::new(move |_| attempt_build(creep)),
        Task::new(move |state| get_energy(&state, creep)),
        Task::new(move |_| harvest(creep)),
        Task::new(move |_| attempt_build(creep)),
        // If nothing can be built
        Task::new(move |_| repairer::attempt_repair(creep)),
        Task::new(move |_| upgrader::attempt_upgrade(creep)),
    ];

    let tree = Control::Sequence(tasks);
    Task::new(move |state| tree.tick(&state))
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
        let target = creep
            .pos()
            .find_closest_by_range(find::MY_CONSTRUCTION_SITES)
            .ok_or_else(|| String::from("Could not find a build target"))?;
        let res = creep.build(&target);
        match res {
            ReturnCode::Ok => Ok(()),
            ReturnCode::NotInRange => move_to(creep, &target),
            _ => {
                let error = format!("Failed to build target {:?} {:?}", res, target.id());
                error!("{}", error);
                Err(error)
            }
        }
    }
}
