use crate::availability::AvailabilityBuffer;
use crate::reservation::{Reservation, SharedReservation};
use crate::sequence::SharedSequence;
use std::ops::RangeInclusive;
use std::sync::atomic::Ordering::{Acquire, Release};

#[derive(Clone, Copy, Debug)]
pub(crate) struct Range {
    low: i64,
    high: i64,
}

pub(crate) trait Cursor: Send + Sync {
    fn upper_bound(&self) -> i64;

    fn highest_available(&self, low: i64, high: i64) -> i64;
}

pub(crate) trait ProducerCursor: Cursor {
    fn publish(&self, range: Range);

    fn publish_range(&self, range: Range);
}

pub(crate) trait ConsumerCursor: Cursor {
    fn complete(&self, range: Range);

    fn complete_range(&self, range: Range);
}

pub(crate) struct TrackedCursor {
    reservation: SharedReservation,
    availability: AvailabilityBuffer,
}

impl Range {
    pub fn new(low: i64, high: i64) -> Self {
        Self { low, high }
    }

    pub fn len(&self) -> usize {
        (self.high - self.low) as usize
    }

    #[inline(always)]
    pub fn low(&self) -> i64 {
        self.low
    }

    #[inline(always)]
    pub fn high(&self) -> i64 {
        self.high
    }
}

impl IntoIterator for Range {
    type Item = i64;
    type IntoIter = RangeInclusive<i64>;

    #[inline(always)]
    fn into_iter(self) -> Self::IntoIter {
        self.low..=self.high
    }
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
    fn publish(&self, range: Range) {
        self.committed.store(range.high(), Release);
    }

    fn publish_range(&self, range: Range) {
        self.publish(range);
    }
}

impl ConsumerCursor for DirectCursor {
    #[inline(always)]
    fn complete(&self, range: Range) {
        self.committed.store(range.high(), Release);
    }

    fn complete_range(&self, range: Range) {
        self.publish(range);
    }
}

impl ProducerCursor for TrackedCursor {
    #[inline(always)]
    fn publish(&self, range: Range) {
        self.availability.set(range.high());
    }

    #[inline(always)]
    fn publish_range(&self, range: Range) {
        self.availability.set_range(range.low(), range.high());
    }
}

impl ConsumerCursor for TrackedCursor {
    #[inline(always)]
    fn complete(&self, range: Range) {
        self.availability.set(range.high());
    }

    #[inline(always)]
    fn complete_range(&self, range: Range) {
        self.availability.set_range(range.low(), range.high());
    }
}
