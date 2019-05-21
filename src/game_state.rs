use crate::creeps::roles::ALL_ROLES;
use crate::creeps::{CREEP_ROLE, HOME_ROOM};
use screeps::{game, memory, raw_memory, Room};
use serde_json::{self, Map, Value};
use std::collections::HashMap;

pub type CreepMemory = HashMap<String, Map<String, Value>>;

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
/// Holds information about the global state of the game
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
    #[serde(skip_serializing)]
    #[serde(default)]
    pub memory_segment: Option<u32>,
    /// Where to save this state when dropping
    /// Defaults to saving to "game_state"
    #[serde(skip_serializing)]
    #[serde(default)]
    pub memory_route: Option<String>,
    #[serde(skip_serializing)]
    #[serde(default)]
    pub save_to_memory: Option<bool>,

    /// Holds the creep memory objects
    /// Structure: name -> data
    creep_memory: CreepMemory,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ScoutInfo {
    pub iff: RoomIFF,
    pub n_sources: u8,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
#[repr(u8)]
pub enum RoomIFF {
    Unknown = 0,

    Hostile,
    Neutral,
    Friendly,
    NoMansLand,
    Keepers,
}

pub struct CreepName<'a>(pub &'a str);

impl Default for RoomIFF {
    fn default() -> Self {
        RoomIFF::Unknown
    }
}

js_deserializable!(GameState);
js_deserializable!(ScoutInfo);
js_deserializable!(RoomIFF);

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
        use stdweb::serde::Serde;
        memory::root().set(route, Serde(&self));

        if let Some(segment) = self.memory_segment {
            let data = serde_json::to_string(self);

            match data {
                Ok(data) => {
                    raw_memory::set_segment(segment, data.as_str());
                }
                Err(e) => {
                    error!("Failed to serialize game_state {:?}", e);
                }
            }
        }
    }
}

impl GameState {
    #[allow(dead_code)]
    pub fn read_from_memory_or_default() -> Self {
        use stdweb::unstable::TryFrom;

        let result = js! {
            return Memory.game_state; // TODO pass key
        };

        Self::try_from(result).unwrap_or_default()
    }

    pub fn read_from_segment_or_default(segment: u32) -> Self {
        raw_memory::get_segment(segment)
            .and_then(|string| {
                serde_json::from_str(&string)
                    .map_err(|e| {
                        error!("Failed to deserialize game_state from segment {:?}", e);
                    })
                    .ok()
            })
            .unwrap_or_default()
    }

    pub fn count_creeps_in_room<'b>(&mut self, room: &'b Room) -> &mut HashMap<String, i8> {
        let name = room.name();
        // TODO: use cached value
        let count = self
            .count_roles_in_room(room)
            .iter()
            .map(|(k, v)| (k.to_string(), *v))
            .collect();
        self.creep_count_by_room.insert(name.clone(), count);
        self.creep_count_by_room.get_mut(&name).unwrap()
    }

    /// Lazily computes the global number of conqueror creeps
    pub fn global_conqueror_count(&mut self) -> i8 {
        self.conqueror_count.unwrap_or_else(|| {
            // Lazily count conquerors
            let count = self.count_conquerors();
            self.conqueror_count = Some(count);
            count
        })
    }

    /// Get an entry in the creep's memory
    /// Inserts and empty map in the creep's name if none is found
    pub fn creep_memory_entry(&mut self, name: CreepName) -> &mut serde_json::Map<String, Value> {
        self.creep_memory
            .entry(name.0.to_owned())
            .or_insert_with(|| serde_json::Map::default())
    }

    pub fn creep_memory_get(&self, creep: CreepName) -> Option<&serde_json::Map<String, Value>> {
        self.creep_memory.get(creep.0)
    }

    pub fn creep_memory_bool(&self, creep: CreepName, key: &str) -> bool {
        self.creep_memory_get(creep)
            .and_then(|map| map.get(key))
            .map(|x| x.as_bool().unwrap_or(false))
            .unwrap_or(false)
    }

    pub fn creep_memory_string<'a>(&'a self, creep: CreepName, key: &str) -> Option<&'a str> {
        self.creep_memory_get(creep)
            .and_then(|map| map.get(key))
            .and_then(|x| x.as_str())
    }

    #[allow(dead_code)]
    pub fn creep_memory_i64(&self, creep: CreepName, key: &str) -> Option<i64> {
        self.creep_memory_get(creep)
            .and_then(|map| map.get(key))
            .and_then(|x| x.as_i64())
    }

    pub fn cleanup_memory(&mut self) -> Result<(), Box<::std::error::Error>> {
        trace!("Cleaning memory");

        let alive_creeps: std::collections::HashSet<String> =
            screeps::game::creeps::keys().into_iter().collect();

        let to_delete = {
            self.creep_memory
                .keys()
                .filter(|k| !alive_creeps.contains(*k))
                .cloned()
                .collect::<Vec<_>>()
        };

        for mem_name in to_delete.iter() {
            debug!("cleaning up creep memory of dead creep {}", mem_name);
            self.creep_memory.remove(mem_name);
            screeps::memory::root().path_del(&format!("creeps.{}", mem_name));
        }

        info!("Cleaned up memory");

        Ok(())
    }

    pub fn count_conquerors(&self) -> i8 {
        game::creeps::values()
            .into_iter()
            .filter_map(|creep| self.creep_memory_string(CreepName(&creep.name()), CREEP_ROLE))
            .map(|role| role == "conqueror")
            .count() as i8
    }

    fn count_roles_in_room(&self, room: &Room) -> HashMap<&'static str, i8> {
        let mut result: HashMap<&'static str, i8> = ALL_ROLES.iter().map(|x| (*x, 0)).collect();

        self.creep_memory
            .iter()
            .filter(|(k, _v)| {
                self.creep_memory_string(CreepName(k), HOME_ROOM)
                    .map(|r| r == room.name())
                    .unwrap_or(false)
            })
            .filter_map(|(k, _v)| self.creep_memory_string(CreepName(k), CREEP_ROLE))
            .for_each(|role| {
                if let Some(count) = result.get_mut(role) {
                    *count += 1
                } else {
                    error!(
                        "Expected {} to be already in count! Skipping counting!",
                        role
                    );
                };
            });

        result
    }
}
