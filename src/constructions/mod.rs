mod construction_state;
mod containers;
mod neighbours;
mod roads;

use self::construction_state::ConstructionState;
use self::neighbours::*;
use crate::prelude::*;
use arrayvec::ArrayVec;
use screeps::{
    constants::StructureType,
    objects::{HasPosition, Room, RoomPosition, StructureSpawn},
    ReturnCode,
};
use std::collections::{HashSet, VecDeque};
use stdweb::unstable::TryFrom;

pub const CONSTRUCTION_SEGMENT: u32 = 2;

#[derive(Debug, Clone, Hash, Eq, PartialEq, Serialize, Deserialize, Copy)]
pub struct Pos {
    pub x: u16,
    pub y: u16,
}

impl From<RoomPosition> for Pos {
    fn from(pos: RoomPosition) -> Self {
        Self {
            x: pos.x() as u16,
            y: pos.y() as u16,
        }
    }
}

impl Pos {
    pub fn new(x: u16, y: u16) -> Self {
        Self { x: x, y: y }
    }

    pub fn valid_neighbours(&self) -> ArrayVec<[Self; 8]> {
        let x = self.x as i16;
        let y = self.y as i16;
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
        .filter(|(x, y)| Self::is_valid(*x, *y))
        .map(|(x, y)| Self::new(*x as u16, *y as u16))
        .collect()
    }

    /// would x,y make a valid position?
    fn is_valid(x: i16, y: i16) -> bool {
        x < 1 || y < 1 || x > 48 || y > 48
    }
}

/// Represents a room split up into 3×3 squares
/// Uses breadth frist search to find empty spaces
/// # Invariants
/// Always use the same room when calling the same matrix
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ConstructionMatrix {
    /// 3×3 positions that have not been explored yet
    pub todo: VecDeque<Pos>,
    /// 3×3 positions that have been explored already
    pub done: HashSet<Pos>,
    /// 1×1 positions that are open for constructions
    pub open_positions: VecDeque<Pos>,
}

impl ConstructionMatrix {
    // pub fn next(&mut self, room: &Room) -> Option<Pos> {
    //     self.open_positions.pop_front().or_else(|| {
    //         self.process_next_tile(room)
    //             .and_then(|_| self.open_positions.pop_front())
    //     })
    // }
    //
    // /// Calculate at most n open positions
    // pub fn take(&mut self, n: usize, room: &Room) -> Vec<Pos> {
    //     // Gotcha: Drain panics if len is more than VecDeque::len()
    //
    //     let mut len = self.open_positions.len().min(n);
    //     let mut result = self.open_positions.drain(0..len).collect::<Vec<_>>();
    //
    //     len = n - len;
    //
    //     while len > 0 && self.process_next_tile(room).is_some() {
    //         let l = self.open_positions.len().min(len);
    //         result.extend(self.open_positions.drain(0..l));
    //         len = n - result.len();
    //     }
    //
    //     result
    // }

    pub fn with_position(mut self, pos: Pos) -> Self {
        let pos = Self::as_matrix_top_left(pos);
        self.todo.push_back(pos);
        self
    }

    /// Build at most 24 structures and return their results
    /// None is returned if could not find a spot to build the structure
    pub fn build_many(
        &mut self,
        room: &Room,
        types: &ArrayVec<[StructureType; 24]>,
    ) -> ArrayVec<[Option<ReturnCode>; 24]> {
        types
            .iter()
            .map(|ty| {
                self.open_positions
                    .front()
                    .or_else(|| {
                        self.process_next_tile(room)
                            .and_then(|_| self.open_positions.front())
                    })
                    .map(|pos| {
                        let pos = RoomPosition::new(pos.x as u32, pos.y as u32, &room.name());
                        let result = room.create_construction_site(&pos, *ty);
                        match result {
                            ReturnCode::Ok => {
                                self.open_positions.pop_front();
                            }
                            ReturnCode::Full => {
                                debug!("cant place construction site {:?}", result);
                            }
                            _ => {}
                        }
                        result
                    })
            })
            .collect()
    }

    /// Return the tile processed if any
    fn process_next_tile(&mut self, room: &Room) -> Option<Pos> {
        let pos = self.todo.pop_front()?;

        let done = &self.done;
        self.todo.extend(
            pos.valid_neighbours()
                .into_iter()
                .filter(|p| !done.contains(p)),
        );

        let x = pos.x * 3;
        let y = pos.y * 3;

        #[rustfmt::skip]
        let tile = [
            Pos::new(x + 0, y + 0), Pos::new(x + 1, y + 0), Pos::new(x + 2, y + 0),
            Pos::new(x + 0, y + 1), Pos::new(x + 1, y + 1), Pos::new(x + 2, y + 1),
            Pos::new(x + 0, y + 2), Pos::new(x + 1, y + 2), Pos::new(x + 2, y + 2),
        ]
            .into_iter()
            .cloned()
            .collect::<ArrayVec<[_; 9]>>();

        let room_name = room.name();

        // Push either + or × pattern depending on the parity of the tile
        let parity = (x + y) % 2;
        let n_free = tile
            .iter()
            .filter(|p| (p.x + p.y) % 2 != parity)
            .filter(|p| is_free(room, &RoomPosition::new(p.x as u32, p.y as u32, &room_name)))
            .count();

        if n_free > 3 {
            // At least 3 free positions needed so no blockage is built

            // Push the center
            self.open_positions.push_back(tile[4]);

            self.open_positions.extend(
                tile.into_iter()
                    .filter(|p| (p.x + p.y) % 2 == parity)
                    .filter(|p| {
                        is_free(room, &RoomPosition::new(p.x as u32, p.y as u32, &room_name))
                    }),
            );
        }

        Some(pos)
    }

    fn as_matrix_top_left(pos: Pos) -> Pos {
        Pos {
            x: pos.x / 3,
            y: pos.y / 3,
        }
    }
}

/// Return the BehaviourTree that runs the spawns
pub fn task<'a>() -> Task<'a, GameState> {
    trace!("Init construction task");

    Task::new(move |_| {
        let time = screeps::game::time();
        let rooms = screeps::game::rooms::values();
        let len = rooms.len() as u32;

        if time % (len * 10) > len {
            Err("Skipping constructions task")?;
        }

        let mut state = ConstructionState::read_from_segment_or_default(CONSTRUCTION_SEGMENT);
        state.save_to_memory = Some(true);
        state.memory_segment = Some(CONSTRUCTION_SEGMENT as u8);

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

    Ok(())
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

    let matrix = match state.construction_matrices.get_mut(&room.name()) {
        Some(matrix) => matrix,
        None => {
            let spawn = js! {
                const room = @{room};
                const spawns = room.find(FIND_STRUCTURES, {
                    filter: { structureType: STRUCTURE_SPAWN }
                });
                return spawns && spawns[0] && spawns[0].pos || null;
            };

            if spawn.is_null() {
                let e = Err(format!("No spawn in room {}", &room.name()));
                debug!("{:?}", e);
                e?;
            }

            let spawn = RoomPosition::try_from(spawn).map_err(|e| {
                let err = format!("Failed to convert spawn position {:?}", e);
                error!("{}", &err);
                err
            })?;

            let result = ConstructionMatrix::default().with_position(Pos::from(spawn));

            state.construction_matrices.insert(room.name(), result);

            &mut state.construction_matrices[&room.name()]
        }
    };

    matrix
        .build_many(room, &structures)
        .into_iter()
        .fold(Ok(()), |result, build_res| {})
}

fn valid_construction_pos(room: &Room, pos: &RoomPosition, taken: &mut HashSet<Pos>) -> bool {
    let pp = Pos::new(pos.x() as u16, pos.y() as u16);
    if taken.contains(&pp) {
        return false;
    }

    let x = pos.x();
    let y = pos.y();

    if x <= 2 || y <= 2 || x >= 48 || y >= 48 {
        // Out of bounds
        return false;
    }

    let name = pos.room_name();
    [
        RoomPosition::new(x - 1, y, name.as_str()),
        RoomPosition::new(x + 1, y, name.as_str()),
        RoomPosition::new(x, y - 1, name.as_str()),
        RoomPosition::new(x, y + 1, name.as_str()),
    ]
    .into_iter()
    .all(|p| {
        let pos = Pos::new(p.x() as u16, p.y() as u16);
        if taken.contains(&pos) {
            false
        } else {
            let result = is_free(room, p);
            if !result {
                taken.insert(pos);
            }
            result
        }
    })
}

fn is_free(room: &Room, pos: &RoomPosition) -> bool {
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

