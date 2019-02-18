use super::*;
use screeps::{
    constants::StructureType,
    game::get_object_typed,
    objects::{HasPosition, Room, RoomPosition, Source},
    ReturnCode,
};
use stdweb::unstable::TryFrom;

pub fn build_containers<'a>(room: &'a Room) -> ExecutionResult {
    trace!("Building containers in room {}", room.name());

    let sources = js! {
        const room = @{room};
        const sources = room.find(FIND_SOURCES).map((source) => source.id) || [];
        return Object.values(sources);
    };

    let sources = Vec::<String>::try_from(sources)
        .map_err(|e| format!("failed to convert list of sources {:?}", e))?;

    let sources = sources
        .into_iter()
        .filter_map(|id| get_object_typed::<Source>(id.as_str()).ok())
        .filter_map(|source| source)
        .collect::<Vec<_>>();

    sources
        .into_iter()
        .map(|source| source.pos())
        .for_each(|source_pos| {
            let candidates = neighbours(&source_pos)
                .into_iter()
                .cloned()
                .map(|p| {
                    let mut points = Vec::<RoomPosition>::with_capacity(9);
                    let neighbours = neighbours(&p);
                    points.push(p);
                    points.extend_from_slice(&neighbours);
                    points
                })
                .flatten()
                .collect::<Vec<_>>();
            candidates.into_iter().any(|pos| {
                is_free(room, &pos)
                    && room.create_construction_site(pos, StructureType::Container)
                        == ReturnCode::Ok
            });
        });
    Ok(())
}
