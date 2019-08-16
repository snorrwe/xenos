use arrayvec::{ArrayString, ArrayVec};
use screeps::traits::TryInto;
use screeps::Room;
use std::ops::{Deref, DerefMut};

/// Representing positions of rooms
pub struct WorldPosition([i16; 2]);

impl Deref for WorldPosition {
    type Target = [i16; 2];
    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl DerefMut for WorldPosition {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}

pub fn manhatten_distance(one: &str, other: &str) -> Result<i32, &'static str> {
    let one = WorldPosition::parse_name(one)?;
    let other = WorldPosition::parse_name(other)?;

    let x = (one[0] - other[0]).abs() as i32;
    let y = (one[1] - other[1]).abs() as i32;

    Ok(x + y)
}

pub fn neighbours(room: &Room) -> Vec<String> {
    let name = room.name();
    let coords = WorldPosition::parse_name(name.as_str());
    match coords {
        Err(e) => {
            warn!("Failed to parse room name {} {:?}", name, e);
            return vec![];
        }
        _ => {}
    }
    let coords = coords.unwrap();
    let neighbours = coords
        .neighbours_in_vectors()
        .into_iter()
        .map(|coords| coords.to_string())
        .collect::<ArrayVec<[_; 8]>>();
    let names: Vec<&str> = neighbours.iter().map(|n| n.as_str()).collect();
    let result = js! {
        const room = @{room};
        const neighbours = @{names};
        // Directions in the same order as in neighbours_in_vectors
        // TODO: return the directions too?
        const directions = [
            FIND_EXIT_TOP,
            FIND_EXIT_LEFT,
            FIND_EXIT_BOTTOM,
            FIND_EXIT_RIGHT,
        ];
        return neighbours.filter((r,i) => room.findExitTo(r) == directions[i]);
    };
    result
        .try_into()
        .map_err(|e| {
            error!("Failed to convert neighbours {:?}", e);
        })
        .unwrap_or_default()
}

impl WorldPosition {
    /// Returns the coordinates of the room
    pub fn parse_name(room_name: &str) -> Result<Self, &'static str> {
        let parts = room_name
            .split(|c| c == 'W' || c == 'E' || c == 'N' || c == 'S')
            .filter_map(|p| p.parse::<i16>().ok())
            .collect::<Vec<_>>();

        if parts.len() != 2 {
            Err("Failed to parse coordinates")?;
        }

        let mut x = parts[0];
        let mut y = parts[1];

        for c in room_name.chars().filter(|c| {
            let c = *c;
            c == 'W' || c == 'E' || c == 'N' || c == 'S'
        }) {
            match c {
                'W' => x *= -1,
                'S' => y *= -1,
                _ => {}
            }
        }

        Ok(Self([x, y]))
    }

    /// Return the neighbouring positions in order: [N, W, S, E]
    pub fn neighbours_in_vectors(&self) -> [Self; 4] {
        let [x, y] = self.0;
        [
            Self([x, y + 1]),
            Self([x + 1, y]),
            Self([x, y - 1]),
            Self([x - 1, y]),
        ]
    }

    pub fn to_string(&self) -> ArrayString<[u8; 16]> {
        let [x, y] = self.0.clone();
        let w = if x >= 0 { 'E' } else { 'W' };
        let n = if y >= 0 { 'N' } else { 'S' };

        let prefixes = [w, n];
        let mut result = ArrayString::default();

        for (num, pre) in [x, y].into_iter().zip(prefixes.into_iter()) {
            let num = num.abs() as u16;
            let len = len_of_num(num);
            result.push(*pre);
            for i in (0..len).rev() {
                const TEN: u32 = 10;
                let factor = TEN.pow(i as u32);
                let num = (num as u32 / factor) % 10;
                let num = num as u8 + '0' as u8;
                result.push(num as char);
            }
        }

        result
    }
}

fn len_of_num(num: u16) -> i32 {
    let mut i = 1;
    let mut count = 1;
    while i * 10 < num {
        i *= 10;
        count += 1;
    }
    count
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_manhatten() {
        let a = "W43N1";
        let b = "W45S1";

        let d = manhatten_distance(a, b).expect("Failed to get the dinstance");

        assert_eq!(d, 4);
    }

    #[test]
    fn test_string_conversion() {
        let pos = WorldPosition([12, 12]);
        let name = pos.to_string();
        assert_eq!(name.as_str(), "E12N12");

        let pos = WorldPosition([-12, 12]);
        let name = pos.to_string();
        assert_eq!(name.as_str(), "W12N12");

        let pos = WorldPosition([1, -8]);
        let name = pos.to_string();
        assert_eq!(name.as_str(), "E1S8");
    }
}

