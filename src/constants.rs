pub const CACHE_LINE_SIZE: usize = 64;
pub const POINTER_SIZE: usize = size_of::<*const u8>();
pub const ARRAY_PADDING: usize = CACHE_LINE_SIZE / POINTER_SIZE;
