use std::collections::BTreeMap;
use terragame_core::{EntityId, EntityStore, Scheduler, SystemId, random::coordinate_hash};
use terragame_world::{ActiveRegions, ChunkKey, ChunkLayout, Point, RegionEvent, SpatialIndex};
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Kind {
    Player,
    Creature,
    Resource,
}
#[derive(Clone, Copy, Debug)]
pub struct Object {
    pub kind: Kind,
    pub home: ChunkKey,
}
#[derive(Clone)]
struct Summary {
    tick: u64,
    objects: Vec<(Kind, Point)>,
}
/// A bounded demo ledger, not a claim of persistent off-screen simulation.
pub struct Sandbox {
    pub entities: EntityStore<Object>,
    pub spatial: SpatialIndex,
    pub player: EntityId,
    pub regions: ActiveRegions,
    pub tick: u64,
    pub nearby: usize,
    pub gathered: u64,
    pub environment_updates: u64,
    pub coarse_updates: u64,
    pub paused: bool,
    pub scheduler: Scheduler,
    dormant: BTreeMap<ChunkKey, Summary>,
    pub activations: u64,
    pub deactivations: u64,
}
impl Default for Sandbox {
    fn default() -> Self {
        let layout = ChunkLayout::new(16).unwrap();
        let mut entities = EntityStore::new(128);
        let mut spatial = SpatialIndex::new(layout, 128);
        let player = entities
            .spawn(Object {
                kind: Kind::Player,
                home: (0, 0),
            })
            .unwrap();
        assert!(spatial.insert(player, Point::new(0, 0)));
        let mut scheduler = Scheduler::new(3);
        for period in [1, 10, 100] {
            scheduler.register(period.try_into().unwrap(), 0).unwrap();
        }
        let mut s = Self {
            entities,
            spatial,
            player,
            regions: ActiveRegions::new(1, 9).unwrap(),
            tick: 0,
            nearby: 0,
            gathered: 0,
            environment_updates: 0,
            coarse_updates: 0,
            paused: false,
            scheduler,
            dormant: BTreeMap::new(),
            activations: 0,
            deactivations: 0,
        };
        s.activate();
        s
    }
}
impl Sandbox {
    pub fn position(&self) -> Point {
        self.spatial.position(self.player).unwrap()
    }
    pub fn dormant_count(&self) -> usize {
        self.dormant.len()
    }
    pub fn move_player(&mut self, dx: i64, dy: i64) {
        if self.paused {
            return;
        }
        if let Some(next) = self.position().offset(dx, dy) {
            self.spatial.move_to(self.player, next);
            self.activate();
        }
    }
    pub fn gather(&mut self) {
        if self.paused {
            return;
        }
        for id in self.spatial.query_radius(self.position(), 2) {
            if self
                .entities
                .get(id)
                .is_some_and(|o| o.kind == Kind::Resource)
            {
                self.spatial.remove(id);
                self.entities.remove(id);
                self.gathered += 1;
            }
        }
    }
    fn activate(&mut self) {
        let layout = ChunkLayout::new(16).unwrap();
        let center = layout.locate(self.position()).0;
        for event in self.regions.update(center, self.tick) {
            match event {
                RegionEvent::Deactivated { key, tick, .. } => {
                    let ids: Vec<_> = self
                        .entities
                        .iter()
                        .filter(|(_, o)| o.kind != Kind::Player && o.home == key)
                        .map(|(id, _)| id)
                        .collect();
                    let mut objects = Vec::new();
                    for id in ids {
                        let p = self.spatial.remove(id).unwrap();
                        let o = self.entities.remove(id).unwrap();
                        objects.push((o.kind, p));
                    }
                    if self.dormant.len() == 64 {
                        let farthest = self
                            .dormant
                            .keys()
                            .max_by_key(|k| {
                                (i128::from(k.0) - i128::from(center.0))
                                    .abs()
                                    .max((i128::from(k.1) - i128::from(center.1)).abs())
                            })
                            .copied()
                            .unwrap();
                        self.dormant.remove(&farthest);
                    }
                    self.dormant.insert(key, Summary { tick, objects });
                    self.deactivations += 1;
                }
                RegionEvent::Activated { key, .. } => {
                    let objects = if let Some(summary) = self.dormant.remove(&key) {
                        let _dormant_ticks = self.tick.saturating_sub(summary.tick);
                        summary.objects
                    } else {
                        (0..6)
                            .filter_map(|slot| {
                                let h = coordinate_hash(71 + slot, key.0, key.1);
                                let index = ((h >> 8) % 16) * 16 + (h % 16);
                                layout.world_at(key, index as usize).map(|p| {
                                    (
                                        if slot < 4 {
                                            Kind::Creature
                                        } else {
                                            Kind::Resource
                                        },
                                        p,
                                    )
                                })
                            })
                            .collect()
                    };
                    for (kind, p) in objects {
                        let id = self.entities.spawn(Object { kind, home: key }).unwrap();
                        assert!(self.spatial.insert(id, p));
                    }
                    self.activations += 1;
                }
            }
        }
    }
    pub fn update(&mut self, tick: u64) {
        self.tick = tick;
        let mut due = Vec::new();
        self.scheduler.dispatch(tick, |s| due.push(s.system));
        for system in due {
            match system {
                SystemId(0) => {
                    let ids: Vec<_> = self
                        .entities
                        .iter()
                        .filter(|(_, o)| o.kind == Kind::Creature)
                        .map(|(id, _)| id)
                        .collect();
                    for id in ids {
                        let p = self.spatial.position(id).unwrap();
                        let h = coordinate_hash(tick ^ id.generation(), p.x, p.y);
                        let (dx, dy) = [(1, 0), (-1, 0), (0, 1), (0, -1)][(h % 4) as usize];
                        if let Some(next) = p.offset(dx, dy) {
                            let home = self.entities.get(id).unwrap().home;
                            if ChunkLayout::new(16).unwrap().locate(next).0 == home
                                && self.spatial.query_radius(next, 0).is_empty()
                            {
                                self.spatial.move_to(id, next);
                            }
                        }
                    }
                }
                SystemId(1) => {
                    self.environment_updates += 1;
                    self.nearby = self
                        .spatial
                        .query_radius(self.position(), 8)
                        .len()
                        .saturating_sub(1);
                }
                SystemId(2) => {
                    self.coarse_updates += 1;
                }
                _ => unreachable!(),
            }
        }
    }
}
