pub mod creep_state;
pub mod roles;
pub mod spawn_info;

mod conqueror;
mod gofer;
mod harvester;
mod lrh;
mod lrw;
mod repairer;
mod scout;
mod upgrader;
mod worker;

use self::creep_state::CreepState;
pub use self::roles::Role;
use crate::game_state::{RoomIFF, ScoutInfo};
use crate::prelude::*;
use screeps::{
    constants::{find, ResourceType},
    game::{self, get_object_erased, get_object_typed},
    objects::{
        Creep, HasId, Resource, RoomObject, RoomObjectProperties, Structure, StructureContainer,
        StructureStorage, Tombstone, Withdrawable,
    },
    prelude::*,
    ReturnCode, Room,
};
use stdweb::{
    unstable::{TryFrom, TryInto},
    Reference,
};

pub const HOME_ROOM: &'static str = "home";
pub const TARGET: &'static str = "target";
pub const CREEP_ROLE: &'static str = "role";
pub const LOADING: &'static str = "loading";
pub const TASK: &'static str = "task";

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct CreepExecutionStats {
    working_creeps: u16,
    idle_creeps: u16,
    total_execution_time: f32,
}

pub fn task<'a>() -> Task<'a, GameState> {
    Task::new(move |state: &mut GameState| {
        let start = game::cpu::get_used();

        screeps::game::creeps::values()
            .into_iter()
            .for_each(|creep| {
                let mut state = CreepState::new(creep, state);
                run_creep(&mut state).unwrap_or(())
            });

        let end = game::cpu::get_used();

        state.creep_stats.total_execution_time = (end - start) as f32;

        Ok(())
    })
}

fn run_creep<'a>(state: &mut CreepState) -> ExecutionResult {
    debug!("Running creep {}", state.creep_name().0);

    if state.creep().spawning() {
        return Ok(());
    }
    let tasks = [
        Task::new(|state: &mut CreepState| {
            run_role(state)
                .map_err(|e| {
                    debug!("Recording failed run {:?}", e);
                    unsafe {
                        (*state.mut_game_state()).creep_stats.idle_creeps += 1;
                    }
                    state.creep().say("💤", false);
                    e
                })
                .map(|_| {
                    debug!("Recording successful run");
                    unsafe {
                        (*state.mut_game_state()).creep_stats.working_creeps += 1;
                    }
                })
        }),
        Task::new(|state: &mut CreepState| {
            let gs = state.mut_game_state();
            unsafe { initialize_creep(&mut *gs, state.creep()) }
        }),
    ];

    sequence(state, tasks.iter())
}

pub fn initialize_creep<'a>(state: &'a mut GameState, creep: &'a Creep) -> ExecutionResult {
    assign_role(state, &creep)
        .ok_or_else(|| "Failed to find a role for creep")?;
    let memory = state.creep_memory_entry(CreepName(&creep.name()));
    memory.insert(HOME_ROOM.into(), creep.room().name().into());
    Ok(())
}

fn assign_role<'a>(state: &'a mut GameState, creep: &'a Creep) -> Option<Role> {
    trace!("Assigning role to {}", creep.name());

    if state
        .creep_memory_role(CreepName(&creep.name()), CREEP_ROLE)
        .is_some()
    {
        trace!("Already has a role");
        None?;
    }

    let result = roles::next_role(state, &creep.room()).or_else(|| {
        warn!("Room is full");
        None
    })?;

    let memory = state.creep_memory_entry(CreepName(&creep.name()));
    memory.insert(CREEP_ROLE.to_string(), (result as i64).into());
    Some(result)
}

fn run_role<'a>(state: &'a mut CreepState) -> ExecutionResult {
    let task = {
        let role = state.creep_memory_role(CREEP_ROLE).ok_or_else(|| {
            let error = "failed to read creep role";
            error!("{}", error);
            error
        })?;

        roles::run_role(role)
    };
    task.tick(state)
}

pub fn move_to<'a>(
    creep: &'a Creep,
    target: &'a impl screeps::RoomObjectProperties,
) -> ExecutionResult {
    let res = js! {
        const creep = @{creep};
        const target = @{target.pos()};
        return creep.moveTo(target, {reusePath: 10});
    };
    let res =
        ReturnCode::try_from(res).map_err(|e| format!("Failed to convert move result {:?}", e))?;
    match res {
        ReturnCode::Ok | ReturnCode::Tired => Ok(()),
        _ => {
            let error = format!("Move failed {:?}", res);
            debug!("{}", error);
            Err(error)
        }
    }
}

pub struct MoveToOptions {
    reuse_path: Option<i32>,
}

pub fn move_to_options<'a>(
    creep: &'a Creep,
    target: &'a impl screeps::RoomObjectProperties,
    options: MoveToOptions,
) -> ExecutionResult {
    let reuse_path = options.reuse_path;
    let res = js! {
        const creep = @{creep};
        const target = @{target.pos()};
        const reusePath = @{reuse_path};
        return creep.moveTo(target, {reusePath: reusePath});
    };
    let res =
        ReturnCode::try_from(res).map_err(|e| format!("Failed to convert move result {:?}", e))?;
    match res {
        ReturnCode::Ok | ReturnCode::Tired => Ok(()),
        _ => {
            let error = format!("Move failed {:?}", res);
            debug!("{}", error);
            Err(error)
        }
    }
}

/// Find and pick up energy from the ground
/// # Contracts & Side effects
/// Required the `loading` flag to be set to true
/// If the creep is full sets the `loading` flag to false
pub fn pickup_energy<'a>(state: &mut CreepState) -> ExecutionResult {
    if !state.creep_memory_bool(LOADING).unwrap_or(false) {
        Err("not loading")?;
    }

    if state.creep().carry_total() == state.creep().carry_capacity() {
        state.creep_memory_set(LOADING.into(), false);
        state.creep_memory_remove(TARGET);
        Err("full")?;
    }

    let target = state
        .creep_memory_string(TARGET)
        .and_then(|id| get_object_typed::<Resource>(id).unwrap_or(None))
        .or_else(|| {
            find_dropped_energy(state.creep()).map(|target| {
                let id = target.id();
                state.creep_memory_set(TARGET.into(), id);
                target
            })
        })
        .ok_or_else(|| {
            state.creep_memory_remove(TARGET);
            "Can't find energy source"
        })?;

    let tasks = [
        Task::new(
            |state: &mut CreepState| match state.creep().pickup(&target) {
                ReturnCode::Ok => Ok(()),
                _ => Err("Can't pick up".to_owned()),
            },
        ),
        Task::new(|state: &mut CreepState| move_to(state.creep(), &target)),
        Task::new(|state: &mut CreepState| {
            state.creep_memory_remove(TARGET);
            Ok(())
        }),
    ];

    sequence(state, tasks.iter()).map_err(|_| {
        state.creep_memory_remove(TARGET);
        "can't pick up energy".into()
    })
}

pub fn find_dropped_energy(creep: &Creep) -> Option<Resource> {
    creep
        .room()
        .find(find::DROPPED_RESOURCES)
        .into_iter()
        .filter(|resource| resource.resource_type() == ResourceType::Energy)
        .max_by_key(|r| r.amount())
}

/// Retreive energy from a Container
/// # Contracts & Side effects
/// Required the `loading` flag to be set to true
/// If the creep is full sets the `loading` flag to false
pub fn withdraw_energy<'a>(state: &'a mut CreepState) -> ExecutionResult {
    trace!("Getting energy");

    let target = {
        if !state.creep_memory_bool(LOADING).unwrap_or(false) {
            Err("not loading")?;
        }

        if state.creep().carry_total() == state.creep().carry_capacity() {
            state.creep_memory_set(LOADING.into(), false);
            state.creep_memory_remove(TARGET);
            Err("full")?;
        }

        state
            .creep_memory_string(TARGET)
            .and_then(|id| get_object_erased(id))
            .or_else(|| {
                find_available_energy(state.creep()).map(|target| {
                    let id = js! {
                        return @{&target}.id;
                    };
                    let id: String = id.try_into().unwrap();

                    state.creep_memory_set(TARGET.into(), id);
                    target
                })
            })
            .ok_or_else(|| {
                state.creep_memory_remove(TARGET);
                "Can't find energy source"
            })?
    };

    let tasks = [
        Task::new(|state| try_withdraw::<Tombstone>(state, &target)),
        Task::new(|state| try_withdraw::<StructureStorage>(state, &target)),
        Task::new(|state| try_withdraw::<StructureContainer>(state, &target)),
    ];
    sequence(state, tasks.iter()).map_err(|_| {
        warn!("Got a target that can not be withdrawn from");
        state.creep_memory_remove(TARGET);
        "can't withdraw".into()
    })
}

fn try_withdraw<'a, T>(state: &mut CreepState, target: &'a RoomObject) -> ExecutionResult
where
    T: 'a + Withdrawable + screeps::traits::TryFrom<&'a Reference>,
{
    let target = T::try_from(target.as_ref()).map_err(|_| "Failed to convert target")?;
    withdraw(state, &target)
}

fn withdraw<'a, T>(state: &mut CreepState, target: &'a T) -> ExecutionResult
where
    T: Withdrawable,
{
    let creep = state.creep();
    if creep.pos().is_near_to(target) {
        let r = creep.withdraw_all(target, ResourceType::Energy);
        if r != ReturnCode::Ok {
            debug!("couldn't withdraw: {:?}", r);
            Err("couldn't withdraw")?;
        }
    } else {
        move_to(creep, target)?;
    }
    Ok(())
}

fn find_available_energy<'a>(creep: &'a Creep) -> Option<RoomObject> {
    trace!("Finding new withdraw target");
    let result = js! {
        const creep = @{creep};
        const ts = creep.pos.findClosestByRange(FIND_TOMBSTONES, {
            filter: (ts) => ts.creep.my && ts.store[RESOURCE_ENERGY]
        });
        if (ts) {
            return ts;
        }
        if (creep.room.storage && creep.room.storage.store[RESOURCE_ENERGY] > 0) {
            return creep.room.storage;
        }
        const container = creep.pos.findClosestByRange(FIND_STRUCTURES, {
            filter: (i) => i.structureType == STRUCTURE_CONTAINER && i.store[RESOURCE_ENERGY] > 0
        });
        return container;
    };
    result.try_into().unwrap_or_else(|_| None)
}

/// Fallback harvest, method for a worker to harvest energy temporary
/// ## Contracts:
/// - Should not interfere with the harvester::harvest functionality
pub fn harvest<'a>(state: &mut CreepState) -> ExecutionResult {
    trace!("Worker harvesting");

    {
        let loading = state.creep_memory_bool(LOADING);
        if !loading.unwrap_or(false) {
            Err("not loading")?;
        }
        let creep = state.creep();
        if creep.carry_total() == creep.carry_capacity() {
            state.creep_memory_set(LOADING.into(), false);
            state.creep_memory_remove(TARGET);
            return Ok(());
        }
    }

    harvester::attempt_harvest(state, Some(TARGET))
}

pub fn find_repair_target<'a>(room: &'a Room) -> Option<Structure> {
    trace!("Finding repair target in room {:?}", room.name());

    let candidates = js! {
        const room = @{room};
        return room.find(FIND_STRUCTURES, {
            filter: s => {
                switch (s.structureType) {
                    case STRUCTURE_WALL:
                        return s.hits < 10*1000;
                    default:
                        return s.hits < s.hitsMax;
                }
            }
        });
    };
    let candidates: Vec<Structure> = candidates
        .try_into()
        .map_err(|e| {
            error!("Failed to deserialize repair candidates {:?}", e);
        })
        .ok()?;

    candidates
        .into_iter()
        .filter(|s| s.as_attackable().is_some())
        .min_by_key(|s| s.as_attackable().map(|s| s.hits()).unwrap())
}

pub fn update_scout_info<'a>(state: &mut CreepState) -> ExecutionResult {
    let creep = state.creep();
    let room = creep.room();

    let n_sources = room.find(find::SOURCES).len() as u8;

    let controller = room.controller();

    let is_my_controller = controller
        .as_ref()
        .map(|c| {
            // c.my() can panic
            let result = js! {
                return @{c}.my;
            };
            result
        })
        .map(|my| bool::try_from(my).unwrap_or(false));

    let iff = match is_my_controller {
        None => RoomIFF::NoMansLand,
        Some(true) => RoomIFF::Friendly,
        Some(false) => match controller.map(|c| c.level()) {
            Some(0) => RoomIFF::Neutral,
            Some(_) => RoomIFF::Hostile,
            None => RoomIFF::Unknown,
        },
    };

    let info = ScoutInfo {
        n_sources,
        iff,
        time_of_recording: game::time(),
    };

    unsafe {
        (*state.mut_game_state())
            .scout_intel
            .insert(WorldPosition::from(room), info)
    };

    Ok(())
}

/// target_key is a memory entry key
pub fn approach_target_room<'a>(state: &mut CreepState, target_key: &str) -> ExecutionResult {
    let target = state.creep_memory_string(target_key).ok_or("no target")?;

    let creep = state.creep();

    let room = creep.room();
    let room_name = room.name();

    if room_name == target {
        Err("Already in the target room")?;
    }

    let result = js! {
        const creep = @{creep};
        const room = @{target};
        const exitDir = creep.room.findExitTo(room);
        const exit = creep.pos.findClosestByRange(exitDir);
        return creep.moveTo(exit);
    };

    let result =
        ReturnCode::try_from(result).map_err(|e| format!("Failed to parse return code {:?}", e))?;

    match result {
        ReturnCode::NoPath | ReturnCode::InvalidTarget => Err("Failed to move".to_owned()),
        _ => Ok(()),
    }
}

