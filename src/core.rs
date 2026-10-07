use crate::cache::Cache;
use crate::claim::ClaimEndpoint;
use crate::cursor::{ConsumerCursor, ProducerCursor};
use crate::reservation::Reservation;
use crate::ring_buffer::RingBuffer;
use std::sync::Arc;

pub type Result<T> = std::result::Result<(), T>;

pub(crate) struct ChannelCore<T, P, C> {
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
    pub fn take(&self, sequence: i64) -> T {
        self.ring_buffer.take(sequence)
    }

    #[inline(always)]
    pub fn write(&self, sequence: i64, value: T) {
        self.ring_buffer.write(sequence, value);
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
    pub fn publish(&self, sequence: i64) {
        self.producer_cursor.publish(sequence);
    }

    #[inline(always)]
    pub fn publish_range(&self, low: i64, high: i64) {
        self.producer_cursor.publish_range(low, high);
    }

    #[inline(always)]
    pub fn complete(&self, sequence: i64) {
        self.consumer_cursor.complete(sequence);
    }

    #[inline(always)]
    pub fn complete_range(&self, low: i64, high: i64) {
        self.consumer_cursor.complete_range(low, high);
    }
}

pub(crate) struct Sender<T, K, R, P, C> {
    cache: K,
    reservation: R,
    core: Arc<ChannelCore<T, P, C>>,
}

pub(crate) struct Receiver<T, K, R, P, C> {
    cache: K,
    reservation: R,
    core: Arc<ChannelCore<T, P, C>>,
}

impl<T, K, R, P, C> ClaimEndpoint for Sender<T, K, R, P, C>
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
    fn required(&self, next: i64) -> i64 {
        next - self.core.capacity()
    }
}

impl<T, K, R, P, C> ClaimEndpoint for Receiver<T, K, R, P, C>
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
    fn required(&self, next: i64) -> i64 {
        next
    }
}

impl<T, K, R, P, C> Sender<T, K, R, P, C>
where
    K: Cache,
    R: Reservation,
    P: ProducerCursor,
    C: ConsumerCursor,
{
    #[inline(always)]
    pub fn try_push(&self, value: T) -> Result<T> {
        let Some(sequence) = self.try_claim_n(1) else {
            return Err(value);
        };

        self.core.write(sequence, value);
        self.core.publish(sequence);

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

        let Some(sequence) = self.try_claim_n(length) else {
            return Err(items);
        };

        let high = sequence;
        let low = high - (length - 1);

        for (sequence, element) in (low..=high).zip(items) {
            self.core.write(sequence, element);
        }

        self.core.publish_range(low, high);

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
    pub fn try_push_array(&self, items: Box<[T]>) -> Result<Box<[T]>> {
        let length = items.len() as i64;
        self.core.assert_claim_size(length);

        let Some(sequence) = self.try_claim_n(length) else {
            return Err(items);
        };

        let high = sequence;
        let low = high - (length - 1);

        for (sequence, element) in (low..=high).zip(items) {
            self.core.write(sequence, element);
        }

        self.core.publish_range(low, high);

        Ok(())
    }

    #[inline]
    pub fn push_array(&self, mut items: Box<[T]>) {
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

impl<T, K, R, P, C> Receiver<T, K, R, P, C>
where
    K: Cache,
    R: Reservation,
    P: ProducerCursor,
    C: ConsumerCursor,
{
    #[inline]
    pub fn try_take(&self) -> Option<T> {
        let sequence = self.try_claim_n(1)?;
        let value = self.core.take(sequence);
        self.core.complete(sequence);
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
        let length = (buffer.capacity() - buffer.len()) as i64;
        self.core.assert_claim_size(length);
        
        let available = self.try_claim_n(length)?;

        let high = std::cmp::min(, );
        let low = high - (length - 1);
        for i in low..=high {
            buffer.push(self.core.take(i));
        }
        self.core.complete_range(low, high);
        Some(())
    }

    #[inline]
    pub fn take_batch(&self, buffer: &mut Vec<T>) {
        while let None = self.try_take_batch(buffer) {
            std::hint::spin_loop();
        }
    }
}
