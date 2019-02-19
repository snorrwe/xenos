use super::bt::*;
use screeps::{
    objects::{CanStoreEnergy, HasId, Room, RoomObjectProperties, StructureTower},
    ReturnCode,
};
use stdweb::{
    unstable::{TryFrom, TryInto},
    Value,
};

/// Return the BehaviourTree that runs the spawns
pub fn task<'a>() -> Task<'a> {
    let structures = js! {
        return Object.values(Game.structures).filter((s) => s.structureType == STRUCTURE_TOWER) || [];
    };
    let towers: Vec<Value> = structures.try_into().expect("brah");
    let tasks = towers
        .into_iter()
        .map(move |t| t.try_into().expect("bro"))
        .map(move |tower: StructureTower| Task::new(move |_| run_tower(&tower)))
        .collect();
    let tree = Control::All(tasks);
    Task::new(move |_| tree.tick())
}

fn run_tower<'a>(tower: &'a StructureTower) -> ExecutionResult {
    debug!("Running tower {:?}", tower.id());

    let tasks = [
        Task::new(move |_| attempt_attack(tower)),
        Task::new(move |_| attempt_repair(tower)),
    ]
    .into_iter()
    .cloned()
    .collect();

    let tree = Control::Sequence(tasks);
    tree.tick()
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
    let result = js! {
        const room = @{room};
        return room.find(FIND_CREEPS, {
            filter: (c) => !c.my,
        })[0];
    };
    result.try_into().ok()
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
    trace!("Got repair target {:?}", target);
    repair(tower, &target)
}

// TODO: return Structure once the Structure bug has been fixed in screeps api
fn repair<'a>(tower: &'a StructureTower, target: &'a String) -> ExecutionResult {
    let res = js! {
        const tower = @{tower};
        let target = @{target};
        target = Game.getObjectById(target);
        let result = tower.repair(target);
        return result;
    };
    let res = ReturnCode::try_from(res).map_err(|e| format!("Expected ReturnCode {:?}", e))?;
    if res == ReturnCode::Ok {
        Ok(())
    } else {
        let error = format!("Unexpected ReturnCode {:?}", res);
        Err(error)
    }
}

// TODO: return Structure once the Structure bug has been fixed in screeps api
fn find_repair_target<'a>(tower: &'a StructureTower) -> Option<String> {
    trace!("Finding repair target");

    let room = tower.room();
    let result = js! {
        const room = @{room};
        const candidates = room.find(FIND_STRUCTURES, {
            filter: function (s) { return s.hits < s.hitsMax; }
        });
        return candidates[0] && candidates[0].id;
    };

    String::try_from(result).ok()
}
