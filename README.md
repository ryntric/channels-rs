#### It is a low-latency rust concurrency library designed around ring buffers, sequencers, and customizable wait strategies. It provides both single-producer and multi-producer configurations, along with batch and single-item publishing modes, to maximize throughput and minimize contention

**Build Environment requirements**
- cargo 1.90.0 or greater
- rustc 1.90.0 

Example of usage
---

```rust
#[derive(Default, Debug)]
struct TestEvent {
    pub id: i64,
    pub name: String,
}
fn main() {
    let ring_buffer: Arc<RingBuffer<TestEvent>> = Arc::new(RingBuffer::new(8192, SequencerType::SingleProducer));

    let handler: fn(TestEvent) = |event: TestEvent| {
        println!("{:?}", event);
    };

    let worker: WorkerThread<TestEvent, fn(TestEvent)> = WorkerThread::new(Arc::clone(&ring_buffer), handler);
    worker.start();

    for i in 0..1000_000_000i64 {
        ring_buffer.push(TestEvent {id: i, name: i.to_string()});
    }
}
```
