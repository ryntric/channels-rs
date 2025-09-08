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

    pub fn get_plain(&self) -> i64 {
        self.sequence.load(Ordering::Relaxed)
    }

    pub fn set_plain(&self, value: i64) {
        self.sequence.store(value, Ordering::Relaxed);
    }

    pub fn get_acquire(&self) -> i64 {
        self.sequence.load(Ordering::Acquire)
    }

    pub fn set_release(&self, value: i64) {
        self.sequence.store(value, Ordering::Release);
    }

    pub fn get_and_add_volatile(&self, value: i64) -> i64 {
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