use crate::creeps::roles::{string_to_role, ALL_ROLES};
use screeps::{find, game, memory, Creep, Room};
use serde_json::{self, Map, Value};
use std::collections::HashMap;
use stdweb::unstable::TryInto;

pub type CreepMemory = HashMap<String, Map<String, Value>>;

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

impl Default for RoomIFF {
    fn default() -> Self {
        RoomIFF::Unknown
    }
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

    pub fn creep_memory_entry(&mut self, name: String) -> &mut serde_json::Map<String, Value> {
        self.creep_memory
            .entry(name)
            .or_insert_with(|| serde_json::Map::default())
    }

    pub fn creep_memory_bool(&mut self, creep: &Creep, key: &str) -> bool {
        self.creep_memory_entry(creep.name())
            .get(key)
            .map(|x| x.as_bool().unwrap_or(false))
            .unwrap_or(false)
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
            screeps::memory::root().del(&format!("creeps.{}", mem_name));
        }

        debug!("Cleaned up memory");

        Ok(())
    }

    pub fn count_conquerors(&mut self) -> i8 {
        game::creeps::values()
            .into_iter()
            .filter_map(|creep| {
                let memory = { self.creep_memory_entry(creep.name()) };
                memory
                    .get("role")
                    .and_then(|x| x.as_str())
                    .map(|x| x.to_string())
            })
            .map(|role| role == "conqueror")
            .count() as i8
    }

    fn count_roles_in_room(&mut self, room: &Room) -> HashMap<&'static str, i8> {
        let mut result: HashMap<&'static str, i8> =
            ALL_ROLES.iter().cloned().map(|x| (x, 0)).collect();
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

        room.find(find::MY_CREEPS).into_iter().for_each(|creep| {
            let memory = self.creep_memory_entry(creep.name());
            let role = memory.get("role").and_then(|r| r.as_str());
            if let Some(role) = role {
                let role = string_to_role(role.to_string());
                result.entry(role).and_modify(|c| *c += 1).or_insert(1);
            }
        });

        result
    }
}

