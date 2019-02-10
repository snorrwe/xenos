mod harvester;

use super::bt::*;
use screeps::{self, objects::Creep};

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

    let result = "harvester";
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
            error!("creep role is null");
        })?;

    trace!("Running creep {} by role {}", creep.name(), role);

    match role.as_str() {
        "harvester" => harvester::run(creep),
        _ => unimplemented!(),
    }
}

