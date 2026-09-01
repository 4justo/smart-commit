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

## Current module

Nothing in progress. Module 0 is complete and verified.

## Next module

Module 1 — Git Repository Detection. Will implement
`Repository::discover(path)` (or equivalent) in `smartcommit-core`,
supporting: current directory, explicit `--repo` path, nested
directory inside a repo, and repository-root detection. Exposes
repository root, `.git` location, and bare-repo status. CLI gets a
`smartcommit doctor` subcommand that reports repository detection
only (state inspection is Module 2). This reuses the *logic* proven
in the earlier prototype's `is_git_repository` check, adapted to the
`Repository::discover` API shape and moved into `smartcommit-core`.

## Last successful verification

```text
cargo build --workspace   → success
cargo test --workspace    → 3 passed, 0 failed
smartcommit --version     → "smartcommit 0.1.0"
smartcommit --help        → usage text printed, exit 0
```
