use channels_rs::prelude::*;
use criterion::{Criterion, Throughput, criterion_group, criterion_main};
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};

#[derive(Copy, Clone, Default)]
struct Event {
    number: i32,
}

fn bench_ring_buffer_offer_poll(c: &mut Criterion) {
    let (tx, rx) = spmc::<Event>(
        8192,
        ProducerWaitStrategyKind::Spinning,
        ConsumerWaitStrategyKind::Spinning,
    );
    let is_running = Arc::new(AtomicBool::new(true));

    for _ in 0..4 {
        let rx_clone = rx.clone();
        let is_running_clone = is_running.clone();

        std::thread::spawn(move || {
            let mut buffer = Vec::with_capacity(1024);

            while is_running_clone.load(Ordering::Acquire) {
                rx_clone.blocking_recv(&mut buffer);
                std::hint::black_box(&buffer);
                buffer.clear();
            }
        });
    }

    let event: Event = Event::default();

    let mut group = c.benchmark_group("spmc/single");
    group.throughput(Throughput::Elements(1));
    group.bench_function("push", |b| {
        b.iter(|| {
            tx.send(event);
        });
    });

    group.finish();
    is_running.store(false, Ordering::Release);
}

criterion_group!(benches, bench_ring_buffer_offer_poll);
criterion_main!(benches);
