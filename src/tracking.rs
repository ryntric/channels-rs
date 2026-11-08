use crate::buffer::Buffer;
use crate::sequence::INITIAL_VALUE;
use core::sync::atomic::AtomicI64;
use core::sync::atomic::Ordering::{Acquire, Release};

pub(crate) struct TrackingBuffer {
    internal: Buffer<AtomicI64>,
}

impl TrackingBuffer {
    pub fn new(capacity: usize) -> Self {
        Self {
            internal: Buffer::with_initializer(capacity, || AtomicI64::new(INITIAL_VALUE)),
        }
    }

    pub fn get_available(&self, low: i64, high: i64) -> i64 {
        for sequence in low..=high {
            let generation = unsafe { self.internal.with_ref(sequence, |value| value.load(Acquire)) };
            if sequence != generation {
                return sequence - 1;
            }
        }
        high
    }

    pub fn set(&self, sequence: i64) {
        unsafe { self.internal.with_ref(sequence, |value| value.store(sequence, Release)) }
    }

    pub fn set_range(&self, low: i64, high: i64) {
        for sequence in low..=high {
            self.set(sequence);
        }
    }
}
