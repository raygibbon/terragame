use std::{num::NonZeroU64, time::Duration};
use terragame_core::{
    Commands, EntityStore, EventQueue, FixedStep, Lifecycle, Scheduler, Simulation,
    entity::{Applied, Command},
    random::{Random, coordinate_hash, mix},
};
use terragame_terminal::{
    Camera, Layer, Layers, Palette, Tile,
    terrabox::{Rect, Size, Style, Surface},
};
use terragame_world::{
    ActiveRegions, Bounds, ChunkCache, ChunkLayout, Point, RegionEvent, RegionLifecycle,
    SpatialIndex,
};
#[path = "../examples/models/sandbox.rs"]
mod sandbox;
#[path = "../examples/models/snake.rs"]
mod snake;
fn nz(n: u64) -> NonZeroU64 {
    NonZeroU64::new(n).unwrap()
}
#[test]
fn fixed_steps_pause_catch_up_and_large_ticks() {
    let mut s = Scheduler::new(3);
    s.register(nz(1), 0).unwrap();
    s.register(nz(4), 0).unwrap();
    s.register(nz(240), 0).unwrap();
    assert!(s.register(nz(1), 0).is_none());
    let mut sim = Simulation::new(FixedStep::new(Duration::from_millis(250), 4).unwrap(), s);
    let mut events = Vec::new();
    assert_eq!(
        sim.advance(Duration::from_millis(249), |e| events.push(e)),
        0
    );
    assert_eq!(sim.advance(Duration::from_millis(1), |e| events.push(e)), 1);
    assert_eq!(events[0].tick, 1);
    assert_eq!(sim.advance(Duration::from_secs(100), |e| events.push(e)), 4);
    assert_eq!(sim.clock.tick(), 5);
    assert_eq!(
        events
            .iter()
            .map(|e| (e.tick, e.system.0))
            .collect::<Vec<_>>(),
        vec![(1, 0), (2, 0), (3, 0), (4, 0), (4, 1), (5, 0)]
    );
    sim.set_state(Lifecycle::Paused);
    assert_eq!(sim.advance(Duration::MAX, |_| panic!()), 0);
    sim.set_state(Lifecycle::Running);
    assert_eq!(sim.advance(Duration::from_millis(249), |_| {}), 0);
    sim.clock.set_tick(u64::MAX - 1);
    assert_eq!(sim.advance(Duration::MAX, |_| {}), 1);
    assert_eq!(sim.clock.tick(), u64::MAX);
    assert_eq!(sim.advance(Duration::MAX, |_| panic!()), 0);
    sim.set_state(Lifecycle::Stopped);
    sim.set_state(Lifecycle::Running);
    assert_eq!(sim.state(), Lifecycle::Stopped);
    assert!(FixedStep::new(Duration::ZERO, 4).is_none());
    assert!(FixedStep::new(Duration::MAX, 2).is_none());
    let mut huge = FixedStep::new(Duration::MAX, 1).unwrap();
    assert_eq!(huge.advance(Duration::MAX), 1);
}
#[test]
fn schedule_order_phase_and_render_partition_independence() {
    fn simulate(parts: &[u64]) -> Vec<(u64, usize)> {
        let mut scheduler = Scheduler::new(3);
        for (period, phase) in [(1, 0), (3, 1), (10, 0)] {
            scheduler.register(nz(period), phase).unwrap();
        }
        let mut sim = Simulation::new(
            FixedStep::new(Duration::from_millis(10), 4).unwrap(),
            scheduler,
        );
        let mut events = Vec::new();
        for &ms in parts {
            sim.advance(Duration::from_millis(ms), |e| {
                events.push((e.tick, e.system.0))
            });
        }
        events
    }
    assert_eq!(simulate(&[40, 40, 20]), simulate(&[10; 10]));
    assert_eq!(simulate(&[7, 3, 20, 30, 40]), simulate(&[10; 10]));
    let mut s = Scheduler::new(1);
    assert!(s.register(nz(3), 3).is_none());
    assert!(s.is_empty());
    s.register(nz(1), 0).unwrap();
    assert_eq!(s.len(), 1);
    s.dispatch(0, |_| panic!());
}
#[test]
fn identity_reuse_stale_ids_and_deferred_backpressure() {
    let mut store = EntityStore::new(2);
    let a = store.spawn("player").unwrap();
    let b = store.spawn("creature").unwrap();
    assert_eq!(store.spawn("resource"), Err("resource"));
    assert_eq!(store.remove(a), Some("player"));
    let c = store.spawn("resource").unwrap();
    assert_eq!(c.slot(), a.slot());
    assert_ne!(c.generation(), a.generation());
    assert!(store.get(a).is_none());
    assert!(store.get_mut(a).is_none());
    assert!(store.remove(a).is_none());
    assert_eq!(
        store.iter().map(|(id, _)| id).collect::<Vec<_>>(),
        vec![c, b]
    );
    let mut commands = Commands::new(3);
    assert!(commands.push(Command::Remove(b)).is_ok());
    assert!(commands.push(Command::Spawn("new")).is_ok());
    assert!(commands.push(Command::Remove(b)).is_ok());
    assert!(commands.push(Command::Spawn("full")).is_err());
    let mut spawned = None;
    let mut stale = 0;
    commands.apply(&mut store, |e| match e {
        Applied::Spawned(id) => spawned = Some(id),
        Applied::Stale(_) => stale += 1,
        _ => {}
    });
    assert_eq!(stale, 1);
    assert_eq!(store.get(spawned.unwrap()), Some(&"new"));
    assert_eq!(store.allocated_slots(), 2);
    for (_, v) in store.iter_mut() {
        *v = "owned";
    }
    assert!(store.iter().all(|(_, v)| *v == "owned"));
    let mut q = EventQueue::new(2);
    assert_eq!(q.push(1), Ok(()));
    q.push(2).unwrap();
    assert_eq!(q.push(3), Err(3));
    assert_eq!(q.pop(), Some(1));
    assert_eq!(q.drain().collect::<Vec<_>>(), vec![2]);
    assert!(q.is_empty());
    assert_eq!(q.len(), 0);
}
#[test]
fn stable_random_golden_values_and_coordinate_streams() {
    let mut rng = Random::new(0);
    assert_eq!(rng.next_u64(), 0xe220a8397b1dcdaf);
    assert_eq!(rng.next_u64(), 0x6e789e6aa1b965f4);
    let mut clone = Random::new(rng.state());
    for _ in 0..100 {
        assert_eq!(rng.next_u64(), clone.next_u64());
    }
    assert_eq!(rng.below(0), None);
    for _ in 0..100 {
        assert!(rng.below(7).unwrap() < 7);
    }
    let old = |seed, x: i64, y: i64| {
        mix(seed ^ mix(x as u64) ^ mix((y as u64).wrapping_add(0x9e3779b97f4a7c15)))
    };
    for p in [
        Point::new(i64::MIN, i64::MAX),
        Point::new(-1, -32),
        Point::new(0, 0),
    ] {
        assert_eq!(coordinate_hash(42, p.x, p.y), old(42, p.x, p.y));
    }
}
#[test]
fn chunks_roundtrip_negative_and_extreme_coordinates() {
    assert!(ChunkLayout::new(0).is_none());
    for side in [1, 3, 16, 32, 65535] {
        let l = ChunkLayout::new(side).unwrap();
        for x in [i64::MIN, -33, -32, -1, 0, 31, 32, i64::MAX] {
            for y in [i64::MIN, -1, 0, i64::MAX] {
                let p = Point::new(x, y);
                let (key, index) = l.locate(p);
                assert_eq!(l.world_at(key, index), Some(p));
                assert!(index < l.cells());
            }
        }
        assert!(l.world_at((0, 0), l.cells()).is_none());
    }
    assert_eq!(
        ChunkLayout::new(32).unwrap().locate(Point::new(-1, -1)),
        ((-1, -1), 1023)
    );
    assert!(Point::new(i64::MAX, 0).offset(1, 0).is_none());
}
#[test]
fn cache_limits_tie_order_eviction_and_recreation() {
    let mut c = ChunkCache::new(2);
    c.get_or_insert_with((-1, 0), |k| k);
    c.get_or_insert_with((1, 0), |k| k);
    c.get_or_insert_with((0, 0), |k| k);
    assert!(c.contains((-1, 0)));
    assert!(!c.contains((1, 0)));
    assert_eq!(c.len(), 2);
    assert_eq!(c.get_or_insert_with((0, 0), |_| panic!()), &(0, 0));
    assert_eq!(c.stats().hits, 1);
    assert_eq!(c.stats().evictions, 1);
    c.focus((i64::MIN, i64::MAX));
    c.get_or_insert_with((i64::MIN, i64::MAX), |k| k);
    assert_eq!(c.len(), 2);
    let mut one = ChunkCache::new(0);
    assert!(one.is_empty());
    for n in 0..100 {
        one.get_or_insert_with((n, 0), |_| 42);
    }
    assert_eq!(one.len(), 1);
}
#[test]
fn spatial_cross_boundary_move_remove_and_sorted_queries() {
    let mut store = EntityStore::new(4);
    let ids: Vec<_> = (0..4).map(|i| store.spawn(i).unwrap()).collect();
    let mut index = SpatialIndex::new(ChunkLayout::new(16).unwrap(), 4);
    for (&id, p) in ids.iter().zip([
        Point::new(-1, -1),
        Point::new(16, 0),
        Point::new(i64::MAX, i64::MIN),
        Point::new(2, 3),
    ]) {
        assert!(index.insert(id, p));
    }
    assert!(!index.insert(ids[0], Point::new(0, 0)));
    assert_eq!(index.query_radius(Point::new(0, 0), 2), vec![ids[0]]);
    assert!(index.move_to(ids[0], Point::new(16, 1)));
    assert_eq!(
        index.query_rect(Bounds::new(Point::new(16, 0), Point::new(16, 1)).unwrap()),
        vec![ids[0], ids[1]]
    );
    assert!(index.query_radius(Point::new(-1, -1), 0).is_empty());
    assert_eq!(index.remove(ids[0]), Some(Point::new(16, 1)));
    assert!(!index.move_to(ids[0], Point::new(0, 0)));
    let all = index.query_radius(Point::new(0, 0), u64::MAX);
    assert_eq!(all, vec![ids[1], ids[2], ids[3]]);
    assert_eq!(index.len(), 3);
    assert!(!index.is_empty());
    assert_eq!(
        index.query_radius(Point::new(i64::MIN, i64::MAX), u64::MAX),
        vec![ids[1], ids[3]]
    );
}
#[test]
fn spatial_index_matches_reference_for_moves_and_extreme_radius() {
    let mut store = EntityStore::new(300);
    let mut index = SpatialIndex::new(ChunkLayout::new(7).unwrap(), 300);
    for n in 0..300 {
        let id = store.spawn(n).unwrap();
        index.insert(id, Point::new(n * 3 - 450, n % 23 - 11));
    }
    for x in (-400..400).step_by(13) {
        let center = Point::new(x, 0);
        let expected: Vec<_> = store
            .iter()
            .filter(|(id, _)| {
                let p = index.position(*id).unwrap();
                let dx = i128::from(p.x) - i128::from(x);
                let dy = i128::from(p.y);
                dx * dx + dy * dy <= 31 * 31
            })
            .map(|(id, _)| id)
            .collect();
        assert_eq!(index.query_radius(center, 31), expected);
    }
}
#[test]
fn active_regions_suspend_and_resume_through_lifecycle() {
    #[derive(Default)]
    struct Listener(Vec<RegionEvent>);
    impl RegionLifecycle for Listener {
        fn activate(&mut self, key: (i64, i64), tick: u64) {
            self.0.push(RegionEvent::Activated { key, tick });
        }
        fn deactivate(&mut self, key: (i64, i64), since: u64, tick: u64) {
            self.0.push(RegionEvent::Deactivated { key, since, tick });
        }
    }
    assert!(ActiveRegions::new(20, 10).is_none());
    let mut r = ActiveRegions::new(1, 9).unwrap();
    let mut l = Listener::default();
    r.reconcile((0, 0), 0, &mut l);
    assert_eq!(l.0.len(), 9);
    assert!(r.contains((-1, -1)));
    assert!(r.update((0, 0), 10).is_empty());
    let e = r.update((1, 0), 20);
    assert_eq!(e.len(), 6);
    assert!(matches!(
        e[0],
        RegionEvent::Deactivated {
            since: 0,
            tick: 20,
            ..
        }
    ));
    assert_eq!(r.keys().count(), 9);
    r.update((i64::MAX, i64::MIN), u64::MAX);
    assert_eq!(r.keys().count(), 4);
}
#[test]
fn camera_extremes_clipping_layers_palette_and_resize() {
    let style = Style::default();
    let mut layers = Layers::new(
        Size {
            width: 8,
            height: 6,
        },
        100,
    )
    .unwrap();
    let camera = Camera::following(Point::new(i64::MAX, i64::MIN), Rect::new(1, 2, 6, 3));
    let p = Point::new(i64::MAX, i64::MIN);
    assert_eq!(camera.project(p).unwrap().x, 6);
    assert_eq!(camera.project(p).unwrap().y, 2);
    assert!(camera.local(Point::new(0, 0)).is_none());
    assert_eq!(camera.world_at(5, 0), p);
    assert!(Tile::new('界', style).is_none());
    assert!(Tile::new('\n', style).is_none());
    assert_eq!(Tile::with_fallback('界', b'?', style).unwrap().glyph(), '?');
    for layer in [
        Layer::Background,
        Layer::Terrain,
        Layer::Structures,
        Layer::Items,
        Layer::Entities,
        Layer::Effects,
        Layer::Overlay,
    ] {
        assert!(layers.put_world(
            layer,
            camera,
            p,
            Tile::ascii(b'A' + layer as u8, style).unwrap()
        ));
    }
    assert!(!layers.put(Layer::Overlay, 8, 0, Tile::ascii(b'x', style).unwrap()));
    let mut surface = Surface::new(Size {
        width: 1,
        height: 1,
    });
    layers.compose(&mut surface, style);
    assert_eq!(surface.cell(6, 2).unwrap().scalar(), 'G');
    assert!(!layers.resize(Size {
        width: 100,
        height: 100
    }));
    assert_eq!(layers.size().width, 8);
    assert!(layers.resize(Size {
        width: 2,
        height: 2
    }));
    layers.compose(&mut surface, style);
    assert_eq!(surface.size().width, 2);
    assert_eq!(surface.cell(0, 0).unwrap().scalar(), ' ');
    let mut palette = Palette::new(1);
    let tile = Tile::ascii(b'.', style).unwrap();
    assert_eq!(palette.add(tile), Some(0));
    assert_eq!(palette.add(tile), None);
    assert_eq!(palette.get(0), Some(tile));
    assert!(palette.replace(0, Tile::ascii(b'#', style).unwrap()));
    assert!(
        Layers::new(
            Size {
                width: 0,
                height: 0
            },
            0
        )
        .is_some()
    );
}
#[test]
fn snake_scoring_collision_pause_and_restart() {
    let mut game = snake::Snake::default();
    for _ in 0..4 {
        game.step();
    }
    assert_eq!(game.score, 1);
    assert_eq!(game.body.len(), 4);
    game.paused = true;
    let before = game.body.clone();
    game.step();
    assert_eq!(game.body, before);
    game.paused = false;
    game.turn((-1, 0));
    game.step();
    assert_eq!(game.body[0].x, 13);
    for _ in 0..40 {
        game.step();
    }
    assert!(game.dead);
    assert!(!snake::Snake::default().dead);
}
#[test]
fn sandbox_multirate_dormancy_and_region_resume() {
    let mut a = sandbox::Sandbox::default();
    let mut b = sandbox::Sandbox::default();
    let original = a.entities.len();
    assert_eq!(original, 55);
    for tick in 1..=100 {
        a.update(tick);
        b.update(tick);
    }
    assert_eq!(a.environment_updates, 10);
    assert_eq!(a.coarse_updates, 1);
    let snapshot = |s: &sandbox::Sandbox| {
        s.entities
            .iter()
            .map(|(id, o)| (id, o.kind, s.spatial.position(id)))
            .collect::<Vec<_>>()
    };
    assert_eq!(snapshot(&a), snapshot(&b));
    let start = a.position();
    a.paused = true;
    a.move_player(1, 0);
    assert_eq!(a.position(), start);
    a.gather();
    a.paused = false;
    for _ in 0..16 {
        a.move_player(1, 0);
    }
    assert_eq!(a.activations, 12);
    assert_eq!(a.deactivations, 3);
    assert_eq!(a.entities.len(), original);
    assert_eq!(a.dormant_count(), 3);
    for _ in 0..16 {
        a.move_player(-1, 0);
    }
    assert_eq!(a.entities.len(), original);
    assert_eq!(a.position(), start);
}

#[test]
fn quit_binding_accepts_decoded_control_letters_and_ignores_alt() {
    use terragame_terminal::{
        quit_key,
        terrabox::{Key, KeyEvent, Modifiers},
    };
    for letter in ['c', 'C'] {
        assert!(quit_key(KeyEvent {
            key: Key::Char(letter),
            modifiers: Modifiers::CTRL
        }));
    }
    assert!(!quit_key(KeyEvent {
        key: Key::Char('q'),
        modifiers: Modifiers::ALT
    }));
}
