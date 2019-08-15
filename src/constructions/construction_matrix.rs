use super::point::Point;
use crate::collections::{ArrayQueue, FlagGrid, FlagGrid1717};
use arrayvec::ArrayVec;
use screeps::constants::Terrain;
use screeps::objects::{LookResult, Room, Structure};
use std::collections::BTreeSet;

const DONE_FLAG: u8 = 1;

/// Represents a room split up into 3×3 squares
/// Uses breadth frist search to find empty spaces
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ConstructionMatrix {
    /// 3×3 positions that have not been explored yet
    todo: ArrayQueue<[Point; 128]>,
    /// 3×3 positions that have been explored already
    done: FlagGrid1717,
    /// 1×1 positions that are open for constructions
    open_positions: ArrayQueue<[Point; 8]>,
}

#[derive(Debug, Clone)]
pub enum ConstructionMatrixError {
    OutOfSpace(String),
}

impl ConstructionMatrix {
    pub fn with_position(mut self, pos: Point) -> Self {
        self.todo.push_back(pos);
        self
    }

    pub fn pop_open_pos(&mut self) -> Option<Point> {
        self.open_positions.try_pop_front().ok()
    }

    /// # Invariants
    /// Always use the same room when calling the same matrix
    pub fn find_next_pos(&mut self, room: &Room) -> Result<Point, ConstructionMatrixError> {
        let open_position = { self.open_positions.front().map(|x| x.clone()) };
        if let Ok(open_position) = open_position {
            return Ok(open_position);
        }
        let pos = self
            .process_next_tile(room)
            .ok_or_else(|| ConstructionMatrixError::OutOfSpace(room.name()))?;
        Ok(pos)
    }

    /// Return the tile processed if any
    fn process_next_tile(&mut self, room: &Room) -> Option<Point> {
        debug!("Processing next tile in room {:?}", room.name());

        let pos = self
            .todo
            .try_pop_front()
            .map_err(|e| {
                debug!("todo queue is empty {:?}", e);
            })
            .ok()?;
        debug!("Processing tile pos {:?}", pos);

        let x = pos.0;
        let y = pos.1;

        {
            let x = x as usize / 3;
            let y = y as usize / 3;
            self.done.set(x, y, self.done.get(x, y) | DONE_FLAG);
        }

        let done = &self.done;
        let todo: BTreeSet<_> = self.todo.iter().map(|x| *x).collect();
        self.todo
            .extend(Self::valid_neighbouring_tiles(pos).into_iter().filter(|p| {
                !todo.contains(p) && done.get(p.0 as usize / 3, p.1 as usize / 3) & DONE_FLAG == 0
            }));

        debug!("Extended todo to a len of {}", self.todo.len());

        #[rustfmt::skip]
        let tile = [
            Point(x - 1, y - 1), Point(x + 0, y - 1), Point(x + 1, y - 1),
            Point(x - 1, y + 0), Point(x + 0, y + 0), Point(x + 1, y + 0),
            Point(x - 1, y + 1), Point(x + 0, y + 1), Point(x + 1, y + 1),
        ];

        const PARITY: i16 = 1;

        let minx = tile[0].0.max(0) as u32;
        let miny = tile[0].1.max(0) as u32;
        let maxx = tile[8].0.min(49) as u32;
        let maxy = tile[8].1.min(49) as u32;

        // Count the number of BAD tiles
        let n_taken = room
            .look_at_area(miny, minx, maxy, maxx)
            .into_iter()
            .filter(|r| (r.x + r.y) % 2 != PARITY as u32)
            .filter(|r| match r.look_result {
                LookResult::Structure(Structure::Road(_)) => false,
                LookResult::Terrain(Terrain::Wall)
                | LookResult::ConstructionSite(_)
                | LookResult::Structure(_) => true,
                _ => false,
            })
            .map(|r| (r.x, r.y))
            // Every position should have only 1 of the 'true' categories max
            // But let's make sure there are no duplicates
            .collect::<BTreeSet<_>>()
            .len();

        let n_free = (maxx - minx) * (maxy - miny) - n_taken as u32;

        if n_free > 3 {
            self.open_positions.push_back(pos);
            self.open_positions.extend(
                tile.into_iter()
                    .enumerate()
                    .filter(|(i, _)| i % 2 == PARITY as usize)
                    .filter(|(_, p)| p.is_valid_room_position())
                    .map(|(_, p)| *p),
            );
            Some(pos)
        } else {
            None
        }
    }

    /// Check neighbours in a + pattern
    fn valid_neighbouring_tiles(pos: Point) -> ArrayVec<[Point; 4]> {
        let x = pos.0 as i16;
        let y = pos.1 as i16;
        [
            (x + 2, y - 2),
            (x + 2, y + 2),
            (x - 2, y - 2),
            (x - 2, y + 2),
        ]
        .into_iter()
        .filter(|(x, y)| 4 <= *x && *x < 50 - 4 && 4 <= *y && *y <= 50 - 4)
        .map(|(x, y)| Point(*x as i16, *y as i16))
        .collect()
    }
}

