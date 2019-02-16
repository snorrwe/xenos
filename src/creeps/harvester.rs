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
    traits::TryFrom,
    ReturnCode,
};
use std::collections::HashMap;
use stdweb::{unstable::TryInto, Reference};

const HARVEST_TARGET: &'static str = "harvest_target";

pub fn run<'a>(creep: &'a Creep) -> Task<'a> {
    trace!("Running harvester {}", creep.name());

    let tasks = vec![
        Task::new(move |_| attempt_harvest(&creep)),
        Task::new(move |_| unload(&creep)),
        Task::new(move |_| attempt_harvest(&creep)),
    ];

    let tree = Control::Sequence(tasks);
    Task::new(move |_| tree.tick())
}

fn unload<'a>(creep: &'a Creep) -> ExecutionResult {
    trace!("Unloading");
    let carry_total = creep.carry_total();

    if carry_total == 0 {
        trace!("Empty");
        creep.memory().del("target");
        return Err("empty".into());
    }

    let target = find_unload_target(creep).ok_or_else(|| String::new())?;

    let tasks = vec![
        Task::new(|_| try_transfer::<StructureContainer>(creep, &target)),
        Task::new(|_| try_transfer::<StructureSpawn>(creep, &target)),
    ]
    .into_iter()
    .collect();

    let tree = Control::Sequence(tasks);
    tree.tick().map_err(|_| {
        creep.memory().del("target");
        String::new()
    })
}

fn find_unload_target<'a>(creep: &'a Creep) -> Option<Reference> {
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
            Task::new(|_| find_container(creep)),
            Task::new(|_| find_spawn(creep)),
        ];
        let tree = Control::Sequence(tasks);
        tree.tick().unwrap_or_else(|e| {
            debug!("Failed to find unload target {:?}", e);
        });
        None
    }
}

fn try_transfer<'a, T>(creep: &'a Creep, target: &'a Reference) -> ExecutionResult
where
    T: Transferable + screeps::traits::TryFrom<&'a Reference>,
{
    let target = T::try_from(target.as_ref()).map_err(|_| String::new())?;
    transfer(creep, &target)
}

fn find_container<'a>(creep: &'a Creep) -> ExecutionResult {
    trace!("Finding new unload target");
    // screeps api is bugged at the moment and FIND_STRUCTURES panics
    let result = js! {
        let creep = @{creep};
        const container = creep.pos.findClosestByRange(FIND_STRUCTURES, {
            filter: function(i) { return i.structureType == STRUCTURE_CONTAINER &&
                           i.store[RESOURCE_ENERGY] < i.storeCapacity }
        });
        if (container) {
            creep.memory.target = container.id;
            return true;
        }
        return false;
    };

    if result.try_into().unwrap_or_else(|_| false) {
        Ok(())
    } else {
        Err(String::new())
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

pub fn attempt_harvest<'a>(creep: &'a Creep) -> ExecutionResult {
    trace!("Harvesting");

    let carry_total = creep.carry_total();
    let carry_cap = creep.carry_capacity();

    if carry_total == carry_cap {
        trace!("Full");
        creep.memory().del(HARVEST_TARGET);
        return Err(String::new());
    }

    let source = harvest_target(creep).map_err(|()| String::new())?;

    if creep.pos().is_near_to(&source) {
        let r = creep.harvest(&source);
        if r != ReturnCode::Ok {
            creep.memory().del(HARVEST_TARGET);
            debug!("Couldn't harvest: {:?}", r);
        }
    } else {
        move_to(creep, &source)?;
    }

    trace!("Harvest finished");
    Ok(())
}

fn harvest_target<'a>(creep: &'a Creep) -> Result<Source, ()> {
    trace!("Setting harvest target");

    let target = creep.memory().string(HARVEST_TARGET).map_err(|e| {
        error!("Failed to read creep target {:?}", e);
    })?;

    if let Some(target) = target {
        unwrap_harvest_target(creep, target)
    } else {
        trace!("Finding new harvest target");
        let room = creep.room();
        let harvester_count = harvester_count();
        let sources = js! {
            const room = @{room};
            let n_harvesters = @{harvester_count};
            n_harvesters  = room.find(FIND_SOURCES).map((source) => [source.id, n_harvesters[source.id] || 0]);
            let result = n_harvesters.reduce((result, source) => {
                if (result[1] > source[1]) {
                    return source;
                }
                return result;
            }, n_harvesters[0]);
            return result && result[0];
        };
        let source: String = sources.try_into().map_err(|e| {
            error!("Can't find Source in creep's room {:?}", e);
        })?;
        creep.memory().set(HARVEST_TARGET, &source);
        let source = unwrap_harvest_target(creep, source)?;
        Ok(source)
    }
}

fn unwrap_harvest_target(creep: &Creep, target: String) -> Result<Source, ()> {
    trace!("Validating existing target");
    let target = get_object_erased(target.as_str()).ok_or_else(|| {
        error!("Target by id {} does not exists", target);
    })?;
    let source = Source::try_from(target.as_ref()).map_err(|e| {
        error!("Failed to convert target to Source {:?}", e);
        creep.memory().del("target");
    })?;
    Ok(source)
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

