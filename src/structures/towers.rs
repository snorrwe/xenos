use super::bt::*;
use screeps::{
    constants::find,
    objects::{CanStoreEnergy, HasId, Room, RoomObjectProperties, Structure, StructureTower},
    ReturnCode,
};
use stdweb::{unstable::TryInto, Value};

/// Return the BehaviourTree that runs the spawns
pub fn task<'a>() -> Task<'a> {
    let structures = js! {
        return Object.values(Game.structures).filter((s) => s.structureType == STRUCTURE_TOWER) || [];
    };
    let towers: Vec<Value> = structures.try_into().expect("brah");
    let tasks = towers
        .into_iter()
        .map(move |t| t.try_into().expect("bro"))
        .map(move |tower: StructureTower| Task::new(move |state| run_tower(state, &tower)))
        .collect();
    let tree = Control::All(tasks);
    Task::new(move |state| tree.tick(state))
}

fn run_tower<'a>(state: &'a mut GameState, tower: &'a StructureTower) -> ExecutionResult {
    debug!("Running tower {:?}", tower.id());

    let tasks = [
        Task::new(move |_| attempt_attack(tower)),
        Task::new(move |_| attempt_repair(tower)),
    ]
    .into_iter()
    .cloned()
    .collect();

    let tree = Control::Sequence(tasks);
    tree.tick(state)
}

fn attempt_attack<'a>(tower: &'a StructureTower) -> ExecutionResult {
    let enemy = find_enemy(&tower.room());
    if let Some(enemy) = enemy {
        let res = tower.attack(&enemy);
        match res {
            ReturnCode::Ok | ReturnCode::RclNotEnough => Ok(()),
            _ => {
                let error = format!("failed ta attack enemy {:?}", res);
                error!("{}", error);
                Err(error)
            }
        }
    } else {
        Err("no target".into())
    }
}

fn find_enemy<'a>(room: &'a Room) -> Option<screeps::Creep> {
    room.find(find::CREEPS)
        .into_iter()
        .find(|creep| !creep.my())
}

pub fn attempt_repair<'a>(tower: &'a StructureTower) -> ExecutionResult {
    trace!("Repairing");

    if tower.energy() < tower.energy_capacity() * 3 / 4 {
        return Err("loading".into());
    }
    let target = find_repair_target(tower).ok_or_else(|| {
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

fn find_repair_target<'a>(tower: &'a StructureTower) -> Option<Structure> {
    trace!("Finding repair target");

    let room = tower.room();
    room.find(find::STRUCTURES).into_iter().find(|s| {
        s.as_attackable()
            .map(|s| s.hits() < s.hits_max())
            .unwrap_or(false)
    })
}

