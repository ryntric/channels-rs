use std::time::Duration;

#[derive(Debug, Copy, Clone)]
pub enum WaitStrategy {
    Parking(Duration),
    Spinning,
    Yielding,
}

impl WaitStrategy {
    pub(crate) fn wait(&self) {
        match self {
            WaitStrategy::Parking(timeout) => std::thread::park_timeout(*timeout),
            WaitStrategy::Spinning => std::hint::spin_loop(),
            WaitStrategy::Yielding => std::thread::yield_now(),
        }
    }
}
