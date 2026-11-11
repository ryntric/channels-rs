use crate::cache::Cache;
use crate::cursor::{Cursor, LogicalRange};
use crate::reservation::Reservation;

#[derive(Clone, Copy, Debug)]
pub(crate) enum ClaimResult {
    Claimed(LogicalRange),
    Contended(i64),
    Unavailable,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum AvailabilityResult {
    Available,
    Unavailable,
}

pub(crate) trait ClaimEndpoint {
    type Cache: Cache;
    type Reservation: Reservation;
    type Gating: Cursor;

    fn cache(&self) -> &Self::Cache;

    fn reservation(&self) -> &Self::Reservation;

    fn gating(&self) -> &Self::Gating;

    #[inline(always)]
    fn try_ensure_availability(&self, required: i64, limit: i64) -> AvailabilityResult {
        let cache = self.cache();
        let cached = cache.value();

        if required <= cached {
            return AvailabilityResult::Available;
        }

        let available = self.try_available(cached + 1, limit, self.gating());
        if available > cached {
            cache.set_value(available);
        }

        if required <= available {
            AvailabilityResult::Available
        } else {
            AvailabilityResult::Unavailable
        }
    }

    fn try_claim(&self, n: i64, current: i64) -> ClaimResult;

    #[inline(always)]
    fn try_available(&self, low: i64, limit: i64, gating: &Self::Gating) -> i64 {
        let high = gating.upper_bound();
        if high < low {
            return low - 1;
        }

        gating.highest_available(low, high, limit)
    }

    #[inline(always)]
    fn try_claim_n(&self, n: i64) -> Option<LogicalRange> {
        let reservation = self.reservation();
        let mut current = reservation.reserved();

        loop {
            match self.try_claim(n, current) {
                ClaimResult::Claimed(range) => {
                    return Some(range);
                }
                ClaimResult::Contended(actual) => {
                    current = actual;
                    std::thread::yield_now();
                }
                ClaimResult::Unavailable => {
                    return None;
                }
            }
        }
    }

    #[inline(always)]
    fn claim_n(&self, n: i64) -> LogicalRange {
        loop {
            if let Some(range) = self.try_claim_n(n) {
                break range;
            }
            std::thread::yield_now();
        }
    }
}
