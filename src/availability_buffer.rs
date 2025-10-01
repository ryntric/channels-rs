use crate::{constants, utils};
use std::mem::MaybeUninit;
use std::sync::atomic::{fence, AtomicI32, Ordering};

/// A buffer is used to track the availability of slots in a ring buffer.
///
/// # Overview
/// `AvailabilityBuffer` is typically used in high-performance
/// concurrent ring buffer implementations (like Disruptor-style designs),
/// where producers mark slots as available and consumers check which
/// slots are visible to them.
///
/// Internally, the buffer holds flags (`AtomicI32`) associated with each slot.
/// These flags are incremented in a way that allows detecting slot reuse
/// across wrap-around without explicit clearing.
///
/// # Concurrency
/// - Uses atomic operations with appropriate memory fences
///   to ensure visibility between producer and consumer threads.
/// - The `set` and `set_range` methods publish availability of sequences.
/// - The `get_available` method checks availability up to a given range.
///
/// # Memory layout
/// The buffer is over-allocated with extra padding (see `constants::ARRAY_PADDING`)
/// to reduce false sharing between cache lines.
///
/// # Safety
/// This struct implements `Send` and `Sync` manually, as it contains
/// atomics and padded memory regions that are safe to share across threads.
pub struct AvailabilityBuffer {
    /// Bitmask for wrapping sequence indices into the buffer length.
    mask: i64,
    /// Number of bits to shift when calculating availability flags.
    flag_shift: usize,
    /// Underlying buffer storing availability flags for each slot.
    /// Includes left and right padding to avoid false sharing.
    buffer: Box<[AtomicI32]>,
}

impl AvailabilityBuffer {
    /// Creates a new `AvailabilityBuffer` with the given size.
    ///
    /// # Arguments
    /// * `buffer_size` - Must be a power of two for wrapping to work correctly.
    ///
    /// # Panics
    /// May panic if `buffer_size` is not a power of two,
    /// depending on usage of `ilog2`.
    pub fn new(buffer_size: usize) -> Self {
        Self {
            mask: (buffer_size - 1) as i64,
            flag_shift: buffer_size.ilog2() as usize,
            buffer: Self::init_buffer(buffer_size),
        }
    }

    /// Initializes the underlying availability buffer with `-1` values,
    /// meaning "not yet available".
    ///
    /// Adds padding on both sides to avoid false sharing.
    fn init_buffer(size: usize) -> Box<[AtomicI32]> {
        let mut buffer: Box<[MaybeUninit<AtomicI32>]> = Box::new_uninit_slice(size + (constants::ARRAY_PADDING << 1));
        for i in 0..size {
            buffer[i + constants::ARRAY_PADDING].write(AtomicI32::new(-1));
        }
        unsafe { buffer.assume_init() }
    }

    /// Computes the availability flag for a given sequence.
    ///
    /// The flag is derived by shifting the sequence number.
    /// This allows detecting wrap-around reuse of slots.
    #[inline(always)]
    fn calculate_flag(&self, sequence: i64) -> i32 {
        (sequence >> self.flag_shift) as i32
    }

    /// Returns the highest available sequence in the given range `[low, high]`.
    ///
    /// Scans each sequence in the range and returns the last contiguous
    /// available index. If a gap is found, returns the last available before it.
    ///
    /// # Memory ordering
    /// Uses an `Acquire` fence to ensure that all prior stores from
    /// producers are visible before reading availability flags.
    pub fn get_available(&self, low: i64, high: i64) -> i64 {
        fence(Ordering::Acquire);
        for sequence in low..=high {
            let index = utils::wrap_index(sequence, self.mask, constants::ARRAY_PADDING);
            let flag = self.calculate_flag(sequence);
            let atomic = &self.buffer[index];
            if atomic.load(Ordering::Relaxed) != flag {
                return sequence - 1;
            }
        }
        high
    }

    /// Marks a single sequence as available.
    ///
    /// # Memory ordering
    /// Uses `Release` to ensure visibility of the write
    /// before consumers check availability.
    pub fn set(&self, sequence: i64) {
        let index = utils::wrap_index(sequence, self.mask, constants::ARRAY_PADDING);
        let flag = self.calculate_flag(sequence);
        let atomic = &self.buffer[index];
        atomic.store(flag, Ordering::Release);
    }

    /// Marks a range of sequences as available.
    ///
    /// # Memory ordering
    /// Stores each flag with `Relaxed`, followed by a `Release` fence
    /// to publish all updates together.
    pub fn set_range(&self, low: i64, high: i64) {
        for sequence in low..=high {
            let index = utils::wrap_index(sequence, self.mask, constants::ARRAY_PADDING);
            let flag = self.calculate_flag(sequence);
            let atomic = &self.buffer[index];
            atomic.store(flag, Ordering::Relaxed);
        }
        fence(Ordering::Release);
    }
}

unsafe impl Sync for AvailabilityBuffer {}

unsafe impl Send for AvailabilityBuffer {}