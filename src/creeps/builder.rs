use super::super::bt::*;
use super::{harvester, move_to};
use screeps::{
    constants::find,
    objects::{ConstructionSite, Creep},
    prelude::*,
    ReturnCode,
};

pub fn run<'a>(creep: &'a Creep) -> ExecutionResult {
    trace!("Running builder {}", creep.name());

    let loading = creep.memory().bool("loading");

    if loading {
        harvest(creep)
    } else {
        build(creep)
    }
}

fn harvest<'a>(creep: &'a Creep) -> ExecutionResult {
    trace!("Harvesting");

    if creep.carry_total() == creep.carry_capacity() {
        creep.memory().set("loading", false);
        creep.memory().del("target");
        Ok(())
    } else {
        harvester::harvest(creep)
    }
}

fn build<'a>(creep: &'a Creep) -> ExecutionResult {
    trace!("Building");

    if creep.carry_total() == 0 {
        creep.memory().set("loading", true);
        Err(())
    } else {
        let target = find_build_target(creep).ok_or_else(|| {
            warn!("Could not find a build target");
        })?;
        let res = creep.build(&target);
        match res {
            ReturnCode::Ok => Ok(()),
            ReturnCode::NotInRange => move_to(creep, &target),
            _ => {
                error!("Failed to build target {:?} {:?}", res, target.id());
                Err(())
            }
        }
    }
}

fn find_build_target<'a>(creep: &'a Creep) -> Option<ConstructionSite> {
    creep
        .pos()
        .find_closest_by_range(find::MY_CONSTRUCTION_SITES)
}
