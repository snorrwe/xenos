//! Long Range Worker
//! Used to work on other rooms
//!
use super::{move_to, update_scout_info, worker, CreepState};
use crate::prelude::*;
use screeps::{game, prelude::*};
use stdweb::unstable::TryInto;

const TARGET_ROOM: &'static str = "target_room";

pub fn task<'a>(state: &'a mut CreepState) -> Task<'a, CreepState> {
    let tasks = [
        Task::new(|state| {
            update_scout_info(state).unwrap_or_else(|e| {
                error!("Failed to update scout info {:?}", e);
            });
            Err("continue")?
        })
        .with_name("Update scout info"),
        Task::new(|state| approach_target_room(state)).with_name("Approach target room"),
        Task::new(|state| set_target(state)).with_name("Set target"),
        worker::task(state).with_name("Worker run"),
    ]
    .into_iter()
    .cloned()
    .collect();

    let tree = Control::Sequence(tasks);
    Task::from(tree).with_required_bucket(300).with_name("LRW")
}

fn approach_target_room<'a>(state: &mut CreepState) -> ExecutionResult {
    let flag = {
        let flag = state
            .creep_memory_string(TARGET_ROOM)
            .ok_or_else(|| "no target set")?;
        game::flags::get(flag)
    };

    let flag = flag.ok_or_else(|| {
        state.creep_memory_remove(TARGET_ROOM);
        "target flag does not exist"
    })?;

    let creep = state.creep();
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

fn set_target<'a>(state: &mut CreepState) -> ExecutionResult {
    trace!("finding target");

    if state.creep_memory_string(TARGET_ROOM).is_some() {
        trace!("has target");
        Err("Creep already has a target")?;
    }
    let flags = game::flags::values();
    let flag = flags.iter().next().ok_or_else(|| "can't find a flag")?;

    state.creep_memory_set(TARGET_ROOM.into(), flag.name().into());
    debug!("set target to {}", flag.name());

    Ok(())
}

