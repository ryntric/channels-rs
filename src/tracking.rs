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

    #[inline]
    pub fn get_available(&self, low: i64, high: i64, limit: i64) -> i64 {
        for sequence in low..=high.min(limit) {
            let value = unsafe { self.inner.get_ref(sequence) };
            if sequence != value.load(Acquire) {
                return sequence - 1;
            }
        }
        high
    }

    #[inline(always)]
    pub fn set(&self, sequence: i64) {
        let value = unsafe { self.inner.get_ref(sequence) };
        value.store(sequence, Release);
    }

    #[inline(always)]
    pub fn set_range(&self, low: i64, high: i64) {
        for sequence in low..=high {
            self.set(sequence);
        }
    }
}
