use crate::ring_buffer::RingBuffer;
use crate::sequencer::Sequencer;

#[derive(Copy, Clone, Debug, PartialEq)]
pub enum PollState {
    Empty,
    Ready,
}

pub(crate) trait Poller<T, S: Sequencer>: Send + Sync + Sized {
    fn new() -> Self;

    fn poll<H: Fn(T)>(&self, sequencer: &S, buffer: &RingBuffer<T, S, Self>, handler: &H) -> PollState;
}

pub struct SingleConsumer;

unsafe impl Send for SingleConsumer {}

unsafe impl Sync for SingleConsumer {}

impl<T, S: Sequencer> Poller<T, S> for SingleConsumer {
    fn new() -> Self {
        Self {}
    }

    fn poll<H: Fn(T)>(&self, sequencer: &S, buffer: &RingBuffer<T, S, Self>, handler: &H) -> PollState {
        let next: i64 = sequencer.get_gating_sequence_relaxed() + 1;
        let available: i64 = sequencer.get_cursor_sequence_acquire();

        if next > available {
            return PollState::Empty;
        }

        let highest: i64 = sequencer.get_highest(next, available);
        for sequence in next..=highest {
            handler(buffer.dequeue(sequence));
        }

        sequencer.publish_gating_sequence(highest);
        PollState::Ready
    }
}
