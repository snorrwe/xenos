//! Repair structures
//!
use super::super::bt::*;
use super::{find_repair_target, move_to};
use screeps::{
    objects::{Creep, RoomObjectProperties, Structure},
    ReturnCode,
};

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
        let target = find_repair_target(&creep.room()).ok_or_else(|| {
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

