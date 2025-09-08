use crate::sequence::Sequence;

pub trait Sequencer: Sync + Send {
    fn next(&self) -> i64;

    fn next_n(&self, n: i32) -> i64;

    fn publish(&self, sequence: i64);

    fn publish_range(&self, low: i64, high: i64);

    fn get_highest(&self, next: i64, available: i64) -> i64;

    fn get_cursor_sequence(&self) -> &Sequence;

    fn get_gating_sequence(&self) -> &Sequence;

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

pub struct OneToOneSequencer {
    sequence: Sequence,
    cached: Sequence,
    buffer_size: i64,
    cursor_sequence: Sequence,
    gating_sequence: Sequence,
}

impl OneToOneSequencer {
    pub fn new(buffer_size: usize) -> Self {
        if buffer_size.count_ones() != 1 {
            panic!("buffer size must be a power of two")
        };
        OneToOneSequencer {
            sequence: Sequence::default(),
            cached: Sequence::default(),
            buffer_size: buffer_size as i64,
            cursor_sequence: Sequence::default(),
            gating_sequence: Sequence::default(),
        }
    }
}

impl Sequencer for OneToOneSequencer {
    fn next(&self) -> i64 {
        self.next_n(1)
    }

    #[inline(always)]
    fn next_n(&self, n: i32) -> i64 {
        let buffer_size: i64 = self.buffer_size;
        let cached: i64 = self.cached.get_plain();
        let next: i64 = self.sequence.get_plain() + n as i64;
        let wrap_point: i64 = next - buffer_size;

        if wrap_point > cached {
            self.cached.set_plain(self.wait(&self.gating_sequence, wrap_point));
        }

        self.sequence.set_plain(next);
        next
    }

    fn publish(&self, sequence: i64) {
        self.cursor_sequence.set_release(sequence);
    }

    fn publish_range(&self, _: i64, high: i64) {
        self.cursor_sequence.set_release(high)
    }

    fn get_highest(&self, _: i64, available: i64) -> i64 {
        available
    }

    fn get_cursor_sequence(&self) -> &Sequence {
        &self.cursor_sequence
    }

    fn get_gating_sequence(&self) -> &Sequence {
        &self.gating_sequence
    }
}
