# Incremental TerraTopia integration

The existing TerraTopia worktree contained uncommitted Phase 3 gameplay changes.
They were preserved; no reset, history rewrite or replacement project was used.
The TerraGame and TerraSnake target directories were empty when inspected.

The integration was performed in compiling stages, running all 53 existing game
tests after the initial extraction and again after cache/session migration:

| TerraTopia file | Reused TerraGame functionality |
|---|---|
| `Cargo.toml`, `Cargo.lock` | Three local engine crates; no new third-party dependency |
| `src/game.rs` | FixedStep replaces the private four-step accumulator |
| `src/world/seed.rs` | Shared exact SplitMix64 mix/coordinate hash; planet names stay local |
| `src/world/chunk.rs` | ChunkLayout replaces generic signed coordinate indexing |
| `src/world/mod.rs` | ChunkCache replaces the same distance cache; existing World API retained |
| `src/camera.rs` | Thin player-specific wrapper around engine camera positioning |
| `src/main.rs` | Session owns Terrabox lifecycle and Surface; existing save-aware loop retained |

The game still uses its existing 250 ms step/four-step bound and authoritative
survival tick. Partial wall time remains transient and is reset across modal
state changes. The engine's private wall-step count is not a new save field.
World seeds, 32-cell layout, cache tie order, generator 2/3 and hash constants are
unchanged. Save schema remains 2, with genuine schema-1 compatibility and backup
protection intact. Existing legacy fixtures and old/new terrain reference tests
pass. No saved IDs, species or loot values were reinterpreted.

TerraTopia retains its game-owned renderer and direct Terrabox type imports; it
is not forced through the example driver or seven layers. Adopting Layers would
require reorganizing an already working renderer without a demonstrated need.
Its wildlife uses region-slot IDs stored in saves, so moving it to engine entity
IDs/spatial storage now would risk save identity and iteration order. That
extraction is deferred until measured active populations justify migration.
Survival frequency rules remain explicit game code; they can use Scheduler later
without changing persisted clock semantics. Biomes, noise/generation, atmosphere,
oxygen, inventory, recipes, species, objectives and lore remain in TerraTopia.

Both repositories use stable Rust 2024 and the same sibling Terrabox source.
See [validation](validation.md) for the existing scanner/crafting/survival/save
and timed full-night PTY acceptance results after integration.

## Phase 4 integration (2026-10-23)

TerraTopia now uses EntityStore, SpatialIndex, Scheduler, ActiveRegions and
EventQueue in the wildlife lifecycle. Its dense records and persistent tuple IDs
remain game-owned; the arena maps transient handles to dense indices. The engine
adds only generic bounded local A*, covered by wall/dynamic-obstacle, budget and
extreme-coordinate tests. Terrabox is unchanged.

The game implements needs/states, foraging, predator/prey combat, scavenging,
births/deaths, coarse dormant counts, schema-3 saves with version-1/2 migration,
and terminal inspection/warnings. See its [architecture](../../terratopia/docs/ecosystem.md),
[performance](../../terratopia/docs/performance-phase4.md) and
[validation](../../terratopia/docs/validation-phase4.md).

Five measured release runs: 1,000 active records across 130 regions had a median
2.2940 ms mean fixed tick and 9.9489 ms p95; 10,000 represented dormant animals
across 500 summaries took 1.465 ms for 24,000 ticks with zero individual AI
updates. These are game-specific host measurements, not engine guarantees.
