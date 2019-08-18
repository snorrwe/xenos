//! Takes Rooms
//!
use super::{move_to, move_to_options, update_scout_info, CreepState, MoveToOptions};
use crate::prelude::*;
use screeps::{game, objects::Creep, prelude::*, ReturnCode};
use stdweb::unstable::TryInto;

const CONQUEST_TARGET: &'static str = "conquest_target";

pub fn task<'a>() -> Task<'a, CreepState> {
    let tasks = [
        Task::new(move |state: &mut CreepState| {
            update_scout_info(state).unwrap_or_else(|e| {
                error!("Failed to update scout info {:?}", e);
            });
            Err("continue")?
        })
        .with_name("Update scout info"),
        Task::new(move |state| claim_target(state)).with_name("Claim target"),
        Task::new(move |state| set_target(state)).with_name("Set target"),
        Task::new(move |state: &mut CreepState| sign_controller(state.creep(), "Become as Gods"))
            .with_name("Set target"),
    ]
    .into_iter()
    .cloned()
    .collect();

    let tree = Control::Sequence(tasks);
    Task::from(tree)
        .with_required_bucket(300)
        .with_name("Conqueror")
}

fn claim_target<'a>(state: &mut CreepState) -> ExecutionResult {
    trace!("claiming room");

    let flag = {
        let flag = state
            .creep_memory_string(CONQUEST_TARGET)
            .ok_or_else(|| "no target set")?;
        game::flags::get(flag)
    };

    let flag = flag.ok_or_else(|| {
        state.creep_memory_remove(CONQUEST_TARGET);
        "target flag does not exist"
    })?;
    let creep = state.creep();

    let room = creep.room();
    let room_name = state.current_room().to_string();
    let room_name = room_name.as_str();

    // The Rust Screeps api may panic here
    let arrived = js! {
        const flag = @{&flag};
        return @{&room_name} == (flag.room && flag.room.name) || false;
    };

    let arrived: bool = arrived
        .try_into()
        .map_err(|e| format!("failed to convert result to bool {:?}", e))?;
    if !arrived {
        trace!("approaching target room");
        return move_to_options(
            creep,
            &flag,
            MoveToOptions {
                reuse_path: Some(30),
            },
        );
    }

    let my = js! {
        return @{&room}.controller.my || false;
    };

    let my = my
        .try_into()
        .map_err(|e| format!("failed to convert 'my' to bool {:?}", e))?;

    if my {
        return Err("room is already claimed")?;
    }

    let controller = room
        .controller()
        .ok_or_else(|| format!("room {:?} has no controller", room.name()))?;

    let result = creep.claim_controller(&controller);

    match result {
        ReturnCode::Ok => Ok(()),
        ReturnCode::NotInRange => move_to(creep, &controller),
        _ => Err(format!("failed to claim controller {:?}", result))?,
    }
}

fn set_target<'a>(state: &mut CreepState) -> ExecutionResult {
    trace!("finding target");

    if state.creep_memory_string(CONQUEST_TARGET).is_some() {
        trace!("has target");
        Err("Creep already has a target")?;
    }
    let flags = game::flags::values();
    let flag = flags.iter().next().ok_or_else(|| "can't find a flag")?;

    state.creep_memory_set(CONQUEST_TARGET.into(), flag.name());
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
        result => Err(format!("failed to sign controller {:?}", result))?,
    }
}

