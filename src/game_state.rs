use crate::creeps::roles::string_to_role;
use screeps::{find, game, Room};
use std::collections::HashMap;
use stdweb::unstable::TryInto;

#[derive(Debug, Clone, Default)]
pub struct GameState {
    /// CPU bucket available this tick
    pub cpu_bucket: Option<i32>,
    /// Lazily countable global conqueror creep count
    pub conqueror_count: Option<i8>,
    /// Count creeps in rooms
    /// Structure: room -> role -> n
    creep_count_by_room: HashMap<String, HashMap<&'static str, i8>>,
}

impl GameState {
    pub fn count_creeps_in_room<'b>(&mut self, room: &'b Room) -> &mut HashMap<&'static str, i8> {
        let name = room.name();
        self.creep_count_by_room
            .entry(name)
            .or_insert_with(|| count_roles_in_room(room))
    }

    /// Lazily computes the global number of conqueror creeps
    pub fn global_conqueror_count(&mut self) -> i8 {
        self.conqueror_count.unwrap_or_else(|| {
            // Lazily count conquerors
            let count = count_conquerors();
            self.conqueror_count = Some(count);
            count
        })
    }
}

fn count_roles_in_room(room: &Room) -> HashMap<&'static str, i8> {
    let mut result: HashMap<&'static str, i8> = [
        ("upgrader", 0),
        ("harvester", 0),
        ("builder", 0),
        ("repairer", 0),
        ("gofer", 0),
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
        .map(|role| string_to_role(role))
        .for_each(|role| {
            result.entry(role).and_modify(|c| *c += 1).or_insert(1);
        });

    room.find(find::MY_CREEPS).into_iter().for_each(|c| {
        let role = c.memory().string("role").unwrap_or(None);
        if let Some(role) = role {
            let role = string_to_role(role);
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

