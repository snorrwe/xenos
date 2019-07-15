use super::bt::*;
use super::creeps;
use crate::game_state::GameState;
use crate::{DEPLOYMENT_TIME, STATISTICS_SEGMENT, VERSION};
use screeps::raw_memory;

#[derive(Serialize, Deserialize, Debug)]
struct TickStats {
    version: String,
    deployment_time: u32,
    time: u32,
    cpu: f32,
    cpu_limit: f32,
    bucket: i32,
    creep_count: u32,
    gcl: u32,
    gcl_progress: f32,
    gcl_progress_total: f32,
    creep_stats: creeps::CreepExecutionStats,
}

pub fn save_stats(
    time: u32,
    creep_count: u32,
    cpu: f32,
    bucket: i32,
    state: &GameState,
) -> ExecutionResult {
    let mut stats: Vec<TickStats> = raw_memory::get_segment(STATISTICS_SEGMENT)
        .and_then(|s| serde_json::from_str(s.as_str()).ok())
        .unwrap_or(vec![]);

    let gcl = screeps::game::gcl::level();
    let gcl_progress = screeps::game::gcl::progress() as f32;
    let gcl_progress_total = screeps::game::gcl::progress_total() as f32;
    let cpu_limit = screeps::game::cpu::limit() as f32;

    let tick_stats = TickStats {
        version: VERSION.to_owned(),
        deployment_time: *DEPLOYMENT_TIME,
        time,
        creep_count,
        cpu,
        cpu_limit,
        bucket,
        gcl,
        gcl_progress,
        gcl_progress_total,
        creep_stats: state.creep_stats.clone(),
    };

    stats.push(tick_stats);

    let data = serde_json::to_string(&stats).unwrap_or("[]".to_owned());

    if data.len() > 100 * 1024 {
        Err("Statistics segment is full")?;
    }

    raw_memory::set_segment(STATISTICS_SEGMENT, &data);

    Ok(())
}

