mod construction_state;
mod containers;
pub mod geometry;
pub mod point;
mod roads;
mod spawns;
mod storage;

use self::point::Point;

use self::construction_state::ConstructionState;
use crate::collections::ArrayQueue;
use crate::prelude::*;
use crate::CONSTRUCTIONS_SEGMENT;
use arrayvec::ArrayVec;
use screeps::{
    constants::{find, StructureType},
    objects::{HasPosition, Room, RoomPosition},
    ReturnCode,
};
use std::collections::HashSet;
use stdweb::unstable::TryFrom;

/// Represents a room split up into 3×3 squares
/// Uses breadth frist search to find empty spaces
/// # Invariants
/// Always use the same room when calling the same matrix
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ConstructionMatrix {
    /// 3×3 positions that have not been explored yet
    todo: ArrayQueue<[Point; 32]>, // TODO: The capacities are pretty strict numbers, let's see how they work out
    /// 3×3 positions that have been explored already
    done: HashSet<Point>,
    /// 1×1 positions that are open for constructions
    open_positions: ArrayQueue<[Point; 64]>,
}

impl ConstructionMatrix {
    pub fn with_position(mut self, pos: Point) -> Self {
        let pos = Self::as_tile_top_left(pos);
        self.todo.push_back(pos);
        self
    }

    fn reset(&mut self, room: &Room) {
        self.done = HashSet::new();
        self.open_positions = Default::default();
        let pos = spawns::find_initial_point(room)
            .map(Point::from)
            .unwrap_or(Point(25, 25));
        self.todo = Default::default();
        let pos = Self::as_tile_top_left(pos);
        self.todo.push_back(pos);
    }

    /// Build at most 24 structures and return their results
    /// None is returned if could not find a spot to build the structure
    pub fn build_many(
        &mut self,
        room: &Room,
        types: &ArrayVec<[StructureType; 24]>,
    ) -> ExecutionResult {
        let name = &room.name();

        for ty in types.iter() {
            let pos = self.find_next_pos(room)?;
            debug!(
                "Attempting build at position {:?} in room {}",
                pos,
                room.name()
            );
            let pos = pos.try_into_room_pos(&name).ok_or_else(|| {
                let err = format!("Failed to cast point {:?} to RoomPosition", pos);
                error!("{}", err);
                err
            })?;
            let result = room.create_construction_site(&pos, *ty);
            match result {
                ReturnCode::InvalidTarget | ReturnCode::Ok => {
                    self.open_positions
                        .try_pop_front()
                        .map_err(|e| format!("Tried to pop front on an empty queue {:?}", e))?;
                }
                ReturnCode::Full => {
                    debug!("cant place construction site {:?}", result);
                    Err("Room is full")?;
                }
                _ => {
                    debug!("Can't place construction site {:?}", result);
                }
            }
        }

        Ok(())
    }

    pub fn find_next_pos(&mut self, room: &Room) -> Result<Point, String> {
        let open_position = { self.open_positions.front().map(|x| x.clone()) };
        if let Ok(open_position) = open_position {
            if is_free(room, &open_position.into_room_pos(&room.name())) {
                self.open_positions
                    .try_pop_front()
                    .map_err(|e| format!("Failed to pop front item {:?}", e))?;
                return Ok(open_position);
            }
        }
        let pos = self
            .process_next_tile(room)
            .and_then(|_| self.open_positions.front().map(|x| x.clone()).ok())
            .ok_or_else(|| {
                self.reset(room);
                format!("No free space is available in room {}", room.name())
            })?;
        Ok(pos)
    }

    /// Return the tile processed if any
    fn process_next_tile(&mut self, room: &Room) -> Option<Point> {
        debug!("Processing next in room {:?}", room.name());

        let pos = self.todo.try_pop_front().ok()?;
        debug!("Processing tile pos {:?}", pos);

        {
            self.done.insert(pos);
        }

        let done = &self.done;
        self.todo.extend(
            Self::valid_neighbouring_tiles(pos)
                .into_iter()
                .filter(|p| !done.contains(p)),
        );

        debug!("Extended todo to a len of {}", self.todo.len());

        let x = pos.0 * 3;
        let y = pos.1 * 3;

        #[rustfmt::skip]
        let tile = [
            Point(x + 0, y + 0), Point(x + 1, y + 0), Point(x + 2, y + 0),
            Point(x + 0, y + 1), Point(x + 1, y + 1), Point(x + 2, y + 1),
            Point(x + 0, y + 2), Point(x + 1, y + 2), Point(x + 2, y + 2),
        ];

        let room_name = room.name();

        let parity = (pos.0 + pos.1) % 2;
        // Push either + or × pattern depending on the parity of the tile
        let n_free = tile
            .iter()
            .enumerate()
            .filter(|(i, _)| *i != 4) // Skip the middle
            .filter(|(_, p)| p.is_valid_room_position())
            .filter(|(_, p)| (p.0 + p.1) % 2 != parity)
            .filter(|(_, p)| is_free(room, &p.into_room_pos(&room_name)))
            .count();

        if n_free >= 3 {
            // At least 3 free positions needed so no blockage is built
            // Note that this method only works for 3×3 tiles

            // Push the center
            self.open_positions.push_back(tile[4]);

            self.open_positions.extend(
                tile.into_iter()
                    .enumerate()
                    .filter(|(_, p)| p.is_valid_room_position())
                    .filter(|(i, _)| *i as i16 % 2 == parity)
                    .filter(|(_, p)| is_free(room, &p.into_room_pos(&room_name)))
                    .map(|(_, p)| *p),
            );
        }

        Some(pos)
    }

    fn as_tile_top_left(pos: Point) -> Point {
        Point(pos.0 / 3, pos.1 / 3)
    }

    fn valid_neighbouring_tiles(pos: Point) -> ArrayVec<[Point; 8]> {
        let x = pos.0 as i16;
        let y = pos.1 as i16;
        [
            (x + 1, y + 0),
            (x - 1, y + 0),
            (x + 0, y + 1),
            (x + 0, y - 1),
            (x + 1, y + 1),
            (x - 1, y + 1),
            (x + 1, y - 1),
            (x - 1, y - 1),
        ]
        .into_iter()
        .filter(|(x, y)| 0 <= *x && *x <= 16 && 0 <= *y && *y <= 16)
        .map(|(x, y)| Point(*x as i16, *y as i16))
        .collect()
    }
}

pub fn task<'a>() -> Task<'a, GameState> {
    trace!("Init construction task");

    Task::new(move |_| {
        let time = screeps::game::time();
        let rooms = screeps::game::rooms::values();
        let len = rooms.len() as u32;

        if time % (len * 3) > len {
            Err("Skipping constructions task")?;
        }

        let mut state = ConstructionState::read_from_segment_or_default(CONSTRUCTIONS_SEGMENT);
        state.save_to_memory = Some(true);
        state.memory_segment = Some(CONSTRUCTIONS_SEGMENT as u8);

        let index = time % len;
        let room = &rooms[index as usize];

        manage_room(room, &mut state)
    })
    .with_required_bucket(5000)
}

fn manage_room<'a>(room: &'a Room, state: &mut ConstructionState) -> ExecutionResult {
    info!("Manage constructionSites of room {:?}", room.name());

    build_structures(room, state).unwrap_or_else(|e| warn!("Failed build_structures {:?}", e));
    containers::build_containers(room).unwrap_or_else(|e| warn!("Failed containers {:?}", e));
    roads::build_roads(room).unwrap_or_else(|e| warn!("Failed roads {:?}", e));
    // FIXME: finish the implementation
    // build_storage(room, state).unwrap_or_else(|e| warn!("Failed storage {:?}", e));

    Ok(())
}

fn build_storage(room: &Room, state: &mut ConstructionState) -> ExecutionResult {
    let mat = get_matrix_mut(state, room);
    let pos = storage::find_storage_pos(room, mat)?;

    warn!("Building storage at {:?}", pos);

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
        StructureType::Extension,
        StructureType::Extension,
        StructureType::Extension,
        StructureType::Spawn,
    ]
    .into_iter()
    .map(|x| *x)
    .collect::<ArrayVec<_>>();

    let matrix = get_matrix_mut(state, room);
    matrix.build_many(room, &structures)
}

fn get_matrix_mut<'a>(state: &'a mut ConstructionState, room: &Room) -> &'a mut ConstructionMatrix {
    let construction_matrices = &mut state.construction_matrices;
    let matrix = construction_matrices.entry(room.name()).or_insert_with(|| {
        let initial_p = spawns::find_initial_point(room)
            .map(Point::from)
            .unwrap_or_else(|e| {
                debug!("Cant find an optimal point {:?}", e);
                room.find(find::MY_STRUCTURES)
                    .iter()
                    .next()
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
            let invalidNeighbour = objects.find((o) => {
                return (o.type == LOOK_TERRAIN && o.terrain == "wall")
                    || (o.type == LOOK_STRUCTURES && o.structure.structureType != STRUCTURE_ROAD)
                    || o.type == LOOK_CONSTRUCTION_SITES;
            });
            return invalidNeighbour == null;
        } catch (e) {
            return false;
        }
    };
    bool::try_from(result).unwrap_or(false)
}
