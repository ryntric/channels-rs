use criterion::{criterion_group, criterion_main, Criterion, Throughput};
use std::sync::Arc;
use workers_core_rust::poller::SingleConsumer;
use workers_core_rust::ring_buffer::RingBuffer;
use workers_core_rust::sequencer::SingleProducer;
use workers_core_rust::worker_th::*;

#[derive(Copy, Clone)]
struct Event {}

fn bench_ring_buffer_offer_poll(c: &mut Criterion) {
    let ring_buffer = Arc::new(RingBuffer::<Event, SingleProducer, SingleConsumer>::new(8192));
    let worker_thread = WorkerThread::new(Arc::clone(&ring_buffer));

    let mut group = c.benchmark_group("one_to_one_sequencer_single");
    group.throughput(Throughput::Elements(1));
    group.bench_function("push_single_item", |b| {
        worker_thread.start();

        let event: Event = Event {};

        b.iter(|| {
            ring_buffer.push(event);
        });
    });

    worker_thread.stop();
    group.finish();

}

criterion_group!(benches, bench_ring_buffer_offer_poll);
criterion_main!(benches);
