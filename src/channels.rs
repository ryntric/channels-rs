use crate::cache::LocalCache;
use crate::claim::ClaimEndpoint;
use crate::core::Result;
use crate::core::{ChannelCore, GenericReceiver, GenericSender};
use crate::cursor::{DirectCursor, TrackedCursor};
use crate::lease::{ReadLease, WriteLease};
use crate::reservation::{LocalReservation, SharedReservation};
use crate::ring_buffer::RingBuffer;

use alloc::sync::Arc;

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

            #[inline]
            pub fn try_push(&self, value: T) -> Result<T> {
                self.inner.try_push(value)
            }

            #[inline]
            pub fn push(&self, value: T) {
                self.inner.push(value)
            }

            #[inline]
            pub fn try_lease(&self, n: usize) -> Option<impl WriteLease<T>> {
                self.inner.try_lease(n)
            }

            #[inline]
            pub fn lease(&self, n: usize) -> impl WriteLease<T> {
                self.inner.lease(n)
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

            #[inline]
            pub fn try_take(&self) -> Option<T> {
                self.inner.try_take()
            }

            #[inline]
            pub fn take(&self) -> T {
                self.inner.take()
            }

            #[inline]
            pub fn try_lease(&self, n: usize) -> Option<impl ReadLease<T>> {
                self.inner.try_lease(n)
            }

            #[inline]
            pub fn lease(&self, n: usize) -> impl ReadLease<T> {
                self.inner.lease(n)
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
