use crate::buffer::Buffer;
use crate::coordinator::Coordinator;
use crate::poller::{Poller, State};
use crate::sequencer::Sequencer;

pub(crate) struct RingBuffer<T: Send, S: Sequencer, P: Poller<T, S>> {
    buffer: Buffer<T>,
    sequencer: S,
    poller: P,
}

impl<T: Send, S: Sequencer, P: Poller<T, S>> RingBuffer<T, S, P> {
    pub fn new(capacity: usize, sequencer: S, poller: P) -> Self {
        RingBuffer {
            buffer: Buffer::new(capacity),
            sequencer,
            poller,
        }
    }

    pub fn poll(&self, dst: &mut Vec<T>) -> State {
        assert!(dst.is_empty(), "buffer should be empty");
        self.poller.poll(&self.sequencer, &self.buffer, dst)
    }

    pub fn push(&self, element: T, coordinator: &Coordinator) {
        let sequence = self.sequencer.next(coordinator);
        unsafe { self.buffer.insert(sequence, element) }
        self.sequencer.publish_cursor_sequence(sequence);
    }

    pub fn push_v(&self, items: Vec<T>, coordinator: &Coordinator) {
        let length = items.len();

        let high = self.sequencer.next_n(length, coordinator);
        let low = high - (length - 1) as i64;

        for (sequence, element) in (low..=high).zip(items) {
            unsafe { self.buffer.insert(sequence, element) }
        }

        self.sequencer.publish_cursor_sequence_range(low, high);
    }

    pub fn push_a<const N: usize>(&self, items: [T; N], coordinator: &Coordinator) {
        let length = items.len();

        let high = self.sequencer.next_n(length, coordinator);
        let low = high - (length - 1) as i64;

        for (sequence, element) in (low..=high).zip(items) {
            unsafe { self.buffer.insert(sequence, element) }
        }

        self.sequencer.publish_cursor_sequence_range(low, high);
    }
}

unsafe impl<T: Send, S: Sequencer, P: Poller<T, S>> Sync for RingBuffer<T, S, P> {}
