use crate::coordinator::{ConsumerWaitStrategyKind, Coordinator, ProducerWaitStrategyKind};
use crate::poller::{MultiConsumerPoller, SingleConsumerPoller, State};
use crate::ring_buffer::RingBuffer;
use crate::sequencer::{MultiProducerSequencer, SingleProducerSequencer};

use std::sync::Arc;

macro_rules! sender {
    (
        $(#[$meta:meta])*
        $name:ident, $sequencer:ty, $poller:ty $(,)?
    ) => {
        $(#[$meta])*
        pub struct $name<T: Send> {
            ring_buffer: Arc<RingBuffer<T, $sequencer, $poller>>,
            coordinator: Arc<Coordinator>,
        }

        impl<T: Send> Sender<T> for $name<T> {
            fn send(&self, value: T) {
                self.ring_buffer.push(value, &self.coordinator);
                self.coordinator.wakeup_consumer();
            }

            fn send_v(&self, items: Vec<T>) {
                self.ring_buffer.push_v(items, &self.coordinator);
                self.coordinator.wakeup_consumer();
            }

            fn send_a<const N: usize>(&self, items: [T; N]) {
                self.ring_buffer.push_a(items, &self.coordinator);
                self.coordinator.wakeup_consumer();
            }
        }
    };
}

macro_rules! receiver {
    (
       $(#[$meta:meta])*
       $name:ident, $sequencer:ty, $poller:ty $(,)?
    ) => {
        $(#[$meta])*
        pub struct $name<T: Send> {
            ring_buffer: Arc<RingBuffer<T, $sequencer, $poller>>,
            coordinator: Arc<Coordinator>,
        }

        impl<T: Send> Receiver<T> for $name<T> {
            fn recv(&self, buffer: &mut Vec<T>) {
                if self.ring_buffer.poll(buffer) == State::Idle {
                    self.coordinator.consumer_wait();
                }
            }

            fn blocking_recv(&self, buffer: &mut Vec<T>) {
                while self.ring_buffer.poll(buffer) == State::Idle  {
                    self.coordinator.consumer_wait();
                }
            }
        }
    };
}

macro_rules! impl_clone {
    ($($name:ident),+ $(,)?) => {
        $(
            impl<T: Send> Clone for $name<T> {
                fn clone(&self) -> Self {
                    Self {
                        ring_buffer: Arc::clone(&self.ring_buffer),
                        coordinator: Arc::clone(&self.coordinator),
                    }
                }
            }
        )+
    };
}

macro_rules! channel_factory {
    (
        $name:ident,
        $sender:ident,
        $receiver:ident,
        $sequencer:ty,
        $poller:ty $(,)?
    ) => {
        pub fn $name<T: Send>(
            capacity: usize,
            pw: ProducerWaitStrategyKind,
            cw: ConsumerWaitStrategyKind,
        ) -> ($sender<T>, $receiver<T>) {
            let coordinator = Arc::new(Coordinator::new(pw, cw));
            let ring_buffer = Arc::new(RingBuffer::new(
                capacity,
                <$sequencer>::new(capacity),
                <$poller>::default(),
            ));

            let sender = $sender {
                ring_buffer: Arc::clone(&ring_buffer),
                coordinator: Arc::clone(&coordinator),
            };
            let receiver = $receiver {
                ring_buffer,
                coordinator,
            };

            (sender, receiver)
        }
    };
}

pub trait Sender<T: Send>: Sized {
    fn send(&self, value: T);

    fn send_v(&self, items: Vec<T>);

    fn send_a<const N: usize>(&self, items: [T; N]);
}

pub trait Receiver<T: Send>: Sized {
    fn recv(&self, buffer: &mut Vec<T>);

    fn blocking_recv(&self, buffer: &mut Vec<T>);
}

sender!(
    #[must_use]
    SpscSender,
    SingleProducerSequencer,
    SingleConsumerPoller
);
sender!(
    #[must_use]
    SpmcSender,
    SingleProducerSequencer,
    MultiConsumerPoller
);
sender!(
    #[must_use]
    MpscSender,
    MultiProducerSequencer,
    SingleConsumerPoller
);
sender!(
    #[must_use]
    MpmcSender,
    MultiProducerSequencer,
    MultiConsumerPoller
);

receiver!(
    #[must_use]
    SpscReceiver,
    SingleProducerSequencer,
    SingleConsumerPoller
);
receiver!(
    #[must_use]
    SpmcReceiver,
    SingleProducerSequencer,
    MultiConsumerPoller
);
receiver!(
    #[must_use]
    MpscReceiver,
    MultiProducerSequencer,
    SingleConsumerPoller
);
receiver!(
    #[must_use]
    MpmcReceiver,
    MultiProducerSequencer,
    MultiConsumerPoller
);

impl_clone!(MpscSender, MpmcSender, SpmcReceiver, MpmcReceiver);

channel_factory!(
    spsc,
    SpscSender,
    SpscReceiver,
    SingleProducerSequencer,
    SingleConsumerPoller,
);

channel_factory!(
    spmc,
    SpmcSender,
    SpmcReceiver,
    SingleProducerSequencer,
    MultiConsumerPoller,
);

channel_factory!(
    mpsc,
    MpscSender,
    MpscReceiver,
    MultiProducerSequencer,
    SingleConsumerPoller,
);

channel_factory!(
    mpmc,
    MpmcSender,
    MpmcReceiver,
    MultiProducerSequencer,
    MultiConsumerPoller,
);
