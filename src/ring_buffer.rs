use crate::buffer::Buffer;

pub(crate) struct RingBuffer<T> {
    buffer: Buffer<Option<T>>,
}

impl<T> RingBuffer<T> {
    pub fn new(capacity: usize) -> Self {
        Self {
            buffer: Buffer::with_default(capacity),
        }
    }

    pub fn capacity(&self) -> i64 {
        self.buffer.capacity()
    }

    pub unsafe fn write(&self, sequence: i64, value: T) {
        unsafe { self.buffer.write(sequence, Some(value)) }
    }

    pub unsafe fn take(&self, sequence: i64) -> Option<T> {
        unsafe { self.buffer.take(sequence) }
    }
}
