use crate::availability_buffer::AvailabilityBuffer;
use crate::sequence::Sequence;
use crate::utils;
use crate::coordinator::Coordinator;

/// Trait defining a sequencer for coordinating producers and consumers in a ring buffer.
///
/// A `Sequencer` tracks available sequences, gating sequences, and cursor positions.
/// It supports both single-producer and multi-producer strategies, providing methods
/// for claiming sequences, publishing cursor progress, and waiting for consumers.
pub trait Sequencer: Sync + Send {
    /// Claim the next sequence for a producer.
    fn next(&self, strategy: &Coordinator) -> i64 {
        self.next_n(1, strategy)
    }

    /// Claim the next `n` sequences for batch production.
    fn next_n(&self, n: usize, strategy: &Coordinator) -> i64;

    /// Publish a sequence to indicate it is ready for consumption.
    fn publish_cursor_sequence(&self, sequence: i64);

    /// Publish a range of sequences for batch production.
    fn publish_cursor_sequence_range(&self, low: i64, high: i64);

    /// Update the gating sequence to indicate the consumer's progress.
    fn publish_gating_sequence(&self, sequence: i64);

    /// Determine the highest available sequence in a range for consumers.
    fn get_highest(&self, low: i64, high: i64) -> i64;

    /// Get the current cursor sequence with Acquire ordering.
    fn get_cursor_sequence_acquire(&self) -> i64;

    /// Get the current gating sequence with Relaxed ordering.
    fn get_gating_sequence_relaxed(&self) -> i64;

    /// Wait until the consumer has processed sequences below `wrap_point`.
    ///
    /// Uses the provided `Coordinator` to apply the producer wait strategy.
    #[inline(always)]
    fn wait(&self, gating_sequence: &Sequence, wrap_point: i64, coordinator: &Coordinator) -> i64 {
        let mut gating: i64;
        loop {
            gating = gating_sequence.get_acquire();
            if wrap_point > gating {
                coordinator.producer_wait();
                continue;
            }
            return gating;
        }
    }
}

/// Sequencer for a **single producer** scenario.
///
/// Uses a local cursor and gating sequences to coordinate with consumers.
pub struct SingleProducerSequencer {
    sequence: Sequence,
    cached: Sequence,
    buffer_size: i64,
    cursor_sequence: Sequence,
    gating_sequence: Sequence,
}

impl SingleProducerSequencer {
    /// Create a new single-producer sequencer with the specified buffer size.
    pub fn new(buffer_size: usize) -> Self {
        Self {
            sequence: Sequence::default(),
            cached: Sequence::default(),
            buffer_size: utils::assert_buffer_size_pow_of_2(buffer_size) as i64,
            cursor_sequence: Sequence::default(),
            gating_sequence: Sequence::default(),
        }
    }
}

impl Sequencer for SingleProducerSequencer {
    fn next_n(&self, n: usize, coordinator: &Coordinator) -> i64 {
        let next: i64 = self.sequence.get_relaxed() + n as i64;
        let wrap_point: i64 = next - self.buffer_size;

        if wrap_point > self.cached.get_relaxed() {
            self.cached.set_relaxed(self.wait(&self.gating_sequence, wrap_point, coordinator));
        }

        self.sequence.set_relaxed(next);
        next
    }

    fn publish_cursor_sequence(&self, sequence: i64) {
        self.cursor_sequence.set_release(sequence);
    }

    fn publish_cursor_sequence_range(&self, _: i64, high: i64) {
        self.cursor_sequence.set_release(high)
    }

    fn publish_gating_sequence(&self, sequence: i64) {
        self.gating_sequence.set_release(sequence);
    }

    fn get_highest(&self, _: i64, high: i64) -> i64 {
        high
    }

    fn get_cursor_sequence_acquire(&self) -> i64 {
        self.cursor_sequence.get_acquire()
    }

    fn get_gating_sequence_relaxed(&self) -> i64 {
        self.gating_sequence.get_relaxed()
    }

}

/// Sequencer for **multiple producers** scenario.
///
/// Coordinates multiple producers using an availability buffer to safely
/// publish sequences without overwriting each other's data.
pub struct MultiProducerSequencer {
    buffer_size: i64,
    cached: Sequence,
    cursor_sequence: Sequence,
    gating_sequence: Sequence,
    availability_buffer: AvailabilityBuffer,
}

impl MultiProducerSequencer {
    /// Create a new multi-producer sequencer with the specified buffer size.
    pub fn new(buffer_size: usize) -> Self {
        Self {
            buffer_size: utils::assert_buffer_size_pow_of_2(buffer_size) as i64,
            cached: Sequence::default(),
            cursor_sequence: Sequence::default(),
            gating_sequence: Sequence::default(),
            availability_buffer: AvailabilityBuffer::new(buffer_size),
        }
    }
}

impl Sequencer for MultiProducerSequencer {
    fn next_n(&self, n: usize, coordinator: &Coordinator) -> i64 {
        let n: i64 = n as i64;
        let next: i64 = self.cursor_sequence.fetch_add_volatile(n) + n;
        let wrap_point: i64 = next - self.buffer_size;

        if wrap_point > self.cached.get_relaxed() {
            self.cached.set_relaxed(self.wait(&self.gating_sequence, wrap_point, coordinator));
        }

        next
    }

    fn publish_cursor_sequence(&self, sequence: i64) {
        self.availability_buffer.set(sequence);
    }

    fn publish_cursor_sequence_range(&self, low: i64, high: i64) {
        self.availability_buffer.set_range(low, high);
    }

    fn publish_gating_sequence(&self, sequence: i64) {
        self.gating_sequence.set_release(sequence);
    }

    fn get_highest(&self, low: i64, high: i64) -> i64 {
        self.availability_buffer.get_available(low, high)
    }

    fn get_cursor_sequence_acquire(&self) -> i64 {
        self.cursor_sequence.get_acquire()
    }

    fn get_gating_sequence_relaxed(&self) -> i64 {
        self.gating_sequence.get_relaxed()
    }

}

// SAFETY: Sequencers are thread-safe because all internal state modifications
// are performed via atomic operations and coordinated with availability buffers.
unsafe impl Send for SingleProducerSequencer {}

unsafe impl Sync for SingleProducerSequencer {}

unsafe impl Send for MultiProducerSequencer {}

unsafe impl Sync for MultiProducerSequencer {}
