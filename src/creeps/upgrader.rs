use super::super::bt::*;
use super::{harvester, move_to};
use screeps::{objects::Creep, prelude::*, ReturnCode};

pub fn run<'a>(creep: &'a Creep) -> ExecutionResult {
    trace!("Running upgrader {}", creep.name());

    let loading: bool = creep.memory().bool("loading");

    if loading {
        harvest(creep)
    } else {
        upgrade(creep)
    }
}

fn harvest<'a>(creep: &'a Creep) -> ExecutionResult {
    trace!("Harvesting");

    if creep.carry_total() == creep.carry_capacity() {
        creep.memory().set("loading", false);
        Err(())
    } else {
        harvester::harvest(creep)
    }
}

fn upgrade<'a>(creep: &'a Creep) -> ExecutionResult {
    trace!("Upgrading");

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
