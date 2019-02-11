mod builder;
mod harvester;
mod upgrader;

use super::bt::*;
use screeps::{self, objects::Creep, ReturnCode};

pub fn task<'a>() -> Node<'a> {
    let tasks = vec![run_creeps()];
    Node::Control(Control::Selector(tasks))
}

fn run_creeps<'a>() -> Node<'a> {
    let tasks = screeps::game::creeps::values()
        .into_iter()
        .map(|creep| run_creep(creep))
        .collect();

    Node::Control(Control::All(tasks))
}

fn run_creep<'a>(creep: Creep) -> Node<'a> {
    let task = move || {
        debug!("Running creep {}", creep.name());
        if creep.spawning() {
            return Ok(());
        }
        let tasks = vec![
            Task::new("run_role", || run_role(&creep)),
            Task::new("assing_role", || assign_role(&creep)),
        ]
        .into_iter()
        .map(|task| Node::Task(task))
        .collect();
        let tree = BehaviourTree::new(Control::Sequence(tasks));
        tree.tick()
    };
    let task = Task::new("run_creep", task);
    Node::Task(task)
}

fn assign_role<'a>(creep: &'a Creep) -> ExecutionResult {
    trace!("Assigning role to {}", creep.name());

    // TODO: more intelligent role assignment
    let time = screeps::game::creeps::keys().len();
    let time = time % 3;
    let result = match time {
        0 => "upgrader",
        1 => "harvester",
        2 => "builder",
        _ => unimplemented!(),
    };

    creep.memory().set("role", result); // TODO

    trace!("Assigned role {} to {}", result, creep.name());
    Ok(())
}

fn run_role<'a>(creep: &'a Creep) -> ExecutionResult {
    let role = creep
        .memory()
        .string("role")
        .map_err(|e| {
            error!("failed to read creep role {:?}", e);
        })?
        .ok_or_else(|| {
            trace!("creep role is null");
        })?;

    trace!("Running creep {} by role {}", creep.name(), role);

    let result = match role.as_str() {
        "harvester" => harvester::run(creep),
        "upgrader" => upgrader::run(creep),
        "builder" => builder::run(creep),
        _ => unimplemented!(),
    };

    if result.is_err() {
        warn!("Running creep {} failed", creep.name());
    }

    Ok(())
}

pub fn move_to<'a>(
    creep: &'a Creep,
    target: &'a impl screeps::RoomObjectProperties,
) -> ExecutionResult {
    let res = creep.move_to(target);
    match res {
        ReturnCode::Ok => Ok(()),
        _ => {
            warn!("Move failed {:?}", res);
            Err(())
        }
    }
}
