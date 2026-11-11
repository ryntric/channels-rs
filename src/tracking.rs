use crate::buffer::Buffer;
use crate::sequence::INITIAL_VALUE;
use core::sync::atomic::Ordering::{Acquire, Relaxed, Release};
use core::sync::atomic::{AtomicI64, fence};

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
    /// slot has since been reused by a later lap. This keeps a lagging observer from
    /// getting stuck on a slot that has already been overwritten.
    ///
    /// Slots are read with `Relaxed` loads and a single `Acquire` fence is issued
    /// afterwards (only if at least one slot was observed as published). The fence
    /// synchronizes with the `Release` store / fence of every writer whose value was
    /// observed, so all data written before those publications is visible to the caller.
    #[inline]
    pub fn get_available(&self, low: i64, high: i64) -> i64 {
        let mut available = high;
        for sequence in low..high + 1 {
            let value = unsafe { self.inner.get_ref(sequence) };
            if value.load(Relaxed) < sequence {
                available = sequence - 1;
                break;
            }
        }

        if available >= low {
            fence(Acquire);
        }
        available
    }

    #[inline(always)]
    pub fn set(&self, sequence: i64) {
        let value = unsafe { self.inner.get_ref(sequence) };
        value.store(sequence, Release);
    }

    /// Publishes every slot in `low..=high`.
    ///
    /// A single `Release` fence followed by `Relaxed` stores replaces one `Release`
    /// store per slot: any reader that observes one of these stores and then issues an
    /// `Acquire` fence (see [`Self::get_available`]) synchronizes with this fence.
    #[inline]
    pub fn set_range(&self, low: i64, high: i64) {
        if low == high {
            self.set(low);
            return;
        }

        fence(Release);
        for sequence in low..high + 1 {
            let value = unsafe { self.inner.get_ref(sequence) };
            value.store(sequence, Relaxed);
        }
    }
}
