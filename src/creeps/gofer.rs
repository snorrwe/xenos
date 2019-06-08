//! Move resources
//!
use super::{move_to, TARGET};
use crate::prelude::*;
use screeps::{
    constants::{find, ResourceType},
    game::{get_object_erased, get_object_typed},
    objects::{
        Creep, Structure, StructureContainer, StructureExtension, StructureSpawn, StructureStorage,
        StructureTower, Transferable,
    },
    prelude::*,
    ReturnCode,
};
use stdweb::{
    unstable::{TryFrom, TryInto},
    Reference,
};

pub fn run<'a>(creep: &'a Creep) -> Task<'a, GameState> {
    trace!("Running gofer {}", creep.name());
    let tasks = [
        Task::new(move |state| attempt_unload(state, creep)),
        Task::new(move |state| get_energy(state, creep)),
        Task::new(move |state| attempt_unload(state, creep)),
    ]
    .into_iter()
    .cloned()
    .collect();

    let tree = Control::Sequence(tasks);
    Task::new(move |state| {
        tree.tick(state).map_err(|err| {
            let memory = state.creep_memory_entry(CreepName(&creep.name()));
            memory.remove(TARGET);
            err
        })
    })
}

pub fn attempt_unload<'a>(state: &'a mut GameState, creep: &'a Creep) -> ExecutionResult {
    trace!("Unloading");
    {
        let loading = state.creep_memory_bool(CreepName(&creep.name()), "loading");
        if loading {
            Err("loading")?;
        }

        let carry_total = creep.carry_total();

        if carry_total == 0 {
            trace!("Empty");
            let memory = state.creep_memory_entry(CreepName(&creep.name()));
            memory.insert("loading".into(), true.into());
            Err("empty")?;
        }
    }

    let target = find_unload_target(state, creep).ok_or_else(|| "no unload target")?;

    let tasks = [
        Task::new(|state| try_transfer::<StructureSpawn>(state, creep, &target)),
        Task::new(|state| try_transfer::<StructureExtension>(state, creep, &target)),
        Task::new(|state| try_transfer::<StructureTower>(state, creep, &target)),
        Task::new(|state| try_transfer::<StructureStorage>(state, creep, &target)),
    ]
    .into_iter()
    .cloned()
    .collect();

    let tree = Control::Sequence(tasks);
    tree.tick(state).map_err(|e| {
        let memory = state.creep_memory_entry(CreepName(&creep.name()));
        memory.remove(TARGET);
        e
    })
}

fn find_unload_target<'a>(state: &'a mut GameState, creep: &'a Creep) -> Option<Reference> {
    trace!("Setting unload target");
    {
        let target = state.creep_memory_string(CreepName(&creep.name()), TARGET);

        if let Some(target) = target {
            trace!("Validating existing target");
            let target = get_object_erased(target)?;
            return Some(target.as_ref().clone());
        }
    }
    let tasks = [
        Task::new(|state| find_unload_target_by_type(state, creep, "spawn")),
        Task::new(|state| find_unload_target_by_type(state, creep, "tower")),
        Task::new(|state| find_unload_target_by_type(state, creep, "extension")),
        Task::new(|state| find_storage(state, creep)),
    ]
    .into_iter()
    .cloned()
    .collect();
    let tree = Control::Sequence(tasks);
    tree.tick(state).unwrap_or_else(|e| {
        debug!("Failed to find unload target {:?}", e);
        let memory = state.creep_memory_entry(CreepName(&creep.name()));
        memory.remove(TARGET);
    });
    None
}

fn try_transfer<'a, T>(
    state: &mut GameState,
    creep: &'a Creep,
    target: &'a Reference,
) -> ExecutionResult
where
    T: Transferable + screeps::traits::TryFrom<&'a Reference>,
{
    let target = T::try_from(target).map_err(|_| "failed to convert transfer target")?;
    transfer(state, creep, &target)
}

fn find_storage<'a>(state: &mut GameState, creep: &'a Creep) -> ExecutionResult {
    let res = creep.room().find(find::STRUCTURES).into_iter().find(|s| {
        if let Structure::Storage(s) = s {
            s.store_total() < s.store_capacity()
        } else {
            false
        }
    });
    if let Some(res) = res {
        state
            .creep_memory_entry(CreepName(&creep.name()))
            .insert(TARGET.into(), res.id().into());
    }
    Ok(())
}

fn find_unload_target_by_type<'a>(
    state: &mut GameState,
    creep: &'a Creep,
    struct_type: &'a str,
) -> ExecutionResult {
    let res = js! {
        const creep = @{creep};
        const ext = creep.pos.findClosestByRange(FIND_STRUCTURES, {
            filter: function (s) {
                return s.structureType == @{struct_type} && s.energy < s.energyCapacity;
            }
        });
        return ext && ext.id;
    };
    let target = String::try_from(res).map_err(|_| "expected string")?;
    state
        .creep_memory_entry(CreepName(&creep.name()))
        .insert(TARGET.into(), target.into());
    Ok(())
}

fn transfer<'a, T>(state: &mut GameState, creep: &'a Creep, target: &'a T) -> ExecutionResult
where
    T: Transferable,
{
    if creep.pos().is_near_to(target) {
        let r = creep.transfer_all(target, ResourceType::Energy);
        if r != ReturnCode::Ok {
            trace!("couldn't unload: {:?}", r);
            state
                .creep_memory_entry(CreepName(&creep.name()))
                .remove(TARGET);
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
pub fn get_energy<'a>(state: &mut GameState, creep: &'a Creep) -> ExecutionResult {
    trace!("Getting energy");

    {
        let loading = state.creep_memory_bool(CreepName(&creep.name()), "loading");
        if !loading {
            Err("not loading")?;
        }
        let memory = state.creep_memory_entry(CreepName(&creep.name()));
        if creep.carry_total() == creep.carry_capacity() {
            memory.insert("loading".into(), false.into());

            Err("full")?
        }
    }

    let target = find_container(state, creep).ok_or_else(|| "no container found")?;
    withdraw(state, creep, &target).map_err(|e| {
        let memory = state.creep_memory_entry(CreepName(&creep.name()));
        memory.remove(TARGET);
        e
    })
}

fn withdraw<'a>(
    state: &mut GameState,
    creep: &'a Creep,
    target: &'a StructureContainer,
) -> ExecutionResult {
    if creep.pos().is_near_to(target) {
        let r = creep.withdraw_all(target, ResourceType::Energy);
        if r != ReturnCode::Ok {
            debug!("couldn't withdraw: {:?}", r);
            Err("can't withdraw")?;
        }
    } else if target.store_total() == 0 {
        let memory = state.creep_memory_entry(CreepName(&creep.name()));
        memory.remove(TARGET);
        Err("Target is empty")?;
    } else {
        move_to(creep, target)?;
    }
    Ok(())
}

fn find_container<'a>(state: &mut GameState, creep: &'a Creep) -> Option<StructureContainer> {
    read_target_container(state, creep).or_else(|| {
        trace!("Finding new withdraw target");
        let memory = state.creep_memory_entry(CreepName(&creep.name()));
        memory.remove(TARGET);
        let result = js! {
            let creep = @{creep};
            const container = creep.pos.findClosestByRange(FIND_STRUCTURES, {
                filter: (i) => i.structureType == STRUCTURE_CONTAINER
                            && i.store[RESOURCE_ENERGY] > 0
            });
            return container;
        };
        result
            .try_into()
            .unwrap_or(None)
            .map(|container: StructureContainer| {
                memory.insert(TARGET.into(), container.id().into());
                container
            })
    })
}

fn read_target_container(state: &GameState, creep: &Creep) -> Option<StructureContainer> {
    state
        .creep_memory_string(CreepName(&creep.name()), TARGET)
        .and_then(|id| get_object_typed(id).ok())?
}

