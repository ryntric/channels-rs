#### It is a low-latency rust concurrency library designed around ring buffers, sequencers, and customizable wait strategies. It provides both single-producer and multi-producer configurations, along with batch and single-item publishing modes, to maximize throughput and minimize contention

**Build Environment requirements**
- cargo 1.90.0 or greater
- rustc 1.90.0 

Example of usage
---

```rust
#[derive(Default, Debug)]
struct Event {}

fn main() {
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
    
    for i in 0..1_000_000 {
        tx.send_n(Event{})
    }
}
```
