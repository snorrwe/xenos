use super::*;
use screeps::{
    constants::StructureType,
    find,
    game::get_object_typed,
    memory,
    objects::{HasId, HasPosition, Room, Source},
    ReturnCode,
};

const MEMORY_KEY: &'static str = "spawn_containers";

pub fn build_containers<'a>(room: &'a Room) -> ExecutionResult {
    trace!("Building containers in room {}", room.name());

    let memory = memory::root();
    let sources = room
        .find(find::SOURCES)
        .into_iter()
        .map(|source| source.id())
        .filter(|id| !memory.path_bool(format!("{}.{}", MEMORY_KEY, id).as_str()))
        .filter_map(|id| get_object_typed::<Source>(id.as_str()).ok().unwrap_or(None))
        .collect::<Vec<_>>();

    sources.into_iter().for_each(|source| {
        let source_pos = source.pos();
        let ok = source_pos.neighbours().into_iter().any(|pos| {
            is_free(room, &pos)
                && room.create_construction_site(pos, StructureType::Container) == ReturnCode::Ok
        });
        if ok {
            memory.path_set(format!("{}.{}", MEMORY_KEY, source.id()).as_str(), true);
        }
    });

    Ok(())
}
