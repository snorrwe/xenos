use serde::{de, ser::SerializeSeq, Deserialize, Deserializer, Serialize, Serializer};
use std::fmt;
use std::marker::PhantomData;
use std::mem::{self, MaybeUninit};
use super::{Container, Index};

#[derive(Debug, Clone, Copy)]
pub enum QueueError {
    Full,
    Empty,
}

/// Fix sized ring buffering FIFO queue
// Invariant: head <= tail
#[derive(Clone)]
pub struct ArrayQueue<C>
where
    C: Container,
{
    head: C::Index,
    tail: C::Index,
    buff: C,
}

impl<T, C> fmt::Debug for ArrayQueue<C>
where
    C: Container<Item = T>,
    T: fmt::Debug,
{
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Head: {:?} Tail: {:?}\nItems: [", self.head, self.tail)?;
        for i in 0..self.capacity() {
            write!(f, "{:?},", self.buff.get(C::Index::from_usize(i)))?;
        }
        write!(f, "]")?;
        Ok(())
    }
}

impl<'de, T, It> From<It> for ArrayQueue<T>
where
    T: Container,
    It: Iterator<Item = T::Item>,
{
    fn from(v: It) -> Self {
        let mut result = Self::default();
        v.take(T::capacity()).for_each(|i| {
            result.push_back(i);
        });
        result
    }
}

impl<T: Container> Default for ArrayQueue<T> {
    fn default() -> Self {
        let buff: T = unsafe { MaybeUninit::zeroed().assume_init() };
        let result = Self {
            buff,
            head: Default::default(),
            tail: Default::default(),
        };
        result
    }
}

impl<C> ArrayQueue<C>
where
    C: Container,
{
    pub fn extend<It>(&mut self, it: It)
    where
        It: Iterator<Item = C::Item>,
    {
        for i in it {
            self.push_back(i);
        }
    }

    /// Try to push an item, fail if the queue is full
    pub fn try_push_back(&mut self, item: C::Item) -> Result<&mut C::Item, QueueError> {
        let tail = Self::increment_one(self.tail);
        if self.head == tail {
            Err(QueueError::Full)?;
        }
        self.buff.set(self.tail, item);
        let result = self.buff.get_mut(self.tail);
        self.tail = tail;
        Ok(result)
    }

    fn increment_one(ind: C::Index) -> C::Index {
        C::Index::from_usize(increment_one::<C>(ind.as_usize()))
    }

    fn decrease_one(ind: C::Index) -> C::Index {
        C::Index::from_usize(decrease_one::<C>(ind.as_usize()))
    }

    /// Push an item, the queue loses the first item if the queue is full
    pub fn push_back(&mut self, item: C::Item) -> &mut C::Item {
        self.buff.set(self.tail, item);
        let result = self.buff.get_mut(self.tail);
        let tail = Self::increment_one(self.tail);
        if self.head == tail {
            self.head = Self::increment_one(self.head);
        }
        self.tail = tail;
        result
    }

    /// Pop the first item if any
    #[allow(unused)]
    pub fn try_pop_front(&mut self) -> Result<C::Item, QueueError> {
        if self.head == self.tail {
            Err(QueueError::Empty)?;
        }
        let result = unsafe {
            // Copy the bits out of the buffer and replace them with zeros
            let garbage = MaybeUninit::zeroed().assume_init();
            let result = mem::replace(self.buff.get_mut(self.head), garbage);
            result
        };
        self.head = Self::increment_one(self.head);
        Ok(result)
    }

    /// Peek the first element
    #[allow(unused)]
    pub fn front(&self) -> Result<&C::Item, QueueError> {
        if self.head == self.tail {
            Err(QueueError::Empty)?;
        }

        let result = self.buff.get(self.head);
        Ok(result)
    }

    /// Peek the last element
    #[allow(unused)]
    pub fn back(&self) -> Result<&C::Item, QueueError> {
        if self.head == self.tail {
            Err(QueueError::Empty)?;
        }
        let tail = Self::decrease_one(self.tail);
        let result = self.buff.get(tail);
        Ok(result)
    }

    pub fn iter<'a>(&'a self) -> QueueIterator<'a, C> {
        QueueIterator {
            container: &self.buff,
            head: self.head,
            tail: self.tail,
        }
    }

    pub fn len(&self) -> usize {
        let head = self.head.as_usize();
        let tail = self.tail.as_usize();
        if head <= tail {
            tail - head
        } else {
            let b = head - tail; // Skipped items
            C::capacity() - b
        }
    }

    #[allow(unused)]
    pub fn capacity(&self) -> usize {
        C::capacity()
    }
}

pub struct QueueIterator<'a, C>
where
    C: Container,
{
    head: C::Index,
    tail: C::Index,
    container: &'a C,
}

impl<'a, C> Iterator for QueueIterator<'a, C>
where
    C: Container,
{
    type Item = &'a C::Item;

    fn next(&mut self) -> Option<Self::Item> {
        if self.head == self.tail {
            None
        } else {
            let res = self.container.get(self.head);
            self.head = ArrayQueue::<C>::increment_one(self.head);
            Some(res)
        }
    }
}

impl<T, C> Serialize for ArrayQueue<C>
where
    T: Serialize,
    C: Container<Item = T>,
{
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let mut seq = serializer.serialize_seq(Some(self.len()))?;
        for e in self.iter() {
            seq.serialize_element(e)?;
        }
        seq.end()
    }
}

impl<'de, T, C> de::Deserialize<'de> for ArrayQueue<C>
where
    T: Serialize + Deserialize<'de>,
    C: Container<Item = T>,
{
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        struct Visitor<T, C>(PhantomData<fn() -> (T, C)>);

        impl<'de, T, C> de::Visitor<'de> for Visitor<T, C>
        where
            T: Deserialize<'de> + Serialize,
            C: Container<Item = T>,
        {
            type Value = ArrayQueue<C>;

            fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
                formatter.write_str("a sequence")
            }

            fn visit_seq<S>(self, mut seq: S) -> Result<Self::Value, S::Error>
            where
                S: de::SeqAccess<'de>,
            {
                let mut result = Self::Value::default();
                while let Some(val) = seq.next_element()? {
                    result.try_push_back(val).map_err(|e| {
                        de::Error::custom(format!("Sequence is too large! {:?}", e))
                    })?;
                }
                Ok(result)
            }
        }
        let visitor = Visitor(PhantomData);
        deserializer.deserialize_seq(visitor)
    }
}

/// Next element
#[inline]
fn increment_one<C: Container>(ind: usize) -> usize {
    (ind + 1) % C::capacity()
}

/// Prev element
#[inline]
fn decrease_one<C: Container>(ind: usize) -> usize {
    (ind + C::capacity() - 1) % C::capacity()
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json;

    const SIZE: usize = 64;

    #[test]
    fn test_serialize() {
        js! {}; // Enables error messages in tests

        let queue: ArrayQueue<[usize; SIZE]> =
            [1, 2, 3, 4, 5, 6, 7, 8, 9].into_iter().map(|x| *x).into();

        assert_eq!(queue.len(), 9);

        let s = serde_json::to_string(&queue).expect("Failed to seralize queue");

        assert_eq!(s, "[1,2,3,4,5,6,7,8,9]");
    }

    #[test]
    fn test_deserialize() {
        js! {}; // Enables error messages in tests

        let s = "[6,7,8,9,1,2,3,4,5]";

        let q: ArrayQueue<[i32; SIZE]> = serde_json::from_str(s).expect("Failed to deserialize");

        assert_eq!(q.len(), 9);

        for (x, y) in [6, 7, 8, 9, 1, 2, 3, 4, 5].into_iter().zip(q.iter()) {
            assert_eq!(x, y);
        }
    }

    #[test]
    fn test_len() {
        js! {};

        let mut q = ArrayQueue::<[usize; 64]>::default();

        assert_eq!(q.len(), 0);

        for i in 0..=(SIZE * 3) {
            q.push_back(i);
        }

        assert_eq!(q.len(), SIZE - 1); // 1 value is retained as a buffer

        let expected = SIZE * 2 + 2..SIZE * 3; // Because capacity is 1 less than the size the first value will be 2 past SIZE*2

        for (x, y) in q.iter().zip(expected) {
            assert_eq!(*x, y);
        }
    }

    #[test]
    fn test_pops() {
        js! {};

        let mut queue: ArrayQueue<[i32; SIZE]> =
            [1, 2, 3, 4, 5, 6, 7, 8, 9].into_iter().map(|x| *x).into();

        assert_eq!(queue.len(), 9);

        for i in 1..=9 {
            let n = queue
                .try_pop_front()
                .expect("Expected more values in the queue");
            assert_eq!(n, i);
        }

        assert_eq!(queue.len(), 0);
        queue
            .try_pop_front()
            .expect_err("Expected the queue to be empty");
    }

    #[test]
    fn test_peek() {
        js! {};

        let mut queue: ArrayQueue<[i32; SIZE]> =
            [1, 2, 3, 4, 5, 6, 7, 8, 9].into_iter().map(|x| *x).into();

        assert_eq!(queue.len(), 9);

        let first = queue.front().expect("Expected queue not to be empty");
        let last = queue.back().expect("Expected queue not to be empty");
        assert_eq!(*first, 1);
        assert_eq!(*last, 9);

        for _ in 0..2 {
            queue
                .try_pop_front()
                .expect("Expected more values in the queue");
        }

        let first = queue.front().expect("Expected queue not to be empty");
        let last = queue.back().expect("Expected queue not to be empty");
        assert_eq!(*first, 3);
        assert_eq!(*last, 9);
    }

}

