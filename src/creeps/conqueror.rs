//! Takes Rooms
//!
use super::{super::bt::*, builder, harvester::attempt_harvest, move_to};
use screeps::{game, objects::Creep, prelude::*, ReturnCode};
use stdweb::unstable::TryInto;

const CONQUEST_TARGET: &'static str = "conquest_target";

pub fn run<'a>(creep: &'a Creep) -> Task<'a> {
    trace!("Running conqueror {}", creep.name());
    let tasks = vec![
        Task::new(move |_| claim_target(creep)),
        Task::new(move |_| set_target(creep)),
        Task::new(move |state| builder::attempt_build(state, creep)),
        Task::new(move |state| attempt_harvest(state, creep, None)),
        Task::new(move |_| reset_target(creep)),
    ];

    let tree = Control::Sequence(tasks);
    Task::new(move |state| tree.tick(state)).with_required_bucket(300)
}

fn reset_target<'a>(creep: &'a Creep) -> ExecutionResult {
    trace!("Resetting conqueror target");
    if !creep.memory().bool("loading") {
        Err("not loading")?;
    }

    if creep.carry_total() == creep.carry_capacity() {
        creep.memory().set("loading", false);
        creep.memory().del("target");
        Err("full")?;
    }
    Ok(())
}

fn claim_target<'a>(creep: &'a Creep) -> ExecutionResult {
    trace!("claiming room");
    let flag = creep
        .memory()
        .string(CONQUEST_TARGET)
        .map_err(|e| format!("failed to read 'target' {:?}", e))?
        .ok_or_else(|| String::from("no target set"))?;

    let flag = game::flags::get(flag.as_str()).ok_or_else(|| {
        creep.memory().del(CONQUEST_TARGET);
        String::from("target flag does not exist")
    })?;

    let room = creep.room();

    let arrived = js! {
        const flag = @{&flag};
        return @{&room}.name == (flag.room && flag.room.name);
    };
    let arrived: bool = arrived
        .try_into()
        .map_err(|e| format!("failed to convert result to bool {:?}", e))?;
    if !arrived {
        trace!("approaching target room");
        return move_to(creep, &flag);
    }

    let controller = room
        .controller()
        .ok_or_else(|| format!("room {:?} has no controller", room.name()))?;

    if controller.my() {
        return Err(format!("room is already claimed"));
    }

    let result = creep.claim_controller(&controller);

    match result {
        ReturnCode::Ok => Ok(()),
        ReturnCode::NotInRange => move_to(creep, &controller),
        _ => Err(format!("failed to claim controller {:?}", result)),
    }
}

fn set_target<'a>(creep: &'a Creep) -> ExecutionResult {
    trace!("finding target");

    if creep
        .memory()
        .string(CONQUEST_TARGET)
        .map_err(|e| format!("failed to read 'target' {:?}", e))?
        .is_some()
    {
        trace!("has target");
        return Err(String::from("Creep already has a target"));
    }
    let flags = game::flags::values();
    let flag = flags
        .iter()
        .next()
        .ok_or_else(|| String::from("can't find a flag"))?;

    creep.memory().set(CONQUEST_TARGET, flag.name());
    debug!("set target to {}", flag.name());

    Ok(())
}

