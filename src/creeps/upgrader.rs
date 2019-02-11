//! Upgrade Controllers
//!
use super::super::bt::*;
use super::{get_energy, harvester, move_to};
use screeps::{objects::Creep, prelude::*, ReturnCode};

pub fn run<'a>(creep: &'a Creep) -> ExecutionResult {
    trace!("Running upgrader {}", creep.name());

    let tasks = vec![
        Task::new("upgrade_0", || upgrade(creep)),
        Task::new("get energy", || get_energy(creep)),
        Task::new("harvest", || harvest(creep)),
        Task::new("upgrade_1", || upgrade(creep)),
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
        Err(())
    } else {
        harvester::harvest(creep)
    }
}

fn upgrade<'a>(creep: &'a Creep) -> ExecutionResult {
    trace!("Upgrading");
    let loading: bool = creep.memory().bool("loading");
    if loading {
        return Err(());
    }
    if creep.carry_total() == 0 {
        creep.memory().set("loading", true);
        Err(())
    } else {
        let controller = creep.room().controller().ok_or_else(|| {
            error!("Creep has no access to a controller in the room!");
        })?;
        let res = creep.upgrade_controller(&controller);
        match res {
            ReturnCode::Ok => Ok(()),
            ReturnCode::NotInRange => move_to(creep, &controller),
            _ => {
                error!("Failed to upgrade controller {:?}", res);
                Err(())
            }
        }
    }
}
