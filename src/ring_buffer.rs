use crate::buffer::{Buffer, RawParts};
use crate::cursor::LogicalRange;
use core::mem::MaybeUninit;

pub(crate) struct RingBuffer<T> {
    inner: Buffer<T>,
}

impl<T> RingBuffer<T> {
    pub fn new(capacity: usize) -> Self {
        Self {
            inner: Buffer::new(capacity),
        }
    }

    #[inline(always)]
    pub const fn capacity(&self) -> i64 {
        self.inner.capacity()
    }

    #[inline(always)]
    pub unsafe fn write(&self, sequence: i64, value: T) {
        unsafe { self.inner.write(sequence, value) }
    }

    #[inline(always)]
    pub unsafe fn take(&self, sequence: i64) -> T {
        unsafe { self.inner.take(sequence) }
    }

    #[inline(always)]
    pub unsafe fn get_ptr(&self, sequence: i64) -> *mut MaybeUninit<T> {
        unsafe { self.inner.get_ptr(sequence) }
    }

    #[inline(always)]
    pub unsafe fn raw_parts<'a>(&self, range: &LogicalRange) -> RawParts<'a, T> {
        unsafe { self.inner.raw_parts(range) }
    }
}
