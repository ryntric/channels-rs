use crate::poller::State::Idle;
use crate::poller::{MultiConsumerPoller, PollerKind, SingleConsumerPoller};
use crate::ring_buffer::RingBuffer;
use crate::sequencer::{MultiProducerSequencer, SequencerKind, SingleProducerSequencer};
use crate::wait_strategy::WaitStrategy;
use std::sync::Arc;

#[derive(Clone)]
pub struct Sender<T> {
    buffer: Arc<RingBuffer<T>>,
}

#[derive(Clone)]
pub struct Receiver<T> {
    buffer: Arc<RingBuffer<T>>,
}

impl<T> Sender<T> {
    pub fn send(&self, value: T, strategy: WaitStrategy) {
        self.buffer.push(value, strategy);
    }

    pub fn send_n<I>(&self, items: I, strategy: WaitStrategy)
    where
        I: IntoIterator<Item = T>,
        I::IntoIter: ExactSizeIterator,
    {
        self.buffer.push_n(items, strategy);
    }
}

impl<T> Receiver<T> {
    pub fn recv<H>(&self, handler: &H)
    where
        H: Fn(T),
    {
        self.buffer.poll(handler);
    }

    pub fn blocking_recv<H>(&self, strategy: WaitStrategy, handler: &H)
    where
        H: Fn(T),
    {
        while self.buffer.poll(handler) == Idle {
            strategy.wait();
        }
    }
}

pub fn spsc<T>(buffer_size: usize) -> (Sender<T>, Receiver<T>) {
    let sequencer = SequencerKind::SingleProducer(SingleProducerSequencer::new(buffer_size));
    let poller = PollerKind::SingleConsumer(SingleConsumerPoller::new((buffer_size >> 4) as i64));

    let buffer: Arc<RingBuffer<T>> = Arc::new(RingBuffer::new(buffer_size, sequencer, poller));
    let sender = Sender { buffer: buffer.clone() };
    let receiver = Receiver { buffer: buffer.clone() };

    (sender, receiver)
}

pub fn mpsc<T>(buffer_size: usize) -> (Sender<T>, Receiver<T>) {
    let sequencer = SequencerKind::MultiProducer(MultiProducerSequencer::new(buffer_size));
    let poller = PollerKind::SingleConsumer(SingleConsumerPoller::new((buffer_size >> 4) as i64));

    let buffer: Arc<RingBuffer<T>> = Arc::new(RingBuffer::new(buffer_size, sequencer, poller));
    let sender = Sender { buffer: buffer.clone() };
    let receiver = Receiver { buffer: buffer.clone() };

    (sender, receiver)
}

pub fn spmc<T>(buffer_size: usize) -> (Sender<T>, Receiver<T>) {
    let sequencer = SequencerKind::SingleProducer(SingleProducerSequencer::new(buffer_size));
    let poller = PollerKind::MultiConsumer(MultiConsumerPoller::new((buffer_size >> 4) as i64));

    let buffer: Arc<RingBuffer<T>> = Arc::new(RingBuffer::new(buffer_size, sequencer, poller));
    let sender = Sender { buffer: buffer.clone() };
    let receiver = Receiver { buffer: buffer.clone() };

    (sender, receiver)
}

pub fn mpmc<T>(buffer_size: usize) -> (Sender<T>, Receiver<T>) {
    let sequencer = SequencerKind::MultiProducer(MultiProducerSequencer::new(buffer_size));
    let poller = PollerKind::MultiConsumer(MultiConsumerPoller::new((buffer_size >> 4) as i64));

    let buffer: Arc<RingBuffer<T>> = Arc::new(RingBuffer::new(buffer_size, sequencer, poller));
    let sender = Sender { buffer: buffer.clone() };
    let receiver = Receiver { buffer: buffer.clone() };

    (sender, receiver)
}
