use crate::buffer::RawParts;
use crate::core::ChannelCore;
use crate::cursor::{ConsumerCursor, LogicalRange, ProducerCursor};
use std::mem::MaybeUninit;

pub trait WriteLease<T> {
    fn write(&mut self, value: T);

    /// Reserves and initialize one element.
    ///
    /// # Safety
    ///
    /// - The callback must fully initialize the slot.
    /// - The callback must not unwind.
    /// - The slot must not be initialized more than once.
    /// - No references to the slot may escape the callback.
    #[cfg(feature = "emplace")]
    unsafe fn emplace<F>(&mut self, f: F)
    where
        F: FnOnce(&mut MaybeUninit<T>);
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
    range: LogicalRange,
    parts: RawParts<'core, T>,
}

impl<'core, T, P, C> GenericWriteLease<'core, T, P, C>
where
    P: ProducerCursor,
    C: ConsumerCursor,
{
    pub(crate) fn new(
        core: &'core ChannelCore<T, P, C>,
        range: LogicalRange,
        parts: RawParts<'core, T>,
    ) -> Self {
        Self { core, range, parts }
    }
}

impl<T, P, C> WriteLease<T> for GenericWriteLease<'_, T, P, C>
where
    P: ProducerCursor,
    C: ConsumerCursor,
{
    #[inline(always)]
    fn write(&mut self, value: T) {
        if let Some(slot) = self.parts.next() {
            slot.write(value);
        }
    }

    #[cfg(feature = "emplace")]
    #[inline(always)]
    unsafe fn emplace<F>(&mut self, f: F)
    where
        F: FnOnce(&mut MaybeUninit<T>),
    {
        if let Some(slot) = self.parts.next() {
            f(slot);
        }
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
    range: LogicalRange,
    parts: RawParts<'core, T>,
}

impl<'core, T, P, C> GenericReadLease<'core, T, P, C>
where
    P: ProducerCursor,
    C: ConsumerCursor,
{
    pub(crate) fn new(
        core: &'core ChannelCore<T, P, C>,
        range: LogicalRange,
        parts: RawParts<'core, T>,
    ) -> Self {
        Self { core, range, parts }
    }
}

impl<T, P, C> ReadLease<T> for GenericReadLease<'_, T, P, C>
where
    P: ProducerCursor,
    C: ConsumerCursor,
{
    #[inline(always)]
    fn read(&mut self) -> Option<T> {
        self.parts
            .next()
            .map(|slot| unsafe { slot.assume_init_read() })
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
        for slot in self.parts.by_ref() {
            unsafe { slot.assume_init_drop() }
        }
        self.core.complete_range(self.range);
    }
}
