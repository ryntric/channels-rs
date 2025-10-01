use criterion::{criterion_group, criterion_main, Criterion, Throughput};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use workers_core_rust::prelude::*;

#[derive(Copy, Clone)]
struct Event {}

fn bench_ring_buffer_offer_poll(c: &mut Criterion) {
    let (tx, rx) = spmc::<Event>(8192, ProducerWaitStrategyKind::Spinning, ConsumerWaitStrategyKind::Spinning);
    let is_running = Arc::new(AtomicBool::new(true));

    for _ in 0..4 {
        let rx_clone = rx.clone();
        let is_running_clone = is_running.clone();

        std::thread::spawn(move || {
            let handler: fn (Event) = |e| {
                std::hint::black_box(e);
            };

            while is_running_clone.load(Ordering::Acquire) {
                rx_clone.blocking_recv(1024, &handler)
            }
        });
    }

    let mut group = c.benchmark_group("push batch");
    group.throughput(Throughput::Elements(8));
    group.bench_function("push_n", |b| {

        let events: [Event; 8] = [Event {}; 8];

        b.iter(|| {
            tx.send_n(events)
        });
    });

    group.finish();
    is_running.store(false, Ordering::Release);

}

criterion_group!(benches, bench_ring_buffer_offer_poll);
criterion_main!(benches);
