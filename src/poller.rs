use crate::ring_buffer::RingBuffer;
use crate::sequence::Sequence;
use crate::sequencer::Sequencer;

fn get_available(available: i64, batch_size: i64) -> i64 {
    if available > batch_size { batch_size } else { available }
}

#[derive(Copy, Clone, Debug, PartialEq)]
pub enum State {
    Idle,
    Processing,
}

pub enum PollerKind {
    SingleConsumer(SingleConsumerPoller), 
    MultiConsumer(MultiConsumerPoller),
}

impl PollerKind {
    pub fn poll<T>(&self, sequencer: &dyn Sequencer, buffer: &RingBuffer<T>, handler: &dyn Fn(T)) -> State {
        match self {
            PollerKind::SingleConsumer(poller) => poller.poll(sequencer, buffer, handler),
            PollerKind::MultiConsumer(poller) => poller.poll(sequencer, buffer, handler),
        }
    }
}

pub trait Poller<T>: Send + Sync {
    fn poll(&self, sequencer: &dyn Sequencer, buffer: &RingBuffer<T>, handler: &dyn Fn(T)) -> State;
}

pub struct SingleConsumerPoller {
    batch_size: i64,
}

impl SingleConsumerPoller {
    pub fn new(batch_size: i64) -> SingleConsumerPoller {
        Self { batch_size }
    }
}

impl<T> Poller<T> for SingleConsumerPoller {
    fn poll(&self, sequencer: &dyn Sequencer, buffer: &RingBuffer<T>, handler: &dyn Fn(T)) -> State {
        let current = sequencer.get_gating_sequence_relaxed();
        let next: i64 = current + 1;
        let available: i64 = get_available(sequencer.get_cursor_sequence_acquire(), current + self.batch_size);

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
    batch_size: i64,
}

impl MultiConsumerPoller {
    pub fn new(batch_size: i64) -> Self {
        Self {
            sequence: Sequence::default(),
            batch_size,
        }
    }
}

impl<T> Poller<T> for MultiConsumerPoller {
    fn poll(&self, sequencer: &dyn Sequencer, buffer: &RingBuffer<T>, handler: &dyn Fn(T)) -> State {
        let mut current: i64;
        let mut next: i64;
        let mut available: i64;
        let mut highest: i64;

        loop {
            current = self.sequence.get_acquire();
            next = current + 1;
            available = get_available(sequencer.get_cursor_sequence_acquire(), current + self.batch_size);

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
