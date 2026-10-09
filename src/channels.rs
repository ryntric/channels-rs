use crate::cache::LocalCache;
use crate::claim::ClaimEndpoint;
use crate::core::Result;
use crate::core::{ChannelCore, GenericReceiver, GenericSender};
use crate::cursor::{DirectCursor, TrackedCursor};
use crate::reservation::{LocalReservation, SharedReservation};
use crate::ring_buffer::RingBuffer;

use std::sync::Arc;

macro_rules! sender {
    (
        $name:ident, $K:ty, $R:ty, $P:ty, $C:ty
    ) => {
        pub struct $name<T> {
            inner: GenericSender<T, $K, $R, $P, $C>,
        }

        impl<T> $name<T> {
            fn new(cache: $K, reservation: $R, core: Arc<ChannelCore<T, $P, $C>>) -> Self {
                Self {
                    inner: GenericSender::new(cache, reservation, core),
                }
            }

            pub fn try_push(&self, value: T) -> Result<T> {
                self.inner.try_push(value)
            }

            #[inline]
            pub fn push(&self, value: T) {
                self.inner.push(value)
            }

            pub fn try_push_vector(&self, items: Vec<T>) -> Result<Vec<T>> {
                self.inner.try_push_vector(items)
            }

            pub fn push_vector(&self, items: Vec<T>) {
                self.inner.push_vector(items)
            }

            pub fn try_push_boxed_slice(&self, items: Box<[T]>) -> Result<Box<[T]>> {
                self.inner.try_push_boxed_slice(items)
            }

            pub fn push_boxed_slice(&self, items: Box<[T]>) {
                self.inner.push_boxed_slice(items)
            }

            pub fn try_push_array<const N: usize>(&self, items: [T; N]) -> Result<[T; N]> {
                self.inner.try_push_array(items)
            }

            pub fn push_array<const N: usize>(&self, items: [T; N]) {
                self.inner.push_array(items)
            }
        }
    };
}

macro_rules! receiver {
    (
        $name:ident, $K:ty, $R:ty, $P:ty, $C:ty
    ) => {
        pub struct $name<T> {
            inner: GenericReceiver<T, $K, $R, $P, $C>,
        }
        impl<T> $name<T> {
            fn new(cache: $K, reservation: $R, core: Arc<ChannelCore<T, $P, $C>>) -> Self {
                Self {
                    inner: GenericReceiver::new(cache, reservation, core),
                }
            }
            
            pub fn try_take(&self) -> Option<T> {
                self.inner.try_take()
            }
            
            pub fn take(&self) -> T {
                self.inner.take()
            }
            
            pub fn try_take_batch(&self, buffer: &mut Vec<T>) -> Option<()> {
                self.inner.try_take_batch(buffer)
            }

            pub fn take_batch(&self, buffer: &mut Vec<T>) {
                self.inner.take_batch(buffer)
            }
        }
    };
}

macro_rules! impl_clone {
    ($name:ident) => {
        impl<T> Clone for $name<T> {
            fn clone(&self) -> Self {
                Self::new(
                    self.inner.cache().clone(),
                    self.inner.reservation().clone(),
                    Arc::clone(self.inner.core()),
                )
            }
        }
    };
}

sender!(
    SpscSender,
    LocalCache,
    LocalReservation,
    DirectCursor,
    DirectCursor
);
sender!(
    SpmcSender,
    LocalCache,
    LocalReservation,
    DirectCursor,
    TrackedCursor
);
sender!(
    MpscSender,
    LocalCache,
    SharedReservation,
    TrackedCursor,
    DirectCursor
);
sender!(
    MpmcSender,
    LocalCache,
    SharedReservation,
    TrackedCursor,
    TrackedCursor
);

receiver!(
    SpscReceiver,
    LocalCache,
    LocalReservation,
    DirectCursor,
    DirectCursor
);
receiver!(
    SpmcReceiver,
    LocalCache,
    SharedReservation,
    DirectCursor,
    TrackedCursor
);
receiver!(
    MpscReceiver,
    LocalCache,
    LocalReservation,
    TrackedCursor,
    DirectCursor
);
receiver!(
    MpmcReceiver,
    LocalCache,
    SharedReservation,
    TrackedCursor,
    TrackedCursor
);

impl_clone!(SpmcReceiver);
impl_clone!(MpscSender);
impl_clone!(MpmcSender);
impl_clone!(MpmcReceiver);

pub fn spsc<T>(capacity: usize) -> (SpscSender<T>, SpscReceiver<T>) {
    let core = Arc::new(ChannelCore::new(
        RingBuffer::new(capacity),
        DirectCursor::default(),
        DirectCursor::default(),
    ));
    let sender = SpscSender::new(
        LocalCache::default(),
        LocalReservation::default(),
        Arc::clone(&core),
    );
    let receiver = SpscReceiver::new(LocalCache::default(), LocalReservation::default(), core);

    (sender, receiver)
}

pub fn spmc<T>(capacity: usize) -> (SpmcSender<T>, SpmcReceiver<T>) {
    let consumer_reservation = SharedReservation::default();
    let core = Arc::new(ChannelCore::new(
        RingBuffer::new(capacity),
        DirectCursor::default(),
        TrackedCursor::new(capacity, consumer_reservation.clone()),
    ));
    let sender = SpmcSender::new(
        LocalCache::default(),
        LocalReservation::default(),
        Arc::clone(&core),
    );
    let receiver = SpmcReceiver::new(LocalCache::default(), consumer_reservation, core);

    (sender, receiver)
}

pub fn mpsc<T>(capacity: usize) -> (MpscSender<T>, MpscReceiver<T>) {
    let producer_reservation = SharedReservation::default();
    let core = Arc::new(ChannelCore::new(
        RingBuffer::new(capacity),
        TrackedCursor::new(capacity, producer_reservation.clone()),
        DirectCursor::default(),
    ));
    let sender = MpscSender::new(
        LocalCache::default(),
        producer_reservation,
        Arc::clone(&core),
    );
    let receiver = MpscReceiver::new(LocalCache::default(), LocalReservation::default(), core);

    (sender, receiver)
}

pub fn mpmc<T>(capacity: usize) -> (MpmcSender<T>, MpmcReceiver<T>) {
    let producer_reservation = SharedReservation::default();
    let consumer_reservation = SharedReservation::default();
    let core = Arc::new(ChannelCore::new(
        RingBuffer::new(capacity),
        TrackedCursor::new(capacity, producer_reservation.clone()),
        TrackedCursor::new(capacity, consumer_reservation.clone()),
    ));
    let sender = MpmcSender::new(
        LocalCache::default(),
        producer_reservation,
        Arc::clone(&core),
    );
    let receiver = MpmcReceiver::new(LocalCache::default(), consumer_reservation, core);

    (sender, receiver)
}
