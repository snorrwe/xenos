//! Long Range Worker
//! Used to work on other rooms
//!
use super::{move_to, update_scout_info, worker};
use crate::prelude::*;
use screeps::{game, objects::Creep, prelude::*};
use stdweb::unstable::TryInto;

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
        Task::new(move |state| approach_target_room(state, creep))
            .with_name("Approach target room"),
        Task::new(move |state| set_target(state, creep)).with_name("Set target"),
        worker::run(creep).with_name("Worker run"),
    ]
    .into_iter()
    .cloned()
    .collect();

    let tree = Control::Sequence(tasks);
    Task::new(move |state| tree.tick(state))
        .with_required_bucket(300)
        .with_name("LRW")
}

fn approach_target_room(state: &mut GameState, creep: &Creep) -> ExecutionResult {
    let memory = state.creep_memory_entry(CreepName(&creep.name()));
    let flag = {
        let flag = memory
            .get(TARGET_ROOM)
            .and_then(|x| x.as_str())
            .ok_or_else(|| "no target set")?;
        game::flags::get(flag)
    };

    let flag = flag.ok_or_else(|| {
        memory.remove(TARGET_ROOM);
        "target flag does not exist"
    })?;

    let room = creep.room();

    // The Rust Screeps api may panic here
    let arrived = js! {
        const flag = @{&flag};
        return @{&room}.name == (flag.room && flag.room.name) || false;
    };

    let arrived: bool = arrived
        .try_into()
        .map_err(|e| format!("failed to convert result to bool {:?}", e))?;
    if arrived {
        Err("Already in the room")?;
    }
    trace!("approaching target room");
    move_to(creep, &flag)
}

fn set_target<'a>(state: &mut GameState, creep: &'a Creep) -> ExecutionResult {
    trace!("finding target");

    if state
        .creep_memory_string(CreepName(&creep.name()), TARGET_ROOM)
        .is_some()
    {
        trace!("has target");
        Err("Creep already has a target")?;
    }
    let flags = game::flags::values();
    let flag = flags.iter().next().ok_or_else(|| "can't find a flag")?;

    let memory = state.creep_memory_entry(CreepName(&creep.name()));
    memory.insert(TARGET_ROOM.into(), flag.name().into());
    debug!("set target to {}", flag.name());

    Ok(())
}

