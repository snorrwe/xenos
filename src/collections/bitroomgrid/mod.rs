pub mod trait_impl;

pub use self::trait_impl::*;

use std::fmt::{self, Write};

/// Specify size in bytes
pub trait BitGrid: Default + Sized {
    const ROWS: usize;
    const ROW_SIZE: usize;
    const ROOM_ROWS: usize;

    type Row: Sized + AsRef<[u8]> + AsMut<[u8]>;

    fn row(&self, ind: usize) -> &[u8];
    fn row_mut(&mut self, ind: usize) -> &mut [u8];

    fn get(&self, x: usize, y: usize) -> bool {
        let row_size = Self::ROW_SIZE * 8;

        let flat_ind = x * Self::ROOM_ROWS + y;
        let flat_ind = flat_ind as usize;
        let row = flat_ind / row_size;
        let col = flat_ind % row_size;

        debug_assert!(row < Self::ROWS);
        debug_assert!(col < row_size);

        let row = &self.row(row);
        let cell = row[col / 8];

        let col = col % 8;

        cell & (1 << col) != 0
    }

    fn set(&mut self, x: usize, y: usize, value: bool) {
        let row_size = Self::ROW_SIZE * 8;
        let flat_ind = x * Self::ROOM_ROWS + y;
        let flat_ind = flat_ind as usize;
        let row = flat_ind / row_size;
        let col = flat_ind % row_size;

        debug_assert!(row < Self::ROWS);
        debug_assert!(col < row_size);

        let row = &mut self.row_mut(row);
        let cell = &mut row[col / 8];

        let col = col % 8;

        if value {
            *cell = *cell | (1 << col);
        } else {
            *cell = *cell & !(1 << col);
        }
    }

    /// Create a compressed String representation
    /// Structure: `"([0-9]{1,3})_([0-9]{1,3});?"`
    /// The compression counts conscutive bytes and records their number
    /// For example `[1, 1, 1, 0]` would become `"1_3-0_1"`
    fn compressed(&self) -> Result<String, fmt::Error> {
        let mut res = String::with_capacity(Self::ROW_SIZE * Self::ROWS);
        let mut current: u8 = self.row(0).as_ref()[0];
        let mut count: usize = 0;
        for row in 0..Self::ROWS {
            let row = self.row(row);
            for byte in 0..Self::ROW_SIZE {
                let byte = row[byte];
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
    fn decompress(value: &str) -> Result<Self, &'static str> {
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
                res.row_mut(row)[col] = value;
                col += 1;
                if col >= Self::ROW_SIZE {
                    col = 0;
                    row += 1;
                }
            }
        }

        Ok(res)
    }
}

