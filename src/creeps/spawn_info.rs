use super::roles::Role;
use crate::rooms::manhatten_distance;
use arrayvec::ArrayVec;
use screeps::{
    constants::find,
    game,
    objects::{HasPosition, Room},
    Part,
};
use stdweb::unstable::TryInto;

pub type BodyCollection = ArrayVec<[Part; 16]>;

pub struct SpawnConfig {
    pub basic_body: BodyCollection,
    pub body_extension: BodyCollection,
    pub body_max: Option<usize>,
}

/// The higher the more important
pub fn role_priority<'a>(_room: &'a Room, role: Role) -> i8 {
    match role {
        Role::Harvester => 3,
        Role::Gofer => 2,
        Role::Worker => 1,
        Role::Lrh => -1,
        Role::Conqueror => -2,
        Role::Lrw => -3,
        _ => 0,
    }
}

/// Max number of creeps of a given role in the given room
pub fn target_number_of_role_in_room<'a>(role: Role, room: &'a Room) -> i8 {
    let level = room.controller().map(|l| l.level()).unwrap_or(0);
    let room_name = room.name();
    let n_flags = game::flags::values()
        .into_iter()
        .filter(|flag| {
            let rn = flag.pos().room_name();
            manhatten_distance(&room_name, &rn)
                .map(|d| d <= 5)
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
        Role::Conqueror => n_flags, // TODO: make the closest room spawn it
        Role::Lrh => level.max(4) as i8, // TODO: scale with avialable rooms
        Role::Gofer => n_sources.min(n_containers as i8),
        Role::Lrw => n_flags * 2,
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
    let it = match role {
        Role::Harvester => [Part::Move, Part::Work, Part::Carry, Part::Work].into_iter(),
        Role::Conqueror => [
            Part::Move,
            Part::Work,
            Part::Carry,
            Part::Claim,
            Part::Move,
            Part::Move,
        ]
        .into_iter(),
        Role::Gofer => [Part::Move, Part::Carry].into_iter().into_iter(),
        Role::Lrh => [Part::Move, Part::Move, Part::Carry, Part::Work].into_iter(),
        Role::Upgrader | Role::Worker | Role::Lrw => {
            [Part::Move, Part::Move, Part::Carry, Part::Work].into_iter()
        }
        Role::Unknown => [].into_iter(),
    };
    it.map(|x| *x).collect()
}

/// Intended parts to be appended to 'role_parts'
fn role_part_scale<'a>(_room: &Room, role: Role) -> BodyCollection {
    let it = match role {
        Role::Harvester => [Part::Work].into_iter(),
        Role::Conqueror => [].into_iter(),
        Role::Gofer => [Part::Move, Part::Carry].into_iter(),
        Role::Lrh => [Part::Move, Part::Carry, Part::Work, Part::Move].into_iter(),
        _ => [Part::Move, Part::Carry, Part::Work].into_iter(),
    };
    it.map(|x| *x).collect()
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
        Role::Lrw | Role::Lrh | Role::Worker | Role::Upgrader | Role::Conqueror => {
            Some(worker_count)
        }
        Role::Gofer => Some((worker_count as f32 * 1.5) as usize),
        Role::Unknown => None,
    };
    result.map(|x| x.min(50))
}

