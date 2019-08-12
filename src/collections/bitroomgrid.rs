const ROWS: usize = 10;
const ROW_SIZE: usize = 32 * 8; // Each row holds 32 * 8 = 256 bits of data

/// 50×50 binary flag map
#[derive(Debug, Clone, Serialize, Deserialize)]
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

}

