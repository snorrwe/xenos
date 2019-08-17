use crate::prelude::*;
use screeps::constants::find;
use screeps::game::flags;
use screeps::objects::{Flag, OwnedStructureProperties, Structure};
use screeps::traits::TryInto;
use screeps::RoomObjectProperties;

pub fn task<'a>() -> Task<'a, GameState> {
    let flags = flags::values();
    if flags.len() == 0 {
        return Task::new(|_| {
            debug!("No flags to manage");
            Ok(())
        });
    }
    let tasks = flags
        .into_iter()
        .map(|flag| {
            let name = flag.name();
            let name = format!("Flag {}", name);
            Task::new(move |_game_state| {
                check_controller(&flag)?;
                Err("continue".to_owned())
            })
            .with_name(&name)
        })
        .collect();
    let seq = Control::Sequence(tasks);
    Task::new(move |state| seq.tick(state)).with_name("Flags task")
}

fn check_controller(flag: &Flag) -> ExecutionResult {
    let controller = js! {
        const flag = @{flag};
        return flag && flag.room && flag.room.controller;
    };
    let controller: Option<Structure> = controller.try_into().map_err(|e| {
        error!("Failed to convert to structure {:?}", e);
        "Conversion failure"
    })?;
    let controller = controller.ok_or("Controller could not be read")?;
    match controller {
        Structure::Controller(ref controller) => {
            let level = controller.level();
            if controller.my() && level >= 3 {
                let spawn = flag.room().find(find::MY_SPAWNS).len();
                if spawn > 0 {
                    flag.remove();
                }
            }
        }
        _ => {
            let err = format!("Expected Controller");
            error!("{}", err);
            Err(err)?;
        }
    }
    Ok(())
}
