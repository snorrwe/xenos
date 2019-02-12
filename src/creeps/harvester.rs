//! Harvest energy and unload it to the appropriate target
//!
use super::super::bt::*;
use super::move_to;
use super::roles::count_roles_in_room;
use screeps::{
    constants::ResourceType,
    find,
    game::get_object_erased,
    objects::{Creep, Source, StructureContainer, StructureSpawn, Transferable},
    prelude::*,
    traits::TryFrom,
    ReturnCode,
};
use stdweb::{unstable::TryInto, Reference};

pub fn run<'a>(creep: &'a Creep) -> ExecutionResult {
    trace!("Running harvester {}", creep.name());

    let tasks = vec![
        Task::new("harvest_0", |_| attempt_harvest(&creep)),
        Task::new("unload", |_| unload(&creep)),
        Task::new("harvest_1", |_| attempt_harvest(&creep)),
    ]
    .into_iter()
    .map(|task| Node::Task(task))
    .collect();
    let tree = BehaviourTree::new(Control::Sequence(tasks));
    tree.tick()
}

fn unload<'a>(creep: &'a Creep) -> ExecutionResult {
    trace!("Unloading");
    let carry_total = creep.carry_total();

    if carry_total == 0 {
        trace!("Empty");
        return Err(());
    }

    let target = find_unload_target(creep).ok_or_else(|| {})?;

    let tasks = vec![
        Task::new("transfer container", |_| {
            try_transfer::<StructureContainer>(creep, &target)
        }),
        Task::new("transfer spawn", |_| {
            try_transfer::<StructureSpawn>(creep, &target)
        }),
    ]
    .into_iter()
    .map(|task| Node::Task(task))
    .collect();

    let tree = BehaviourTree::new(Control::Sequence(tasks));
    tree.tick().map_err(|_| {
        creep.memory().del("target");
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
            Node::Task(Task::new("find container", |_| find_container(creep))),
            Node::Task(Task::new("find spawn", |_| find_spawn(creep))),
        ];
        let tree = BehaviourTree::new(Control::Sequence(tasks));
        tree.tick().unwrap_or_else(|()| {
            warn!("Failed to find unload target");
        });
        None
    }
}

fn try_transfer<'a, T>(creep: &'a Creep, target: &'a Reference) -> ExecutionResult
where
    T: Transferable + screeps::traits::TryFrom<&'a Reference>,
{
    let target = T::try_from(target.as_ref()).map_err(|_| {})?;
    transfer(creep, &target)
}

fn find_container<'a>(creep: &'a Creep) -> ExecutionResult {
    trace!("Finding new unload target");
    // screeps api is bugged at the moment and FIND_STRUCTURES panics
    let result = js!{
        let creep = @{creep};
        const containers = creep.room.find(FIND_STRUCTURES, {
            filter: (i) => i.structureType == STRUCTURE_CONTAINER &&
                           i.store[RESOURCE_ENERGY] < i.storeCapacity
        });
        if (containers[0]) {
            let structure = containers[0];
            creep.memory.target = structure.id;
            return true;
        }
        return false;
    };

    if result.try_into().unwrap_or_else(|_| false) {
        Ok(())
    } else {
        Err(())
    }
}

fn find_spawn<'a>(creep: &'a Creep) -> ExecutionResult {
    trace!("Finding new unload target");
    let target = creep
        .pos()
        .find_closest_by_range(find::MY_SPAWNS)
        .ok_or_else(|| {})?;
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
            warn!("couldn't unload: {:?}", r);
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
        return Err(());
    }

    let source = harvest_target(creep)?;

    if creep.pos().is_near_to(&source) {
        let r = creep.harvest(&source);
        if r != ReturnCode::Ok {
            warn!("Couldn't harvest: {:?}", r);
        }
    } else {
        move_to(creep, &source)?;
    }

    trace!("Harvest finished");
    Ok(())
}

fn harvest_target<'a>(creep: &'a Creep) -> Result<Source, ()> {
    trace!("Setting harvest target");

    let target = creep.memory().string("harvest_target").map_err(|e| {
        error!("Failed to read creep target {:?}", e);
    })?;

    if let Some(target) = target {
        trace!("Validating existing target");
        let target = get_object_erased(target.as_str()).ok_or_else(|| {
            error!("Target by id {} does not exists", target);
        })?;
        let source = Source::try_from(target.as_ref()).map_err(|e| {
            error!("Failed to convert target to Source {:?}", e);
            creep.memory().del("target");
        })?;
        Ok(source)
    } else {
        trace!("Finding new harvest target");
        let room = creep.room();
        let count = count_roles_in_room(&room);
        let n_harvesters = count["harvester"];
        let n_harvesters = unsafe {
            // In case multiple creeps require harvest target in a single tick
            static mut N: i32 = 0;
            N += 1;
            n_harvesters as i32 + N
        };
        let sources = js!{
            const room = @{room};
            const sources = room.find(FIND_SOURCES) || [];
            const n_harvesters = @{n_harvesters};
            return sources && sources[n_harvesters % sources.length];
        };
        let source: Source = sources.try_into().map_err(|e| {
            error!("Can't find Source in creep's room {:?}", e);
        })?;
        creep.memory().set("harvest_target", source.id());
        Ok(source)
    }
}
