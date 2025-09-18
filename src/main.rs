use crate::event_handler::EvenHandler;
use crate::event_poller::{EventPoller, EventPollerState};
use crate::event_translator::EventTranslatorOneArg;
use crate::ring_buffer::RingBuffer;
use crate::sequencer::SequencerType;
use std::error::Error;

pub(crate) mod availability_buffer;
pub(crate) mod constants;
pub(crate) mod event_handler;
pub(crate) mod event_poller;
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

struct TestEventHandler;

impl EvenHandler<TestEvent> for TestEventHandler {
    #[inline(always)]
    fn on_event(&self, event: &TestEvent) -> Result<(), Box<dyn Error>> {
        Ok(())
    }

    fn on_error(&self, error: Box<dyn Error>) {
        println!("Error: {:?}", error);
    }
}

impl EventTranslatorOneArg<TestEvent, i64> for TestEventTranslator {
    fn translate_to(&self, event: &mut TestEvent, arg: i64) {
        event.id = arg;
    }
}

fn main() {
    let ring_buffer: RingBuffer<TestEvent> = RingBuffer::new(8192, SequencerType::SingleProducer);
    let poller: EventPoller<TestEvent> = EventPoller::new(&ring_buffer);

    std::thread::scope(|scope| {
        scope.spawn(|| {
            let handler = TestEventHandler;
            loop {
                if poller.poll(&handler) == EventPollerState::Idle {
                    std::hint::spin_loop();
                }
            }
        });

        for i in 0..1000_000_000i64 {
            ring_buffer.publish_event(TestEventTranslator, i);
        }
    });
}
