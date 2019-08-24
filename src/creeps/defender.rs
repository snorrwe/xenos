//! Basic creep to attack enemy creeps in rooms
//!
use super::{move_to, CreepState};
use crate::prelude::*;
use screeps::{find, game::get_object_typed, prelude::*, Creep, ReturnCode};

const ATTACK_TARGET: &'static str = "attack_target";

pub fn run<'a>(state: &mut CreepState) -> ExecutionResult {
    let tasks = [
        Task::new(|state| attack_simple(state)),
        Task::new(|state| {
            state.creep().say("⚔️", true);
            Ok(())
        }),
    ];

    selector(state, tasks.iter())
}

pub fn attack_simple(state: &mut CreepState) -> ExecutionResult {
    if let Some(ref target) = find_target(state) {
        let result = state.creep().attack(target);
        match result {
            ReturnCode::Ok => return Ok(()),
            ReturnCode::NotInRange => return move_to(state.creep(), target),
            _ => {
                warn!(
                    "Creep {} failed to attack {} {:?}",
                    state.creep_name().0,
                    target.name(),
                    result
                );
                Err("Failed to attack")?;
            }
        }
    }
    Err("Can't find target to attack")?
}

fn find_target(state: &mut CreepState) -> Option<Creep> {
    if let Some(id) = state.creep_memory_string(ATTACK_TARGET) {
        if let Ok(Some(creep)) = get_object_typed::<Creep>(&id) {
            return Some(creep);
        }
    }
    state
        .creep()
        .pos()
        .find_closest_by_range(find::HOSTILE_CREEPS)
        .map(|creep| {
            state.creep_memory_set(ATTACK_TARGET, creep.id());
            creep
        })
}

