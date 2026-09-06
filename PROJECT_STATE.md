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

## Current module

Nothing in progress. Module 3 is complete and verified.

## Next module

Module 4 — Diff Extraction Engine. For every `ChangedFile`, extract
the staged diff (HEAD vs. index) as hunks — `FileDiff { path, hunks,
additions, deletions }`, `DiffHunk { header, lines }`. Must detect
and skip binary files rather than treating their content as text.
`smartcommit analyze` gains `+N -M` / hunk-count summary output per
file.

## Last successful verification

```text
cargo build --workspace   → success
cargo test --workspace    → 21 passed, 0 failed (2 CLI + 19 core)
smartcommit doctor  → full state report (branch/HEAD/staged/unstaged/untracked/safety)
smartcommit analyze → staged file list with change kind, or "No staged changes found." + exit 1
```
