use super::super::bt::*;
use super::move_to;
use screeps::{
    constants::ResourceType,
    find,
    game::get_object_erased,
    objects::{Creep, Source, StructureSpawn as Spawn},
    prelude::*,
    traits::TryFrom,
    ReturnCode,
};
use stdweb::InstanceOf;

pub fn run<'a>(creep: &'a Creep) -> ExecutionResult {
    trace!("Running harvester {}", creep.name());

    let tasks = vec![
        Task::new("harvest", || harvest(&creep)),
        Task::new("unload", || unload(&creep)),
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

    let target = set_unload_target(creep)?;

    if creep.pos().is_near_to(&target) {
        let r = creep.transfer_all(&target, ResourceType::Energy);
        if r != ReturnCode::Ok {
            warn!("couldn't unload: {:?}", r);
        }
    } else {
        move_to(creep, &target)?;
    }

    trace!("Unloading finished");
    Ok(())
}

pub fn harvest<'a>(creep: &'a Creep) -> ExecutionResult {
    trace!("Harvesting");

    let carry_total = creep.carry_total();
    let carry_cap = creep.carry_capacity();

    if carry_total == carry_cap {
        trace!("Full");
        return Err(());
    }

    let source = set_harvest_target(creep)?;

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

fn set_harvest_target<'a>(creep: &'a Creep) -> Result<Source, ()> {
    trace!("Setting harvest target");

    let target = creep.memory().string("target").map_err(|e| {
        error!("failed to read creep target {:?}", e);
    })?;

    if let Some(target) = target {
        trace!("Validating existing target");
        let target = get_object_erased(target.as_str()).ok_or_else(|| {
            error!("Target by id {} does not exists", target);
        })?;
        if !Source::instance_of(target.as_ref()) {
            trace!("Existing target is not a Source");
            creep.memory().del("target");
            Err(())
        } else {
            let source = Source::try_from(target.as_ref()).map_err(|e| {
                error!("Failed to convert target to Source {:?}", e);
                creep.memory().del("target");
            })?;
            Ok(source)
        }
    } else {
        trace!("Finding new harvest target");
        let source = creep
            .pos()
            .find_closest_by_range(find::SOURCES)
            .ok_or_else(|| {
                error!("Can't find Source in creep's room");
            })?;
        creep.memory().set("target", source.id());
        Ok(source)
    }
}

fn set_unload_target<'a>(creep: &'a Creep) -> Result<Spawn, ()> {
    trace!("Setting unload target");

    let target = creep.memory().string("target").map_err(|e| {
        error!("failed to read creep target {:?}", e);
    })?;

    if let Some(target) = target {
        trace!("Validating existing target");
        let target = get_object_erased(target.as_str()).ok_or_else(|| {
            error!("Target by id {} does not exists", target);
        })?;
        if !Spawn::instance_of(target.as_ref()) {
            trace!("Existing target is not a Source");
            creep.memory().del("target");
            Err(())
        } else {
            let target = Spawn::try_from(target.as_ref()).map_err(|e| {
                error!("Failed to convert target to Source {:?}", e);
                creep.memory().del("target");
            })?;
            Ok(target)
        }
    } else {
        trace!("Finding new unload target");
        let target = creep
            .pos()
            .find_closest_by_range(find::MY_SPAWNS)
            .ok_or_else(|| {
                error!("Can't find Spawns in creep's room");
            })?;
        creep.memory().set("target", target.id());
        Ok(target)
    }
}
