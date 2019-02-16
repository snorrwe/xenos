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

    let spawn_config = spawn_config_by_role(role);

    let mut body = spawn_config
        .basic_body
        .into_iter()
        .map(|x| *x)
        .collect::<Vec<_>>();

    // Max n tries
    for _ in 0..10 {
        let spawn_options = SpawnOptions::new().dry_run(true);
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

    let name = screeps::game::time();
    let mut prefix = 0;
    let res = loop {
        let name = format!("{}{:x}", prefix, name);
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

