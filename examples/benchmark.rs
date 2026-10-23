//! Repeatable CPU workloads. Terminal I/O and emulator painting are excluded.
use std::{hint::black_box, time::Instant};
use terragame_core::{EntityStore, FixedStep, Scheduler};
use terragame_terminal::{
    Camera, Layer, Layers, Tile,
    terrabox::{Rect, Size, Style, Surface},
};
use terragame_world::{Bounds, ChunkCache, ChunkLayout, Point, SpatialIndex};
fn main() {
    let mut store = EntityStore::new(10000);
    let start = Instant::now();
    for _ in 0..10 {
        let ids: Vec<_> = (0..10000).map(|n| store.spawn(n as u64).unwrap()).collect();
        for id in ids {
            black_box(store.remove(id));
        }
    }
    println!(
        "entity churn: 100000 spawn/remove pairs in {:?}",
        start.elapsed()
    );
    let ids: Vec<_> = (0..10000).map(|n| store.spawn(n as u64).unwrap()).collect();
    let start = Instant::now();
    for _ in 0..100 {
        for (id, value) in store.iter() {
            black_box((id, value));
        }
    }
    println!("entity iteration: 1000000 visits in {:?}", start.elapsed());
    println!(
        "entity storage bytes: {} (Vec capacity, excluding store header)",
        store.storage_bytes()
    );
    let layout = ChunkLayout::new(16).unwrap();
    let mut spatial = SpatialIndex::new(layout, 10000);
    for (n, &id) in ids.iter().enumerate() {
        assert!(spatial.insert(
            id,
            Point::new((n % 100) as i64 * 4 - 200, (n / 100) as i64 * 4 - 200)
        ));
    }
    let start = Instant::now();
    for n in 0..10000 {
        black_box(spatial.query_radius(Point::new(n % 200 - 100, n % 117 - 58), 12));
    }
    println!(
        "spatial radius: 10000 queries across 10000 objects in {:?}",
        start.elapsed()
    );
    let start = Instant::now();
    for n in 0..10000 {
        black_box(spatial.query_rect(
            Bounds::new(Point::new(n % 100 - 50, -25), Point::new(n % 100 - 25, 25)).unwrap(),
        ));
    }
    println!(
        "spatial rectangle: 10000 queries across 10000 objects in {:?}",
        start.elapsed()
    );
    let start = Instant::now();
    for round in 0..10 {
        for (n, &id) in ids.iter().enumerate() {
            black_box(spatial.move_to(id, Point::new(n as i64 * 16 + round, round * 16)));
        }
    }
    println!(
        "spatial movement: 100000 cross-chunk moves in {:?}",
        start.elapsed()
    );
    let mut cache = ChunkCache::new(128);
    let start = Instant::now();
    for n in 0..10000 {
        cache.focus((n, 0));
        black_box(cache.get_or_insert_with((n, 0), |k| vec![k.0 as u8; 1024]));
    }
    println!(
        "cache misses: 10000 generated/evicted chunks in {:?}",
        start.elapsed()
    );
    let start = Instant::now();
    for _ in 0..1000000 {
        black_box(cache.get_or_insert_with((9999, 0), |_| panic!()));
    }
    println!("cache hits: 1000000 lookups in {:?}", start.elapsed());
    println!(
        "cache retained: {}; counters {:?}; payload bytes {}",
        cache.len(),
        cache.stats(),
        cache.len() * 1024
    );
    let mut schedule = Scheduler::new(6);
    for period in [1, 4, 240, 14400, 345600, 1000000] {
        schedule.register(period.try_into().unwrap(), 0).unwrap();
    }
    let mut due = 0u64;
    let start = Instant::now();
    for tick in 1..=1000000 {
        schedule.dispatch(tick, |e| {
            due += 1;
            black_box(e);
        });
    }
    println!(
        "scheduler: 1000000 ticks / 6 rates in {:?}; {} dispatches",
        start.elapsed(),
        due
    );
    let mut fixed = FixedStep::new(std::time::Duration::from_millis(10), 4).unwrap();
    let start = Instant::now();
    for _ in 0..1000000 {
        black_box(fixed.advance(std::time::Duration::from_millis(11)));
    }
    println!("fixed step: 1000000 samples in {:?}", start.elapsed());
    let size = Size {
        width: 100,
        height: 30,
    };
    let mut layers = Layers::new(size, 3000).unwrap();
    let mut surface = Surface::new(size);
    let tile = Tile::ascii(b'.', Style::default()).unwrap();
    for y in 0..30 {
        for x in 0..100 {
            layers.put(Layer::Terrain, x, y, tile);
        }
    }
    let start = Instant::now();
    for _ in 0..1000 {
        layers.compose(&mut surface, Style::default());
        black_box(&surface);
    }
    println!(
        "tile composition: 1000 frames at 100x30 in {:?}",
        start.elapsed()
    );
    println!("layer storage bytes: {}", layers.storage_bytes());
    let start = Instant::now();
    for x in 0..1000 {
        layers.clear();
        let c = Camera::following(Point::new(x, 0), Rect::new(0, 0, 100, 30));
        for y in 0..30 {
            for sx in 0..100 {
                let p = c.world_at(sx, y);
                layers.put_world(Layer::Terrain, c, p, tile);
            }
        }
        layers.compose(&mut surface, Style::default());
        black_box(&surface);
    }
    println!(
        "camera redraw: 1000 moving frames at 100x30 in {:?}",
        start.elapsed()
    );
    if let Ok(status) = std::fs::read_to_string("/proc/self/status") {
        for line in status
            .lines()
            .filter(|s| s.starts_with("VmRSS:") || s.starts_with("VmHWM:"))
        {
            println!("{line}");
        }
    }
}
