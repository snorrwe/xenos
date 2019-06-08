//! Long Range Harvester
//! Harvest energy from foreign rooms and move it back to the owning room
//!

use super::{gofer, harvester, update_scout_info, HOME_ROOM, TARGET};
use crate::game_state::RoomIFF;
use crate::prelude::*;
use crate::rooms::neighbours;
use screeps::{objects::Creep, prelude::*, traits::TryFrom, ReturnCode};

const HARVEST_TARGET_ROOM: &'static str = "harvest_target_room";

pub fn run<'a>(creep: &'a Creep) -> Task<'a, GameState> {
    trace!("Running long_range_harvester");

    let tasks = [
        Task::new(move |state| load(state, creep)),
        Task::new(move |state| unload(state, creep)),
        Task::new(move |state| harvester::unload(state, creep)),
    ]
    .into_iter()
    .cloned()
    .collect();

    let tree = Control::Sequence(tasks);
    Task::new(move |state| tree.tick(state)).with_required_bucket(2000)
}

/// Load up on energy from the target room
fn load<'a>(state: &mut GameState, creep: &'a Creep) -> ExecutionResult {
    trace!("Loading");

    if !state.creep_memory_bool(CreepName(&creep.name()), "loading") {
        Err("not loading")?;
    }
    let tree = {
        let memory = state.creep_memory_entry(CreepName(&creep.name()));
        if creep.carry_total() == creep.carry_capacity() {
            memory.insert("loading".into(), false.into());
            memory.remove(TARGET);
            Err("full")?;
        }
        let tasks = [
            Task::new(move |state| approach_target_room(state, creep, HARVEST_TARGET_ROOM)),
            Task::new(move |state| set_target_room(state, creep)),
            Task::new(move |state| {
                update_scout_info(state, creep)?;
                Err("continue")?
            }),
            Task::new(move |state| harvester::attempt_harvest(state, creep, Some(TARGET))),
        ]
        .into_iter()
        .cloned()
        .collect();

        Control::Sequence(tasks)
    };
    tree.tick(state)
}

fn approach_target_room<'a>(
    state: &mut GameState,
    creep: &'a Creep,
    target_key: &str,
) -> ExecutionResult {
    let target = state
        .creep_memory_string(CreepName(&creep.name()), target_key)
        .ok_or("no target")?;

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
    {
        let target = state.creep_memory_string(CreepName(&creep.name()), HARVEST_TARGET_ROOM);
        if target.is_some() {
            Err("Already has a target")?;
        }
    }

    let room = creep.room();

    let neighbours = neighbours(&room);

    let target = {
        let counts: &mut _ = state
            .long_range_harvesters
            .entry(room.name())
            .or_insert_with(|| [0; 4]);

        let scout_intel = &state.scout_intel;

        let (i, target) = neighbours
            .into_iter()
            .enumerate()
            .filter(|(_, name)| {
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
        target
    };

    let memory = state.creep_memory_entry(CreepName(&creep.name()));
    memory.insert(HARVEST_TARGET_ROOM.into(), target.into());

    Ok(())
}

/// Unload energy in the parent room
fn unload<'a>(state: &mut GameState, creep: &'a Creep) -> ExecutionResult {
    trace!("Unloading");

    if state.creep_memory_bool(CreepName(&creep.name()), "loading") {
        Err("loading")?;
    }
    let tree = {
        let memory = state.creep_memory_entry(CreepName(&creep.name()));
        if creep.carry_total() == 0 {
            memory.insert("loading".into(), true.into());
            memory.remove(TARGET);
            Err("empty")?;
        }
        let tasks = [
            Task::new(move |state| approach_target_room(state, creep, HOME_ROOM)),
            Task::new(move |state| gofer::attempt_unload(state, creep)),
        ]
        .into_iter()
        .cloned()
        .collect();

        Control::Sequence(tasks)
    };
    tree.tick(state)
}

