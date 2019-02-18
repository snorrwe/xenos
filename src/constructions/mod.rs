mod extensions;

use super::bt::*;
use screeps::{memory, objects::Room};

#[derive(Debug, Clone, Hash, Eq, PartialEq)]
struct Pos {
    pub x: u32,
    pub y: u32,
}

/// Return the BehaviourTree that runs the spawns
pub fn task<'a>() -> Task<'a> {
    trace!("Building task");

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

    let tasks = vec![
        Task::new(move |_| build_roads(room)),
        Task::new(move |_| extensions::build_extensions(room)),
    ];
    let tree = Control::All(tasks);
    tree.tick()
}

fn build_roads<'a>(_room: &'a Room) -> ExecutionResult {
    Err("unimplemented".into())
}
