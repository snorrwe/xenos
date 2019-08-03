use crate::creeps::find_repair_target;
use crate::prelude::*;
use screeps::{
    constants::find,
    game,
    objects::{CanStoreEnergy, HasId, Room, RoomObjectProperties, Structure, StructureTower},
    ReturnCode,
};

/// Return the BehaviourTree that runs the spawns
pub fn task<'a>() -> Task<'a, GameState> {
    Task::new(move |state| {
        game::structures::values()
            .into_iter()
            .filter_map(|s| match s {
                Structure::Tower(t) => Some(t),
                _ => None,
            })
            .for_each(move |tower| {
                run_tower(state, &tower)
                    .map_err(|e| {
                        debug!("Tower in room {:?} is idle, {:?}", tower.room().name(), e);
                        e
                    })
                    .unwrap_or(())
            });
        Ok(())
    })
}

fn run_tower<'a>(state: &'a mut GameState, tower: &'a StructureTower) -> ExecutionResult {
    debug!("Running tower {:?}", tower.id());

    let tasks = [
        Task::new(move |_| attempt_attack(tower)),
        Task::new(move |_| attempt_repair(tower)).with_required_bucket(1000),
    ];
    sequence(state, tasks.iter())
}

fn attempt_attack<'a>(tower: &'a StructureTower) -> ExecutionResult {
    let enemy = find_enemy(&tower.room());
    if let Some(enemy) = enemy {
        let res = tower.attack(&enemy);
        match res {
            ReturnCode::Ok | ReturnCode::RclNotEnough => Ok(()),
            _ => {
                let error = format!("Failed to attack enemy {:?}", res);
                error!("{}", error);
                Err(error)
            }
        }
    } else {
        Err("no target".into())
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
        Err(error)
    }
}

