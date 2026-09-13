# Module 10 — Testing Manual: Commit Executor

## Prerequisites
- You're in the `smartcommit/` repo root
- Modules 0-9 tests still pass

Like Modules 6, 8, and 9, there's no CLI command yet — Module 11 is
where `execute_commit_plan` gets wired into `smartcommit commit`.
This module is the most safety-critical one in the whole MVP, so
lean on the test suite here rather than eyeballing output.

## 1. Build & test
```bash
cargo build --workspace
cargo test --workspace
```
✅ Pass: `83 passed; 0 failed` (5 CLI + 78 core)

## 2. What each test category actually proves
Since there's no CLI to click through, here's what to trust and why:

- **Two/three commits from groups** — each group becomes exactly one
  real commit, in order; a mid-sequence commit's tree is checked
  directly (`git show <oid>:path`) to confirm it only contains what
  it should at that point in the sequence.
- **Timestamps applied** — a commit's author *and* committer dates
  (checked separately via `git show -s --format=%ad`/`%cd`) both
  match the planned timestamp exactly.
- **Unstaged changes preserved** — including the trickiest case: an
  unstaged edit stacked *on top of* a file that's simultaneously
  being split into a commit right now. Checked by reading the file
  off disk after execution and comparing byte-for-byte.
- **Staged content is correct** — the commit reflects what was
  *staged*, not a later working-tree edit made after staging but
  before execution runs.
- **Failure midway** — a group referencing a file that was never
  actually staged forces a failure partway through a 3-group plan;
  checks that exactly the right commits ended up completed, the
  right one is reported as the failure, the right ones are listed as
  not-attempted, and `git log` shows HEAD stopped at exactly the
  right point.
- **Real hook rejection** — this test installs an actual executable
  `.git/hooks/pre-commit` script that rejects any commit touching a
  specific file, then confirms: the failure is reported with the
  hook's own stderr message, HEAD doesn't move past the last real
  success, and the real index still shows the rejected file staged
  (nothing was silently consumed).
- **Deleted / added files** — each produces the correct resulting
  tree (verified via `git ls-tree` / `git show`).
- **Empty groups** — skipped without being misreported as a failure.
- **HEAD moved since checkpoint** — if something else commits to the
  repo between checkpoint creation and execution, the executor
  refuses to run at all rather than building on a stale assumption.

## 3. Manual end-to-end check (optional, mirrors what was done during development)
If you want to see the whole pipeline (Modules 1-10) work together
on realistic data, this is the shape of it — written as a throwaway
`cargo run --example`, deleted afterward:

```rust
let messages: CommitMessages = "Fix authentication in login module\n\nImprove dashboard widgets\n".parse().unwrap();
let files = get_staged_changes(&dir).unwrap();
let diffs = get_staged_diffs(&dir).unwrap();
let scores = match_files_to_messages(&messages.0, &files, &diffs);
let plan = build_commit_plan(&messages.0, &files, &scores);

let checkpoint = create_safety_checkpoint(&dir, now_utc).unwrap();

let mut rng = SeededRandomSource::new(1);
let timestamps = generate_timeline(
    plan.groups.len(),
    TimelineMode::Fixed { interval: Duration::minutes(10) },
    None, now, false, &mut rng,
).unwrap();

let report = execute_commit_plan(&dir, &plan.groups, Some(&timestamps), &checkpoint).unwrap();
```
✅ Expect `report.is_full_success()` to be `true`, `report.completed`
to have one entry per message with a real commit hash, and `git log
--oneline` in that repo to show the new commits on top of the
original one.

## If anything fails
Paste the exact test name and output back. For the hook-rejection
test specifically, also note your OS — the test sets Unix file
permissions (`0o755`) on the hook script via `#[cfg(unix)]`, so it's
untested on Windows as written; if you're on Windows and it behaves
differently, that's useful to know.
