use super::bt::*;
use creeps::roles::{next_role, role_part_scale, role_parts};
use screeps::{
    self, game,
    objects::{SpawnOptions, StructureSpawn},
    prelude::*,
    ReturnCode,
};

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

    let next_role = next_role(&spawn.room());
    if next_role.is_none() {
        debug!("Skipping spawn due to overpopulation");
        return Ok(());
    }

    let next_role = next_role.unwrap();

    spawn_creep(spawn, &next_role.as_str())?;

    Ok(())
}

fn spawn_creep(spawn: &StructureSpawn, role: &str) -> ExecutionResult {
    trace!("Spawning creep");

    let mut body = role_parts(role);

    // Overflow procetion
    for _ in 0..10 {
        let spawn_options = SpawnOptions::new().dry_run(true);
        let mut b = body.clone();
        b.append(&mut role_part_scale(role));
        let result = spawn.spawn_creep_with_options(&b, "___test_name", &spawn_options);
        if result == ReturnCode::Ok {
            body = b;
        } else if result == ReturnCode::NotEnough {
            break;
        } else {
            warn!("Can not spawn, error: {:?}", result);
            return Err(());
        }
    }

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
        warn!("Failed to spawn: {:?}", res);
    }
    Ok(())
}

