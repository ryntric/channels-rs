use crate::ring_buffer::RingBuffer;
use crate::sequence::Sequence;
use crate::sequencer::Sequencer;
use std::sync::Arc;

#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub enum EventPollerState {
    Idle,
    Processing,
}

pub struct EventPoller<T> {
    buffer: Arc<RingBuffer<T>>,
    sequencer: Arc<dyn Sequencer>,
    sequence: Arc<Sequence>,
    gating_sequences: Arc<Sequence>,
}

impl<T> EventPoller<T> {
    pub fn new(ring_buffer: Arc<RingBuffer<T>>) -> Self {
        let sequencer = ring_buffer.get_sequencer();
        Self {
            buffer: ring_buffer,
            sequencer: sequencer.clone(),
            sequence: sequencer.get_gating_sequence(),
            gating_sequences: sequencer.get_cursor_sequence(),
        }
    }

    pub fn poll<H: Fn(T)>(&self, handler: &H) -> EventPollerState {
        let next: i64 = self.sequence.get_plain() + 1;
        let available = self.gating_sequences.get_acquire();

        if next > available {
            return EventPollerState::Idle;
        }

        let highest: i64 = self.sequencer.get_highest(next, available);
        for sequence in next..=highest {
            handler(self.buffer.poll(sequence));
        }

        self.sequence.set_release(highest);
        EventPollerState::Processing
    }
}

unsafe impl<T> Sync for EventPoller<T> {}

unsafe impl<T> Send for EventPoller<T> {}
