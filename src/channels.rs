use crate::coordinator::Coordinator;
use crate::poller::State::Idle;
use crate::poller::{MultiConsumerPoller, SingleConsumerPoller, State};
use crate::prelude::{ConsumerWaitStrategyKind, ProducerWaitStrategyKind};
use crate::ring_buffer::RingBuffer;
use crate::sequencer::{MultiProducerSequencer, SingleProducerSequencer};
use std::sync::Arc;

#[derive(Clone)]
pub struct Sender<T> {
    buffer: Arc<RingBuffer<T>>,
    coordinator: Arc<Coordinator>
}

#[derive(Clone)]
pub struct Receiver<T> {
    buffer: Arc<RingBuffer<T>>,
    coordinator: Arc<Coordinator>
}

impl<T> Sender<T> {
    pub fn send(&self, value: T) {
        self.buffer.push(value, &*self.coordinator);
        self.coordinator.wakeup_consumer()
    }

    pub fn send_n<I>(&self, items: I)
    where
        I: IntoIterator<Item = T>,
        I::IntoIter: ExactSizeIterator,
    {
        self.buffer.push_n(items, &*self.coordinator);
        self.coordinator.wakeup_consumer()
    }
}

impl<T> Receiver<T> {
    pub fn recv<H>(&self, batch_size: usize, handler: &H) -> State
    where
        H: Fn(T),
    {
        self.buffer.poll(batch_size, handler)
    }

    pub fn blocking_recv<H>(&self, batch_size: usize, handler: &H)
    where
        H: Fn(T),
    {
        while self.recv(batch_size, handler) == Idle {
            self.coordinator.consumer_wait()
        }
    }
}

pub fn spsc<T>(buffer_size: usize, pw: ProducerWaitStrategyKind, cw: ConsumerWaitStrategyKind) -> (Sender<T>, Receiver<T>) {
    let sequencer = Box::new(SingleProducerSequencer::new(buffer_size));
    let poller = Box::new(SingleConsumerPoller::new());
    let coordinator = Arc::new(Coordinator::new(pw, cw));

    let buffer: Arc<RingBuffer<T>> = Arc::new(RingBuffer::new(buffer_size, sequencer, poller));
    let sender = Sender { buffer: buffer.clone(), coordinator: coordinator.clone() };
    let receiver = Receiver { buffer: buffer.clone(), coordinator: coordinator.clone() };

    (sender, receiver)
}

pub fn mpsc<T>(buffer_size: usize, pw: ProducerWaitStrategyKind, cw: ConsumerWaitStrategyKind) -> (Sender<T>, Receiver<T>) {
    let sequencer = Box::new(MultiProducerSequencer::new(buffer_size));
    let poller = Box::new(SingleConsumerPoller::new());
    let coordinator = Arc::new(Coordinator::new(pw, cw));
    
    let buffer: Arc<RingBuffer<T>> = Arc::new(RingBuffer::new(buffer_size, sequencer, poller));
    let sender = Sender { buffer: buffer.clone(), coordinator: coordinator.clone() };
    let receiver = Receiver { buffer: buffer.clone(), coordinator: coordinator.clone() };

    (sender, receiver)
}

pub fn spmc<T>(buffer_size: usize, pw: ProducerWaitStrategyKind, cw: ConsumerWaitStrategyKind) -> (Sender<T>, Receiver<T>) {
    let sequencer = Box::new(SingleProducerSequencer::new(buffer_size));
    let poller = Box::new(MultiConsumerPoller::new());
    let coordinator = Arc::new(Coordinator::new(pw, cw));

    let buffer: Arc<RingBuffer<T>> = Arc::new(RingBuffer::new(buffer_size, sequencer, poller));
    let sender = Sender { buffer: buffer.clone(), coordinator: coordinator.clone() };
    let receiver = Receiver { buffer: buffer.clone(), coordinator: coordinator.clone() };

    (sender, receiver)
}

pub fn mpmc<T>(buffer_size: usize, pw: ProducerWaitStrategyKind, cw: ConsumerWaitStrategyKind) -> (Sender<T>, Receiver<T>) {
    let sequencer = Box::new(MultiProducerSequencer::new(buffer_size));
    let poller = Box::new(MultiConsumerPoller::new());
    let coordinator = Arc::new(Coordinator::new(pw, cw));

    let buffer: Arc<RingBuffer<T>> = Arc::new(RingBuffer::new(buffer_size, sequencer, poller));
    let sender = Sender { buffer: buffer.clone(), coordinator: coordinator.clone() };
    let receiver = Receiver { buffer: buffer.clone(), coordinator: coordinator.clone() };

    (sender, receiver)
}
