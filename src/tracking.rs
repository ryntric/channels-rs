use crate::buffer::Buffer;
use crate::sequence::INITIAL_VALUE;
use core::sync::atomic::AtomicI64;
use core::sync::atomic::Ordering::{Acquire, Release};

pub(crate) struct TrackingBuffer {
    inner: Buffer<AtomicI64>,
}

impl TrackingBuffer {
    pub fn new(capacity: usize) -> Self {
        let buffer = Buffer::new(capacity);
        let capacity = capacity as i64;
        for i in 0..capacity {
            unsafe { buffer.write(i, AtomicI64::new(INITIAL_VALUE)) }
        }
        Self { inner: buffer }
    }

    /// Returns the highest sequence `s` in `low - 1..=high` such that every
    /// slot in `low..=s` has been published.
    ///
    /// Slot values only grow (`s`, `s + capacity`, `s + 2 * capacity`, ...), so a value
    /// `>= sequence` means `sequence` itself was published at some point, even if the
    /// slot has since been reused by a later lap. This keeps an observer that lags by
    /// more than one lap from getting stuck on an already overwritten slot.
    pub fn get_available(&self, low: i64, high: i64) -> i64 {
        for sequence in low..=high {
            let value = unsafe { self.inner.get_ref(sequence) };
            if value.load(Acquire) < sequence {
                return sequence - 1;
            }
        }
        high
    }

    pub fn set(&self, sequence: i64) {
        let value = unsafe { self.inner.get_ref(sequence) };
        value.store(sequence, Release);
    }

    pub fn set_range(&self, low: i64, high: i64) {
        for sequence in low..=high {
            self.set(sequence);
        }
    }
}
