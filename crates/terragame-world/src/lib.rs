//! Generic spatial data. Terrain generation and persistence stay with the game.
pub mod cache;
pub mod navigation;
pub mod regions;
pub mod spatial;
pub use cache::{CacheStats, ChunkCache};
pub use regions::{ActiveRegions, RegionEvent, RegionLifecycle};
pub use spatial::SpatialIndex;
pub use terragame_core::random::coordinate_hash;

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Point {
    pub x: i64,
    pub y: i64,
}
impl Point {
    pub const fn new(x: i64, y: i64) -> Self {
        Self { x, y }
    }
    pub fn offset(self, dx: i64, dy: i64) -> Option<Self> {
        Some(Self {
            x: self.x.checked_add(dx)?,
            y: self.y.checked_add(dy)?,
        })
    }
}
impl From<(i64, i64)> for Point {
    fn from(p: (i64, i64)) -> Self {
        Self::new(p.0, p.1)
    }
}
impl From<Point> for (i64, i64) {
    fn from(p: Point) -> Self {
        (p.x, p.y)
    }
}
pub type ChunkKey = (i64, i64);
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ChunkLayout {
    side: u16,
}
impl ChunkLayout {
    pub const fn new(side: u16) -> Option<Self> {
        if side == 0 { None } else { Some(Self { side }) }
    }
    pub fn side(self) -> u16 {
        self.side
    }
    pub fn cells(self) -> usize {
        usize::from(self.side) * usize::from(self.side)
    }
    pub fn locate(self, p: Point) -> (ChunkKey, usize) {
        let side = i64::from(self.side);
        (
            (p.x.div_euclid(side), p.y.div_euclid(side)),
            (p.y.rem_euclid(side) * side + p.x.rem_euclid(side)) as usize,
        )
    }
    /// Edge chunks can be partial for non-power-of-two sides. Out-of-world cells
    /// return None rather than wrapping. Intermediate multiplication uses i128.
    pub fn world_at(self, key: ChunkKey, index: usize) -> Option<Point> {
        if index >= self.cells() {
            return None;
        }
        let side = i128::from(self.side);
        Some(Point::new(
            i64::try_from(i128::from(key.0) * side + (index % usize::from(self.side)) as i128)
                .ok()?,
            i64::try_from(i128::from(key.1) * side + (index / usize::from(self.side)) as i128)
                .ok()?,
        ))
    }
}
/// Inclusive world bounds, including both i64 extremes.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Bounds {
    pub min: Point,
    pub max: Point,
}
impl Bounds {
    pub fn new(min: Point, max: Point) -> Option<Self> {
        (min.x <= max.x && min.y <= max.y).then_some(Self { min, max })
    }
    pub fn around(center: Point, radius: u64) -> Self {
        let clamp = |v: i128| v.clamp(i128::from(i64::MIN), i128::from(i64::MAX)) as i64;
        Self {
            min: Point::new(
                clamp(i128::from(center.x) - i128::from(radius)),
                clamp(i128::from(center.y) - i128::from(radius)),
            ),
            max: Point::new(
                clamp(i128::from(center.x) + i128::from(radius)),
                clamp(i128::from(center.y) + i128::from(radius)),
            ),
        }
    }
    pub fn contains(self, p: Point) -> bool {
        p.x >= self.min.x && p.x <= self.max.x && p.y >= self.min.y && p.y <= self.max.y
    }
}
