use crate::creeps::roles::string_to_role;
use screeps::{find, game, memory, Room};
use serde_json;
use std::collections::HashMap;
use stdweb::unstable::TryInto;

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct GameState {
    /// CPU bucket available this tick
    #[serde(skip_serializing)]
    #[serde(default)]
    pub cpu_bucket: Option<i32>,

    /// Lazily countable global conqueror creep count
    #[serde(skip_serializing)]
    #[serde(default)]
    pub conqueror_count: Option<i8>,

    /// Count creeps in rooms
    /// Structure: room -> role -> n
    #[serde(skip_serializing)]
    #[serde(default)]
    creep_count_by_room: HashMap<String, HashMap<String, i8>>,

    /// Information about rooms
    /// Structure: room -> info
    pub scout_intel: HashMap<String, ScoutInfo>,

    /// Number of LRH per room
    /// In directions: [N, W, S, E]
    pub long_range_harvesters: HashMap<String, [u8; 4]>,

    /// Where to save this state when dropping
    /// Defaults to saving to "game_state"
    pub memory_route: Option<String>,
    pub save_to_memory: Option<bool>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ScoutInfo {
    hostile: bool,
    player_controlled: bool,
    n_sources: u8,
}

impl Drop for GameState {
    fn drop(&mut self) {
        if let Some(false) = self.save_to_memory {
            return;
        }
        debug!("Saving GameState");
        let route = self
            .memory_route
            .as_ref()
            .map(|x| x.as_str())
            .unwrap_or("game_state");
        memory::root().set(
            route,
            serde_json::to_string(self).expect("Failed to serialize"),
        );
    }
}

impl GameState {
    pub fn read_from_memory_or_default() -> Self {
        memory::root()
            .string("game_state")
            .map_err(|e| error!("Failed to read game_state from memory {:?}", e))
            .unwrap_or(None)
            .and_then(|s| serde_json::from_str(s.as_str()).ok())
            .unwrap_or_else(|| GameState::default())
    }

    pub fn count_creeps_in_room<'b>(&mut self, room: &'b Room) -> &mut HashMap<String, i8> {
        let name = room.name();
        self.creep_count_by_room.entry(name).or_insert_with(|| {
            count_roles_in_room(room)
                .iter()
                .map(|(k, v)| (k.to_string(), *v))
                .collect()
        })
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

