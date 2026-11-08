use crate::cursor::LogicalRange;
use crate::sequence::{LocalSequence, SharedSequence};
use alloc::sync::Arc;
use core::sync::atomic::Ordering::Relaxed;

pub(crate) type ReservationResult = Result<LogicalRange, i64>;

pub(crate) trait Reservation {
    fn reserved(&self) -> i64;

    fn try_reserve(&self, expected: i64, next: i64) -> ReservationResult;
}

#[derive(Default)]
pub(crate) struct LocalReservation {
    inner: LocalSequence,
}

impl Reservation for LocalReservation {
    #[inline(always)]
    fn reserved(&self) -> i64 {
        self.inner.get()
    }

    #[inline(always)]
    fn try_reserve(&self, expected: i64, next: i64) -> ReservationResult {
        self.inner.set(next);
        Ok(LogicalRange::new(expected + 1, next))
    }
}

#[derive(Default, Clone)]
pub(crate) struct SharedReservation {
    inner: Arc<SharedSequence>,
}

impl Reservation for SharedReservation {
    #[inline(always)]
    fn reserved(&self) -> i64 {
        self.inner.load(Relaxed)
    }

    #[inline(always)]
    fn try_reserve(&self, expected: i64, next: i64) -> ReservationResult {
        self.inner
            .compare_exchange_weak(expected, next, Relaxed, Relaxed)
            .map(|claimed| LogicalRange::new(claimed + 1, next))
    }
}
