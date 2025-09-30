use crate::ring_buffer::RingBuffer;
use crate::sequence::Sequence;
use crate::sequencer::Sequencer;

#[derive(Copy, Clone, Debug, PartialEq)]
pub enum State {
    Idle,
    Processing,
}

pub(crate) trait Poller<T>: Send + Sync {
    fn poll(&self, sequencer: &dyn Sequencer, buffer: &RingBuffer<T>, batch_size: i64, handler: &dyn Fn(T)) -> State;
}

pub(crate) struct SingleConsumerPoller {}

impl SingleConsumerPoller {
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

pub(crate) struct MultiConsumerPoller {
    sequence: Sequence,
}

impl MultiConsumerPoller {
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

unsafe impl Send for SingleConsumerPoller {}

unsafe impl Sync for SingleConsumerPoller {}

unsafe impl Send for MultiConsumerPoller {}

unsafe impl Sync for MultiConsumerPoller {}
