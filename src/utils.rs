pub fn wrap_index(sequence: i64, mask: i64) -> usize {
    (sequence & mask) as usize
}
