use crate::cache::Cache;
use crate::claim::{AvailabilityResult, ClaimEndpoint, ClaimResult};
use crate::cursor::{ConsumerCursor, ProducerCursor, Range};
use crate::reservation::Reservation;
use crate::ring_buffer::RingBuffer;

use crate::claim::ClaimResult::Unavailable;
use std::sync::Arc;

pub type Result<T> = std::result::Result<(), T>;

pub(crate) struct ChannelCore<T, P, C>
where
    P: ProducerCursor,
    C: ConsumerCursor,
{
    producer_cursor: P,
    consumer_cursor: C,
    ring_buffer: RingBuffer<T>,
}

impl<T, P, C> Drop for ChannelCore<T, P, C>
where
    P: ProducerCursor,
    C: ConsumerCursor,
{
    fn drop(&mut self) {
        // self.ring_buffer.clear(self.consumer_cursor.)
    }
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
    pub fn take(&self, sequence: i64) -> T {
        unsafe { self.ring_buffer.take(sequence) }
    }

    #[inline(always)]
    pub fn write(&self, sequence: i64, value: T) {
        unsafe { self.ring_buffer.write(sequence, value) }
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
    pub fn publish(&self, range: Range) {
        self.producer_cursor.publish(range);
    }

    #[inline(always)]
    pub fn publish_range(&self, range: Range) {
        self.producer_cursor.publish_range(range);
    }

    #[inline(always)]
    pub fn complete(&self, range: Range) {
        self.consumer_cursor.complete(range);
    }

    #[inline(always)]
    pub fn complete_range(&self, range: Range) {
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
        if matches!(self.try_ensure_availability(next - self.core.capacity()), AvailabilityResult::Unavailable) {
            return Unavailable;
        }

        let reservation = self.reservation();
        match reservation.try_reserve(current, next) {
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
        if matches!(self.try_ensure_availability(current + 1), AvailabilityResult::Unavailable) {
            return ClaimResult::Unavailable;
        }

        let cache = self.cache();
        let next = std::cmp::min(current + n, cache.value());

        let reservation = self.reservation();
        match reservation.try_reserve(current, next) {
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

        self.core.write(range.high(), value);
        self.core.publish(range);

        Ok(())
    }

    #[inline]
    pub fn push(&self, mut value: T) {
        loop {
            match self.try_push(value) {
                Ok(()) => return,
                Err(returned) => {
                    value = returned;
                    std::hint::spin_loop();
                }
            }
        }
    }

    #[inline]
    pub fn try_push_vector(&self, items: Vec<T>) -> Result<Vec<T>> {
        let length = items.len() as i64;
        self.core.assert_claim_size(length);

        let Some(range) = self.try_claim_n(length) else {
            return Err(items);
        };

        for (sequence, element) in range.into_iter().zip(items) {
            self.core.write(sequence, element);
        }

        self.core.publish_range(range);

        Ok(())
    }

    #[inline]
    pub fn push_vector(&self, mut items: Vec<T>) {
        loop {
            match self.try_push_vector(items) {
                Ok(()) => return,
                Err(returned) => {
                    items = returned;
                    std::hint::spin_loop();
                }
            }
        }
    }

    #[inline]
    pub fn try_push_boxed_slice(&self, items: Box<[T]>) -> Result<Box<[T]>> {
        let length = items.len() as i64;
        self.core.assert_claim_size(length);

        let Some(range) = self.try_claim_n(length) else {
            return Err(items);
        };

        for (sequence, element) in range.into_iter().zip(items) {
            self.core.write(sequence, element);
        }

        self.core.publish_range(range);

        Ok(())
    }

    #[inline]
    pub fn try_push_array<const N: usize>(&self, items: [T; N]) -> Result<[T; N]> {
        let length = items.len() as i64;
        self.core.assert_claim_size(length);

        let Some(range) = self.try_claim_n(length) else {
            return Err(items);
        };

        for (sequence, element) in range.into_iter().zip(items) {
            self.core.write(sequence, element);
        }

        self.core.publish_range(range);

        Ok(())
    }

    #[inline]
    pub fn push_boxed_slice(&self, mut items: Box<[T]>) {
        loop {
            match self.try_push_boxed_slice(items) {
                Ok(()) => return,
                Err(returned) => {
                    items = returned;
                    std::hint::spin_loop();
                }
            }
        }
    }

    #[inline]
    pub fn push_array<const N: usize>(&self, mut items: [T; N]) {
        loop {
            match self.try_push_array(items) {
                Ok(()) => return,
                Err(returned) => {
                    items = returned;
                    std::hint::spin_loop();
                }
            }
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

    #[inline]
    pub fn try_take(&self) -> Option<T> {
        let range = self.try_claim_n(1)?;
        let value = self.core.take(range.high());
        self.core.complete(range);
        Some(value)
    }

    #[inline]
    pub fn take(&self) -> T {
        loop {
            if let Some(value) = self.try_take() {
                return value;
            }
            std::hint::spin_loop()
        }
    }

    #[inline]
    pub fn try_take_batch(&self, buffer: &mut Vec<T>) -> Option<()> {
        let length = buffer.capacity() as i64;
        self.core.assert_claim_size(length);

        let range = self.try_claim_n(length)?;
        for i in range {
            buffer.push(self.core.take(i));
        }

        self.core.complete_range(range);

        Some(())
    }

    #[inline]
    pub fn take_batch(&self, buffer: &mut Vec<T>) {
        while self.try_take_batch(buffer).is_none() {
            std::hint::spin_loop();
        }
    }
}
