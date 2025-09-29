use crate::channel::Sender;
use crate::ring_buffer::RingBuffer;
use crate::sequencer::Sequencer;

#[derive(Copy, Clone, Debug, PartialEq)]
pub enum PollState {
    Idle,
    Processing,
}

pub(crate) trait Poller<T>: Send + Sync {
    fn poll(&self, sequencer: &dyn Sequencer, buffer: &RingBuffer<T>, handler: &dyn Fn(T)) -> PollState;
}

pub(crate) struct SinglePoller;

impl SinglePoller {
    pub fn new() -> SinglePoller {
        Self
    }
}

impl<T> Poller<T> for SinglePoller {
    fn poll(&self, sequencer: &dyn Sequencer, buffer: &RingBuffer<T>, handler: &dyn Fn(T)) -> PollState {
        let next: i64 = sequencer.get_gating_sequence_relaxed() + 1;
        let available: i64 = sequencer.get_cursor_sequence_acquire();

        if next > available {
            return PollState::Idle;
        }

        let highest: i64 = sequencer.get_highest(next, available);
        for sequence in next..=highest {
            handler(buffer.dequeue(sequence));
        }

        sequencer.publish_gating_sequence(highest);
        PollState::Processing
    }
}

unsafe impl Send for SinglePoller {}

unsafe impl Sync for SinglePoller {}
