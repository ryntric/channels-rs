#### It is a low-latency rust concurrency library designed around ring buffers, sequencers, and customizable wait strategies. It provides both spsc, mpsc, spmc, mpmc along with batch and single-item publishing modes, to maximize throughput and minimize contention

**Build Environment requirements**
- cargo 1.90.0 or greater
- rustc 1.90.0 

Example of usage
---

```rust
#[derive(Default, Debug)]
struct Event {}

fn main() {
    let (tx, rx) = spmc::<Event>(8192,  ProducerWaitStrategyKind::Spinning, ConsumerWaitStrategyKind::Spinning);
    let is_running = Arc::new(AtomicBool::new(true));

    for _ in 0..4 {
        let rx_clone = rx.clone();
        let is_running_clone = is_running.clone();

        std::thread::spawn(move || {
            let handler: fn (Event) = |e| {
                std::hint::black_box(e);
            };

            while is_running_clone.load(Ordering::Acquire) {
                rx_clone.blocking_recv(1024, &handler)
            }
        });
    }
    
    for _ in 0..100_000 {
        tx.send(Event{})
    }
    is_running.store(false, Ordering::Release);
}
```
