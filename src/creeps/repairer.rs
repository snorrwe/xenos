//! Repair structures
//!
use super::super::bt::*;
use super::{builder, get_energy, move_to, upgrader};
use screeps::{
    constants::find,
    objects::{Creep, Structure},
    prelude::*,
    ReturnCode,
};

pub fn run<'a>(creep: &'a Creep) -> Task<'a> {
    trace!("Running repairer {}", creep.name());

    let tasks = vec![
        Task::new(move |_| attempt_repair(creep)),
        Task::new(move |state| get_energy(state, creep)),
        // Fall back
        Task::new(move |_| builder::attempt_build(creep)),
        Task::new(move |_| upgrader::attempt_upgrade(creep)),
    ];

    let tree = Control::Sequence(tasks);
    Task::new(move |state| tree.tick(state))
}

pub fn attempt_repair<'a>(creep: &'a Creep) -> ExecutionResult {
    trace!("Repairing");

    let loading: bool = creep.memory().bool("loading");
    if loading {
        return Err("loading".into());
    }
    if creep.carry_total() == 0 {
        creep.memory().set("loading", true);
        Err("empty".into())
    } else {
        trace!("Repairing");
        let target = find_repair_target(creep).ok_or_else(|| {
            let error = format!("Could not find a repair target");
            debug!("{}", error);
            error
        })?;
        repair(creep, &target)
    }
}

fn repair<'a>(creep: &'a Creep, target: &'a Structure) -> ExecutionResult {
    let res = creep.repair(target);
    match res {
        ReturnCode::Ok => Ok(()),
        ReturnCode::NotInRange => move_to(creep, target),
        _ => Err(format!("Unexpected ReturnCode {:?}", res)),
    }
}

fn find_repair_target<'a>(creep: &'a Creep) -> Option<Structure> {
    trace!("Finding repair target");

    let room = creep.room();
    room.find(find::STRUCTURES).into_iter().find(|s| {
        s.as_attackable()
            .map(|s| s.hits() < s.hits_max())
            .unwrap_or(false)
    })
}

