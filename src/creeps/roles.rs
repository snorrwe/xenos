use super::{super::bt::*, builder, conqueror, gofer, harvester, repairer, upgrader};
use screeps::{
    constants::find,
    game,
    objects::{Creep, Room},
    Part,
};
use std::collections::HashMap;
use stdweb::unstable::TryInto;

pub struct SpawnConfig {
    pub basic_body: Vec<Part>,
    pub body_extension: Vec<Part>,
    pub body_max: Option<usize>,
}

// TODO: return an array of all roles to spawn in order of priority
/// Get the next target role in the given room
pub fn next_role<'a>(state: &'a mut GameState, room: &'a Room) -> Option<String> {
    let mut counts = count_roles_in_room(room);
    counts.entry("conqueror".into()).or_insert_with(|| {
        state.conqueror_count.unwrap_or_else(|| {
            // Lazily count conquerors
            let count = count_conquerors();
            state.conqueror_count = Some(count);
            count
        })
    });
    counts
        .into_iter()
        .fold(None, |result: Option<String>, (role, actual)| {
            let expected = target_number_of_role_in_room(role.as_str(), room);
            if expected <= actual {
                return result;
            }
            result
                .map(|result| {
                    let result_prio = role_priority(room, result.as_str());
                    let role_prio = role_priority(room, role.as_str());
                    if role_prio > result_prio {
                        role.clone()
                    } else {
                        result
                    }
                })
                .or_else(|| Some(role))
        })
}

/// Run the creep according to the given role
pub fn run_role<'a>(role: &'a str, creep: &'a Creep) -> Task<'a> {
    trace!("Running creep {} by role {}", creep.name(), role);

    let task = match role {
        "upgrader" => upgrader::run(creep),
        "harvester" => harvester::run(creep),
        "builder" => builder::run(creep),
        "repairer" => repairer::run(creep),
        "gofer" => gofer::run(creep),
        "conqueror" => conqueror::run(creep),
        _ => unimplemented!(),
    };

    Task::new(move |state| {
        task.tick(state).map_err(|e| {
            let error = format!("Creep {} is idle: {:?}", creep.name(), e);
            warn!("{}", error);
            error
        })
    })
}

/// The higher the more important
pub fn role_priority<'a>(_room: &'a Room, role: &'a str) -> i8 {
    match role {
        "harvester" => 3,
        "gofer" => 2,
        "builder" => 1,
        "conqueror" => -1,
        _ => 0,
    }
}

pub fn count_roles_in_room<'a>(room: &'a Room) -> HashMap<String, i8> {
    let mut result: HashMap<String, i8> = [
        ("upgrader".into(), 0),
        ("harvester".into(), 0),
        ("builder".into(), 0),
        ("repairer".into(), 0),
        ("gofer".into(), 0),
    ]
    .into_iter()
    .cloned()
    .collect();
    // Also count the creeps spawning right now
    room.find(find::MY_SPAWNS)
        .into_iter()
        .filter_map(|s| {
            let role = js! {
                let spawn = @{s};
                let spawning = spawn.spawning;
                if (!spawning) {
                    return null;
                }
                let name = spawning.name;
                let role = Memory.creeps && Memory.creeps[name].role;
                return role;

            };
            let role = role.try_into();
            role.ok()
        })
        .for_each(|role| {
            result.entry(role).and_modify(|c| *c += 1).or_insert(1);
        });

    room.find(find::MY_CREEPS).into_iter().for_each(|c| {
        let role = c.memory().string("role").unwrap_or(None);
        if let Some(role) = role {
            result.entry(role).and_modify(|c| *c += 1).or_insert(1);
        }
    });
    result
}

pub fn count_conquerors() -> i8 {
    game::creeps::values()
        .into_iter()
        .filter(|c| c.memory().string("role").unwrap_or(None) == Some("conqueror".into()))
        .count() as i8
}

/// Max number of creeps of a given role in the given room
pub fn target_number_of_role_in_room<'a>(role: &'a str, room: &'a Room) -> i8 {
    let n_flags = game::flags::keys().len() as i8;
    let n_sources = room.find(find::SOURCES).len() as i8;
    let n_containers = js! {
        const room = @{room};
        return room.find(FIND_STRUCTURES, {
            filter: (s) => s.structureType == STRUCTURE_CONTAINER
        }).length;
    };
    let n_containers: i64 = n_containers.try_into().unwrap();
    let n_containers: i8 = n_containers as i8;
    const UPGRADER_COUNT: i8 = 2;
    const WORKER_COUNT: i8 = 2;
    match role {
        "upgrader" => n_containers.min(UPGRADER_COUNT),
        "harvester" => n_sources,
        "builder" => {
            if n_containers > 0 {
                WORKER_COUNT
            } else {
                WORKER_COUNT + UPGRADER_COUNT
            }
        }
        "repairer" => 0,            // Disable repairers for now
        "conqueror" => n_flags * 2, // TODO: make the closest room spawn it
        "gofer" => n_sources.min(n_containers as i8),
        _ => unimplemented!(),
    }
}

pub fn spawn_config_by_role(role: &str) -> SpawnConfig {
    SpawnConfig {
        basic_body: basic_role_parts(role),
        body_extension: role_part_scale(role),
        body_max: role_part_max(role),
    }
}

/// The minimum parts required by the role
fn basic_role_parts<'a>(role: &'a str) -> Vec<Part> {
    match role {
        "harvester" => vec![Part::Move, Part::Work, Part::Carry, Part::Work],
        "conqueror" => vec![
            Part::Move,
            Part::Work,
            Part::Carry,
            Part::Claim,
            Part::Move,
            Part::Move,
        ],
        "gofer" => vec![Part::Move, Part::Carry],
        "upgrader" | "builder" | "repairer" => {
            vec![Part::Move, Part::Move, Part::Carry, Part::Work]
        }
        _ => unimplemented!(),
    }
}

/// Intended parts to be appended to 'role_parts'
fn role_part_scale<'a>(role: &'a str) -> Vec<Part> {
    match role {
        "harvester" => vec![Part::Work],
        "conqueror" => vec![],
        "gofer" => vec![Part::Move, Part::Carry],
        _ => vec![Part::Move, Part::Carry, Part::Work],
    }
}

/// The largest a creep of role `role` may be
fn role_part_max(role: &str) -> Option<usize> {
    match role {
        "harvester" => Some(8),
        "gofer" => Some(24),
        "builder" | "repairer" | "upgrader" => Some(24),
        _ => None,
    }
}

