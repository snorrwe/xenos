use crate::bt::*;
use crate::constructions;
use crate::creeps;
use crate::flags;
use crate::game_state::GameState;
use crate::stats::save_stats;
use crate::structures::{spawns, towers};
use crate::MAIN_SEGMENT;
use log::Level::Info;
use stdweb::unstable::TryFrom;

pub fn game_loop() {
    debug!("Loop starting! CPU: {}", screeps::game::cpu::get_used());

    trace!("Running");

    // screeps api `bucket` method panics in simulation
    let bucket = js! {
        return Game.cpu.bucket;
    };

    let bucket = Option::<i32>::try_from(bucket).expect("Expected bucket to be a number");

    let mut state = GameState::read_from_segment_or_default(MAIN_SEGMENT);
    state.cpu_bucket = bucket.map(|x| x as i16);
    run_game_logic(&mut state);

    let bucket = bucket.unwrap_or(-1);

    if log_enabled!(Info) && bucket > 1000 {
        save_stats(
            screeps::game::time() as u32,
            screeps::game::creeps::keys().len() as u32,
            // Note that cpu stats won't take the stats saving into account
            screeps::game::cpu::get_used() as f32,
            bucket,
            &state,
        )
        .map(|_| {
            info!("Statistics saved!");
        })
        .unwrap_or_else(|e| error!("Failed to save stats {:?}", e));
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
/// Consumes the state object
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

    if screeps::game::time() % 16 == 0 {
        state.cleanup_memory().unwrap_or_else(|e| {
            error!("Failed to clean up memory {:?}", e);
        });
    }
}

