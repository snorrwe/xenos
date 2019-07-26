use crate::creeps::roles::Role;
use crate::creeps::{CreepExecutionStats, CREEP_ROLE, HOME_ROOM, TASK};
use crate::prelude::*;
use num::ToPrimitive;
use screeps::{raw_memory, Room};
use serde_json::{self, Map, Value};
use std::collections::HashMap;
use std::error::Error;

pub type CreepMemory = HashMap<String, Map<String, Value>>;

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
/// Holds information about the global state of the game
pub struct GameState {
    /// CPU bucket available this tick
    #[serde(skip_serializing)]
    #[serde(default)]
    pub cpu_bucket: Option<i16>,

    /// Count creeps in rooms
    /// Structure: room -> role -> n
    #[serde(skip_serializing)]
    #[serde(default)]
    creep_count_by_room: HashMap<String, HashMap<Role, i8>>,

    /// Information about rooms
    /// Structure: room -> info
    pub scout_intel: HashMap<String, ScoutInfo>,

    /// Number of LRH per room
    /// In directions: [N, W, S, E]
    pub long_range_harvesters: HashMap<String, [u8; 4]>,

    /// Where to save this state when dropping
    /// Defaults to 0
    #[serde(skip_serializing)]
    #[serde(default)]
    pub memory_segment: Option<u8>,
    #[serde(skip_serializing)]
    #[serde(default)]
    pub save_to_memory: Option<bool>,

    /// Holds the creep memory objects
    /// Structure: name -> data
    creep_memory: CreepMemory,

    /// Data about creep task execution in a tick
    #[serde(skip_serializing)]
    #[serde(skip_deserializing)]
    #[serde(default)]
    pub creep_stats: CreepExecutionStats,
}

impl TaskInput for GameState {
    fn cpu_bucket(&self) -> Option<i16> {
        self.cpu_bucket
    }
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

/// Used to make sure the right string is passed
/// to the right parameter when accessing creep memory
/// It's deliberately verbose
#[derive(Debug, Clone, Copy)]
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

        let segment = self.memory_segment.unwrap_or(0);

        match serde_json::to_string(self) {
            Ok(data) => {
                raw_memory::set_segment(segment as u32, data.as_str());
            }
            Err(e) => {
                error!("Failed to serialize game_state {:?}", e);
            }
        }
    }
}

impl GameState {
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

    pub fn count_creeps_in_room<'b>(&mut self, room: &'b Room) -> &mut HashMap<Role, i8> {
        let name = room.name();
        // TODO: use cached value
        let count = self
            .count_roles_in_room(room)
            .iter()
            .map(|(k, v)| (*k, *v))
            .collect();
        self.creep_count_by_room.insert(name.clone(), count);
        self.creep_count_by_room.get_mut(&name).unwrap()
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

    pub fn creep_memory_set<T: Into<Value>>(&mut self, creep: CreepName, key: &str, value: T) {
        let val: Value = value.into();
        self.creep_memory_entry(creep).insert(key.to_owned(), val);
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

    pub fn creep_memory_i64(&self, creep: CreepName, key: &str) -> Option<i64> {
        self.creep_memory_get(creep)
            .and_then(|map| map.get(key))
            .and_then(|x| x.as_i64())
    }

    pub fn creep_memory_role(&self, creep: CreepName, key: &str) -> Option<Role> {
        self.creep_memory_i64(creep, key)
            .map(|x| Role::from(x as u8))
    }

    pub fn cleanup_memory(&mut self) -> Result<(), Box<dyn Error>> {
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

    fn count_roles_in_room(&self, room: &Room) -> HashMap<Role, i8> {
        let mut result = Role::all_roles()
            .into_iter()
            .map(|x| (x, 0))
            .collect::<HashMap<_, _>>();

        self.creep_memory
            .iter()
            .filter(|(k, _v)| {
                self.creep_memory_string(CreepName(k), HOME_ROOM)
                    .map(|r| r == room.name())
                    .unwrap_or(false)
            })
            .filter_map(|(k, _v)| {
                self.creep_memory_i64(CreepName(k), CREEP_ROLE)
                    .map(|x| x as u8)
            })
            .for_each(|role| {
                if let Some(count) = result.get_mut(&Role::from(role)) {
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

pub trait WithStateSave<'a> {
    fn with_state_save<T: ToPrimitive + 'a>(self, creep: String, task_id: T)
        -> Task<'a, GameState>;
}

impl<'a> WithStateSave<'a> for Task<'a, GameState> {
    fn with_state_save<T: ToPrimitive + 'a>(
        self,
        creep: String,
        task_id: T,
    ) -> Task<'a, GameState> {
        let tasks = [
            self,
            Task::new(move |state: &mut GameState| {
                state.creep_memory_set(
                    CreepName(&creep),
                    TASK,
                    task_id.to_u32().unwrap_or(0) as i32,
                );
                Ok(())
            }),
        ]
        .into_iter()
        .cloned()
        .collect();

        // Only save the state if the task succeeded
        let selector = Control::Selector(tasks);
        selector.into()
    }
}

