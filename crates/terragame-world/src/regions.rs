use crate::ChunkKey;
use std::collections::{BTreeMap, BTreeSet};
use terragame_core::Tick;
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum RegionEvent {
    Activated {
        key: ChunkKey,
        tick: Tick,
    },
    Deactivated {
        key: ChunkKey,
        since: Tick,
        tick: Tick,
    },
}
/// Games own persisted dormant summaries. The engine retains active keys only.
pub trait RegionLifecycle {
    fn activate(&mut self, key: ChunkKey, tick: Tick);
    fn deactivate(&mut self, key: ChunkKey, since: Tick, tick: Tick);
}
pub struct ActiveRegions {
    radius: u16,
    active: BTreeMap<ChunkKey, Tick>,
}
impl ActiveRegions {
    pub fn new(radius: u16, max_regions: usize) -> Option<Self> {
        let width = usize::from(radius).checked_mul(2)?.checked_add(1)?;
        if width.checked_mul(width)? > max_regions {
            return None;
        }
        Some(Self {
            radius,
            active: BTreeMap::new(),
        })
    }
    pub fn keys(&self) -> impl Iterator<Item = ChunkKey> + '_ {
        self.active.keys().copied()
    }
    pub fn contains(&self, key: ChunkKey) -> bool {
        self.active.contains_key(&key)
    }
    /// Deactivate in key order before activating in key order. Same center is idle.
    pub fn update(&mut self, center: ChunkKey, tick: Tick) -> Vec<RegionEvent> {
        let mut wanted = BTreeSet::new();
        let radius = i64::from(self.radius);
        for dy in -radius..=radius {
            for dx in -radius..=radius {
                if let (Some(x), Some(y)) = (center.0.checked_add(dx), center.1.checked_add(dy)) {
                    wanted.insert((x, y));
                }
            }
        }
        let mut events = Vec::new();
        self.active.retain(|&key, &mut since| {
            if wanted.contains(&key) {
                true
            } else {
                events.push(RegionEvent::Deactivated { key, since, tick });
                false
            }
        });
        for key in wanted {
            if let std::collections::btree_map::Entry::Vacant(entry) = self.active.entry(key) {
                entry.insert(tick);
                events.push(RegionEvent::Activated { key, tick });
            }
        }
        events
    }
    pub fn reconcile(
        &mut self,
        center: ChunkKey,
        tick: Tick,
        lifecycle: &mut impl RegionLifecycle,
    ) {
        for event in self.update(center, tick) {
            match event {
                RegionEvent::Activated { key, tick } => lifecycle.activate(key, tick),
                RegionEvent::Deactivated { key, since, tick } => {
                    lifecycle.deactivate(key, since, tick)
                }
            }
        }
    }
}
