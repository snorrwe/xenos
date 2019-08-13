use super::*;
use serde::de::{self, Deserialize, Deserializer, Visitor};
use serde::ser::{self, Serialize, Serializer};
use std::fmt::{self};

macro_rules! implement_serde_for_bitgrid {
    ($name: ident) => {
        impl Serialize for $name {
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
        impl<'de> Deserialize<'de> for $name {
            fn deserialize<D>(deserializer: D) -> Result<$name, D::Error>
            where
                D: Deserializer<'de>,
            {
                struct BitGridVisitor;

                impl<'de> Visitor<'de> for BitGridVisitor {
                    type Value = $name;

                    fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
                        formatter.write_str("Compressed BitRoomGrid string")
                    }

                    fn visit_str<E>(self, value: &str) -> Result<$name, E>
                    where
                        E: de::Error,
                    {
                        BitGrid::decompress(value).map_err(|e| de::Error::custom(e))
                    }
                }

                deserializer.deserialize_str(BitGridVisitor)
            }
        }
    };
}

/// 50×50 binary flag map
/// With a size of 320 bytes
#[derive(Debug, Clone)]
pub struct BitRoomGrid5050 {
    buffer: [[u8; 32]; 10],
}

impl Default for BitRoomGrid5050 {
    fn default() -> Self {
        BitRoomGrid5050 {
            buffer: [Default::default(); 10],
        }
    }
}

impl BitGrid for BitRoomGrid5050 {
    const ROWS: usize = 10;
    const ROW_SIZE: usize = 32;
    const ROOM_ROWS: usize = 50;
    type Row = [u8; 32];

    fn row(&self, ind: usize) -> &[u8] {
        &self.buffer[ind]
    }

    fn row_mut(&mut self, ind: usize) -> &mut [u8] {
        &mut self.buffer[ind]
    }
}

implement_serde_for_bitgrid!(BitRoomGrid5050);

/// 17×17 binary flag map
#[derive(Debug, Clone)]
pub struct BitRoomGrid1717 {
    buffer: [[u8; 32]; 2],
}

impl Default for BitRoomGrid1717 {
    fn default() -> Self {
        BitRoomGrid1717 {
            buffer: [Default::default(); 2],
        }
    }
}

impl BitGrid for BitRoomGrid1717 {
    const ROWS: usize = 2;
    const ROW_SIZE: usize = 32;
    const ROOM_ROWS: usize = 17;
    type Row = [u8; 32];

    fn row(&self, ind: usize) -> &[u8] {
        &self.buffer[ind]
    }

    fn row_mut(&mut self, ind: usize) -> &mut [u8] {
        &mut self.buffer[ind]
    }
}

implement_serde_for_bitgrid!(BitRoomGrid1717);


#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_bitmap() {
        js!{};

        let mat = BitRoomGrid5050::default();
        for i in 0..50 {
            for j in 0..50 {
                assert!(!mat.get(i, j), "{} {}", i, j);
            }
        }
    }

    #[test]
    fn test_set_is_consistent() {
        js!{};

        let mut mat = BitRoomGrid5050::default();
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
        js!{};

        let mut mat = BitRoomGrid5050::default();

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
        js!{};

        let mut mat = BitRoomGrid5050::default();

        for i in (0..50).step_by(2) {
            for j in (0..50).step_by(2) {
                assert!(!mat.get(i, j));
                mat.set(i, j, true);
            }
        }

        let compressed = mat.compressed().expect("Failed to serialize");

        let mat = BitRoomGrid5050::decompress(compressed.as_str()).expect("Failed to parse");

        for i in 0..50 {
            for j in 0..50 {
                let val = i % 2 == 0 && j % 2 == 0;
                assert!(mat.get(i, j) == val);
            }
        }
    }

    #[test]
    fn test_size_of_grid() {
        js!{};

        use std::mem::size_of;

        // Application code can't use doc tests
        assert_eq!(
            size_of::<BitRoomGrid5050>(),
            320,
            "You forgot to update the doc comment of the struct after changing its size"
        );
    }
}
