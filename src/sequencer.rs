use crate::availability::AvailabilityBuffer;
use crate::coordinator::Coordinator;
use crate::sequence::{LocalSequence, SharedSequence};
use std::sync::atomic::Ordering::{Acquire, Relaxed, Release};

pub(crate) trait Sequencer {
    fn next(&self, strategy: &Coordinator) -> i64 {
        self.next_n(1, strategy)
    }

    fn next_n(&self, n: usize, coordinator: &Coordinator) -> i64;

    fn publish_cursor_sequence(&self, sequence: i64);

    fn publish_cursor_sequence_range(&self, low: i64, high: i64);

    fn publish_gating_sequence(&self, sequence: i64);

    fn publish_gating_sequence_cas(&self, sequence: i64);

    fn advance_gating_sequence(&self, gating_sequence: &SharedSequence, sequence: i64) {
        let mut current: i64 = gating_sequence.load(Relaxed);

        while sequence > current {
            match gating_sequence.compare_exchange_weak(current, sequence, Release, Relaxed) {
                Ok(_) => break,
                Err(actual) => current = actual,
            }
        }
    }

    fn get_cursor_highest(&self, low: i64, high: i64) -> i64;

    fn get_cursor_sequence(&self) -> i64;

    fn get_gating_sequence_relaxed(&self) -> i64;

    #[inline(always)]
    fn wait(
        &self,
        gating_sequence: &SharedSequence,
        wrap_point: i64,
        coordinator: &Coordinator,
    ) -> i64 {
        let mut gating: i64;
        loop {
            gating = gating_sequence.load(Acquire);
            if wrap_point > gating {
                coordinator.producer_wait();
                continue;
            }
            return gating;
        }
    }
}

pub struct SingleProducerSequencer {
    sequence: LocalSequence,
    cached_gating: LocalSequence,
    cursor_sequence: SharedSequence,
    gating_sequence: SharedSequence,
    capacity: i64,
}

impl SingleProducerSequencer {
    pub fn new(capacity: usize) -> Self {
        Self {
            sequence: LocalSequence::default(),
            cached_gating: LocalSequence::default(),
            cursor_sequence: SharedSequence::default(),
            gating_sequence: SharedSequence::default(),
            capacity: capacity as i64,
        }
    }
}

impl Sequencer for SingleProducerSequencer {
    fn next_n(&self, n: usize, coordinator: &Coordinator) -> i64 {
        let next: i64 = self.sequence.get() + n as i64;
        let wrap_point: i64 = next - self.capacity;

        if wrap_point > self.cached_gating.get() {
            self.cached_gating
                .set(self.wait(&self.gating_sequence, wrap_point, coordinator));
        }

        self.sequence.set(next);
        next
    }

    fn publish_cursor_sequence(&self, sequence: i64) {
        self.cursor_sequence.store(sequence, Release);
    }

    fn publish_cursor_sequence_range(&self, _: i64, high: i64) {
        self.cursor_sequence.store(high, Release)
    }

    fn publish_gating_sequence(&self, sequence: i64) {
        self.gating_sequence.store(sequence, Release);
    }

    fn publish_gating_sequence_cas(&self, sequence: i64) {
        self.advance_gating_sequence(&self.gating_sequence, sequence);
    }

    fn get_cursor_highest(&self, _: i64, high: i64) -> i64 {
        high
    }

    fn get_cursor_sequence(&self) -> i64 {
        self.cursor_sequence.load(Acquire)
    }

    fn get_gating_sequence_relaxed(&self) -> i64 {
        self.gating_sequence.load(Relaxed)
    }
}

pub struct MultiProducerSequencer {
    capacity: i64,
    cached: SharedSequence,
    cursor_sequence: SharedSequence,
    gating_sequence: SharedSequence,
    availability_buffer: AvailabilityBuffer,
}

impl MultiProducerSequencer {
    pub fn new(capacity: usize) -> Self {
        Self {
            capacity: capacity as i64,
            cached: SharedSequence::default(),
            cursor_sequence: SharedSequence::default(),
            gating_sequence: SharedSequence::default(),
            availability_buffer: AvailabilityBuffer::new(capacity),
        }
    }
}

impl Sequencer for MultiProducerSequencer {
    fn next_n(&self, n: usize, coordinator: &Coordinator) -> i64 {
        let n: i64 = n as i64;
        let next: i64 = self.cursor_sequence.fetch_add(n, Relaxed) + n;
        let wrap_point: i64 = next - self.capacity;

        if wrap_point > self.cached.load(Acquire) {
            self.cached.fetch_max(
                self.wait(&self.gating_sequence, wrap_point, coordinator),
                Release,
            );
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
        self.gating_sequence.store(sequence, Release);
    }

    fn publish_gating_sequence_cas(&self, sequence: i64) {
        self.advance_gating_sequence(&self.gating_sequence, sequence);
    }

    fn get_cursor_highest(&self, low: i64, high: i64) -> i64 {
        self.availability_buffer.get_available(low, high)
    }

    fn get_cursor_sequence(&self) -> i64 {
        self.cursor_sequence.load(Relaxed)
    }

    fn get_gating_sequence_relaxed(&self) -> i64 {
        self.gating_sequence.load(Relaxed)
    }
}
