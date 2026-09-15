# Changelog

All notable changes to SmartCommit are documented in this file.

The format follows [Keep a Changelog](https://keepachangelog.com/en/1.0.0/).
SmartCommit uses [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

---

## [0.1.0] — 2026-09-15

First public release. The MVP delivers a small, safe, reliable Git
commit-splitting engine with no dependency on AI or any external
service.

### Added

**Core commands**

- `smartcommit doctor` — detect the current repository and report its
  full state: branch, HEAD, in-progress merge/rebase/cherry-pick,
  conflicts, and staged/unstaged/untracked status.
- `smartcommit analyze` — list currently staged files with diff
  summaries (`+N -M`, hunk count, binary detection). `--verbose` prints
  full hunk content.
- `smartcommit plan --messages-file <file>` — preview a proposed
  commit plan without touching the repository. Shows proposed commits
  with their assigned files, confidence scores, planned timestamps,
  unassigned files, and warnings.
- `smartcommit commit --messages-file <file>` — build the same plan,
  prompt for confirmation (`[y/N]`, default No), then create the
  commits for real. `--yes` skips the prompt; `--dry-run` runs every
  step except the Git write.
- `smartcommit restore` — undo the most recent SmartCommit operation
  using the safety checkpoint recorded before it ran. Shows exactly
  what will change and requires confirmation. Refuses automatically
  when restoration would be unsafe and prints the exact manual
  `git update-ref` command instead.

**Planning engine**

- Deterministic file-to-message matching using filename tokens,
  directory tokens, diff-added-line tokens, and file extension hints.
  No AI, no embeddings.
- Confidence scoring and ambiguity detection: files that match two
  messages almost equally are left unassigned rather than arbitrarily
  placed.
- Configurable thresholds via `.smartcommit.toml`.

**Timeline engine**

- Fixed-interval mode (default 10 minutes between commits).
- Random-range mode (`--random-min-minutes` / `--random-max-minutes`).
- Custom start time (`--start <RFC3339>`).
- By default, the last commit is anchored at the current time and
  earlier commits are placed backward — a future timestamp is
  structurally impossible in this mode.

**Safety system**

- Safety checkpoint created before the first commit, stored as a Git
  ref under `refs/smartcommit/backups/`. Never appears in
  `git branch`.
- Operation record written to `.git/smartcommit/last-operation.toml`
  and `.git/smartcommit/operations/<id>.toml` after each run.
- Commit executor uses a temporary index (`GIT_INDEX_FILE`) per
  commit, so the real `.git/index` is never touched during execution.
  Unstaged changes survive the entire operation intact.
- Partial-failure reporting: if commit N of M fails, the report lists
  completed commits, the failure reason, and commits not yet
  attempted. Exit code 3 distinguishes a partial failure from a total
  failure (exit code 1).

**Configuration**

- Optional `.smartcommit.toml` at the repository root.
- Configurable: `matching.minimum_confidence`, `matching.ambiguity_margin`,
  `timeline.gap_minutes`, `timeline.random_min_minutes`,
  `timeline.random_max_minutes`, `commit.require_confirmation`.
- Resolution order: CLI flag > config file > built-in default.
- Invalid config (bad syntax or out-of-range value) is always caught
  before any Git work begins.

**Exit codes**

| Code | Meaning |
|---|---|
| 0 | Success |
| 1 | Validation, repository, or configuration failure |
| 2 | User declined the confirmation prompt |
| 3 | Execution failed partway (some commits were created) |

**Test suite**

- 135 tests: 106 unit tests in `smartcommit-core`, 12 CLI invocation
  tests, 17 end-to-end tests against real temporary repositories.
- Passes `cargo fmt --check` and
  `cargo clippy --workspace --all-targets -- -D warnings`.

### Not included in v0.1.0

The following are explicitly out of scope for this release and will be
considered for later versions:

- LLMs, embeddings, or any AI-assisted matching
- Hunk-level splitting (commits today are file-granularity only)
- Automatic commit message generation
- Interactive TUI
- VS Code or GitHub integration
- Semantic history search or bug tracing
