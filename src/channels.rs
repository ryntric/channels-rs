//! High-performance channel abstractions based on ring buffers.
//!
//! This module provides [`Sender`] and [`Receiver`] types built on top of
//! a ringBuffer with pluggable wait strategies. It supports different
//! concurrency configurations such as SPSC, MPSC, SPMC, and MPMC.
//!
//! The design is inspired by the Disruptor pattern, but with Rust’s ownership
//! and type safety. It allows batching, lock-free sending, and configurable
//! waiting strategies for both producers and consumers.

use crate::coordinator::Coordinator;
use crate::poller::State::Idle;
use crate::poller::{MultiConsumerPoller, SingleConsumerPoller};
use crate::prelude::{ConsumerWaitStrategyKind, ProducerWaitStrategyKind};
use crate::ring_buffer::RingBuffer;
use crate::sequencer::{MultiProducerSequencer, SingleProducerSequencer};
use crate::utils;
use std::sync::Arc;

/// A sending half of the channel.
///
/// `Sender<T>` pushes values into a ringBuffer and notifies the consumer
/// through the coordinator. It supports both single-item and batched sends.
#[derive(Clone)]
pub struct Sender<T> {
    buffer: Arc<RingBuffer<T>>,
    coordinator: Arc<Coordinator>
}

/// A receiving half of the channel.
///
/// `Receiver<T>` pulls values from a ringBuffer using a poller and can either
/// spin/yield/park/block depending on the chosen wait strategy. It supports both
/// non-blocking and blocking receive loops.
#[derive(Clone)]
pub struct Receiver<T> {
    buffer: Arc<RingBuffer<T>>,
    coordinator: Arc<Coordinator>
}

impl<T> Sender<T> {
    /// Send a single value into the buffer.
    ///
    /// If the buffer is full, the configured producer wait strategy determines
    /// how the call behaves (e.g. spin, yield, or park).
    pub fn send(&self, value: T) {
        self.buffer.push(value, &self.coordinator);
        self.coordinator.wakeup_consumer()
    }

    /// Send multiple values into the buffer in a batch.
    ///
    /// This is more efficient than calling [`send`](Self::send) repeatedly,
    /// as it reduces synchronization overhead.
    ///
    /// # Type Parameters
    /// - `I`: an `IntoIterator` where the iterator implements `ExactSizeIterator`.
    pub fn send_n<I>(&self, items: I)
    where
        I: IntoIterator<Item = T>,
        I::IntoIter: ExactSizeIterator,
    {
        self.buffer.push_n(items, &self.coordinator);
        self.coordinator.wakeup_consumer()
    }
}

impl<T> Receiver<T> {
    /// Attempt to receive up to `batch_size` items.
    ///
    /// Invokes the provided `handler` closure for each item.
    pub fn recv<H>(&self, batch_size: usize, handler: &H)
    where
        H: Fn(T),
    {
        if self.buffer.poll(batch_size, handler) == Idle {
            self.coordinator.consumer_wait();
        }
    }

    /// Continuously attempt to receive items until at least one batch is processed.
    ///
    /// This method blocks according to the configured consumer wait strategy.
    /// It is typically used in consumer loops.
    pub fn blocking_recv<H>(&self, batch_size: usize, handler: &H)
    where
        H: Fn(T),
    {
        while self.buffer.poll(batch_size, handler) == Idle {
            self.coordinator.consumer_wait();
        }
    }
}

/// Create a **single-producer single-consumer (SPSC)** channel.
///
/// - One producer thread
/// - One consumer thread
///
/// # Parameters
/// - `buffer_size`: capacity of the underlying ring buffer.
/// - `pw`: producer wait strategy.
/// - `cw`: consumer wait strategy.
pub fn spsc<T>(buffer_size: usize, pw: ProducerWaitStrategyKind, cw: ConsumerWaitStrategyKind) -> (Sender<T>, Receiver<T>) {
    utils::assert_buffer_size_is_equal_or_less_than_i64(buffer_size);
    utils::assert_buffer_size_pow_of_2(buffer_size);

    let sequencer = Box::new(SingleProducerSequencer::new(buffer_size));
    let poller = Box::new(SingleConsumerPoller::new());
    let coordinator = Arc::new(Coordinator::new(pw, cw));

    let buffer: Arc<RingBuffer<T>> = Arc::new(RingBuffer::new(buffer_size, sequencer, poller));
    let sender = Sender { buffer: buffer.clone(), coordinator: coordinator.clone() };
    let receiver = Receiver { buffer: buffer.clone(), coordinator: coordinator.clone() };

    (sender, receiver)
}

/// Create a **multi-producer single-consumer (MPSC)** channel.
///
/// - Multiple producers
/// - One consumer
///
/// # Parameters
/// - `buffer_size`: capacity of the underlying ring buffer.
/// - `pw`: producer wait strategy.
/// - `cw`: consumer wait strategy.
pub fn mpsc<T>(buffer_size: usize, pw: ProducerWaitStrategyKind, cw: ConsumerWaitStrategyKind) -> (Sender<T>, Receiver<T>) {
    utils::assert_buffer_size_is_equal_or_less_than_i64(buffer_size);
    utils::assert_buffer_size_pow_of_2(buffer_size);

    let sequencer = Box::new(MultiProducerSequencer::new(buffer_size));
    let poller = Box::new(SingleConsumerPoller::new());
    let coordinator = Arc::new(Coordinator::new(pw, cw));
    
    let buffer: Arc<RingBuffer<T>> = Arc::new(RingBuffer::new(buffer_size, sequencer, poller));
    let sender = Sender { buffer: buffer.clone(), coordinator: coordinator.clone() };
    let receiver = Receiver { buffer: buffer.clone(), coordinator: coordinator.clone() };

    (sender, receiver)
}

/// Create a **single-producer multi-consumer (SPMC)** channel.
///
/// - One producer
/// - Multiple consumers
///
/// # Parameters
/// - `buffer_size`: capacity of the underlying ring buffer.
/// - `pw`: producer wait strategy.
/// - `cw`: consumer wait strategy.
pub fn spmc<T>(buffer_size: usize, pw: ProducerWaitStrategyKind, cw: ConsumerWaitStrategyKind) -> (Sender<T>, Receiver<T>) {
    utils::assert_buffer_size_is_equal_or_less_than_i64(buffer_size);
    utils::assert_buffer_size_pow_of_2(buffer_size);

    let sequencer = Box::new(SingleProducerSequencer::new(buffer_size));
    let poller = Box::new(MultiConsumerPoller::new());
    let coordinator = Arc::new(Coordinator::new(pw, cw));

    let buffer: Arc<RingBuffer<T>> = Arc::new(RingBuffer::new(buffer_size, sequencer, poller));
    let sender = Sender { buffer: buffer.clone(), coordinator: coordinator.clone() };
    let receiver = Receiver { buffer: buffer.clone(), coordinator: coordinator.clone() };

    (sender, receiver)
}

/// Create a **multi-producer multi-consumer (MPMC)** channel.
///
/// - Multiple producers
/// - Multiple consumers
///
/// # Parameters
/// - `buffer_size`: capacity of the underlying ring buffer.
/// - `pw`: producer wait strategy.
/// - `cw`: consumer wait strategy.
pub fn mpmc<T>(buffer_size: usize, pw: ProducerWaitStrategyKind, cw: ConsumerWaitStrategyKind) -> (Sender<T>, Receiver<T>) {
    utils::assert_buffer_size_is_equal_or_less_than_i64(buffer_size);
    utils::assert_buffer_size_pow_of_2(buffer_size);

    let sequencer = Box::new(MultiProducerSequencer::new(buffer_size));
    let poller = Box::new(MultiConsumerPoller::new());
    let coordinator = Arc::new(Coordinator::new(pw, cw));

    let buffer: Arc<RingBuffer<T>> = Arc::new(RingBuffer::new(buffer_size, sequencer, poller));
    let sender = Sender { buffer: buffer.clone(), coordinator: coordinator.clone() };
    let receiver = Receiver { buffer: buffer.clone(), coordinator: coordinator.clone() };

    (sender, receiver)
}
