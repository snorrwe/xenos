use super::bt::*;
use super::constructions;
use super::creeps;
use super::structures::{spawns, towers};
use crate::game_state::GameState;
use std::collections::HashSet;
use stdweb::unstable::TryFrom;

pub fn game_loop() {
    debug!("Loop starting! CPU: {}", screeps::game::cpu::get_used());

    trace!("Running");

    // screeps api `bucket` method panics in simulation
    let bucket = js! {
        return Game.cpu.bucket;
    };

    let bucket = Option::<i32>::try_from(bucket).expect("Expected bucket to be a number");

    let mut state = GameState::read_from_memory_or_default();
    state.cpu_bucket = bucket;

    creeps::task()
        .tick(&mut state)
        .unwrap_or_else(|e| warn!("Failed to run creeps {:?}", e));

    towers::task()
        .tick(&mut state)
        .unwrap_or_else(|e| warn!("Failed to run towers {:?}", e));

    spawns::task()
        .tick(&mut state)
        .unwrap_or_else(|e| warn!("Failed to run spawns {:?}", e));

    constructions::task()
        .tick(&mut state)
        .unwrap_or_else(|e| warn!("Failed to run constructions {:?}", e));

    if screeps::game::time() % 32 == 0 {
        state.cleanup_memory().unwrap_or_else(|e| {
            error!("Failed to clean up memory {:?}", e);
        });
    }

    info!(
        "---------------- Done! CPU: {:.4} Bucket: {} ----------------",
        screeps::game::cpu::get_used(),
        bucket.unwrap_or(-1)
    );
}

