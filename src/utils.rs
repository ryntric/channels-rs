#[inline(always)]
pub fn wrap_index(sequence: i64, mask: i64, padding: usize) -> usize {
    (sequence & mask) as usize + padding
}

pub fn assert_buffer_size_pow_of_2(buffer_size: usize) -> usize {
    assert!(buffer_size.is_power_of_two(), "buffer_size must be a power of two");
    buffer_size
}
