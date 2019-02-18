use super::*;
use screeps::{
    constants::StructureType,
    objects::{Room, RoomPosition},
    ReturnCode,
};
use stdweb::unstable::TryFrom;

pub fn build_containers<'a>(room: &'a Room) -> ExecutionResult {
    let sources = js! {
        const room = @{room};
        return room.find(FIND_SOURCES).map((source) => source.pos);
    };
    let sources = Vec::<RoomPosition>::try_from(sources)
        .map_err(|e| format!("failed to convert list of sources {:?}", e))?;

    let tasks = sources
        .into_iter()
        .map(|source_pos| {
            move |_| {
                let neighbours = neighbours(&source_pos);
                let found = neighbours.into_iter().any(|pos| {
                    if is_free(room, pos) {
                        room.create_construction_site((*pos).clone(), StructureType::Container)
                            == ReturnCode::Ok
                    } else {
                        false
                    }
                });

                if found {
                    Ok(())
                } else {
                    Err(format!("Could not place container anywhere"))
                }
            }
        })
        .map(|task| Task::new(task))
        .collect();

    let tree = Control::All(tasks);
    tree.tick()
}
