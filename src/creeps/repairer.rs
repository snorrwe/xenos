//! Repair structures
//!
use super::{find_repair_target, move_to, CreepState, LOADING};
use crate::prelude::*;
use screeps::{
    objects::{Creep, RoomObjectProperties, Structure},
    ReturnCode,
};

pub fn attempt_repair<'a>(state: &mut CreepState) -> ExecutionResult {
    let loading = state.creep_memory_bool(LOADING);
    if loading.unwrap_or(false) {
        return Err("loading".into());
    }
    let creep = state.creep();
    if creep.carry_total() == 0 {
        state.creep_memory_set("loading".into(), true);
        Err("empty".into())
    } else {
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
        _ => Err(format!("Unexpected ReturnCode {:?}", res))?,
    }
}

