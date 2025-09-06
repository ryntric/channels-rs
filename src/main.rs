use crate::ring_buffer::RingBuffer;
use crate::sequence::Sequence;
use crate::sequencer::Sequencer;
use std::hint;
use std::sync::Arc;

mod event_translator;
mod ring_buffer;
mod sequence;
mod sequencer;

struct Event {
    id: i32,
}

impl Default for Event {
    fn default() -> Self {
        Self { id: 0 }
    }
}

impl Clone for Event {
    fn clone(&self) -> Self {
        Self { id: self.id }
    }
}

impl Copy for Event {

}

fn main() {
    let ring_buffer: RingBuffer<Event, 8192> = RingBuffer::new();

    let sequencer = Arc::new(sequencer::OneToOneSequencer::new(8192));

    let arc = sequencer.clone();
    let handle = std::thread::spawn(move || {
        let sequence: Arc<Sequence> = arc.get_gating_sequence();
        let gating_sequence: Arc<Sequence> = arc.get_cursor_sequence();

        let mut next: i64;
        let mut available: i64;
        loop {
            next = sequence.get_plain() + 1;
            available = gating_sequence.get_plain();
            if next > available {
                loop {
                    available = gating_sequence.get_acquire();
                    if next <= available {
                        break;
                    }
                    hint::spin_loop();
                }
            }

            for i in next..=available {
                hint::spin_loop();
            }
            println!("{}", available);
            sequence.set_plain(available);
        }
    });

    for i in (0..1_000_000_000).step_by(5000) {
        sequencer.publish(sequencer.next_n(5000));
    }
    handle.join().unwrap();
}
