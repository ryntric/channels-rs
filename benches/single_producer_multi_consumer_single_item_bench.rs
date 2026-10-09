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
            let mut buffer = Vec::with_capacity(1024);

            while is_running_clone.load(Ordering::Acquire) {
                rx_clone.try_take_batch(&mut buffer);
                std::hint::black_box(&buffer);
                buffer.clear();
            }
        });
        handles.push(handle);
    }

    let event: Event = Event::default();

    let mut group = c.benchmark_group("spmc/single");
    group.throughput(Throughput::Elements(1));
    group.bench_function("push", |b| {
        b.iter(|| {
            tx.push(event);
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
