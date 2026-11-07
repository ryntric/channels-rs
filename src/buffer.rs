use alloc::alloc::{alloc, dealloc, handle_alloc_error};
use core::alloc::Layout;
use core::any::type_name;
use core::mem::MaybeUninit;
use core::ptr::NonNull;

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
            // SAFETY: Each index is within the allocated capacity.
            // Every slot is written exactly once and is initially uninitialized.
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

    /// Moves the initialized value out of the specified slot.
    ///
    /// # Safety
    ///
    /// - The slot must contain a fully initialized value of type `T`.
    /// - The caller must have exclusive ownership of the slot.
    /// - No other thread may concurrently access or modify the slot.
    /// - The value must not be read, moved, or dropped again until
    ///   the slot has been reinitialized.
    pub unsafe fn take(&self, sequence: i64) -> T {
        let index = self.wrap_index(sequence);

        // SAFETY: The caller guarantees that the slot is initialized,
        // exclusively owned, and not accessed concurrently.
        unsafe { (*self.ptr.as_ptr().add(index)).assume_init_read() }
    }

    /// Writes a value into the specified slot.
    ///
    /// # Safety
    ///
    /// - The target slot must be logically uninitialized.
    /// - The caller must have exclusive ownership of the slot.
    /// - No other thread may concurrently access or modify the slot.
    /// - Any previously initialized value must have been moved out
    ///   or properly dropped before this operation.
    pub unsafe fn write(&self, sequence: i64, value: T) {
        let index = self.wrap_index(sequence);

        // SAFETY: The caller guarantees exclusive access to the slot.
        // The index is within bounds, and the slot is logically uninitialized.
        unsafe { self.ptr.as_ptr().add(index).write(MaybeUninit::new(value)) }
    }

    /// Returns a shared reference to the initialized value in the slot.
    ///
    /// # Safety
    ///
    /// - The slot must contain a fully initialized value of type `T`.
    /// - The value must remain initialized and unchanged for the
    ///   entire lifetime of the returned reference.
    /// - No mutable access, move, or drop may occur while the
    ///   returned reference is alive.
    /// - Concurrent access must satisfy the thread-safety
    ///   requirements of `T`.
    pub unsafe fn get(&self, sequence: i64) -> &T {
        let index = self.wrap_index(sequence);

        // SAFETY: The caller guarantees that the slot is initialized
        // and remains valid and immutably accessible for the
        // lifetime of the returned reference.
        unsafe { (*self.ptr.as_ptr().add(index)).assume_init_ref() }
    }

    /// Drops all initialized values in the inclusive range `[low, high]`.
    ///
    /// # Safety
    ///
    /// - Every slot in the range must contain a fully initialized `T`.
    /// - The range must not contain the same physical slot more than once.
    /// - The caller must have exclusive ownership of every affected slot.
    /// - No other thread may concurrently access or modify these slots.
    /// - The values must not be accessed or dropped again until
    ///   their slots have been reinitialized.
    /// - The caller must ensure that `low <= high` and that the
    ///   range contains no more than `capacity` elements.
    pub unsafe fn free(&self, low: i64, high: i64) {
        for sequence in low..=high {
            let index = self.wrap_index(sequence);

            // SAFETY: The caller guarantees that each slot is
            // initialized, exclusively owned, and dropped only once.
            unsafe {
                (*self.ptr.as_ptr().add(index)).assume_init_drop();
            }
        }
    }
}

impl<T> Drop for Buffer<T> {
    fn drop(&mut self) {
        if self.layout.size() > 0 {
            // SAFETY: `ptr` was allocated using `alloc` with exactly
            // this layout and has not been deallocated previously.
            // No references into the allocation may remain alive.
            unsafe { dealloc(self.ptr.as_ptr().cast(), self.layout) }
        }
    }
}

unsafe impl<T> Sync for Buffer<T> {}

unsafe impl<T: Send> Send for Buffer<T> {}
