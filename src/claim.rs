use crate::cache::Cache;
use crate::cursor::Cursor;
use crate::reservation::Reservation;

pub(crate) enum ClaimResult {
    Claimed(i64),
    Contended(i64),
    Unavailable,
}

pub(crate) trait ClaimEndpoint {
    type Cache: Cache;
    type Reservation: Reservation;
    type Gating: Cursor;

    fn cache(&self) -> &Self::Cache;

    fn reservation(&self) -> &Self::Reservation;

    fn gating(&self) -> &Self::Gating;

    fn required(&self, next: i64) -> i64;

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
    fn try_claim(&self, n: i64, current: i64) -> ClaimResult {
        let next = current + n;
        let required = self.required(next);

        let cache = self.cache();
        let cached = cache.cached();

        if required > cached {
            let Some(available) = self.try_available(cached + 1, required, self.gating()) else {
                return ClaimResult::Unavailable;
            };
            cache.set_cached(available);
        }

        let reservation = self.reservation();
        match reservation.try_reserve(current, next) {
            Ok(sequence) => ClaimResult::Claimed(sequence),
            Err(actual) => ClaimResult::Contended(actual),
        }
    }

    #[inline(always)]
    fn try_claim_n(&self, n: i64) -> Option<i64> {
        let reservation = self.reservation();
        let mut current = reservation.reserved();

        loop {
            match self.try_claim(n, current) {
                ClaimResult::Claimed(sequence) => {
                    return Some(sequence);
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
