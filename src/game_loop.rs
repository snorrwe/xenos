use super::bt::*;
use super::creeps;
use super::spawns;
use std::collections::HashSet;

pub fn game_loop() {
    info!("Loop starting! CPU: {}", screeps::game::cpu::get_used());

    run();

    cleanup_memory().unwrap_or_else(|e| {
        error!("Failed to clean up memory {:?}", e);
    });

    info!("Done! CPU: {} Bucket: {}", screeps::game::cpu::get_used(), screeps::game::cpu::bucket());
}

/// Run the game logic
/// Adding new module:
///     - add the modules tasks to `tasks`
fn run() {
    trace!("Running");

    let tasks = vec![spawns::task(), creeps::task()];

    let tree = BehaviourTree::new(Control::All(tasks));

    let result = tree.tick();

    debug!("Run result {:?}", result);

    trace!("Running Done");
}

fn cleanup_memory() -> Result<(), Box<::std::error::Error>> {
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

    Ok(())
}

