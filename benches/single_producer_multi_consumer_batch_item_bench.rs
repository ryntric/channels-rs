use criterion::{criterion_group, criterion_main, BatchSize, Criterion, Throughput};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::time::Duration;
use wait_strategy::WaitStrategy;
use workers_core_rust::{channel, wait_strategy};

#[derive(Copy, Clone)]
struct Event {}

fn bench_ring_buffer_offer_poll(c: &mut Criterion) {
    let (tx, rx) = channel::spmc::<Event>(8192, WaitStrategy::Yielding, WaitStrategy::Yielding);
    let is_running = Arc::new(AtomicBool::new(true));
    
    for _ in 0..4 {
        let rx_clone = rx.clone();
        let is_running_clone = is_running.clone();
        
        std::thread::spawn(move || {
            let handler: fn (Event) = |e| {
                std::hint::black_box(e);
            };

            while is_running_clone.load(Ordering::Acquire) {
                rx_clone.blocking_recv(&handler)
            }
        });
    }

    let mut group = c.benchmark_group("push batch");
    group.throughput(Throughput::Elements(8));
    group.bench_function("push_n", |b| {

        b.iter_batched(|| {
            vec![Event {},Event {},Event {},Event {},Event {},Event {},Event {},Event {}]
        },|events|{
            tx.send_n(events)
        }, BatchSize::LargeInput);
    });

    group.finish();
    is_running.store(false, Ordering::Release);

}

criterion_group!(benches, bench_ring_buffer_offer_poll);
criterion_main!(benches);
