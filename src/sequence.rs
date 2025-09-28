use std::sync::atomic::{AtomicI64, Ordering};

pub const INITIAL_VALUE: i64 = -1;

#[repr(align(64))]
pub struct Sequence {
    sequence: AtomicI64,
}

impl Sequence {
    pub fn new(value: i64) -> Self {
        Sequence {
            sequence: AtomicI64::new(value),
        }
    }

    #[inline(always)]
    pub fn get_relaxed(&self) -> i64 {
        self.sequence.load(Ordering::Relaxed)
    }

    #[inline(always)]
    pub fn set_relaxed(&self, value: i64) {
        self.sequence.store(value, Ordering::Relaxed);
    }

    #[inline(always)]
    pub fn get_acquire(&self) -> i64 {
        self.sequence.load(Ordering::Acquire)
    }

    #[inline(always)]
    pub fn set_release(&self, value: i64) {
        self.sequence.store(value, Ordering::Release);
    }

    #[inline(always)]
    pub fn fetch_add_volatile(&self, value: i64) -> i64 {
        self.sequence.fetch_add(value, Ordering::AcqRel)
    }
}

impl Default for Sequence {
    fn default() -> Self {
        Sequence::new(INITIAL_VALUE)
    }
}

unsafe impl Sync for Sequence {}

unsafe impl Send for Sequence {}