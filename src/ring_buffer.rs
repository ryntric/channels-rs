use crate::coordinator::Coordinator;
use crate::poller::{Poller, State};
use crate::sequencer::Sequencer;
use crate::{constants, utils};
use std::cell::UnsafeCell;
use std::mem::MaybeUninit;
use std::ptr;

/// A high-performance ring buffer for concurrent producers and consumers.
///
/// `RingBuffer<T>` stores elements in a pre-allocated, fixed-size array with
/// cache-line padding to reduce false sharing. It supports both **single**
/// and **multi-consumer** pollers via a [`Poller<T>`] trait and coordinates
/// access through a [`Sequencer`] and [`Coordinator`].
///
/// # Safety
/// Internally uses [`UnsafeCell`] and [`MaybeUninit`] to perform lock-free reads and writes.
pub(crate) struct RingBuffer<T: Send> {
    buffer: Box<[UnsafeCell<MaybeUninit<T>>]>,
    sequencer: Box<dyn Sequencer>,
    poller: Box<dyn Poller<T>>,
    mask: i64,
    buffer_size: usize,
}

impl<T: Send> RingBuffer<T> {
    /// Create a new ring buffer with the specified size, sequencer, and poller.
    ///
    /// # Parameters
    /// - `buffer_size`: number of elements in the buffer (must be power of two for mask).
    /// - `sequencer`: manages sequences for producer/consumer coordination.
    /// - `poller`: manages of polling of items from this buffer.
    ///
    /// # Returns
    /// A new `RingBuffer<T>` instance ready for push and poll operations.
    pub fn new(
        buffer_size: usize,
        sequencer: Box<dyn Sequencer>,
        poller: Box<dyn Poller<T>>,
    ) -> RingBuffer<T> {
        RingBuffer {
            buffer: Self::create_buffer(buffer_size),
            sequencer,
            poller,
            mask: (buffer_size - 1) as i64,
            buffer_size,
        }
    }

    /// Allocate the underlying buffer with cache-line padding.
    fn create_buffer(buffer_size: usize) -> Box<[UnsafeCell<MaybeUninit<T>>]> {
        (0..buffer_size + (constants::ARRAY_PADDING << 1))
            .map(|_| UnsafeCell::new(MaybeUninit::uninit()))
            .collect::<Vec<_>>()
            .into_boxed_slice()
    }

    /// Check that a requested batch size does not exceed the buffer capacity.
    #[inline(always)]
    fn assert_that_size_is_valid(&self, size: usize) {
        if size > self.buffer_size || size == 0 {
            panic!("size should be between 1 and {}", self.buffer_size);
        }
    }

    /// Dequeue an element from the buffer by sequence number.
    ///
    /// # Safety
    /// Performs an unchecked read from the internal `UnsafeCell`. Ensure that
    /// the element at `sequence` has been properly initialized via `push` before calling.
    /// This method is only called by `Poller`. If the buffer has no available data to consume, the 'Poller' will wait for it.
    pub(crate) fn dequeue(&self, sequence: i64) -> T {
        let index: usize = utils::wrap_index(sequence, self.mask, constants::ARRAY_PADDING);
        let cell = &self.buffer[index];

        // SAFETY:
        // An item is only moved once, and it is managed and guaranteed by the sequencer.
        unsafe { ptr::read((*cell.get()).as_ptr()) }
    }

    /// Writes an element into the buffer at the position derived from the given `sequence`.
    ///
    /// The sequence number is first transformed into an array index using
    /// [`utils::wrap_index`], taking into account the ring buffer's mask and
    /// padding. The resulting index is then used to locate the corresponding
    /// buffer cell, and the provided element is written directly into it.
    ///
    /// # Safety
    ///
    /// This method uses [`UnsafeCell::get`] and [`MaybeUninit::write`] internally,
    /// which allows writing into the memory location without runtime checks.
    /// It assumes that:
    /// - and that reads/writes follow the ring buffer’s concurrency protocol
    ///   to avoid data races or uninitialized access.
    ///
    /// # Parameters
    ///
    /// - `sequence`: The monotonically increasing sequence number identifying
    ///   the logical slot in the ring buffer.
    /// - `element`: The element to be stored in the buffer at that slot.
    ///
    #[inline(always)]
    fn write(&self, sequence: i64, element: T) {
        let index = utils::wrap_index(sequence, self.mask, constants::ARRAY_PADDING);
        let cell = &self.buffer[index];

        // SAFETY:
        // The item may not be overwritten if it was not consumed and it is managed and guaranteed by the sequencer.
        unsafe {
            (*cell.get()).write(element);
        }
    }

    /// Poll up to `buffer.capaticy()` if there is enough elements or drains rest of elements from `ring_buffer`.
    ///
    /// Returns [`State::Idle`] if no elements are available, or [`State::Processing`] if
    /// one or more items were consumed.
    ///
    /// # Panics
    /// If the buffer is not empty
    pub fn poll(&self, buffer: &mut Vec<T>) -> State {
        let length = buffer.capacity();
        assert!(buffer.is_empty(), "buffer should be empty");
        self.poller
            .poll(&*self.sequencer, self, length as i64, buffer)
    }

    /// Push a single element into the ring buffer.
    ///
    /// Blocks or spins according to the `Coordinator` if necessary.
    ///
    /// # Safety
    /// If there is no available space the producer will wait for it until it became available
    pub fn push(&self, element: T, coordinator: &Coordinator) {
        let sequence = self.sequencer.next(coordinator);
        self.write(sequence, element);
        self.sequencer.publish_cursor_sequence(sequence);
    }

    pub fn push_v(&self, items: Vec<T>, coordinator: &Coordinator) {
        let length = items.len();
        self.assert_that_size_is_valid(length);

        let high = self.sequencer.next_n(length, coordinator);
        let low = high - (length - 1) as i64;

        for (sequence, element) in (low..=high).zip(items) {
            self.write(sequence, element);
        }

        self.sequencer.publish_cursor_sequence_range(low, high);
    }

    pub fn push_a<const N: usize>(&self, items: [T; N], coordinator: &Coordinator) {
        let length = items.len();
        self.assert_that_size_is_valid(length);

        let high = self.sequencer.next_n(length, coordinator);
        let low = high - (length - 1) as i64;

        for (sequence, element) in (low..=high).zip(items) {
            self.write(sequence, element);
        }

        self.sequencer.publish_cursor_sequence_range(low, high);
    }
}

unsafe impl<T: Send> Sync for RingBuffer<T> {}
