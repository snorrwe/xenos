use super::{conqueror, gofer, harvester, lrh, upgrader, worker};
use crate::prelude::*;
use crate::rooms::manhatten_distance;
use arrayvec::ArrayVec;
use screeps::{
    constants::find,
    game,
    objects::{Creep, HasPosition, Room},
    Part,
};
use std::fmt::{self, Display, Formatter};
use stdweb::unstable::TryInto;

pub type BodyCollection = ArrayVec<[Part; 16]>;

pub struct SpawnConfig {
    pub basic_body: BodyCollection,
    pub body_extension: BodyCollection,
    pub body_max: Option<usize>,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, Hash, Eq, PartialEq)]
#[repr(u8)]
pub enum Role {
    Unknown = 0,
    Upgrader = 1,
    Harvester = 2,
    Worker = 3,
    Gofer = 4,
    Lrh = 5,
    Conqueror = 6,
}

impl From<u8> for Role {
    fn from(item: u8) -> Self {
        match item {
            0 => Role::Unknown,
            1 => Role::Upgrader,
            2 => Role::Harvester,
            3 => Role::Worker,
            4 => Role::Gofer,
            5 => Role::Lrh,
            6 => Role::Conqueror,
            _ => unimplemented!("Role {} is not unimplemented!", item),
        }
    }
}

impl Display for Role {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        let name = match self {
            Role::Unknown => "Unknown",
            Role::Upgrader => "Upgrader",
            Role::Harvester => "Harvester",
            Role::Worker => "Worker",
            Role::Gofer => "Gofer",
            Role::Lrh => "Lrh",
            Role::Conqueror => "Conqueror",
        };
        write!(f, "{}", name)?;
        Ok(())
    }
}

impl Role {
    pub fn all_roles() -> &'static [Self] {
        use self::Role::*;
        &[Upgrader, Harvester, Worker, Gofer, Lrh, Conqueror]
    }
}

// TODO: return an array of all roles to spawn in order of priority
/// Get the next target role in the given room
pub fn next_role<'a>(state: &'a mut GameState, room: &'a Room) -> Option<Role> {
    let conqueror_count = state.global_conqueror_count();
    let counts = state.count_creeps_in_room(room);
    counts.insert(Role::Conqueror, conqueror_count);

    counts
        .into_iter()
        .fold(None, |result: Option<Role>, (role, actual)| {
            let expected = target_number_of_role_in_room(*role, room);
            if expected <= *actual {
                return result;
            }
            result
                .map(|result| {
                    let result_prio = role_priority(room, result);
                    let role_prio = role_priority(room, *role);
                    if role_prio > result_prio {
                        *role
                    } else {
                        result
                    }
                })
                .or_else(|| Some(*role))
        })
}

/// Run the creep according to the given role
pub fn run_role<'a>(role: Role, creep: &'a Creep) -> Task<'a, GameState> {
    trace!("Running creep {} by role {}", creep.name(), role);

    let task = match role {
        Role::Upgrader => upgrader::run(creep),
        Role::Harvester => harvester::run(creep),
        Role::Worker => worker::run(creep),
        Role::Gofer => gofer::run(creep),
        Role::Conqueror => conqueror::run(creep),
        Role::Lrh => lrh::run(creep),
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
pub fn role_priority<'a>(_room: &'a Room, role: Role) -> i8 {
    match role {
        Role::Harvester => 3,
        Role::Gofer => 2,
        Role::Worker => 1,
        Role::Lrh => -1,
        Role::Conqueror => -2,
        _ => 0,
    }
}

/// Max number of creeps of a given role in the given room
pub fn target_number_of_role_in_room<'a>(role: Role, room: &'a Room) -> i8 {
    let room_name = room.name();
    let n_flags = game::flags::values()
        .into_iter()
        .filter(|flag| {
            let rn = flag.pos().room_name();
            manhatten_distance(&room_name, &rn)
                .map(|d| d < 10)
                .unwrap_or_else(|e| {
                    error!(
                        "Failed to calculate distance from {:?} to {:?}, {:?}",
                        &room_name, &rn, e
                    );
                    false
                })
        })
        .count() as i8;
    let n_sources = room.find(find::SOURCES).len() as i8;
    let n_containers = js! {
        const room = @{room};
        return room.find(FIND_STRUCTURES, {
            filter: (s) => s.structureType == STRUCTURE_CONTAINER
        }).length;
    };
    let n_containers: i64 = n_containers.try_into().unwrap();
    let n_containers = n_containers as i8;
    let n_constructions = (room.find(find::CONSTRUCTION_SITES).len()) as i8;
    const UPGRADER_COUNT: i8 = 1;
    const WORKER_COUNT: i8 = 1;
    match role {
        Role::Upgrader => n_containers.min(UPGRADER_COUNT),
        Role::Harvester => n_sources,
        Role::Worker => {
            let target_workers = n_constructions.min(1) + WORKER_COUNT;
            if n_containers > 0 {
                target_workers
            } else {
                target_workers + UPGRADER_COUNT
            }
        }
        Role::Conqueror => n_flags * 2, // TODO: make the closest room spawn it
        Role::Lrh => 8,
        Role::Gofer => n_sources.min(n_containers as i8),
        _ => unimplemented!(),
    }
}

pub fn spawn_config_by_role(room: &Room, role: Role) -> SpawnConfig {
    SpawnConfig {
        basic_body: basic_role_parts(room, role),
        body_extension: role_part_scale(room, role),
        body_max: role_part_max(room, role),
    }
}

/// The minimum parts required by the role
fn basic_role_parts<'a>(_room: &Room, role: Role) -> BodyCollection {
    match role {
        Role::Harvester => [Part::Move, Part::Work, Part::Carry, Part::Work]
            .into_iter()
            .map(|x| *x)
            .collect::<BodyCollection>(),
        Role::Conqueror => [
            Part::Move,
            Part::Work,
            Part::Carry,
            Part::Claim,
            Part::Move,
            Part::Move,
        ]
        .into_iter()
        .map(|x| *x)
        .collect::<BodyCollection>(),
        Role::Gofer => [Part::Move, Part::Carry]
            .into_iter()
            .map(|x| *x)
            .collect::<BodyCollection>(),
        Role::Lrh => [Part::Move, Part::Move, Part::Carry, Part::Work]
            .into_iter()
            .map(|x| *x)
            .collect::<BodyCollection>(),
        Role::Upgrader | Role::Worker => [Part::Move, Part::Move, Part::Carry, Part::Work]
            .into_iter()
            .map(|x| *x)
            .collect::<BodyCollection>(),
        Role::Unknown => [].into_iter().map(|x| *x).collect::<BodyCollection>(),
    }
}

/// Intended parts to be appended to 'role_parts'
fn role_part_scale<'a>(_room: &Room, role: Role) -> BodyCollection {
    match role {
        Role::Harvester => [Part::Work].into_iter().map(|x| *x).collect(),
        Role::Conqueror => [].into_iter().map(|x| *x).collect(),
        Role::Gofer => [Part::Move, Part::Carry].into_iter().map(|x| *x).collect(),
        Role::Lrh => [Part::Move, Part::Carry, Part::Work, Part::Move]
            .into_iter()
            .map(|x| *x)
            .collect(),
        _ => [Part::Move, Part::Carry, Part::Work]
            .into_iter()
            .map(|x| *x)
            .collect(),
    }
}

/// The largest a creep of role `role` may be
fn role_part_max(room: &Room, role: Role) -> Option<usize> {
    let level = room.controller().map(|c| c.level()).unwrap_or(0);

    let worker_count = {
        if level < 5 {
            16
        } else if level < 8 {
            24
        } else {
            48
        }
    };

    let result = match role {
        Role::Harvester => Some(8),
        Role::Lrh | Role::Gofer | Role::Worker | Role::Upgrader | Role::Conqueror => {
            Some(worker_count)
        }
        Role::Unknown => None,
    };
    result.map(|x| x.min(50))
}

