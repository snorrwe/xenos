use super::{Container, Index};
use serde::{de, ser::SerializeSeq, Deserialize, Deserializer, Serialize, Serializer};
use std::fmt;
use std::marker::PhantomData;
use std::mem::{self, MaybeUninit};

#[derive(Debug, Clone, Copy)]
pub enum QueueError {
    Full,
    Empty,
}

/// Fixed size, compact deque
// Invariant: head <= tail
// Uses a ring buffer internally
#[derive(Clone)]
pub struct ArrayQueue<C>
where
    C: Container,
{
    head: C::Index,
    size: usize,
    buff: C,
}

impl<T, C> fmt::Debug for ArrayQueue<C>
where
    C: Container<Item = T>,
    T: fmt::Debug,
{
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Head: {:?} Size: {:?}\nItems: [", self.head, self.size)?;
        for i in 0..self.capacity() {
            write!(f, "{:?},", self.buff.get(C::Index::from_usize(i)))?;
        }
        write!(f, "]")
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
            size: Default::default(),
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

    #[allow(unused)]
    pub fn try_push_front(&mut self, item: C::Item) -> Result<&mut C::Item, QueueError> {
        let size = self.size + 1;
        if size > self.capacity() {
            Err(QueueError::Full)?;
        }
        self.size = size;
        self.head = Self::decrease_one(self.head);
        self.buff.set(self.head, item);
        let result = self.buff.get_mut(self.head);
        Ok(result)
    }

    #[allow(unused)]
    /// Push to the front of the queue
    /// Remove the last element if the queue is full
    pub fn push_front(&mut self, item: C::Item) -> &mut C::Item {
        let size = self.size + 1;
        if size < self.capacity() {
            self.size = size;
        }
        self.head = Self::decrease_one(self.head);
        self.buff.set(self.head, item);
        let result = self.buff.get_mut(self.head);
        result
    }

    /// Try to push an item, fail if the queue is full
    pub fn try_push_back(&mut self, item: C::Item) -> Result<&mut C::Item, QueueError> {
        let size = self.size + 1;
        if size > self.capacity() {
            Err(QueueError::Full)?;
        }
        self.size = size;
        let tail = self.tail();
        self.buff.set(tail, item);
        let result = self.buff.get_mut(tail);
        Ok(result)
    }

    /// Push an item, the queue loses the first item if the queue is full
    pub fn push_back(&mut self, item: C::Item) -> &mut C::Item {
        let size = self.size + 1;
        if size > self.capacity() {
            self.head = Self::increment_one(self.head);
        } else {
            self.size = size;
        }

        let tail = self.tail();
        self.buff.set(tail, item);
        let result = self.buff.get_mut(tail);
        result
    }

    /// Pop the first item if any
    #[allow(unused)]
    pub fn try_pop_front(&mut self) -> Result<C::Item, QueueError> {
        if self.size == 0 {
            Err(QueueError::Empty)?;
        }
        let result = unsafe {
            // Copy the bits out of the buffer and replace them with zeros
            let garbage = MaybeUninit::zeroed().assume_init();
            let result = mem::replace(self.buff.get_mut(self.head), garbage);
            result
        };
        self.size -= 1;
        self.head = Self::increment_one(self.head);
        Ok(result)
    }

    /// Peek the first element
    #[allow(unused)]
    pub fn front(&self) -> Result<&C::Item, QueueError> {
        if self.size == 0 {
            Err(QueueError::Empty)?;
        }

        let result = self.buff.get(self.head);
        Ok(result)
    }

    /// Peek the last element
    #[allow(unused)]
    pub fn back(&self) -> Result<&C::Item, QueueError> {
        if self.size == 0 {
            Err(QueueError::Empty)?;
        }
        let tail = self.tail();
        let result = self.buff.get(tail);
        Ok(result)
    }

    pub fn iter<'a>(&'a self) -> QueueIterator<'a, C> {
        if self.size == 0 {
            QueueIterator {
                container: &self.buff,
                head: self.head,
                tail: self.head,
            }
        } else {
            QueueIterator {
                container: &self.buff,
                head: self.head,
                tail: Self::increment_one(self.tail()),
            }
        }
    }

    pub fn len(&self) -> usize {
        self.size
    }

    pub fn capacity(&self) -> usize {
        C::capacity()
    }

    #[inline]
    fn increment_one(ind: C::Index) -> C::Index {
        let ind = ind.as_usize();
        let ind = (ind + 1) % C::capacity();
        C::Index::from_usize(ind)
    }

    #[allow(unused)]
    #[inline]
    fn decrease_one(ind: C::Index) -> C::Index {
        let ind = ind.as_usize();
        let ind = (ind + C::capacity() - 1) % C::capacity();
        C::Index::from_usize(ind)
    }

    #[inline]
    fn tail(&self) -> C::Index {
        debug_assert!(self.size != 0);
        let tail = self.head.as_usize() + self.size - 1;
        let tail = tail % self.capacity();
        C::Index::from_usize(tail)
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

        let mut q = ArrayQueue::<[usize; SIZE]>::default();

        assert_eq!(q.len(), 0);

        for i in 0..=(SIZE * 3) {
            q.push_back(i);
        }

        assert_eq!(q.len(), SIZE);
        let expected = SIZE * 2 + 1..SIZE * 3;
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

