//! Stable SplitMix64 streams; wrapping is intentional only inside randomness.
pub fn mix(mut value: u64) -> u64 {
    value = (value ^ (value >> 30)).wrapping_mul(0xbf58476d1ce4e5b9);
    value = (value ^ (value >> 27)).wrapping_mul(0x94d049bb133111eb);
    value ^ (value >> 31)
}
/// Stable across exploration order, architecture and Rust's HashMap versions.
pub fn coordinate_hash(seed: u64, x: i64, y: i64) -> u64 {
    mix(seed ^ mix(x as u64) ^ mix((y as u64).wrapping_add(0x9e3779b97f4a7c15)))
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Random {
    state: u64,
}
impl Random {
    pub fn new(seed: u64) -> Self {
        Self { state: seed }
    }
    pub fn state(self) -> u64 {
        self.state
    }
    pub fn next_u64(&mut self) -> u64 {
        self.state = self.state.wrapping_add(0x9e3779b97f4a7c15);
        mix(self.state)
    }
    /// Unbiased [0, upper), or None for an empty interval.
    pub fn below(&mut self, upper: u64) -> Option<u64> {
        if upper == 0 {
            return None;
        }
        let threshold = upper.wrapping_neg() % upper;
        loop {
            let n = self.next_u64();
            if n >= threshold {
                return Some(n % upper);
            }
        }
    }
}
