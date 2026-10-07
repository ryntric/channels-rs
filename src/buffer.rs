use std::alloc::{Layout, alloc, handle_alloc_error};
use std::any::type_name;
use std::mem::MaybeUninit;
use std::ptr::NonNull;

pub(crate) struct Buffer<T> {
    ptr: NonNull<MaybeUninit<T>>,
    layout: Layout,
    mask: i64,
    capacity: i64,
}

impl<T> Buffer<T> {
    pub fn new(capacity: usize) -> Self {
        assert!(
            capacity.is_power_of_two(),
            "Buffer capacity must be greater than 0 and must be pow of 2"
        );
        assert!(
            i64::try_from(capacity).is_ok(),
            "Buffer max capacity is {}",
            i64::MAX
        );

        let layout = Layout::array::<MaybeUninit<T>>(capacity).unwrap_or_else(|_| {
            panic!(
                "Couldn't create a layout for {}",
                type_name::<MaybeUninit<T>>()
            )
        });

        let ptr = if layout.size() == 0 {
            NonNull::dangling()
        } else {
            // SAFETY: layout has a non-zero size.
            let raw = unsafe { alloc(layout) }.cast::<MaybeUninit<T>>();
            NonNull::new(raw).unwrap_or_else(|| handle_alloc_error(layout))
        };

        Self {
            ptr,
            capacity: capacity as i64,
            layout,
            mask: (capacity - 1) as i64,
        }
    }

    pub fn with_default(capacity: usize, func: impl Fn() -> T) -> Self {
        let buffer = Self::new(capacity);
        for index in 0..buffer.capacity {
            unsafe { buffer.write(index, func()) }
        }
        buffer
    }

    pub fn capacity(&self) -> i64 {
        self.capacity
    }

    #[inline(always)]
    fn wrap_index(&self, sequence: i64) -> usize {
        (sequence & self.mask) as usize
    }

    pub unsafe fn take(&self, sequence: i64) -> T {
        let index = self.wrap_index(sequence);
        unsafe { (*self.ptr.as_ptr().add(index)).assume_init_read() }
    }

    pub unsafe fn write(&self, sequence: i64, value: T) {
        let index = self.wrap_index(sequence);
        unsafe { self.ptr.as_ptr().add(index).write(MaybeUninit::new(value)) }
    }

    pub unsafe fn get(&self, sequence: i64) -> &T {
        let index = self.wrap_index(sequence);
        unsafe { (*self.ptr.as_ptr().add(index)).assume_init_ref() }
    }

    pub unsafe fn clear(&self, low: i64, high: i64) {
        for sequence in low..=high {
            let value = unsafe { self.take(sequence) };
            drop(value);
        }
    }
}

impl<T> Drop for Buffer<T> {
    fn drop(&mut self) {
        if self.layout.size() != 0 {
            unsafe { std::alloc::dealloc(self.ptr.as_ptr().cast(), self.layout) }
        }
    }
}

unsafe impl<T> Sync for Buffer<T> {}

unsafe impl<T: Send> Send for Buffer<T> {}
