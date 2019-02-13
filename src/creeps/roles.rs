use super::super::bt::*;
use super::builder;
use super::gofer;
use super::harvester;
use super::repairer;
use super::upgrader;
use screeps::{
    game,
    objects::{Creep, Room, RoomObjectProperties},
    Part,
};
use std::collections::HashMap;
use stdweb::unstable::TryInto;

/// Get the next target role in the given room
pub fn next_role<'a>(room: &'a Room) -> Option<String> {
    let counts = count_roles_in_room(room);
    counts.into_iter().find_map(|(role, actual)| {
        let expected = target_number_of_role_in_room(role.as_str(), room);
        if actual < expected {
            Some(role)
        } else {
            None
        }
    })
}

/// Run the creep according to the given role
pub fn run_role<'a>(role: &'a str, creep: &'a Creep) -> ExecutionResult {
    trace!("Running creep {} by role {}", creep.name(), role);

    let result = match role {
        "upgrader" => upgrader::run(creep),
        "harvester" => harvester::run(creep),
        "builder" => builder::run(creep),
        "repairer" => repairer::run(creep),
        "gofer" => gofer::run(creep),
        _ => unimplemented!(),
    };

    if result.is_err() {
        warn!("Running creep {} failed", creep.name());
    }

    Ok(())
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
    game::creeps::values()
        .into_iter()
        .filter(|c| c.room().name() == room.name())
        .for_each(|c| {
            let role = c.memory().string("role").unwrap_or(None);
            if let Some(role) = role {
                *result.entry(role).or_insert(0) += 1;
            }
        });
    result
}

pub fn target_number_of_role_in_room<'a>(role: &'a str, room: &'a Room) -> i8 {
    let n_sources = js! {
        const room = @{room};
        const sources = room.find(FIND_SOURCES) || [];
        return sources && sources.length;
    };

    let n_sources = n_sources.try_into().unwrap_or(0);

    match role {
        "upgrader" => 1,
        "harvester" => n_sources,
        "builder" => 1,
        "repairer" => 1,
        "gofer" => n_sources,
        _ => unimplemented!(),
    }
}

pub struct SpawnConfig {
    pub basic_body: [Part; 4],
    pub body_extension: Vec<Part>,
}

pub fn spawn_config_by_role(role: &str) -> SpawnConfig {
    SpawnConfig {
        basic_body: basic_role_parts(role),
        body_extension: role_part_scale(role),
    }
}

fn basic_role_parts<'a>(role: &'a str) -> [Part; 4] {
    match role {
        "harvester" => [Part::Move, Part::Work, Part::Carry, Part::Work],
        "upgrader" | "builder" | "repairer" | "gofer" => {
            [Part::Move, Part::Move, Part::Carry, Part::Work]
        }
        _ => unimplemented!(),
    }
}

/// Intended parts to be appended to 'role_parts'
fn role_part_scale<'a>(role: &'a str) -> Vec<Part> {
    match role {
        "harvester" => vec![Part::Work],
        "gofer" => vec![Part::Move, Part::Carry],
        _ => vec![Part::Move, Part::Carry, Part::Work],
    }
}
