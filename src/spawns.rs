use super::bt::*;
use creeps::roles::next_role;
use screeps::{self, objects::StructureSpawn, prelude::*, Part, ReturnCode};

/// Return the BehaviourTree that runs the spawns
pub fn task<'a>() -> Node<'a> {
    let tasks = screeps::game::spawns::values()
        .into_iter()
        .map(|spawn| {
            let task = Task::new("spawn_task", move |_| run_spawn(&spawn));
            Node::Task(task)
        })
        .collect();
    Node::Control(Control::Sequence(tasks))
}

fn run_spawn(spawn: &StructureSpawn) -> ExecutionResult {
    debug!("Running spawn {}", spawn.name());

    if next_role(&spawn.room()).is_none() {
        debug!("Skipping spawn due to overpopulation");
        return Ok(());
    }

    let body = [Part::Move, Part::Move, Part::Carry, Part::Work];
    spawn_creep(spawn, &body)?;

    Ok(())
}

fn spawn_creep(spawn: &StructureSpawn, body: &[Part]) -> ExecutionResult {
    trace!("Spawning creep");

    let name = screeps::game::time();
    let mut prefix = 0;
    let res = loop {
        let name = format!("{}{:x}", prefix, name);
        let res = spawn.spawn_creep(&body, &name);

        if res == ReturnCode::NameExists {
            prefix += 1;
        } else {
            debug!("Spawning creep: {}, result: {}", name, res as i32);
            break res;
        }
    };

    if res != ReturnCode::Ok {
        warn!("couldn't spawn: {:?}", res);
    }
    Ok(())
}
