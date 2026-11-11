use crate::cursor::LogicalRange;
use alloc::alloc::{alloc, dealloc, handle_alloc_error};
use core::alloc::Layout;
use core::any::type_name;
use core::marker::PhantomData;
use core::mem::MaybeUninit;
use core::ptr::NonNull;

#[derive(Copy, Clone, Debug)]
struct SliceRange {
    pub start: usize,
    pub first_len: usize,
    pub second_len: usize,
}

pub struct SlicePtr<'a, T> {
    ptr: *mut MaybeUninit<T>,
    len: usize,
    phantom: PhantomData<&'a mut MaybeUninit<T>>,
}

impl<'a, T> Iterator for SlicePtr<'a, T> {
    type Item = &'a mut MaybeUninit<T>;

    #[inline(always)]
    fn next(&mut self) -> Option<Self::Item> {
        if self.len == 0 {
            return None;
        }

        let ptr = self.ptr;

        self.ptr = unsafe { ptr.add(1) };
        self.len -= 1;

        Some(unsafe { &mut *ptr })
    }
}

pub struct RawParts<'a, T> {
    first: SlicePtr<'a, T>,
    second: SlicePtr<'a, T>,
}

impl<'a, T> Iterator for RawParts<'a, T> {
    type Item = &'a mut MaybeUninit<T>;

    #[inline(always)]
    fn next(&mut self) -> Option<Self::Item> {
        if self.first.len != 0 {
            return self.first.next();
        }
        self.second.next()
    }

    #[inline(always)]
    fn size_hint(&self) -> (usize, Option<usize>) {
        let len = self.first.len + self.second.len;
        (len, Some(len))
    }
}

impl<T> ExactSizeIterator for RawParts<'_, T> {}

impl<T> core::iter::FusedIterator for RawParts<'_, T> {}

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
    const fn wrap_sequence(&self, sequence: i64) -> usize {
        (sequence & self.mask) as usize
    }

    #[inline(always)]
    pub unsafe fn write(&self, sequence: i64, value: T) {
        let index = self.wrap_sequence(sequence);
        unsafe { (*self.ptr.as_ptr().add(index)).write(value) };
    }

    #[inline(always)]
    pub unsafe fn take(&self, sequence: i64) -> T {
        let index = self.wrap_sequence(sequence);
        unsafe { (*self.ptr.as_ptr().add(index)).assume_init_read() }
    }

    #[inline(always)]
    pub unsafe fn get_ref(&self, sequence: i64) -> &T {
        let index = self.wrap_sequence(sequence);
        unsafe { (*self.ptr.as_ptr().add(index)).assume_init_ref() }
    }

    #[inline(always)]
    pub unsafe fn get_ptr(&self, sequence: i64) -> *mut MaybeUninit<T> {
        let index = self.wrap_sequence(sequence);
        unsafe { self.ptr.as_ptr().add(index) }
    }

    #[inline(always)]
    fn slice_range(&self, range: &LogicalRange) -> SliceRange {
        let start = self.wrap_sequence(range.low());
        let capacity = self.capacity as usize;
        let length = range.len();

        let first_len = length.min(capacity - start);
        let second_len = length - first_len;

        SliceRange {
            start,
            first_len,
            second_len,
        }
    }

    #[inline(always)]
    pub unsafe fn raw_parts<'a>(&self, range: &LogicalRange) -> RawParts<'a, T> {
        let slice_range = self.slice_range(range);
        let ptr = self.ptr.as_ptr().cast::<MaybeUninit<T>>();
        let first = SlicePtr {
            ptr: unsafe { ptr.add(slice_range.start) },
            len: slice_range.first_len,
            phantom: Default::default(),
        };

        let second = SlicePtr {
            ptr,
            len: slice_range.second_len,
            phantom: Default::default(),
        };

        RawParts { first, second }
    }

    /// # Safety
    ///
    /// Every slot in the range must contain a valid T.
    /// Each slot must be visited at most once.
    /// No concurrent access or outstanding references are allowed.
    pub unsafe fn free(self, low: i64, high: i64) {
        for sequence in low..=high {
            let index = self.wrap_sequence(sequence);
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
