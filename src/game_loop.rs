use super::bt::*;
use super::constructions;
use super::creeps;
use super::structures::{spawns, towers};
use crate::game_state::GameState;
use log::Level::Info;
use screeps::raw_memory;
use stdweb::unstable::TryFrom;

const STATISTICS_SEGMENT: u32 = 1;

pub fn game_loop() {
    debug!("Loop starting! CPU: {}", screeps::game::cpu::get_used());

    trace!("Running");

    // screeps api `bucket` method panics in simulation
    let bucket = js! {
        return Game.cpu.bucket;
    };

    let bucket = Option::<i32>::try_from(bucket).expect("Expected bucket to be a number");

    let mut state = GameState::read_from_segment_or_default(0);
    state.cpu_bucket = bucket;
    state.memory_segment = Some(0);
    state.memory_route = None;
    run_game_logic(state);

    let bucket = bucket.unwrap_or(-1);

    if log_enabled!(Info) && bucket > 1000 {
        save_stats(
            screeps::game::time() as u32,
            screeps::game::creeps::keys().len() as u32,
            // Note that cpu stats won't take the stats saving into account
            screeps::game::cpu::get_used() as f32,
            bucket,
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
fn run_game_logic(mut state: GameState) {
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

    if screeps::game::time() % 16 == 0 {
        state.cleanup_memory().unwrap_or_else(|e| {
            error!("Failed to clean up memory {:?}", e);
        });
    }
}

fn save_stats(time: u32, creep_count: u32, cpu: f32, bucket: i32) -> ExecutionResult {
    let mut stats: Vec<TickStats> = raw_memory::get_segment(STATISTICS_SEGMENT)
        .and_then(|s| serde_json::from_str(s.as_str()).ok())
        .unwrap_or(vec![]);

    let gcl = screeps::game::gcl::level();
    let gcl_progress = screeps::game::gcl::progress() as f32;
    let gcl_progress_total = screeps::game::gcl::progress_total() as f32;

    let tick_stats = TickStats {
        time,
        creep_count,
        cpu,
        bucket,
        gcl,
        gcl_progress,
        gcl_progress_total,
    };

    stats.push(tick_stats);

    let data = serde_json::to_string(&stats).unwrap_or("[]".into());

    if data.len() > 99_999 {
        Err("Statistics segment is full")?;
    }

    raw_memory::set_segment(STATISTICS_SEGMENT, &data);

    Ok(())
}

#[derive(Serialize, Deserialize, Debug)]
struct TickStats {
    time: u32,
    cpu: f32,
    bucket: i32,
    creep_count: u32,
    gcl: u32,
    gcl_progress: f32,
    gcl_progress_total: f32,
}
