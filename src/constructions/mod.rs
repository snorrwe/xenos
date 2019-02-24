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

    let time = screeps::game::time();
    let tasks = screeps::game::rooms::values()
        .into_iter()
        .enumerate()
        // Do not update all rooms in the same tick to hopefully reduce cpu load of contructions in
        // a single tick
        .filter(|(i, _)| (time + *i as u32) % 16 == 0)
        .map(|(_, room)| Task::new(move |state| manage_room(state, &room)))
        .collect();

    let task = Control::All(tasks);

    Task::new(move |state| {
        task.tick(state)
            .map_err(|_| "Failed all building subtasks".into())
    })
}

fn manage_room<'a>(state: &'a mut GameState, room: &'a Room) -> ExecutionResult {
    debug!("Manage constructionSites of room {:?}", room.name());
    let tasks = vec![
        Task::new(move |_| build_structures(room)),
        Task::new(move |_| containers::build_containers(room)),
        Task::new(move |_| roads::build_roads(room)),
    ];
    let tree = Control::All(tasks);
    tree.tick(state)
}

fn build_structures<'a>(room: &'a Room) -> ExecutionResult {
    let structures = [
        StructureType::Extension,
        StructureType::Extension,
        StructureType::Tower,
    ]
    .into_iter()
    .cloned()
    .collect();
    place_construction_sites(room, structures)
}

fn valid_construction_pos(room: &Room, pos: &RoomPosition, taken: &HashSet<Pos>) -> bool {
    let pp = Pos::new(pos.x(), pos.y());
    if taken.contains(&pp) {
        return false;
    }

    let x = pos.x();
    let y = pos.y();
    let name = pos.room_name();
    [
        RoomPosition::new(x - 1, y, name.as_str()),
        RoomPosition::new(x + 1, y, name.as_str()),
        RoomPosition::new(x, y - 1, name.as_str()),
        RoomPosition::new(x, y + 1, name.as_str()),
    ]
    .into_iter()
    .all(|p| is_free(room, p) && !taken.contains(&Pos::new(p.x(), p.y())))
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

    let mut todo = pos
        .neighbours()
        .into_iter()
        .cloned()
        .collect::<VecDeque<_>>();

    while !todo.is_empty() && !structures.is_empty() {
        let pos = todo.pop_front().unwrap();
        let pp = Pos::new(pos.x(), pos.y());
        if visited.contains(&pp) {
            continue;
        }

        visited.insert(pp.clone());
        let neighbour_pos = pos.neighbours();

        todo.extend(
            neighbour_pos
                .iter()
                .filter(|p| !visited.contains(&Pos::new(p.x(), p.y())))
                .cloned(),
        );

        if !valid_construction_pos(room, &pos, &construction) {
            continue;
        }

        let structure = structures.pop_front().unwrap();
        let result = room.create_construction_site(&pos, structure);
        match result {
            ReturnCode::Ok => {
                construction.insert(pp);
            }
            ReturnCode::RclNotEnough => {}
            ReturnCode::Full => return Err(format!("cant place extension {:?}", result)),
            _ => {
                structures.push_back(structure);
            }
        }
    }

    Ok(())
}
