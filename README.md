#### It is a low-latency rust concurrency library designed around ring buffers, sequencers, and customizable wait strategies. It provides spsc, mpsc, spmc, mpmc along with batch and single-item publishing modes, to maximize throughput and minimize contention

**Build Environment requirements**
- cargo 1.90.0 or greater
- rustc 1.90.0 

---
#### To build and run bench execute the following commands:
```shell
cargo build --release
cargo bench
```

#### If you want to run with thread affinity execute the following command:
```shell
taskset -c <corerange> cargo bench 
```

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

---
## Benchmark

### Hardware information:

#### OS:

```text
NAME="Fedora Linux"
VERSION="42 (KDE Plasma Desktop Edition)"
RELEASE_TYPE=stable
ID=fedora
VERSION_ID=42
VERSION_CODENAME=""
PLATFORM_ID="platform:f42"
PRETTY_NAME="Fedora Linux 42 (KDE Plasma Desktop Edition)"
CPE_NAME="cpe:/o:fedoraproject:fedora:42"
SUPPORT_END=2026-05-13
VARIANT="KDE Plasma Desktop Edition"
VARIANT_ID=kde
```

#### CPU:

```text
Architecture:                x86_64
  CPU op-mode(s):            32-bit, 64-bit
  Address sizes:             48 bits physical, 48 bits virtual
  Byte Order:                Little Endian
CPU(s):                      12
  On-line CPU(s) list:       0-11
Vendor ID:                   AuthenticAMD
  Model name:                AMD Ryzen 9 5900X 12-Core Processor
    CPU family:              25
    Model:                   33
    Thread(s) per core:      1
    Core(s) per socket:      12
    Socket(s):               1
    Stepping:                2
    Frequency boost:         disabled
    CPU(s) scaling MHz:      61%
    CPU max MHz:             6291.9360
    CPU min MHz:             720.1620
    BogoMIPS:                9399.78
    Flags:                   fpu vme de pse tsc msr pae mce cx8 apic sep mtrr pge mca cmov pat pse36 clflush mmx fxsr sse sse2 ht syscall nx mmxext fxsr_opt pdpe1gb rdt
                             scp lm constant_tsc rep_good nopl xtopology nonstop_tsc cpuid extd_apicid aperfmperf rapl pni pclmulqdq monitor ssse3 fma cx16 sse4_1 sse4_
                             2 x2apic movbe popcnt aes xsave avx f16c rdrand lahf_lm cmp_legacy svm extapic cr8_legacy abm sse4a misalignsse 3dnowprefetch osvw ibs skin
                             it wdt tce topoext perfctr_core perfctr_nb bpext perfctr_llc mwaitx cat_l3 cdp_l3 hw_pstate ssbd mba ibrs ibpb stibp vmmcall fsgsbase bmi1 
                             avx2 smep bmi2 erms invpcid cqm rdt_a rdseed adx smap clflushopt clwb sha_ni xsaveopt xsavec xgetbv1 xsaves cqm_llc cqm_occup_llc cqm_mbm_t
                             otal cqm_mbm_local user_shstk clzero irperf xsaveerptr rdpru wbnoinvd arat npt lbrv svm_lock nrip_save tsc_scale vmcb_clean flushbyasid dec
                             odeassists pausefilter pfthreshold avic v_vmsave_vmload vgif v_spec_ctrl umip pku ospke vaes vpclmulqdq rdpid overflow_recov succor smca fs
                             rm debug_swap
Virtualization features:     
  Virtualization:            AMD-V
Caches (sum of all):         
  L1d:                       384 KiB (12 instances)
  L1i:                       384 KiB (12 instances)
  L2:                        6 MiB (12 instances)
  L3:                        64 MiB (2 instances)
NUMA:                        
  NUMA node(s):              1
  NUMA node0 CPU(s):         0-11
Vulnerabilities:             
  Gather data sampling:      Not affected
  Ghostwrite:                Not affected
  Indirect target selection: Not affected
  Itlb multihit:             Not affected
  L1tf:                      Not affected
  Mds:                       Not affected
  Meltdown:                  Not affected
  Mmio stale data:           Not affected
  Reg file data sampling:    Not affected
  Retbleed:                  Not affected
  Spec rstack overflow:      Mitigation; Safe RET
  Spec store bypass:         Mitigation; Speculative Store Bypass disabled via prctl
  Spectre v1:                Mitigation; usercopy/swapgs barriers and __user pointer sanitization
  Spectre v2:                Mitigation; Retpolines; IBPB conditional; IBRS_FW; STIBP disabled; RSB filling; PBRSB-eIBRS Not affected; BHI Not affected
  Srbds:                     Not affected
  Tsa:                       Mitigation; Clear CPU buffers
  Tsx async abort:           Not affected

```

**Results:**

| Benchmark                                                 | Mode  | Score          | time      |
|-----------------------------------------------------------| ------|----------------|-----------|
| single_producer_multi_consumer_batch_item_bench.rs        | thrpt | 1.4613 Gelem/s | 5.4822 ns |
| single_producer_multi_consumer_single_item_bench.rs       | thrpt | 151.66 Melem/s | 6.6091 ns |
| single_producer_single_consumer_batch_item_bench.rs       | thrpt | 658.39 Melem/s | 12.156 ns |
| single_producer_single_consumer_single_item_bench.rs      | thrpt | 215.40 Melem/s | 4.6439 ns |
