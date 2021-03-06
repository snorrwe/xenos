use super::*;
use crate::collections::FlagGrid;
use arrayvec::ArrayVec;
use screeps::{
    constants::{find, StructureType},
    objects::{Room, RoomPosition, StructureProperties},
    ReturnCode,
};
use stdweb::unstable::TryFrom;

const CONNECTED_FLAG: u8 = 1;

pub fn build_roads<'a>(room: &'a Room, state: &'a mut ConstructionState) -> ExecutionResult {
    trace!("Building roads in room {}", room.name());

    can_continue_building(room)?;

    let matrix = state.connections.entry(room.name()).or_default();

    let targets = js! {
        const room = @{room};
        const targets = [
            ...room.find(FIND_MY_SPAWNS),
            ...room.find(FIND_MY_STRUCTURES, {
                filter: (s) => s
                            && s.structureType != STRUCTURE_ROAD
                            && s.structureType != STRUCTURE_WALL
                            && s.structureType != STRUCTURE_RAMPART
            }),
            ...room.find(FIND_SOURCES)
        ];
        const result = targets.map((t)=> t && t.pos).filter((p) => p);
        return Object.values(result);
    };
    let targets = Vec::<RoomPosition>::try_from(targets)
        .map_err(|e| format!("Failed to read list of target positions {:?}", e))?;

    if targets.is_empty() {
        Err(format!("Nothing to connect in room {}", room.name()))?;
    }

    let mut targets = targets.into_iter();
    let center = targets
        .next()
        .ok_or_else(|| "No target found for road building")?;

    let c = center.pos();
    matrix.set_or(c.x() as usize, c.y() as usize, CONNECTED_FLAG);

    // Connect at most 16 positions in a tick
    let targets: ArrayVec<[_; 16]> = targets
        .filter(|pos| matrix.get(pos.x() as usize, pos.y() as usize) & CONNECTED_FLAG == 0)
        .collect();
    for pos in targets.iter() {
        connect(&center, &pos, room).and_then(|_| {
            matrix.set_or(pos.x() as usize, pos.y() as usize, CONNECTED_FLAG);
            Ok(())
        })?;
    }
    Ok(())
}

fn can_continue_building(room: &Room) -> ExecutionResult {
    let rcl = room.controller().map(|c| c.level()).unwrap_or(0);
    if rcl < 3 {
        Err(format!(
            "controller is not advanced enough to warrant road construction in room {}",
            room.name()
        ))?;
    }

    let has_construction = room
        .find(find::MY_CONSTRUCTION_SITES)
        .into_iter()
        .next()
        .is_some();
    if has_construction {
        Err(format!("Room {} has incomplete constructions", room.name()))?;
    }

    let has_tower = room
        .find(find::STRUCTURES)
        .into_iter()
        .any(|s| s.structure_type() == StructureType::Tower);
    if !has_tower {
        Err(format!("Room {} does not have a Tower yet", room.name()))?;
    }

    Ok(())
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
        const room = @{room};
        const path = room.findPath(@{pos0}, @{pos1}, {
            ignoreCreeps: true,
            plainCost: 1,
            swampCost: 2,
            range: 0,
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

