# Architecture and design decisions

## Ownership boundaries

The engine has three crates. Core is headless, world depends on core only for
entity identity/ticks/hashing, and terminal integrates both with Terrabox.
Terrain, oxygen, health, crafting, species and objectives are game code. No game
is required to use every module. Independent crates make headless simulation
possible without linking a terminal backend.

The initial repository inspection covered actual TerraTopia clock, main loop,
camera, chunk/noise/hash/cache, renderer, input, persistence and validation APIs,
and Terrabox's Terminal, Surface, Canvas, FrameClock and key APIs. Those already
provided all required terminal capabilities. Terrabox source is unchanged.

## Simulation time

A `FixedStep` is integer Duration plus a u64 tick counter and a sub-step residue.
Each advance accepts at most `max_steps * step` of elapsed wall time and emits at
most `max_steps`. Extra wall time is intentionally dropped after a stall. The
residue survives normal frames; pausing/resuming clears it so paused time is not
replayed. At u64::MAX no more steps are emitted, rather than wrapping. Large
Duration arithmetic uses u128 nanoseconds and reconstructs full seconds.
Invalid zero steps, zero catch-up bounds and overflowing duration bounds are
rejected. Counter restoration is explicit through `set_tick`.

The multi-rate `Scheduler` stores bounded registrations: nonzero period, phase
strictly smaller than period and stable registration ID. Tick zero is initial
state. For positive tick `t`, system `s` runs if `t % period == phase`. Multiple
systems run in registration order. The headless `Simulation` dispatches ticks
in ascending order; Running/Paused/Stopped is explicit and Stopped is terminal.
Games needing dependency graphs or dynamic cancellation should add those only
when justified. There is no catch-up across millions of missed simulation ticks.

The terminal application driver uses a separate Terrabox FrameClock for render
pacing. It waits for the earliest input, simulation or render deadline using
Terrabox's blocking event API. Ticks report whether anything changed; render deadlines skip composition when
no changes are pending. Render lateness skips frames; it does not schedule
simulation systems. Pause removes deadlines and blocks until input/resize. A
render function receives `&self`; derived terrain/composition caches may use
interior mutability, but no simulation RNG/state depends on dimensions.
The example renderer is tested against an unrendered simulation at 0×0, 10×4,
80×24 and 100×30. Wall stalls may intentionally discard time; determinism is a
function of simulation ticks and ordered input, not an assertion of real-time
lockstep between differently stalled machines.

## Entities and events

`EntityStore<T>` is a bounded slot arena. IDs contain a u32 index and u64
generation. Stale lookup/removal fails. Removing a live entity increments the
generation; an exhausted generation retires its slot permanently instead of
aliasing an old ID. The free list is LIFO; deterministic iteration follows slot
order, not insertion order. Capacity bounds allocated slots; payload allocations
inside `T` are the game's responsibility. The storage estimate includes Vec
capacity, not just live objects. There is no runtime reflection or implicit
component borrowing.

IDs are store-local, not globally unique or serializable engine world IDs.
`SpatialIndex` intentionally does not borrow an EntityStore: games synchronize
insert/remove explicitly at system boundaries. Bounded `Commands<T>` is a FIFO
of spawn/removal commands, applied after iteration; results include created IDs,
removed data, rejected data and stale removals. Event queues return ownership on
full instead of silently dropping events. Unbounded game logs are not hidden
inside the engine.

Randomness uses documented SplitMix64 constants and independently seeded streams.
Coordinate hashing is bit-identical to TerraTopia's previous function. Standard
library randomized map hashes never define generated content.

## Chunks and spatial queries

`Point` uses signed i64; checked offsets reject world exits. `ChunkLayout` uses
Euclidean division/remainder, including negative coordinates. Positive u16 sides
are supported. `world_at` uses i128 products and rejects non-representable cells
in partial chunks at i64 limits. Bounds are inclusive, so MAX is representable.

The cache preserves TerraTopia's distance-based eviction policy: a bounded
BTreeMap of generated chunks, focus-relative Chebyshev distance calculated in
i128, and greatest-key tie breaking. It generates before eviction, so a generator
panic cannot remove existing cached state. Cache generation must be pure.
Mutable overlays and persistent objects must not be hidden in an evictable chunk.
Eviction is O(cache capacity); no LRU list or thread was justified for this phase.

The spatial index uses ordered x-chunk → y-chunk → entity-ID buckets and a reverse
ID → position map. Range queries visit occupied buckets in a rectangular chunk
range, then filter exact coordinates and sort full IDs. Radius queries first
prune with a bounding rectangle, then use overflow-safe u128 squared distances.
Moving across a boundary removes empty buckets. Very large queries can return
all loaded entities; the explicit membership cap bounds work/memory. This is not
an index over millions of unloaded objects or a promise of constant-time queries.

## Active and dormant regions

`ActiveRegions` stores only a bounded square of active chunk keys and activation
ticks. Radius/cap are checked on construction. On reconcile it deactivates old
keys in sorted order before activating new keys in sorted order. The lifecycle
trait hands state management back to the game. Callers supply monotonic ticks.
No per-object update takes place outside active regions.

The sandbox demonstrates this with nine 16×16 active chunks, four creatures and
two resources per chunk. On exit it suspends objects in a summary containing
positions and the exit tick; on return it resumes that state without pretending
to simulate the elapsed dormant time. At most 64 dormant summaries are retained;
farthest/key-tied eviction regenerates forgotten demo regions. A real game would
persist its own ledger before eviction. The sandbox therefore demonstrates
bounded suspension, not durable world saves or ecosystem catch-up.

## Terminal composition

Tiles validate their cell width through Terrabox. ASCII printable tiles are a
predictable baseline. Seven fixed transparent layers compose back to front into
Terrabox Surface; a space tile is opaque and None is transparent. The game chooses
a viewport, camera and palette, with a caller-defined cell cap for layer storage.
Camera arithmetic remains safe at extreme coordinates. Resize failure at the
cap preserves existing layers; example code displays a message. Terrabox still
owns clipping, graphemes, front/back buffers, platform I/O and changed-cell writes.
The Session adapter merely owns Terminal plus Surface and forwards wait/present.
Terminal restoration on normal exit and unwinding follows Terrabox RAII; aborts,
SIGKILL and machine failures are not recoverable through Drop.

## Deliberately rejected alternatives

A heavyweight ECS, async runtime, worker threads, custom input parser, duplicated
terminal backend and graphical sprite system would add ownership/portability
costs without a measured benefit here. Millions of persistent world objects need
a persistence design and coarse summaries, not millions of active Rust entities.
No speculative civilisation, economy, job graph, networking or script framework
has been introduced. Public APIs are a small 0.1 foundation and can evolve after
real games demonstrate their next requirements.
