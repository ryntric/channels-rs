use crate::cursor::LogicalRange;
use alloc::alloc::{alloc, dealloc, handle_alloc_error};
use core::alloc::Layout;
use core::any::type_name;
use core::marker::PhantomData;
use core::mem::MaybeUninit;
use core::ptr::NonNull;
use core::slice::IterMut;
use std::slice;

pub struct RawParts<'a, T> {
    first: IterMut<'a, MaybeUninit<T>>,
    second: IterMut<'a, MaybeUninit<T>>,
}

impl<'a, T> RawParts<'a, T> {
    pub fn new(first: &'a mut [MaybeUninit<T>], second: &'a mut [MaybeUninit<T>]) -> Self {
        Self {
            first: first.iter_mut(),
            second: second.iter_mut(),
        }
    }
}

impl<'a, T> Iterator for RawParts<'a, T> {
    type Item = &'a mut MaybeUninit<T>;

    #[inline(always)]
    fn next(&mut self) -> Option<Self::Item> {
        self.first.next().or_else(|| self.second.next())
    }
}
pub(crate) struct Buffer<T> {
    ptr: NonNull<MaybeUninit<T>>,
    layout: Layout,
    mask: i64,
    capacity: i64,
    phantom: PhantomData<T>,
}

impl<T> Buffer<T> {
    fn check_capacity(capacity: usize) {
        assert!(
            capacity.is_power_of_two(),
            "Buffer capacity must be greater than 0 and must be power of 2"
        );
        assert!(
            i64::try_from(capacity).is_ok(),
            "Buffer max capacity is {}",
            i64::MAX
        );
    }

    pub fn new(capacity: usize) -> Self {
        Self::check_capacity(capacity);

        let layout = Layout::array::<MaybeUninit<T>>(capacity).unwrap_or_else(|_| {
            panic!(
                "Couldn't create layout for {}",
                type_name::<MaybeUninit<T>>()
            )
        });
        let ptr = if layout.size() == 0 {
            NonNull::dangling()
        } else {
            NonNull::new(unsafe { alloc(layout).cast::<MaybeUninit<T>>() })
                .unwrap_or_else(|| handle_alloc_error(layout))
        };

        let capacity = capacity as i64;
        let mask = capacity - 1;
        let phantom = PhantomData;

        Self {
            ptr,
            layout,
            mask,
            capacity,
            phantom,
        }
    }

    #[inline(always)]
    pub const fn capacity(&self) -> i64 {
        self.capacity
    }

    #[inline(always)]
    const fn wrap_index(&self, sequence: i64) -> usize {
        (sequence & self.mask) as usize
    }

    #[inline(always)]
    pub unsafe fn write(&self, sequence: i64, value: T) {
        let index = self.wrap_index(sequence);
        unsafe { (*self.ptr.as_ptr().add(index)).write(value) };
    }

    #[inline(always)]
    pub unsafe fn take(&self, sequence: i64) -> T {
        let index = self.wrap_index(sequence);
        unsafe { (*self.ptr.as_ptr().add(index)).assume_init_read() }
    }

    #[inline(always)]
    pub unsafe fn get_ref(&self, sequence: i64) -> &T {
        let index = self.wrap_index(sequence);
        unsafe { (*self.ptr.as_ptr().add(index)).assume_init_ref() }
    }

    #[inline(always)]
    pub unsafe fn get_ptr(&self, sequence: i64) -> *mut MaybeUninit<T> {
        let index = self.wrap_index(sequence);
        unsafe { self.ptr.as_ptr().add(index) }
    }

    #[inline(always)]
    pub unsafe fn raw_parts(&self, range: &LogicalRange) -> RawParts<'_, T> {
        let start = self.wrap_index(range.low());
        let capacity = self.capacity as usize;
        let length = range.len();

        let first = length.min(capacity - start);
        let second = length - first;

        let ptr = self.ptr.as_ptr().cast::<MaybeUninit<T>>();

        let first = unsafe { slice::from_raw_parts_mut(ptr.add(start), first) };
        let second = unsafe { slice::from_raw_parts_mut(ptr, second) };

        RawParts::new(first, second)
    }

    /// # Safety
    ///
    /// Every slot in the range must contain a valid T.
    /// Each slot must be visited at most once.
    /// No concurrent access or outstanding references are allowed.
    pub unsafe fn free(self, low: i64, high: i64) {
        for sequence in low..=high {
            let index = self.wrap_index(sequence);
            unsafe { (*self.ptr.as_ptr().add(index)).assume_init_drop() }
        }
    }
}

impl<T> Drop for Buffer<T> {
    fn drop(&mut self) {
        if self.layout.size() != 0 {
            unsafe { dealloc(self.ptr.as_ptr().cast::<u8>(), self.layout) }
        }
    }
}

// SAFETY:
// Slot ownership and synchronization are managed by the sequencer.
// Concurrent non-atomic access to the same slot is forbidden.
unsafe impl<T: Send> Sync for Buffer<T> {}

// SAFETY:
// Slot ownership and synchronization are managed by the sequencer.
// Concurrent non-atomic access to the same slot is forbidden.
unsafe impl<T: Send> Send for Buffer<T> {}
