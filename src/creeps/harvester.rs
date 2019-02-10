use super::super::bt::*;
use screeps::{
    find,
    game::get_object_erased,
    objects::{Creep, Source},
    prelude::*,
    traits::TryFrom,
    ReturnCode,
};
use stdweb::InstanceOf;

pub fn run<'a>(creep: &'a Creep) -> ExecutionResult {
    trace!("Running harvester {}", creep.name());

    let tasks = vec![Task::new("harvest", || harvest(&creep))]
        .into_iter()
        .map(|task| Node::Task(task))
        .collect();
    let tree = BehaviourTree::new(Control::Sequence(tasks));
    tree.tick()
}

fn harvest<'a>(creep: &'a Creep) -> ExecutionResult {
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
            warn!("couldn't harvest: {:?}", r);
        }
    } else {
        creep.move_to(&source);
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

