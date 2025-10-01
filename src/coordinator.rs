use std::sync::{Arc, Condvar, Mutex};
use std::time::Duration;

#[derive(Copy, Clone, PartialEq, Debug)]
pub enum ConsumerWaitStrategyKind {
    Spinning,
    Parking(Duration),
    Yielding,
    Blocking
}

#[derive(Copy, Clone, PartialEq, Debug)]
pub enum ProducerWaitStrategyKind {
    Spinning,
    Parking(Duration),
    Yielding,
}

pub(crate) trait ConsumerWaitStrategy: Send + Sync {
    fn wait(&self);

    fn signal(&self);
}

#[derive(Clone)]
pub(crate) struct ConsumerSpinningStrategy {}

impl ConsumerSpinningStrategy {
    pub fn new() -> Self {
        Self {}
    }
}

impl ConsumerWaitStrategy for ConsumerSpinningStrategy {
    fn wait(&self) {
        std::hint::spin_loop();
    }

    #[warn(unused)]
    fn signal(&self) {
        //no-op
    }
}

#[derive(Clone)]
pub(crate) struct ConsumerParkingStrategy {
    duration: Duration,
}

impl ConsumerParkingStrategy {
    pub fn new(duration: Duration) -> Self {
        Self {duration}
    }
}

impl ConsumerWaitStrategy for ConsumerParkingStrategy {
    fn wait(&self) {
        std::thread::park_timeout(self.duration);
    }

    #[warn(unused)]
    fn signal(&self) {
        //no-op
    }
}

#[derive(Clone)]
pub(crate) struct ConsumerYieldingStrategy {}

impl ConsumerYieldingStrategy {
    pub fn new() -> Self {
        Self {}
    }
}

impl ConsumerWaitStrategy for ConsumerYieldingStrategy {
    fn wait(&self) {
        std::thread::yield_now();
    }

    #[warn(unused)]
    fn signal(&self) {
        //no-op
    }
}

#[derive(Clone)]
pub(crate) struct ConsumerBlockingStrategy {
    state: Arc<(Condvar, Mutex<bool>)>
}

impl ConsumerBlockingStrategy {
    pub fn new() -> Self {
        Self { state: Arc::new((Condvar::new(), Mutex::new(false))) }
    }
}

impl ConsumerWaitStrategy for ConsumerBlockingStrategy {
    fn wait(&self) {
        let (condvar, mutex) = &*self.state;
        let mut guard = mutex.lock().unwrap();
        while !*guard {
            guard = condvar.wait(guard).unwrap();
        }
        *guard = false;
    }

    fn signal(&self) {
        let (condvar, mutex) = &*self.state;
        let mut guard = mutex.lock().unwrap();
        *guard = true;
        condvar.notify_all();
    }
}

pub(crate) trait ProducerWaitStrategy: Send + Sync {
    fn wait(&self);
}

#[derive(Clone)]
pub(crate) struct ProducerSpinningStrategy {}

impl ProducerSpinningStrategy {
    pub fn new() -> Self {
        Self {}
    }
}

impl ProducerWaitStrategy for ProducerSpinningStrategy {
    fn wait(&self) {
        std::hint::spin_loop();
    }
}

#[derive(Clone)]
pub(crate) struct ProducerParkingStrategy {
    duration: Duration,
}

impl ProducerParkingStrategy {
    pub fn new(duration: Duration) -> Self {
        Self {duration}
    }
}

impl ProducerWaitStrategy for ProducerParkingStrategy {
    fn wait(&self) {
        std::thread::park_timeout(self.duration);
    }
}

#[derive(Clone)]
pub(crate) struct ProducerYieldingStrategy {}

impl ProducerYieldingStrategy {
    pub fn new() -> Self {
        Self {}
    }
}

impl ProducerWaitStrategy for ProducerYieldingStrategy {
    fn wait(&self) {
        std::thread::yield_now();
    }
}


pub(crate) struct Coordinator {
    cw: Box<dyn ConsumerWaitStrategy>,
    pw: Box<dyn ProducerWaitStrategy>,
}

impl Coordinator {
    pub fn new(pw: ProducerWaitStrategyKind, cw: ConsumerWaitStrategyKind) -> Self {
        let cw: Box<dyn ConsumerWaitStrategy> = match cw {
            ConsumerWaitStrategyKind::Spinning => Box::new(ConsumerSpinningStrategy::new()),
            ConsumerWaitStrategyKind::Parking(duration) => Box::new(ConsumerParkingStrategy::new(duration)),
            ConsumerWaitStrategyKind::Yielding => Box::new(ConsumerYieldingStrategy::new()),
            ConsumerWaitStrategyKind::Blocking => Box::new(ConsumerBlockingStrategy::new())
        };

        let pw: Box<dyn ProducerWaitStrategy> = match pw {
            ProducerWaitStrategyKind::Spinning => Box::new(ProducerSpinningStrategy::new()),
            ProducerWaitStrategyKind::Parking(duration) => Box::new(ProducerParkingStrategy::new(duration)),
            ProducerWaitStrategyKind::Yielding => Box::new(ProducerYieldingStrategy::new())
        };

        Self { cw, pw }
    }

    pub fn wait_for_consumer(&self) {
        self.pw.wait();
    }

    pub fn wait_for_producer(&self) {
        self.cw.wait();
    }

    pub fn signal_consumer(&self) {
        self.cw.signal();
    }

}
