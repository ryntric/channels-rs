use std::sync::atomic::{AtomicI64, Ordering};

/// Initial value for a [`Sequence`] when uninitialized.
pub const INITIAL_VALUE: i64 = -1;

/// A sequence counter for coordinating producers and consumers in concurrent data structures.
///
/// `Sequence` wraps an [`AtomicI64`] and provides atomic operations with
/// configurable memory ordering. It is used to track **cursor positions**,
/// **gating sequences**.
///
/// The struct is aligned to 64 bytes to avoid false sharing between threads.
#[repr(align(64))]
pub struct Sequence {
    sequence: AtomicI64,
}

// SAFETY: Sequence is thread-safe due to internal atomic operations.
unsafe impl Sync for Sequence {}

unsafe impl Send for Sequence {}

impl Sequence {
    /// Create a new sequence initialized to `value`.
    pub fn new(value: i64) -> Self {
        Sequence {
            sequence: AtomicI64::new(value),
        }
    }

    /// Get the current value with **Relaxed** memory ordering.
    pub fn get_relaxed(&self) -> i64 {
        self.sequence.load(Ordering::Relaxed)
    }

    /// Set the value with **Relaxed** memory ordering.
    pub fn set_relaxed(&self, value: i64) {
        self.sequence.store(value, Ordering::Relaxed);
    }

    /// Get the current value with **Acquire** memory ordering.
    ///
    /// Ensures that subsequent reads cannot be reordered before this load.
    pub fn get_acquire(&self) -> i64 {
        self.sequence.load(Ordering::Acquire)
    }

    /// Set the value with **Release** memory ordering.
    ///
    /// Ensures that previous writes cannot be reordered after this store
    pub fn set_release(&self, value: i64) {
        self.sequence.store(value, Ordering::Release);
    }

    /// Atomically add `value` to the current sequence using **AcqRel** ordering.
    ///
    /// Returns the previous value before addition.
    pub fn fetch_add_volatile(&self, value: i64) -> i64 {
        self.sequence.fetch_add(value, Ordering::AcqRel)
    }

    /// Perform a weak compare-and-swap operation with **AcqRel** for success
    /// and **Relaxed** for failure.
    ///
    /// Returns `true` if the exchange was successful.
    pub fn compare_and_exchange_weak_volatile(&self, current: i64, new: i64) -> bool {
        self.sequence
            .compare_exchange_weak(current, new, Ordering::AcqRel, Ordering::Relaxed)
            .is_ok()
    }
}

impl Default for Sequence {
    /// Create a default sequence initialized to [`INITIAL_VALUE`].
    fn default() -> Self {
        Sequence::new(INITIAL_VALUE)
    }
}

#[cfg(test)]
mod tests {
    use crate::sequence::Sequence;
    use loom::sync::Arc;

    #[test]
    pub fn test_default_sequence_value() {
        let sequence = Sequence::default();
        assert_eq!(sequence.get_relaxed(), -1);
    }


    #[test]
    fn test_set_and_get_relaxed() {
        loom::model(|| {
            let sequence= Arc::new(Sequence::default());
            let cloned = sequence.clone();

            loom::thread::spawn(move || {
                cloned.set_relaxed(1);
            });

            let value = sequence.get_relaxed();
            assert!(value == -1 || value == 1);
        })
    }

    #[test]
    fn test_set_and_get_rls_acq() {
        loom::model(|| {
            let sequence= Arc::new(Sequence::default());
            let cloned = sequence.clone();

            loom::thread::spawn(move || {
                cloned.set_release(1);
            });

            let value = sequence.get_acquire();
            assert!(value == -1 || value == 1);
        })
    }

    #[test]
    fn test_fetch_add_volatile() {
        loom::model(|| {
            let sequence= Arc::new(Sequence::default());
            let cloned = sequence.clone();

            loom::thread::spawn(move || {
                cloned.fetch_add_volatile(1);
            });

            let value = sequence.get_acquire();
            assert!(value == -1 || value == 1);
        })
    }

    #[test]
    fn test_compare_and_exchange_weak_volatile() {
        loom::model(|| {
            let sequence= Arc::new(Sequence::default());
            let cloned = sequence.clone();

            loom::thread::spawn(move || {

                loop {
                    if cloned.compare_and_exchange_weak_volatile(-1, 1) {
                        break;
                    }
                }
            });

            let value = sequence.get_acquire();
            assert!(value == -1 || value == 1);
        })
    }
}
