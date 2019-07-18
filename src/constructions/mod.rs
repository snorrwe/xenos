mod construction_state;
mod containers;
mod geometry;
mod roads;
mod spawns;

use self::construction_state::ConstructionState;
use crate::prelude::*;
use arrayvec::ArrayVec;
use screeps::{
    constants::{find, StructureType},
    objects::{HasPosition, Room, RoomPosition},
    ReturnCode,
};
use std::collections::{HashSet, VecDeque};
use stdweb::unstable::TryFrom;

pub const CONSTRUCTION_SEGMENT: u32 = 2;

#[derive(Debug, Clone, Hash, Eq, PartialEq, Serialize, Deserialize, Copy, Default)]
pub struct Pos(u16, u16);

impl From<RoomPosition> for Pos {
    fn from(pos: RoomPosition) -> Self {
        Self(pos.x() as u16, pos.y() as u16)
    }
}

impl Pos {
    #[allow(dead_code)]
    pub fn valid_neighbours(&self) -> ArrayVec<[Self; 8]> {
        let x = self.0 as i16;
        let y = self.1 as i16;
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
        .filter(|(x, y)| {
            let x = *x;
            let y = *y;
            1 <= x && x <= 48 && 1 <= y && y <= 48
        })
        .map(|(x, y)| Self(*x as u16, *y as u16))
        .collect()
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
    pub fn with_position(mut self, pos: Pos) -> Self {
        let pos = Self::as_matrix_top_left(pos);
        self.todo.push_back(pos);
        self
    }

    fn soft_reset(&mut self, room: &Room) {
        self.done = HashSet::new();
        let pos = spawns::find_initial_point(room)
            .map(Pos::from)
            .unwrap_or(Pos(25, 25));
        let pos = Self::as_matrix_top_left(pos);
        self.todo.push_back(pos);
    }

    /// Build at most 24 structures and return their results
    /// None is returned if could not find a spot to build the structure
    pub fn build_many(
        &mut self,
        room: &Room,
        types: &ArrayVec<[StructureType; 24]>,
    ) -> ExecutionResult {
        for ty in types.iter() {
            let open_position = { self.open_positions.front().map(|x| x.clone()) };
            let pos = match open_position {
                Some(x) => x,
                None => self
                    .process_next_tile(room)
                    .and_then(|_| self.open_positions.front().map(|x| x.clone()))
                    .ok_or_else(|| {
                        self.soft_reset(room);
                        format!("No free space is available in room {}", room.name())
                    })?,
            };
            debug!(
                "Attempting build at position {:?} in room {}",
                pos,
                room.name()
            );
            let pos = RoomPosition::new(pos.0 as u32, pos.1 as u32, &room.name());
            let result = room.create_construction_site(&pos, *ty);
            match result {
                ReturnCode::InvalidTarget | ReturnCode::Ok => {
                    self.open_positions.pop_front();
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

    /// Return the tile processed if any
    fn process_next_tile(&mut self, room: &Room) -> Option<Pos> {
        debug!("Processing next in room {:?}", room.name());

        let pos = self.todo.pop_front()?;
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
            Pos(x + 0, y + 0), Pos(x + 1, y + 0), Pos(x + 2, y + 0),
            Pos(x + 0, y + 1), Pos(x + 1, y + 1), Pos(x + 2, y + 1),
            Pos(x + 0, y + 2), Pos(x + 1, y + 2), Pos(x + 2, y + 2),
        ];

        let room_name = room.name();

        let parity = (pos.0 + pos.1) % 2;
        // Push either + or × pattern depending on the parity of the tile
        let n_free = tile
            .iter()
            .enumerate()
            .filter(|(i, _)| *i != 4) // Skip the middle
            .filter(|(_, p)| (p.0 + p.1) % 2 != parity)
            .filter(|(_, p)| is_free(room, &RoomPosition::new(p.0 as u32, p.1 as u32, &room_name)))
            .count();

        if n_free >= 3 {
            // At least 3 free positions needed so no blockage is built

            // Push the center
            self.open_positions.push_back(tile[4]);

            self.open_positions.extend(
                tile.into_iter()
                    .enumerate()
                    .filter(|(i, _)| *i as u16 % 2 == parity)
                    .filter(|(_, p)| {
                        is_free(room, &RoomPosition::new(p.0 as u32, p.1 as u32, &room_name))
                    })
                    .map(|(_, p)| *p),
            );
        }

        Some(pos)
    }

    fn as_matrix_top_left(pos: Pos) -> Pos {
        Pos(pos.0 / 3, pos.1 / 3)
    }

    fn valid_neighbouring_tiles(pos: Pos) -> ArrayVec<[Pos; 8]> {
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
        .map(|(x, y)| Pos(*x as u16, *y as u16))
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

    let construction_matrices = &mut state.construction_matrices;
    let matrix = construction_matrices.entry(room.name()).or_insert_with(|| {
        let initial_p = spawns::find_initial_point(room)
            .map(Pos::from)
            .unwrap_or_else(|e| {
                debug!("Cant find an optimal point {:?}", e);
                room.find(find::MY_STRUCTURES)
                    .iter()
                    .next()
                    .map(|s| s.pos())
                    .map(|p| {
                        let x = p.x() as u16;
                        let y = p.y() as u16;
                        Pos(x, y)
                    })
                    .unwrap_or(Pos(25, 25))
            });
        ConstructionMatrix::default().with_position(Pos::from(initial_p))
    });

    matrix.build_many(room, &structures)
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

