//! Harvest energy from foreign rooms and move it back to the owning room
//!

use super::{gofer, harvester};
use crate::bt::*;
use crate::rooms::neighbours;
use screeps::{objects::Creep, prelude::*, traits::TryFrom, ReturnCode};

pub const HOME_ROOM: &'static str = "home_room";

const HARVEST_TARGET_ROOM: &'static str = "harvest_target_room";
const LRH_TARGET: &'static str = "target";

pub fn run<'a>(creep: &'a Creep) -> Task<'a> {
    trace!("Running long_range_harvester");

    let tasks = vec![
        Task::new(move |state| load(state, creep)),
        Task::new(move |state| unload(state, creep)),
        Task::new(move |state| load(state, creep)),
        Task::new(move |state| set_target_room(state, creep)),
    ];

    let tree = Control::Sequence(tasks);
    Task::new(move |state| tree.tick(state)).with_required_bucket(1000)
}

/// Load up on energy from the target room
fn load<'a>(state: &mut GameState, creep: &'a Creep) -> ExecutionResult {
    trace!("Loading");

    if !creep.memory().bool("loading") {
        Err("not loading")?;
    }
    if creep.carry_total() == creep.carry_capacity() {
        creep.memory().set("loading", false);
        creep.memory().del("target");
        Err("full")?;
    }
    let tasks = vec![
        Task::new(move |state| move_to_room(state, creep, HARVEST_TARGET_ROOM)),
        Task::new(move |_| harvester::attempt_harvest(creep, Some(LRH_TARGET))),
    ];

    let tree = Control::Sequence(tasks);
    tree.tick(state)
}

fn move_to_room<'a>(_state: &mut GameState, creep: &'a Creep, target_key: &str) -> ExecutionResult {
    let target = creep
        .memory()
        .string(target_key)
        .map_err(|e| format!("Failed to read target room {:?}", e))?
        .ok_or("Creep has no target")?;

    let room = creep.room();

    if room.name() == target {
        Err("Already in the target room")?;
    }

    let result = js! {
        const creep = @{creep};
        const room = @{target};
        const exitDir = creep.room.findExitTo(room);
        const exit = creep.pos.findClosestByRange(exitDir);
        return creep.moveTo(exit);
    };

    let result =
        ReturnCode::try_from(result).map_err(|e| format!("Failed to parse return code {:?}", e))?;

    match result {
        ReturnCode::NoPath | ReturnCode::InvalidTarget => Err("Failed to move".to_owned()),
        _ => Ok(()),
    }
}

fn set_target_room<'a>(state: &mut GameState, creep: &'a Creep) -> ExecutionResult {
    let target = creep
        .memory()
        .string(HARVEST_TARGET_ROOM)
        .map_err(|e| format!("Failed to read target room {:?}", e))?;

    if target.is_some() {
        Err("Already has a target")?;
    }

    let room = creep.room();

    creep.memory().set(HOME_ROOM, room.name());

    let neighbours = neighbours(&room);

    let counts = state
        .long_range_harvesters
        .entry(room.name())
        .or_insert_with(|| [0; 4]);

    let (i, _, target) = neighbours
        .into_iter()
        .enumerate()
        .map(|(i, name)| (i, counts[i], name))
        .min_by_key(|(_, c, _)| c.clone())
        .ok_or("Failed to find a target room")?;

    counts[i] += 1;

    creep.memory().set(HARVEST_TARGET_ROOM, target);

    Ok(())
}

/// Unload energy in the parent room
fn unload<'a>(state: &mut GameState, creep: &'a Creep) -> ExecutionResult {
    trace!("Unloading");

    if creep.memory().bool("loading") {
        Err("loading")?;
    }
    if creep.carry_total() == 0 {
        creep.memory().set("loading", true);
        creep.memory().del("target");
        Err("empty")?;
    }
    let tasks = vec![
        Task::new(move |state| move_to_room(state, creep, HOME_ROOM)),
        Task::new(move |state| gofer::attempt_unload(state, creep)),
    ];

    let tree = Control::Sequence(tasks);
    tree.tick(state)
}

