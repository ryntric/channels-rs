use alloc::boxed::Box;
use core::cell::UnsafeCell;
use core::iter;

pub(crate) struct Buffer<T> {
    buf: Box<[UnsafeCell<T>]>,
    mask: i64,
    capacity: i64,
}

impl<T> Buffer<T> {
    pub fn with_initializer<F>(capacity: usize, initializer: F) -> Self
    where
        F: Fn() -> T,
    {
        assert!(
            capacity.is_power_of_two(),
            "Buffer capacity must be greater than 0 and must be pow of 2"
        );
        assert!(
            i64::try_from(capacity).is_ok(),
            "Buffer max capacity is {}",
            i64::MAX
        );

        let buf = iter::repeat_with(|| UnsafeCell::new(initializer()))
            .take(capacity)
            .collect::<Box<[_]>>();
        let capacity = capacity as i64;
        let mask = capacity - 1;

        Self {
            capacity,
            mask,
            buf,
        }
    }

    pub fn capacity(&self) -> i64 {
        self.capacity
    }

    #[inline(always)]
    fn wrap_index(&self, sequence: i64) -> usize {
        (sequence & self.mask) as usize
    }

    #[inline]
    pub unsafe fn write(&self, sequence: i64, value: T) {
        let index = self.wrap_index(sequence);
        let cell = unsafe { self.buf.get_unchecked(index) };
        unsafe { cell.get().write(value) }
    }

    #[inline]
    pub unsafe fn with_ref<F, R>(&self, sequence: i64, f: F) -> R
    where
        F: Fn(&T) -> R,
    {
        let index = self.wrap_index(sequence);
        let cell = unsafe { self.buf.get_unchecked(index) };
        let value = unsafe { &*cell.get() };
        f(value)
    }
}

impl<T: Default> Buffer<T> {
    pub fn with_default(capacity: usize) -> Self {
        Self::with_initializer(capacity, T::default)
    }

    #[inline]
    pub unsafe fn take(&self, sequence: i64) -> T {
        let index = self.wrap_index(sequence);
        let cell = unsafe { self.buf.get_unchecked(index) };
        unsafe { core::mem::take(&mut *cell.get()) }
    }
}

// SAFETY:
// Slot ownership and synchronization are managed by the sequencer.
// Concurrent non-atomic access to the same slot is forbidden.
unsafe impl<T: Send> Sync for Buffer<T> {}
