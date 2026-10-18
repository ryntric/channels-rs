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

    /// Writes a value into the slot identified by the sequence.
    ///
    /// # Safety
    ///
    /// - The target slot must be logically uninitialized.
    /// - The caller must have exclusive access to the slot.
    /// - No other thread may concurrently access the slot.
    /// - Any previous value must have been moved out or dropped.
    pub unsafe fn write(&self, sequence: i64, value: T) {
        // SAFETY: The caller guarantees exclusive access
        // and that the target slot is logically uninitialized.
        unsafe { self.buffer.write(sequence, value) }
    }

    /// Moves the initialized value out of the specified slot.
    ///
    /// # Safety
    ///
    /// - The slot must contain a fully initialized value.
    /// - The caller must have exclusive access to the slot.
    /// - No other thread may concurrently access the slot.
    /// - The value must not be read or dropped again
    ///   until the slot has been reinitialized.
    pub unsafe fn take(&self, sequence: i64) -> T {
        // SAFETY: The caller guarantees that the slot
        // is initialized and exclusively owned.
        unsafe { self.buffer.take(sequence) }
    }

    /// Drops all values in the inclusive range `[low, high]`.
    ///
    /// # Safety
    ///
    /// - Every slot in the range must contain an initialized value.
    /// - The caller must have exclusive access to all affected slots.
    /// - The range must not contain duplicate physical slots.
    /// - No other thread may concurrently access these slots.
    /// - The caller must ensure `low <= high` and that the
    ///   range contains at most `capacity` elements.
    /// - Dropped values must not be accessed again until
    ///   their slots have been reinitialized.
    pub unsafe fn free(&self, low: i64, high: i64) {
        // SAFETY: The caller guarantees that every affected slot
        // is initialized, exclusively owned, and dropped exactly once.
        unsafe { self.buffer.free(low, high) }
    }
}
