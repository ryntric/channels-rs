use std::sync::atomic::{AtomicI64, Ordering};

pub const INITIAL_VALUE: i64 = -1;

#[repr(align(64))]
pub struct Sequence {
    sequence: AtomicI64,
}

unsafe impl Sync for Sequence {}

unsafe impl Send for Sequence {}

impl Sequence {
    pub fn new(value: i64) -> Self {
        Sequence {
            sequence: AtomicI64::new(value),
        }
    }

    pub fn get_relaxed(&self) -> i64 {
        self.sequence.load(Ordering::Relaxed)
    }

    pub fn set_relaxed(&self, value: i64) {
        self.sequence.store(value, Ordering::Relaxed);
    }

    pub fn get_acquire(&self) -> i64 {
        self.sequence.load(Ordering::Acquire)
    }

    pub fn set_release(&self, value: i64) {
        self.sequence.store(value, Ordering::Release);
    }

    pub fn fetch_add_volatile(&self, value: i64) -> i64 {
        self.sequence.fetch_add(value, Ordering::AcqRel)
    }

    #[allow(unused)]
    pub fn compare_and_exchange_volatile(&self, current: i64, new: i64) -> bool {
        self.sequence
            .compare_exchange(current, new, Ordering::AcqRel, Ordering::Relaxed)
            .is_ok()
    }

    pub fn compare_and_exchange_weak_volatile(&self, current: i64, new: i64) -> bool {
        self.sequence
            .compare_exchange_weak(current, new, Ordering::AcqRel, Ordering::Relaxed)
            .is_ok()
    }
}

impl Default for Sequence {
    fn default() -> Self {
        Sequence::new(INITIAL_VALUE)
    }
}
