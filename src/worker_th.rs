use crate::poller::{PollState, Poller};
use crate::ring_buffer::RingBuffer;
use crate::sequencer::Sequencer;
use crate::utils;
use std::io;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::thread::JoinHandle;

pub struct WorkerThread<T: 'static, S, P>
where
    S: Sequencer + 'static,
    P: Poller<T, S> + 'static,
{
    is_running: Arc<AtomicBool>,
    buffer: Arc<RingBuffer<T, S, P>>,
}

impl<T: 'static, S, P> WorkerThread<T, S, P>
where
    S: Sequencer,
    P: Poller<T, S>,
{
    pub fn new(buffer: Arc<RingBuffer<T, S, P>>) -> Self {
        Self {
            is_running: Arc::new(AtomicBool::new(false)),
            buffer,
        }
    }

    pub fn start(&self) {
        if utils::compare_and_exchange_bool(&self.is_running, false, true, Ordering::AcqRel, Ordering::Relaxed) {
            let is_running = Arc::clone(&self.is_running);
            let buffer = Arc::clone(&self.buffer);

            let closure = move || {
                let handler: fn(T) = |item| {
                    std::hint::black_box(item);
                };
                while is_running.load(Ordering::Acquire) {
                    if buffer.poll(&handler) == PollState::Empty {
                        std::hint::spin_loop();
                    }
                }
            };

            if let Err(e) = Self::build_thread("test".to_string(), closure) {
                panic!("Can not create a worker thread");
            };
        };
    }

    pub fn stop(&self) {
        if utils::compare_and_exchange_bool(
            &self.is_running,
            true,
            false,
            Ordering::AcqRel,
            Ordering::Relaxed,
        ) {
            self.is_running.store(false, Ordering::Release);
        }
    }

    fn build_thread<C: Fn() + 'static + Sync + Send>(
        name: String,
        closure: C,
    ) -> io::Result<JoinHandle<()>> {
        std::thread::Builder::new().name(name).spawn(closure)
    }
}

unsafe impl<T, S, P> Sync for WorkerThread<T, S, P>
where
    S: Sequencer,
    P: Poller<T, S>,
{
}

unsafe impl<T, S, P> Send for WorkerThread<T, S, P>
where
    S: Sequencer,
    P: Poller<T, S>,
{
}
