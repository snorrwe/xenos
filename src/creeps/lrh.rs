//! Long Range Harvester
//! Harvest energy from foreign rooms and move it back to the owning room
//!

use super::{
    approach_target_room, gofer, harvester, update_scout_info, upgrader, CreepState, HOME_ROOM,
    LOADING, TARGET, TASK,
};
use crate::prelude::*;
use crate::rooms::neighbours;
use crate::state::RoomIFF;
use num::FromPrimitive;
use screeps::prelude::*;

const HARVEST_TARGET_ROOM: &'static str = "harvest_target_room";

#[derive(Debug, Clone, Copy, FromPrimitive, ToPrimitive)]
#[repr(u8)]
enum LrhState {
    Idle = 0,
    Loading = 1,
    Unloading = 2,
}

pub fn run<'a>(state: &mut CreepState) -> ExecutionResult {
    let last_task = state.creep_memory_i64(TASK).unwrap_or(0);
    let last_task = LrhState::from_u32(last_task as u32).unwrap_or(LrhState::Idle);

    let mut priorities = [0; 3];
    priorities[last_task as usize] += 1;

    let mut tasks = [
        Task::new(|state| load(state))
            .with_name("Load")
            .with_state_save(LrhState::Loading)
            .with_priority(priorities[LrhState::Loading as usize])
            .with_required_bucket(2000),
        Task::new(|state| unload(state))
            .with_name("Unload")
            .with_state_save(LrhState::Unloading)
            .with_priority(priorities[LrhState::Unloading as usize]),
        Task::new(|state| harvester::unload(state))
            .with_name("Harvester unload")
            .with_priority(-1),
        Task::new(|state| upgrader::attempt_upgrade(state)).with_priority(-2),
    ];

    sorted_by_priority(&mut tasks);
    sequence(state, tasks.iter())
}

/// Load up on energy from the target room
fn load<'a>(state: &mut CreepState) -> ExecutionResult {
    trace!("Loading");

    if !state.creep_memory_bool(LOADING).unwrap_or(false) {
        Err("not loading")?;
    }
    let creep = state.creep();
    if creep.carry_total() == creep.carry_capacity() {
        state.creep_memory_set(LOADING.into(), false);
        state.creep_memory_remove(TARGET);
        Err("full")?;
    }
    let tasks = [
        Task::new(|state| approach_target_room(state, HARVEST_TARGET_ROOM))
            .with_name("Approach target room"),
        Task::new(|state| set_target_room(state)).with_name("Set target room"),
        Task::new(|state| {
            update_scout_info(state)?;
            Err("continue")?
        })
        .with_name("Update scout info"),
        Task::new(|state| harvester::attempt_harvest(state, Some(TARGET)))
            .with_name("Attempt harvest"),
    ];

    sequence(state, tasks.iter())
}

fn set_target_room<'a>(state: &'a mut CreepState) -> ExecutionResult {
    {
        let target = state.creep_memory_string(HARVEST_TARGET_ROOM);
        if target.is_some() {
            Err("Already has a target")?;
        }
    }

    let room = {
        let creep = state.creep();
        creep.room()
    };
    let neighbours = neighbours(&room);

    let target = {
        let gs: &mut GameState = unsafe { &mut *state.mut_game_state() };
        let counts: &mut _ = gs
            .long_range_harvesters
            .entry(WorldPosition::from(room))
            .or_insert_with(|| [0; 4]);

        let scout_intel = &gs.scout_intel;

        let (i, target) = neighbours
            .into_iter()
            .enumerate()
            .filter(|(_, wp)| {
                scout_intel
                    .get(&wp)
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
                    state.creep().name(),
                    state.creep().room().name()
                );
                "Failed to find a target room"
            })?;

        counts[i] += 1;
        target
    };

    state.creep_memory_set(HARVEST_TARGET_ROOM.into(), target.to_string().as_str());

    Ok(())
}

/// Unload energy in the parent room
fn unload<'a>(state: &mut CreepState) -> ExecutionResult {
    trace!("Unloading");

    if state.creep_memory_bool(LOADING).unwrap_or(false) {
        Err("loading")?;
    }
    if state.creep().carry_total() == 0 {
        state.creep_memory_set(LOADING.into(), true);
        state.creep_memory_remove(TARGET);
        Err("empty")?;
    }
    let tasks = [
        Task::new(|state| approach_target_room(state, HOME_ROOM)).with_name("Approach target room"),
        Task::new(|state| gofer::attempt_unload(state)).with_name("Attempt unload"),
    ];

    sequence(state, tasks.iter())
}

