use arrayvec::ArrayString;
use screeps::{Room, RoomPosition};
use serde::de::{self, Deserialize, Deserializer, Visitor};
use serde::ser::{Serialize, Serializer};
use std::ops::{Deref, DerefMut};

/// Representing positions of rooms
#[derive(Debug, Clone, Copy, Eq, PartialEq, Ord, PartialOrd, Default, Hash)]
pub struct WorldPosition([i16; 2]);

impl From<Room> for WorldPosition {
    fn from(room: Room) -> Self {
        Self::parse_name(&room.name()).unwrap()
    }
}

impl<'a> From<&'a Room> for WorldPosition {
    fn from(room: &'a Room) -> Self {
        Self::parse_name(&room.name()).unwrap()
    }
}

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

impl WorldPosition {
    pub fn dist(&self, other: WorldPosition) -> u16 {
        let x = (self.0[0] - other.0[0]).abs();
        let y = (self.0[1] - other.0[1]).abs();
        (x + y) as u16
    }

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

        for (num, pre) in [x, y].iter().zip(prefixes.iter()) {
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

    pub fn as_room_center(&self) -> RoomPosition {
        RoomPosition::new(24, 24, self.to_string().as_str())
    }
}

impl Serialize for WorldPosition {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let st = self.to_string();
        serializer.serialize_str(&st)
    }
}

impl<'de> Deserialize<'de> for WorldPosition {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        struct WPVisitor;

        impl<'de> Visitor<'de> for WPVisitor {
            type Value = WorldPosition;

            fn expecting(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
                formatter.write_str("a serialized world position")
            }

            fn visit_str<E: de::Error>(self, string: &str) -> Result<Self::Value, E> {
                WorldPosition::parse_name(string).map_err(|e| de::Error::custom(e))
            }
        }

        let visitor = WPVisitor;
        deserializer.deserialize_str(visitor)
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
