use crate::sequence::{LocalSequence, SharedSequence};
use std::sync::Arc;
use std::sync::atomic::Ordering::Relaxed;

pub(crate) type ReservationResult = Result<i64, i64>;

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
    fn try_reserve(&self, _: i64, next: i64) -> ReservationResult {
        self.inner.set(next);
        Ok(next)
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
            .map(|_| next)
    }
}
