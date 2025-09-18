use crate::event_handler::EvenHandler;
use crate::ring_buffer::RingBuffer;
use crate::sequence::Sequence;
use crate::sequencer::Sequencer;

#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub(crate) enum EventPollerState {
    Idle,
    Processing,
}

pub(crate) struct EventPoller<'a, T: Default> {
    buffer: &'a RingBuffer<T>,
    sequencer: &'a dyn Sequencer,
    sequence: &'a Sequence,
    gating_sequences: &'a Sequence,
}

impl<'a, T: Default> EventPoller<'a, T> {
    pub fn new(ring_buffer: &'a RingBuffer<T>) -> Self {
        let sequencer = ring_buffer.get_sequencer();
        Self {
            buffer: ring_buffer,
            sequencer: sequencer,
            sequence: sequencer.get_gating_sequence(),
            gating_sequences: sequencer.get_cursor_sequence(),
        }
    }

    pub fn poll<H>(&self, handler: &H) -> EventPollerState
    where
        H: EvenHandler<T>,
    {
        let next: i64 = self.sequence.get_plain() + 1;
        let available = self.gating_sequences.get_acquire();

        if next > available {
            return EventPollerState::Idle;
        }

        let highest: i64 = self.sequencer.get_highest(next, available);
        for sequence in next..=highest {
            if let Err(error) = handler.on_event(self.buffer.get(sequence)) {
                handler.on_error(error);
            }
        }

        self.sequence.set_release(highest);
        EventPollerState::Processing
    }
}

unsafe impl<'a, T: Default> Sync for EventPoller<'a, T> {}

unsafe impl<'a, T: Default> Send for EventPoller<'a, T> {}
