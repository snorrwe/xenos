//! Flag rooms as targets for expansion based on scout data
//!
use crate::prelude::*;
use screeps::constants::find;
use screeps::game;
use screeps::objects::OwnedStructureProperties;
use std::collections::HashMap;

pub fn run<'a>(state: &'a mut GameState) -> ExecutionResult {
    let tasks = [Task::new(remove_exp_markers)];
    selector(state, tasks.iter())
}

fn remove_exp_markers(state: &mut GameState) -> ExecutionResult {
    let rooms = game::rooms::values();

    let rooms = rooms
        .into_iter()
        .map(|r| (WorldPosition::from(&r), r))
        .collect::<HashMap<_, _>>();

    let mut retain = Vec::with_capacity(state.expansion.len());

    for exp in state.expansion.iter().cloned() {
        if let Some(room) = rooms.get(&exp) {
            let controller = room.controller();
            if controller.is_none() {
                continue;
            }
            let controller = controller.unwrap();
            // Help the room until it reaches level 4
            if controller.my() && controller.level() >= 4 {
                let spawn = room.find(find::MY_SPAWNS).len();
                if spawn == 0 {
                    retain.push(exp);
                }
            } else {
                retain.push(exp);
            }
        } else {
            retain.push(exp);
        }
    }

    state.expansion = retain;

    Ok(())
}

