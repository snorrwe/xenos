//! Repair structures
//!
use super::super::bt::*;
use super::upgrader;
use super::{get_energy, harvester, move_to};
use screeps::{
    objects::{Creep, Structure},
    prelude::*,
    traits::TryFrom,
    ReturnCode,
};

pub fn run<'a>(creep: &'a Creep) -> ExecutionResult {
    trace!("Running repairer {}", creep.name());

    let tasks = vec![
        Task::new("repair_0", |_| attempt_repair(creep)),
        Task::new("get energy", |_| get_energy(creep)),
        Task::new("harvest", |_| harvest(creep)),
        Task::new("repair_1", |_| attempt_repair(creep)),
        // Fall back to upgrading
        Task::new("upgrade", |_| upgrader::run(creep)),
    ]
    .into_iter()
    .map(|t| Node::Task(t))
    .collect();

    let tree = Control::Sequence(tasks);
    tree.tick()
}

fn harvest<'a>(creep: &'a Creep) -> ExecutionResult {
    trace!("Harvesting");

    let loading: bool = creep.memory().bool("loading");
    if !loading {
        return Err(());
    }
    if creep.carry_total() == creep.carry_capacity() {
        creep.memory().set("loading", false);
        creep.memory().del("target");
        Ok(())
    } else {
        harvester::harvest(creep)
    }
}

fn attempt_repair<'a>(creep: &'a Creep) -> ExecutionResult {
    trace!("Repairing");

    let loading: bool = creep.memory().bool("loading");
    if loading {
        return Err(());
    }
    if creep.carry_total() == 0 {
        creep.memory().set("loading", true);
        Err(())
    } else {
        trace!("Repairing");
        let target = find_repair_target(creep).ok_or_else(|| {
            warn!("Could not find a repair target");
        })?;
        trace!("Got repair target {:?}", target.id());
        repair(creep, &target)
    }
}

fn repair<'a>(creep: &'a Creep, target: &'a Structure) -> ExecutionResult {
    let res = creep.repair(target);
    match res {
        ReturnCode::Ok => Ok(()),
        ReturnCode::NotInRange => move_to(creep, target),
        _ => {
            error!("Failed to repair target {:?} {:?}", res, target.id());
            Err(())
        }
    }
}

fn find_repair_target<'a>(creep: &'a Creep) -> Option<Structure> {
    trace!("Finding repair target");

    let room = creep.room();
    let result = js!{
        const candidates = @{room}.find(FIND_STRUCTURES, {
            filter: function (s) { return s.hits < s.hitsMax / 2; }
        });
        return candidates[0];
    };

    Structure::try_from(result).ok()
}
