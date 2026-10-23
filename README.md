# TerraGame

A small, deterministic Rust 2024 simulation engine for real terminal games.
TerraGame supplies reusable scheduling, entity and spatial primitives plus a
Terrabox adapter. It runs synchronously on one thread with ordinary Rust structs;
there is no ECS framework, async runtime, graphical renderer or second terminal
backend. MIT licensed. This is an implemented foundation, not a civilisation
simulator.

```text
TerraTopia / your game
    ├── terragame-core      fixed time, rates, IDs, storage, queues, random streams
    ├── terragame-world     coordinates, chunks, spatial queries, active regions
    └── terragame-terminal  camera, tile layers, session and application driver
             └── Terrabox → real terminal
```

`core` has no dependencies; `world` depends only on `core`; `terminal` depends on
both and Terrabox. No third-party dependency was added beyond Terrabox's existing
transitive dependencies. TerraTopia still uses Terrabox types directly in its
existing UI; its terminal session now runs through TerraGame.

## Build and play

Keep the repositories next to one another (`terragame/`, `terrabox/`, optionally
`terratopia/`). Terrabox is a local path dependency. Use stable Rust; the tested
toolchain is recorded in [validation](docs/validation.md). No display server,
optional graphics protocol or special font is required. ASCII is the baseline.

From `terragame/`:

```sh
cargo build --release
cargo run --release -p terragame-terminal --example terrasnake
cargo run --release -p terragame-terminal --example sandbox
```

TerraSnake uses WASD/arrows, P/Space to pause, N to restart and Q/Escape/Ctrl+C to
quit. Eat `*`, avoid walls and your body. Its fixed 40×18 board needs 40×21 cells
to show the entire game; smaller windows display a resize message. Board size and
simulation do not change with terminal dimensions. No previous TerraSnake source
was present in the supplied empty `terrasnake/` directory, so this example is new.

The sandbox uses WASD/arrows to scroll, E to gather nearby `*` resources, P/Space
to pause and Q/Escape/Ctrl+C to quit. `@` is the player and `g` are creatures.
It shows live ticks, active/dormant counts, nearby queries, resources gathered,
region transitions and multi-rate counters. Detailed updates run every 100 ms,
local environment statistics every simulated second and a deliberately empty
coarse demonstration counter every ten seconds. These are example rates, not
settlement or ecosystem implementations. It supports 80×24 and clipped smaller
views. Its entity state is independent of viewport size.

## Start a game

Depend on whichever crates you need (paths relative to your project's manifest):

```toml
[dependencies]
terragame-core = { path = "../terragame/crates/terragame-core" }
terragame-world = { path = "../terragame/crates/terragame-world" }
terragame-terminal = { path = "../terragame/crates/terragame-terminal" }
```

Implement `terragame_terminal::Application`: `event` handles already decoded
Terrabox events, `tick` changes simulation and returns whether a redraw is needed,
`render(&self, ...)` draws a Terrabox
Surface, and `paused` removes all timed wakeups. Call `run(&mut game, step,
render_interval)`. Rendering and simulation have independent clocks; resize is
an event, not a simulation step. Returning `Control::QUIT` drops the Session and
Terrabox restores the terminal. See the complete [Snake example](examples/terrasnake.rs)
and [sandbox](examples/sandbox.rs) rather than introducing your own raw-mode code.

For an existing event loop, use `Session`, `FixedStep` and `Camera` separately,
as TerraTopia does. The engine does not force its application driver on a game.

```rust
use std::time::Duration;
use terragame_core::{FixedStep, Scheduler, Simulation};

let mut rates = Scheduler::new(8); // bound system registrations
rates.register(1.try_into().unwrap(), 0).unwrap();    // nearby entities
rates.register(4.try_into().unwrap(), 0).unwrap();    // one second
rates.register(240.try_into().unwrap(), 0).unwrap();  // one minute
let mut sim = Simulation::new(
    FixedStep::new(Duration::from_millis(250), 4).unwrap(), rates,
);
sim.advance(Duration::from_secs(1), |due| {
    // Explicitly match due.system and run the corresponding game system.
    println!("tick {} system {}", due.tick, due.system.0);
});
```

`Scheduler::dispatch` can also be used with a game's existing authoritative tick.
Registration order breaks simultaneous ties. Integer periods/phases avoid float
drift; no callbacks, reflection or hidden worker threads are stored in systems.

## Data and rendering

`EntityStore<T>` owns your ordinary structs. Alive IDs remain stable; removing an
entity invalidates its generation and reuses the vacant slot. Iteration is in
slot order. `Commands<T>` defers creation/removal until after a system finishes
iterating; rejected spawns and stale removals are explicitly reported. Bounded
`EventQueue<T>` provides FIFO messages with backpressure. IDs belong to one store,
not a process-wide universe; games must keep spatial membership in sync with
entity removal. Entity IDs are not a persistence format.

`ChunkLayout` supports positive configurable sides, Euclidean negative indexing
and partial edge chunks. `ChunkCache<T>` bounds immutable generated payloads and
evicts farthest from focus with deterministic key ties. Depletion and persistent
objects must live separately. `SpatialIndex` has a membership cap and ordered
chunk buckets, rectangular and Euclidean-radius queries, movement and sorted ID
results. It never scans unloaded world objects. `ActiveRegions` emits ordered
activation/deactivation callbacks; games own dormant state and storage.

`Tile` accepts a single cell as measured by Terrabox, or an ASCII fallback.
`Palette` maps game-chosen IDs to tiles. `Layers` composes transparent cell grids
in this order: background, terrain, structures, items, entities, effects,
overlay. `Camera` projects signed world coordinates through a clipped viewport.
Text and HUD widgets can use Terrabox Canvas directly. No new Unicode parser,
front/back presentation buffer, mouse decoder or ANSI emitter exists here.

Read [architecture and decisions](docs/architecture.md) for limits, overflow,
clock semantics and dormant-region ownership; [TerraTopia integration](docs/terratopia.md)
for compatibility and deferred extractions.

## Checks and measurements

```sh
cargo fmt --all --check
cargo clippy --all-targets -- -D warnings
cargo test
cargo test --workspace --all-targets  # includes the actual example renderer test
cargo build --release
cargo build --release --examples
python3 scripts/smoke-terminal.py
cargo run --release -p terragame-terminal --example benchmark
```

The PTY script uses Python's standard library and Unix PTYs. It runs both examples
with Q, Escape and Ctrl+C, verifies scoring/interactions, pause, restart, resizing
and terminal restoration. [Validation](docs/validation.md) records actual results,
including TerraTopia's timed full-night scenario. [Performance](docs/performance.md)
contains five measured release samples and memory estimates. No cross-platform
runtime claim follows from a Linux PTY test.

The engine intentionally stops short of world persistence, pathfinding, game
components, jobs, colonies, economic history, travel, networking or scripting.
The next step is gameplay-driven use and profiling at larger active populations,
plus native terminal testing on the remaining portability targets.
