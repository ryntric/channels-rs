use crate::buffer::Buffer;
use crate::sequence::INITIAL_VALUE;

use std::sync::atomic::AtomicI64;
use std::sync::atomic::Ordering::{Acquire, Release};

pub(crate) struct AvailabilityBuffer {
    shift: u32,
    buffer: Buffer<AtomicI64>,
}

impl AvailabilityBuffer {
    pub fn new(capacity: usize) -> Self {
        let buffer = Buffer::with_default(capacity, || AtomicI64::new(INITIAL_VALUE));
        let shift = capacity.ilog2();
        Self { buffer, shift }
    }

    #[inline(always)]
    fn flag(&self, sequence: i64) -> i64 {
        sequence >> self.shift
    }

    pub fn get_available(&self, low: i64, high: i64) -> i64 {
        for sequence in low..=high {
            let slot = unsafe { self.buffer.get(sequence) };
            if slot.load(Acquire) != self.flag(sequence) {
                return sequence - 1;
            }
        }
        high
    }

    pub fn set(&self, sequence: i64) {
        let slot = unsafe { self.buffer.get(sequence) };
        slot.store(self.flag(sequence), Release);
    }

    pub fn set_range(&self, low: i64, high: i64) {
        for sequence in low..=high {
            self.set(sequence);
        }
    }
}

// #[cfg(test)]
// mod tests {
//     use crate::availability_buffer::AvailabilityBuffer;
//     use crate::constants;
//     use std::sync::atomic::Ordering::Relaxed;
//
//     #[test]
//     pub fn test_size_with_paddings() {
//         let buffer_size = 8;
//         let availability_buffer = AvailabilityBuffer::new(buffer_size);
//         let buffer = availability_buffer.buffer;
//         assert_eq!(buffer.len(), buffer_size + (constants::ARRAY_PADDING << 1));
//     }
//
//     #[test]
//     pub fn test_buffer_is_fully_initialized() {
//         let availability_buffer = AvailabilityBuffer::new(8);
//         let buffer = availability_buffer.buffer;
//         for x in buffer {
//             assert_eq!(x.load(Relaxed), -1)
//         }
//     }
// }
