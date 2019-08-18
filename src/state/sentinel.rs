use screeps::raw_memory;
use serde::Serialize;
use std::marker::PhantomData;
use std::ptr::NonNull;

pub struct MemorySentinel<'a, T: Serialize> {
    pub memory_segment: u8,
    pub save_to_memory: bool,

    item: NonNull<T>,

    _m: PhantomData<&'a i8>,
}

impl<'a, T: Serialize> MemorySentinel<'a, T> {
    pub fn new(memory_segment: u8, item: &T) -> Self {
        Self {
            item: NonNull::from(item),
            memory_segment,
            save_to_memory: true,
            _m: PhantomData,
        }
    }
}

impl<'a, T: Serialize> Drop for MemorySentinel<'a, T> {
    fn drop(&mut self) {
        if !self.save_to_memory {
            return;
        }
        let segment = self.memory_segment;

        unsafe {
            match serde_json::to_string(self.item.as_ref()) {
                Ok(data) => {
                    raw_memory::set_segment(segment as u32, data.as_str());
                }
                Err(e) => {
                    error!("Failed to serialize {:?}", e);
                }
            }
        }
    }
}

