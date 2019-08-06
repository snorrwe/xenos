//! Move resources
//!
use super::{move_to, pickup_energy, CreepState, LOADING, TARGET, TASK};
use crate::prelude::*;
use num::FromPrimitive;
use screeps::{
    constants::ResourceType,
    game::{get_object_erased, get_object_typed},
    objects::{
        HasStore, StructureContainer, StructureExtension, StructureSpawn, StructureStorage,
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

pub fn task<'a>(creep: &'a CreepState) -> Task<'a, CreepState> {
    Task::new(move |state: &mut CreepState| {
        let task = prepare_task(state);
        task.tick(state).map_err(|err| {
            state.creep_memory_remove(TARGET);
            err
        })
    })
    .with_name("Gofer")
}

fn prepare_task<'a>(state: &mut CreepState) -> Task<'a, CreepState> {
    let name = state.creep_name();
    let last_task = state.creep_memory_i64(TASK).unwrap_or(0);
    let last_task: GoferState = GoferState::from_u32(last_task as u32).unwrap_or(GoferState::Idle);

    let mut priorities = [0, 0, 0];

    match last_task {
        GoferState::Unloading => priorities[0] += 1,
        GoferState::WithdrawingEnergy => priorities[1] += 1,
        GoferState::PickingUpEnergy => priorities[2] += 1,
        _ => {}
    }

    let tasks = [
        Task::new(|state| get_energy(state))
            .with_name("Get energy")
            .with_priority(priorities[1])
            .with_state_save(name.0.to_owned(), GoferState::WithdrawingEnergy),
        Task::new(|state| pickup_energy(state))
            .with_name("Pickup energy")
            .with_priority(priorities[2])
            .with_state_save(name.0.to_owned(), GoferState::PickingUpEnergy),
        Task::new(|state| attempt_unload(state))
            .with_name("Attempt unload")
            .with_priority(priorities[0])
            .with_state_save(name.0.to_owned(), GoferState::Unloading),
    ]
    .into_iter()
    .cloned()
    .collect();

    Control::Sequence(tasks).sorted_by_priority().into()
}

pub fn attempt_unload<'a>(state: &mut CreepState) -> ExecutionResult {
    trace!("Unloading");
    let loading = state.creep_memory_bool(LOADING).unwrap_or(false);
    if loading {
        Err("loading")?;
    }

    let creep = state.creep();

    let carry_total = creep.carry_total();

    if carry_total == 0 {
        state.creep_memory_set(LOADING.into(), true);
        Err("empty")?;
    }

    let target = find_unload_target(state).ok_or_else(|| "no unload target")?;

    let tasks = [
        Task::new(|state| try_transfer::<StructureSpawn>(state, &target))
            .with_name("Try transfer to StructureSpawn"),
        Task::new(|state| try_transfer::<StructureExtension>(state, &target))
            .with_name("Try transfer to StructureExtension"),
        Task::new(|state| try_transfer::<StructureTower>(state, &target))
            .with_name("Try transfer to StructureTower"),
        Task::new(|state| try_transfer::<StructureStorage>(state, &target))
            .with_name("Try transfer to StructureStorage"),
    ];

    sequence(state, tasks.iter()).map_err(|e| {
        state.creep_memory_remove(TARGET);
        e
    })
}

fn find_unload_target<'a>(state: &mut CreepState) -> Option<Reference> {
    trace!("Setting unload target");
    if let Some(target) = read_unload_target(state) {
        let full = js! {
            const target = @{&target};
            return !target.energyCapacity || target.energy < target.energyCapacity;
        };
        let full: bool = full.try_into().unwrap_or(true);
        if full {
            return Some(target);
        }
    }
    let tasks = [
        Task::new(|state| find_unload_target_by_type(state, "spawn"))
            .with_name("Find unload target by type spawn"),
        Task::new(|state| find_unload_target_by_type(state, "tower"))
            .with_name("Find unload target by type tower"),
        Task::new(|state| find_unload_target_by_type(state, "extension"))
            .with_name("Find unload target by type extension"),
        Task::new(|state| find_storage(state)).with_name("Find unload target by type storage"),
    ];
    match sequence(state, tasks.iter()) {
        Ok(_) => read_unload_target(state),
        Err(e) => {
            debug!("Failed to find unload target {:?}", e);
            state.creep_memory_remove(TARGET);
            None
        }
    }
}

fn read_unload_target<'a>(state: &mut CreepState) -> Option<Reference> {
    state.creep_memory_string(TARGET).and_then(|target| {
        trace!("Validating existing target");
        get_object_erased(target).map(|target| target.as_ref().clone())
    })
}

fn try_transfer<'a, T>(state: &mut CreepState, target: &'a Reference) -> ExecutionResult
where
    T: Transferable + screeps::traits::TryFrom<&'a Reference>,
{
    let target = T::try_from(target).map_err(|_| "failed to convert transfer target")?;
    transfer(state, &target)
}

fn find_storage<'a>(state: &mut CreepState) -> ExecutionResult {
    let storage = state
        .creep()
        .room()
        .storage()
        .ok_or_else(|| format!("No storage in room {:?}", state.creep().room().name()))?;
    if storage.store_total() == storage.store_capacity() {
        Err("Storage is full")?;
    }
    state.creep_memory_set(TARGET.into(), storage.id());
    Ok(())
}

fn find_unload_target_by_type<'a>(
    state: &mut CreepState,
    struct_type: &'a str,
) -> ExecutionResult {
    let res = js! {
        const creep = @{state.creep()};
        const ext = creep.pos.findClosestByRange(FIND_STRUCTURES, {
            filter: function (s) {
                return s.structureType == @{struct_type} && s.energy < s.energyCapacity;
            }
        });
        return ext && ext.id;
    };
    let target = String::try_from(res).map_err(|_| "expected string")?;
    state.creep_memory_set(TARGET.into(), target);
    Ok(())
}

fn transfer<'a, T>(state: &mut CreepState, target: &T) -> ExecutionResult
where
    T: Transferable,
{
    let creep = state.creep();
    if creep.pos().is_near_to(target) {
        let r = creep.transfer_all(target, ResourceType::Energy);
        if r != ReturnCode::Ok {
            debug!("couldn't unload: {:?}", r);
            state.creep_memory_remove(TARGET);
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
pub fn get_energy<'a>(state: &mut CreepState) -> ExecutionResult {
    let creep = state.creep();
    {
        let loading = state.creep_memory_bool(LOADING).unwrap_or(false);
        if !loading {
            Err("not loading")?;
        }
        if creep.carry_total() == creep.carry_capacity() {
            state.creep_memory_set(LOADING.into(), false);
            state.creep_memory_remove(TARGET);
            Err("full")?
        }
    }

    let target = find_container(state).ok_or_else(|| "no container found")?;
    withdraw(state, &target).map_err(|e| {
        state.creep_memory_remove(TARGET);
        e
    })
}

fn withdraw<'a>(state: &mut CreepState, target: &'a StructureContainer) -> ExecutionResult {
    let tasks = [
        Task::new(move |state: &mut CreepState| {
            let creep = state.creep();
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
    ];

    selector(state, tasks.iter())
}

fn find_container<'a>(state: &mut CreepState) -> Option<StructureContainer> {
    read_target_container(state).or_else(|| {
        trace!("Finding new withdraw target");
        state.creep_memory_remove(TARGET);
        let creep = state.creep();
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
            state.creep_memory_set(TARGET.into(), c.id());
            c
        })
    })
}

fn read_target_container<'a>(state: &CreepState) -> Option<StructureContainer> {
    state
        .creep_memory_string(TARGET)
        .and_then(|id| get_object_typed(id).ok())?
}

