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

Nothing in progress. Module 12 is complete and verified.

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

## Module 10 — Commit Executor ✅

The most safety-critical module in the MVP. **The entire approach was
validated empirically against a real repository before any Rust was
written** — see the design note at the top of `executor.rs` for the
full reasoning; summarized here.

- `smartcommit-core::execute_commit_plan(repo_path, groups, timestamps, checkpoint)`
  — creates one real commit per non-empty `CommitGroup` (Module 7),
  in order, applying Module 8's timestamps when `timestamps` is
  `Some` (git's real current time when `None` — the direct
  implementation of "if timeline mode is enabled")
- **Rejected `git commit-tree` (the "obviously safest" plumbing
  approach) for one specific, load-bearing reason: it never runs
  hooks.** modules.md explicitly requires treating hook rejection as
  a real execution failure, which is impossible if hooks can't fire
  at all. Used a **temporary index per commit** instead (`GIT_INDEX_FILE`
  pointed at a throwaway file under `.git/smartcommit/tmp/`,
  populated via `git read-tree` + `git update-index --cacheinfo`),
  committed with ordinary `git commit` — hooks run, scoped to
  exactly that commit's own content
- Confirmed empirically, before writing any Rust, in a real repo with
  a real `pre-commit` hook:
  - `git commit` with `GIT_INDEX_FILE` set never touches the real
    `.git/index` — `git ls-files --stage` byte-for-byte identical
    before and after a full multi-commit sequence
  - An unstaged edit stacked *on top of* a file being split into a
    commit right now survives completely untouched, as does an
    unrelated file's unstaged-only edit
  - A `pre-commit` hook invoked this way sees `git diff --cached`
    scoped to *only* that specific commit's files — not the original
    full staged set
  - A rejected commit (hook exits non-zero) leaves HEAD unmoved and
    the real index unmoved; git's own exit code and stderr are all
    that's needed to detect and report it
- Each group's commit is created for real (branch ref actually moves)
  before the next group starts — not built as a chain of dangling
  objects attached at the end. This matches modules.md's own failure
  narrative directly: "if commit #3 of 5 fails, report completed
  commits" only makes sense if commits 1 and 2 are already real
  history at that point
- Precondition check: the repository's actual current HEAD must
  still match `checkpoint.original_head` before anything happens —
  if something changed the repo after the checkpoint was taken,
  execution refuses to proceed at all (`ExecutionError::HeadMoved`)
  rather than building on a stale assumption about "the original
  state"
- `ExecutionReport { completed, failure, not_attempted, checkpoint_backup_ref }`
  — a mid-sequence failure is a normal, fully-described *outcome*
  (`Ok(report)` with `report.failure: Some(...)`), not a Rust `Err`,
  since the caller needs the completed-commits list *alongside* the
  failure info in one place, matching the exact report shape
  `ARCHITECTURE.md` §29 shows (completed / failed / not executed /
  recovery checkpoint)
- An empty group (no files) is silently skipped — Module 7 already
  reported it as a warning during planning; the executor doesn't
  re-report it as a failure
- `--cacheinfo` is passed as three separate arguments (mode, object,
  path) rather than the comma-joined single-argument form —
  empirically confirmed this correctly handles a path containing a
  literal comma, which the joined form would parse ambiguously
- `TempIndexArea` (an internal `Drop`-guarded temp directory under
  `.git/smartcommit/tmp/`) guarantees per-run cleanup even on an
  early return from a failure, so a failed attempt never leaves temp
  index files behind to confuse the next one
- Author identity is never touched — only `GIT_AUTHOR_DATE`/
  `GIT_COMMITTER_DATE` are set (and only when a timestamp is
  supplied), confirmed to require *both* env vars since `--date`
  alone only covers the author date (this was actually verified back
  in Module 8's manual checks, and reused here)
- Tests: 11 unit tests — two commits from two groups, three commits
  in sequence (with a direct tree-content check partway through),
  planned timestamps applied to both author and committer dates,
  unstaged changes preserved (including one stacked on a staged
  file), staged content correctly reflects the *staged* version (not
  a later working-tree edit), failure midway (completed/failure/
  not_attempted all correct, HEAD stops exactly where expected), a
  **real hook rejection** via an actual installed `pre-commit` script
  (HEAD doesn't move past the last success, the real index still
  shows the rejected file staged), deleted files, added files, empty
  groups skipped without being reported as failures, and refusing to
  run at all when HEAD has moved since the checkpoint
- Manually verified end-to-end beyond the unit tests: a full
  plan → checkpoint → timeline → execute pipeline on a realistic
  two-message scenario, using every module built so far together,
  producing two correctly-scoped real commits and a clean final
  `git status`

### Post-delivery fix: `applies_planned_timestamps` test was git-version-fragile

Reported by the developer running `cargo test --workspace` locally
(git 2.43.0 in the sandbox where this was built vs. a newer git on
the developer's machine): the test failed with `left: "...Z"` vs.
`right: "...+00:00"`. Different git versions render a zero UTC
offset differently under `--date=iso-strict` (`Z` vs. `+00:00` — both
valid ISO 8601 for the identical instant), and the test asserted an
exact string match against one specific rendering. **The executor
itself was never wrong** — the timestamps were being applied
correctly on both machines; only the test's string comparison was
fragile to a formatting choice that isn't actually part of git's
documented/stable output contract. Fixed by parsing both the actual
and expected dates into `DateTime<FixedOffset>` and comparing the
parsed instants, which are equal regardless of which valid ISO 8601
rendering git chooses. All 83 tests pass with this fix.

## Module 11 — Preview and Confirmation CLI ✅

**This is the module where SmartCommit becomes an actually usable
tool** — the full core loop (`git add .` → `smartcommit plan` →
`smartcommit commit`) works end to end from the terminal.

- `smartcommit plan --messages-file commits.txt` — full preview,
  read-only, matching modules.md's own worked example output
  byte-for-byte in structure (verified — see below)
- `smartcommit commit --messages-file commits.txt` — the same
  preview, then `[y/N]` confirmation (default No — confirmed via a
  test that pipes a bare newline to stdin and asserts cancellation),
  `--yes` to skip it, `--dry-run` to stop after the preview with no
  Git modification
- **Flow order corrected to match `ARCHITECTURE.md` §34/§35
  precisely** (discover repository → inspect state → read staged
  changes → parse messages → match → build plan → generate timeline
  → render), rather than the ad-hoc ordering `plan` had from Module
  7 (which parsed messages before touching the repository at all).
  One consequence: `plan`/`commit` now also call `inspect_repository_state`
  and refuse to proceed if `RepositorySafety::Blocked` (merge/
  rebase/cherry-pick in progress, or unresolved conflicts) — this
  wasn't explicit in modules.md's Module 11 text, but is directly
  implied by §34's "Inspect state" step applying to `plan` too, not
  just `commit`, and is a real safety improvement (previously
  `plan`/`commit` would attempt to match and plan against a
  repository in an unsafe state without ever checking)
- `plan`, `commit --dry-run`, and `commit` all share one
  `build_preview()` pipeline — `ARCHITECTURE.md` §36's explicit
  requirement ("the same planning code should be used for dry-run and
  real execution"), extended here to cover the plain preview path too
  as the natural third instance of the same pipeline
- Timeline is now wired into the CLI for the first time (Module 8
  had no CLI surface until now): `--start`, `--interval-minutes`
  (default 10), `--random-min-minutes`/`--random-max-minutes` (must
  be given together), `--allow-future`. Manually verified: default
  fixed timeline, custom `--start`, random mode, and the "only one of
  random-min/random-max given" validation error
- Exit codes made meaningful and distinct, per modules.md's own
  requirement: `0` success, `1` validation/repository failure, `2`
  user declined confirmation, `3` execution failed partway (some
  commits were created — distinct from `1` specifically because the
  repository state has genuinely changed and a blind retry would be
  wrong)
- Execution report rendering matches `ARCHITECTURE.md` §29's exact
  shape (completed / failed / not executed / recovery checkpoint) —
  manually verified with a real rejecting `pre-commit` hook,
  confirming the printed `refs/smartcommit/backups/...` ref is real
  and resolvable after a partial failure
- `commit` validates there's actually something to commit (at least
  one group has files) before proceeding to confirmation/checkpoint/
  execution — printing "Nothing to commit" and exiting `1` rather
  than running the whole safety-checkpoint machinery for a plan that
  would create zero commits
- Tests: 9 CLI integration tests (up from 5) — full plan preview
  (updated assertions for the new `SMARTCOMMIT PLAN` format, staged
  file count, proposed commit count, `Time:` lines), no-staged-
  changes, blocked-repository refusal (a real merge conflict
  provoked and confirmed via stderr), `commit --dry-run` (asserts
  `git log`/`git status` byte-identical before and after), `commit
  --yes` (asserts real commits with correct messages, and that the
  unassigned file remains staged afterward), and declining
  confirmation via a piped empty line (asserts exit code 2, no
  commits created)
- One existing test (`plan_rejects_empty_messages_file`) needed a
  real fix, not just an assertion update: it previously ran with no
  `--repo` flag at all, relying on message-parsing happening before
  any repository check. Now that repository checks happen first (per
  the corrected flow order above), that test needed a real temp repo
  with staged changes to reach the message-parsing step at all —
  fixed by giving it one, rather than reverting the flow-order fix to
  keep the old test passing
- README.md rewritten — it was still describing Module 0's
  `--version`/`--help`-only state; now documents the actual working
  workflow, timeline flags, and exit codes

## Module 12 — Restore and Recovery ✅

- **Key design insight, a direct payoff of Module 10's careful
  design:** since the commit executor never touches the real
  `.git/index` (confirmed empirically back in Module 10), the index
  still holds exactly the pre-operation staged blobs the entire time
  a SmartCommit operation is in effect. That means restore doesn't
  need to touch the index or working tree *at all* — it only needs
  to move the branch (or HEAD, if detached) back to `original_head`.
  Once that ref moves, `git status` automatically and correctly shows
  every originally-staged file as staged again relative to the
  restored HEAD, with zero extra logic required. Confirmed
  empirically before writing any Rust: `git update-ref
  refs/heads/<branch> <oid>` on the checked-out branch moves HEAD
  while leaving the working tree and index completely untouched.
- **New dependency, one module earlier than my own prior
  prediction:** `serde` (with the `derive` and, on `chrono`, `serde`
  features) and `toml` were added in *this* module, not Module 13 as
  an earlier `ARCHITECTURE.md` addendum note predicted — corrected
  in that note now that it's known to be wrong, rather than left
  standing. `ARCHITECTURE.md` §31's own TOML example
  (`operation_record.rs`) is what actually needed it first.
  - Sandbox note: `toml = "0.8"` pulls in `indexmap`/`hashbrown`
    versions requiring the `edition2024` Cargo feature, which this
    sandbox's cargo (1.75.0) doesn't support — same class of issue
    as the earlier `clap` pin. Pinned to `toml = "0.5"` here, which
    has no such transitive requirement and exposes the same
    `to_string_pretty`/`from_str` API used. A more capable local
    toolchain can likely use a newer `toml` without any code changes.
- `smartcommit-core::operation_record`: `OperationRecord { id,
  start_time, original_head, final_head, branch, checkpoint_ref,
  created_commits, status }`, matching `ARCHITECTURE.md` §31's field
  list exactly. `OperationStatus::{Completed, PartialFailure, Failed}`
  derived from an `ExecutionReport` — `Failed` specifically means
  nothing at all was completed (the very first commit failed),
  distinct from `PartialFailure` where at least one real commit
  exists. `save_operation_record` writes both `last-operation.toml`
  (always overwritten) and `operations/<id>.toml` (kept per
  operation, `id` reusing the same timestamp-format convention as
  Module 9's checkpoint refs) — matching §30's storage layout exactly.
  No source code or secrets are stored, per §30's explicit rule —
  only commit hashes, a branch name, and timestamps.
- `smartcommit-core::restore`: `plan_restore` (read-only — inspects
  current state, compares against the last operation record, judges
  safety) and `execute_restore` (the single ref-move, refuses outright
  if the plan it's given isn't marked safe). Unsafe conditions,
  matching modules.md's "do not destroy unrelated work" requirement:
  current HEAD no longer matches the record's `final_head` (something
  else committed since), current branch doesn't match the recorded
  branch, or the checkpoint ref no longer resolves to the recorded
  `original_head`. An unstaged/untracked/staged change in the working
  tree is reported (`working_tree_has_changes`) but is **not** a
  safety gate on its own — restore never touches the working tree
  regardless, so it can't be endangered by restoring
- `manual_recovery_instructions` prints the exact `git update-ref`
  command a developer could run themselves if they've reviewed an
  unsafe plan and are sure — matching modules.md's "refuse automatic
  restore and provide exact manual recovery instructions" precisely,
  and reusing the identical logic `execute_restore` would use so the
  instructions are always correct, never hand-derived separately
- CLI: `smartcommit restore [--yes]` — shows current HEAD, checkpoint
  HEAD, branch, commits that would be removed, and working-tree
  state (modules.md's exact "Safety" list) before any confirmation;
  `[y/N]` prompt (default No, reusing the same pattern as `commit`);
  refuses with `Safety: BLOCKED` plus the manual instructions when
  unsafe, without ever prompting for confirmation in that case (there
  is nothing safe to confirm). `commit` now calls
  `save_operation_record` right after execution (success or partial
  failure alike) — a record-save failure is a warning, not treated as
  the commits themselves having failed, since they already exist for
  real at that point
- Tests: 13 new unit tests (7 `operation_record` + 6 `restore`) plus
  4 new CLI integration tests. The `restore` unit tests run a real
  plan → checkpoint → execute → record sequence (not synthetic
  fixtures) before testing restore itself, since restore's whole
  premise depends on exactly how execution leaves the real index —
  a synthetic checkpoint/record pair wouldn't actually prove the
  index-preservation claim the way running the real pipeline does
- Manually verified beyond the test suite: a real `commit --yes` run
  followed by `restore --yes` — confirmed HEAD returns to the exact
  pre-operation commit and both originally-staged files reappear as
  staged, byte-for-byte the pre-operation state; the unsafe-refusal
  path with a real unrelated intervening commit (correct reasons
  shown, repository completely unaffected by the refused attempt,
  correct manual command printed); and the "no operation recorded
  yet" case on a fresh repository

## Module 13 — Configuration ✅

- `smartcommit-core::load_config(repo_root)` — loads
  `.smartcommit.toml` from the repository root. A missing file is not
  an error; an invalid file (bad TOML syntax or out-of-range value)
  is always a hard error reported before any Git work begins.
- `SmartCommitConfig { matching, timeline, commit }` — fully resolved
  config; every field guaranteed to hold a valid, in-range value
  (either from the file or from the built-in default).
- `MatchingConfig { minimum_confidence, ambiguity_margin }` —
  previously hardcoded in `planner.rs` as `MIN_ASSIGNMENT_SCORE`
  (0.3) and `AMBIGUITY_MARGIN` (0.1). Making them configurable lets
  teams tune the matcher for their codebase layout without recompiling.
- `TimelineConfig { gap_minutes, random_min_minutes,
  random_max_minutes }` — default fixed interval (10 min) or optional
  random range; `random_min` and `random_max` must both be present or
  both absent (either alone is a validation error).
- `CommitConfig { require_confirmation }` — `false` makes
  non-interactive execution the default (equivalent to always passing
  `--yes`); `--yes` still overrides this when given.
- Resolution order: CLI flag > `.smartcommit.toml` > built-in
  default. `load_config` handles the file layer only; the CLI applies
  flag overrides on top of the returned struct.
- `ConfigError`: `Io` (file unreadable), `Parse` (invalid TOML),
  `Validation` (value out of range). All three produce readable
  messages mentioning `.smartcommit.toml`. No panics.
- `build_commit_plan` now takes `min_score` and `ambiguity_margin` as
  explicit parameters instead of hardcoded constants — the CLI passes
  `config.matching.*` through, and the planner's own tests use the
  same default values as before so pre-existing behaviour is
  unchanged.
- New dependency: none beyond `serde` + `toml` already added in
  Module 12. `toml` pinned to `0.5` to avoid `indexmap`/`hashbrown`
  transitive requirements incompatible with the sandbox's cargo 1.75.
- Tests: 16 unit tests in `config.rs` covering defaults, full
  parse, partial parse (missing fields fall back to defaults), empty
  file, invalid syntax, out-of-range values (both above and below),
  random range (valid, inverted, one-sided), zero `gap_minutes`, and
  zero `random_min_minutes`.
- Manually verified: default run (no file), a full config, a high
  `minimum_confidence` forcing a file into Unassigned, `gap_minutes`
  reflected in `Time:` output, CLI `--interval-minutes` overriding
  config, `require_confirmation = false` skipping the prompt,
  `--yes` overriding `require_confirmation = true`, broken TOML
  rejected before any Git work, out-of-range value rejected.

## Module 14 — End-to-End MVP Test Suite ✅

- `crates/smartcommit-cli/tests/end_to_end.rs` — 17 integration tests
  that drive the real `smartcommit` binary against temporary Git
  repositories. Never touches the SmartCommit repository itself.
- All 17 scenarios from `modules.md` Module 14 are covered:
  1. Main scenario — three files, three correctly-scoped commits,
     each commit contains only its own file
  2. No repository — non-zero exit, readable error, no panic
  3. No staged changes — `No staged changes found.`, exit 1
  4. One message — single commit created
  5. Multiple messages — all messages in history in order
  6. Ambiguous file — appears in `Unassigned:`, never committed
  7. Unassigned file — stays staged after operation, absent from
     every commit tree
  8. Added file — present in commit tree
  9. Deleted file — removed from tree and from disk
  10. Unstaged changes preserved — working-tree edit and scratch file
      survive; commit contains the staged version, not the later edit
  11. Timeline timestamps — 15-minute gap between author dates verified
  12. Dry run — `git log` and `git status` byte-identical before/after
  13. User cancellation — exit code 2, no commits created
  14. Recovery checkpoint — ref under `refs/smartcommit/backups/`
      points to original HEAD, hidden from `git branch`
  15. Restore — HEAD reset, originally-staged files re-staged
  16. Binary file — no panic in `plan` or `commit`
  17. Invalid config — broken TOML and out-of-range value both caught
      before any Git work; no commits created, no panic
- Pre-existing clippy warnings surfaced by `-D warnings` and fixed as
  part of this module: `derivable_impls` on `SmartCommitConfig`
  (replaced manual `impl Default` with `#[derive(Default)]`),
  `ptr_arg` in `write_config` test helper (`&PathBuf` → `&Path`),
  `unnecessary_to_owned` in checkpoint test, `cmp_owned` in diff test.
- All three Module 14 completion criteria pass:
  - `cargo fmt --check` → clean
  - `cargo clippy --workspace --all-targets -- -D warnings` → clean
  - `cargo test --workspace` → 135 passed, 0 failed
    (12 cli_invocation + 17 end_to_end + 106 core)
- Manual testing guide: `docs/testing/module-14.md`

## Module 15 — MVP Polish and Release Preparation ✅

- CLI help text reviewed and polished for all subcommands.
- Module-level doc comment in `main.rs` updated to reflect current
  state (was still labelled "Module 11 status").
- `CHANGELOG.md` created — v0.1.0 entry documenting the full MVP.
- `SECURITY.md` created — repository safety model, recovery path, and
  scope of what SmartCommit does and does not do to your repo.
- `ARCHITECTURE.md` implementation-status addendum updated for Modules
  13 and 14, dependency table corrected, workspace layout updated.
- `README.md` updated — correct status header, `.smartcommit.toml`
  configuration section added, accurate project layout.
- `PROJECT_STATE.md` updated with Module 13, 14, and 15 entries.
- Final verification:
  - `cargo fmt` → clean
  - `cargo clippy --workspace --all-targets -- -D warnings` → clean
  - `cargo test --workspace` → 135 passed, 0 failed
  - `cargo build --release` → success

## Current module

None. MVP complete. Ready to tag v0.1.0.

## Last successful verification

```text
cargo fmt --check                                    → clean
cargo clippy --workspace --all-targets -- -D warnings → clean
cargo build --workspace                              → success
cargo build --release                                → success
cargo test --workspace                               → 135 passed, 0 failed
                                                       (12 cli_invocation +
                                                        17 end_to_end +
                                                       106 core)

smartcommit commit --yes then smartcommit restore --yes
  → HEAD returns to exact pre-operation commit, files re-staged, exit 0
smartcommit restore after an unrelated intervening commit
  → Safety: BLOCKED, reasons shown, repository untouched, exit 1
smartcommit restore with no prior operation
  → "No SmartCommit operation found to restore.", exit 1
smartcommit commit --messages-file … --dry-run
  → full preview, "Dry run: no changes were made.", exit 0
smartcommit plan with .smartcommit.toml gap_minutes = 30
  → Time: lines 30 minutes apart
```
