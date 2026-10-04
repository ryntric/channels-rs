use std::ops::Deref;
use std::sync::atomic::AtomicI64;

#[repr(align(64))]
pub struct Sequence {
    sequence: AtomicI64,
}

impl Deref for Sequence {
    type Target = AtomicI64;

    fn deref(&self) -> &Self::Target {
        &self.sequence
    }
}

impl Default for Sequence {
    fn default() -> Self {
        Self::new()
    }
}

impl Sequence {
    pub const INITIAL_VALUE: i64 = -1;
}

impl Sequence {
    pub const fn new() -> Self {
        Self {
            sequence: AtomicI64::new(Self::INITIAL_VALUE),
        }
    }
}
