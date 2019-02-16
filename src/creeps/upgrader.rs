//! Upgrade Controllers
//!
use super::super::bt::*;
use super::{get_energy, harvester, move_to};
use screeps::{objects::Creep, prelude::*, ReturnCode};

pub fn run<'a>(creep: &'a Creep) -> Task<'a> {
    trace!("Running upgrader {}", creep.name());

    let tasks = vec![
        Task::new(move |_| attempt_upgrade(creep)),
        Task::new(move |_| get_energy(creep)),
        Task::new(move |_| harvest(creep)),
        Task::new(move |_| attempt_upgrade(creep)),
    ];

    let tree = Control::Sequence(tasks);
    Task::new(move |_| tree.tick())
}

fn harvest<'a>(creep: &'a Creep) -> ExecutionResult {
    trace!("Harvesting");

    let loading: bool = creep.memory().bool("loading");
    if !loading {
        return Err("not loading".into());
    }
    if creep.carry_total() == creep.carry_capacity() {
        creep.memory().set("loading", false);
        Err("full".into())
    } else {
        harvester::attempt_harvest(creep)
    }
}

pub fn attempt_upgrade<'a>(creep: &'a Creep) -> ExecutionResult {
    trace!("Upgrading");
    let loading: bool = creep.memory().bool("loading");
    if loading {
        return Err("loading".into());
    }
    if creep.carry_total() == 0 {
        creep.memory().set("loading", true);
        Err("empty".into())
    } else {
        let controller = creep.room().controller().ok_or_else(|| {
            let error = format!("Creep has no access to a controller in the room!");
            error!("{}", error);
            error
        })?;
        let res = creep.upgrade_controller(&controller);
        match res {
            ReturnCode::Ok => Ok(()),
            ReturnCode::NotInRange => move_to(creep, &controller),
            _ => {
                let error = format!("Failed to upgrade controller {:?}", res);
                error!("{}", error);
                Err(error)
            }
        }
    }
}

