use crate::availability::AvailabilityBuffer;
use crate::coordinator::Coordinator;
use crate::sequence::Sequence;

pub(crate) trait Sequencer {
    fn next(&self, strategy: &Coordinator) -> i64 {
        self.next_n(1, strategy)
    }

    fn next_n(&self, n: usize, coordinator: &Coordinator) -> i64;

    fn publish_cursor_sequence(&self, sequence: i64);

    fn publish_cursor_sequence_range(&self, low: i64, high: i64);

    fn publish_gating_sequence(&self, sequence: i64);

    fn advance_gating_sequence(&self, gating_sequence: &Sequence, sequence: i64) {
        let mut current: i64 = gating_sequence.get_relaxed();
        loop {
            if current > sequence
                || gating_sequence.compare_and_exchange_weak_volatile(current, sequence)
            {
                break;
            }
            current = gating_sequence.get_acquire();
        }
    }

    fn get_highest(&self, low: i64, high: i64) -> i64;

    fn get_cursor_sequence_acquire(&self) -> i64;

    fn get_gating_sequence_relaxed(&self) -> i64;

    #[inline(always)]
    fn wait(&self, gating_sequence: &Sequence, wrap_point: i64, coordinator: &Coordinator) -> i64 {
        let mut gating: i64;
        loop {
            gating = gating_sequence.get_acquire();
            if wrap_point > gating {
                coordinator.producer_wait();
                continue;
            }
            return gating;
        }
    }
}

pub struct SingleProducerSequencer {
    sequence: Sequence,
    cached: Sequence,
    cursor_sequence: Sequence,
    gating_sequence: Sequence,
    capacity: i64,
}

impl SingleProducerSequencer {
    pub fn new(capacity: usize) -> Self {
        Self {
            sequence: Sequence::default(),
            cached: Sequence::default(),
            cursor_sequence: Sequence::default(),
            gating_sequence: Sequence::default(),
            capacity: capacity as i64,
        }
    }
}

impl Sequencer for SingleProducerSequencer {
    fn next_n(&self, n: usize, coordinator: &Coordinator) -> i64 {
        let next: i64 = self.sequence.get_relaxed() + n as i64;
        let wrap_point: i64 = next - self.capacity;

        if wrap_point > self.cached.get_relaxed() {
            self.cached
                .set_relaxed(self.wait(&self.gating_sequence, wrap_point, coordinator));
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
        self.advance_gating_sequence(&self.gating_sequence, sequence);
    }

    fn get_highest(&self, _: i64, high: i64) -> i64 {
        high
    }

    fn get_cursor_sequence_acquire(&self) -> i64 {
        self.cursor_sequence.get_acquire()
    }

    fn get_gating_sequence_relaxed(&self) -> i64 {
        self.gating_sequence.get_relaxed()
    }
}

pub struct MultiProducerSequencer {
    capacity: i64,
    cached: Sequence,
    cursor_sequence: Sequence,
    gating_sequence: Sequence,
    availability_buffer: AvailabilityBuffer,
}

impl MultiProducerSequencer {
    pub fn new(capacity: usize) -> Self {
        Self {
            capacity: capacity as i64,
            cached: Sequence::default(),
            cursor_sequence: Sequence::default(),
            gating_sequence: Sequence::default(),
            availability_buffer: AvailabilityBuffer::new(capacity),
        }
    }
}

impl Sequencer for MultiProducerSequencer {
    fn next_n(&self, n: usize, coordinator: &Coordinator) -> i64 {
        let n: i64 = n as i64;
        let next: i64 = self.cursor_sequence.fetch_add_volatile(n) + n;
        let wrap_point: i64 = next - self.capacity;

        if wrap_point > self.cached.get_relaxed() {
            self.cached
                .set_relaxed(self.wait(&self.gating_sequence, wrap_point, coordinator));
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
        self.advance_gating_sequence(&self.gating_sequence, sequence);
    }

    fn get_highest(&self, low: i64, high: i64) -> i64 {
        self.availability_buffer.get_available(low, high)
    }

    fn get_cursor_sequence_acquire(&self) -> i64 {
        self.cursor_sequence.get_acquire()
    }

    fn get_gating_sequence_relaxed(&self) -> i64 {
        self.gating_sequence.get_relaxed()
    }
}
