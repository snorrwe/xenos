pub mod construction_matrix;
mod containers;
pub mod geometry;
pub mod point;
mod roads;
mod spawns;
mod storage;

use self::construction_matrix::ConstructionMatrix;
use crate::state::ConstructionState;
use self::point::Point;
use crate::state::MemorySentinel;
use crate::prelude::*;
use crate::CONSTRUCTIONS_SEGMENT;
use screeps::{
    constants::{find, StructureType},
    objects::{HasPosition, Room, RoomPosition},
    ReturnCode,
};
use stdweb::unstable::{TryFrom, TryInto};

pub fn task<'a>() -> Task<'a, GameState> {
    debug!("Init construction task");

    Task::new(move |_| {
        let time = screeps::game::time();
        let rooms = screeps::game::rooms::values();
        let len = rooms.len() as u32;

        if time % (len * 3) > len {
            Err("Skipping constructions task")?;
        }

        let mut state = ConstructionState::read_from_segment_or_default(CONSTRUCTIONS_SEGMENT);
        let _sentinel = MemorySentinel::new(CONSTRUCTIONS_SEGMENT as u8, &state);

        let index = time % len;
        let room = &rooms[index as usize];

        manage_room(room, &mut state)
    })
    .with_required_bucket(5000)
}

fn manage_room<'a>(room: &'a Room, state: &mut ConstructionState) -> ExecutionResult {
    info!("Manage constructionSites of room {:?}", room.name());

    let my = js! {
        const room = @{room};
        return room.controller && room.controller.my || false;
    };
    let my: bool = my.try_into().map_err(|e| {
        error!("Failed to convert bool, {:?}", e);
        "Conversion error"
    })?;
    if !my {
        Err("Room is not mine")?;
    }

    build_structures(room, state).unwrap_or_else(|e| warn!("Failed build_structures {:?}", e));
    containers::build_containers(room).unwrap_or_else(|e| warn!("Failed containers {:?}", e));
    roads::build_roads(room, state).unwrap_or_else(|e| warn!("Failed roads {:?}", e));
    build_storage(room, state).unwrap_or_else(|e| warn!("Failed storage {:?}", e));

    Ok(())
}

fn build_storage(room: &Room, _state: &mut ConstructionState) -> ExecutionResult {
    let pos = storage::find_storage_pos(room)?;

    debug!("Building storage at {:?}", pos);

    let pos = pos.into_room_pos(&room.name());
    let result = room.create_construction_site(&pos, StructureType::Storage);
    match result {
        ReturnCode::Ok => Ok(()),
        ReturnCode::Full => {
            debug!("Can't place construction site {:?}", result);
            Err("Room is full")?
        }
        _ => {
            debug!("Can't place construction site {:?}", result);
            Err(format!("Failed to place construction site {:?}", result))?
        }
    }
}

fn build_structures<'a>(room: &'a Room, state: &'a mut ConstructionState) -> ExecutionResult {
    let structures = [
        StructureType::Tower,
        StructureType::Extension,
        StructureType::Extension,
        StructureType::Spawn,
    ];

    let matrix = get_matrix_mut(state, room);
    let mut pos = matrix
        .find_next_pos(room)
        .map_err(|e| format!("Failed to get the next position {:?}", e))?;

    let name = room.name();

    for structure in structures.iter() {
        debug!("Attempting build at position {:?} in room {}", pos, &name);
        let roompos = pos.try_into_room_pos(&name).ok_or_else(|| {
            let err = format!("Failed to cast point {:?} to RoomPosition", pos);
            error!("{}", err);
            err
        })?;
        let result = room.create_construction_site(&roompos, *structure);
        match result {
            ReturnCode::InvalidTarget | ReturnCode::Ok => {
                matrix
                    .pop_open_pos()
                    .ok_or_else(|| format!("Tried to pop front on an empty queue"))?;
                pos = matrix
                    .find_next_pos(room)
                    .map_err(|e| format!("Failed to get the next position {:?}", e))?;
            }
            ReturnCode::Full => {
                debug!("Can' t place construction site {:?}", result);
                Err("Room is full")?;
            }
            _ => {
                debug!("Can't place construction site {:?}", result);
            }
        }
    }
    Ok(())
}

fn get_matrix_mut<'a>(state: &'a mut ConstructionState, room: &Room) -> &'a mut ConstructionMatrix {
    let construction_matrices = &mut state.construction_matrices;
    let matrix = construction_matrices.entry(room.name()).or_insert_with(|| {
        let initial_p = spawns::find_initial_point(room)
            .map(Point::from)
            .unwrap_or_else(|e| {
                debug!("Cant find an optimal point {:?}", e);
                let structs = room.find(find::MY_STRUCTURES);
                structs
                    .last()
                    .map(|s| s.pos())
                    .map(|p| {
                        let x = p.x() as i16;
                        let y = p.y() as i16;
                        Point(x, y)
                    })
                    .unwrap_or(Point(25, 25))
            });
        ConstructionMatrix::default().with_position(Point::from(initial_p))
    });
    matrix
}

pub fn is_free(room: &Room, pos: &RoomPosition) -> bool {
    let result = js! {
        const pos = @{pos};
        const room = @{room};
        try {
            let objects = room.lookAt(pos);
            let invalid = objects.find((o) => {
                return (o.type == LOOK_TERRAIN && o.terrain == "wall")
                    || (o.type == LOOK_STRUCTURES && o.structure.structureType != STRUCTURE_ROAD)
                    || o.type == LOOK_CONSTRUCTION_SITES;
            });
            return !invalid;
        } catch (e) {
            return false;
        }
    };
    bool::try_from(result).unwrap_or(false)
}

