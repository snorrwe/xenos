pub mod arrayqueue;
pub mod trait_impl;

pub use self::arrayqueue::*;
pub use self::trait_impl::*;

use std::fmt;
use std::ops::{Add, Sub};

pub trait Index:
    Add + Sub + Default + Copy + Clone + Eq + PartialEq + Ord + PartialOrd + fmt::Debug
{
    fn as_usize(self) -> usize;
    fn from_usize(s: usize) -> Self;
}

pub trait Container {
    type Item;
    type Index: Index;

    fn capacity() -> usize;

    fn get(&self, index: Self::Index) -> &Self::Item;
    fn get_mut(&mut self, index: Self::Index) -> &mut Self::Item;
    fn set(&mut self, index: Self::Index, item: Self::Item);
}

impl Index for u8 {
    #[inline]
    fn as_usize(self) -> usize {
        self as usize
    }
    #[inline]
    fn from_usize(s: usize) -> Self {
        debug_assert!(s < std::u8::MAX as usize);
        s as u8
    }
}

impl Index for u16 {
    #[inline]
    fn as_usize(self) -> usize {
        self as usize
    }
    #[inline]
    fn from_usize(s: usize) -> Self {
        debug_assert!(s < std::u16::MAX as usize);
        s as u16
    }
}
