use crate::constructions;
use crate::creeps;
use crate::expansion;
use crate::flags;
use crate::prelude::*;
use crate::state::MemorySentinel;
use crate::stats::save_stats;
use crate::structures::{spawns, towers};
use crate::MAIN_SEGMENT;
use log::Level::Info;
use std::pin::Pin;
use stdweb::unstable::TryFrom;

pub fn game_loop() {
    debug!("Loop starting! CPU: {}", screeps::game::cpu::get_used());

    let mut game_state = GameState::read_from_segment_or_default(MAIN_SEGMENT);
    let game_state = unsafe {
        let mut_ref: Pin<&mut GameState> = Pin::as_mut(&mut game_state);
        Pin::get_unchecked_mut(mut_ref)
    };

    let _sentinel = MemorySentinel::<GameState>::new(MAIN_SEGMENT as u8, &*game_state);

    trace!("Running");

    // screeps api `bucket` method panics in simulation
    let bucket = js! {
        return Game.cpu.bucket;
    };

    let bucket = Option::<i32>::try_from(bucket).expect("Expected bucket to be a number");

    game_state.cpu_bucket = bucket.map(|x| x as i16);
    run_game_logic(game_state);

    let bucket = bucket.unwrap_or(-1);

    if log_enabled!(Info) {
        save_stats(
            screeps::game::time() as u32,
            screeps::game::creeps::keys().len() as u32,
            // Note that cpu stats won't take the stats saving into account
            screeps::game::cpu::get_used() as f32,
            bucket,
            &game_state,
        )
        .map(|_| {
            info!("Statistics saved!");
        })
        .unwrap_or_else(|e| warn!("Failed to save stats {:?}", e));
    }

    // Yes, measure again even after stats save
    let cpu = screeps::game::cpu::get_used();

    info!(
        "---------------- Done! CPU: {:.4} Bucket: {} ----------------",
        cpu, bucket
    );
}

/// Call subsystems in order of priority
/// Runs to completion even if a subsystem fails
///
/// TODO: GameResult object to return?
fn run_game_logic(state: &mut GameState) {
    creeps::task()
        .tick(state)
        .unwrap_or_else(|e| warn!("Failed to run creeps {}", e));
    towers::task()
        .tick(state)
        .unwrap_or_else(|e| warn!("Failed to run towers {}", e));
    spawns::task()
        .tick(state)
        .unwrap_or_else(|e| warn!("Failed to run spawns {}", e));
    constructions::task()
        .tick(state)
        .unwrap_or_else(|e| warn!("Failed to run constructions {}", e));
    flags::task()
        .tick(state)
        .unwrap_or_else(|e| warn!("Failed to run flags {}", e));

    expansion::run(state).unwrap_or_else(|e| warn!("Failed to expansion {}", e));

    if screeps::game::time() % 16 == 0 {
        state.cleanup_memory().unwrap_or_else(|e| {
            error!("Failed to clean up memory {:?}", e);
        });
    }
}

