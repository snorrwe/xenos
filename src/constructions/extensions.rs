use super::*;
use screeps::{
    constants::StructureType,
    objects::{HasPosition, Room, RoomPosition, StructureSpawn},
    ReturnCode,
};
use std::collections::{HashSet, VecDeque};
use stdweb::unstable::TryFrom;

pub fn build_extensions<'a>(room: &'a Room) -> ExecutionResult {
    let spawn = js! {
        const room = @{room};
        const spawns = room.find(FIND_STRUCTURES, {
            filter: { structureType: STRUCTURE_SPAWN }
        });
        return spawns && spawns[0] || null;
    };

    if spawn.is_null() {
        return Err("No spawn in the room".into());
    }

    let pos = StructureSpawn::try_from(spawn)
        .map_err(|e| format!("failed to find spawn {:?}", e))?
        .pos();

    let mut visited = HashSet::with_capacity(100);
    visited.insert(Pos {
        x: pos.x(),
        y: pos.y(),
    });
    let mut construction = HashSet::with_capacity(5);

    let neighbour_pos = neighbours(&pos);
    let mut todo = neighbour_pos
        .into_iter()
        .filter(|p| is_free(room, p))
        .cloned()
        .collect::<VecDeque<_>>();

    while !todo.is_empty() && construction.len() < 5 {
        let pos = todo.pop_front().unwrap();
        let pp = Pos {
            x: pos.x(),
            y: pos.y(),
        };
        if visited.contains(&pp) {
            continue;
        }
        visited.insert(pp.clone());
        let neighbour_pos = neighbours(&pos);

        neighbour_pos
            .iter()
            .filter(|p| !visited.contains(&Pos { x: p.x(), y: p.y() }) && is_free(room, p))
            .cloned()
            .for_each(|p| todo.push_back(p));

        let valid = valid_construction_pos(room, &pos, &construction);
        if !valid {
            continue;
        }

        let result = place_extension(room, &pos);
        if result != ReturnCode::Ok && result != ReturnCode::RclNotEnough {
            return Err(format!("cant place extension {:?}", result));
        } else if result == ReturnCode::RclNotEnough {
            return Ok(());
        } else {
            construction.insert(pp);
            continue;
        }
    }

    Ok(())
}

fn place_extension<'a>(room: &'a Room, pos: &'a RoomPosition) -> ReturnCode {
    room.create_construction_site(pos.clone(), StructureType::Extension)
}

