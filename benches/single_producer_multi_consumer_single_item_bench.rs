use channels_rs::prelude::*;
use criterion::{Criterion, Throughput, criterion_group, criterion_main};
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};

#[derive(Copy, Clone, Default)]
struct Event {
    #[allow(dead_code)]
    number: i32,
}

fn bench_ring_buffer_offer_poll(c: &mut Criterion) {
    let (tx, rx) = spmc::<Event>(8192);
    let is_running = Arc::new(AtomicBool::new(true));

    let mut handles = Vec::new();

    for _ in 0..4 {
        let rx_clone = rx.clone();
        let is_running_clone = is_running.clone();

        let handle = std::thread::spawn(move || {
            while is_running_clone.load(Ordering::Acquire) {
                if let Some(lease) = rx_clone.try_lease(1024) {
                    for value in lease {
                        std::hint::black_box(value);
                    }
                } else {
                    std::hint::spin_loop();
                }
            }
        });
        handles.push(handle);
    }

    let mut group = c.benchmark_group("spmc/single");
    group.throughput(Throughput::Elements(1));
    group.bench_function("push", |b| {
        let mut sequence = 0i32;
        b.iter(|| {
            tx.push(Event {
                number: sequence.wrapping_add(1),
            });
            sequence = sequence.wrapping_add(1);
        });
    });

    group.finish();
    is_running.store(false, Ordering::Release);

    for handle in handles {
        handle.join().expect("consumer thread panicked");
    }
}

criterion_group!(benches, bench_ring_buffer_offer_poll);
criterion_main!(benches);
