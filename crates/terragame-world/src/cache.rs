use crate::ChunkKey;
use std::collections::BTreeMap;
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct CacheStats {
    pub hits: u64,
    pub misses: u64,
    pub evictions: u64,
}
/// Distance eviction, with greatest key breaking ties. Capacity at least one.
/// Immutable content only: mutable depletion/entities belong to the game's ledger.
pub struct ChunkCache<T> {
    chunks: BTreeMap<ChunkKey, T>,
    focus: ChunkKey,
    capacity: usize,
    stats: CacheStats,
}
impl<T> ChunkCache<T> {
    pub fn new(capacity: usize) -> Self {
        Self {
            chunks: BTreeMap::new(),
            focus: (0, 0),
            capacity: capacity.max(1),
            stats: CacheStats::default(),
        }
    }
    pub fn focus(&mut self, key: ChunkKey) {
        self.focus = key;
    }
    pub fn len(&self) -> usize {
        self.chunks.len()
    }
    pub fn is_empty(&self) -> bool {
        self.chunks.is_empty()
    }
    pub fn contains(&self, key: ChunkKey) -> bool {
        self.chunks.contains_key(&key)
    }
    pub fn stats(&self) -> CacheStats {
        self.stats
    }
    pub fn get_or_insert_with(
        &mut self,
        key: ChunkKey,
        generate: impl FnOnce(ChunkKey) -> T,
    ) -> &T {
        if self.chunks.contains_key(&key) {
            self.stats.hits = self.stats.hits.saturating_add(1);
        } else {
            // Generate first: a panic cannot evict an existing value.
            let value = generate(key);
            self.stats.misses = self.stats.misses.saturating_add(1);
            if self.chunks.len() >= self.capacity {
                let distance = |k: &ChunkKey| {
                    (i128::from(k.0) - i128::from(self.focus.0))
                        .abs()
                        .max((i128::from(k.1) - i128::from(self.focus.1)).abs())
                };
                if let Some(farthest) = self.chunks.keys().max_by_key(|k| distance(k)).copied() {
                    self.chunks.remove(&farthest);
                    self.stats.evictions = self.stats.evictions.saturating_add(1);
                }
            }
            self.chunks.insert(key, value);
        }
        &self.chunks[&key]
    }
}
