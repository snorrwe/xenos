//! Harvest energy from foreign rooms and move it back to the owning room
//!

use super::{gofer, harvester};
use crate::rooms::neighbours;
use crate::bt::*;
use screeps::{objects::Creep, prelude::*};

const HARVEST_TARGET_ROOM: &'static str = "harvest_target_room";
const LRH_TARGET: &'static str = "target";
const HOME_ROOM: &'static str = "home_room";

pub fn run<'a>(creep: &'a Creep) -> Task<'a> {
    trace!("Running long_range_harvester");

    let tasks = vec![
        Task::new(move |state| load(state, creep)),
        Task::new(move |state| unload(state, creep)),
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
        Task::new(move |state| set_target_room(state, creep)),
        // move to room
        // harvest
    ];

    let tree = Control::Sequence(tasks);
    tree.tick(state)
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


    unimplemented!()
}

/// Unload energy in the parent room
fn unload<'a>(state: &mut GameState, creep: &'a Creep) -> ExecutionResult {
    trace!("Unloading");
    unimplemented!()
}

