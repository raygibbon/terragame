//! Multi-rate systems are dispatched by simulation tick, in registration order.
use std::num::NonZeroU64;
pub type Tick = u64;
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct SystemId(pub usize);
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Scheduled {
    pub system: SystemId,
    pub tick: Tick,
}
struct Rate {
    period: NonZeroU64,
    phase: u64,
}
pub struct Scheduler {
    rates: Vec<Rate>,
    limit: usize,
}
impl Scheduler {
    pub fn new(limit: usize) -> Self {
        Self {
            rates: Vec::new(),
            limit,
        }
    }
    /// Phase in [0, period). Phase zero runs on positive multiples of period.
    pub fn register(&mut self, period: NonZeroU64, phase: u64) -> Option<SystemId> {
        if self.rates.len() >= self.limit || phase >= period.get() {
            return None;
        }
        let id = SystemId(self.rates.len());
        self.rates.push(Rate { period, phase });
        Some(id)
    }
    pub fn len(&self) -> usize {
        self.rates.len()
    }
    pub fn is_empty(&self) -> bool {
        self.rates.is_empty()
    }
    pub fn dispatch(&self, tick: Tick, mut run: impl FnMut(Scheduled)) {
        if tick == 0 {
            return;
        }
        for (index, rate) in self.rates.iter().enumerate() {
            if tick % rate.period.get() == rate.phase {
                run(Scheduled {
                    system: SystemId(index),
                    tick,
                });
            }
        }
    }
}
