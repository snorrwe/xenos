//! Move resources
//!
use super::super::bt::*;
use super::move_to;
use screeps::{
    constants::ResourceType,
    game::get_object_erased,
    objects::{
        Creep, StructureContainer, StructureExtension, StructureSpawn, StructureStorage,
        StructureTower, Transferable,
    },
    prelude::*,
    ReturnCode,
};
use stdweb::{
    unstable::{TryFrom, TryInto},
    Reference,
};

pub fn run<'a>(creep: &'a Creep) -> Task<'a> {
    trace!("Running gofer {}", creep.name());
    let tasks = vec![
        Task::new(move |state| attempt_unload(state, creep)),
        Task::new(move |_| get_energy(creep)),
        Task::new(move |state| attempt_unload(state, creep)),
    ];

    let tree = Control::Sequence(tasks);
    Task::new(move |state| {
        tree.tick(state).map_err(|e| {
            creep.memory().del("target");
            e
        })
    })
}

fn attempt_unload<'a>(state: &'a mut GameState, creep: &'a Creep) -> ExecutionResult {
    trace!("Unloading");
    let loading: bool = creep.memory().bool("loading");
    if loading {
        return Err("loading".into());
    }

    let carry_total = creep.carry_total();

    if carry_total == 0 {
        trace!("Empty");
        creep.memory().set("loading", true);
        return Err("empty".into());
    }

    let target = find_unload_target(state, creep).ok_or_else(|| String::new())?;

    let tasks = vec![
        Task::new(|_| try_transfer::<StructureSpawn>(creep, &target)),
        Task::new(|_| try_transfer::<StructureExtension>(creep, &target)),
        Task::new(|_| try_transfer::<StructureTower>(creep, &target)),
        Task::new(|_| try_transfer::<StructureStorage>(creep, &target)),
    ];

    let tree = Control::Sequence(tasks);
    tree.tick(state).map_err(|e| {
        creep.memory().del("target");
        e
    })
}

fn find_unload_target<'a>(state: &'a mut GameState, creep: &'a Creep) -> Option<Reference> {
    trace!("Setting unload target");

    let target = creep
        .memory()
        .string("target")
        .map_err(|e| {
            error!("failed to read creep target {:?}", e);
        })
        .ok()?;

    if let Some(target) = target {
        trace!("Validating existing target");
        let target = get_object_erased(target.as_str())?;
        Some(target.as_ref().clone())
    } else {
        let tasks = vec![
            Task::new(|_| find_unload_target_by_type(creep, "spawn")),
            Task::new(|_| find_unload_target_by_type(creep, "tower")),
            Task::new(|_| find_unload_target_by_type(creep, "extension")),
            Task::new(|_| find_storage(creep)),
        ];
        let tree = Control::Sequence(tasks);
        tree.tick(state).unwrap_or_else(|e| {
            debug!("Failed to find unload target {:?}", e);
            creep.memory().del("target");
        });
        None
    }
}

fn try_transfer<'a, T>(creep: &'a Creep, target: &'a Reference) -> ExecutionResult
where
    T: Transferable + screeps::traits::TryFrom<&'a Reference>,
{
    let target = T::try_from(target.as_ref())
        .map_err(|_| String::from("failed to convert transfer target"))?;
    transfer(creep, &target)
}

fn find_storage<'a>(creep: &'a Creep) -> ExecutionResult {
    let res = js! {
        const creep = @{creep};
        const exts = creep.room.find(FIND_STRUCTURES, {
            filter: function (s) {
                return s.structureType == STRUCTURE_STORAGE && s.store[RESOURCE_ENERGY] < s.storeCapacity;
            }
        });
        return exts[0] && exts[0].id;
    };
    let target = String::try_from(res).map_err(|_| String::from("expected string"))?;
    creep.memory().set("target", target);
    Ok(())
}

fn find_unload_target_by_type<'a>(creep: &'a Creep, struct_type: &'a str) -> ExecutionResult {
    let res = js! {
        const creep = @{creep};
        const ext = creep.pos.findClosestByRange(FIND_STRUCTURES, {
            filter: function (s) {
                return s.structureType == @{struct_type} && s.energy < s.energyCapacity;
            }
        });
        return ext && ext.id;
    };
    let target = String::try_from(res).map_err(|_| String::from("expected string"))?;
    creep.memory().set("target", target);
    Ok(())
}

fn transfer<'a, T>(creep: &'a Creep, target: &'a T) -> ExecutionResult
where
    T: Transferable,
{
    if creep.pos().is_near_to(target) {
        let r = creep.transfer_all(target, ResourceType::Energy);
        if r != ReturnCode::Ok {
            trace!("couldn't unload: {:?}", r);
            creep.memory().del("target");
        }
    } else {
        move_to(creep, target)?;
    }
    Ok(())
}

/// Retreive energy from a Container
/// # Contracts & Side effects
/// Required the `loading` flag to be set to true
/// If the creep is full sets the `loading` flag to false
pub fn get_energy<'a>(creep: &'a Creep) -> ExecutionResult {
    trace!("Getting energy");

    let loading: bool = creep.memory().bool("loading");
    if !loading {
        return Err("not loading".into());
    }
    if creep.carry_total() == creep.carry_capacity() {
        creep.memory().set("loading", false);
        Err("full".into())
    } else {
        let target = find_container(creep).ok_or_else(|| String::new())?;

        let target = StructureContainer::try_from(target.as_ref())
            .map_err(|e| format!("Failed to convert target to container {:?}", e))?;
        withdraw(creep, &target)
    }
}

fn withdraw<'a>(creep: &'a Creep, target: &'a StructureContainer) -> ExecutionResult {
    if creep.pos().is_near_to(target) {
        let r = creep.withdraw_all(target, ResourceType::Energy);
        if r != ReturnCode::Ok {
            debug!("couldn't unload: {:?}", r);
        }
    } else {
        move_to(creep, target)?;
    }
    Ok(())
}

fn find_container<'a>(creep: &'a Creep) -> Option<Reference> {
    trace!("Finding new withdraw target");
    // screeps api is bugged at the moment and FIND_STRUCTURES panics
    let result = js! {
        let creep = @{creep};
        const container = creep.pos.findClosestByRange(FIND_STRUCTURES, {
            filter: (i) => i.structureType == STRUCTURE_CONTAINER &&
                i.store[RESOURCE_ENERGY] > 0
        });
        return container;
    };
    let result = result.try_into().unwrap_or_else(|_| None);
    result
}

