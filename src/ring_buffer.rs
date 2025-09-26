use crate::sequencer::{ManyToOneSequencer, OneToOneSequencer, Sequencer, SequencerType};
use crate::{constants, utils};
use std::cell::UnsafeCell;
use std::mem::MaybeUninit;
use std::ptr;
use std::sync::Arc;

pub struct RingBuffer<T> {
    buffer: Box<[UnsafeCell<MaybeUninit<T>>]>,
    sequencer: Arc<dyn Sequencer>,
    mask: i64,
}

impl<T> RingBuffer<T> {
    pub fn new(buffer_size: usize, sequencer_type: SequencerType) -> RingBuffer<T> {
        RingBuffer {
            buffer: Self::create_buffer(buffer_size),
            sequencer: Self::create_sequencer(buffer_size, sequencer_type),
            mask: (buffer_size - 1) as i64,
        }
    }

    fn create_buffer(buffer_size: usize) -> Box<[UnsafeCell<MaybeUninit<T>>]> {
        (0..buffer_size + (constants::ARRAY_PADDING << 1))
            .map(|_| UnsafeCell::new(MaybeUninit::uninit()))
            .collect::<Vec<_>>()
            .into_boxed_slice()
    }

    fn create_sequencer(buffer_size: usize, sequencer_type: SequencerType) -> Arc<dyn Sequencer> {
        match sequencer_type {
            SequencerType::SingleProducer => Arc::new(OneToOneSequencer::new(buffer_size)),
            SequencerType::MultiProducer => Arc::new(ManyToOneSequencer::new(buffer_size)),
        }
    }

    pub fn poll(&self, sequence: i64) -> T {
        let index: usize = utils::wrap_index(sequence, self.mask, constants::ARRAY_PADDING);
        let cell = &self.buffer[index];
        unsafe { ptr::read((*cell.get()).as_ptr()) }
    }

    pub fn get_sequencer(&self) -> Arc<dyn Sequencer> {
        Arc::clone(&self.sequencer)
    }

    pub fn push(&self, element: T) {
        let sequence = self.sequencer.next();
        let index: usize = utils::wrap_index(sequence, self.mask, constants::ARRAY_PADDING);
        let cell = &self.buffer[index];
        unsafe { (*cell.get()).write(element); }
        self.sequencer.publish(sequence);
    }
}

unsafe impl<V: Default> Sync for RingBuffer<V> {}

unsafe impl<V: Default> Send for RingBuffer<V> {}
