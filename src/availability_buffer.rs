use crate::{constants, utils};
use std::mem::MaybeUninit;
use std::sync::atomic::{fence, AtomicI32, Ordering};

pub struct AvailabilityBuffer {
    mask: i64,
    flag_shift: usize,
    buffer: Box<[AtomicI32]>,
}

unsafe impl Sync for AvailabilityBuffer {}

unsafe impl Send for AvailabilityBuffer {}

impl AvailabilityBuffer {
    pub fn new(buffer_size: usize) -> Self {
        Self {
            mask: (buffer_size - 1) as i64,
            flag_shift: buffer_size.ilog2() as usize,
            buffer: Self::init_buffer(buffer_size),
        }
    }

    fn init_buffer(size: usize) -> Box<[AtomicI32]> {
        let mut buffer: Box<[MaybeUninit<AtomicI32>]> = Box::new_uninit_slice(size + (constants::ARRAY_PADDING << 1));
        for i in 0..size {
            buffer[i + constants::ARRAY_PADDING].write(AtomicI32::new(-1));
        }
        unsafe { buffer.assume_init() }
    }

    #[inline(always)]
    fn calculate_flag(&self, sequence: i64) -> i32 {
        (sequence >> self.flag_shift) as i32
    }

    #[inline(always)]
    pub fn get_available(&self, low: i64, high: i64) -> i64 {
        fence(Ordering::Acquire);
        for sequence in low..=high {
            let index = utils::wrap_index(sequence, self.mask, constants::ARRAY_PADDING);
            let flag = self.calculate_flag(sequence);
            let atomic = &self.buffer[index];
            if atomic.load(Ordering::Relaxed) != flag {
                return sequence - 1;
            }
        }
        high
    }
    
    pub fn set(&self, sequence: i64) {
        let index = utils::wrap_index(sequence, self.mask, constants::ARRAY_PADDING);
        let flag = self.calculate_flag(sequence);
        let atomic = &self.buffer[index];
        atomic.store(flag, Ordering::Release);
    }
    
    pub fn set_range(&self, low: i64, high: i64) {
        for sequence in low..=high {
            let index = utils::wrap_index(sequence, self.mask, constants::ARRAY_PADDING);
            let flag = self.calculate_flag(sequence);
            let atomic = &self.buffer[index];
            atomic.store(flag, Ordering::Relaxed);
        }
        fence(Ordering::Release);
    }
}
