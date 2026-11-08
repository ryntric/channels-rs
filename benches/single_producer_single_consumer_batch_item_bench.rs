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
    let (tx, rx) = spsc::<Event>(8192);
    let is_running = Arc::new(AtomicBool::new(true));

    let is_running_clone = is_running.clone();

    let handle = std::thread::spawn(move || {
        while is_running_clone.load(Ordering::Acquire) {
            if let Some(lease) = rx.try_lease(1024) {
                for value in lease {
                    std::hint::black_box(value);
                }
            } else {
                std::hint::spin_loop();
            }
        }
    });

    let mut group = c.benchmark_group("spsc/batch");
    group.throughput(Throughput::Elements(512));

    let mut sequence = 0i32;
    group.bench_function("push_512", |b| {
        b.iter(|| {
            let mut lease = tx.lease(512);

            for i in 0..512i32 {
                lease.write(Event {
                    number: sequence.wrapping_add(i),
                });
            }

            sequence = sequence.wrapping_add(512);
        });
    });

    group.finish();
    is_running.store(false, Ordering::Release);

    handle.join().expect("consumer thread panicked");
}

criterion_group!(benches, bench_ring_buffer_offer_poll);
criterion_main!(benches);
