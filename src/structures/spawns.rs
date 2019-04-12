use super::bt::*;
use creeps::roles::{next_role, spawn_config_by_role};
use screeps::{
    self, game,
    memory::MemoryReference,
    objects::{SpawnOptions, StructureSpawn},
    prelude::*,
    ReturnCode,
};

/// Return the BehaviourTree that runs the spawns
pub fn task<'a>() -> Task<'a> {
    Task::new(move |_| {
        screeps::game::spawns::values()
            .into_iter()
            .for_each(|spawn| run_spawn(&spawn).unwrap_or(()));
        Ok(())
    })
    .with_required_bucket(500)
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

    let spawn_config = spawn_config_by_role(role);

    let mut body = spawn_config.basic_body;
    let max_len = spawn_config.body_max;

    if !spawn_config.body_extension.is_empty() {
        // Limit number of tries
        for _ in 0..10 {
            let spawn_options = SpawnOptions::new().dry_run(true);
            if body.len() >= max_len.unwrap_or(body.len() + 1) {
                break;
            }
            let mut b = body.clone();
            b.extend(spawn_config.body_extension.iter());
            let result = spawn.spawn_creep_with_options(&b, "___test_name", &spawn_options);
            if result == ReturnCode::Ok {
                body = b;
            } else if result == ReturnCode::NotEnough {
                break;
            } else {
                warn!("Can not spawn, error: {:?}", result);
                return Err("Can not spawn".into());
            }
        }
    }

    let name = screeps::game::time() % 1_000;
    let mut prefix = 0;
    let res = loop {
        let name = format!("{}_{:x}", role, name + prefix);
        let mut memory = MemoryReference::new();
        memory.set("role", role);
        let spawn_options = SpawnOptions::new().memory(memory);
        let res = spawn.spawn_creep_with_options(&body, &name, &spawn_options);

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
