use super::*;
use screeps::{
    constants::StructureType,
    game::get_object_typed,
    memory,
    objects::{HasId, HasPosition, Room, Source},
    ReturnCode,
};
use stdweb::unstable::TryFrom;

const MEMORY_KEY: &'static str = "spawn_containers";

pub fn build_containers<'a>(room: &'a Room) -> ExecutionResult {
    trace!("Building containers in room {}", room.name());

    let sources = js! {
        const room = @{room};
        const sources = room.find(FIND_SOURCES).map((source) => source.id) || [];
        return Object.values(sources);
    };

    let sources = Vec::<String>::try_from(sources)
        .map_err(|e| format!("failed to convert list of sources {:?}", e))?;

    let memory = memory::root();
    let sources = sources
        .into_iter()
        .filter(|id| !memory.path_bool(format!("{}.{}", MEMORY_KEY, id).as_str()))
        .filter_map(|id| get_object_typed::<Source>(id.as_str()).ok().unwrap_or(None))
        .collect::<Vec<_>>();

    sources.into_iter().for_each(|source| {
        let source_pos = source.pos();
        let ok = neighbours(&source_pos).into_iter().any(|pos| {
            is_free(room, &pos)
                && room.create_construction_site(pos.clone(), StructureType::Container)
                    == ReturnCode::Ok
        });
        if ok {
            memory.path_set(format!("{}.{}", MEMORY_KEY, source.id()).as_str(), true);
        }
    });

    Ok(())
}
