use crate::constants;
use crate::sequencer::{OneToOneSequencer, Sequencer, SequencerType};
use std::cell::UnsafeCell;

pub fn wrap_index(sequence: i64, mask: i64, padding: usize) -> usize {
    (sequence & mask) as usize + padding
}

pub fn create_sequencer(buffer_size: usize, sequencer_type: SequencerType) -> Box<dyn Sequencer> {
    match sequencer_type {
        SequencerType::SingleProducer => Box::new(OneToOneSequencer::new(buffer_size)),
        SequencerType::MultiProducer => Box::new(OneToOneSequencer::new(buffer_size)),
    }
}

pub fn create_padded_buffer<T: Default>(buffer_size: usize, padding: usize) -> Box<[UnsafeCell<T>]> {
    (0..buffer_size + (padding << 1))
        .map(|_| UnsafeCell::new(T::default()))
        .collect::<Vec<_>>()
        .into_boxed_slice()
}

pub fn assert_buffer_size_pow_of_2(buffer_size: usize) -> usize {
    if buffer_size.count_ones() != 1 {
        panic!("buffer size must be a power of two")
    };
    buffer_size
}

pub fn log2(buffer_size: usize) -> u32 {
    (usize::BITS - 1) - buffer_size.leading_zeros()
}
