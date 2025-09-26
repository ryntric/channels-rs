use crate::event_poller::{EventPoller, EventPollerState};
use crate::ring_buffer::RingBuffer;
use crate::utils;
use std::io;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::thread::JoinHandle;

pub(crate) struct WorkerThread<T: 'static, H>
where
    H: Fn(T) + 'static,
{
    is_running: Arc<AtomicBool>,
    poller: Arc<EventPoller<T>>,
    handler: Arc<H>,
}

impl<T: 'static, H> WorkerThread<T, H>
where
    H: Fn(T) + Sync + Send,
{
    pub fn new(buffer: Arc<RingBuffer<T>>, handler: H) -> Self {
        Self {
            is_running: Arc::new(AtomicBool::new(false)),
            poller: Arc::new(EventPoller::new(buffer)),
            handler: Arc::new(handler),
        }
    }

    pub fn start(&self) {
        if utils::compare_and_exchange_bool(&self.is_running, false, true, Ordering::AcqRel, Ordering::Relaxed) {
            let is_running = Arc::clone(&self.is_running);
            let poller = Arc::clone(&self.poller);
            let handler = Arc::clone(&self.handler);

            let closure = move || {
                while is_running.load(Ordering::Acquire) {
                    if poller.poll(&*handler) == EventPollerState::Idle {
                        std::hint::spin_loop();
                    }
                }
            };
            
            if let Err(e) = Self::build_thread("test".to_string(), closure) {
                panic!("Can not create a worker thread");
            };
        };
    }

    fn build_thread<C: Fn() + 'static + Sync + Send>(name: String, closure: C) -> io::Result<JoinHandle<()>> {
        std::thread::Builder::new()
            .name(name)
            .spawn(closure)
    }
}

unsafe impl<T, H> Sync for WorkerThread<T, H> where H: Fn(T) + Send + Sync {}

unsafe impl<T, H> Send for WorkerThread<T, H> where H: Fn(T) + Send + Sync {}
