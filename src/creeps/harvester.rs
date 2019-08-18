//! Harvest energy and unload it to the appropriate target
//!
use super::{gofer, move_to, CreepState, TARGET};
use crate::prelude::*;
use screeps::{
    constants::ResourceType,
    find, game,
    game::get_object_erased,
    objects::{Creep, Source, StructureContainer, Transferable},
    prelude::*,
    ReturnCode,
};
use std::collections::HashMap;
use stdweb::{
    unstable::{TryFrom, TryInto},
    Reference,
};

const HARVEST_TARGET: &'static str = "harvest_target";

pub fn task<'a>() -> Task<'a, CreepState> {
    let tasks = [
        Task::new(|state| attempt_harvest(state, None)).with_name("Attempt harvest"),
        Task::new(|state| {
            let tasks = [
                Task::new(|state| unload(state)),
                Task::new(|state| attempt_harvest(state, None)).with_name("Attempt harvest"),
            ];
            // On success attempt to continue harvesting right away
            selector(state, tasks.iter())
        }),
        Task::new(|state| unload(state)).with_name("Attempt unload"),
    ]
    .into_iter()
    .cloned()
    .collect();

    let tree = Control::Sequence(tasks);
    Task::from(tree).with_name("Harvester")
}

pub fn unload<'a>(state: &mut CreepState) -> ExecutionResult {
    let carry_total = state.creep().carry_total();
    if carry_total == 0 {
        trace!("Empty");
        state.creep_memory_remove(TARGET);
        Err("empty")?;
    }

    let target = find_unload_target(state).ok_or_else(|| {
        state.creep_memory_remove(TARGET);
        let error = String::from("could not find unload target");
        debug!("{}", error);
        error
    })?;

    let tasks = [
        Task::new(|state: &mut CreepState| {
            try_transfer::<StructureContainer>(state.creep(), &target)
        })
        .with_name("Try transfer container"),
        Task::new(|state: &mut CreepState| gofer::attempt_unload(state))
            .with_name("Attempt gofer unload"),
    ];

    sequence(state, tasks.iter()).map_err(|error| {
        state.creep_memory_remove(TARGET);
        debug!("failed to unload {:?}", error);
        error
    })
}

fn find_unload_target<'a>(state: &mut CreepState) -> Option<Reference> {
    trace!("Setting unload target");

    read_unload_target(state).or_else(|| {
        let tasks = [
            Task::new(|state| find_container(state)).with_name("Find container"),
            Task::new(|state| find_spawn(state)).with_name("Find spawn"),
        ];
        sequence(state, tasks.iter()).unwrap_or_else(|e| {
            debug!("Failed to find unload target {:?}", e);
        });
        read_unload_target(state)
    })
}

fn read_unload_target<'a>(state: &mut CreepState) -> Option<Reference> {
    let target = state.creep_memory_string(TARGET);

    if let Some(target) = target {
        trace!("Validating existing target");
        let target = get_object_erased(target)?;
        Some(target.as_ref().clone())
    } else {
        None
    }
}

fn try_transfer<'a, T>(creep: &'a Creep, target: &'a Reference) -> ExecutionResult
where
    T: Transferable + screeps::traits::TryFrom<&'a Reference>,
{
    let target = T::try_from(target.as_ref()).map_err(|_| format!("Bad type"))?;
    transfer(creep, &target)
}

fn find_container<'a>(state: &mut CreepState) -> ExecutionResult {
    trace!("Finding new unload target");

    let result = js! {
        let creep = @{state.creep()};
        const container = creep.pos.findClosestByRange(FIND_STRUCTURES, {
            filter: (i) => i.structureType == STRUCTURE_CONTAINER
                && i.store[RESOURCE_ENERGY] < i.storeCapacity
        });
        return container;
    };

    let container: Option<StructureContainer> = result.try_into().unwrap_or(None);
    if let Some(container) = container {
        state.creep_memory_set(TARGET.into(), container.id());
        Ok(())
    } else {
        let error = format!("No container was found");
        Err(error)
    }
}

fn find_spawn<'a>(state: &mut CreepState) -> ExecutionResult {
    debug!("Finding new unload target");

    let target = state
        .creep()
        .pos()
        .find_closest_by_range(find::MY_SPAWNS)
        .ok_or_else(|| String::new())?;
    state.creep_memory_set(TARGET.into(), target.id());
    Ok(())
}

fn transfer<'a, T>(creep: &'a Creep, target: &'a T) -> ExecutionResult
where
    T: Transferable,
{
    if creep.pos().is_near_to(target) {
        let r = creep.transfer_all(target, ResourceType::Energy);
        if r != ReturnCode::Ok {
            debug!("couldn't unload: {:?}", r);
            Err("")?;
        }
    } else {
        move_to(creep, target)?;
    }
    Ok(())
}

pub fn attempt_harvest<'a>(
    state: &mut CreepState,
    target_memory: Option<&'a str>,
) -> ExecutionResult {
    trace!("Harvesting");

    let target_memory = target_memory.unwrap_or(HARVEST_TARGET);
    let carry_total = state.creep().carry_total();
    let carry_cap = state.creep().carry_capacity();

    if carry_total == carry_cap {
        state.creep_memory_remove(target_memory);
        Err("full")?;
    }

    let source =
        harvest_target(state, target_memory).ok_or_else(|| format!("No harvest target found"))?;

    if state.creep().pos().is_near_to(&source) {
        let r = state.creep().harvest(&source);
        if r != ReturnCode::Ok {
            state.creep_memory_remove(target_memory);
            debug!("Couldn't harvest: {:?}", r);
        }
    } else {
        move_to(state.creep(), &source)?;
    }

    trace!("Harvest finished");
    Ok(())
}

fn harvest_target<'a>(state: &mut CreepState, target_memory: &'a str) -> Option<Source> {
    trace!("Setting harvest target");

    let target = state
        .creep_memory_string(target_memory)
        .and_then(|id| get_object_erased(id));

    if let Some(target) = target {
        trace!("Validating existing target");
        return Source::try_from(target.as_ref())
            .map_err(|e| {
                debug!("Failed to convert target to Source {:?}", e);
                state.creep_memory_remove(target_memory);
            })
            .ok();
    }

    find_harvest_target(state).map(|source| {
        state.creep_memory_set(target_memory.into(), source.id());
        source
    })
}

fn find_harvest_target<'a>(state: &mut CreepState) -> Option<Source> {
    trace!("Finding harvest target");

    let room = state.creep().room();
    let harvester_count = harvester_count(state);

    debug!(
        "harvester count in room {:?} {:#?}",
        room.name(),
        harvester_count
    );

    let sources = room.find(find::SOURCES);
    let mut sources = sources.into_iter();
    let first_source = sources.next()?;
    let first_dist = first_source.pos().get_range_to(&state.creep().pos());
    let first_count = harvester_count
        .get(&first_source.id())
        .map(|x| *x)
        .unwrap_or(0);
    let (source, _, _) = sources.fold((first_source, first_dist, first_count), |result, source| {
        let dist = source.pos().get_range_to(&state.creep().pos());
        let count = harvester_count.get(&source.id()).map(|x| *x).unwrap_or(0);
        if count < result.2 || (count == result.2 && dist < result.1) {
            (source, dist, count)
        } else {
            result
        }
    });
    Some(source)
}

fn harvester_count<'a>(state: &mut CreepState) -> HashMap<String, i32> {
    let mut result = HashMap::new();

    game::creeps::values().into_iter().for_each(|creep| {
        let target = state
            .get_game_state()
            .creep_memory_string(CreepName(&creep.name()), HARVEST_TARGET);
        if let Some(target) = target {
            *result.entry(target.to_string()).or_insert(0) += 1;
        }
    });
    result
}

