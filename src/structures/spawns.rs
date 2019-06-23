use crate::prelude::*;
use arrayvec::ArrayVec;
use creeps::roles::{next_role, spawn_config_by_role, Role};
use creeps::{CREEP_ROLE, HOME_ROOM};
use screeps::{
    constants::find,
    game,
    objects::{SpawnOptions, StructureSpawn},
    prelude::*,
    Part, ReturnCode,
};

/// Return the BehaviourTree that runs the spawns
pub fn task<'a>() -> Task<'a, GameState> {
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

    spawn_creep(state, &spawn, next_role)?;

    Ok(())
}

fn spawn_creep(state: &mut GameState, spawn: &StructureSpawn, role: Role) -> ExecutionResult {
    trace!("Spawning creep");

    let room = spawn.room();

    let spawn_config = spawn_config_by_role(&room, role);

    let mut body = spawn_config
        .basic_body
        .into_iter()
        .collect::<ArrayVec<[Part; 50]>>();
    let mut body_len = body.len(); // the index until the body is valid
    let max_len = spawn_config.body_max;

    if !spawn_config.body_extension.is_empty() {
        loop {
            let spawn_options = SpawnOptions::new().dry_run(true);
            if body.len() == 50
                || max_len
                    .map(|max_len| max_len <= body.len())
                    .unwrap_or(false)
            {
                break;
            }
            body.extend(spawn_config.body_extension.iter().cloned());
            let result = spawn.spawn_creep_with_options(&body, "___test_name", &spawn_options);
            if result == ReturnCode::Ok {
                body_len = body.len();
            } else if result == ReturnCode::NotEnough {
                break;
            } else {
                warn!("Can not spawn, error: {:?}", result);
                return Err("Can not spawn".into());
            }
        }
    }

    let name = game::time() % 10_000;
    let mut prefix = 0;
    let res = 'spawn_loop: loop {
        let name = format!("{}_{:04x}", role, name + prefix);
        let res = spawn.spawn_creep(&body[..body_len], &name);

        match res {
            ReturnCode::NameExists => {
                prefix += 1;
            }
            ReturnCode::Ok => {
                let memory = state.creep_memory_entry(CreepName(&name));
                memory.insert(HOME_ROOM.into(), spawn.room().name().into());
                memory.insert(CREEP_ROLE.into(), (role as i64).into());
                info!(
                    "Spawn {} is spawning creep: {}, result: {}",
                    spawn.name(),
                    name,
                    res as i32
                );
                break 'spawn_loop res;
            }
            _ => {
                break 'spawn_loop res;
            }
        }
    };

    if res != ReturnCode::Ok {
        Err(format!("Failed to spawn: {:?}", res))?;
    }
    Ok(())
}

