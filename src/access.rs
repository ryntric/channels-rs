use crate::claim::ClaimEndpoint;
use crate::core::ChannelCore;
use crate::cursor::{ConsumerCursor, ProducerCursor, Range};
use core::marker::PhantomData;
use core::ops::RangeInclusive;

pub(crate) struct WriteAccess<'core, T, E, P, C>
where
    E: ClaimEndpoint,
    P: ProducerCursor,
    C: ConsumerCursor,
{
    endpoint: &'core E,
    core: &'core ChannelCore<T, P, C>,
}

impl<'core, T, E, P, C> WriteAccess<'core, T, E, P, C>
where
    E: ClaimEndpoint,
    P: ProducerCursor,
    C: ConsumerCursor,
{
    pub fn new(core: &'core ChannelCore<T, P, C>, endpoint: &'core E) -> Self {
        Self { core, endpoint }
    }

    pub fn try_reserve(&self, n: usize) -> Option<Range> {
        self.endpoint.try_claim_n(n as i64)
    }

    pub fn write(&mut self, sequence: i64, value: T) {
        unsafe { self.core.write(sequence, value) }
    }
}

pub struct WriteReservation<'core> {
    range: Range,
    _marker: PhantomData<&'core ()>,
}

impl<'core> IntoIterator for WriteReservation<'core> {
    type Item = i64;
    type IntoIter = RangeInclusive<i64>;

    fn into_iter(self) -> Self::IntoIter {
        self.range.into_iter()
    }
}
