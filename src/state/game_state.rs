use crate::creeps::roles::Role;
use crate::creeps::{CreepExecutionStats, CREEP_ROLE, HOME_ROOM};
use crate::prelude::*;
use screeps::{raw_memory, Room};
use serde_json::{self, Value};
use std::collections::{BTreeMap, BTreeSet};
use std::error::Error;
use std::pin::Pin;

pub type CreepMemoryEntry = BTreeMap<String, Value>;
pub type CreepMemory = BTreeMap<String, CreepMemoryEntry>;

#[derive(Debug, Default, Serialize, Deserialize)]
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
    creep_count_by_room: BTreeMap<WorldPosition, BTreeMap<Role, i8>>,

    /// Information about rooms
    /// Structure: room -> info
    pub scout_intel: BTreeMap<WorldPosition, ScoutInfo>,

    /// Number of LRH per room
    /// In directions: [N, W, S, E]
    pub long_range_harvesters: BTreeMap<WorldPosition, [u8; 4]>,

    /// Holds the creep memory objects
    /// Structure: name -> data
    creep_memory: CreepMemory,

    /// Data about creep task execution in a tick
    #[serde(skip_serializing)]
    #[serde(skip_deserializing)]
    #[serde(default)]
    pub creep_stats: CreepExecutionStats,

    /// Rooms targeted for expansion
    pub expansion: BTreeSet<WorldPosition>,
}

impl Clone for GameState {
    fn clone(&self) -> Self {
        panic!(
            "Do not clone GameState objects, the trait impl is provided so the Tasks are cloneable"
        );
    }
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
    pub time_of_recording: u32,
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
#[derive(Debug, Clone, Copy, Ord, PartialEq, PartialOrd, Eq, Hash)]
pub struct CreepName<'a>(pub &'a str);

impl Default for RoomIFF {
    fn default() -> Self {
        RoomIFF::Unknown
    }
}

impl GameState {
    pub fn read_from_segment_or_default(segment: u32) -> Pin<Box<Self>> {
        let state = raw_memory::get_segment(segment)
            .and_then(|string| {
                serde_json::from_str(&string)
                    .map_err(|e| {
                        error!("Failed to deserialize game_state from segment {:?}", e);
                    })
                    .ok()
            })
            .unwrap_or_default();
        Box::pin(state)
    }

    pub fn count_creeps_in_room<'a>(&'a mut self, room: &Room) -> &'a BTreeMap<Role, i8> {
        let pos = WorldPosition::from(room);
        if let None = self.creep_count_by_room.get(&pos) {
            let count = self
                .count_roles_in_room(room)
                .iter()
                .map(|(k, v)| (*k, *v))
                .collect();
            self.creep_count_by_room.insert(pos, count);
        }
        self.creep_count_by_room.get(&pos).unwrap()
    }

    /// Get an entry in the creep's memory
    /// Inserts and empty map in the creep's name if none is found
    pub fn creep_memory_entry(&mut self, name: CreepName) -> &mut CreepMemoryEntry {
        self.creep_memory
            .entry(name.0.to_owned())
            .or_insert_with(|| Default::default())
    }

    pub fn creep_memory_get(&self, creep: CreepName) -> Option<&CreepMemoryEntry> {
        self.creep_memory.get(creep.0)
    }

    #[allow(unused)]
    pub fn creep_memory_set<T: Into<Value>>(&mut self, creep: CreepName, key: &str, value: T) {
        let val: Value = value.into();
        self.creep_memory_entry(creep).insert(key.to_owned(), val);
    }

    #[allow(unused)]
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

    fn count_roles_in_room(&self, room: &Room) -> BTreeMap<Role, i8> {
        let mut result = Role::all_roles()
            .into_iter()
            .map(|x| (x, 0))
            .collect::<BTreeMap<_, _>>();

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

