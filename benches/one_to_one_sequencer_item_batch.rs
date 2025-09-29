use criterion::{criterion_group, criterion_main, BatchSize, Criterion, Throughput};
use std::sync::Arc;
use workers_core_rust::poller::SingleConsumer;
use workers_core_rust::ring_buffer::RingBuffer;
use workers_core_rust::sequencer::{MultiProducer, SingleProducer};
use workers_core_rust::worker_th::*;

#[derive(Copy, Clone)]
struct Event {}

fn bench_ring_buffer_offer_poll(c: &mut Criterion) {
    let ring_buffer = Arc::new(RingBuffer::<Event, SingleProducer, SingleConsumer>::new(8192));
    let worker_thread = WorkerThread::new(Arc::clone(&ring_buffer));
    worker_thread.start();

    let mut group = c.benchmark_group("push batch");
    group.throughput(Throughput::Elements(8));
    group.bench_function("push_n", |b| {

        let event: Event = Event {};

        b.iter_batched(|| {
            vec![Event {},Event {},Event {},Event {},Event {},Event {},Event {},Event {}]
        },|events|{
            ring_buffer.push_n(events)
        }, BatchSize::LargeInput);
    });

    worker_thread.stop();
    group.finish();

}

criterion_group!(benches, bench_ring_buffer_offer_poll);
criterion_main!(benches);
