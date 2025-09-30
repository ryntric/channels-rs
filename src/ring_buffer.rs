use crate::poller::{Poller, State};
use crate::sequencer::Sequencer;
use crate::wait_strategy::WaitStrategy;
use crate::{constants, utils};
use std::cell::UnsafeCell;
use std::mem::MaybeUninit;
use std::ptr;

pub(crate) struct RingBuffer<T> {
    buffer: Box<[UnsafeCell<MaybeUninit<T>>]>,
    sequencer: Box<dyn Sequencer>,
    poller: Box<dyn Poller<T>>,
    mask: i64,
}

impl<T> RingBuffer<T> {
    pub fn new(buffer_size: usize, sequencer: Box<dyn Sequencer>, poller: Box<dyn Poller<T>>) -> RingBuffer<T> {
        RingBuffer {
            buffer: Self::create_buffer(buffer_size),
            sequencer: sequencer,
            poller: poller,
            mask: (buffer_size - 1) as i64,
        }
    }

    fn create_buffer(buffer_size: usize) -> Box<[UnsafeCell<MaybeUninit<T>>]> {
        (0..buffer_size + (constants::ARRAY_PADDING << 1))
            .map(|_| UnsafeCell::new(MaybeUninit::uninit()))
            .collect::<Vec<_>>()
            .into_boxed_slice()
    }

    pub(crate) fn dequeue(&self, sequence: i64) -> T {
        let index: usize = utils::wrap_index(sequence, self.mask, constants::ARRAY_PADDING);
        let cell = &self.buffer[index];
        unsafe { ptr::read((*cell.get()).as_ptr()) }
    }

    pub fn poll<H: Fn(T)>(&self, batch_size: i64, handler: &H) -> State {
        self.poller.poll(&*self.sequencer, &self, batch_size, &handler)
    }

    pub fn push(&self, element: T, wait_strategy: &WaitStrategy) {
        let sequence = self.sequencer.next(wait_strategy);
        let cell = &self.buffer[utils::wrap_index(sequence, self.mask, constants::ARRAY_PADDING)];
        unsafe { (*cell.get()).write(element); }
        self.sequencer.publish_cursor_sequence(sequence);
    }

    pub fn push_n<I>(&self, items: I, wait_strategy: &WaitStrategy)
    where
        I: IntoIterator<Item=T>,
        I::IntoIter: ExactSizeIterator,
    {
        let iterator = items.into_iter();
        let length = iterator.len();
        let high = self.sequencer.next_n(length, wait_strategy);
        let low = high - (length - 1) as i64;

        for (index, item) in iterator.enumerate() {
            let cell = &self.buffer[utils::wrap_index(index as i64 + low, self.mask, constants::ARRAY_PADDING)];
            unsafe { (*cell.get()).write(item); }
        }

        self.sequencer.publish_cursor_sequence_range(low, high);
    }
}

unsafe impl<T> Sync for RingBuffer<T> {}

unsafe impl<T> Send for RingBuffer<T> {}
