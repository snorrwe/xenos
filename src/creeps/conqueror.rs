//! Takes Rooms
//!
use super::{super::bt::*, builder, harvest, move_to};
use screeps::{game, objects::Creep, prelude::*, ReturnCode};
use stdweb::unstable::TryInto;

const CONQUEST_TARGET: &'static str = "conquest_target";

pub fn run<'a>(creep: &'a Creep) -> Task<'a> {
    trace!("Running conqueror {}", creep.name());
    let tasks = vec![
        Task::new(move |_| claim_target(creep)),
        Task::new(move |_| set_target(creep)),
        Task::new(move |_| builder::attempt_build(creep)),
        Task::new(move |_| harvest(creep)),
    ];

    let tree = Control::Sequence(tasks);
    Task::new(move |state| tree.tick(state))
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
        const creep = @{creep};
        const flag = @{&flag};
        return creep.room.name == (flag.room && flag.room.name);
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
