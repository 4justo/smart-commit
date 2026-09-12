# SmartCommit — Project State

This file tracks exact progress against the module build order in
`modules.md` (the module-by-module AI development prompts). Update
it at the end of every module.

## Completed modules

### Module 0 — Rust Workspace Foundation ✅

- Cargo workspace with two crates:
  - `smartcommit-core` (library)
  - `smartcommit-cli` (binary) — depends on `smartcommit-core` and
    `clap` (derive API)
- Directory layout: `crates/`, `tests/`, `fixtures/`, `docs/`,
  `Cargo.toml`, `README.md`, `ARCHITECTURE.md`, `PROJECT_STATE.md`
- CLI: `smartcommit --version`, `smartcommit --help` both work

### Module 1 — Git Repository Detection ✅

- `smartcommit-core::Repository::discover(path)` — detects the Git
  repository containing `path` by shelling out to `git -C <path>
  rev-parse ...`
- `RepositoryInfo { root, git_dir, is_bare }`. For a bare
  repository, `root` is the bare repository's own directory, since
  there's no working tree to report
- `RepositoryError` (via `thiserror`): `PathNotFound`,
  `PathNotAccessible`, `NotARepository`, `GitUnavailable` — no
  panics for any expected user-input case
- Implementation detail: `git rev-parse --show-toplevel` hard-fails
  the *entire* invocation on a bare repo, even combined with flags
  that would otherwise succeed — so discovery queries
  `--absolute-git-dir` + `--is-bare-repository` together first, and
  only asks for `--show-toplevel` separately once confirmed non-bare
- Nested-directory support comes free from `git -C <path>`'s own
  upward search
- CLI: `smartcommit doctor` (detection only at this point);
  `--repo <path>` global option, inherited by every subcommand
- Tests: 5 unit tests in `smartcommit-core` (valid root, nested dir,
  non-repo dir, invalid path, bare repo) — all against real temp
  repos

### Module 2 — Repository State Inspector ✅

- `smartcommit-core::inspect_repository_state(path)` — builds a full
  `RepositoryState` (branch, HEAD, detached-HEAD, merge/rebase/
  cherry-pick in progress, conflicts, staged/unstaged/untracked
  status) via `git status --porcelain=v2 --branch`, plus checking
  for `MERGE_HEAD` / `CHERRY_PICK_HEAD` / `rebase-apply` /
  `rebase-merge` markers in `.git`
- `RepositoryState::safety()` classifies as `Safe` / `Warning` /
  `Blocked` — in-progress operations or conflicts are always
  `Blocked`; no staged changes, or staged alongside unstaged/
  untracked, is `Warning`; clean staged-only is `Safe`
- `InspectorError`: wraps `RepositoryError` (via `#[from]`) plus
  `GitUnavailable` and `StatusFailed` for the status call itself
- CLI: `smartcommit doctor` expanded to report full state (branch,
  HEAD, staged/unstaged/untracked, safety label), not just detection
- Tests: 6 unit tests (clean repo, staged, unstaged, untracked,
  detached HEAD, merge conflict — the conflict test asserts
  `Blocked`)
- **Integration note:** this module was originally built by a
  collaborator as a fully standalone `module2/` crate with its own
  `[workspace]` table and its own `main.rs`/binary. That meant it was
  invisible to `cargo build --workspace` / `cargo test --workspace`
  at the repo root — the exact completion criteria every module in
  `modules.md` is checked against — and didn't follow Module 2's own
  CLI spec (expand `smartcommit doctor`, not add a second binary).
  The logic itself was correct and is preserved verbatim; it was
  moved into `crates/smartcommit-core/src/repository_state.rs` and
  wired into the existing `doctor` subcommand instead. The standalone
  `module2/` directory has been removed. One test fix: the merge
  step in the conflict test now sets `GIT_AUTHOR_NAME`/
  `GIT_AUTHOR_EMAIL`/etc. explicitly (the other git calls in that
  test already did), since a machine with no global git identity
  configured would otherwise fail that step for reasons unrelated to
  the code under test

### Module 3 — Staged Change Discovery ✅

- `smartcommit-core::get_staged_changes(path)` — lists files staged
  for commit (index vs. HEAD) via `git diff --cached --name-status
  -M -z`, never the unstaged working-tree diff
- `ChangeKind`: `Added` / `Modified` / `Deleted` / `Renamed` (matches
  `modules.md`'s spec exactly — no extra variants speculated)
- `ChangedFile { path, change_kind, old_path }` — `old_path` is
  `Some` only for `Renamed`; added because a rename entry without it
  would be indistinguishable from an add, so the field earns its
  place rather than being speculative
- `-z` (NUL-delimited output) avoids ambiguity/quoting issues for
  filenames with spaces or special characters; `-M` (rename
  detection) is passed explicitly so `Renamed` results don't depend
  on the caller's `diff.renames` git config
- Any staged status code outside A/M/D/R (e.g. a type change) folds
  into `Modified` rather than being dropped or causing a panic — the
  file stays visible, which matters more at this stage than an
  exactly-right label for a rare edge case
- CLI: `smartcommit analyze` — lists staged files only (`Modified
  src/auth.rs`, `Renamed old.rs -> new.rs`, etc.); `"No staged
  changes found."` + exit 1 when the index is empty
- `StagedChangesError`: wraps `RepositoryError` (via `#[from]`) plus
  `GitUnavailable` and `CommandFailed` for the diff call itself
- Tests: 7 unit tests — staged add, staged modify, staged delete,
  staged rename (with old_path asserted), multiple staged files of
  different kinds, an unstaged-only modification excluded, an
  untracked-only file excluded
- Manually verified: multi-file staged set (add/modify/delete),
  a clean rename with old_path shown, empty staging area, unstaged+
  untracked-only (correctly excluded), non-repo directory

### Module 4 — Diff Extraction Engine ✅

- `smartcommit-core::get_staged_diffs(path)` — extracts the staged
  diff (HEAD vs. index) for every changed file via a single
  `git diff --cached -M --no-color --no-ext-diff` call, then parses
  the unified-diff text into structured `FileDiff`/`DiffHunk`/
  `DiffLine` values itself
- **Important implementation detail:** an earlier version called
  `git diff --cached -M -- <path>` once per file (mirroring Module
  3's per-file model). That breaks rename detection — restricting
  the diff to a single pathspec stops git from pairing the old and
  new sides of a rename, so a genuine rename gets reported as a
  delete+add instead. Renames can only be detected correctly when
  git sees the *entire* staged change set at once. So this module
  runs the diff exactly once (no pathspec) and splits the result
  into per-file blocks itself, matching Module 3's `-M` usage so the
  two stay consistent. A regression test
  (`correctly_diffs_a_rename_alongside_other_changes`) locks this in
- `FileDiff { path, hunks, additions, deletions, is_binary }`;
  `DiffHunk { header, lines }`; `DiffLine::{Context, Added, Removed}`
  with the git diff-line prefix already stripped
- Binary files are detected via git's own `Binary files a/X and b/Y
  differ` marker line — `is_binary` is set true, `hunks` stays
  empty, binary content is never parsed as text
- File path is extracted from the `diff --git a/<path> b/<path>`
  header (split on the first `" b/"` after the `"a/"` prefix) —
  documented as a known limitation for the rare case of a path
  containing the literal substring `" b/"`, consistent with how most
  diff parsers handle this
- CLI: `smartcommit analyze` gains a per-file diff summary (`+N -M`,
  hunk count, or `Binary file changed`); new `--verbose` flag prints
  full hunk content (each line prefixed `+`/`-`/` `) for non-binary
  files
- If diff extraction itself fails, `analyze` still shows the file
  list from Module 3 (with a warning on stderr) rather than losing
  the whole command's output over a secondary failure
- Tests: 7 unit tests — text modification, new file, deleted file,
  multiple hunks in one file, binary file, hand-countable line
  counts, and the rename-alongside-other-changes regression test
- Manually verified: mixed add/modify/delete set (both plain and
  `--verbose`), a binary file, and an unmodified rename (0 hunks,
  correctly still identified as a rename rather than a 20-line
  delete+add)

## Current module

Nothing in progress. Module 9 is complete and verified.

## Corrections made while starting Module 6

Per the standing instruction to check the repo for issues before
each module, the following were found and fixed — none were part of
Module 6 itself, but all were real, demonstrated problems rather
than style preferences:

- **`ARCHITECTURE.md` / `Architecture.md` filename collision.**
  The repo had two architecture documents differing only by case:
  `Architecture.md` (a pre-existing, detailed 68-section spec — the
  actual authoritative source, predating even Module 0) and
  `ARCHITECTURE.md` (a much shorter file written during the Module 0
  workspace restructuring, which unintentionally shadowed rather
  than replaced the original on a case-sensitive filesystem). On a
  case-insensitive filesystem (macOS, Windows — both explicitly
  in-scope per `AI_DEVELOPMENT_CONTEXT.md` §76) these two files
  collide, which risks silent data loss or a broken checkout.
  Consolidated into a single `ARCHITECTURE.md`: the original
  68-section spec is preserved in full, with a new "Implementation
  Status Addendum" appended mapping each architectural module (§9
  Repository, §10 Repository State, §11 Change Discovery, §12 Diff,
  §13 Message) to its actual source file, plus notes on dependency
  policy and Git-integration approach as actually implemented.
  `Architecture.md` (the differently-cased duplicate) was removed.
- **Duplicate `modules.md`.** An identical (bar one trailing
  newline) copy sat at the repo root alongside the canonical
  `docs/modules.md`. Removed the root-level duplicate.
- **Testing manuals scattered at repo root.** `module0.md`,
  `module3-manual.md`, `module4-manual.md`, `module5-manual.md` were
  loose files in the repo root, inconsistent with the project's own
  `docs/` convention. Moved into `docs/testing/` and renamed for
  consistency: `module-0.md`, `module-3.md`, `module-4.md`,
  `module-5.md`.
- **Module 5's types lived directly in `lib.rs`** instead of their
  own file, unlike every other module (`repository.rs`,
  `repository_state.rs`, `staged_changes.rs`, `diff.rs`). Extracted
  `CommitMessage`, `CommitMessages`, and `ParseCommitMessagesError`
  into `crates/smartcommit-core/src/message_parser.rs` for
  consistency. While there, simplified the parser to branch once on
  whether the input has blank lines rather than always computing the
  blank-line-grouped result and sometimes discarding it — same
  behavior (all original tests still pass unchanged), less wasted
  work. Also added two tests: a genuinely multi-line message within
  one blank-line-separated block (the parser already supported this;
  it just wasn't tested), and extra/repeated blank lines between
  messages.

## Module 6 — Basic Deterministic File Matcher ✅

- `smartcommit-core::match_files_to_messages(messages, files, diffs)`
  — scores every (message, file) pair using plain token evidence,
  no AI, no embeddings. Returns `messages.len() * files.len()`
  `MatchScore { message_id, file_id, score, reasons }` entries, in
  message-major order.
- Evidence categories and weights: filename token match (0.5),
  directory token match (0.3), diff-added-line token match (0.15),
  literal file-extension mention (0.1) — summed and capped at 1.0.
  Each message token can contribute filename-OR-directory (filename
  wins if both), plus independently diff and extension.
- **Key implementation decision, found via testing, not assumed
  upfront:** modules.md's own worked example ("Fix authentication"
  matching `src/auth/login.rs` with evidence `auth`, `login`,
  `authentication`) requires "authentication" to credit a directory
  literally named "auth" — an exact-token-equality matcher can never
  do this. Added `tokens_related(a, b)`: true if equal, or if one is
  a prefix of the other and both are ≥3 characters. This is the
  module's entire implementation of modules.md's suggested
  "stemmed/simple normalized match" category, deliberately without a
  real stemming library. First implementation used strict equality
  and 3 of the 6 tests failed against realistic fixtures — this
  fix was driven by those failures, not written speculatively.
- Tokenizer splits on any non-alphanumeric character (handles
  `snake_case`, `kebab-case`, path separators, punctuation in one
  pass) plus camelCase boundaries, lowercases everything, filters a
  short English stopword list, and applies minimal plural stemming
  (strip one trailing `s` from words >3 chars, skipping words ending
  `ss`). Domain verbs like "fix"/"update"/"improve" are deliberately
  *not* treated as stopwords — spec gives no basis for assuming
  which verbs are noise vs. signal.
- Diff evidence only looks at *added* lines (`DiffLine::Added`) — a
  removed line describes what used to be true, not what the staged
  change is about.
- Uses `BTreeSet<String>` (not `HashSet`) for every token collection
  specifically for determinism: `HashSet`'s default hasher is
  randomized per process, which would make `reasons` ordering
  non-reproducible across runs — directly undermining the module's
  own completion criteria ("repeatable scores and explanations").
  A dedicated test (`results_are_deterministic_across_repeated_runs`)
  asserts two calls with identical input produce byte-identical
  output, reasons included.
- No CLI command added — modules.md's Module 6 section has no "##
  CLI" heading (unlike every other module so far), so matching
  results will surface through Module 7's `plan` command instead,
  not a new subcommand here.
- Tests: 7 unit tests — obvious auth match (now genuinely
  cross-referencing filename/directory/diff, not just asserting a
  number), obvious docs match, ambiguous shared utility (two
  messages scoring comparably on one shared file), no relevant
  message (score 0.0, empty reasons), multiple relevant messages
  scoring independently, the determinism regression test, and a
  tokenizer unit test for camelCase/snake_case/kebab-case.
- Manually sanity-checked against modules.md's own worked example
  (`src/auth/login.rs` and `src/auth/jwt.rs` both score via the
  shared `auth` directory token against "Fix authentication";
  `README.md` scores 0.0) using a throwaway `cargo run --example`,
  since Module 6 has no CLI surface to exercise directly — the
  example was deleted after confirming the output.

## Module 7 — Commit Planner ✅

- `smartcommit-core::build_commit_plan(messages, files, match_scores)`
  — pure in-memory grouping, no Git access at all (it doesn't even
  take a repo path). `CommitPlan { groups: Vec<CommitGroup>,
  unassigned: Vec<ChangedFile>, warnings: Vec<String> }`;
  `CommitGroup { message: String, files: Vec<ChangedFile>, confidence: f32 }`
  — matches modules.md's suggested struct, with `warnings` added
  since the module's own Validation section requires detecting and
  reporting four specific conditions that don't fit naturally into
  `groups`/`unassigned` alone
- Assignment rule: a file goes to its highest-scoring message only
  if that score clears `MIN_ASSIGNMENT_SCORE` (0.3) *and* isn't
  within `AMBIGUITY_MARGIN` (0.1) of the second-best candidate.
  Otherwise (too weak, or an unclear tie) the file goes to
  `unassigned` — never split across two groups, per rule 2
- Groups are always in the same order as the input `messages` slice
  (rule 5), and a message with zero assigned files still gets a
  `CommitGroup` entry with an empty `files` list and 0.0 confidence
  rather than being dropped (rule 6) — with a matching warning
- Group confidence = mean of its assigned files' match scores,
  capped at 1.0
- `warnings` covers all four items in modules.md's Validation list:
  a message with no matching files, a file with no matching message,
  a file matched almost equally to multiple messages, and (as a
  positive confirmation) "All files were assigned." when nothing
  ended up unassigned
- Determinism: candidate scores are sorted with `message_id` as an
  explicit tiebreaker before picking the best/second-best, so a
  literal tie in score always resolves the same way — same
  motivation as Module 6's `BTreeSet` choice, tested directly
  (`produces_deterministic_output_across_repeated_runs`)
- CLI: `smartcommit plan --messages-file commits.txt` now builds and
  prints the actual plan (previously — Module 5 — it only parsed and
  echoed the messages). Output shape matches modules.md's own
  worked example exactly: `[n] message` / indented file list (or
  `(no matching files)` for an empty group) / `Confidence: NN%`,
  then an `Unassigned:` section (omitted if empty), then a
  `Warnings:` section (omitted if empty). Message parsing and
  reading still happen before repository/staged-changes resolution,
  so a messages-file error is reported without needing a valid repo
  at all
  - `--repo` and staged-changes/diff error handling reuse the same
    patterns already established for `analyze` (including the same
    "diff extraction failed, continue with reduced evidence rather
    than losing the whole command" fallback — here it just means the
    matcher scores on filename/directory only, not diff keywords too)
- Tests: 7 unit tests in `planner.rs` — successful one-to-one
  grouping, one message to several files, an empty group with its
  warning, an unassigned file with no matching message, an ambiguous
  file left unassigned with its warning, full-plan determinism
  across repeated runs, and a regression test for a subtle edge case
  (a second-place score of exactly 0.0 must never trigger the
  ambiguity path, even though the *margin* check alone would treat
  "0.9 vs 0.0" as unambiguously fine — the `second.score > 0.0` guard
  is what makes that so, and it's exercised directly)
- CLI integration tests rewritten: the old Module 5 tests asserted
  message-echo output and didn't touch Git at all; `plan` now
  requires a real repo with staged changes, so the tests build one
  in a temp directory (mirroring the pattern already used throughout
  `smartcommit-core`'s own tests) with files specifically chosen so
  the matcher assigns two of them confidently and leaves the third
  unrelated — directly mirroring modules.md's own CLI example (two
  assigned groups, one unassigned file). A third new test covers the
  "no staged changes at all" case.
- Manually verified: full plan output byte-for-byte matches the
  shape of modules.md's own worked example; a scenario with a
  genuinely empty group (message with nothing staged for it) shows
  `(no matching files)` and its warning rather than being silently
  dropped; confirmed via `git status`/`git log` before and after
  running `plan` that the repository is byte-for-byte unchanged — no
  Git modification occurred, per the module's own requirement

## Module 8 — Timeline Engine ✅

- `smartcommit-core::generate_timeline(commit_count, mode, start, now, allow_future, rng)`
  — produces one `PlannedCommitTime { commit_index, timestamp }` per
  commit. Pure computation: takes `now` as a parameter rather than
  reading the system clock internally, specifically so
  future-timestamp validation is testable without waiting on a real
  clock or mocking time globally
- `TimelineMode::Fixed { interval }` and `TimelineMode::Random { min, max }`
  (both using `chrono::Duration`) — this is the module's first real
  need for a date/time crate, so `chrono = "0.4"` was added to
  `smartcommit-core` (re-exported — `DateTime`, `Duration`,
  `FixedOffset` — from the crate root so downstream code doesn't need
  to add chrono itself just to touch `PlannedCommitTime`'s fields)
- **Key design decision, not directly stated in modules.md but
  required to satisfy its "never accidentally generate a future
  timestamp" rule unconditionally:** the two anchor modes behave
  differently on purpose.
  - No `--start` (the default, most common case): the *last* commit
    is anchored at `now`, earlier commits are placed backward. This
    makes a future timestamp structurally impossible — there's
    nothing to validate, since repeatedly subtracting a non-negative
    gap from `now` can never overshoot it. Verified directly by a
    test with a large random range and 50 commits, asserting every
    timestamp is `<= now`.
  - Custom `--start`: the *first* commit is anchored there, later
    commits move forward. This is the only branch that can actually
    reach a future timestamp, so it's the only branch the
    `allow_future` check guards.
- No `rand` crate dependency — implemented a small, fully
  deterministic splitmix64 PRNG (`SeededRandomSource`) directly,
  since the requirement is "spread out staggered timestamps," not
  cryptographic unpredictability, and modules.md's dependency policy
  is to add a crate only when actually needed. `RandomSource` is a
  trait, so this is modules.md's "seeded RNG or deterministic test
  interface" requirement — `SystemRandomSource` (seeded from wall-clock
  nanos) is the production default; tests always use
  `SeededRandomSource` with a fixed seed for reproducibility
- Validation: zero commits, a non-positive `Fixed` interval, or an
  inverted/negative `Random` range are all rejected before any
  timestamp is computed (`TimelineError::{ZeroCommits, InvalidInterval, InvalidRange}`)
- No CLI command added — like Module 6, modules.md's Module 8 section
  has no "## CLI" heading. Timestamps will actually surface in the
  `plan` preview once Module 11 (Preview and Confirmation CLI) adds
  the timeline flags (`--start`, interval/range options) to the
  command line; wiring it into `plan` now would mean guessing at
  flag names and UX that Module 11 is explicitly responsible for
  deciding
- Tests: 11 unit tests — exact fixed intervals, random gaps staying
  within `[min, max]` (checked across 20 commits), strict
  chronological ordering (backward-from-now branch), a single commit
  with and without a custom start, custom start respected exactly,
  an inverted random range rejected, a non-positive fixed interval
  rejected, zero commits rejected, a custom start that would run
  past `now` rejected by default, the same scenario succeeding with
  `allow_future: true`, and the 50-commit/huge-range regression test
  for the backward-from-now guarantee
- One genuine bug caught during testing (not just a test-writing
  slip): my first version of the "custom start runs past now" test
  asserted the *last* commit (`commit_index: 2`) would be the
  reported offender. Running it showed the engine correctly reports
  `commit_index: 1` instead — the future-timestamp check returns on
  the *first* offending commit it finds, and with 10-minute gaps
  starting 5 minutes before `now`, commit 1 (not commit 2) is already
  past `now`. The engine's behavior was right; my test's arithmetic
  was off by one commit. Fixed the test, not the engine, once I
  hand-verified the timestamps.
- **`ARCHITECTURE.md` (the authoritative spec, not just `modules.md`)
  §21 names an intermediate artifact this module needs to produce:
  the pipeline is explicitly `CommitPlan -> TimelineEngine ->
  TimedCommitPlan -> Executor`.** `modules.md`'s own Module 8 section
  never mentions this type — only the bare `PlannedCommitTime`. Added
  `TimedCommitGroup` / `TimedCommitPlan` and
  `apply_timeline_to_plan(plan, timestamps)`, which pairs each
  `CommitGroup` with its timestamp by `commit_index` (sorting its own
  copy of the timestamp slice first, so caller order doesn't matter)
  and returns `ApplyTimelineError::CountMismatch` if the timestamp
  count doesn't match the group count. This stays Module 8's own
  responsibility (pairing plan + timestamps) rather than reaching
  into Module 9/10 territory (checkpointing, execution) — it produces
  the named artifact and stops there.
- Tests: 13 unit tests total — the 11 above, plus
  `applies_timeline_to_a_plan_pairing_by_commit_index` and
  `rejects_a_timestamp_count_mismatch` for the `TimedCommitPlan` step

## Module 9 — Safety Checkpoint Engine ✅

- `smartcommit-core::create_safety_checkpoint(path, now)` — records
  the pre-operation state and creates a recovery point. Never touches
  an existing branch or HEAD; the only Git write is creating a new
  ref under `refs/smartcommit/backups/<timestamp>`
- `SafetyCheckpoint { original_head, branch, backup_ref, created_at, repository_state, staged_files }`
  — `created_at` is named explicitly in `ARCHITECTURE.md` §24's
  version of this struct (`modules.md`'s own sketch omits it);
  `repository_state` and `staged_files` are full snapshots (Modules
  2 and 3) captured at checkpoint time, since `modules.md`'s
  Requirements text explicitly says to capture "Repository state"
  and "Staged changes," even though its struct sketch only shows
  three fields — Module 12 (Restore) will want this context without
  reconstructing it after the fact
- **Mechanism, verified empirically before relying on it:**
  `git update-ref --create-reflog <ref> <oid> ""` — the empty
  old-value argument makes git refuse to overwrite an existing ref
  (`fatal: ... reference already exists`, confirmed with a live
  repo), which is both the collision-detection signal this module
  needs *and* an atomicity guarantee against a race between two
  processes, without needing to know the all-zeros OID's length
  (which differs between SHA-1 and SHA-256 repos)
- A ref outside `refs/heads/` never appears in `git branch` —
  confirmed empirically and locked in by a dedicated test
  (`backup_ref_does_not_appear_in_branch_listing`) — satisfying
  modules.md's "do not clutter normal branch listings" requirement
  without needing a bespoke listing-filter mechanism
- Collision handling: if the base timestamp-derived ref name already
  exists (two checkpoints requested with the same `now`, most likely
  within the same second, or — as in the test — the exact same
  injected timestamp), the name gets `-1`, `-2`, etc. appended until
  an unused one is found (bounded at 1000 attempts, after which
  `CheckpointError::NameExhausted`). Only "already exists" failures
  are treated as collisions to retry past — any other `update-ref`
  failure (bad OID, permissions, corrupt repo) surfaces immediately
  rather than being silently absorbed into 1000 retries
- Ordering matters for the "failure must not modify the repository"
  rule: HEAD, branch, repository state, and staged files are all
  read *before* the one mutating step (ref creation). If gathering
  any of them fails — including `NoCommits`, returned when HEAD is
  unborn on a brand-new repo with nothing to checkpoint yet — nothing
  has been written. Verified directly by comparing `.git` directory
  entry counts before and after a failed checkpoint attempt
- `now` is a parameter, not read from the system clock internally —
  same rationale as Module 8's `generate_timeline`: deterministic
  tests, and it lets a test force a same-timestamp collision on
  demand instead of relying on a real clock race
- Tests: 7 unit tests — backup ref resolves to the correct HEAD,
  branch and staged-files snapshots are captured correctly, detached
  HEAD is detected as `branch: None`, the backup ref remains valid
  and unchanged after further commits are made, a checkpoint attempt
  on a repo with no commits yet fails cleanly with no repository
  modification, a same-timestamp collision produces two distinct
  refs (both resolving to the same commit), and the backup ref never
  shows up in `git branch --list`
- Manually verified outside the test suite too: created a checkpoint
  ref by hand in a real repo, confirmed `git for-each-ref
  refs/smartcommit` shows it, `git branch` doesn't, and it has a
  (empty but present) reflog entry

## Next module

Module 10 — Commit Executor. The most safety-critical MVP module:
creates the actual commits from an approved `TimedCommitPlan`,
staging only each group's intended files per commit, preserving
unstaged changes, applying planned timestamps, and stopping
immediately (with clear recovery instructions pointing at the
Module 9 checkpoint) if any commit in the sequence fails partway
through.

## Last successful verification

```text
cargo build --workspace   → success
cargo test --workspace    → 72 passed, 0 failed (5 CLI + 67 core)
(no CLI surface for checkpointing yet — it's invoked internally by Module 10, not a standalone command; verified via unit tests and a manual for-each-ref/branch check)
```
