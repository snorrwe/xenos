use super::bt::*;
use screeps;
use screeps::{objects::StructureSpawn, prelude::*, Part, ReturnCode};

/// Return the BehaviourTree that runs the spawns
pub fn task<'a>() -> Node<'a> {
    let tasks = vec![run_spawns()];
    Node::Control(Control::Sequence(tasks))
}

fn run_spawns<'a>() -> Node<'a> {
    let tasks = screeps::game::spawns::values()
        .into_iter()
        .map(|spawn| {
            let fun = move || run_spawn(&spawn);
            let task = Task::new("spawn_task", fun);
            Node::Task(task)
        })
        .collect();

    Node::Control(Control::Sequence(tasks))
}

fn run_spawn(spawn: &StructureSpawn) -> ExecutionResult {
    debug!("Running spawn {}", spawn.name());
    let body = [Part::Move, Part::Move, Part::Carry, Part::Work];

    if spawn.energy() >= body.iter().map(|p| p.cost()).sum() {
        let name = screeps::game::time();
        let mut additional = 0;
        let res = loop {
            let name = format!("{:x}_{}", name, additional);
            let res = spawn.spawn_creep(&body, &name);

            if res == ReturnCode::NameExists {
                additional += 1;
            } else {
                debug!("Spawning creep: {}, result: {}", name, res as i32);
                break res;
            }
        };

        if res != ReturnCode::Ok {
            warn!("couldn't spawn: {:?}", res);
        }
    }
    Ok(())
}

