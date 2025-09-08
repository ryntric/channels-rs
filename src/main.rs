use crate::event_translator::EventTranslatorOneArg;
use crate::ring_buffer::RingBuffer;
use crate::sequence::Sequence;
use crate::sequencer::{OneToOneSequencer, Sequencer};
use std::hint;

mod event_translator;
mod ring_buffer;
mod sequence;
mod sequencer;
mod utils;

#[derive(Copy, Clone, Default, Debug)]
struct TestEvent {
    id: i64,
}

struct TestEventTranslator;

impl EventTranslatorOneArg<TestEvent, i64> for TestEventTranslator {
    fn translate_to(&self, event: &mut TestEvent, arg: i64) {
        event.id = arg;
    }
}

fn main() {
    let ring_buffer: RingBuffer<TestEvent,> = RingBuffer::new(8192, Box::new(OneToOneSequencer::new(8192)));
    std::thread::scope(|scope| {
        scope.spawn(|| {
            let sequencer = ring_buffer.get_sequencer();
            let sequence: &Sequence = sequencer.get_gating_sequence();
            let gating_sequence: &Sequence = sequencer.get_cursor_sequence();

            let mut next: i64;
            let mut available: i64;
            loop {
                next = sequence.get_plain() + 1;
                available = gating_sequence.get_plain();

                loop {
                    if next > available {
                        hint::spin_loop();
                        available = gating_sequence.get_acquire();
                        continue;
                    }
                    break;
                }
                println!("{}", available);
                sequence.set_plain(available);
            }
        });

        for i in 0..100_000_000_0_000_000_000i64 {
            ring_buffer.publish_event(TestEventTranslator, i);
        }
    });


}
