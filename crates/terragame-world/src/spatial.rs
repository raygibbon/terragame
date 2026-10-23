use crate::{Bounds, ChunkLayout, Point};
use std::collections::{BTreeMap, BTreeSet};
use terragame_core::EntityId;
/// Ordered two-level chunk index: queries visit occupied chunks within the box,
/// never every entity or every empty coordinate. Results are sorted by full ID.
pub struct SpatialIndex {
    layout: ChunkLayout,
    positions: BTreeMap<EntityId, Point>,
    columns: BTreeMap<i64, BTreeMap<i64, BTreeSet<EntityId>>>,
    limit: usize,
}
impl SpatialIndex {
    pub fn new(layout: ChunkLayout, limit: usize) -> Self {
        Self {
            layout,
            limit,
            positions: BTreeMap::new(),
            columns: BTreeMap::new(),
        }
    }
    pub fn len(&self) -> usize {
        self.positions.len()
    }
    pub fn is_empty(&self) -> bool {
        self.positions.is_empty()
    }
    pub fn position(&self, id: EntityId) -> Option<Point> {
        self.positions.get(&id).copied()
    }
    /// False for duplicate IDs or a full index. No partial mutation.
    pub fn insert(&mut self, id: EntityId, point: Point) -> bool {
        if self.len() >= self.limit || self.positions.contains_key(&id) {
            return false;
        }
        let (key, _) = self.layout.locate(point);
        self.columns
            .entry(key.0)
            .or_default()
            .entry(key.1)
            .or_default()
            .insert(id);
        self.positions.insert(id, point);
        true
    }
    pub fn remove(&mut self, id: EntityId) -> Option<Point> {
        let point = self.positions.remove(&id)?;
        let (key, _) = self.layout.locate(point);
        let column = self.columns.get_mut(&key.0).expect("indexed column");
        let bucket = column.get_mut(&key.1).expect("indexed bucket");
        bucket.remove(&id);
        if bucket.is_empty() {
            column.remove(&key.1);
        }
        if column.is_empty() {
            self.columns.remove(&key.0);
        }
        Some(point)
    }
    pub fn move_to(&mut self, id: EntityId, point: Point) -> bool {
        let Some(previous) = self.position(id) else {
            return false;
        };
        if self.layout.locate(previous).0 == self.layout.locate(point).0 {
            self.positions.insert(id, point);
            return true;
        }
        self.remove(id);
        self.insert(id, point)
    }
    pub fn query_rect(&self, bounds: Bounds) -> Vec<EntityId> {
        if bounds.min.x > bounds.max.x || bounds.min.y > bounds.max.y {
            return Vec::new();
        }
        let (min, _) = self.layout.locate(bounds.min);
        let (max, _) = self.layout.locate(bounds.max);
        let mut found = Vec::new();
        for (_, column) in self.columns.range(min.0..=max.0) {
            for (_, bucket) in column.range(min.1..=max.1) {
                found.extend(
                    bucket
                        .iter()
                        .copied()
                        .filter(|id| bounds.contains(self.positions[id])),
                );
            }
        }
        found.sort_unstable();
        found
    }
    /// Euclidean radius, using overflow-safe integer arithmetic even at u64::MAX.
    pub fn query_radius(&self, center: Point, radius: u64) -> Vec<EntityId> {
        let r2 = u128::from(radius) * u128::from(radius);
        self.query_rect(Bounds::around(center, radius))
            .into_iter()
            .filter(|id| {
                let p = self.positions[id];
                let dx = (i128::from(p.x) - i128::from(center.x)).unsigned_abs();
                let dy = (i128::from(p.y) - i128::from(center.y)).unsigned_abs();
                dx <= u128::from(radius)
                    && dy <= u128::from(radius)
                    && dx * dx <= r2
                    && dy * dy <= r2 - dx * dx
            })
            .collect()
    }
}
