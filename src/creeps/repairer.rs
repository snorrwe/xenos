//! Repair structures
//!
use super::super::bt::*;
use super::{builder, get_energy, upgrader};
use screeps::{
    game::get_object_erased,
    objects::{Creep, Structure},
    prelude::*,
    ReturnCode,
};
use stdweb::unstable::TryFrom;

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
        let target = validate_creep_target_as_repair_target(creep)
            .map(|target| {
                creep.memory().set("target", target.id());
                target
            })
            .or_else(|| find_repair_target(creep))
            .ok_or_else(|| {
                let error = format!("Could not find a repair target");
                debug!("{}", error);
                error
            })?;
        trace!("Got repair target {:?}", target.id());
        repair(creep, &target)
    }
}

fn validate_creep_target_as_repair_target<'a>(creep: &'a Creep) -> Option<Structure> {
    let target = creep.memory().string("target").ok()??;
    let target = get_object_erased(target.as_str())?;

    let target = Structure::try_from(target.as_ref().clone())
        .map_err(|e| {
            debug!("failed to read exisitng target, {:?}", e);
        })
        .ok()?;

    let repairable = target
        .as_attackable()
        .map(|s| s.hits() < s.hits_max())
        .ok_or_else(|| {
            error!("Expected target to be attackable {:?}", target.id());
            creep.memory().del("target");
        })
        .ok()?;
    if repairable {
        Some(target)
    } else {
        None
    }
}

fn repair<'a>(creep: &'a Creep, target: &'a Structure) -> ExecutionResult {
    let res = js! {
        const creep = @{creep};
        let target = @{target.as_ref()};
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
        Err(format!("Unexpected ReturnCode {:?}", res))
    }
}

fn find_repair_target<'a>(creep: &'a Creep) -> Option<Structure> {
    trace!("Finding repair target");

    let room = creep.room();
    let result = js! {
        const room = @{room};
        const candidates = room.find(FIND_STRUCTURES, {
            filter: function (s) { return s.hits < s.hitsMax; }
        });
        return candidates[0];
    };

    Structure::try_from(result).ok()
}
