/// Typical CPU cache line size in bytes.
///
/// Most modern CPUs have a cache line of 64 bytes.
pub const CACHE_LINE_SIZE: usize = 64;

/// Size of a raw pointer on the target architecture in bytes.
///
/// On a 64-bit system, this is usually 8 bytes; on a 32-bit system, 4 bytes.
/// This is used for calculating padding or memory layout alignment.
pub const POINTER_SIZE: usize = size_of::<*const u8>();

/// Number of pointer-sized elements that fit into a single cache line.
///
/// This is computed as `CACHE_LINE_SIZE / POINTER_SIZE` and is commonly used
/// to pad arrays or structs to align to cache lines, reducing false sharing
/// between threads in concurrent data structures.
///
/// # Examples
///
/// ```
/// # use channels::constants::ARRAY_PADDING;
/// // On a 64-bit system, ARRAY_PADDING is typically 8
/// assert_eq!(ARRAY_PADDING, 64 / 8);
/// ```
pub const ARRAY_PADDING: usize = CACHE_LINE_SIZE / POINTER_SIZE;
