use crate::poller::{PollState, Poller};
use crate::sequencer::Sequencer;
use crate::{constants, utils};
use std::cell::UnsafeCell;
use std::mem::MaybeUninit;
use std::ptr;

pub struct RingBuffer<T, S, P>
where
    S: Sequencer,
    P: Poller<T, S>,
{
    buffer: Box<[UnsafeCell<MaybeUninit<T>>]>,
    sequencer: S,
    poller: P,
    mask: i64,
}

unsafe impl<T, S, P> Sync for RingBuffer<T, S, P> where S: Sequencer, P: Poller<T, S> {}

unsafe impl<T, S, P> Send for RingBuffer<T, S, P> where S: Sequencer, P: Poller<T, S> {}

impl<T, S, P> RingBuffer<T, S, P>
where
    S: Sequencer,
    P: Poller<T, S>,
{
    pub fn new(buffer_size: usize) -> RingBuffer<T, S, P> {
        RingBuffer {
            buffer: Self::create_buffer(buffer_size),
            sequencer: S::new(buffer_size),
            poller: P::new(),
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

    pub fn poll<H: Fn(T)>(&self, handler: &H) -> PollState {
        self.poller.poll(&self.sequencer, &self, &handler)
    }

    pub fn push(&self, element: T) {
        let sequence = self.sequencer.next();
        let index: usize = utils::wrap_index(sequence, self.mask, constants::ARRAY_PADDING);
        let cell = &self.buffer[index];
        unsafe { (*cell.get()).write(element); }
        self.sequencer.publish_cursor_sequence(sequence);
    }
}
