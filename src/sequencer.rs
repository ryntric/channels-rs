use crate::availability_buffer::AvailabilityBuffer;
use crate::sequence::Sequence;
use crate::utils;

pub trait Sequencer: Sync + Send {

    fn next(&self) -> i64 {
        self.next_n(1)
    }

    fn next_n(&self, n: usize) -> i64;

    fn publish_cursor_sequence(&self, sequence: i64);

    fn publish_cursor_sequence_range(&self, low: i64, high: i64);

    fn publish_gating_sequence(&self, sequence: i64);

    fn get_highest(&self, low: i64, high: i64) -> i64;

    fn get_cursor_sequence_relaxed(&self) -> i64;

    fn get_cursor_sequence_acquire(&self) -> i64;

    fn get_gating_sequence_relaxed(&self) -> i64;

    fn get_gating_sequence_acquire(&self) -> i64;

    #[inline(always)]
    fn wait(&self, gating_sequence: &Sequence, wrap_point: i64) -> i64 {
        let mut gating: i64;
        loop {
            gating = gating_sequence.get_acquire();
            if wrap_point > gating {
                std::hint::spin_loop();
                continue;
            }
            return gating;
        }
    }
}

pub struct SingleProducerSequencer {
    sequence: Sequence,
    cached: Sequence,
    buffer_size: i64,
    cursor_sequence: Sequence,
    gating_sequence: Sequence,
}

impl SingleProducerSequencer {
    pub fn new(buffer_size: usize) -> Self {
        Self {
            sequence: Sequence::default(),
            cached: Sequence::default(),
            buffer_size: utils::assert_buffer_size_pow_of_2(buffer_size) as i64,
            cursor_sequence: Sequence::default(),
            gating_sequence: Sequence::default(),
        }
    }
}

impl Sequencer for SingleProducerSequencer {
    fn next_n(&self, n: usize) -> i64 {
        let next: i64 = self.sequence.get_relaxed() + n as i64;
        let wrap_point: i64 = next - self.buffer_size;

        if wrap_point > self.cached.get_relaxed() {
            self.cached.set_relaxed(self.wait(&self.gating_sequence, wrap_point));
        }

        self.sequence.set_relaxed(next);
        next
    }

    fn publish_cursor_sequence(&self, sequence: i64) {
        self.cursor_sequence.set_release(sequence);
    }

    fn publish_cursor_sequence_range(&self, _: i64, high: i64) {
        self.cursor_sequence.set_release(high)
    }

    fn publish_gating_sequence(&self, sequence: i64) {
        self.gating_sequence.set_release(sequence);
    }

    fn get_highest(&self, _: i64, high: i64) -> i64 {
        high
    }

    fn get_cursor_sequence_relaxed(&self) -> i64 {
        self.cursor_sequence.get_relaxed()
    }

    fn get_cursor_sequence_acquire(&self) -> i64 {
        self.cursor_sequence.get_acquire()
    }

    fn get_gating_sequence_relaxed(&self) -> i64 {
        self.gating_sequence.get_relaxed()
    }

    fn get_gating_sequence_acquire(&self) -> i64 {
        self.gating_sequence.get_relaxed()
    }
}

pub struct MultiProducerSequencer {
    buffer_size: i64,
    cached: Sequence,
    cursor_sequence: Sequence,
    gating_sequence: Sequence,
    availability_buffer: AvailabilityBuffer,
}

impl MultiProducerSequencer {
    pub fn new(buffer_size: usize) -> Self {
        Self {
            buffer_size: utils::assert_buffer_size_pow_of_2(buffer_size) as i64,
            cached: Sequence::default(),
            cursor_sequence: Sequence::default(),
            gating_sequence: Sequence::default(),
            availability_buffer: AvailabilityBuffer::new(buffer_size),
        }
    }
}

impl Sequencer for MultiProducerSequencer {

    fn next_n(&self, n: usize) -> i64 {
        let n: i64 = n as i64;
        let next: i64 = self.cursor_sequence.fetch_add_volatile(n) + n;
        let wrap_point: i64 = next - self.buffer_size;

        if wrap_point > self.cached.get_relaxed() {
            self.cached.set_relaxed(self.wait(&self.gating_sequence, wrap_point));
        }

        next
    }

    fn publish_cursor_sequence(&self, sequence: i64) {
        self.availability_buffer.set(sequence);
    }

    fn publish_cursor_sequence_range(&self, low: i64, high: i64) {
        self.availability_buffer.set_range(low, high);
    }

    fn publish_gating_sequence(&self, sequence: i64) {
        self.gating_sequence.set_release(sequence);
    }

    fn get_highest(&self, low: i64, high: i64) -> i64 {
        self.availability_buffer.get_available(low, high)
    }

    fn get_cursor_sequence_relaxed(&self) -> i64 {
        self.cursor_sequence.get_relaxed()
    }

    fn get_cursor_sequence_acquire(&self) -> i64 {
        self.cursor_sequence.get_acquire()
    }

    fn get_gating_sequence_relaxed(&self) -> i64 {
        self.gating_sequence.get_relaxed()
    }

    fn get_gating_sequence_acquire(&self) -> i64 {
        self.gating_sequence.get_acquire()
    }
}


unsafe impl Send for SingleProducerSequencer {}

unsafe impl Sync for SingleProducerSequencer {}

unsafe impl Send for MultiProducerSequencer {}

unsafe impl Sync for MultiProducerSequencer {}
