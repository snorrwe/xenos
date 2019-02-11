//! Build structures
//!
use super::super::bt::*;
use super::{get_energy, harvester, move_to, repairer};
use screeps::{
    constants::find,
    objects::{ConstructionSite, Creep},
    prelude::*,
    ReturnCode,
};

pub fn run<'a>(creep: &'a Creep) -> ExecutionResult {
    trace!("Running builder {}", creep.name());
    let tasks = vec![
        Task::new("build_0", |_| build(creep)),
        Task::new("get energy", |_| get_energy(creep)),
        Task::new("harvest", |_| harvest(creep)),
        Task::new("build_1", |_| build(creep)),
        // If nothing can be built
        Task::new("upgrade", |_| repairer::run(creep)),
    ]
    .into_iter()
    .map(|t| Node::Task(t))
    .collect();

    let tree = BehaviourTree::new(Control::Sequence(tasks));
    tree.tick()
}

fn harvest<'a>(creep: &'a Creep) -> ExecutionResult {
    trace!("Harvesting");

    let loading: bool = creep.memory().bool("loading");
    if !loading {
        return Err(());
    }
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

    let loading: bool = creep.memory().bool("loading");
    if loading {
        return Err(());
    }
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
