use super::bt::*;
use creeps::roles::next_role;
use screeps::{self, game, objects::StructureSpawn, prelude::*, Part, ReturnCode};

/// Return the BehaviourTree that runs the spawns
pub fn task<'a>() -> Task<'a> {
    let tasks = screeps::game::spawns::values()
        .into_iter()
        .map(|spawn| Task::new(move |_| run_spawn(&spawn)))
        .collect();
    let tree = Control::Sequence(tasks);
    Task::new(move |_| tree.tick())
}

fn run_spawn(spawn: &StructureSpawn) -> ExecutionResult {
    if game::time() % 8 != 0 {
        trace!("Waiting with spawn");
        return Ok(());
    }
    debug!("Running spawn {}", spawn.name());

    if next_role(&spawn.room()).is_none() {
        debug!("Skipping spawn due to overpopulation");
        return Ok(());
    }

    let body = [Part::Move, Part::Move, Part::Carry, Part::Work];
    spawn_creep(spawn, &body)?;

    Ok(())
}

fn spawn_creep(spawn: &StructureSpawn, body: &[Part]) -> ExecutionResult {
    trace!("Spawning creep");

    let name = screeps::game::time();
    let mut prefix = 0;
    let res = loop {
        let name = format!("{}{:x}", prefix, name);
        let res = spawn.spawn_creep(&body, &name);

        if res == ReturnCode::NameExists {
            prefix += 1;
        } else {
            debug!("Spawning creep: {}, result: {}", name, res as i32);
            break res;
        }
    };

    if res != ReturnCode::Ok {
        warn!("couldn't spawn: {:?}", res);
    }
    Ok(())
}

