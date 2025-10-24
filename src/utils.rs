/// Wrap a sequence index to the actual buffer index, taking mask and padding into account.
///
/// This is used in ring buffers to convert a monotonically increasing sequence number
/// into a valid array index, while preserving cache-line padding to avoid false sharing.
///
/// # Parameters
/// - `sequence`: The sequence number to wrap.
/// - `mask`: Typically `buffer_size - 1`, used to wrap the index efficiently (power-of-two buffer).
/// - `padding`: Number of padding slots at the start of the array (cache-line padding).
///
/// # Returns
/// The computed index within the internal buffer slice.
#[inline(always)]
pub fn wrap_index(sequence: i64, mask: i64, padding: usize) -> usize {
    (sequence & mask) as usize + padding
}

/// Assert that a buffer size is a power of two.
///
/// Many ring buffer implementations rely on power-of-two sizes to efficiently
/// compute indices using a mask instead of modulus operations.
///
/// # Parameters
/// - `buffer_size`: The size of the buffer to check.
///
/// # Returns
/// The same buffer size if the assertion passes.
///
/// # Panics
/// Panics if `buffer_size` is not a power of two.
pub fn assert_buffer_size_pow_of_2(buffer_size: usize) {
    assert!(buffer_size.is_power_of_two(), "buffer_size must be a power of two");
}

/// Asserts that a given buffer size fits within the range of an `i64`.
///
/// # Panics
///
/// This function will panic if `buffer_size` is greater than `i64::MAX`
/// (i.e., if the buffer size cannot be represented by a signed 64-bit integer).
///
/// # Arguments
///
/// * `buffer_size` - The size of a buffer, in bytes, to validate.
///
/// # Safety
///
/// This function performs a simple assertion and does not invoke any unsafe behavior.
/// It is intended as a guard before converting `usize` values to `i64`.
pub fn assert_buffer_size_is_equal_or_less_than_i64(buffer_size: usize) {
    assert!(buffer_size <= i64::MAX as usize, "buffer_size must be less than i64::MAX");
}
