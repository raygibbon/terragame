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
