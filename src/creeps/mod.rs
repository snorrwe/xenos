pub mod roles;

mod builder;
mod conqueror;
mod gofer;
mod harvester;
mod repairer;
mod upgrader;

use super::bt::*;
use screeps::{
    constants::ResourceType,
    game::get_object_erased,
    objects::{
        Creep, RoomObject, RoomObjectProperties, StructureContainer, StructureStorage, Tombstone,
        Withdrawable,
    },
    prelude::*,
    ReturnCode,
};
use stdweb::{unstable::TryInto, Reference};

pub fn task<'a>() -> Task<'a> {
    let tasks = screeps::game::creeps::values()
        .into_iter()
        .map(|creep| run_creep(creep))
        .collect();

    let tree = Control::All(tasks);
    Task::new(move |state| tree.tick(state))
}

fn run_creep<'a>(creep: Creep) -> Task<'a> {
    Task::new(move |state| {
        debug!("Running creep {}", creep.name());
        if creep.spawning() {
            return Ok(());
        }
        let tasks = vec![
            Task::new(|state| run_role(state, &creep)),
            Task::new(|_| {
                assign_role(&creep)
                    .map(|_| {})
                    .ok_or_else(|| "Failed to find a role for creep".into())
            }),
        ];
        let tree = Control::Sequence(tasks);
        tree.tick(state)
    })
}

fn assign_role<'a>(creep: &'a Creep) -> Option<String> {
    trace!("Assigning role to {}", creep.name());

    if creep.memory().string("role").ok().is_some() {
        trace!("Already has a role");
        return None;
    }

    let result = roles::next_role(&creep.room()).or_else(|| {
        debug!("Room is full");
        None
    })?;

    creep.memory().set("role", &result);

    trace!("Assigned role {} to {}", result, creep.name());
    Some(result)
}

fn run_role<'a>(state: &'a mut GameState, creep: &'a Creep) -> ExecutionResult {
    let role = creep
        .memory()
        .string("role")
        .map_err(|e| {
            let error = format!("failed to read creep role {:?}", e);
            error!("{}", error);
            error
        })?
        .ok_or_else(|| {
            let error: String = "creep role is null".into();
            trace!("{}", error);
            error
        })?;

    let task = roles::run_role(role.as_str(), creep);
    task.tick(state)
}

pub fn move_to<'a>(
    creep: &'a Creep,
    target: &'a impl screeps::RoomObjectProperties,
) -> ExecutionResult {
    let res = creep.move_to(target);
    match res {
        ReturnCode::Ok | ReturnCode::Tired => Ok(()),
        _ => {
            let error = format!("Move failed {:?}", res);
            debug!("{}", error);
            Err(error)
        }
    }
}

/// Retreive energy from a Container
/// # Contracts & Side effects
/// Required the `loading` flag to be set to true
/// If the creep is full sets the `loading` flag to false
pub fn get_energy<'a>(state: &'a mut GameState, creep: &'a Creep) -> ExecutionResult {
    trace!("Getting energy");

    if !creep.memory().bool("loading") {
        return Err("not loading".into());
    }

    if creep.carry_total() == creep.carry_capacity() {
        creep.memory().set("loading", false);
        creep.memory().del("target");
        Err("full".to_string())?;
    }

    let target = creep
        .memory()
        .string("target")
        .map_err(|e| {
            error!("Failed to read target {:?}", e);
            "error in reading target".to_string()
        })?
        .map(|id| get_object_erased(id.as_str()))
        .unwrap_or_else(|| {
            find_available_energy(creep).map(|o| {
                js! {
                    @{creep}.memory.target = @{&o}.id;
                };
                o
            })
        })
        .ok_or_else(|| {
            creep.memory().del("target");
            "Can't find energy source".to_string()
        })?;

    let tasks = vec![
        Task::new(|_| try_withdraw::<Tombstone>(creep, &target)),
        Task::new(|_| try_withdraw::<StructureStorage>(creep, &target)),
        Task::new(|_| try_withdraw::<StructureContainer>(creep, &target)),
        Task::new(|_| {
            creep.memory().del("target");
            Ok(())
        }),
    ];
    let tree = Control::Sequence(tasks);
    tree.tick(state).map_err(|_| {
        creep.memory().del("target");
        "can't withdraw".into()
    })
}

fn try_withdraw<'a, T>(creep: &'a Creep, target: &'a RoomObject) -> ExecutionResult
where
    T: Withdrawable + screeps::traits::TryFrom<&'a Reference>,
{
    let target = T::try_from(target.as_ref()).map_err(|_| String::new())?;
    withdraw(creep, &target)
}

fn withdraw<'a, T>(creep: &'a Creep, target: &'a T) -> ExecutionResult
where
    T: Withdrawable,
{
    if creep.pos().is_near_to(target) {
        let r = creep.withdraw_all(target, ResourceType::Energy);
        if r != ReturnCode::Ok {
            debug!("couldn't withdraw: {:?}", r);
            return Err("couldn't withdraw".into());
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
        const energy = creep.pos.findClosestByRange(FIND_TOMBSTONES, {
            filter: (ts) => ts.creep.my && ts.store[RESOURCE_ENERGY]
        });
        if (energy) {
            return energy;
        }
        const container = creep.pos.findClosestByRange(FIND_STRUCTURES, {
            filter: (i) => (i.structureType == STRUCTURE_CONTAINER || i.structureType == STRUCTURE_STORAGE) &&
                           i.store[RESOURCE_ENERGY] > 0
        });
        return container;
    };
    result.try_into().unwrap_or_else(|_| None)
}

/// Fallback harvest, method for a worker to harvest energy temporary
/// ## Contracts:
/// - Should not interfere with the harvester::harvest functionality
pub fn harvest<'a>(creep: &'a Creep) -> ExecutionResult {
    trace!("Worker harvesting");

    let loading: bool = creep.memory().bool("loading");
    if !loading {
        return Err("not loading".into());
    }
    if creep.carry_total() == creep.carry_capacity() {
        creep.memory().set("loading", false);
        creep.memory().del("target");
        Ok(())
    } else {
        harvester::attempt_harvest(creep, Some("target"))
    }
}
