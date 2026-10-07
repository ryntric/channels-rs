use crate::sequence::LocalSequence;

pub(crate) trait Cache {
    fn cached(&self) -> i64;

    fn set_cached(&self, value: i64);
}

#[derive(Default)]
pub(crate) struct LocalCache {
    inner: LocalSequence,
}

impl Clone for LocalCache {
    #[inline]
    fn clone(&self) -> Self {
        Self::default()
    }
}

impl Cache for LocalCache {
    #[inline(always)]
    fn cached(&self) -> i64 {
        self.inner.get()
    }

    #[inline(always)]
    fn set_cached(&self, value: i64) {
        self.inner.set(value)
    }
}
