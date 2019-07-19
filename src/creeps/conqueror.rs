//! Takes Rooms
//!
use super::{move_to, update_scout_info};
use crate::prelude::*;
use screeps::{game, objects::Creep, prelude::*, ReturnCode};
use stdweb::unstable::TryInto;

const CONQUEST_TARGET: &'static str = "conquest_target";

pub fn run<'a>(creep: &'a Creep) -> Task<'a, GameState> {
    trace!("Running conqueror {}", creep.name());
    let tasks = [
        Task::new(move |state| {
            update_scout_info(state, creep).unwrap_or_else(|e| {
                error!("Failed to update scout info {:?}", e);
            });
            Err("continue")?
        })
        .with_name("Update scout info"),
        claim_task(creep),
        Task::new(move |state| set_target(state, creep)).with_name("Set target"),
    ]
    .into_iter()
    .cloned()
    .collect();

    let tree = Control::Sequence(tasks);
    Task::from(tree)
        .with_required_bucket(300)
        .with_name("Conqueror")
}

fn claim_task<'a>(creep: &'a Creep) -> Task<'a, GameState> {
    Control::Selector(
        [
            Task::new(move |state| claim_target(state, creep)).with_name("Claim target"),
            Task::new(move |_| sign_controller(creep, "Frenetiq was here")) // TODO: more signatures
                .with_name("Sign controller"),
        ]
        .into_iter()
        .cloned()
        .collect(),
    )
    .into()
}

fn claim_target<'a>(state: &mut GameState, creep: &'a Creep) -> ExecutionResult {
    trace!("claiming room");

    let flag = {
        let flag = state
            .creep_memory_string(CreepName(&creep.name()), CONQUEST_TARGET)
            .ok_or_else(|| "no target set")?;
        game::flags::get(flag)
    };

    let flag = flag.ok_or_else(|| {
        let memory = state.creep_memory_entry(CreepName(&creep.name()));
        memory.remove(CONQUEST_TARGET);
        String::from("target flag does not exist")
    })?;

    let room = creep.room();

    // The Rust Screeps api may panic here
    let arrived = js! {
        const flag = @{&flag};
        return @{&room}.name == (flag.room && flag.room.name) || false;
    };

    let arrived: bool = arrived
        .try_into()
        .map_err(|e| format!("failed to convert result to bool {:?}", e))?;
    if !arrived {
        trace!("approaching target room");
        return move_to(creep, &flag);
    }

    let my = js! {
        return @{&room}.controller.my || false;
    };

    let my = my
        .try_into()
        .map_err(|e| format!("failed to convert 'my' to bool {:?}", e))?;

    if my {
        return Err(format!("room is already claimed"));
    }

    let controller = room
        .controller()
        .ok_or_else(|| format!("room {:?} has no controller", room.name()))?;

    let result = creep.claim_controller(&controller);

    match result {
        ReturnCode::Ok => Ok(()),
        ReturnCode::NotInRange => move_to(creep, &controller),
        _ => Err(format!("failed to claim controller {:?}", result)),
    }
}

fn set_target<'a>(state: &mut GameState, creep: &'a Creep) -> ExecutionResult {
    trace!("finding target");

    if state
        .creep_memory_string(CreepName(&creep.name()), CONQUEST_TARGET)
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

    let memory = state.creep_memory_entry(CreepName(&creep.name()));
    memory.insert(CONQUEST_TARGET.into(), flag.name().into());
    debug!("set target to {}", flag.name());

    Ok(())
}

pub fn sign_controller(creep: &Creep, msg: &str) -> ExecutionResult {
    let controller = creep
        .room()
        .controller()
        .ok_or_else(|| "Room has no controller")?;

    match creep.sign_controller(&controller, msg) {
        ReturnCode::Ok => Ok(()),
        ReturnCode::NotInRange => move_to(creep, &controller),
        result => Err(format!("failed to sign controller {:?}", result)),
    }
}

