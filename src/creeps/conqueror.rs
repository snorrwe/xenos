//! Takes Rooms
//!
use super::{move_to, update_scout_info, TARGET};
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
        Task::new(move |state| claim_target(state, creep)).with_name("Claim target"),
        Task::new(move |state| set_target(state, creep)).with_name("Set target"),
        Task::new(move |state| reset_target(state, creep)).with_name("Reset target"),
    ]
    .into_iter()
    .cloned()
    .collect();

    let tree = Control::Sequence(tasks);
    Task::new(move |state| tree.tick(state))
        .with_required_bucket(300)
        .with_name("Conqueror")
}

fn reset_target<'a>(state: &mut GameState, creep: &'a Creep) -> ExecutionResult {
    trace!("Resetting conqueror target");

    if !state.creep_memory_bool(CreepName(&creep.name()), "loading") {
        Err("not loading")?;
    }
    let memory = state.creep_memory_entry(CreepName(&creep.name()));

    if creep.carry_total() == creep.carry_capacity() {
        memory.insert("loading".into(), false.into());
        memory.remove(TARGET);
        Err("full")?;
    }
    Ok(())
}

fn claim_target<'a>(state: &mut GameState, creep: &'a Creep) -> ExecutionResult {
    trace!("claiming room");

    let memory = state.creep_memory_entry(CreepName(&creep.name()));
    let flag = {
        let flag = memory
            .get(CONQUEST_TARGET)
            .and_then(|x| x.as_str())
            .ok_or_else(|| "no target set")?;
        game::flags::get(flag)
    };

    let flag = flag.ok_or_else(|| {
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

