use crate::buffer::Buffer;
use crate::poller::State::Idle;
use crate::sequence::Sequence;
use crate::sequencer::Sequencer;

#[derive(Copy, Clone, Debug, PartialEq)]
pub(crate) enum State {
    Idle,
    Processing,
}

pub(crate) trait Poller<T: Send, S: Sequencer>: Sized {
    fn poll(&self, sequencer: &S, buffer: &Buffer<T>, vector: &mut Vec<T>) -> State;
}

#[derive(Default)]
pub(crate) struct SingleConsumerPoller {}

impl<T: Send, S: Sequencer> Poller<T, S> for SingleConsumerPoller {
    fn poll(&self, sequencer: &S, buffer: &Buffer<T>, sink: &mut Vec<T>) -> State {
        let limit = i64::try_from(sink.capacity()).unwrap_or(i64::MAX);
        let current = sequencer.get_gating_sequence_relaxed();
        let next: i64 = current + 1;
        let available: i64 =
            std::cmp::min(sequencer.get_cursor_sequence_acquire(), current + limit);

        if next > available {
            return Idle;
        }

        let highest: i64 = sequencer.get_highest(next, available);
        if next > highest {
            return Idle;
        }

        for sequence in next..=highest {
            unsafe { sink.push(buffer.take(sequence)) }
        }

        sequencer.publish_gating_sequence(highest);
        State::Processing
    }
}

#[derive(Default)]
pub(crate) struct MultiConsumerPoller {
    sequence: Sequence,
}

impl<T: Send, S: Sequencer> Poller<T, S> for MultiConsumerPoller {
    fn poll(&self, sequencer: &S, buffer: &Buffer<T>, sink: &mut Vec<T>) -> State {
        let limit = i64::try_from(sink.capacity()).unwrap_or(i64::MAX);

        let mut current: i64;
        let mut next: i64;
        let mut available: i64;
        let mut highest: i64;

        loop {
            current = self.sequence.get_acquire();
            next = current + 1;
            available = std::cmp::min(sequencer.get_cursor_sequence_acquire(), current + limit);

            if next > available {
                return Idle;
            }

            highest = sequencer.get_highest(next, available);
            if next > highest {
                return Idle;
            }

            if self
                .sequence
                .compare_and_exchange_weak_volatile(current, highest)
            {
                break;
            }
        }

        for sequence in next..=highest {
            unsafe { sink.push(buffer.take(sequence)) }
        }

        sequencer.publish_gating_sequence(highest);
        State::Processing
    }
}

unsafe impl Sync for MultiConsumerPoller {}

// #[cfg(test)]
// mod tests {
//     use crate::coordinator::{ConsumerWaitStrategyKind, Coordinator, ProducerWaitStrategyKind};
//     use crate::poller::MultiConsumerPoller;
//     use crate::poller::State::Idle;
//     use crate::ring_buffer::RingBuffer;
//     use crate::sequencer::{MultiProducerSequencer, Sequencer};
//     use ConsumerWaitStrategyKind::Spinning;
//
//     #[test]
//     pub fn test_that_state_is_idle_when_sequence_is_claimed_but_not_published() {
//         let sequencer = MultiProducerSequencer::new(8);
//         let coordinator = Coordinator::new(ProducerWaitStrategyKind::Spinning, Spinning);
//
//         let _ = sequencer.next(&coordinator);
//
//         let mut buffer: Vec<i32> = Vec::with_capacity(8);
//
//         let ring_buffer = RingBuffer::new(8, sequencer, MultiConsumerPoller::new());
//
//         let state = ring_buffer.poll(&mut buffer);
//         assert_eq!(Idle, state);
//     }
// }
