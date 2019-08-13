use serde::de::{self, Deserialize, Deserializer, Visitor};
use serde::ser::{self, Serialize, Serializer};
use std::fmt::{self, Write};

const ROWS: usize = 10;
const ROW_SIZE: usize = 32 * 8; // Each row holds 32 * 8 = 256 bits of data

/// 50×50 binary flag map
#[derive(Debug, Clone)]
pub struct BitRoomGrid {
    buffer: [[u8; ROW_SIZE / 8]; ROWS],
}

impl Default for BitRoomGrid {
    fn default() -> Self {
        BitRoomGrid {
            buffer: [[0; ROW_SIZE / 8]; ROWS],
        }
    }
}

impl BitRoomGrid {
    pub fn get(&self, x: u16, y: u16) -> bool {
        let flat_ind = x * 50 + y;
        let flat_ind = flat_ind as usize;
        let row = flat_ind / ROW_SIZE;
        let col = flat_ind % ROW_SIZE;

        debug_assert!(row < ROWS);
        debug_assert!(col < ROW_SIZE);

        let row = &self.buffer[row];
        let cell = row[col / 8];

        let col = col % 8;

        cell & (1 << col) != 0
    }

    pub fn set(&mut self, x: u16, y: u16, value: bool) {
        let flat_ind = x * 50 + y;
        let flat_ind = flat_ind as usize;
        let row = flat_ind / ROW_SIZE;
        let col = flat_ind % ROW_SIZE;

        debug_assert!(row < ROWS);
        debug_assert!(col < ROW_SIZE);

        let row = &mut self.buffer[row];
        let cell = &mut row[col / 8];

        let col = col % 8;

        if value {
            *cell = *cell | (1 << col);
        } else {
            *cell = *cell & !(1 << col);
        }
    }

    /// Create a compressed String representation
    /// Structure: `"([0-9]{1,3})_([0-9]{1,3})-?"`
    /// The compression counts conscutive bytes and records their number
    /// For example `[1, 1, 1, 0]` would become `"1_3-0_1"`
    pub fn compressed(&self) -> Result<String, fmt::Error> {
        let mut res = String::with_capacity(320);
        let mut current: u8 = self.buffer[0][0];
        let mut count: usize = 0;
        for row in self.buffer.iter() {
            for byte in row {
                let byte = *byte;
                if byte == current {
                    count += 1;
                } else {
                    write!(res, "{}_{};", current, count)?;
                    current = byte;
                    count = 1;
                }
            }
        }
        if count != 0 {
            write!(res, "{}_{}", current, count)?;
        }
        Ok(res)
    }

    /// Decompress Strings compressed by `compressed` method
    pub fn decompress(value: &str) -> Result<Self, &'static str> {
        let mut res = Self::default();

        let mut row = 0;
        let mut col = 0;

        for block in value.split(';') {
            let mut it = block.split('_').map(|x| x.parse::<usize>().ok());
            let value = it
                .next()
                .ok_or_else(|| "Unexpected end of input")?
                .ok_or_else(|| "Failed to deserialize: invalid input")?;
            let value = value as u8;
            let count = it
                .next()
                .ok_or_else(|| "Unexpected end of input")?
                .ok_or_else(|| "Failed to deserialize: invalid input")?;

            for _ in 0..count {
                res.buffer[row][col] = value;
                col += 1;
                if col >= 32 {
                    col = 0;
                    row += 1;
                }
            }
        }

        Ok(res)
    }
}

impl Serialize for BitRoomGrid {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serializer.serialize_str(
            self.compressed()
                .map_err(|e| ser::Error::custom(e))?
                .as_str(),
        )
    }
}

impl<'de> Deserialize<'de> for BitRoomGrid {
    fn deserialize<D>(deserializer: D) -> Result<BitRoomGrid, D::Error>
    where
        D: Deserializer<'de>,
    {
        struct BitRoomGridVisitor;

        impl<'de> Visitor<'de> for BitRoomGridVisitor {
            type Value = BitRoomGrid;

            fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
                formatter.write_str("Compressed BitRoomGrid string")
            }

            fn visit_str<E>(self, value: &str) -> Result<BitRoomGrid, E>
            where
                E: de::Error,
            {
                BitRoomGrid::decompress(value).map_err(|e| de::Error::custom(e))
            }
        }

        deserializer.deserialize_str(BitRoomGridVisitor)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_bitmap() {
        let mat = BitRoomGrid::default();
        for i in 0..50 {
            for j in 0..50 {
                assert!(!mat.get(i, j), "{} {}", i, j);
            }
        }
    }

    #[test]
    fn test_set_is_consistent() {
        let mut mat = BitRoomGrid::default();
        for i in 0..50 {
            for j in 0..50 {
                assert!(!mat.get(i, j), "{} {}", i, j);
            }
        }

        mat.set(33, 42, true);

        for i in 0..50 {
            for j in 0..50 {
                if i != 33 || j != 42 {
                    assert!(!mat.get(i, j), "{} {}", i, j);
                }
            }
        }

        assert!(mat.get(33, 42));
    }

    #[test]
    // Test if the indexing is fine and does not set a bit twice
    fn test_setting_is_not_overlapping() {
        let mut mat = BitRoomGrid::default();

        for i in 0..50 {
            for j in 0..50 {
                assert!(!mat.get(i, j));
                mat.set(i, j, true);
            }
        }

        for i in 0..50 {
            for j in 0..50 {
                assert!(mat.get(i, j));
                mat.set(i, j, false);
                assert!(!mat.get(i, j));
            }
        }
    }

    #[test]
    fn test_serialize_deserialize() {
        let mut mat = BitRoomGrid::default();

        for i in (0..50).step_by(2) {
            for j in (0..50).step_by(2) {
                assert!(!mat.get(i, j));
                mat.set(i, j, true);
            }
        }

        let compressed = mat.compressed().expect("Failed to serialize");

        let mat = BitRoomGrid::decompress(compressed.as_str()).expect("Failed to parse");

        for i in 0..50 {
            for j in 0..50 {
                let val = i % 2 == 0 && j % 2 == 0;
                assert!(mat.get(i, j) == val);
            }
        }
    }

}

