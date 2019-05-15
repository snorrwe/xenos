use super::bt::*;
use creeps::roles::{next_role, spawn_config_by_role};
use screeps::{
    constants::find,
    game,
    memory::MemoryReference,
    objects::{SpawnOptions, StructureSpawn},
    prelude::*,
    ReturnCode,
};

/// Return the BehaviourTree that runs the spawns
pub fn task<'a>() -> Task<'a> {
    Task::new(move |state| {
        const SPAWN_SKIP: u32 = 5;

        let time = game::time();
        if time % SPAWN_SKIP != 0 {
            Err("Skip spawns this tick")?;
        }
        let rooms = game::rooms::values();
        rooms
            .into_iter()
            .map(|room| room.find(find::MY_SPAWNS))
            .filter(|spawns| spawns.len() > 0)
            .for_each(move |spawns| {
                let index = time as usize % spawns.len();
                let spawn = &spawns[index as usize];
                run_spawn(state, spawn).unwrap_or(())
            });
        Ok(())
    })
    .with_required_bucket(500)
}

fn run_spawn<'a>(state: &'a mut GameState, spawn: &'a StructureSpawn) -> ExecutionResult {
    debug!("Running spawn {}", spawn.name());

    let next_role = next_role(state, &spawn.room());

    if next_role.is_none() {
        debug!("Skipping spawn due to overpopulation");
        return Ok(());
    }

    let next_role = next_role.unwrap();

    spawn_creep(state, &spawn, &next_role)?;

    match next_role.as_str() {
        "conqueror" => *state.conqueror_count.as_mut().unwrap() += 1,
        _ => {}
    }

    Ok(())
}

fn spawn_creep(state: &mut GameState, spawn: &StructureSpawn, role: &str) -> ExecutionResult {
    trace!("Spawning creep");

    let room = spawn.room();

    let spawn_config = spawn_config_by_role(&room, role);

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

    let name = game::time() % 1_000;
    let mut prefix = 0;
    let res = loop {
        let name = format!("{}_{:x}", role, name + prefix);
        let memory = state.creep_memory_entry(name.clone());
        memory.insert("role".into(), role.into());
        let mut memory = MemoryReference::new();
        let spawn_options = SpawnOptions::new().memory(memory);
        let res = spawn.spawn_creep_with_options(&body, &name, &spawn_options);

        if res == ReturnCode::NameExists {
            prefix += 1;
        } else {
            info!(
                "Spawn {} is spawning creep: {}, result: {}",
                spawn.name(),
                name,
                res as i32
            );
            break res;
        }
    };

    if res != ReturnCode::Ok {
        Err(format!("Failed to spawn: {:?}", res))?;
    }
    Ok(())
}

