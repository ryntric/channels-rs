use std::time::Duration;

static PARKING_TIMEOUT: Duration = Duration::from_nanos(1);

pub enum WaitStrategy {
    Parking,
    Spinning,
    Yielding,
}

impl WaitStrategy {
    pub(crate) fn wait(&self) {
        match self {
            WaitStrategy::Parking => std::thread::park_timeout(PARKING_TIMEOUT),
            WaitStrategy::Spinning => std::hint::spin_loop(),
            WaitStrategy::Yielding => std::thread::yield_now(),
        }
    }
}
