use super::bt::*;
use std::collections::HashSet;

pub fn game_loop() {
    info!("loop starting! CPU: {}", screeps::game::cpu::get_used());

    test_simple_graph();

    cleanup_memory().unwrap_or_else(|e| {
        error!("Failed to clean up memory {:?}", e);
    });

    info!("done! cpu: {}", screeps::game::cpu::get_used());
}

fn cleanup_memory() -> Result<(), Box<::std::error::Error>> {
    let alive_creeps: HashSet<String> = screeps::game::creeps::keys().into_iter().collect();

    let screeps_memory = match screeps::memory::root().dict("creeps")? {
        Some(v) => v,
        None => {
            warn!("not cleaning game creep memory: no Memory.creeps dict");
            return Ok(());
        }
    };

    for mem_name in screeps_memory.keys() {
        if !alive_creeps.contains(&mem_name) {
            debug!("cleaning up creep memory of dead creep {}", mem_name);
            screeps_memory.del(&mem_name);
        }
    }

    Ok(())
}

fn test_simple_graph() {
    info!("Running sample BT");

    let tasks = vec![
        Node::Task(Task::new("success1", &success)),
        Node::Control(Control::Sequence(vec![
            Node::Task(Task::new("fail1", &fail)),
            Node::Task(Task::new("fail1", &fail)),
            Node::Task(Task::new("fail1", &fail)),
            Node::Task(Task::new("success1", &success)),
            Node::Task(Task::new("fail1", &fail)),
            Node::Task(Task::new("fail1", &fail)),
        ])),
        Node::Task(Task::new("fail1", &fail)),
        Node::Task(Task::new("success2", &success)),
    ];

    let tree = BehaviourTree::new(Control::Selector(tasks));

    let result = tree.tick();

    assert_eq!(result.is_err(), true);
}

fn success() -> ExecutionResult {
    warn!("Success");
    Ok(())
}

fn fail() -> ExecutionResult {
    warn!("Failure");
    Err(())
}

