use crate::prelude::*;
use screeps::game::flags;
use screeps::HasPosition;

pub fn run<'a>(state: &mut GameState) -> ExecutionResult {
    let flags = flags::values();
    flags.into_iter().for_each(move |flag| {
        let room = WorldPosition::parse_name(&flag.pos().room_name()).unwrap();
        flag.remove();
        state.expansion.insert(room);
    });
    Ok(())
}

