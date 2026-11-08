use crate::core::ChannelCore;
use crate::cursor::{ConsumerCursor, ProducerCursor, Range};

pub trait WriteLease<T> {
    fn write(&mut self, value: T);
}

pub trait ReadLease<T>: Iterator {
    fn read(&mut self) -> Option<T>;
}

pub(crate) struct GenericWriteLease<'core, T, P, C>
where
    P: ProducerCursor,
    C: ConsumerCursor,
{
    core: &'core ChannelCore<T, P, C>,
    range: Range,
    cursor: i64,
}

impl<'core, T, P, C> GenericWriteLease<'core, T, P, C>
where
    P: ProducerCursor,
    C: ConsumerCursor,
{
    pub(crate) fn new(core: &'core ChannelCore<T, P, C>, range: Range) -> Self {
        Self {
            core,
            range,
            cursor: range.low(),
        }
    }
}

impl<T, P, C> WriteLease<T> for GenericWriteLease<'_, T, P, C>
where
    P: ProducerCursor,
    C: ConsumerCursor,
{
    #[inline]
    fn write(&mut self, value: T) {
        assert!(self.cursor <= self.range.high());

        // SAFETY: The slot belongs to the reserved range
        // and contains an initialized value.
        unsafe { self.core.write(self.cursor, value) }
        self.cursor += 1;
    }
}

impl<T, P, C> Drop for GenericWriteLease<'_, T, P, C>
where
    P: ProducerCursor,
    C: ConsumerCursor,
{
    fn drop(&mut self) {
        self.core.publish_range(self.range)
    }
}

pub(crate) struct GenericReadLease<'core, T, P, C>
where
    P: ProducerCursor,
    C: ConsumerCursor,
{
    core: &'core ChannelCore<T, P, C>,
    range: Range,
    cursor: i64,
}

impl<'core, T, P, C> GenericReadLease<'core, T, P, C>
where
    P: ProducerCursor,
    C: ConsumerCursor,
{
    pub(crate) fn new(core: &'core ChannelCore<T, P, C>, range: Range) -> Self {
        Self {
            core,
            range,
            cursor: range.low(),
        }
    }
}

impl<T, P, C> ReadLease<T> for GenericReadLease<'_, T, P, C>
where
    P: ProducerCursor,
    C: ConsumerCursor,
{
    #[inline]
    fn read(&mut self) -> Option<T> {
        while self.cursor <= self.range.high() {
            let value = unsafe { self.core.take(self.cursor) };
            self.cursor += 1;

            if value.is_some() {
                return value;
            }
        }
        None
    }
}

impl<T, P, C> Iterator for GenericReadLease<'_, T, P, C>
where
    P: ProducerCursor,
    C: ConsumerCursor,
{
    type Item = T;

    fn next(&mut self) -> Option<Self::Item> {
        self.read()
    }
}

impl<T, P, C> Drop for GenericReadLease<'_, T, P, C>
where
    P: ProducerCursor,
    C: ConsumerCursor,
{
    fn drop(&mut self) {
        for sequence in self.cursor..=self.range.high() {
            drop(unsafe { self.core.take(sequence) });
        }
        self.core.complete_range(self.range);
    }
}
