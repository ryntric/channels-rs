use crate::ring_buffer::RingBuffer;
use crate::sequence::Sequence;
use crate::sequencer::Sequencer;

/// Represents the current state of a consumer poll operation.
#[derive(Copy, Clone, Debug, PartialEq)]
pub(crate) enum State {
    /// No items were available to process.
    Idle,
    /// One or more items were processed.
    Processing,
}

/// Trait defining a poller for a ring buffer.
///
/// A poller is responsible for consuming items from a [`RingBuffer`]
/// according to the rules of a sequencer. It allows both single and
/// multi-consumer implementations.
pub(crate) trait Poller<T>: Send + Sync {
    /// Poll up to `batch_size` items from the ring buffer.
    ///
    /// # Parameters
    /// - `sequencer`: Tracks available and consumed sequences.
    /// - `buffer`: The underlying ring buffer to consume from.
    /// - `batch_size`: Maximum number of items to consume in this poll.
    /// - `handler`: Closure called for each consumed item.
    ///
    /// # Returns
    /// - [`State::Idle`] if no items were available.
    /// - [`State::Processing`] if one or more items were consumed.
    fn poll(&self, sequencer: &dyn Sequencer, buffer: &RingBuffer<T>, batch_size: i64, handler: &dyn Fn(T)) -> State;
}

/// Single-consumer poller.
///
/// Designed for scenarios where only one consumer thread processes the buffer.
pub(crate) struct SingleConsumerPoller {}

impl SingleConsumerPoller {
    /// Create a new single-consumer poller.
    pub fn new() -> Self {
        Self {}
    }
}

impl<T> Poller<T> for SingleConsumerPoller {
    fn poll(&self, sequencer: &dyn Sequencer, buffer: &RingBuffer<T>, batch_size: i64, handler: &dyn Fn(T)) -> State {
        let current = sequencer.get_gating_sequence_relaxed();
        let next: i64 = current + 1;
        let available: i64 = std::cmp::min(sequencer.get_cursor_sequence_acquire(), current + batch_size);

        if next > available {
            return State::Idle;
        }

        let highest: i64 = sequencer.get_highest(next, available);
        for sequence in next..=highest {
            handler(buffer.dequeue(sequence));
        }

        sequencer.publish_gating_sequence(highest);
        State::Processing
    }
}

/// Multi-consumer poller.
///
/// Supports multiple consumers consuming concurrently from a single buffer.
/// Uses a local [`Sequence`] to claim ranges of items safely.
pub(crate) struct MultiConsumerPoller {
    sequence: Sequence,
}

impl MultiConsumerPoller {
    /// Create a new multi-consumer poller.
    pub fn new() -> Self {
        Self {
            sequence: Sequence::default(),
        }
    }
}

impl<T> Poller<T> for MultiConsumerPoller {
    fn poll(&self, sequencer: &dyn Sequencer, buffer: &RingBuffer<T>, batch_size: i64, handler: &dyn Fn(T)) -> State {
        let mut current: i64;
        let mut next: i64;
        let mut available: i64;
        let mut highest: i64;

        loop {
            current = self.sequence.get_acquire();
            next = current + 1;
            available = std::cmp::min(sequencer.get_cursor_sequence_acquire(), current + batch_size);

            if next > available {
                return State::Idle;
            }

            highest = sequencer.get_highest(next, available);
            if self.sequence.compare_and_exchange_weak_volatile(current, highest) {
                break;
            }
        }

        for sequence in next..=highest {
            handler(buffer.dequeue(sequence));
        }

        sequencer.publish_gating_sequence(highest);
        State::Processing
    }
}

// SAFETY: SingleConsumerPoller and MultiConsumerPoller are thread-safe as designed.
unsafe impl Send for SingleConsumerPoller {}

unsafe impl Sync for SingleConsumerPoller {}

unsafe impl Send for MultiConsumerPoller {}

unsafe impl Sync for MultiConsumerPoller {}
