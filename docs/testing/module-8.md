# Module 8 — Testing Manual: Timeline Engine

## Prerequisites
- You're in the `smartcommit/` repo root
- Modules 0-7 tests still pass

Module 8 has no CLI surface yet (same as Module 6 — modules.md's own
Module 8 section has no "## CLI" heading). Timestamps will actually
show up in `smartcommit plan`'s output once Module 11 adds the
`--start`/interval/range flags. Until then, this module is verified
through its test suite only.

## 1. Build & test
```bash
cargo build --workspace
cargo test --workspace
```
✅ Pass: `65 passed; 0 failed` (5 CLI + 60 core)

## 2. What's actually being tested (no CLI to click through)
Since there's nothing to run by hand yet, here's what each test
category proves, so you can sanity-check the reasoning rather than
just trusting a green checkmark:

- **Fixed intervals are exact** — 3 commits, 10-minute gaps, custom
  start: timestamps land exactly on start, start+10m, start+20m.
- **Random gaps stay within bounds** — 20 commits with a 5–15 minute
  range: every consecutive gap is checked to be within that range.
- **Chronological ordering** — with no custom start (the "anchor the
  last commit at now, work backward" branch), timestamps are
  strictly increasing by `commit_index`, and the *last* one lands
  exactly on `now`.
- **No future timestamps by default** — a custom start close enough
  to `now` that a later commit would land after it is rejected
  (`TimelineError::FutureTimestamp`), unless `allow_future: true` is
  passed explicitly.
- **The "no start" branch can never produce a future timestamp** —
  tested directly with 50 commits and a huge random range (1–48
  hours): every single timestamp is asserted `<= now`.
- **`TimedCommitPlan` pairing** — a `CommitPlan` from Module 7 paired
  with generated timestamps by `commit_index`; a count mismatch
  between the plan's groups and the timestamp list is rejected
  rather than silently truncated or padded.

## 3. If you want to poke at it directly
There's no CLI, but you can exercise it from a `cargo test`-style
snippet or the Rust REPL-equivalent (a scratch `#[test]` or
`cargo run --example`, deleted after). A minimal example:

```rust
use smartcommit_core::{generate_timeline, TimelineMode, SeededRandomSource, Duration, DateTime};

let now = DateTime::parse_from_rfc3339("2026-12-01T00:00:00+03:00").unwrap();
let mut rng = SeededRandomSource::new(42);

let timestamps = generate_timeline(
    3,
    TimelineMode::Fixed { interval: Duration::minutes(10) },
    None,   // no custom start — anchors the last commit at `now`
    now,
    false,  // allow_future
    &mut rng,
).unwrap();

for t in &timestamps {
    println!("{}: {}", t.commit_index, t.timestamp);
}
```
✅ Expect the last line's timestamp to equal `now` exactly, and each
earlier line to be exactly 10 minutes before the next.

## If anything fails
Paste the exact test name and output back — especially anything
under `timeline::tests::`, since that's all there is to check until
Module 11 wires this into the CLI.
