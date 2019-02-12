//! Repair structures
//!
use super::super::bt::*;
use super::{builder, upgrader};
use super::{get_energy, harvester};
use screeps::{objects::Creep, prelude::*, traits::TryFrom, ReturnCode};

pub fn run<'a>(creep: &'a Creep) -> ExecutionResult {
    trace!("Running repairer {}", creep.name());

    let tasks = vec![
        Task::new("repair_0", |_| attempt_repair(creep)),
        Task::new("get energy", |_| get_energy(creep)),
        Task::new("harvest", |_| harvest(creep)),
        Task::new("repair_1", |_| attempt_repair(creep)),
        // Fall back to upgrading
        Task::new("build", |_| builder::attempt_build(creep)),
        Task::new("upgrade", |_| upgrader::attempt_upgrade(creep)),
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
        harvester::attempt_harvest(creep)
    }
}

pub fn attempt_repair<'a>(creep: &'a Creep) -> ExecutionResult {
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
        trace!("Got repair target {:?}", target);
        repair(creep, &target)
    }
}

// TODO: return Structure once the Structure bug has been fixed in screeps api
fn repair<'a>(creep: &'a Creep, target: &'a String) -> ExecutionResult {
    let res = js!{
        const creep = @{creep};
        let target = @{target};
        target = Game.getObjectById(target);
        let result = creep.repair(target);
        if (result == ERR_NOT_IN_RANGE) {
            result = creep.moveTo(target);
        }
        return result;
    };
    let res = ReturnCode::try_from(res).map_err(|e| error!("Expected ReturnCode {:?}", e))?;
    if res == ReturnCode::Ok {
        Ok(())
    } else {
        Err(())
    }
}

// TODO: return Structure once the Structure bug has been fixed in screeps api
fn find_repair_target<'a>(creep: &'a Creep) -> Option<String> {
    trace!("Finding repair target");

    let room = creep.room();
    let result = js!{
        const room = @{room};
        const candidates = room.find(FIND_STRUCTURES, {
            filter: function (s) { return s.hits < s.hitsMax / 2; }
        });
        return candidates[0] && candidates[0].id;
    };

    String::try_from(result).ok()
}
