//! Move resources
//!
use super::{move_to, pickup_energy, TARGET, TASK};
use crate::prelude::*;
use num::FromPrimitive;
use screeps::{
    constants::ResourceType,
    game::{get_object_erased, get_object_typed},
    objects::{
        Creep, HasStore, StructureContainer, StructureExtension, StructureSpawn, StructureStorage,
        StructureTower, Transferable,
    },
    prelude::*,
    ReturnCode,
};
use stdweb::{
    unstable::{TryFrom, TryInto},
    Reference,
};

#[derive(Debug, Clone, Copy, FromPrimitive, ToPrimitive)]
#[repr(u8)]
enum GoferState {
    Idle = 0,
    PickingUpEnergy,
    WithdrawingEnergy,
    Unloading,
}

pub fn run<'a>(creep: &'a Creep) -> Task<'a, GameState> {
    trace!("Running gofer {}", creep.name());
    Task::new(move |state: &mut GameState| {
        let task = prepare_task(creep, state);
        task.tick(state).map_err(|err| {
            let memory = state.creep_memory_entry(CreepName(&creep.name()));
            memory.remove(TARGET);
            err
        })
    })
    .with_name("Gofer")
}

fn prepare_task<'a>(creep: &'a Creep, state: &GameState) -> Task<'a, GameState> {
    let name = creep.name();
    let last_task = state
        .creep_memory_i64(CreepName(name.as_str()), TASK)
        .unwrap_or(0);
    let last_task: GoferState = GoferState::from_u32(last_task as u32).unwrap_or(GoferState::Idle);

    let mut priorities = [0, 0, 0];

    match last_task {
        GoferState::Unloading => priorities[0] += 1,
        GoferState::WithdrawingEnergy => priorities[1] += 1,
        GoferState::PickingUpEnergy => priorities[2] += 1,
        _ => {}
    }

    let tasks = [
        Task::new(move |state| get_energy(state, creep))
            .with_name("Get energy")
            .with_priority(priorities[1])
            .with_state_save(name.clone(), GoferState::WithdrawingEnergy),
        Task::new(move |state| pickup_energy(state, creep))
            .with_name("Pickup energy")
            .with_priority(priorities[2])
            .with_state_save(name.clone(), GoferState::PickingUpEnergy),
        Task::new(move |state| attempt_unload(state, creep))
            .with_name("Attempt unload")
            .with_priority(priorities[0])
            .with_state_save(name.clone(), GoferState::Unloading),
    ]
    .into_iter()
    .cloned()
    .collect();

    Control::Sequence(tasks).sorted_by_priority().into()
}

pub fn attempt_unload<'a>(state: &'a mut GameState, creep: &'a Creep) -> ExecutionResult {
    trace!("Unloading");
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

    let target = find_unload_target(state, creep).ok_or_else(|| "no unload target")?;

    let tasks = [
        Task::new(|state| try_transfer::<StructureSpawn>(state, creep, &target))
            .with_name("Try transfer to StructureSpawn"),
        Task::new(|state| try_transfer::<StructureExtension>(state, creep, &target))
            .with_name("Try transfer to StructureExtension"),
        Task::new(|state| try_transfer::<StructureTower>(state, creep, &target))
            .with_name("Try transfer to StructureTower"),
        Task::new(|state| try_transfer::<StructureStorage>(state, creep, &target))
            .with_name("Try transfer to StructureStorage"),
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
    if let Some(target) = read_unload_target(state, creep) {
        return Some(target);
    }
    let tasks = [
        Task::new(|state| find_unload_target_by_type(state, creep, "spawn"))
            .with_name("Find unload target by type spawn"),
        Task::new(|state| find_unload_target_by_type(state, creep, "tower"))
            .with_name("Find unload target by type tower"),
        Task::new(|state| find_unload_target_by_type(state, creep, "extension"))
            .with_name("Find unload target by type extension"),
        Task::new(|state| find_storage(state, creep))
            .with_name("Find unload target by type storage"),
    ]
    .into_iter()
    .cloned()
    .collect();
    let tree = Control::Sequence(tasks);
    match tree.tick(state) {
        Ok(_) => read_unload_target(state, creep),
        Err(e) => {
            debug!("Failed to find unload target {:?}", e);
            let memory = state.creep_memory_entry(CreepName(&creep.name()));
            memory.remove(TARGET);
            None
        }
    }
}

fn read_unload_target(state: &mut GameState, creep: &Creep) -> Option<Reference> {
    state
        .creep_memory_string(CreepName(&creep.name()), TARGET)
        .and_then(|target| {
            trace!("Validating existing target");
            get_object_erased(target).map(|target| target.as_ref().clone())
        })
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
    let storage = creep
        .room()
        .storage()
        .ok_or_else(|| format!("No storage in room {:?}", creep.room().name()))?;
    if storage.store_total() == storage.store_capacity() {
        Err("Storage is full")?;
    }
    state
        .creep_memory_entry(CreepName(&creep.name()))
        .insert(TARGET.into(), storage.id().into());
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
            memory.remove(TARGET);
            Err("full")?
        }
    }

    let target = find_container(state, creep).ok_or_else(|| "no container found")?;
    let task = withdraw(creep, &target);
    task.tick(state).map_err(|e| {
        let memory = state.creep_memory_entry(CreepName(&creep.name()));
        memory.remove(TARGET);
        e
    })
}

fn withdraw<'a>(creep: &'a Creep, target: &'a StructureContainer) -> Task<'a, GameState> {
    let tasks = [
        Task::new(move |_| {
            if creep.pos().is_near_to(target) {
                let r = creep.withdraw_all(target, ResourceType::Energy);
                if r != ReturnCode::Ok {
                    debug!("couldn't withdraw: {:?}", r);
                    Err("can't withdraw")?;
                }
            } else {
                move_to(creep, target)?;
            }
            Ok(())
        }),
        Task::new(move |_| {
            if target.store_total() == 0 {
                Err("Target is empty")?;
            }
            Ok(())
        }),
    ]
    .into_iter()
    .cloned()
    .collect();

    let selector = Control::Selector(tasks);
    selector.into()
}

fn find_container<'a>(state: &mut GameState, creep: &'a Creep) -> Option<StructureContainer> {
    read_target_container(state, creep).or_else(|| {
        trace!("Finding new withdraw target");
        let memory = state.creep_memory_entry(CreepName(&creep.name()));
        memory.remove(TARGET);
        let containers = js! {
            let creep = @{creep};
            const containers = creep.room.find(FIND_STRUCTURES, {
                filter: (i) => i.structureType == STRUCTURE_CONTAINER
                    && i.store[RESOURCE_ENERGY] > 0
            });
            return containers;
        };
        let containers: Vec<StructureContainer> =
            containers.try_into().map(|c| Some(c)).unwrap_or(None)?;

        let result = containers
            .into_iter()
            .max_by_key(|i| i.store_of(ResourceType::Energy));

        result.map(|c| {
            memory.insert(TARGET.into(), c.id().into());
            c
        })
    })
}

fn read_target_container(state: &GameState, creep: &Creep) -> Option<StructureContainer> {
    state
        .creep_memory_string(CreepName(&creep.name()), TARGET)
        .and_then(|id| get_object_typed(id).ok())?
}

