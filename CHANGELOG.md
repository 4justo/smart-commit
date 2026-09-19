# Changelog

All notable changes to SmartCommit are documented here. This project
has not yet made a `v0.1.0` release — Module 15 (release polish) is
still open, so this file tracks progress by module rather than by
version tag for now.

Format loosely follows [Keep a Changelog](https://keepachangelog.com/),
adapted for a pre-release, module-by-module build.

## [Unreleased]

### Added — Module 16: Hunk-Level Change Analysis
- `DiffHunk` (Module 4) extended with deterministic `id`, `file_path`,
  parsed `old_start`/`old_count`/`new_start`/`new_count`,
  `raw_content`, and `stats: HunkStats { additions, deletions,
  context, total_changed_lines }`.
- `scm analyze --verbose` shows one new summary line per hunk
  (id, stats, old/new ranges) ahead of the existing header/content
  output, which is otherwise unchanged.
- Read-only: no Git-mutating command is used anywhere in this module.
  Hunk *splitting* into separate commits is explicitly not
  implemented yet.

### Changed
- **Breaking (CLI):** the produced binary is now named `scm`
  (previously `smartcommit`). The Rust crate names
  (`smartcommit-core`, `smartcommit-cli`) and the
  `refs/smartcommit/backups/` Git ref namespace (Module 9) are
  unchanged.

### Fixed
- Two pre-existing `cargo clippy` findings unrelated to Module 16's
  own changes (a needless double-borrow in
  `smartcommit-cli/src/main.rs`, a single-character string pattern in
  a `checkpoint.rs` test) — found and fixed while getting `cargo
  clippy --workspace --all-targets -- -D warnings` to pass clean,
  which Module 16's own testing requirements call for across the
  whole workspace.
- `PROJECT_STATE.md` had not been updated past "Module 12 complete"
  even though Modules 13 and 14 were already implemented in the
  codebase — brought up to date.

## Modules 0-14 (undated — backfilled retroactively)

- **Module 0** — Cargo workspace foundation (`smartcommit-core`
  library, `smartcommit-cli` binary), `--version`/`--help`.
- **Module 1** — Git repository detection (`Repository::discover`),
  `doctor` (detection only at this point).
- **Module 2** — Repository state inspection (branch, HEAD,
  merge/rebase/cherry-pick in progress, conflicts,
  staged/unstaged/untracked), `doctor` expanded to report full state.
- **Module 3** — Staged change discovery (`get_staged_changes`),
  `analyze` command.
- **Module 4** — Diff extraction (`get_staged_diffs`, `FileDiff`,
  `DiffHunk`, `DiffLine`), binary-file detection, `analyze` gains
  diff summaries and `--verbose`.
- **Module 5** — Multiline commit message parsing
  (`CommitMessage`/`CommitMessages`), blank-line-separated and
  one-per-line modes.
- **Module 6** — Deterministic file-to-message matching
  (`match_files_to_messages`), no AI/embeddings.
- **Module 7** — Commit planning (`build_commit_plan`), grouping,
  confidence, ambiguity handling, warnings.
- **Module 8** — Timeline generation (`generate_timeline`), fixed and
  random modes, no-future-timestamp guarantee.
- **Module 9** — Safety checkpoint creation
  (`create_safety_checkpoint`), a Git ref under
  `refs/smartcommit/backups/`.
- **Module 10** — Commit execution (`execute_commit_plan`) via a
  per-commit temporary index — never touches the real `.git/index`
  or the working tree; hooks run correctly, scoped per commit.
- **Module 11** — Preview and confirmation CLI: `plan`/`commit` wired
  together, `[y/N]` confirmation, `--yes`, `--dry-run`, meaningful
  exit codes.
- **Module 12** — Restore and recovery (`plan_restore`/
  `execute_restore`), operation records under `.git/smartcommit/`,
  safety-gated automatic restore with manual-instruction fallback.
- **Module 13** — `.smartcommit.toml` configuration
  (`SmartCommitConfig`), CLI-flag-over-config-over-default resolution.
- **Module 14** — End-to-end MVP test suite (`tests/end_to_end.rs`),
  17 scenarios covering the full pipeline.

## Not yet done

- **Module 15** — Release polish: this file, `SECURITY.md`, a
  cross-platform review, `--help` text audit, and an actual `v0.1.0`
  tag. Not done as of Module 16.
- **Module 17+** — Explicitly out of scope for Module 16 per its own
  completion rule.
