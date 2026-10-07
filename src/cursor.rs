use crate::availability::AvailabilityBuffer;
use crate::reservation::{Reservation, SharedReservation};
use crate::sequence::SharedSequence;
use std::sync::atomic::Ordering::{Acquire, Release};

pub(crate) trait Cursor: Send + Sync {
    fn upper_bound(&self) -> i64;

    fn highest_available(&self, low: i64, high: i64) -> i64;
}

pub(crate) trait ProducerCursor: Cursor {
    fn publish(&self, sequence: i64);

    fn publish_range(&self, low: i64, high: i64);
}

pub(crate) trait ConsumerCursor: Cursor {
    fn complete(&self, sequence: i64);

    fn complete_range(&self, low: i64, high: i64);
}

pub(crate) struct TrackedCursor {
    reservation: SharedReservation,
    availability: AvailabilityBuffer,
}

impl TrackedCursor {
    pub fn new(capacity: usize, reservation: SharedReservation) -> Self {
        Self {
            reservation,
            availability: AvailabilityBuffer::new(capacity),
        }
    }
}

impl Cursor for TrackedCursor {
    #[inline(always)]
    fn upper_bound(&self) -> i64 {
        self.reservation.reserved()
    }

    #[inline(always)]
    fn highest_available(&self, low: i64, high: i64) -> i64 {
        self.availability.get_available(low, high)
    }
}

#[derive(Default)]
pub(crate) struct DirectCursor {
    committed: SharedSequence,
}

impl Cursor for DirectCursor {
    #[inline(always)]
    fn upper_bound(&self) -> i64 {
        self.committed.load(Acquire)
    }

    #[inline(always)]
    fn highest_available(&self, _: i64, high: i64) -> i64 {
        high
    }
}

impl ProducerCursor for DirectCursor {
    #[inline(always)]
    fn publish(&self, sequence: i64) {
        self.committed.store(sequence, Release);
    }

    #[inline(always)]
    fn publish_range(&self, _: i64, high: i64) {
        self.publish(high);
    }
}

impl ConsumerCursor for DirectCursor {
    #[inline(always)]
    fn complete(&self, sequence: i64) {
        self.committed.store(sequence, Release);
    }

    #[inline(always)]
    fn complete_range(&self, _: i64, high: i64) {
        self.complete(high);
    }
}

impl ProducerCursor for TrackedCursor {
    #[inline(always)]
    fn publish(&self, sequence: i64) {
        self.availability.set(sequence);
    }

    #[inline(always)]
    fn publish_range(&self, low: i64, high: i64) {
        self.availability.set_range(low, high);
    }
}

impl ConsumerCursor for TrackedCursor {
    #[inline(always)]
    fn complete(&self, sequence: i64) {
        self.availability.set(sequence);
    }

    #[inline(always)]
    fn complete_range(&self, low: i64, high: i64) {
        self.availability.set_range(low, high);
    }
}
