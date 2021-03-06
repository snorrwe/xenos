use super::{approach_target_room, update_scout_info, CreepState, TARGET};
use crate::prelude::*;
use crate::rooms::neighbours;
use screeps::prelude::*;

pub fn run<'a>(state: &mut CreepState) -> ExecutionResult {
    let tasks = [
        Task::new(|state| {
            update_scout_info(state).unwrap_or_else(|e| {
                warn!("Failed to update scout info {}", e);
            });
            state.creep().say("🛰️", false);
            Err("Continue")?
        }),
        Task::new(|state| {
            approach_target_room(state, TARGET).map_err(|e| {
                state.creep_memory_remove(TARGET);
                debug!("Approach failed {}", e);
                e
            })
        }),
        Task::new(|state| set_next_room(state)),
    ];

    sequence(state, tasks.iter())
}

fn set_next_room(state: &mut CreepState) -> ExecutionResult {
    if state.creep_memory_string(TARGET).is_some() {
        Err("Already has a target")?;
    }

    let room = state.creep().room();
    let gs = state.get_game_state();

    let mut min = 1 << 30;
    let mut target_room = WorldPosition::default();
    for room in neighbours(&room) {
        if let Some(intel) = gs.scout_intel.get(&room) {
            if intel.time_of_recording < min {
                min = intel.time_of_recording;
                target_room = room;
            }
        } else {
            target_room = room;
            break;
        }
    }

    state.creep_memory_set(TARGET, target_room.to_string().as_str());
    Ok(())
}

