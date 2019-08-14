pub mod trait_impl;

pub use self::trait_impl::*;

use std::fmt::{self, Write};

/// Specify size in bytes
/// Hold 1 byte of data per tile
pub trait FlagGrid: Default + Sized {
    const ROWS: usize;
    const COLS: usize;

    fn buffer(&self) -> &[u8];
    fn buffer_mut(&mut self) -> &mut [u8];

    fn get(&self, x: usize, y: usize) -> u8 {
        let ind = Self::get_flat_index(x, y);
        self.buffer()[ind]
    }

    fn set(&mut self, x: usize, y: usize, value: u8) {
        let ind = Self::get_flat_index(x, y);
        self.buffer_mut()[ind] = value;
    }

    /// Create a compressed String representation
    /// Structure: `"([0-9]{1,3})_([0-9]{1,3});?"`
    /// The compression counts conscutive bytes and records their number
    /// For example `[1, 1, 1, 0]` would become `"1_3-0_1"`
    fn compressed(&self) -> Result<String, fmt::Error> {
        let mut res = String::with_capacity(Self::COLS * Self::ROWS);
        let mut current: u8 = self.buffer()[0];
        let mut count: usize = 0;
        for byte in self.buffer().iter().cloned() {
            if byte == current {
                count += 1;
            } else {
                write!(res, "{}_{};", current, count)?;
                current = byte;
                count = 1;
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

        let mut ind = 0;
        let buff = res.buffer_mut();

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
                buff[ind] = value;
                ind += 1;
            }
        }

        Ok(res)
    }

    #[inline]
    fn get_flat_index(x: usize, y: usize) -> usize {
        let flat_ind = x * Self::ROWS + y;
        flat_ind as usize
    }
}

