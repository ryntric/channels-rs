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
pub(crate) struct RingBuffer<T> {
    buffer: Box<[UnsafeCell<MaybeUninit<T>>]>,
    sequencer: Box<dyn Sequencer>,
    poller: Box<dyn Poller<T>>,
    mask: i64,
    buffer_size: usize,
}

impl<T> RingBuffer<T> {
    /// Create a new ring buffer with the specified size, sequencer, and poller.
    ///
    /// # Parameters
    /// - `buffer_size`: number of elements in the buffer (must be power of two for mask).
    /// - `sequencer`: manages sequences for producer/consumer coordination.
    /// - `poller`: manages of polling of items from this buffer.
    ///
    /// # Returns
    /// A new `RingBuffer<T>` instance ready for push and poll operations.
    pub fn new(buffer_size: usize, sequencer: Box<dyn Sequencer>, poller: Box<dyn Poller<T>>) -> RingBuffer<T> {
        RingBuffer {
            buffer: Self::create_buffer(buffer_size),
            sequencer: sequencer,
            poller: poller,
            mask: (buffer_size - 1) as i64,
            buffer_size: buffer_size
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
    fn check_size(&self, size: usize) {
        if size > self.buffer_size {
            std::panic::panic_any("size is greater than buffer size");
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
        unsafe { (*cell.get()).write(element); }
    }

    /// Poll up to `batch_size` elements and process them with the provided handler.
    ///
    /// Returns [`State::Idle`] if no elements are available, or [`State::Processing`] if
    /// one or more items were consumed.
    ///
    /// # Panics
    // If the batch size is greater than buffer size it will panic
    pub fn poll<H: Fn(T)>(&self, batch_size: usize, handler: &H) -> State {
        self.check_size(batch_size);
        self.poller.poll(&*self.sequencer, &self, batch_size as i64, &handler)
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

    /// Push multiple elements into the ring buffer in a batch.
    ///
    /// More efficient than calling `push` repeatedly, reducing sequencer overhead.
    ///
    /// # Parameters
    /// - `items`: iterable of elements to push (must implement `ExactSizeIterator`).
    /// - `coordinator`: coordinates waiting if buffer space is not available.
    ///
    ///# Safety
    /// If there is no available space the producer will wait for it until it became available
    ///
    /// # Panics
    /// If items size is greater than buffer size it will panic
    pub fn push_n<I>(&self, items: I, coordinator: &Coordinator)
    where
        I: IntoIterator<Item=T>,
        I::IntoIter: ExactSizeIterator,
    {
        let iterator = items.into_iter();
        let length = iterator.len();
        self.check_size(length);
        let high = self.sequencer.next_n(length, coordinator);
        let low = high - (length - 1) as i64;

        for (index, item) in iterator.enumerate() {
            self.write(index as i64 + low, item);
        }

        self.sequencer.publish_cursor_sequence_range(low, high);
    }
}

// SAFETY: `RingBuffer` is safe to share between threads because all internal mutability
// is handled with `UnsafeCell` and sequencer coordination ensures proper synchronization.
unsafe impl<T> Sync for RingBuffer<T> {}

unsafe impl<T> Send for RingBuffer<T> {}
