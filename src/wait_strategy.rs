use std::time::Duration;

static PARKING_TIMEOUT: Duration = Duration::from_nanos(1);

pub type WaitStrategy = fn();

pub static PARKING: WaitStrategy = || std::thread::park_timeout(PARKING_TIMEOUT);
pub static SPINNING: WaitStrategy = std::hint::spin_loop;
pub static YIELDING: WaitStrategy = std::thread::yield_now;
