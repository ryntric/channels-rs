use crate::event_translator::EventTranslatorOneArg;
use crate::sequencer::{ManyToOneSequencer, OneToOneSequencer, Sequencer, SequencerType};
use crate::{constants, utils};
use std::cell::UnsafeCell;

pub struct RingBuffer<V: Default> {
    buffer_size: usize,
    buffer: Box<[UnsafeCell<V>]>,
    sequencer: Box<dyn Sequencer>,
    mask: i64,
}

impl<V: Default> RingBuffer<V> {
    pub fn new(buffer_size: usize, sequencer_type: SequencerType) -> RingBuffer<V> {
        RingBuffer {
            buffer_size: buffer_size,
            buffer: Self::create_buffer(buffer_size),
            sequencer: Self::create_sequencer(buffer_size, sequencer_type),
            mask: (buffer_size - 1) as i64,
        }
    }

    fn create_buffer<T: Default>(buffer_size: usize) -> Box<[UnsafeCell<T>]> {
        (0..buffer_size + (constants::ARRAY_PADDING << 1))
            .map(|_| UnsafeCell::new(T::default()))
            .collect::<Vec<_>>()
            .into_boxed_slice()
    }

    fn create_sequencer(buffer_size: usize, sequencer_type: SequencerType) -> Box<dyn Sequencer> {
        match sequencer_type {
            SequencerType::SingleProducer => Box::new(OneToOneSequencer::new(buffer_size)),
            SequencerType::MultiProducer => Box::new(ManyToOneSequencer::new(buffer_size)),
        }
    }
    
    pub fn element_at(&self, sequence: i64) -> *mut V {
        let index: usize = utils::wrap_index(sequence, self.mask, constants::ARRAY_PADDING);
        let cell = &self.buffer[index];
        cell.get()
    }

    pub fn get(&self, sequence: i64) -> &V {
        unsafe { &*self.element_at(sequence) }
    }

    pub fn get_sequencer(&self) -> &dyn Sequencer {
        &*self.sequencer
    }

    pub fn publish_event<T, A>(&self, translator: T, arg: A)
    where
        T: EventTranslatorOneArg<V, A>,
    {
        let sequence = self.sequencer.next();
        let event = self.element_at(sequence);
        unsafe { translator.translate_to(&mut *event, arg) }
        self.sequencer.publish(sequence);
    }
}

unsafe impl<V: Default> Sync for RingBuffer<V> {}

unsafe impl<V: Default> Send for RingBuffer<V> {}
