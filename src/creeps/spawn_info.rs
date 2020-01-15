use super::roles::Role;
use crate::prelude::WorldPosition;
use crate::state::GameState;
use arrayvec::ArrayVec;
use screeps::{
    constants::find,
    objects::{HasStore, Room, StructureContainer},
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
        Role::Defender => 4,
        Role::Harvester => 3,
        Role::Gofer => 2,
        Role::Worker => 1,
        Role::Scout => -1,
        Role::Lrh => -2,
        Role::Conqueror => -3,
        Role::Lrw => -4,
        _ => 0,
    }
}

/// Max number of creeps of a given role in the given room
pub fn target_number_of_role_in_room<'a>(role: Role, room: &'a Room, game_state: &GameState) -> i8 {
    let level = room.controller().map(|l| l.level()).unwrap_or(0);
    let room_pos = WorldPosition::from(room);
    let n_flags = game_state
        .expansion
        .iter()
        .filter(|w| w.dist(room_pos) <= 10)
        .count()
        .min(255) as i8;
    let n_sources = room.find(find::SOURCES).len() as i8;
    let containers = js! {
        const room = @{room};
        return room.find(FIND_STRUCTURES, {
            filter: (s) => s.structureType == STRUCTURE_CONTAINER
        });
    };
    let containers: Vec<StructureContainer> = containers.try_into().unwrap();
    let energy_in_containers = containers.iter().map(|c| c.energy()).sum::<u32>();
    let n_containers = containers.len() as i8;
    let n_constructions = (room.find(find::CONSTRUCTION_SITES).len()) as i8;
    const UPGRADER_COUNT: i8 = 1;
    match role {
        Role::Defender => room.find(find::HOSTILE_CREEPS).len().min(1) as i8,
        Role::Upgrader => n_containers.min(UPGRADER_COUNT),
        Role::Harvester => n_sources,
        Role::Worker => {
            let mut target_workers = n_constructions.min(2);
            if n_containers > 0 {
                if energy_in_containers > 1000 {
                    target_workers += 3;
                }
                target_workers += UPGRADER_COUNT
            }

            target_workers
        }
        Role::Conqueror => n_flags.max(1),
        Role::Lrh => {
            if n_containers == 0 {
                0
            } else {
                (level * 2).min(4) as i8
            }
        }
        Role::Gofer => n_sources.min(n_containers as i8),
        Role::Lrw => n_flags.max(1),
        Role::Scout => 1,
        Role::Unknown => 0,
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
        Role::Harvester => [Part::Move, Part::Work, Part::Carry, Part::Work].iter(),
        Role::Conqueror => [Part::Move, Part::Claim].iter(),
        Role::Gofer => [Part::Move, Part::Carry].iter(),
        Role::Lrh => [Part::Move, Part::Move, Part::Carry, Part::Work].iter(),
        Role::Upgrader | Role::Worker => [Part::Move, Part::Carry, Part::Work].iter(),
        Role::Lrw => [Part::Move, Part::Move, Part::Carry, Part::Work].iter(),
        Role::Scout => [Part::Move].iter(),
        Role::Defender => [Part::Move, Part::Attack].iter(),
        Role::Unknown => [].iter(),
    };
    it.map(|x| *x).collect()
}

/// Intended parts to be appended to 'role_parts'
fn role_part_scale<'a>(_room: &Room, role: Role) -> BodyCollection {
    let it = match role {
        Role::Harvester => [Part::Work].iter(),
        Role::Scout | Role::Conqueror => [].iter(),
        Role::Gofer => [Part::Move, Part::Carry, Part::Carry].iter(),
        Role::Lrh => [Part::Move, Part::Carry, Part::Work, Part::Move].iter(),
        Role::Defender => [Part::Attack, Part::Move].iter(),
        _ => [Part::Move, Part::Carry, Part::Work].iter(),
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
            36
        }
    };

    let result = match role {
        Role::Harvester => Some(8),
        Role::Lrw | Role::Lrh | Role::Worker | Role::Upgrader => Some(worker_count),
        Role::Conqueror => None,
        Role::Scout => None,
        Role::Gofer => Some(worker_count * 2),
        Role::Defender => None,
        Role::Unknown => None,
    };
    result.map(|x| x.min(50))
}
