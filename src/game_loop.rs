use super::bt::*;
use super::creeps;
use super::spawns;
use super::towers;
use std::collections::HashSet;
use stdweb::unstable::TryFrom;

pub fn game_loop() {
    debug!("Loop starting! CPU: {}", screeps::game::cpu::get_used());

    run();

    if screeps::game::time() % 32 == 0 {
        cleanup_memory().unwrap_or_else(|e| {
            error!("Failed to clean up memory {:?}", e);
        });
    }

    // screeps api `bucket` method panics in simulation
    let bucket = js! {
        let bucket = Game.cpu.bucket;
        return bucket != null ? bucket : -1;
    };

    let bucket = i32::try_from(bucket).expect("Expected bucket to be a number");

    info!(
        "Done! CPU: {:.4} Bucket: {}",
        screeps::game::cpu::get_used(),
        bucket
    );
}

/// Run the game logic
fn run() {
    trace!("Running");

    let tasks = vec![spawns::task(), creeps::task(), towers::task()];
    let tree = Control::All(tasks);
    let result = tree.tick();

    trace!("Run result {:?}", result);
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

    info!("Cleaned up memory");

    Ok(())
}

