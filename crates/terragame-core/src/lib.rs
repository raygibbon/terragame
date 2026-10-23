//! Headless deterministic simulation primitives. No terminal or gameplay rules.
pub mod clock;
pub mod entity;
pub mod queue;
pub mod random;
pub mod schedule;
pub use clock::{FixedStep, Lifecycle, Simulation};
pub use entity::{Commands, EntityId, EntityStore};
pub use queue::EventQueue;
pub use schedule::{Scheduled, Scheduler, SystemId, Tick};
