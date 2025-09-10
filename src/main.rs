use crate::event_translator::EventTranslatorOneArg;
use crate::ring_buffer::RingBuffer;
use crate::sequence::Sequence;
use crate::sequencer::SequencerType;
use std::hint;

mod availability_buffer;
mod constants;
pub(crate) mod event_translator;
pub(crate) mod ring_buffer;
pub(crate) mod sequence;
pub(crate) mod sequencer;
pub(crate) mod utils;

#[derive(Copy, Clone, Default, Debug)]
struct TestEvent {
    pub id: i64,
}

struct TestEventTranslator;

impl EventTranslatorOneArg<TestEvent, i64> for TestEventTranslator {
    fn translate_to(&self, event: &mut TestEvent, arg: i64) {
        event.id = arg;
    }
}

fn main() {
    let ring_buffer: RingBuffer<TestEvent> = RingBuffer::new(8192, SequencerType::SingleProducer);
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

                    for sequence in next..=available {
                        let _ = ring_buffer.get(sequence);
                    }
                    break;
                }

                sequence.set_release(available);
            }
        });

        for i in 0..100_000_000_0_000_000_000i64 {
            ring_buffer.publish_event(TestEventTranslator, i);
        }
    });
}
