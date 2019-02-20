mod containers;
mod extensions;
mod roads;

use super::bt::*;
use screeps::{
    memory,
    objects::{Room, RoomPosition},
};
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

    if screeps::game::time() % 16 != 2 {
        trace!("Skipping build");
        return Task::new(move |_| Ok(()));
    }

    let tasks = screeps::game::rooms::values()
        .into_iter()
        .map(|room| move |_| manage_room(&room))
        .map(|task| Task::new(task))
        .collect();
    let task = Control::All(tasks);

    Task::new(move |_| {
        task.tick()
            .map_err(|_| "Failed all building subtasks".into())
    })
}

fn manage_room<'a>(room: &'a Room) -> ExecutionResult {
    let tasks = vec![
        Task::new(move |_| build_structures(room)),
        Task::new(move |_| containers::build_containers(room)),
        Task::new(move |_| roads::build_roads(room)),
    ];
    let tree = Control::All(tasks);
    tree.tick()
}

fn build_structures<'a>(room: &'a Room) -> ExecutionResult {
    let rcl = room
        .controller()
        .ok_or_else(|| format!("room {} has no controller", room.name()))?
        .level();

    let memory = memory::root();

    let rcl_path = format!("roomManagement.{}.builtGcl", room.name());

    let last_rcl = memory
        .path_i32(&rcl_path)
        .map_err(|e| format!("failed to get rcl of room {} {:?}", room.name(), e))?
        .unwrap_or(0) as u32;

    if last_rcl == rcl {
        // TODO: check for damages
        trace!("nothing to do in room {}", room.name());
        return Ok(());
    }

    memory.path_set(&rcl_path, rcl);
    let tasks = vec![Task::new(move |_| extensions::build_extensions(room))];
    let tree = Control::All(tasks);
    tree.tick()
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
                return (o.type == "terrain" && o.terrain != "swamp" && o.terrain != "plain")
                    || (o.type == "structure" && o.structure != "road")
                    || o.type == "constructionSite";
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
