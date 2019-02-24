use super::*;
use screeps::{
    constants::StructureType,
    objects::{HasPosition, Room, StructureSpawn},
    ReturnCode,
};
use std::collections::{HashSet, VecDeque};
use stdweb::unstable::TryFrom;

pub fn build_extensions<'a>(room: &'a Room) -> ExecutionResult {
    trace!("Building extensions in room {:?}", room.name());

    let spawn = js! {
        const room = @{room};
        const spawns = room.find(FIND_STRUCTURES, {
            filter: { structureType: STRUCTURE_SPAWN }
        });
        return spawns && spawns[0] || null;
    };

    if spawn.is_null() {
        let e = Err("No spawn in the room".into());
        trace!("{:?}", e);
        return e;
    }

    let pos = StructureSpawn::try_from(spawn)
        .map_err(|e| {
            let e = format!("failed to find spawn {:?}", e);
            trace!("{}", e);
            e
        })?
        .pos();

    let mut visited = HashSet::with_capacity(100);
    visited.insert(Pos::new(pos.x(), pos.y()));
    let mut construction = HashSet::with_capacity(5);

    let neighbour_pos = neighbours(&pos);
    let mut todo = neighbour_pos
        .into_iter()
        // .filter(|p| is_free(room, p))
        .cloned()
        .collect::<VecDeque<_>>();

    while !todo.is_empty() && construction.len() < 5 {
        let pos = todo.pop_front().unwrap();
        let pp = Pos::new(pos.x(), pos.y());
        if visited.contains(&pp) {
            continue;
        }
        trace!(
            "trying to construct on position {:?}, {}",
            pp,
            construction.len()
        );

        visited.insert(pp.clone());
        let neighbour_pos = neighbours(&pos);

        neighbour_pos
            .iter()
            .filter(|p| !visited.contains(&Pos::new(p.x(), p.y())))
            .cloned()
            .for_each(|p| todo.push_back(p));

        if !valid_construction_pos(room, &pos, &construction) {
            continue;
        }

        let result = room.create_construction_site(&pos, StructureType::Extension);
        trace!("extension construction result {:?}", result);
        match result {
            ReturnCode::Ok => {
                construction.insert(pp);
            }
            ReturnCode::RclNotEnough => return Ok(()),
            ReturnCode::Full => return Err(format!("cant place extension {:?}", result)),
            _ => {}
        }
    }

    Ok(())
}
