//! Harvest energy and unload it to the appropriate target
//!
use super::super::bt::*;
use super::move_to;
use screeps::{
    constants::ResourceType,
    find, game,
    game::get_object_erased,
    objects::{Creep, Source, StructureContainer, StructureSpawn, Transferable},
    prelude::*,
    ReturnCode,
};
use std::collections::HashMap;
use stdweb::{
    unstable::{TryFrom, TryInto},
    Reference,
};

const HARVEST_TARGET: &'static str = "harvest_target";

pub fn run<'a>(creep: &'a Creep) -> Task<'a> {
    trace!("Running harvester {}", creep.name());

    let tasks = vec![
        Task::new(move |_| attempt_harvest(&creep, None)),
        Task::new(move |state| unload(state, &creep)),
        Task::new(move |_| attempt_harvest(&creep, None)),
    ];

    let tree = Control::Sequence(tasks);
    Task::new(move |state| tree.tick(state))
}

fn unload<'a>(state: &'a mut GameState, creep: &'a Creep) -> ExecutionResult {
    trace!("Unloading");

    let carry_total = creep.carry_total();
    if carry_total == 0 {
        trace!("Empty");
        creep.memory().del("target");
        return Err("empty".into());
    }

    let target = find_unload_target(state, creep).ok_or_else(|| {
        creep.memory().del("target");
        let error = String::from("could not find unload target");
        debug!("{}", error);
        error
    })?;

    let tasks = vec![
        Task::new(|_| try_transfer::<StructureContainer>(creep, &target)),
        Task::new(|_| try_transfer::<StructureSpawn>(creep, &target)),
    ];

    let tree = Control::Sequence(tasks);
    tree.tick(state).map_err(|error| {
        creep.memory().del("target");
        debug!("failed to unload {:?}", error);
        error
    })
}

fn find_unload_target<'a>(state: &'a mut GameState, creep: &'a Creep) -> Option<Reference> {
    trace!("Setting unload target");

    read_unload_target(creep).or_else(|| {
        let tasks = vec![
            Task::new(|_| find_container(creep)),
            Task::new(|_| find_spawn(creep)),
        ];
        let tree = Control::Sequence(tasks);
        tree.tick(state).unwrap_or_else(|e| {
            debug!("Failed to find unload target {:?}", e);
        });
        read_unload_target(creep)
    })
}

fn read_unload_target<'a>(creep: &'a Creep) -> Option<Reference> {
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

fn find_container<'a>(creep: &'a Creep) -> ExecutionResult {
    trace!("Finding new unload target");
    let result = js! {
        let creep = @{creep};
        const container = creep.pos.findClosestByRange(FIND_STRUCTURES, {
            filter: (i) => i.structureType == STRUCTURE_CONTAINER
                && i.store[RESOURCE_ENERGY] < i.storeCapacity
        });
        if (container) {
            creep.memory.target = container.id;
            return true;
        }
        return false;
    };

    if result.try_into().unwrap_or_else(|_| false) {
        trace!("unload target found");
        Ok(())
    } else {
        let error = format!("No container was found");
        trace!("{}", error);
        Err(error)
    }
}

fn find_spawn<'a>(creep: &'a Creep) -> ExecutionResult {
    trace!("Finding new unload target");
    let target = creep
        .pos()
        .find_closest_by_range(find::MY_SPAWNS)
        .ok_or_else(|| String::new())?;
    creep.memory().set("target", target.id());
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
            return Err(String::new());
        }
    } else {
        move_to(creep, target)?;
    }
    Ok(())
}

pub fn attempt_harvest<'a>(creep: &'a Creep, target_memory: Option<&'a str>) -> ExecutionResult {
    trace!("Harvesting");

    let target_memory = target_memory.unwrap_or(HARVEST_TARGET);

    let carry_total = creep.carry_total();
    let carry_cap = creep.carry_capacity();

    if carry_total == carry_cap {
        trace!("Full");
        creep.memory().del(target_memory);
        return Err(String::new());
    }

    let source =
        harvest_target(creep, target_memory).ok_or_else(|| format!("No harvest target found"))?;

    if creep.pos().is_near_to(&source) {
        let r = creep.harvest(&source);
        if r != ReturnCode::Ok {
            creep.memory().del(target_memory);
            debug!("Couldn't harvest: {:?}", r);
        }
    } else {
        move_to(creep, &source)?;
    }

    trace!("Harvest finished");
    creep.memory().del("target");
    Ok(())
}

fn harvest_target<'a>(creep: &'a Creep, target_memory: &'a str) -> Option<Source> {
    trace!("Setting harvest target");

    let target = creep
        .memory()
        .string(target_memory)
        .map_err(|e| {
            error!("Failed to read creep target {:?}", e);
        })
        .ok()?;

    let source = if let Some(target) = target {
        trace!("Validating existing target");
        let target = get_object_erased(target.as_str())?;
        Source::try_from(target.as_ref())
            .map_err(|e| {
                debug!("Failed to convert target to Source {:?}", e);
                creep.memory().del(target_memory);
            })
            .ok()?
    } else {
        find_harvest_target(creep).map(|source| {
            creep.memory().set(target_memory, source.id());
            source
        })?
    };

    Some(source)
}

fn find_harvest_target<'a>(creep: &'a Creep) -> Option<Source> {
    trace!("Finding harvest target");
    let room = creep.room();
    let harvester_count = harvester_count();

    debug!(
        "harvester count in room {:?} {:#?}",
        room.name(),
        harvester_count
    );

    let sources = room.find(find::SOURCES);
    let mut sources = sources.into_iter();
    let first_source = sources.next()?;
    let first_dist = first_source.pos().get_range_to(&creep.pos());
    let first_count = harvester_count
        .get(&first_source.id())
        .map(|x| *x)
        .unwrap_or(0);
    let (source, _, _) = sources.fold((first_source, first_dist, first_count), |result, source| {
        let dist = source.pos().get_range_to(&creep.pos());
        let count = harvester_count.get(&source.id()).map(|x| *x).unwrap_or(0);
        if count < result.2 || (count == result.2 && dist < result.1) {
            (source, dist, count)
        } else {
            result
        }
    });
    Some(source)
}

fn harvester_count() -> HashMap<String, i32> {
    let mut result = HashMap::new();
    game::creeps::values().into_iter().for_each(|creep| {
        let target = creep.memory().string(HARVEST_TARGET);
        if let Ok(target) = target {
            if let Some(target) = target {
                *result.entry(target).or_insert(0) += 1;
            }
        }
    });
    result
}

