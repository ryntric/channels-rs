use std::sync::{Arc, Condvar, Mutex};
use std::time::Duration;

/// Describes the wait strategy for a consumer in a concurrent data structure.
///
/// Used to determine how a consumer thread waits when no data is available.
#[derive(Copy, Clone, PartialEq, Debug)]
pub enum ConsumerWaitStrategyKind {
    /// Continuously spin in a busy loop.
    Spinning,
    /// Park the thread for the specified duration.
    Parking(Duration),
    /// Yield the thread to the scheduler.
    Yielding,
    /// Block using a condition variable until signaled.
    Blocking
}

/// Describes the wait strategy for a producer in a concurrent data structure.
///
/// Used to determine how a producer thread waits when the buffer is full.
#[derive(Copy, Clone, PartialEq, Debug)]
pub enum ProducerWaitStrategyKind {
    /// Continuously spin in a busy loop.
    Spinning,
    /// Park the thread for the specified duration.
    Parking(Duration),
    /// Yield the thread to the scheduler.
    Yielding,
}

/// Trait representing a consumer wait strategy.
pub(crate) trait ConsumerWaitStrategy: Send + Sync {
    /// Wait according to the strategy.
    fn wait(&self);

    /// Optionally wake up the consumer if it is blocked.
    fn signal(&self);
}

/// Spin-loop wait strategy for consumers.
#[derive(Clone)]
pub(crate) struct ConsumerSpinningStrategy {}

impl ConsumerSpinningStrategy {
    /// Create a new spinning strategy.
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

/// Parking wait strategy for consumers.
#[derive(Clone)]
pub(crate) struct ConsumerParkingStrategy {
    duration: Duration,
}

impl ConsumerParkingStrategy {
    /// Create a new parking strategy with the specified duration.
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

/// Yielding wait strategy for consumers.
#[derive(Clone)]
pub(crate) struct ConsumerYieldingStrategy {}

impl ConsumerYieldingStrategy {
    /// Create a new yielding strategy.
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

/// Blocking wait strategy for consumers using a condition variable.
#[derive(Clone)]
pub(crate) struct ConsumerBlockingStrategy {
    state: Arc<(Condvar, Mutex<bool>)>
}

impl ConsumerBlockingStrategy {
    /// Create a new blocking strategy.
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

/// Trait representing a producer wait strategy.
pub(crate) trait ProducerWaitStrategy: Send + Sync {
    fn wait(&self);
}

/// Spin-loop wait strategy for producers.
#[derive(Clone)]
pub(crate) struct ProducerSpinningStrategy {}

impl ProducerSpinningStrategy {
    /// Create a new spinning strategy.
    pub fn new() -> Self {
        Self {}
    }
}

impl ProducerWaitStrategy for ProducerSpinningStrategy {
    fn wait(&self) {
        std::hint::spin_loop();
    }
}

/// Parking wait strategy for producers.
#[derive(Clone)]
pub(crate) struct ProducerParkingStrategy {
    duration: Duration,
}

impl ProducerParkingStrategy {
    /// Create a new parking strategy with the specified duration.
    pub fn new(duration: Duration) -> Self {
        Self {duration}
    }
}

impl ProducerWaitStrategy for ProducerParkingStrategy {
    fn wait(&self) {
        std::thread::park_timeout(self.duration);
    }
}

/// Yielding wait strategy for producers.
#[derive(Clone)]
pub(crate) struct ProducerYieldingStrategy {}

impl ProducerYieldingStrategy {
    /// Create a new yielding strategy.
    pub fn new() -> Self {
        Self {}
    }
}

impl ProducerWaitStrategy for ProducerYieldingStrategy {
    fn wait(&self) {
        std::thread::yield_now();
    }
}

/// Coordinates producer and consumer wait strategies.
pub(crate) struct Coordinator {
    cw: Box<dyn ConsumerWaitStrategy>,
    pw: Box<dyn ProducerWaitStrategy>,
}

impl Coordinator {
    /// Create a new coordinator with the specified producer and consumer wait strategies.
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

    /// Wait according to the producer strategy.
    pub fn producer_wait(&self) {
        self.pw.wait();
    }

    /// Wait according to the consumer strategy.
    pub fn consumer_wait(&self) {
        self.cw.wait();
    }

    /// Wake up a consumer that may be blocked.
    pub fn wakeup_consumer(&self) {
        self.cw.signal();
    }

}
