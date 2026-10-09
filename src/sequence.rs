use core::cell::Cell;
use core::ops::Deref;
use core::sync::atomic::AtomicI64;

pub(crate) const INITIAL_VALUE: i64 = -1;

#[cfg_attr(
    any(
        target_arch = "x86_64",
        target_arch = "aarch64",
        target_arch = "arm64ec",
        target_arch = "powerpc64",
    ),
    repr(align(128))
)]
#[cfg_attr(
    any(
        target_arch = "arm",
        target_arch = "mips",
        target_arch = "mips64",
        target_arch = "sparc",
        target_arch = "hexagon"
    ),
    repr(align(32))
)]
#[cfg_attr(target_arch = "s390x", repr(align(256)))]
#[cfg_attr(
    not(any(
        target_arch = "x86_64",
        target_arch = "aarch64",
        target_arch = "arm64ec",
        target_arch = "powerpc64",
        target_arch = "arm",
        target_arch = "mips",
        target_arch = "mips64",
        target_arch = "sparc",
        target_arch = "hexagon",
        target_arch = "s390x",
    )),
    repr(align(64))
)]
#[repr(C)]
pub(crate) struct CachePadded<T> {
    value: T,
}

impl<T> CachePadded<T> {
    pub fn new(value: T) -> Self {
        Self { value }
    }
}

impl<T> Deref for CachePadded<T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        &self.value
    }
}

#[repr(transparent)]
pub(crate) struct LocalSequence {
    inner: CachePadded<Cell<i64>>,
}

impl Deref for LocalSequence {
    type Target = Cell<i64>;

    fn deref(&self) -> &Self::Target {
        &self.inner
    }
}

impl Default for LocalSequence {
    fn default() -> Self {
        Self {
            inner: CachePadded::new(Cell::new(INITIAL_VALUE)),
        }
    }
}

#[repr(transparent)]
pub(crate) struct SharedSequence {
    inner: CachePadded<AtomicI64>,
}

impl Deref for SharedSequence {
    type Target = AtomicI64;

    fn deref(&self) -> &Self::Target {
        &self.inner
    }
}

impl Default for SharedSequence {
    fn default() -> Self {
        Self {
            inner: CachePadded::new(AtomicI64::new(INITIAL_VALUE)),
        }
    }
}
