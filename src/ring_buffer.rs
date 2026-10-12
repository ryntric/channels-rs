use crate::buffer::Buffer;

pub(crate) struct RingBuffer<T> {
    buffer: Buffer<T>,
}

impl<T> RingBuffer<T> {
    pub fn new(capacity: usize) -> Self {
        Self {
            buffer: Buffer::new(capacity),
        }
    }

    pub fn capacity(&self) -> i64 {
        self.buffer.capacity()
    }

    pub unsafe fn write(&self, sequence: i64, value: T) {
        unsafe { self.buffer.write(sequence, value) }
    }

    pub unsafe fn take(&self, sequence: i64) -> T {
        unsafe { self.buffer.take(sequence) }
    }

    pub unsafe fn clear(&self, low: i64, high: i64) {
        unsafe { self.buffer.clear(low, high) }
    }
}
