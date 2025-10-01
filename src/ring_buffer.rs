use crate::coordinator::Coordinator;
use crate::poller::{Poller, State};
use crate::sequencer::Sequencer;
use crate::{constants, utils};
use std::cell::UnsafeCell;
use std::mem::MaybeUninit;
use std::ptr;

pub(crate) struct RingBuffer<T> {
    buffer: Box<[UnsafeCell<MaybeUninit<T>>]>,
    sequencer: Box<dyn Sequencer>,
    poller: Box<dyn Poller<T>>,
    mask: i64,
    buffer_size: usize,
}

impl<T> RingBuffer<T> {
    pub fn new(buffer_size: usize, sequencer: Box<dyn Sequencer>, poller: Box<dyn Poller<T>>) -> RingBuffer<T> {
        RingBuffer {
            buffer: Self::create_buffer(buffer_size),
            sequencer: sequencer,
            poller: poller,
            mask: (buffer_size - 1) as i64,
            buffer_size: buffer_size
        }
    }

    fn create_buffer(buffer_size: usize) -> Box<[UnsafeCell<MaybeUninit<T>>]> {
        (0..buffer_size + (constants::ARRAY_PADDING << 1))
            .map(|_| UnsafeCell::new(MaybeUninit::uninit()))
            .collect::<Vec<_>>()
            .into_boxed_slice()
    }

    #[inline(always)]
    fn check_size(&self, size: usize) {
        if size > self.buffer_size {
            std::panic::panic_any("size is greater than buffer size");
        }
    }

    pub(crate) fn dequeue(&self, sequence: i64) -> T {
        let index: usize = utils::wrap_index(sequence, self.mask, constants::ARRAY_PADDING);
        let cell = &self.buffer[index];
        unsafe { ptr::read((*cell.get()).as_ptr()) }
    }

    pub fn poll<H: Fn(T)>(&self, batch_size: usize, handler: &H) -> State {
        self.check_size(batch_size);
        self.poller.poll(&*self.sequencer, &self, batch_size as i64, &handler)
    }

    pub fn push(&self, element: T, coordinator: &Coordinator) {
        let sequence = self.sequencer.next(coordinator);
        let cell = &self.buffer[utils::wrap_index(sequence, self.mask, constants::ARRAY_PADDING)];
        unsafe { (*cell.get()).write(element); }
        self.sequencer.publish_cursor_sequence(sequence);
    }

    pub fn push_n<I>(&self, items: I, coordinator: &Coordinator)
    where
        I: IntoIterator<Item=T>,
        I::IntoIter: ExactSizeIterator,
    {
        let iterator = items.into_iter();
        let length = iterator.len();
        self.check_size(length);
        let high = self.sequencer.next_n(length, coordinator);
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
