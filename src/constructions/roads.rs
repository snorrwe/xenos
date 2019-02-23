use super::*;
use screeps::{
    constants::StructureType,
    objects::{Room, RoomPosition},
    ReturnCode,
};
use stdweb::unstable::TryFrom;

pub fn build_roads<'a>(room: &'a Room) -> ExecutionResult {
    trace!("Building roads in room {}", room.name());

    let rcl = room.controller().map(|c| c.level()).unwrap_or(0);
    if rcl < 3 {
        return Err(format!(
            "controller is not advanced enough to warrant road construction in room {}",
            room.name()
        ));
    }

    let targets = js! {
        const room = @{room};
        const targets = [
            ...room.find(FIND_MY_SPAWNS),
            ...room.find(FIND_MY_STRUCTURES, {
                filter: (s) => s.structureType != STRUCTURE_ROAD
                            && s.structureType != STRUCTURE_WALL
            }),
            ...room.find(FIND_SOURCES)
        ];
        const result = targets.map((t)=> t && t.pos).filter((p) => p);
        return Object.values(result);
    };
    let targets = Vec::<RoomPosition>::try_from(targets)
        .map_err(|e| format!("Failed to read list of target positions {:?}", e))?;

    if targets.is_empty() {
        return Err(format!("Nothing to connect in room {}", room.name()));
    }

    let mut targets = targets.into_iter();
    let center = targets.next().expect("oops");
    targets.try_for_each(|pos| connect(&center, &pos, room))
}

fn connect(pos0: &RoomPosition, pos1: &RoomPosition, room: &Room) -> ExecutionResult {
    trace!(
        "Connecting {} {} and {} {} in room {}",
        pos0.x(),
        pos0.y(),
        pos1.x(),
        pos1.y(),
        room.name()
    );

    let path = js! {
        const pos0 = @{pos0};
        const pos1 = @{pos1};
        const room = @{room};
        const path = room.findPath(pos0, pos1, {
            ignoreCreeps: true,
            plainCost: 1,
            swampCost: 2,

        });
        return Object.values(path.map((step) => new RoomPosition( step.x, step.y, room.name )));
    };
    let path = Vec::<RoomPosition>::try_from(path)
        .map_err(|e| format!("Failed to read list of connections {:?}", e))?;

    if path.len() < 2 {
        trace!("points are too close to connect");
        return Ok(());
    }

    path[0..path.len() - 1].into_iter().try_for_each(|pos| {
        let result = room.create_construction_site(pos, StructureType::Road);
        if result == ReturnCode::Full {
            Err("can't place any more construction sites".into())
        } else {
            Ok(())
        }
    })
}
