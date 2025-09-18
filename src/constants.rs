pub(crate) const CACHE_LINE_SIZE: usize = 64;
pub(crate) const POINTER_SIZE: usize = size_of::<*const u8>();
pub(crate) const ARRAY_PADDING: usize = CACHE_LINE_SIZE / POINTER_SIZE;
