//! Repair structures
//!
use super::super::bt::*;
use super::{builder, get_energy, harvest, upgrader};
use screeps::{objects::Creep, prelude::*, traits::TryFrom, ReturnCode};

pub fn run<'a>(creep: &'a Creep) -> Task<'a> {
    trace!("Running repairer {}", creep.name());

    let tasks = vec![
        Task::new(move |_| attempt_repair(creep)),
        Task::new(move |_| get_energy(creep)),
        Task::new(move |_| harvest(creep)),
        Task::new(move |_| attempt_repair(creep)),
        // Fall back
        Task::new(move |_| builder::attempt_build(creep)),
        Task::new(move |_| upgrader::attempt_upgrade(creep)),
    ];

    let tree = Control::Sequence(tasks);
    Task::new(move |_| tree.tick())
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
        trace!("Got repair target {:?}", target);
        repair(creep, &target)
    }
}

// TODO: return Structure once the Structure bug has been fixed in screeps api
fn repair<'a>(creep: &'a Creep, target: &'a String) -> ExecutionResult {
    let res = js! {
        const creep = @{creep};
        let target = @{target};
        target = Game.getObjectById(target);
        let result = creep.repair(target);
        if (result == ERR_NOT_IN_RANGE) {
            result = creep.moveTo(target);
        }
        return result;
    };
    let res = ReturnCode::try_from(res).map_err(|e| {
        let error = format!("Expected ReturnCode {:?}", e);
        error!("{}", error);
        error
    })?;
    if res == ReturnCode::Ok {
        Ok(())
    } else {
        let error = format!("Unexpected ReturnCode {:?}", res);
        Err(error)
    }
}

// TODO: return Structure once the Structure bug has been fixed in screeps api
fn find_repair_target<'a>(creep: &'a Creep) -> Option<String> {
    trace!("Finding repair target");

    let room = creep.room();
    let result = js! {
        const room = @{room};
        const candidates = room.find(FIND_STRUCTURES, {
            filter: function (s) { return s.hits < s.hitsMax; }
        });
        return candidates[0] && candidates[0].id;
    };

    String::try_from(result).ok()
}

