mod containers;
mod extensions;

use super::bt::*;
use screeps::{
    memory,
    objects::{Room, RoomPosition},
};
use std::collections::HashSet;

#[derive(Debug, Clone, Hash, Eq, PartialEq)]
struct Pos {
    pub x: u32,
    pub y: u32,
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
        Task::new(move |_| build_roads(room)),
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

fn build_roads<'a>(_room: &'a Room) -> ExecutionResult {
    Err("unimplemented".into())
}

fn valid_construction_pos(room: &Room, pos: &RoomPosition, taken: &HashSet<Pos>) -> bool {
    let pp = Pos {
        x: pos.x(),
        y: pos.y(),
    };
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
    .all(|p| is_free(room, p) && !taken.contains(&Pos { x: p.x(), y: p.y() }))
}

fn is_free(room: &Room, pos: &RoomPosition) -> bool {
    let result = js! {
        const p = @{pos};
        const room = @{room};
        let objects = room.lookAt(p);
        try {
            return objects.find((o) => {
                return (o.type == "terrain" && o.terrain != "swamp" && o.terrain != "plain")
                    || (o.type == "structure" && o.structure != "road")
                    || o.type == "constructionSite";
            }) || null;
        } catch (e) {
            return null;
        }
    };
    result.is_null()
}

fn neighbours(pos: &RoomPosition) -> [RoomPosition; 8] {
    let x = pos.x();
    let y = pos.y();
    let name = pos.room_name();
    [
        RoomPosition::new(x - 1, y, name.as_str()),
        RoomPosition::new(x + 1, y, name.as_str()),
        RoomPosition::new(x, y - 1, name.as_str()),
        RoomPosition::new(x, y + 1, name.as_str()),
        RoomPosition::new(x - 1, y - 1, name.as_str()),
        RoomPosition::new(x - 1, y + 1, name.as_str()),
        RoomPosition::new(x + 1, y - 1, name.as_str()),
        RoomPosition::new(x + 1, y + 1, name.as_str()),
    ]
}
