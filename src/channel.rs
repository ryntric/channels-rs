use crate::poller::{PollState, Poller, SinglePoller};
use crate::ring_buffer::RingBuffer;
use crate::sequencer::{MultiProducerSequencer, SingleProducerSequencer};
use std::sync::Arc;

#[derive(Clone)]
pub struct Sender<T> {
    buffer: Arc<RingBuffer<T>>,
}

impl<T> Sender<T> {
    
    pub fn send(&self, value: T) {
        self.buffer.push(value);
    }

    pub fn send_n<I>(&self, items: I)
    where
        I: IntoIterator<Item = T>,
        I::IntoIter: ExactSizeIterator,
    {
        self.buffer.push_n(items);
    }
}

#[derive(Clone)]
pub struct Receiver<T> {
    buffer: Arc<RingBuffer<T>>,
}

impl<T> Receiver<T> {
    pub fn recv<H>(&self, handler: &H) -> PollState
    where
        H: Fn(T),
    {
        self.buffer.poll(handler)
    }
}

pub fn spsc<T>(buffer_size: usize) -> (Sender<T>, Receiver<T>) {
    let sequencer = SingleProducerSequencer::new(buffer_size);
    let poller = SinglePoller::new();
    
    let buffer: Arc<RingBuffer<T>> = Arc::new(RingBuffer::new(buffer_size, Box::new(sequencer), Box::new(poller)));
    let sender = Sender { buffer: buffer.clone() };
    let receiver = Receiver { buffer: buffer.clone() };
    
    (sender, receiver)
}

pub fn mpsc<T>(buffer_size: usize) -> (Sender<T>, Receiver<T>) {
    let sequencer = MultiProducerSequencer::new(buffer_size);
    let poller = SinglePoller::new();
    
    let buffer: Arc<RingBuffer<T>> = Arc::new(RingBuffer::new(buffer_size, Box::new(sequencer), Box::new(poller)));
    let sender = Sender { buffer: buffer.clone() };
    let receiver = Receiver { buffer: buffer.clone() };
    
    (sender, receiver)
}
