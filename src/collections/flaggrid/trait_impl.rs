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
                        FlagGrid::decompress(value).map_err(|e| de::Error::custom(e))
                    }
                }

                deserializer.deserialize_str(BitGridVisitor)
            }
        }
    };
}

/// 50×50 binary flag map
/// With a size of 320 bytes
#[derive(Clone)]
pub struct BitRoomGrid5050 {
    buffer: Vec<u8>,
}

impl Default for BitRoomGrid5050 {
    fn default() -> Self {
        BitRoomGrid5050 {
            buffer: vec![0; 2500],
        }
    }
}

impl FlagGrid for BitRoomGrid5050 {
    const ROWS: usize = 50;
    const COLS: usize = 50;

    fn buffer(&self) -> &[u8] {
        &self.buffer[..]
    }

    fn buffer_mut(&mut self) -> &mut [u8] {
        &mut self.buffer[..]
    }
}

implement_serde_for_bitgrid!(BitRoomGrid5050);

/// 17×17 binary flag map
#[derive(Debug, Clone)]
pub struct FlagGrid1717 {
    buffer: Vec<u8>,
}

impl Default for FlagGrid1717 {
    fn default() -> Self {
        FlagGrid1717 {
            buffer: vec![0; 17 * 17],
        }
    }
}

impl FlagGrid for FlagGrid1717 {
    const ROWS: usize = 17;
    const COLS: usize = 17;

    fn buffer(&self) -> &[u8] {
        &self.buffer[..]
    }

    fn buffer_mut(&mut self) -> &mut [u8] {
        &mut self.buffer[..]
    }
}

implement_serde_for_bitgrid!(FlagGrid1717);

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_bitmap() {
        js! {};

        let mat = BitRoomGrid5050::default();
        for i in 0..50 {
            for j in 0..50 {
                assert!(mat.get(i, j) == 0, "{} {}", i, j);
            }
        }
    }

    #[test]
    fn test_set_is_consistent() {
        js! {};

        let mut mat = BitRoomGrid5050::default();
        for i in 0..50 {
            for j in 0..50 {
                assert!(mat.get(i, j) == 0, "{} {}", i, j);
            }
        }

        mat.set(33, 42, 1);

        for i in 0..50 {
            for j in 0..50 {
                if i != 33 || j != 42 {
                    assert!(mat.get(i, j) == 0, "{} {}", i, j);
                }
            }
        }

        assert!(mat.get(33, 42) == 1);
    }

    #[test]
    // Test if the indexing is fine and does not set a bit twice
    fn test_setting_is_not_overlapping() {
        js! {};

        let mut mat = BitRoomGrid5050::default();

        for i in 0..50 {
            for j in 0..50 {
                assert!(mat.get(i, j) == 0);
                mat.set(i, j, 1);
            }
        }

        for i in 0..50 {
            for j in 0..50 {
                assert!(mat.get(i, j) == 1);
                mat.set(i, j, 0);
                assert!(mat.get(i, j) == 0);
            }
        }
    }

    #[test]
    fn test_serialize_deserialize() {
        js! {};

        let mut mat = BitRoomGrid5050::default();

        for i in (0..50).step_by(2) {
            for j in (0..50).step_by(2) {
                assert!(mat.get(i, j) == 0);
                mat.set(i, j, 1);
            }
        }

        let compressed = mat.compressed().expect("Failed to serialize");

        let mat = BitRoomGrid5050::decompress(compressed.as_str()).expect("Failed to parse");

        for i in 0..50 {
            for j in 0..50 {
                let val = if i % 2 == 0 && j % 2 == 0 { 1 } else { 0 };
                assert!(mat.get(i, j) == val);
            }
        }
    }
}

