use std::sync::atomic::{AtomicBool, Ordering};

#[inline(always)]
pub fn wrap_index(sequence: i64, mask: i64, padding: usize) -> usize {
    (sequence & mask) as usize + padding
}

pub fn assert_buffer_size_pow_of_2(buffer_size: usize) -> usize {
    assert!(buffer_size.is_power_of_two(), "buffer_size must be a power of two");
    buffer_size
}

pub fn log2(buffer_size: usize) -> u32 {
    (usize::BITS - 1) - buffer_size.leading_zeros()
}

pub fn compare_and_exchange_bool(value: &AtomicBool, current: bool, new: bool, success: Ordering, failure: Ordering) -> bool {
    value.compare_exchange(current, new, success, failure)
        .is_ok()
}
