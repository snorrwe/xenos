use crate::creeps::find_repair_target;
use crate::prelude::*;
use screeps::{
    constants::find,
    game,
    objects::{CanStoreEnergy, HasId, Room, RoomObjectProperties, Structure, StructureTower},
    ReturnCode,
};

pub fn run<'a>(state: &mut GameState) -> ExecutionResult {
    game::structures::values()
        .into_iter()
        .filter_map(|s| match s {
            Structure::Tower(t) => Some(t),
            _ => None,
        })
        .for_each(move |tower| {
            let mut state = WrappedState::new(tower, state);
            run_tower(&mut state)
                .map_err(move |e| {
                    debug!("Tower in room {:?} is idle, {:?}", state.item.room().name(), e);
                    e
                })
                .unwrap_or(())
        });
    Ok(())
}

fn run_tower<'a>(tower: &mut WrappedState<StructureTower, GameState>) -> ExecutionResult {
    debug!("Running tower {:?}", tower.item.id());

    let tasks = [
        Task::new(move |tower: &mut WrappedState<StructureTower, GameState>| attempt_attack(&tower.item)),
        Task::new(move |tower: &mut WrappedState<StructureTower, GameState>| attempt_repair(&tower.item))
            .with_required_bucket(1000),
    ];
    sequence(tower, tasks.iter())
}

fn attempt_attack<'a>(tower: &'a StructureTower) -> ExecutionResult {
    let enemy = find_enemy(&tower.room());
    if let Some(enemy) = enemy {
        let res = tower.attack(&enemy);
        match res {
            ReturnCode::Ok | ReturnCode::RclNotEnough => Ok(()),
            _ => {
                error!("Failed to attack enemy {:?}", res);
                Err("Failed to attack enemy")?
            }
        }
    } else {
        Err("no target")?
    }
}

fn find_enemy<'a>(room: &'a Room) -> Option<screeps::Creep> {
    room.find(find::HOSTILE_CREEPS).into_iter().next()
}

pub fn attempt_repair<'a>(tower: &'a StructureTower) -> ExecutionResult {
    trace!("Repairing");

    if tower.energy() < tower.energy_capacity() * 3 / 4 {
        return Err("loading".into());
    }
    let target = find_repair_target(&tower.room()).ok_or_else(|| {
        let error = format!("Could not find a repair target");
        debug!("{}", error);
        error
    })?;
    trace!("Got repair target {:?}", target.id());
    repair(tower, &target)
}

fn repair<'a>(tower: &'a StructureTower, target: &'a Structure) -> ExecutionResult {
    let res = tower.repair(target);
    if res == ReturnCode::Ok {
        Ok(())
    } else {
        let error = format!("Unexpected ReturnCode {:?}", res);
        Err(error)?
    }
}

