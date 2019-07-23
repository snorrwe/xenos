use super::Container;
use super::Index;

macro_rules! impl_container {
    ($size: expr, $ind: ty) => {
        impl<T> Container for [T; $size] {
            type Item = T;
            type Index = $ind;

            fn capacity() -> usize {
                $size
            }
            fn get(&self, index: Self::Index) -> &Self::Item {
                &self[index.as_usize()]
            }
            fn get_mut(&mut self, index: Self::Index) -> &mut Self::Item {
                &mut self[index.as_usize()]
            }
            fn set(&mut self, index: Self::Index, item: Self::Item) {
                self[index.as_usize()] = item;
            }
        }
    };
}

impl_container!(1, u8);
impl_container!(2, u8);
impl_container!(4, u8);
impl_container!(8, u8);
impl_container!(16, u8);
impl_container!(32, u8);
impl_container!(64, u8);
impl_container!(128, u8);
impl_container!(256, u16);
impl_container!(512, u16);
impl_container!(1024, u16);
