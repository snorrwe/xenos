//! Long Range Worker
//! Used to work on other rooms
//!
use super::{approach_target_room, update_scout_info, worker};
use crate::prelude::*;
use screeps::{game, objects::Creep};

const TARGET_ROOM: &'static str = "target_room";

pub fn run<'a>(creep: &'a Creep) -> Task<'a, GameState> {
    trace!("Running long range worker {}", creep.name());
    let tasks = [
        Task::new(move |state| {
            update_scout_info(state, creep).unwrap_or_else(|e| {
                error!("Failed to update scout info {:?}", e);
            });
            Err("continue")?
        })
        .with_name("Update scout info"),
        Task::new(move |state| set_target(state, creep)).with_name("Set target"),
        Task::new(move |state| approach_target_room(state, creep, TARGET_ROOM)),
        worker::run(creep),
    ]
    .into_iter()
    .cloned()
    .collect();

    let tree = Control::Sequence(tasks);
    Task::new(move |state| tree.tick(state))
        .with_required_bucket(300)
        .with_name("LRW")
}

fn set_target<'a>(state: &mut GameState, creep: &'a Creep) -> ExecutionResult {
    trace!("finding target");

    if state
        .creep_memory_string(CreepName(&creep.name()), TARGET_ROOM)
        .is_some()
    {
        trace!("has target");
        return Err(String::from("Creep already has a target"));
    }
    let flags = game::flags::values();
    let flag = flags
        .iter()
        .next()
        .ok_or_else(|| String::from("can't find a flag"))?;

    let memory = state.creep_memory_entry(CreepName(&creep.name()));
    memory.insert(TARGET_ROOM.into(), flag.name().into());
    debug!("set target to {}", flag.name());

    Ok(())
}

