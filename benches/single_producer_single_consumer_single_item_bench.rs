use criterion::{criterion_group, criterion_main, Criterion, Throughput};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use channels::prelude::*;

#[derive(Copy, Clone)]
struct Event {}

fn bench_ring_buffer_offer_poll(c: &mut Criterion) {
    let (tx, rx) = spmc::<Event>(8192,  ProducerWaitStrategyKind::Spinning, ConsumerWaitStrategyKind::Spinning);
    let is_running = Arc::new(AtomicBool::new(true));

    let rx_clone = rx.clone();
    let is_running_clone = is_running.clone();

    std::thread::spawn(move || {
        let handler: fn(Event) = |e| {
            std::hint::black_box(e);
        };

        while is_running_clone.load(Ordering::Acquire) {
            rx_clone.blocking_recv(1024, &handler)
        }
    });

    let event: Event = Event {};

    let mut group = c.benchmark_group("spsc/single");
    group.throughput(Throughput::Elements(1));
    group.bench_function("push", |b| {
        b.iter(|| {
            tx.send(*&event);
        });
    });

    group.finish();
    is_running.store(false, Ordering::Release);
}

criterion_group!(benches, bench_ring_buffer_offer_poll);
criterion_main!(benches);
