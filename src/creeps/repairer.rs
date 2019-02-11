//! Repair structures
//!
use super::super::bt::*;
use super::{get_energy, harvester, move_to};
use screeps::{
    constants::find,
    objects::{Attackable, Creep, Structure},
    prelude::*,
    traits::TryFrom,
    ReturnCode,
};

pub fn run<'a>(creep: &'a Creep) -> ExecutionResult {
    trace!("Running repairer {}", creep.name());
    let tasks = vec![
        Task::new("repair_0", || repair(creep)),
        Task::new("get energy", || get_energy(creep)),
        Task::new("harvest", || harvest(creep)),
        Task::new("repair_1", || repair(creep)),
    ]
    .into_iter()
    .map(|t| Node::Task(t))
    .collect();

    let tree = BehaviourTree::new(Control::Sequence(tasks));
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

fn repair<'a>(creep: &'a Creep) -> ExecutionResult {
    trace!("Building");

    let loading: bool = creep.memory().bool("loading");
    if loading {
        return Err(());
    }
    if creep.carry_total() == 0 {
        creep.memory().set("loading", true);
        Err(())
    } else {
        let target = find_repair_target(creep).ok_or_else(|| {
            warn!("Could not find a repair target");
        })?;
        let res = js!{
            const id = @{target.id()};
            const target = Game.getObjectById(id);
            return creep.repair(target);
        };
        let res = ReturnCode::try_from(res).map_err(|e| {
            error!("Expected return code {:?}", e);
        })?;
        match res {
            ReturnCode::Ok => Ok(()),
            ReturnCode::NotInRange => move_to(creep, &target),
            _ => {
                error!("Failed to repair target {:?} {:?}", res, target.id());
                Err(())
            }
        }
    }
}

fn find_repair_target<'a>(creep: &'a Creep) -> Option<Structure> {
    creep
        .room()
        .find(find::STRUCTURES)
        .into_iter()
        .find(|s| s.hits() < s.hits_max() / 2)
}

