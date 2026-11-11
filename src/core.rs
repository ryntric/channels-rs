use crate::cache::Cache;
use crate::claim::ClaimResult::Unavailable;
use crate::claim::{AvailabilityResult, ClaimEndpoint, ClaimResult};
use crate::cursor::{ConsumerCursor, LogicalRange, ProducerCursor};
use crate::lease::{GenericReadLease, GenericWriteLease, ReadLease, WriteLease};
use crate::reservation::Reservation;
use crate::ring_buffer::RingBuffer;

use crate::buffer::RawParts;
use alloc::sync::Arc;
use core::mem::MaybeUninit;

pub type Result<T> = core::result::Result<(), T>;

pub(crate) struct ChannelCore<T, P, C>
where
    P: ProducerCursor,
    C: ConsumerCursor,
{
    producer_cursor: P,
    consumer_cursor: C,
    ring_buffer: RingBuffer<T>,
}

impl<T, P, C> ChannelCore<T, P, C>
where
    P: ProducerCursor,
    C: ConsumerCursor,
{
    pub fn new(ring_buffer: RingBuffer<T>, producer_cursor: P, consumer_cursor: C) -> Self {
        Self {
            producer_cursor,
            consumer_cursor,
            ring_buffer,
        }
    }

    #[inline(always)]
    pub fn assert_claim_size(&self, n: i64) {
        let capacity = self.ring_buffer.capacity();

        assert!(
            n > 0 && n <= capacity,
            "claim size must be in 1..={capacity}, got {n}"
        );
    }

    #[inline(always)]
    pub unsafe fn take(&self, sequence: i64) -> T {
        unsafe { self.ring_buffer.take(sequence) }
    }

    #[inline(always)]
    pub unsafe fn write(&self, sequence: i64, value: T) {
        unsafe { self.ring_buffer.write(sequence, value) }
    }

    #[inline(always)]
    pub unsafe fn raw_parts(&self, range: &LogicalRange) -> RawParts<'_, T> {
        unsafe { self.ring_buffer.raw_parts(range) }
    }

    #[inline(always)]
    pub unsafe fn get_ptr(&self, sequence: i64) -> *mut MaybeUninit<T> {
        unsafe { self.ring_buffer.get_ptr(sequence) }
    }

    #[inline(always)]
    pub fn read_lease(&self, range: LogicalRange) -> impl ReadLease<T> {
        GenericReadLease::new(self, range, unsafe { self.raw_parts(&range) })
    }

    #[inline(always)]
    pub fn write_lease(&self, range: LogicalRange) -> impl WriteLease<T> {
        GenericWriteLease::new(self, range, unsafe { self.raw_parts(&range) })
    }

    #[inline(always)]
    pub fn capacity(&self) -> i64 {
        self.ring_buffer.capacity()
    }

    #[inline(always)]
    pub fn producer_cursor(&self) -> &P {
        &self.producer_cursor
    }

    #[inline(always)]
    pub fn consumer_cursor(&self) -> &C {
        &self.consumer_cursor
    }

    #[inline(always)]
    pub fn publish(&self, range: LogicalRange) {
        self.producer_cursor.publish(range);
    }

    #[inline(always)]
    pub fn publish_range(&self, range: LogicalRange) {
        self.producer_cursor.publish_range(range);
    }

    #[inline(always)]
    pub fn complete(&self, range: LogicalRange) {
        self.consumer_cursor.complete(range);
    }

    #[inline(always)]
    pub fn complete_range(&self, range: LogicalRange) {
        self.consumer_cursor.complete_range(range);
    }
}

pub(crate) struct GenericSender<T, K, R, P, C>
where
    P: ProducerCursor,
    C: ConsumerCursor,
{
    cache: K,
    reservation: R,
    core: Arc<ChannelCore<T, P, C>>,
}

pub(crate) struct GenericReceiver<T, K, R, P, C>
where
    P: ProducerCursor,
    C: ConsumerCursor,
{
    cache: K,
    reservation: R,
    core: Arc<ChannelCore<T, P, C>>,
}

impl<T, K, R, P, C> ClaimEndpoint for GenericSender<T, K, R, P, C>
where
    K: Cache,
    R: Reservation,
    P: ProducerCursor,
    C: ConsumerCursor,
{
    type Cache = K;
    type Reservation = R;
    type Gating = C;

    #[inline(always)]
    fn cache(&self) -> &Self::Cache {
        &self.cache
    }

    #[inline(always)]
    fn reservation(&self) -> &Self::Reservation {
        &self.reservation
    }

    #[inline(always)]
    fn gating(&self) -> &Self::Gating {
        self.core.consumer_cursor()
    }

    #[inline(always)]
    fn try_claim(&self, n: i64, current: i64) -> ClaimResult {
        let next = current + n;
        let required = next - self.core.capacity();
        if matches!(
            self.try_ensure_availability(required, required),
            AvailabilityResult::Unavailable
        ) {
            return Unavailable;
        }

        match self.reservation().try_reserve(current, next) {
            Ok(sequence) => ClaimResult::Claimed(sequence),
            Err(actual) => ClaimResult::Contended(actual),
        }
    }
}

impl<T, K, R, P, C> ClaimEndpoint for GenericReceiver<T, K, R, P, C>
where
    K: Cache,
    R: Reservation,
    P: ProducerCursor,
    C: ConsumerCursor,
{
    type Cache = K;
    type Reservation = R;
    type Gating = P;

    #[inline(always)]
    fn cache(&self) -> &Self::Cache {
        &self.cache
    }

    #[inline(always)]
    fn reservation(&self) -> &Self::Reservation {
        &self.reservation
    }

    #[inline(always)]
    fn gating(&self) -> &Self::Gating {
        self.core.producer_cursor()
    }

    #[inline(always)]
    fn try_claim(&self, n: i64, current: i64) -> ClaimResult {
        if matches!(
            self.try_ensure_availability(current + 1, current + n),
            AvailabilityResult::Unavailable
        ) {
            return ClaimResult::Unavailable;
        }

        let cache = self.cache();
        let next = core::cmp::min(current + n, cache.value());

        match self.reservation().try_reserve(current, next) {
            Ok(range) => ClaimResult::Claimed(range),
            Err(actual) => ClaimResult::Contended(actual),
        }
    }
}

impl<T, K, R, P, C> GenericSender<T, K, R, P, C>
where
    K: Cache,
    R: Reservation,
    P: ProducerCursor,
    C: ConsumerCursor,
{
    pub fn new(cache: K, reservation: R, core: Arc<ChannelCore<T, P, C>>) -> Self {
        Self {
            cache,
            reservation,
            core,
        }
    }

    #[inline(always)]
    pub fn core(&self) -> &Arc<ChannelCore<T, P, C>> {
        &self.core
    }

    #[inline(always)]
    pub fn try_push(&self, value: T) -> Result<T> {
        let Some(range) = self.try_claim_n(1) else {
            return Err(value);
        };

        unsafe { self.core.write(range.high(), value) }
        self.core.publish(range);

        Ok(())
    }

    /// Attempts to reserve and initialize one element.
    ///
    /// # Safety
    ///
    /// - The callback must fully initialize the slot.
    /// - The callback must not unwind.
    /// - The slot must not be initialized more than once.
    /// - No references to the slot may escape the callback.
    #[cfg(feature = "emplace")]
    #[inline(always)]
    pub unsafe fn try_emplace<F>(&self, f: F) -> Result<F>
    where
        F: FnOnce(&mut MaybeUninit<T>),
    {
        let Some(range) = self.try_claim_n(1) else {
            return Err(f);
        };

        unsafe { f(&mut *self.core.get_ptr(range.high())) };

        self.core.publish(range);

        Ok(())
    }

    #[inline(always)]
    pub fn push(&self, value: T) {
        let range = self.claim_n(1);

        unsafe { self.core.write(range.high(), value) }
        self.core.publish(range);
    }

    /// Reserves and initialize one element.
    ///
    /// # Safety
    ///
    /// - The callback must fully initialize the slot.
    /// - The callback must not unwind.
    /// - The slot must not be initialized more than once.
    /// - No references to the slot may escape the callback.
    #[cfg(feature = "emplace")]
    #[inline(always)]
    pub unsafe fn emplace<F>(&self, f: F)
    where
        F: FnOnce(&mut MaybeUninit<T>),
    {
        let range = self.claim_n(1);

        unsafe { f(&mut *self.core.get_ptr(range.high())) };

        self.core.publish(range);
    }

    fn try_lease_n(&self, n: i64) -> Option<impl WriteLease<T>> {
        let range = self.try_claim_n(n)?;
        Some(self.core.write_lease(range))
    }

    pub fn try_lease(&self, n: usize) -> Option<impl WriteLease<T>> {
        let n = n as i64;
        self.core.assert_claim_size(n);
        self.try_lease_n(n)
    }

    pub fn lease(&self, n: usize) -> impl WriteLease<T> {
        let n = n as i64;
        self.core.assert_claim_size(n);

        loop {
            if let Some(lease) = self.try_lease_n(n) {
                return lease;
            }
            core::hint::spin_loop();
        }
    }
}

impl<T, K, R, P, C> GenericReceiver<T, K, R, P, C>
where
    K: Cache,
    R: Reservation,
    P: ProducerCursor,
    C: ConsumerCursor,
{
    pub fn new(cache: K, reservation: R, core: Arc<ChannelCore<T, P, C>>) -> Self {
        Self {
            cache,
            reservation,
            core,
        }
    }

    #[inline(always)]
    pub fn core(&self) -> &Arc<ChannelCore<T, P, C>> {
        &self.core
    }

    #[inline(always)]
    pub fn try_take(&self) -> Option<T> {
        let range = self.try_claim_n(1)?;
        let value = unsafe { self.core.take(range.high()) };
        self.core.complete(range);
        Some(value)
    }

    #[inline(always)]
    pub fn take(&self) -> T {
        let range = self.claim_n(1);
        let value = unsafe { self.core.take(range.high()) };
        self.core.complete(range);
        value
    }

    #[inline(always)]
    pub fn try_lease(&self, n: usize) -> Option<impl ReadLease<T>> {
        let n = n as i64;
        self.core.assert_claim_size(n);

        let range = self.try_claim_n(n)?;
        Some(self.core.read_lease(range))
    }

    #[inline(always)]
    pub fn lease(&self, n: usize) -> impl ReadLease<T> {
        let n = n as i64;
        self.core.assert_claim_size(n);

        let range = self.claim_n(n);
        self.core.read_lease(range)
    }
}
