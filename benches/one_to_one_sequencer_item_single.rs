use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use criterion::{criterion_group, criterion_main, Criterion, Throughput};
use workers_core_rust::channel;
use workers_core_rust::poller::State::Idle;

#[derive(Copy, Clone)]
struct Event {}

fn bench_ring_buffer_offer_poll(c: &mut Criterion) {
    let (tx, rx) = channel::spsc::<Event>(8192);

    let is_running = Arc::new(AtomicBool::new(true));
    let is_running_clone = is_running.clone();
    std::thread::spawn(move || {
        let handler = |e| {
            std::hint::black_box(e);
        };

        while is_running_clone.load(Ordering::Acquire) {
            if rx.recv(&handler) == Idle {
                std::hint::spin_loop()
            }
        }
    });

    let event: Event = Event {};

    let mut group = c.benchmark_group("push single");
    group.throughput(Throughput::Elements(1));
    group.bench_function("push", |b| {
        b.iter(|| {
            tx.send(*&event);
        });
    });

    is_running.store(false, Ordering::Release);
    group.finish();
}

criterion_group!(benches, bench_ring_buffer_offer_poll);
criterion_main!(benches);
