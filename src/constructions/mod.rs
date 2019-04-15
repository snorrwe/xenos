mod containers;
mod neighbours;
mod roads;

use self::neighbours::*;
use super::bt::*;
use screeps::{
    constants::StructureType,
    objects::{HasPosition, Room, RoomPosition, StructureSpawn},
    ReturnCode,
};
use std::collections::{HashSet, VecDeque};
use stdweb::unstable::TryFrom;

#[derive(Debug, Clone, Hash, Eq, PartialEq)]
struct Pos {
    pub x: u32,
    pub y: u32,
}

impl Pos {
    pub fn new(x: u32, y: u32) -> Self {
        Self { x: x, y: y }
    }
}

/// Return the BehaviourTree that runs the spawns
pub fn task<'a>() -> Task<'a> {
    trace!("Init construction task");

    Task::new(move |_| {
        let time = screeps::game::time();
        let rooms = screeps::game::rooms::values();
        let len = rooms.len() as u32;

        let index = time % len;
        let room = &rooms[index as usize];

        manage_room(room)
    })
    .with_required_bucket(3000)
}

fn manage_room<'a>(room: &'a Room) -> ExecutionResult {
    info!("Manage constructionSites of room {:?}", room.name());

    build_structures(room).unwrap_or_else(|e| warn!("Failed build_structures {:?}", e));
    containers::build_containers(room).unwrap_or_else(|e| warn!("Failed containers {:?}", e));
    roads::build_roads(room).unwrap_or_else(|e| warn!("Failed roads {:?}", e));

    Ok(())
}

fn build_structures<'a>(room: &'a Room) -> ExecutionResult {
    let structures = [
        StructureType::Tower,
        StructureType::Extension,
        StructureType::Extension,
        StructureType::Extension,
        StructureType::Extension,
        StructureType::Extension,
        StructureType::Spawn,
    ]
    .into_iter()
    .map(|x| *x)
    .collect();
    place_construction_sites(room, structures)
}

fn valid_construction_pos(room: &Room, pos: &RoomPosition, taken: &mut HashSet<Pos>) -> bool {
    let pp = Pos::new(pos.x(), pos.y());
    if taken.contains(&pp) {
        return false;
    }

    let x = pos.x();
    let y = pos.y();

    if x <= 2 || y <= 2 || x >= 48 || y >= 48 {
        // Out of bounds
        return false;
    }

    let name = pos.room_name();
    [
        RoomPosition::new(x - 1, y, name.as_str()),
        RoomPosition::new(x + 1, y, name.as_str()),
        RoomPosition::new(x, y - 1, name.as_str()),
        RoomPosition::new(x, y + 1, name.as_str()),
    ]
    .into_iter()
    .all(|p| {
        let pos = Pos::new(p.x(), p.y());
        if taken.contains(&pos) {
            false
        } else {
            let result = is_free(room, p);
            if !result {
                taken.insert(pos);
            }
            result
        }
    })
}

fn is_free(room: &Room, pos: &RoomPosition) -> bool {
    let result = js! {
        const pos = @{pos};
        const room = @{room};
        try {
            let objects = room.lookAt(pos);
            let invalidNeighbour = objects.find((o) => {
                return (o.type == LOOK_TERRAIN && o.terrain == "wall")
                    || (o.type == LOOK_STRUCTURES && o.structure.structureType != STRUCTURE_ROAD)
                    || o.type == LOOK_CONSTRUCTION_SITES;
            });
            return invalidNeighbour == null;
        } catch (e) {
            return false;
        }
    };
    bool::try_from(result).unwrap_or(false)
}

pub fn place_construction_sites<'a>(
    room: &'a Room,
    mut structures: VecDeque<StructureType>,
) -> ExecutionResult {
    trace!("Building extensions in room {:?}", room.name());

    let spawn = js! {
        const room = @{room};
        const spawns = room.find(FIND_STRUCTURES, {
            filter: { structureType: STRUCTURE_SPAWN }
        });
        return spawns && spawns[0] || null;
    };

    if spawn.is_null() {
        let e = Err("No spawn in the room");
        trace!("{:?}", e);
        e?;
    }

    let pos = StructureSpawn::try_from(spawn)
        .map_err(|e| {
            let e = format!("failed to find spawn {:?}", e);
            trace!("{}", e);
            e
        })?
        .pos();

    let mut visited = HashSet::with_capacity(1000);
    visited.insert(Pos::new(pos.x(), pos.y()));
    let mut construction = HashSet::with_capacity(500);

    let mut todo = pos
        .neighbours()
        .into_iter()
        .cloned()
        .collect::<VecDeque<_>>();

    let mut limit = 1000;

    while !todo.is_empty() && !structures.is_empty() && limit > 0 {
        limit -= 1;

        let pos = todo.pop_front().unwrap();
        let x = pos.x();
        let y = pos.y();
        let pp = Pos::new(x, y);
        if x <= 2 || 48 <= x || y <= 2 || 48 <= y || visited.contains(&pp) {
            continue;
        }

        visited.insert(pp.clone());
        let neighbour_pos = pos.neighbours();

        todo.extend(
            neighbour_pos
                .into_iter()
                .filter(|p| !visited.contains(&Pos::new(p.x(), p.y())))
                .cloned(),
        );

        if !valid_construction_pos(room, &pos, &mut construction) {
            continue;
        }

        let structure = structures.pop_front().unwrap();
        let result = room.create_construction_site(&pos, structure);
        match result {
            ReturnCode::Ok => {
                construction.insert(pp);
            }
            ReturnCode::RclNotEnough => break,
            ReturnCode::Full => Err(format!("cant place extension {:?}", result))?,
            _ => {
                structures.push_back(structure);
            }
        }
    }

    Ok(())
}

