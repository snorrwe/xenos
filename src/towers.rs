use super::bt::*;
use screeps::{
    objects::{HasId, Room, RoomObjectProperties, StructureTower},
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
        .map(move |tower: StructureTower| Task::new(move |_| run_tower(&tower)))
        .collect();
    let tree = Control::All(tasks);
    Task::new(move |_| tree.tick())
}

fn run_tower<'a>(tower: &'a StructureTower) -> ExecutionResult {
    debug!("Running tower {:?}", tower.id());

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
        Ok(())
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
