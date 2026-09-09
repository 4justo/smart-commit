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

Nothing in progress. Module 6 is complete and verified.

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

## Next module

Module 7 — Commit Planner. Turns parsed messages, staged files,
diffs, and match scores into a proposed `CommitPlan` (groups +
unassigned files), with per-group confidence and an ambiguity
threshold applied on top of Module 6's raw scores. CLI:
`smartcommit plan --messages-file commits.txt` gets its first real
plan output (currently `plan` only parses and prints messages).

## Last successful verification

```text
cargo build --workspace   → success
cargo test --workspace    → 44 passed, 0 failed (4 CLI + 40 core)
smartcommit plan --messages-file commits.txt → parses and prints commit messages
(matcher has no CLI surface yet — verified via unit tests and a throwaway `cargo run --example`)
```
