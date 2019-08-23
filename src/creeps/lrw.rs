//! Long Range Worker
//! Used to work on other rooms
//!
use super::{move_to_options, update_scout_info, worker, CreepState, MoveToOptions};
use crate::prelude::*;

const TARGET_ROOM: &'static str = "target_room";

pub fn run<'a>(state: &mut CreepState) -> ExecutionResult {
    Task::new(_run)
        .with_required_bucket(300)
        .with_name("LRW")
        .tick(state)
}

fn _run(state: &mut CreepState) -> ExecutionResult {
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
        Task::new(|state| worker::run(state)).with_name("Worker run"),
    ];

    sequence(state, tasks.iter())
}

fn approach_target_room<'a>(state: &mut CreepState) -> ExecutionResult {
    let target_room = {
        state
            .creep_memory_string(TARGET_ROOM)
            .ok_or_else(|| "no target set")?
    };

    let creep = state.creep();

    let arrived = state.current_room().to_string().as_str() == target_room;

    if arrived {
        Err("Already in the room")?;
    }
    let target_room = WorldPosition::parse_name(target_room)
        .map_err(|e| format!("Got an invalid room name as conquest target {:?}", e))?
        .as_room_center();
    move_to_options(
        creep,
        &target_room,
        MoveToOptions {
            reuse_path: Some(30),
        },
    )
}

fn set_target<'a>(state: &mut CreepState) -> ExecutionResult {
    if state.creep_memory_string(TARGET_ROOM).is_some() {
        trace!("has target");
        Err("Creep already has a target")?;
    }
    let flag = {
        state
            .get_game_state()
            .expansion
            .iter()
            .next()
            .ok_or_else(|| "can't find a target")?
            .clone()
    };

    state.creep_memory_set(TARGET_ROOM.into(), flag.to_string().as_str());

    Ok(())
}

