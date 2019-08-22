use crate::prelude::*;
use screeps::HasPosition;
use screeps::game::flags;

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
            Task::new(move |state: &mut GameState| {
                let room = WorldPosition::parse_name(
                    &flag.pos().room_name()
                    ).unwrap();
                flag.remove();
                state.expansion.insert(room);
                Ok(())
            })
        })
        .collect();
    let seq = Control::Selector(tasks);
    Task::from(seq).with_name("Flags task")
}

