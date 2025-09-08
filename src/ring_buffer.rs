use crate::event_translator::EventTranslatorOneArg;
use crate::sequencer::Sequencer;
use crate::utils;
use std::cell::UnsafeCell;

pub struct RingBuffer<V: Default + Copy> {
    buffer: Box<[UnsafeCell<V>]>,
    sequencer: Box<dyn Sequencer>,
    mask: i64,
}

impl<V: Default + Copy> RingBuffer<V> {
    pub fn new(buffer_size: usize, sequencer: Box<dyn Sequencer>) -> RingBuffer<V> {
        RingBuffer {
            buffer: (0..buffer_size)
                .map(|_| UnsafeCell::new(V::default()))
                .collect::<Vec<_>>()
                .into_boxed_slice(),
            sequencer,
            mask: (buffer_size - 1) as i64,
        }
    }

    #[inline(always)]
    fn element_at(&self, sequence: i64, mask: i64) -> &UnsafeCell<V> {
        let index: usize = utils::wrap_index(sequence, mask);
        &self.buffer[index]
    }

    pub fn get_sequencer(&self) -> &Box<dyn Sequencer> {
        &self.sequencer
    }

    pub fn publish_event<T, A>(&self, translator: T, arg: A)
    where
        T: EventTranslatorOneArg<V, A>,
    {
        let sequence = self.sequencer.next();
        let element = self.element_at(sequence, self.mask);
        unsafe { translator.translate_to(&mut *element.get(), arg) }
        self.sequencer.publish(sequence);
    }
}

unsafe impl<V: Default + Copy> Sync for RingBuffer<V> {}

unsafe impl<V: Default + Copy> Send for RingBuffer<V> {}
