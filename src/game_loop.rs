use super::bt::*;
use super::constructions;
use super::creeps;
use super::structures::{spawns, towers};
use std::collections::HashSet;
use stdweb::unstable::TryFrom;

pub fn game_loop() {
    debug!("Loop starting! CPU: {}", screeps::game::cpu::get_used());

    trace!("Running");

    // screeps api `bucket` method panics in simulation
    let bucket = js! {
        let bucket = Game.cpu.bucket;
        return bucket != null ? bucket : -1;
    };

    let bucket = i32::try_from(bucket).expect("Expected bucket to be a number");

    let mut state = GameState { cpu_bucket: bucket };

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
        cleanup_memory().unwrap_or_else(|e| {
            error!("Failed to clean up memory {:?}", e);
        });
    }

    info!(
        "---------------- Done! CPU: {:.4} Bucket: {} ----------------",
        screeps::game::cpu::get_used(),
        bucket
    );
}

fn cleanup_memory() -> Result<(), Box<::std::error::Error>> {
    trace!("Cleaning memory");

    let alive_creeps: HashSet<String> = screeps::game::creeps::keys().into_iter().collect();

    let screeps_memory = match screeps::memory::root().dict("creeps")? {
        Some(v) => v,
        None => {
            warn!("not cleaning game creep memory: no Memory.creeps dict");
            return Ok(());
        }
    };

    for mem_name in screeps_memory.keys() {
        if !alive_creeps.contains(&mem_name) {
            debug!("cleaning up creep memory of dead creep {}", mem_name);
            screeps_memory.del(&mem_name);
        }
    }

    debug!("Cleaned up memory");

    Ok(())
}
