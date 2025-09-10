use crate::{constants, utils};
use std::mem::MaybeUninit;
use std::sync::atomic::{fence, AtomicI32, Ordering};

pub(crate) struct AvailabilityBuffer {
    mask: i64,
    flag_shift: usize,
    buffer: Box<[AtomicI32]>,
}

impl AvailabilityBuffer {
    pub fn new(buffer_size: usize) -> Self {
        Self {
            mask: (buffer_size - 1) as i64,
            flag_shift: utils::log2(buffer_size) as usize,
            buffer: Self::init_buffer(buffer_size),
        }
    }

    fn init_buffer(size: usize) -> Box<[AtomicI32]> {
        let mut buffer: Box<[MaybeUninit<AtomicI32>]> = Box::new_uninit_slice(size + (constants::BUFFER_PADDING << 1));
        for i in 0..size {
            buffer[i + constants::BUFFER_PADDING].write(AtomicI32::new(-1));
        }
        unsafe { buffer.assume_init() }
    }

    #[inline(always)]
    fn calculate_flag(&self, sequence: i64) -> i32 {
        (sequence >> self.flag_shift) as i32
    }

    #[inline(always)]
    pub fn is_available(&self, sequence: i64) -> bool {
        let index = utils::wrap_index(sequence, self.mask, constants::BUFFER_PADDING);
        let flag = self.calculate_flag(sequence);
        let atomic = &self.buffer[index];
        atomic.load(Ordering::Acquire) == flag
    }

    #[inline(always)]
    pub fn set(&self, sequence: i64) {
        let index = utils::wrap_index(sequence, self.mask, constants::BUFFER_PADDING);
        let flag = self.calculate_flag(sequence);
        let atomic = &self.buffer[index];
        atomic.store(flag, Ordering::Release);
    }

    #[inline(always)]
    pub fn set_range(&self, low: i64, high: i64) {
        for sequence in low..=high {
            let index = utils::wrap_index(sequence, self.mask, constants::BUFFER_PADDING);
            let flag = self.calculate_flag(sequence);
            let atomic = &self.buffer[index];
            atomic.store(flag, Ordering::Relaxed);
        }
        fence(Ordering::Release);
    }
}
