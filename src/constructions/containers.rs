use super::geometry::*;
use super::*;
use screeps::{
    constants::StructureType,
    find,
    objects::{HasPosition, Room, StructureProperties},
    ReturnCode,
};

pub fn build_containers<'a>(room: &'a Room) -> ExecutionResult {
    trace!("Building containers in room {}", room.name());

    let spawn = room
        .find(find::MY_STRUCTURES)
        .into_iter()
        .find(|structure| structure.structure_type() == StructureType::Spawn);
    if spawn.is_none() {
        Err(format!(
            "Skipping container build until a spawn is built in room {}",
            room.name()
        ))?;
    }

    let sources = room
        .find(find::SOURCES)
        .into_iter()
        .filter(|source| {
            let has_construction_site = source
                .pos()
                .find_in_range(find::CONSTRUCTION_SITES, 1)
                .into_iter()
                .next()
                .is_some();

            let has_container = source
                .pos()
                .find_in_range(find::STRUCTURES, 1)
                .into_iter()
                .any(|s| s.structure_type() == StructureType::Container);

            !has_construction_site && !has_container
        })
        .collect::<Vec<_>>();

    sources.into_iter().for_each(|source| {
        let source_pos = source.pos();
        source_pos.neighbours().iter().any(|pos| {
            is_free(room, &pos)
                && room.create_construction_site(pos, StructureType::Container) == ReturnCode::Ok
        });
    });

    Ok(())
}
