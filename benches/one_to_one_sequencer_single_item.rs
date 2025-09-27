use criterion::{criterion_group, criterion_main, Criterion, Throughput};
use std::sync::Arc;
use workers_core_rust::ring_buffer::RingBuffer;
use workers_core_rust::sequencer::SequencerType;
use workers_core_rust::worker_th::*;

#[derive(Copy, Clone)]
struct Event {}

fn bench_ring_buffer_offer_poll(c: &mut Criterion) {
    let ring_buffer = Arc::new(RingBuffer::<Event>::new(8192, SequencerType::SingleProducer, ));
    let worker_thread = WorkerThread::new(Arc::clone(&ring_buffer), move |e| {
        std::hint::black_box(e);
    });

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
