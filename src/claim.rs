use crate::cache::Cache;
use crate::cursor::{Cursor, Range};
use crate::reservation::Reservation;

#[derive(Clone, Copy, Debug)]
pub(crate) enum ClaimResult {
    Claimed(Range),
    Contended(i64),
    Unavailable,
}

#[derive(Clone, Copy, Debug)]
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

    fn try_ensure_availability(&self, required: i64) -> AvailabilityResult {
        let cache = self.cache();
        let cached = cache.value();

        if required <= cached {
            return AvailabilityResult::Available;
        }

        if let Some(available) = self.try_available(cached + 1, required, self.gating()) {
            cache.set_value(available);
            return AvailabilityResult::Available;
        }

        AvailabilityResult::Unavailable
    }

    fn try_claim(&self, n: i64, current: i64) -> ClaimResult;

    #[inline(always)]
    fn try_available(&self, low: i64, required: i64, gating: &Self::Gating) -> Option<i64> {
        let high = gating.upper_bound();
        if required > high {
            return None;
        }

        let available = gating.highest_available(low, high);
        if required > available {
            return None;
        }

        Some(available)
    }

    #[inline(always)]
    fn try_claim_n(&self, n: i64) -> Option<Range> {
        let reservation = self.reservation();
        let mut current = reservation.reserved();

        loop {
            match self.try_claim(n, current) {
                ClaimResult::Claimed(range) => {
                    return Some(range);
                }
                ClaimResult::Contended(actual) => {
                    current = actual;
                }
                ClaimResult::Unavailable => {
                    return None;
                }
            }
        }
    }
}
