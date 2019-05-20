use super::{super::bt::*, builder, conqueror, gofer, harvester, lrh, upgrader};
use screeps::{
    constants::find,
    game,
    objects::{Creep, Room},
    Part,
};
use stdweb::unstable::TryInto;

pub struct SpawnConfig {
    pub basic_body: Vec<Part>,
    pub body_extension: Vec<Part>,
    pub body_max: Option<usize>,
}

pub const ALL_ROLES: &'static [&'static str] = &[
    "upgrader",
    "harvester",
    "builder",
    "gofer",
    "lrh",
    "conqueror",
];

// TODO: return an array of all roles to spawn in order of priority
/// Get the next target role in the given room
pub fn next_role<'a>(state: &'a mut GameState, room: &'a Room) -> Option<String> {
    let conqueror_count = state.global_conqueror_count();
    let counts = state.count_creeps_in_room(room);
    counts.insert("conqueror".into(), conqueror_count);

    counts
        .into_iter()
        .fold(None, |result: Option<String>, (role, actual)| {
            let expected = target_number_of_role_in_room(role, room);
            if expected <= *actual {
                return result;
            }
            result
                .map(|result| {
                    let result_prio = role_priority(room, result.as_str());
                    let role_prio = role_priority(room, role);
                    if role_prio > result_prio {
                        role.clone()
                    } else {
                        result.into()
                    }
                })
                .or_else(|| Some(role.to_string()))
        })
}

/// Run the creep according to the given role
pub fn run_role<'a>(role: &str, creep: &'a Creep) -> Task<'a> {
    trace!("Running creep {} by role {}", creep.name(), role);

    let task = match role {
        "upgrader" => upgrader::run(creep),
        "harvester" => harvester::run(creep),
        "builder" => builder::run(creep),
        "gofer" => gofer::run(creep),
        "conqueror" => conqueror::run(creep),
        "lrh" => lrh::run(creep),
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
        "lrh" => -1,
        "conqueror" => -2,
        _ => 0,
    }
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
    let n_constructions = (room.find(find::CONSTRUCTION_SITES).len()) as i8;
    const UPGRADER_COUNT: i8 = 2;
    const WORKER_COUNT: i8 = 1;
    match role {
        "upgrader" => n_containers.min(UPGRADER_COUNT),
        "harvester" => n_sources,
        "builder" => {
            let target_builders = n_constructions.min(1) + WORKER_COUNT;
            if n_containers > 0 {
                target_builders
            } else {
                target_builders + UPGRADER_COUNT
            }
        }
        "conqueror" => n_flags * 2, // TODO: make the closest room spawn it
        "lrh" => 0,                 // TODO: reenable once the cpu budget can afford it
        "gofer" => n_sources.min(n_containers as i8),
        _ => unimplemented!(),
    }
}

pub fn spawn_config_by_role(room: &Room, role: &str) -> SpawnConfig {
    SpawnConfig {
        basic_body: basic_role_parts(room, role),
        body_extension: role_part_scale(room, role),
        body_max: role_part_max(room, role),
    }
}

/// The minimum parts required by the role
fn basic_role_parts<'a>(_room: &Room, role: &'a str) -> Vec<Part> {
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
        "lrh" => vec![
            Part::Move,
            Part::Move,
            Part::Carry,
            Part::Work,
            Part::Attack,
        ],
        "upgrader" | "builder" => vec![Part::Move, Part::Move, Part::Carry, Part::Work],
        _ => unimplemented!(),
    }
}

/// Intended parts to be appended to 'role_parts'
fn role_part_scale<'a>(_room: &Room, role: &'a str) -> Vec<Part> {
    match role {
        "harvester" => vec![Part::Work],
        "conqueror" => vec![],
        "gofer" => vec![Part::Move, Part::Carry],
        "lrh" => vec![
            Part::Move,
            Part::Carry,
            Part::Work,
            Part::Move,
            Part::Attack,
        ],
        _ => vec![Part::Move, Part::Carry, Part::Work],
    }
}

/// The largest a creep of role `role` may be
fn role_part_max(room: &Room, role: &str) -> Option<usize> {
    let level = room.controller().map(|c| c.level()).unwrap_or(0);

    let worker_count = || {
        if level < 6 {
            16
        } else if level < 8 {
            24
        } else {
            32
        }
    };

    match role {
        "harvester" => Some(8),
        "lrh" | "gofer" => Some(worker_count() * 2),
        "builder" | "upgrader" => Some(worker_count()),
        _ => None,
    }
}
