use channels_rs::prelude::*;
use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::thread;

const CAPACITY: usize = 64;
const PER_PRODUCER: u64 = 100_000;
const BATCH: usize = 16;

fn encode(producer: u64, i: u64) -> u64 {
    (producer << 32) | i
}

/// Every value must be received exactly once, and a single consumer must see
/// each producer's values in increasing order.
fn verify(producers: u64, per_consumer: Vec<Vec<u64>>) {
    for received in &per_consumer {
        let mut last = vec![None::<u64>; producers as usize];
        for &v in received {
            let (p, i) = ((v >> 32) as usize, v & 0xFFFF_FFFF);
            if let Some(prev) = last[p] {
                assert!(i > prev, "producer {p}: {i} received after {prev}");
            }
            last[p] = Some(i);
        }
    }

    let mut all: Vec<u64> = per_consumer.into_iter().flatten().collect();
    all.sort_unstable();
    let expected: Vec<u64> = (0..producers)
        .flat_map(|p| (0..PER_PRODUCER).map(move |i| encode(p, i)))
        .collect();
    assert_eq!(all.len(), expected.len(), "lost or duplicated values");
    assert!(all == expected, "lost or duplicated values");
}

/// Runs `producers` x `consumers` threads over one channel.
/// `$batch` switches between single-item and lease-based send/receive.
macro_rules! stress {
    ($ctor:ident, $producers:expr, $consumers:expr, $batch:expr) => {{
        let producers: u64 = $producers;
        let consumers: usize = $consumers;
        let total = (producers * PER_PRODUCER) as usize;
        let (tx, rx) = $ctor::<u64>(CAPACITY);
        let received = Arc::new(AtomicUsize::new(0));

        let consumer_handles: Vec<_> = (0..consumers)
            .map(|_| {
                let rx = rx.clone();
                let received = received.clone();
                thread::spawn(move || {
                    let mut out = Vec::new();
                    while received.load(Ordering::Relaxed) < total {
                        let before = out.len();
                        if $batch {
                            if let Some(mut lease) = rx.try_lease(BATCH) {
                                while let Some(v) = lease.read() {
                                    out.push(v);
                                }
                            }
                        } else if let Some(v) = rx.try_take() {
                            out.push(v);
                        }
                        let got = out.len() - before;
                        if got == 0 {
                            std::hint::spin_loop();
                        } else {
                            received.fetch_add(got, Ordering::Relaxed);
                        }
                    }
                    out
                })
            })
            .collect();
        drop(rx);

        let producer_handles: Vec<_> = (0..producers)
            .map(|p| {
                let tx = tx.clone();
                thread::spawn(move || {
                    if $batch {
                        let mut i = 0;
                        while i < PER_PRODUCER {
                            let n = BATCH.min((PER_PRODUCER - i) as usize);
                            let mut lease = tx.lease(n);
                            for _ in 0..n {
                                lease.write(encode(p, i));
                                i += 1;
                            }
                        }
                    } else {
                        for i in 0..PER_PRODUCER {
                            tx.push(encode(p, i));
                        }
                    }
                })
            })
            .collect();
        drop(tx);

        for h in producer_handles {
            h.join().expect("producer panicked");
        }
        let per_consumer = consumer_handles
            .into_iter()
            .map(|h| h.join().expect("consumer panicked"))
            .collect();
        verify(producers, per_consumer);
    }};
}

/// SPSC/MPSC receivers and SPSC/SPMC senders are not `Clone`; wrap them so the
/// macro can treat every flavour uniformly (each wrapper is cloned exactly once).
macro_rules! once_clone {
    ($name:ident, $inner:ty) => {
        struct $name<T>(std::cell::Cell<Option<$inner>>);
        impl<T> $name<T> {
            fn clone(&self) -> $inner {
                self.0.take().expect("cloned more than once")
            }
        }
    };
}

once_clone!(SpscTx, channels_rs::channels::SpscSender<T>);
once_clone!(SpscRx, channels_rs::channels::SpscReceiver<T>);
once_clone!(SpmcTx, channels_rs::channels::SpmcSender<T>);
once_clone!(MpscRx, channels_rs::channels::MpscReceiver<T>);

fn spsc_once<T>(capacity: usize) -> (SpscTx<T>, SpscRx<T>) {
    let (tx, rx) = spsc(capacity);
    (SpscTx(Some(tx).into()), SpscRx(Some(rx).into()))
}

fn spmc_once<T>(capacity: usize) -> (SpmcTx<T>, channels_rs::channels::SpmcReceiver<T>) {
    let (tx, rx) = spmc(capacity);
    (SpmcTx(Some(tx).into()), rx)
}

fn mpsc_once<T>(capacity: usize) -> (channels_rs::channels::MpscSender<T>, MpscRx<T>) {
    let (tx, rx) = mpsc(capacity);
    (tx, MpscRx(Some(rx).into()))
}

#[test]
fn spsc_single() {
    stress!(spsc_once, 1, 1, false);
}

#[test]
fn spsc_batch() {
    stress!(spsc_once, 1, 1, true);
}

#[test]
fn spmc_single() {
    stress!(spmc_once, 1, 4, false);
}

#[test]
fn spmc_batch() {
    stress!(spmc_once, 1, 4, true);
}

#[test]
fn mpsc_single() {
    stress!(mpsc_once, 4, 1, false);
}

#[test]
fn mpsc_batch() {
    stress!(mpsc_once, 4, 1, true);
}

#[test]
fn mpmc_single() {
    stress!(mpmc, 4, 4, false);
}

#[test]
fn mpmc_batch() {
    stress!(mpmc, 4, 4, true);
}
