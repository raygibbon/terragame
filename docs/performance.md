# Performance and memory

Measured on 2026-10-23, Linux x86_64, `rustc 1.97.1 (8bab26f4f 2026-07-14)` in release mode.
Five consecutive runs of the same executable are recorded in
[benchmark.json](benchmark.json). This shared desktop had variable load; ranges
are observations, not confidence intervals or portable performance guarantees.

```sh
cargo run --release -p terragame-terminal --example benchmark
target/release/examples/benchmark  # subsequent samples of the same build
```

| Workload | Median (ms) | Min–max (ms) |
|---|---:|---:|
| 100,000 spawn/remove pairs | 1.460 | 1.197–2.403 |
| 1,000,000 entity visits | 1.434 | 1.225–2.944 |
| 10,000 radius queries / 10,000 objects | 90.228 | 85.252–116.869 |
| 10,000 rectangle queries / 10,000 objects | 91.755 | 89.319–94.729 |
| 100,000 cross-chunk moves | 83.974 | 77.109–92.918 |
| 10,000 generated/evicted 1 KiB chunks | 7.963 | 7.298–8.796 |
| 1,000,000 cache hits | 80.081 | 77.190–82.394 |
| 1,000,000 ticks / 6 rates | 19.323 | 18.923–21.489 |
| 1,000,000 elapsed-time samples | 25.999 | 24.618–29.783 |
| 1,000 layered frames, 100×30 | 264.572 | 248.472–311.243 |
| 1,000 moving camera frames, 100×30 | 299.320 | 277.540–387.031 |

The arena churn reuses 10,000 slots over ten rounds. Iteration visits all 10,000
live objects 100 times. Spatial queries use a 100×100 grid spaced four cells
apart and exact sorted results. Movement then relocates 10,000 IDs across chunks
ten times. The cache retains 128 chunks; counters end at 1,000,000 hits, 10,000
misses and 9,872 evictions. The scheduler has periods 1, 4, 240, 14,400, 345,600
and 1,000,000 and emits 1,254,238 system updates over a million ticks. The render
workload composes seven possible layers into a Terrabox Surface; moving frames
also clear/refill terrain through camera projection. Black-box barriers keep
observable workloads from being optimized away.

## Memory observations

- Arena Vec capacity for 10,000 `u64` payloads plus its free list: 458,752 bytes
  (448 KiB), excluding headers and allocator bookkeeping. Retained free-list
  capacity is counted even when empty.
- Seven 100×30 layer arrays: 504,000 bytes (about 492 KiB), excluding their Vec
  headers and the separate Terrabox Surface/front/back storage.
- 128 cached 1 KiB chunk payloads: 131,072 bytes (128 KiB), excluding BTreeMap
  nodes, Vec headers and allocator overhead.
- Linux process peak RSS across the complete benchmark:
  10,468–10,588 KiB. This includes code, libraries, all retained arena/index/cache
  data and rendering buffers; it is not solely engine heap usage.

These estimates apply to the benchmark payload types. The engine does not bound
heap allocations hidden inside a game-owned `T`, result consumers or game logs.
Entity/index/region/cache/queue and layer limits are explicit, but callers must
choose realistic limits and persist dormant state themselves.

## Findings and limits

A median layered composition is about 0.265 ms; camera movement plus refill and
composition about 0.299 ms. Radius queries average about 9 microseconds at this
density; this changes with occupied buckets and result count. No measured CPU
workload justified threads, an async runtime, a new cache policy or a heavier ECS.
The old distance eviction policy scans its small bounded cache on misses.
Spatial range lookups avoid global entity scans; very broad queries still visit
all loaded results. The arena scans allocated slots during deterministic
iteration, so sparse high-water arenas can cost more than dense live lists.

The driver skips composition when ticks report no visual changes. Terrabox owns
changed-cell output. Pause removes deadlines and blocks; PTY tests measured zero
CPU ticks and zero output in 600 ms paused windows. These short samples are not
a long-duration energy/power guarantee.

All timings exclude actual terminal writes, emulator rasterization, SSH latency
and slow-link throughput. Real PTY tests separately verify interaction and
restoration. There is no comparative speedup claim, test of millions of
persistent objects, or assumption that dormant objects are continuously updated.
