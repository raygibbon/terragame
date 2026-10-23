use crate::schedule::{Scheduled, Scheduler, Tick};
use std::time::Duration;

/// Integer fixed steps. Late wall time is deliberately dropped, never queued.
/// Render timing belongs to the caller and cannot change simulation ordering.
#[derive(Clone, Debug)]
pub struct FixedStep {
    step: Duration,
    max_steps: u32,
    remainder: Duration,
    tick: Tick,
}
impl FixedStep {
    pub fn new(step: Duration, max_steps: u32) -> Option<Self> {
        if step.is_zero() || max_steps == 0 || step.checked_mul(max_steps).is_none() {
            return None;
        }
        Some(Self {
            step,
            max_steps,
            remainder: Duration::ZERO,
            tick: 0,
        })
    }
    pub fn step(&self) -> Duration {
        self.step
    }
    pub fn tick(&self) -> Tick {
        self.tick
    }
    /// Restore a simulation timeline. Clears partial wall time; never rolls over.
    pub fn set_tick(&mut self, tick: Tick) {
        self.tick = tick;
        self.reset();
    }
    pub fn reset(&mut self) {
        self.remainder = Duration::ZERO;
    }
    pub fn remaining(&self) -> Duration {
        self.step - self.remainder
    }
    pub fn advance(&mut self, elapsed: Duration) -> u32 {
        // u128 nanos avoids Duration addition overflow even at Duration::MAX.
        let cap = self.step.as_nanos() * u128::from(self.max_steps);
        let total = self.remainder.as_nanos() + elapsed.as_nanos().min(cap);
        let count = (total / self.step.as_nanos()).min(u128::from(self.max_steps));
        let count = count.min(u128::from(u64::MAX - self.tick)) as u32;
        if self.tick == u64::MAX {
            self.reset();
            return 0;
        }
        // Sub-step residue survives ordinary frames and bounded stalls.
        // Duration may exceed u64 nanoseconds; reconstruct the general case.
        let residue = total % self.step.as_nanos();
        self.remainder = Duration::new(
            (residue / 1_000_000_000) as u64,
            (residue % 1_000_000_000) as u32,
        );
        self.tick += u64::from(count);
        count
    }
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Lifecycle {
    Running,
    Paused,
    Stopped,
}

/// Small headless lifecycle controller; callbacks are explicit game systems.
/// Tick zero is initial state; the first update runs at tick one.
pub struct Simulation {
    pub clock: FixedStep,
    pub scheduler: Scheduler,
    state: Lifecycle,
}
impl Simulation {
    pub fn new(clock: FixedStep, scheduler: Scheduler) -> Self {
        Self {
            clock,
            scheduler,
            state: Lifecycle::Running,
        }
    }
    pub fn state(&self) -> Lifecycle {
        self.state
    }
    /// Stopped is terminal. A new expedition creates a new Simulation.
    pub fn set_state(&mut self, state: Lifecycle) {
        if self.state != Lifecycle::Stopped {
            self.state = state;
            self.clock.reset();
        }
    }
    pub fn advance(&mut self, elapsed: Duration, mut run: impl FnMut(Scheduled)) -> u32 {
        if self.state != Lifecycle::Running {
            return 0;
        }
        let before = self.clock.tick();
        let count = self.clock.advance(elapsed);
        for offset in 1..=count {
            self.scheduler
                .dispatch(before + u64::from(offset), &mut run);
        }
        count
    }
}
