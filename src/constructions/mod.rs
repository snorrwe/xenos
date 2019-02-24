mod containers;
mod extensions;
mod roads;

use super::bt::*;
use screeps::objects::{Room, RoomPosition};
use std::collections::HashSet;
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
    extensions::build_extensions(room).unwrap_or_else(|e| {
        debug!("Failed to build extensions {:?}", e);
    });
    Ok(())
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

fn neighbours(pos: &RoomPosition) -> [RoomPosition; 8] {
    let x = pos.x();
    let y = pos.y();
    let name = pos.room_name();
    let name = name.as_str();
    [
        RoomPosition::new(x - 1, y, name),
        RoomPosition::new(x + 1, y, name),
        RoomPosition::new(x, y - 1, name),
        RoomPosition::new(x, y + 1, name),
        RoomPosition::new(x - 1, y - 1, name),
        RoomPosition::new(x - 1, y + 1, name),
        RoomPosition::new(x + 1, y - 1, name),
        RoomPosition::new(x + 1, y + 1, name),
    ]
}
