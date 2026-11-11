use channels_rs::prelude::*;
use std::time::{Duration, Instant};

/// A producer whose cached view of the consumers lags by more than `capacity`
/// must still be able to make progress (the tracking slots it re-scans have
/// already been overwritten by later laps).
#[test]
fn mpmc_lagging_producer_makes_progress() {
	let (tx_a, rx) = mpmc::<u64>(8);
	let tx_b = tx_a.clone();

	tx_a.push(0);
	assert_eq!(rx.take(), 0);

	for i in 0..1_000 {
		tx_b.push(i);
		assert_eq!(rx.take(), i);
	}

	let deadline = Instant::now() + Duration::from_secs(2);
	loop {
		if tx_a.try_push(42).is_ok() {
			break;
		}
		assert!(Instant::now() < deadline, "lagging producer is stuck");
	}
	assert_eq!(rx.take(), 42);
}

/// Same as above, for a receiver that lags behind other receivers.
#[test]
fn mpmc_lagging_consumer_makes_progress() {
	// A single producer that never lags, so only the receiver side is exercised.
	let (tx, rx_a) = mpmc::<u64>(8);
	let rx_b = rx_a.clone();

	tx.push(0);
	assert_eq!(rx_a.take(), 0);

	for i in 0..1_000 {
		tx.push(i);
		assert_eq!(rx_b.take(), i);
	}

	tx.push(42);
	let deadline = Instant::now() + Duration::from_secs(2);
	loop {
		if let Some(v) = rx_a.try_take() {
			assert_eq!(v, 42);
			break;
		}
		assert!(Instant::now() < deadline, "lagging consumer is stuck");
	}
}