# Actual validation

2026-10-23, Linux x86_64, stable Rust 1.97.1. The sibling Terrabox source was
inspected and remains unchanged. Existing TerraTopia history and uncommitted
Phase 3 work were preserved. No third-party framework/dependency was added.

| Command / scope | Result |
|---|---|
| TerraGame `cargo fmt --all --check` | Pass |
| TerraGame `cargo clippy --all-targets -- -D warnings` | Pass |
| TerraGame `cargo test` | Pass: 14 tests (core + integration) |
| TerraGame `cargo test --workspace --all-targets` | Pass: 15 tests including actual example renderer |
| TerraGame `cargo build --release` | Pass: standalone workspace libraries |
| TerraGame `cargo build --release --examples` | Pass: both games and benchmark |
| TerraGame `python3 scripts/smoke-terminal.py` | Pass: six real example PTY runs |
| TerraGame `cargo check --all-targets --target x86_64-pc-windows-gnu` | Pass; compilation only |
| TerraTopia formatting, Clippy, tests, release build | Pass: all 53 existing tests |
| TerraTopia `python3 scripts/smoke-terminal.py --full-night` | Pass after integration; real-time dawn, save and continuation |
| TerraTopia `cargo check --all-targets --target x86_64-pc-windows-gnu` | Pass; compilation only |

The engine tests cover partial fixed steps, pause/resume, stopped lifecycle,
catch-up and Duration/tick overflow, multi-rate order/phase and partition
independence; entity reuse, stale IDs, capacity, generation retirement, deferred
commands and queues; deterministic RNG golden values and original coordinate
hash compatibility; negative/extreme chunk indexing, cache ties and eviction;
spatial boundary moves, sorted queries, reference comparison and huge-radius
arithmetic; region callbacks/resume; single-cell tiles, palette, clipping,
layer order, cameras and resize; Snake scoring/collision/pause/restart; sandbox
rates/dormancy and the actual renderer's simulation/dimension independence.

## Real terminal examples

The standard-library Python PTY suite launches actual release executables in
80×24 terminals and sends ordinary key bytes. For each example it tests Q,
Escape and Ctrl+C. It checks exact termios restoration, alternate-screen exit,
cursor restoration and successful process return codes. The Snake case observes
food consumed through real timer ticks, game over from a wall, pause, restart,
keyboard direction changes and resizing to 20×6/back to 100×30. The sandbox case
walks to a deterministic resource and gathers it, crosses a region boundary,
observes active/dormant/statistic changes, pauses, resizes to 10×4/back and resumes.
Paused windows show zero unsolicited output and measured zero CPU ticks in each
600 ms sample. PTY input is synthetic; it is not a physical keyboard sign-off.

## TerraTopia regression

The unchanged existing test suite checks both terrain generator versions,
chunk/reference agreement, scanner/fog, gameplay collisions, crafting, inventory,
resource depletion, environmental exposure, wildlife, old schema-1 import and
schema-2 save/load safety. The post-integration full-night PTY suite exercises
three crash/gather/craft/build/power runs with distinct exit keys, inventory/help/
pause, scanner, save/load, resize and restoration. The connected toxic-world
expedition then scans creatures, gathers flora/minerals, crafts, salvages a ruin,
escapes a hostile creature, observes a real forecast/storm, returns to shelter,
waits through actual night/dawn, saves the completed objective and loads/continues.
It also tests save failure and startup load failure. No simulation clock was
edited and no elapsed-time fixture substituted for that terminal run.

## Performance and limits

Five release benchmark samples and memory estimates are in
[performance.md](performance.md) and [benchmark.json](benchmark.json). Tests do
not establish emulator throughput or performance over a slow SSH connection.
No new screenshots are claimed for these examples; existing TerraTopia xterm
captures remain historical Phase 3 artifacts.

No native Windows, macOS, Android Termux or SSH runtime test was performed.
Windows checks establish type-checking only, not linked/native execution. Forced
termination/abort restoration is outside Drop guarantees. Read
[review.md](review.md) for the final code review, remaining limitations and next
milestone, and [terratopia.md](terratopia.md) for integration scope.
