use std::cell::Cell;
use std::ops::Deref;
use std::sync::atomic::AtomicI64;

pub(crate) const INITIAL_VALUE: i64 = -1;

#[repr(align(64))]
pub(crate) struct LocalSequence {
    inner: Cell<i64>,
}

impl Deref for LocalSequence {
    type Target = Cell<i64>;

    fn deref(&self) -> &Self::Target {
        &self.inner
    }
}

impl Default for LocalSequence {
    fn default() -> Self {
        Self {
            inner: Cell::new(INITIAL_VALUE),
        }
    }
}

#[repr(align(64))]
pub struct SharedSequence {
    inner: AtomicI64,
}

impl Deref for SharedSequence {
    type Target = AtomicI64;

    fn deref(&self) -> &Self::Target {
        &self.inner
    }
}

impl Default for SharedSequence {
    fn default() -> Self {
        Self {
            inner: AtomicI64::new(INITIAL_VALUE),
        }
    }
}
