use serde::{de, ser::SerializeSeq, Deserialize, Deserializer, Serialize, Serializer};
use std::fmt;
use std::marker::PhantomData;
use std::mem::MaybeUninit;

#[derive(Debug, Clone, Copy)]
pub enum QueueError {
    Full,
    Empty,
}

// TODO: generate multiple sizes of queues
const SIZE: usize = 128;

// Invariant: head <= tail
#[derive(Clone)]
pub struct ArrayQueue<Item>
where
    Item: Clone,
{
    head: i16,
    tail: i16,
    buff: [Item; SIZE],
}

impl<T> fmt::Debug for ArrayQueue<T>
where
    T: Clone + fmt::Debug,
{
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Head: {} Tail: {}\nItems: [", self.head, self.tail)?;
        for i in self.buff.iter() {
            write!(f, "{:?},", i)?;
        }
        write!(f, "]")?;
        Ok(())
    }
}

impl<'de, T, It: Iterator<Item = T>> From<It> for ArrayQueue<T>
where
    T: Deserialize<'de> + Serialize + Clone,
{
    fn from(v: It) -> Self {
        let mut result = Self::default();
        v.take(SIZE).for_each(|i| {
            result.push_back(i);
        });
        result
    }
}

impl<T> Default for ArrayQueue<T>
where
    T: Clone,
{
    fn default() -> Self {
        let buff = unsafe { MaybeUninit::zeroed().assume_init() };
        Self {
            head: 0,
            tail: 0,
            buff,
        }
    }
}

impl<T> ArrayQueue<T>
where
    T: Clone,
{
    pub fn extend<It>(&mut self, it: It)
    where
        It: Iterator<Item = T>,
    {
        for i in it {
            self.push_back(i);
        }
    }

    /// Try to push an item, fail if the queue is full
    pub fn try_push_back(&mut self, item: T) -> Result<&mut T, QueueError> {
        let tail = increment_one(self.tail);
        if self.head == tail {
            Err(QueueError::Full)?;
        }
        self.buff[self.tail as usize] = item;
        let result = &mut self.buff[self.tail as usize];
        self.tail = tail;
        Ok(result)
    }

    /// Push an item, the queue loses the first item if the queue is full
    pub fn push_back(&mut self, item: T) -> &mut T {
        let tail = increment_one(self.tail);
        self.buff[self.tail as usize] = item;
        let result = &mut self.buff[self.tail as usize];
        if self.head == tail {
            self.head = increment_one(self.head);
        }
        self.tail = tail;
        result
    }

    /// Pop the first item if any
    #[allow(unused)]
    pub fn try_pop_front(&mut self) -> Result<T, QueueError> {
        if self.head == self.tail {
            Err(QueueError::Empty)?;
        }
        let result = self.buff[self.head as usize].clone();
        self.head = increment_one(self.head);
        Ok(result)
    }

    /// Peek the first element
    #[allow(unused)]
    pub fn front(&self) -> Result<&T, QueueError> {
        if self.head == self.tail {
            Err(QueueError::Empty)?;
        }

        let result = &self.buff[self.head as usize];
        Ok(result)
    }

    /// Peek the last element
    #[allow(unused)]
    pub fn back(&self) -> Result<&T, QueueError> {
        if self.head == self.tail {
            Err(QueueError::Empty)?;
        }

        let result = &self.buff[decrement_one(self.tail) as usize];
        Ok(result)
    }

    /// Creates a copy of the queue in a vector
    #[allow(unused)]
    pub fn as_vec(&self) -> Vec<T> {
        self.iter().cloned().collect()
    }

    pub fn iter<'a>(&'a self) -> QueueIterator<'a, T> {
        QueueIterator {
            queue: self,
            head: self.head,
            tail: self.tail,
        }
    }

    pub fn len(&self) -> usize {
        let head = self.head as usize;
        let tail = self.tail as usize;
        if head <= tail {
            tail - head
        } else {
            SIZE - head + tail
        }
    }

    #[allow(unused)]
    pub fn capacity(&self) -> usize {
        SIZE - 1
    }
}

pub struct QueueIterator<'a, T>
where
    T: Clone,
{
    queue: &'a ArrayQueue<T>,
    head: i16,
    tail: i16,
}

impl<'a, T> Iterator for QueueIterator<'a, T>
where
    T: Clone,
{
    type Item = &'a T;

    fn next(&mut self) -> Option<Self::Item> {
        if self.head == self.tail {
            None
        } else {
            let res = &self.queue.buff[self.head as usize];
            self.head = increment_one(self.head);
            Some(res)
        }
    }
}

impl<T> Serialize for ArrayQueue<T>
where
    T: Clone + Serialize,
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

impl<'de, T> de::Deserialize<'de> for ArrayQueue<T>
where
    T: Serialize + Deserialize<'de> + Clone,
{
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        struct Visitor<T>(PhantomData<fn() -> T>);

        impl<'de, T> de::Visitor<'de> for Visitor<T>
        where
            T: Deserialize<'de> + Serialize + Clone,
        {
            type Value = ArrayQueue<T>;

            fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
                formatter.write_str("a nonempty sequence of numbers")
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

#[inline]
fn increment_one(ind: i16) -> i16 {
    (ind + 1) % SIZE as i16
}

#[allow(unused)]
#[inline]
fn decrement_one(ind: i16) -> i16 {
    (ind + SIZE as i16 - 1) % SIZE as i16
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json;

    #[test]
    fn test_serialize() {
        js! {}; // Enables error messages in tests

        let queue: ArrayQueue<_> = [1, 2, 3, 4, 5, 6, 7, 8, 9].into_iter().map(|x| *x).into();

        assert_eq!(queue.len(), 9);

        let s = serde_json::to_string(&queue).expect("Failed to seralize queue");

        assert_eq!(s, "[1,2,3,4,5,6,7,8,9]");
    }

    #[test]
    fn test_deserialize() {
        js! {}; // Enables error messages in tests

        let s = "[6,7,8,9,1,2,3,4,5]";

        let q: ArrayQueue<i32> = serde_json::from_str(s).expect("Failed to deserialize");

        assert_eq!(q.len(), 9);

        for (x, y) in [6, 7, 8, 9, 1, 2, 3, 4, 5].into_iter().zip(q.iter()) {
            assert_eq!(x, y);
        }
    }

    #[test]
    fn test_len() {
        js! {};

        let mut q = ArrayQueue::default();

        assert_eq!(q.len(), 0);

        for i in 0..=(SIZE * 3) {
            q.push_back(i);
        }

        assert_eq!(q.len(), SIZE - 1); // 1 value is retained as a buffer

        let expected = SIZE * 2 + 2..SIZE * 3; // Because capacity is 1 less than the size the first value will be 2 past SIZE*2

        for ((x, y), z) in q.iter().zip(q.as_vec().into_iter()).zip(expected) {
            assert_eq!(*x, y);
            assert_eq!(y, z);
        }
    }

    #[test]
    fn test_pops() {
        js! {};

        let mut queue: ArrayQueue<_> = [1, 2, 3, 4, 5, 6, 7, 8, 9].into_iter().map(|x| *x).into();

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

        let mut queue: ArrayQueue<_> = [1, 2, 3, 4, 5, 6, 7, 8, 9].into_iter().map(|x| *x).into();

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

