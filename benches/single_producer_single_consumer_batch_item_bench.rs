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
        let mut buffer = Vec::with_capacity(1024);

        while is_running_clone.load(Ordering::Acquire) {
            rx.try_take_batch(&mut buffer);
            std::hint::black_box(&buffer);
            buffer.clear();
        }
    });

    let mut group = c.benchmark_group("spsc/batch");
    group.throughput(Throughput::Elements(16));
    group.bench_function("push_16", |b| {
        let events = [Event::default(); 16];

        b.iter(|| tx.push_array(events));
    });

    group.finish();
    is_running.store(false, Ordering::Release);

    handle.join().expect("consumer thread panicked");
}

criterion_group!(benches, bench_ring_buffer_offer_poll);
criterion_main!(benches);
