# SmartCommit — Project State

This file tracks exact progress against the module build order in
`modules.md` (the module-by-module AI development prompts). Update
it at the end of every module.

## Note on prior history

An earlier, non-workspace prototype (single binary crate shelling
out to `git` for repo-detection and staged-changes listing) was
built before `modules.md` was provided. That prototype is **not**
part of this codebase going forward — Module 0 of `modules.md`
specifies a `smartcommit-core` / `smartcommit-cli` workspace split,
and Module 0 explicitly says not to implement Git behavior during
scaffolding. The prototype's logic will be re-introduced, reshaped
to the APIs `modules.md` specifies (`Repository::discover`,
`ChangedFile`/`ChangeKind`, etc.), during Module 1 and Module 3
respectively — not copied in as-is.

## Note on missing context file

`modules.md`'s general instructions say to read
`AI_DEVELOPMENT_CONTEXT.md` if it exists. It was not provided along
with `modules.md`, `README.md`, or `ARCHITECTURE.md`. If it exists
in the developer's actual repository, please share it — it may
contain rules (an earlier `PROJECT_STATE.md` referenced "section 38"
of it) that should apply from here on.

## Completed modules

### Module 0 — Rust Workspace Foundation ✅

- Cargo workspace with two crates:
  - `smartcommit-core` (library) — no dependencies yet
  - `smartcommit-cli` (binary) — depends on `smartcommit-core` and
    `clap` (derive API, pinned to `=4.4.18` for build-environment
    compatibility during development; adjust if unnecessary in the
    developer's own toolchain)
- Directory layout: `crates/`, `tests/`, `fixtures/`, `docs/`,
  `Cargo.toml`, `README.md`, `ARCHITECTURE.md`, `PROJECT_STATE.md`
- Placeholder shared types in `smartcommit-core`: `RepositoryInfo`,
  `ChangedFile`, `CommitMessage`, `CommitPlan` — deliberately
  unfilled; fields are added by the module that actually needs them
  (Modules 1, 3, 5, 7 respectively)
- CLI: `smartcommit --version`, `smartcommit --help` both work; no
  subcommands yet
- Tests: 1 unit test in `smartcommit-core` (placeholder types
  construct), 2 integration tests in `smartcommit-cli` (`--version`
  and `--help` both exit 0 with expected output)
- `cargo build --workspace` and `cargo test --workspace` both
  succeed (3 tests passed, 0 failed)

### Module 1 — Git Repository Detection ✅

- `smartcommit-core::Repository::discover(path)` — detects the Git
  repository containing `path` by shelling out to `git -C <path>
  rev-parse ...` (consistent with the rest of the MVP engine; no
  `libgit2`/`git2` dependency added yet)
- `RepositoryInfo { root, git_dir, is_bare }` — now a real struct
  (was a Module-0 placeholder). For a bare repository, `root` is the
  bare repository's own directory, since there's no working tree to
  report
- `RepositoryError` (via `thiserror`, now a dependency of
  `smartcommit-core`): `PathNotFound`, `PathNotAccessible`,
  `NotARepository`, `GitUnavailable` — no panics for any expected
  user-input case
- Implementation detail worth knowing: `git rev-parse
  --show-toplevel` hard-fails the *entire* invocation on a bare
  repo, even when combined with flags that would otherwise succeed.
  So discovery queries `--absolute-git-dir` + `--is-bare-repository`
  together first, and only asks for `--show-toplevel` separately,
  once it's confirmed the repo isn't bare
- Nested-directory support comes for free from `git -C <path>`'s own
  upward search — no extra logic needed for that case
- CLI: `smartcommit doctor` reports detection only (state inspection
  is Module 2's scope); `smartcommit --repo <path> doctor` targets
  an explicit path via a new global `--repo` option on `Cli`
  (`global = true` so every future subcommand inherits it for free)
- Tests: 5 new unit tests in `smartcommit-core` (valid repo root,
  nested directory, non-repo directory, invalid/nonexistent path,
  bare repository) — all against real temporary repos, none against
  the SmartCommit repo itself
- Manually verified 6 scenarios end-to-end: repo root, nested dir,
  non-repo dir, explicit `--repo` to a different valid repo,
  explicit `--repo` to a nonexistent path, and a bare repo — exit
  codes and output all matched expectations
- `cargo build --workspace` and `cargo test --workspace` both
  succeed (8 tests passed, 0 failed: 2 CLI + 6 core)

## Note on parallel work

A friend is independently building Module 1 as well. This
implementation was built to the `modules.md` spec without seeing
that work. If/when it's shared, the two will need to be reconciled
before Module 2 builds on top of either one — Module 2 should build
against a single agreed `Repository::discover` API, not against
whichever version happened to be built last.


## Current module

Nothing in progress. Module 1 is complete and verified.

## Next module

Module 2 — Repository State Inspector. Will build a
`RepositoryState` model (branch, HEAD, detached-HEAD, merge/rebase/
cherry-pick in progress, conflicts, staged/unstaged/untracked
status) and a Safe/Warning/Blocked classification, on top of
`Repository::discover`. `smartcommit doctor` expands to report full
state, not just detection. **Should not start until the parallel
Module 1 work is reconciled — see note above.**

## Last successful verification

```text
cargo build --workspace   → success
cargo test --workspace    → 8 passed, 0 failed
smartcommit doctor (real repo)        → "Repository detected" / "Root: ..." / exit 0
smartcommit doctor (nested dir)       → resolves to repo root / exit 0
smartcommit doctor (non-repo dir)     → "No Git repository found." / exit 1
smartcommit --repo <path> doctor      → targets explicit path / exit 0
smartcommit --repo <bad-path> doctor  → "Path does not exist: ..." / exit 1
smartcommit --repo <bare-repo> doctor → "Bare repository: yes" / exit 0
```

