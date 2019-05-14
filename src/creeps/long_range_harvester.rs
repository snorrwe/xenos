//! Harvest energy from foreign rooms and move it back to the owning room
//!

use super::{gofer, harvester};
use crate::bt::*;
use crate::game_state::{RoomIFF, ScoutInfo};
use crate::rooms::neighbours;
use screeps::{constants::find, objects::Creep, prelude::*, traits::TryFrom, ReturnCode};

pub const HOME_ROOM: &'static str = "home_room";

const HARVEST_TARGET_ROOM: &'static str = "harvest_target_room";
const LRH_TARGET: &'static str = "target";

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
        creep.memory().del(LRH_TARGET);
        Err("full")?;
    }
    let tasks = vec![
        Task::new(move |_| approach_target_room(creep, HARVEST_TARGET_ROOM)),
        Task::new(move |state| set_target_room(state, creep)),
        Task::new(move |state| {
            update_scout_info(state, creep).unwrap_or(());
            Err("continue")?
        }),
        Task::new(move |_| harvester::attempt_harvest(creep, Some(LRH_TARGET))),
    ];

    let tree = Control::Sequence(tasks);
    tree.tick(state)
}

fn update_scout_info(state: &mut GameState, creep: &Creep) -> ExecutionResult {
    let room = creep.room();

    let n_sources = room.find(find::SOURCES).len() as u8;

    let controller = room.controller();

    let iff = match controller.as_ref().map(|c| c.my()) {
        None => RoomIFF::NoMansLand,
        Some(true) => RoomIFF::Friendly,
        Some(false) => match controller.map(|c| c.level()) {
            Some(0) => RoomIFF::Neutral,
            Some(_) => RoomIFF::Hostile,
            None => RoomIFF::Unknown,
        },
    };

    let info = ScoutInfo { n_sources, iff };

    state.scout_intel.insert(room.name(), info);

    Ok(())
}

fn approach_target_room<'a>(creep: &'a Creep, target_key: &str) -> ExecutionResult {
    let target = creep
        .memory()
        .string(target_key)
        .map_err(|e| format!("Failed to read target room {:?}", e))?
        .ok_or("Creep has no target")?;

    let room = creep.room();
    let room_name = room.name();

    if room_name == target {
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
    let target = creep.memory().string(HARVEST_TARGET_ROOM).unwrap_or(None);

    if target.is_some() {
        Err("Already has a target")?;
    }

    let room = creep.room();

    creep.memory().set(HOME_ROOM, room.name());

    let neighbours = neighbours(&room);

    let counts: &mut _ = state
        .long_range_harvesters
        .entry(room.name())
        .or_insert_with(|| [0; 4]);

    let scout_intel = &state.scout_intel;

    let (i, target) = neighbours
        .into_iter()
        .enumerate()
        .filter(|(_, name)| {
            info!("??? {:?} {:?}", name, scout_intel.get(name));
            scout_intel
                .get(name)
                .map(|int| match int.iff {
                    RoomIFF::Unknown | RoomIFF::Neutral => true,
                    _ => false,
                })
                .unwrap_or(true)
        })
        .min_by_key(|(i, _)| counts[*i])
        .ok_or_else(|| {
            warn!(
                "Failed to set target room of LRH {:?} in room {:?}",
                creep.name(),
                creep.room().name()
            );
            "Failed to find a target room"
        })?;

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
        Task::new(move |_| approach_target_room(creep, HOME_ROOM)),
        Task::new(move |state| gofer::attempt_unload(state, creep)),
    ];

    let tree = Control::Sequence(tasks);
    tree.tick(state)
}

