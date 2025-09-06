use crate::event_translator::EventTranslatorOneArg;
use crate::sequence::Sequence;
use crate::sequencer;
use crate::sequencer::OneToOneSequencer;
use std::sync::Arc;

pub struct RingBuffer<V: Default + Copy, const N: usize> {
    buffer: [V; N],
    sequencer: Box<dyn sequencer::Sequencer>,
    buffer_size: usize,
    mask: i64,
}

impl<V: Default + Copy, const N: usize> RingBuffer<V, N> {
    pub fn new() -> RingBuffer<V, N> {
        RingBuffer {
            buffer: [V::default(); N],
            sequencer: Box::new(OneToOneSequencer::new(N)),
            buffer_size: N,
            mask: (N - 1) as i64,
        }
    }

    fn wrap_index(sequence: i64, mask: i64) -> usize {
        (sequence & mask) as usize
    }

    pub fn get(&self, sequence: i64) -> V {
        self.buffer[Self::wrap_index(sequence, self.mask)]
    }

    pub fn publish_event<T, A>(&mut self, translator: T, arg: A)
    where
        T: EventTranslatorOneArg<V, A>,
    {
        let next = self.sequencer.next();
        let index = Self::wrap_index(next, self.mask);
        let event = &mut self.buffer[index];
        translator.translate_to(event, arg);
        self.sequencer.publish(next);
    }
}
