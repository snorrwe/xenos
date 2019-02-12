use super::super::bt::*;
use super::builder;
use super::gofer;
use super::harvester;
use super::repairer;
use super::upgrader;
use screeps::game;
use screeps::objects::{Creep, Room, RoomObjectProperties};
use std::collections::HashMap;

/// Get the next target role in the given room
pub fn next_role<'a>(room: &'a Room) -> Option<String> {
    let counts = count_roles_in_room(room);
    counts.into_iter().find_map(|(role, actual)| {
        let expected = target_number_of_role_in_room(role.as_str());
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

pub fn target_number_of_role_in_room<'a>(role: &'a str) -> i8 {
    match role {
        "upgrader" => 2,
        "harvester" => 2, // TODO smarter harvester distribution
        "builder" => 2,
        "repairer" => 1,
        "gofer" => 1,
        _ => unimplemented!(),
    }
}
